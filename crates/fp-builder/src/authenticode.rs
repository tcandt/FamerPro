use anyhow::{bail, Context, Result};
use fp_bundle_contract::{
    AuthenticodeTrustStatus, AuthenticodeVerificationReport, SigningMode,
};
use fp_bundle_verify::verify_authenticode_pe;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QaSelfSignedCertInfo {
    pub subject: String,
    pub thumbprint: String,
    pub store_location: String,
    pub cert_ref: String,
    pub not_after_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SigningToolchainStatus {
    pub signtool_found: bool,
    pub discovered_path: Option<PathBuf>,
    pub sdk_bundled_in_app: bool,
    pub searched_locations: Vec<String>,
    pub unsigned_internal_supported: bool,
    pub guidance: String,
}

#[derive(Debug, Clone, Default)]
pub struct AuthenticodeSignRequest {
    pub signing_mode: SigningMode,
    /// Reference string ONLY (`CERT_STORE://CurrentUser/My/<thumbprint>` or `PFX://<path>` or bare SHA1 thumbprint).
    /// Passwords for PFX are NEVER persisted here (read transiently from `FP_AUTHENTICODE_PFX_PASSWORD` if PFX used).
    pub cert_ref: Option<String>,
    pub description: Option<String>,
    pub timestamp_url: Option<String>,
}

/// Discovers an installed Windows SDK `signtool.exe` cleanly via filesystem & PATH probe
/// (never hardcodes one SDK build, zero shell interpolation, and NEVER bundles Microsoft SDK files).
pub fn discover_signing_toolchain() -> SigningToolchainStatus {
    let mut searched = Vec::new();
    if let Ok(explicit) = std::env::var("FP_SIGNTOOL_PATH") {
        let p = PathBuf::from(explicit.trim());
        searched.push(format!("FP_SIGNTOOL_PATH={}", p.display()));
        if p.is_file() {
            return SigningToolchainStatus {
                signtool_found: true,
                discovered_path: Some(p),
                sdk_bundled_in_app: false,
                searched_locations: searched,
                unsigned_internal_supported: true,
                guidance: "Using FP_SIGNTOOL_PATH override; UnsignedInternal works without SDK.".to_string(),
            };
        }
    }
    searched.push("PATH".to_string());
    if let Ok(path_env) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_env) {
            let c = dir.join("signtool.exe");
            if c.is_file() {
                return SigningToolchainStatus {
                    signtool_found: true,
                    discovered_path: Some(c),
                    sdk_bundled_in_app: false,
                    searched_locations: searched,
                    unsigned_internal_supported: true,
                    guidance: "Found signtool.exe on PATH; SDK components are not bundled.".to_string(),
                };
            }
        }
    }
    for arch in ["x64", "arm64", "x86"] {
        let kits_bin = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
        searched.push(format!("{}\\*\\{}\\signtool.exe", kits_bin.display(), arch));
        if kits_bin.is_dir() {
            if let Ok(rd) = std::fs::read_dir(kits_bin) {
                let mut versions: Vec<PathBuf> = rd
                    .flatten()
                    .map(|e| e.path().join(arch).join("signtool.exe"))
                    .filter(|p| p.is_file())
                    .collect();
                versions.sort();
                if let Some(latest) = versions.pop() {
                    return SigningToolchainStatus {
                        signtool_found: true,
                        discovered_path: Some(latest),
                        sdk_bundled_in_app: false,
                        searched_locations: searched,
                        unsigned_internal_supported: true,
                        guidance: "Detected Windows SDK signtool.exe on host (external SDK; not bundled).".to_string(),
                    };
                }
            }
        }
    }
    let kit_ack = Path::new(r"C:\Program Files (x86)\Windows Kits\10\App Certification Kit\signtool.exe");
    searched.push(kit_ack.to_string_lossy().to_string());
    if kit_ack.is_file() {
        return SigningToolchainStatus {
            signtool_found: true,
            discovered_path: Some(kit_ack.to_path_buf()),
            sdk_bundled_in_app: false,
            searched_locations: searched,
            unsigned_internal_supported: true,
            guidance: "Detected App Certification Kit signtool.exe on host.".to_string(),
        };
    }

    SigningToolchainStatus {
        signtool_found: false,
        discovered_path: None,
        sdk_bundled_in_app: false,
        searched_locations: searched,
        unsigned_internal_supported: true,
        guidance: "Windows SDK signtool.exe not installed on this machine. UnsignedInternal builds with Ed25519 + VERSIONINFO still work normally; install Windows 10/11 SDK or set FP_SIGNTOOL_PATH for Authenticode signing.".to_string(),
    }
}

/// Locates Windows SDK `signtool.exe` cleanly via filesystem probe (zero shell interpolation).
pub fn find_signtool_exe() -> Option<PathBuf> {
    discover_signing_toolchain().discovered_path
}

/// Checks whether a certificate Subject matches Publisher Profile identity
/// (never relies solely on `VERSIONINFO.CompanyName`, which is unauthenticated metadata).
pub fn verify_publisher_matches_cert_subject(
    publisher: Option<&fp_bundle_contract::PublisherIdentitySpec>,
    cert_subject: Option<&str>,
) -> (bool, String) {
    let Some(subj) = cert_subject.map(|s| s.trim()).filter(|s| !s.is_empty()) else {
        return (
            false,
            "No X.509 certificate subject present (VERSIONINFO CompanyName alone does not verify publisher identity).".to_string(),
        );
    };
    let Some(pub_spec) = publisher else {
        return (
            false,
            format!("Certificate subject '{}' present, but no PublisherIdentitySpec declared.", subj),
        );
    };
    let subj_lc = subj.to_ascii_lowercase();
    let org_lc = pub_spec.organization.trim().to_ascii_lowercase();
    let disp_lc = pub_spec.display_name.trim().to_ascii_lowercase();
    let matches = (!org_lc.is_empty() && subj_lc.contains(&org_lc))
        || (!disp_lc.is_empty() && subj_lc.contains(&disp_lc));
    if matches {
        (
            true,
            format!("Certificate subject '{}' matches declared publisher '{}'", subj, pub_spec.organization),
        )
    } else {
        (
            false,
            format!("PUBLISHER_CERT_SUBJECT_MISMATCH: cert subject '{}' does not match publisher organization '{}' / '{}'", subj, pub_spec.organization, pub_spec.display_name),
        )
    }
}

/// Creates a genuine X.509 RSA-2048 CodeSigning certificate directly inside `Cert:\CurrentUser\My`
/// using Windows DPAPI/CNG private key storage — ZERO private key files or PFX passwords written to disk!
pub fn create_self_signed_qa_cert(subject_cn: &str) -> Result<QaSelfSignedCertInfo> {
    let clean_cn: String = subject_cn
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.'))
        .take(64)
        .collect();
    if clean_cn.trim().is_empty() {
        bail!("INVALID_CERT_SUBJECT: subject CN cannot be empty");
    }

    let script = r#"
$subj = $env:FP_QA_CERT_SUBJECT
$cert = New-SelfSignedCertificate -Type CodeSigningCert -Subject ("CN=" + $subj) -KeyAlgorithm RSA -KeyLength 2048 -HashAlgorithm SHA256 -CertStoreLocation "Cert:\CurrentUser\My" -NotAfter (Get-Date).AddDays(90) -FriendlyName ("FarmerPro QA - " + $subj)
[PSCustomObject]@{
  subject = $cert.Subject
  thumbprint = $cert.Thumbprint
  storeLocation = 'Cert:\CurrentUser\My'
  notAfterUtc = $cert.NotAfter.ToUniversalTime().ToString('o')
} | ConvertTo-Json -Compress
"#;

    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("FP_QA_CERT_SUBJECT", clean_cn.trim())
        .output()
        .context("Failed launching powershell New-SelfSignedCertificate")?;
    if !out.status.success() {
        bail!(
            "QA_CERT_CREATE_FAILED: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let raw = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(raw.trim())
        .with_context(|| format!("Unexpected QA cert JSON output: {}", raw.trim()))?;
    let thumb = parsed["thumbprint"]
        .as_str()
        .context("Missing thumbprint in QA cert response")?
        .to_string();
    let subj = parsed["subject"].as_str().unwrap_or(&clean_cn).to_string();
    let not_after = parsed["notAfterUtc"].as_str().unwrap_or("").to_string();

    Ok(QaSelfSignedCertInfo {
        subject: subj,
        cert_ref: format!("CERT_STORE://CurrentUser/My/{}", thumb),
        thumbprint: thumb,
        store_location: r"Cert:\CurrentUser\My".to_string(),
        not_after_utc: not_after,
    })
}

fn parse_cert_store_thumbprint(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("CERT_STORE://CurrentUser/My/") {
        return Some(rest.trim().to_string());
    }
    if let Some(rest) = trimmed.strip_prefix("CERT_STORE://") {
        if let Some((_, last)) = rest.rsplit_once('/') {
            return Some(last.trim().to_string());
        }
    }
    if trimmed.len() == 40 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(trimmed.to_string());
    }
    None
}

/// Signs `exe_path` strictly AFTER all PE resources (`RT_GROUP_ICON`, `RT_VERSION`, `RT_RCDATA #7777`) are finalized.
/// Always verifies with Win32 `WinVerifyTrust` immediately after signing and enforces trust-policy truthfulness!
pub fn sign_pe_authenticode_and_verify(
    exe_path: &Path,
    req: &AuthenticodeSignRequest,
) -> Result<AuthenticodeVerificationReport> {
    match req.signing_mode {
        SigningMode::UnsignedInternal => {
            let report = verify_authenticode_pe(exe_path);
            Ok(report)
        }

        SigningMode::SelfSignedQa | SigningMode::PublicSigned => {
            let signtool = find_signtool_exe().ok_or_else(|| {
                anyhow::anyhow!("SIGNTOOL_MISSING: Windows SDK signtool.exe not found on host")
            })?;

            let cert_ref_str = req.cert_ref.as_deref().map(|s| s.trim()).unwrap_or("");
            if cert_ref_str.is_empty() {
                if req.signing_mode == SigningMode::PublicSigned {
                    bail!("PUBLIC_CERT_NOT_CONFIGURED: PublicSigned requires an external certificate reference (cert_ref)");
                }
                bail!("QA_CERT_NOT_CONFIGURED: SelfSignedQa requires an existing QA cert_ref or generated CertStore thumbprint");
            }

            let mut args: Vec<String> = vec![
                "sign".to_string(),
                "/fd".to_string(),
                "SHA256".to_string(),
            ];
            if let Some(ref desc) = req.description {
                if !desc.trim().is_empty() {
                    args.push("/d".to_string());
                    args.push(desc.trim().to_string());
                }
            }

            let mut use_pfx_env_pwd = false;
            if let Some(pfx_path) = cert_ref_str.strip_prefix("PFX://") {
                if !Path::new(pfx_path).is_file() {
                    bail!("AUTHENTICODE_CERT_FILE_MISSING: PFX not found at {}", pfx_path);
                }
                args.push("/f".to_string());
                args.push(pfx_path.to_string());
                if std::env::var("FP_AUTHENTICODE_PFX_PASSWORD").is_ok() {
                    use_pfx_env_pwd = true;
                }
            } else if let Some(sha1) = parse_cert_store_thumbprint(cert_ref_str) {
                args.push("/sha1".to_string());
                args.push(sha1);
                args.push("/s".to_string());
                args.push("My".to_string());
            } else {
                bail!(
                    "INVALID_CERT_REF: expected CERT_STORE://CurrentUser/My/<sha1>, 40-hex SHA1, or PFX://<path>"
                );
            }

            if let Some(ref ts) = req.timestamp_url {
                if !ts.trim().is_empty() {
                    args.push("/tr".to_string());
                    args.push(ts.trim().to_string());
                    args.push("/td".to_string());
                    args.push("SHA256".to_string());
                }
            }
            args.push(exe_path.to_string_lossy().to_string());

            let mut cmd = Command::new(&signtool);
            cmd.args(&args);
            if use_pfx_env_pwd {
                if let Ok(pwd) = std::env::var("FP_AUTHENTICODE_PFX_PASSWORD") {
                    cmd.arg("/p");
                    cmd.arg(pwd);
                }
            }
            let out = cmd
                .output()
                .with_context(|| format!("Failed executing {}", signtool.display()))?;
            if !out.status.success() {
                let err_raw = format!(
                    "{} {}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
                // Redact any accidental secret strings before surfacing errors
                let clean_err = fp_security::sanitize_error_message(err_raw.trim());
                bail!("AUTHENTICODE_SIGN_FAILED: {}", clean_err);
            }

            let report = verify_authenticode_pe(exe_path);
            if matches!(
                report.trust_status,
                AuthenticodeTrustStatus::TamperedOrInvalid
                    | AuthenticodeTrustStatus::Unsigned
                    | AuthenticodeTrustStatus::ExpiredOrTimestampError
            ) {
                bail!(
                    "AUTHENTICODE_VERIFY_POST_SIGN_FAILED ({:?}): {}",
                    report.trust_status,
                    report.detail
                );
            }
            if req.signing_mode == SigningMode::PublicSigned && !report.trusted_public_release {
                bail!(
                    "PUBLIC_SIGN_TRUST_FAILED: certificate signed binary cleanly but failed Windows public trust chain ({:?}: {}). Self-signed certificates cannot be marked TrustedPublicRelease.",
                    report.trust_status,
                    report.detail
                );
            }
            Ok(report)
        }
    }
}

//! Verification-only crate for FarmerPro Mini Auto-Tool bundles.
//!
//! ARCHITECTURAL GUARANTEE:
//! - This crate is linked by `fp-mini-runtime.exe`.
//! - It depends ONLY on `ed25519_dalek::{VerifyingKey, Signature, Verifier}` and `sha2::Sha256`.
//! - It contains ZERO signing functions (`SigningKey` is never used or imported here)
//!   and ZERO private key material.

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use fp_bundle_contract::{
    canonical_key_rotation_message, evaluate_bundle_compatibility, AuthenticodeTrustStatus,
    AuthenticodeVerificationReport, CompatibilityReport, KeyRotationDelegation, MiniToolUiSchema,
    PeVersionInfoSpec, ReleaseBundleManifest, RuntimeCompatibilityProfile,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

/// Computes lowercase hex SHA-256 of a byte slice
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

/// Decodes a hex string into raw bytes
pub fn hex_decode(hex_str: &str) -> Result<Vec<u8>> {
    let clean = hex_str.trim();
    if clean.len() % 2 != 0 {
        bail!("HEX_DECODE_ERROR: Odd length hex string ({})", clean.len());
    }
    let mut out = Vec::with_capacity(clean.len() / 2);
    for i in (0..clean.len()).step_by(2) {
        let byte = u8::from_str_radix(&clean[i..i + 2], 16)
            .with_context(|| format!("Invalid hex byte '{}'", &clean[i..i + 2]))?;
        out.push(byte);
    }
    Ok(out)
}

/// Encodes raw bytes as lowercase hex
pub fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

/// Validates that a relative path stays strictly inside `project_root` (blocks `..`, absolute paths, drive prefixes)
pub fn validate_safe_relative_path(project_root: &Path, rel_path: &str) -> Result<PathBuf> {
    if rel_path.trim().is_empty() {
        bail!("PATH_SECURITY_ERROR: Empty asset path is not allowed");
    }
    let p = Path::new(rel_path);
    if p.is_absolute() {
        bail!(
            "PATH_SECURITY_ERROR: Absolute path '{}' is forbidden in project bundle",
            rel_path
        );
    }
    for comp in p.components() {
        match comp {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!(
                    "PATH_SECURITY_ERROR: Path traversal component detected in '{}'",
                    rel_path
                );
            }
        }
    }
    Ok(project_root.join(p))
}

/// Canonical signing payload message constructed over the bundle's manifest and checksum table
pub fn canonical_signing_message(
    key_id: &str,
    manifest_sha256: &str,
    checksums_table_sha256: &str,
) -> Vec<u8> {
    format!(
        "FARMERPRO-BUNDLE-ED25519-V1|key_id={}|manifest_sha256={}|table_sha256={}",
        key_id, manifest_sha256, checksums_table_sha256
    )
    .into_bytes()
}

/// RFC 8032 Ed25519 Signed Bundle Envelope.
/// Separates:
/// 1. File Integrity (`file_checksums` + `checksums_table_sha256` via SHA-256)
/// 2. Publisher Authenticity (`signature_ed25519_hex` verified via RFC 8032 Ed25519 `VerifyingKey`)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedBundleEnvelope {
    pub algorithm: String,
    pub key_id: String,
    pub publisher_public_key_hex: String,
    pub manifest_sha256: String,
    pub checksums_table_sha256: String,
    pub file_checksums: BTreeMap<String, String>,
    pub signature_ed25519_hex: String,
    pub signed_at_utc: String,
}

impl SignedBundleEnvelope {
    /// Verifies the bundle using ONLY the trusted 32-byte Ed25519 Public Key (`VerifyingKey`).
    /// No private key exists anywhere in this crate or in `fp-mini-runtime`.
    pub fn verify_ed25519_with_public_key(
        &self,
        bundle_dir: &Path,
        trusted_public_key_hex: &str,
    ) -> Result<()> {
        if self.algorithm != "Ed25519-RFC8032-SHA256" {
            bail!(
                "UNSUPPORTED_SIGNATURE_ALGORITHM: Expected 'Ed25519-RFC8032-SHA256', got '{}'",
                self.algorithm
            );
        }

        // 1. Check that the envelope's public key matches the runtime's trusted public key
        if !self
            .publisher_public_key_hex
            .eq_ignore_ascii_case(trusted_public_key_hex.trim())
        {
            bail!(
                "ED25519_PUBKEY_UNTRUSTED: Bundle public key '{}' does not match trusted public key '{}'",
                self.publisher_public_key_hex,
                trusted_public_key_hex.trim()
            );
        }

        let pubkey_bytes_vec = hex_decode(&self.publisher_public_key_hex)
            .context("ED25519_PUBKEY_INVALID_HEX")?;
        if pubkey_bytes_vec.len() != 32 {
            bail!(
                "ED25519_PUBKEY_INVALID_LEN: Expected 32 bytes, got {}",
                pubkey_bytes_vec.len()
            );
        }
        let mut pubkey_arr = [0u8; 32];
        pubkey_arr.copy_from_slice(&pubkey_bytes_vec);
        let verifying_key = VerifyingKey::from_bytes(&pubkey_arr)
            .map_err(|e| anyhow::anyhow!("ED25519_PUBKEY_DECODE_ERROR: {}", e))?;

        let sig_bytes_vec = hex_decode(&self.signature_ed25519_hex)
            .context("ED25519_SIG_INVALID_HEX")?;
        if sig_bytes_vec.len() != 64 {
            bail!(
                "ED25519_SIG_INVALID_LEN: Expected 64 bytes, got {}",
                sig_bytes_vec.len()
            );
        }
        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&sig_bytes_vec);
        let signature = Signature::from_bytes(&sig_arr);

        // 2. Verify that checksums_table_sha256 matches the serialized file_checksums map
        let canonical_table = serde_json::to_string(&self.file_checksums)?;
        let actual_table_sha = sha256_hex(canonical_table.as_bytes());
        if actual_table_sha != self.checksums_table_sha256 {
            bail!(
                "INTEGRITY_TABLE_MISMATCH: checksums_table_sha256 '{}' != actual '{}'",
                self.checksums_table_sha256,
                actual_table_sha
            );
        }

        // 3. Verify RFC 8032 Ed25519 digital signature over the canonical signing message
        let msg = canonical_signing_message(
            &self.key_id,
            &self.manifest_sha256,
            &self.checksums_table_sha256,
        );
        verifying_key.verify(&msg, &signature).map_err(|e| {
            anyhow::anyhow!(
                "ED25519_SIGNATURE_INVALID: RFC 8032 Ed25519 signature verification failed ({}). Bundle was tampered with or forged!",
                e
            )
        })?;

        // 4. Verify manifest.fpbundle.json exists and matches manifest_sha256
        let manifest_path = bundle_dir.join("manifest.fpbundle.json");
        if !manifest_path.exists() {
            bail!("INTEGRITY_MISSING_MANIFEST: manifest.fpbundle.json is missing from bundle");
        }
        let manifest_bytes = std::fs::read(&manifest_path)?;
        let actual_manifest_sha = sha256_hex(&manifest_bytes);
        if actual_manifest_sha != self.manifest_sha256 {
            bail!(
                "INTEGRITY_MANIFEST_HASH_MISMATCH: manifest.fpbundle.json SHA-256 '{}' != signed '{}'",
                actual_manifest_sha,
                self.manifest_sha256
            );
        }

        // 5. Verify every file in file_checksums exists and matches its signed SHA-256
        for (rel_path, expected_sha) in &self.file_checksums {
            let safe_path = validate_safe_relative_path(bundle_dir, rel_path)?;
            if !safe_path.exists() {
                bail!(
                    "INTEGRITY_MISSING_FILE: Required bundle file '{}' is missing on disk",
                    rel_path
                );
            }
            let bytes = std::fs::read(&safe_path)?;
            let actual_sha = sha256_hex(&bytes);
            if &actual_sha != expected_sha {
                bail!(
                    "INTEGRITY_HASH_MISMATCH: File '{}' SHA-256 mismatch (expected {}, got {})",
                    rel_path,
                    expected_sha,
                    actual_sha
                );
            }
        }

        Ok(())
    }
}

/// Verifies compatibility across all 5 dimensions: Runtime, Workflow Schema, UI Schema, Module API, SQLite Schema.
pub fn verify_bundle_compatibility_gate(
    manifest: &ReleaseBundleManifest,
    ui_schema: &MiniToolUiSchema,
    workflow_schema_versions: &[u32],
    profile: &RuntimeCompatibilityProfile,
) -> Result<CompatibilityReport> {
    if ui_schema.project_slug != manifest.project.slug {
        bail!(
            "COMPATIBILITY_ERROR: UI schema slug '{}' != manifest project slug '{}'",
            ui_schema.project_slug,
            manifest.project.slug
        );
    }
    let report = evaluate_bundle_compatibility(
        &manifest.compatibility,
        ui_schema.schema_version,
        workflow_schema_versions,
        profile,
    );
    if !report.compatible {
        bail!(
            "COMPATIBILITY_ERROR: {}",
            report.blocking_reasons.join("; ")
        );
    }
    Ok(report)
}

/// Resource ID for the PE-embedded Ed25519 Root of Trust Public Key (`RT_RCDATA = 10`, ID `7777`)
pub const PE_ED25519_PUBKEY_RESOURCE_ID: usize = 7777;
pub const PE_RT_RCDATA_TYPE_ID: usize = 10;

#[cfg(windows)]
mod win_pe_trust {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    type HANDLE = *mut core::ffi::c_void;
    type HMODULE = *mut core::ffi::c_void;
    type HRSRC = *mut core::ffi::c_void;
    type HGLOBAL = *mut core::ffi::c_void;
    type BOOL = i32;
    type DWORD = u32;
    type LPCWSTR = *const u16;

    const LOAD_LIBRARY_AS_DATAFILE: DWORD = 0x00000002;

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryExW(lpLibFileName: LPCWSTR, hFile: HANDLE, dwFlags: DWORD) -> HMODULE;
        fn FindResourceW(hModule: HMODULE, lpName: LPCWSTR, lpType: LPCWSTR) -> HRSRC;
        fn LoadResource(hModule: HMODULE, hResInfo: HRSRC) -> HGLOBAL;
        fn LockResource(hResData: HGLOBAL) -> *const u8;
        fn SizeofResource(hModule: HMODULE, hResInfo: HRSRC) -> DWORD;
        fn FreeLibrary(hLibModule: HMODULE) -> BOOL;
    }

    fn to_wide(s: &OsStr) -> Vec<u16> {
        s.encode_wide().chain(std::iter::once(0)).collect()
    }

    const fn make_int_resource(id: usize) -> LPCWSTR {
        id as LPCWSTR
    }

    pub fn read_pe_pubkey_win32(exe_path: &Path) -> Result<String> {
        let exe_wide = to_wide(exe_path.as_os_str());
        unsafe {
            let h_mod = LoadLibraryExW(
                exe_wide.as_ptr(),
                core::ptr::null_mut(),
                LOAD_LIBRARY_AS_DATAFILE,
            );
            if h_mod.is_null() {
                bail!(
                    "PE_ROOT_OF_TRUST_LOAD_ERROR: LoadLibraryExW failed on {}",
                    exe_path.display()
                );
            }

            let h_res = FindResourceW(
                h_mod,
                make_int_resource(PE_ED25519_PUBKEY_RESOURCE_ID),
                make_int_resource(PE_RT_RCDATA_TYPE_ID),
            );
            if h_res.is_null() {
                let _ = FreeLibrary(h_mod);
                bail!(
                    "PE_ROOT_OF_TRUST_MISSING: Executable '{}' does not contain embedded RT_RCDATA #{} Ed25519 public key resource",
                    exe_path.display(),
                    PE_ED25519_PUBKEY_RESOURCE_ID
                );
            }

            let size = SizeofResource(h_mod, h_res);
            let h_glob = LoadResource(h_mod, h_res);
            if h_glob.is_null() || size == 0 {
                let _ = FreeLibrary(h_mod);
                bail!(
                    "PE_ROOT_OF_TRUST_EMPTY: Failed to load RT_RCDATA #{} from {}",
                    PE_ED25519_PUBKEY_RESOURCE_ID,
                    exe_path.display()
                );
            }

            let ptr = LockResource(h_glob);
            if ptr.is_null() {
                let _ = FreeLibrary(h_mod);
                bail!("PE_ROOT_OF_TRUST_LOCK_ERROR: LockResource returned null");
            }

            let slice = core::slice::from_raw_parts(ptr, size as usize);
            let raw_str = std::str::from_utf8(slice)
                .map_err(|e| {
                    let _ = FreeLibrary(h_mod);
                    anyhow::anyhow!("PE_ROOT_OF_TRUST_UTF8_ERROR: {}", e)
                })?
                .trim()
                .to_string();

            let _ = FreeLibrary(h_mod);

            // Validate it is a valid 32-byte RFC 8032 Ed25519 VerifyingKey
            let decoded = hex_decode(&raw_str).context("PE_ROOT_OF_TRUST_INVALID_HEX")?;
            if decoded.len() != 32 {
                bail!(
                    "PE_ROOT_OF_TRUST_INVALID_LEN: Expected 32-byte Ed25519 key (64 hex chars), got {} bytes",
                    decoded.len()
                );
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&decoded);
            let _ = VerifyingKey::from_bytes(&arr)
                .map_err(|e| anyhow::anyhow!("PE_ROOT_OF_TRUST_INVALID_POINT: {}", e))?;

            Ok(raw_str)
        }
    }
}

/// Reads the pinned 32-byte Ed25519 Public Key (64 hex chars) directly from the PE `.rsrc`
/// section (`RT_RCDATA = 10`, ID `7777`) of the specified `.exe` file.
/// This ensures an attacker cannot swap the trusted public key by editing files in `bundle/`.
pub fn read_pe_embedded_pubkey_from_exe(exe_path: &Path) -> Result<String> {
    #[cfg(windows)]
    {
        win_pe_trust::read_pe_pubkey_win32(exe_path)
    }
    #[cfg(not(windows))]
    {
        let _ = exe_path;
        bail!("PE resource reading is only supported on Windows targets")
    }
}

/// Reads the pinned 32-byte Ed25519 Public Key from the currently running Mini Tool `.exe` binary.
pub fn read_pe_embedded_pubkey_from_current_process() -> Result<String> {
    let exe = std::env::current_exe().context("Failed to resolve current_exe path")?;
    read_pe_embedded_pubkey_from_exe(&exe)
}

/// Verifies a forward-signed Ed25519 `KeyRotationDelegation` certificate under `expected_old_pubkey_hex`.
pub fn verify_key_rotation_delegation(
    delegation: &KeyRotationDelegation,
    expected_old_pubkey_hex: &str,
    expected_publisher_id: Option<&str>,
    expected_product_slug: Option<&str>,
    now_utc: Option<DateTime<Utc>>,
) -> Result<()> {
    if !delegation
        .old_pubkey_hex
        .trim()
        .eq_ignore_ascii_case(expected_old_pubkey_hex.trim())
    {
        bail!(
            "KEY_ROTATION_OLD_PUBKEY_MISMATCH: delegation old_pubkey '{}' != expected '{}'",
            delegation.old_pubkey_hex,
            expected_old_pubkey_hex
        );
    }
    if delegation
        .old_pubkey_hex
        .trim()
        .eq_ignore_ascii_case(delegation.new_pubkey_hex.trim())
    {
        bail!("KEY_ROTATION_NOOP_KEY: new_pubkey_hex must differ from old_pubkey_hex");
    }
    if let Some(exp_pub) = expected_publisher_id {
        if delegation.publisher_id.trim() != exp_pub.trim() {
            bail!(
                "KEY_ROTATION_PUBLISHER_MISMATCH: '{}' != '{}'",
                delegation.publisher_id,
                exp_pub
            );
        }
    }
    if let Some(exp_slug) = expected_product_slug {
        if delegation.product_slug.trim() != exp_slug.trim() {
            bail!(
                "KEY_ROTATION_PRODUCT_MISMATCH: '{}' != '{}'",
                delegation.product_slug,
                exp_slug
            );
        }
    }
    if delegation.nonce.trim().len() < 8 {
        bail!("KEY_ROTATION_INVALID_NONCE: nonce missing or too short");
    }

    let iat = DateTime::parse_from_rfc3339(&delegation.issued_at_utc)
        .context("KEY_ROTATION_INVALID_IAT")?
        .with_timezone(&Utc);
    let exp = DateTime::parse_from_rfc3339(&delegation.expires_at_utc)
        .context("KEY_ROTATION_INVALID_EXP")?
        .with_timezone(&Utc);
    let now = now_utc.unwrap_or_else(Utc::now);
    if exp <= iat {
        bail!("KEY_ROTATION_INVALID_WINDOW: expiresAtUtc <= issuedAtUtc");
    }
    if now > exp {
        bail!("KEY_ROTATION_EXPIRED: delegation expired at {}", exp.to_rfc3339());
    }
    if now < iat - chrono::Duration::minutes(5) {
        bail!("KEY_ROTATION_NOT_YET_VALID: issuedAtUtc {} is in the future", iat.to_rfc3339());
    }

    let old_bytes = hex_decode(&delegation.old_pubkey_hex).context("KEY_ROTATION_OLD_HEX")?;
    if old_bytes.len() != 32 {
        bail!("KEY_ROTATION_OLD_LEN: expected 32 bytes");
    }
    let old_arr: [u8; 32] = old_bytes.try_into().unwrap();
    let old_vk = VerifyingKey::from_bytes(&old_arr)
        .map_err(|e| anyhow::anyhow!("KEY_ROTATION_OLD_VK: {}", e))?;

    let new_bytes = hex_decode(&delegation.new_pubkey_hex).context("KEY_ROTATION_NEW_HEX")?;
    if new_bytes.len() != 32 {
        bail!("KEY_ROTATION_NEW_LEN: expected 32 bytes");
    }
    let new_arr: [u8; 32] = new_bytes.try_into().unwrap();
    let _ = VerifyingKey::from_bytes(&new_arr)
        .map_err(|e| anyhow::anyhow!("KEY_ROTATION_NEW_VK: {}", e))?;

    let sig_bytes = hex_decode(&delegation.signature_by_old_key_hex).context("KEY_ROTATION_SIG_HEX")?;
    if sig_bytes.len() != 64 {
        bail!("KEY_ROTATION_SIG_LEN: expected 64 bytes");
    }
    let sig_arr: [u8; 64] = sig_bytes.try_into().unwrap();
    let sig = Signature::from_bytes(&sig_arr);

    let msg = canonical_key_rotation_message(
        &delegation.publisher_id,
        &delegation.product_slug,
        &delegation.old_pubkey_hex,
        &delegation.new_pubkey_hex,
        &delegation.issued_at_utc,
        &delegation.expires_at_utc,
        &delegation.nonce,
    );
    old_vk.verify(&msg, &sig).map_err(|e| {
        anyhow::anyhow!(
            "KEY_ROTATION_SIGNATURE_INVALID: delegation not validly signed by trusted old key ({})",
            e
        )
    })?;
    Ok(())
}

#[cfg(windows)]
mod win_authenticode_and_version {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    type HANDLE = *mut core::ffi::c_void;
    type HWND = *mut core::ffi::c_void;
    type DWORD = u32;
    type LONG = i32;
    type BOOL = i32;
    type LPCWSTR = *const u16;
    type LPVOID = *mut core::ffi::c_void;

    #[repr(C)]
    struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    #[repr(C)]
    struct WintrustFileInfo {
        cb_struct: DWORD,
        pcwsz_file_path: LPCWSTR,
        h_file: HANDLE,
        pg_known_subject: *const Guid,
    }

    #[repr(C)]
    struct WintrustData {
        cb_struct: DWORD,
        p_policy_callback_data: LPVOID,
        p_sip_client_data: LPVOID,
        dw_ui_choice: DWORD,
        fdw_revocation_checks: DWORD,
        dw_union_choice: DWORD,
        p_file: *mut WintrustFileInfo,
        dw_state_action: DWORD,
        h_wvt_state_data: HANDLE,
        pwsz_url_reference: LPCWSTR,
        dw_prov_flags: DWORD,
        dw_ui_context: DWORD,
        p_signature_settings: LPVOID,
    }

    #[repr(C)]
    struct CertContext {
        dw_cert_encoding_type: DWORD,
        pb_cert_encoded: *const u8,
        cb_cert_encoded: DWORD,
        p_cert_info: LPVOID,
        h_cert_store: LPVOID,
    }

    const WTD_UI_NONE: DWORD = 2;
    const WTD_REVOKE_NONE: DWORD = 0;
    const WTD_CHOICE_FILE: DWORD = 1;
    const WTD_STATEACTION_VERIFY: DWORD = 1;
    const WTD_STATEACTION_CLOSE: DWORD = 2;
    const WTD_CACHE_ONLY_URL_RETRIEVAL: DWORD = 0x0000_1000;

    const CERT_QUERY_OBJECT_FILE: DWORD = 1;
    const CERT_QUERY_CONTENT_FLAG_PKCS7_SIGNED_EMBED: DWORD = 1 << 10;
    const CERT_QUERY_FORMAT_FLAG_BINARY: DWORD = 1 << 1;
    const CERT_NAME_SIMPLE_DISPLAY_TYPE: DWORD = 4;
    const CERT_NAME_ISSUER_FLAG: DWORD = 1;
    const CERT_SHA1_HASH_PROP_ID: DWORD = 3;

    #[link(name = "wintrust")]
    unsafe extern "system" {
        fn WinVerifyTrust(hwnd: HWND, pgActionID: *mut Guid, pWVTData: *mut WintrustData) -> LONG;
    }

    #[link(name = "crypt32")]
    unsafe extern "system" {
        fn CryptQueryObject(
            dwObjectType: DWORD,
            pvObject: LPVOID,
            dwExpectedContentTypeFlags: DWORD,
            dwExpectedFormatTypeFlags: DWORD,
            dwFlags: DWORD,
            pdwMsgAndCertEncodingType: *mut DWORD,
            pdwContentType: *mut DWORD,
            pdwFormatType: *mut DWORD,
            phCertStore: *mut HANDLE,
            phMsg: *mut LPVOID,
            ppvContext: *mut LPVOID,
        ) -> BOOL;
        fn CertEnumCertificatesInStore(
            hCertStore: HANDLE,
            pPrevCertContext: *const CertContext,
        ) -> *const CertContext;
        fn CertGetNameStringW(
            pCertContext: *const CertContext,
            dwType: DWORD,
            dwFlags: DWORD,
            pvTypePara: LPVOID,
            pszNameString: *mut u16,
            cchNameString: DWORD,
        ) -> DWORD;
        fn CertGetCertificateContextProperty(
            pCertContext: *const CertContext,
            dwPropId: DWORD,
            pvData: LPVOID,
            pcbData: *mut DWORD,
        ) -> BOOL;
        fn CertFreeCertificateContext(pCertContext: *const CertContext) -> BOOL;
        fn CertCloseStore(hCertStore: HANDLE, dwFlags: DWORD) -> BOOL;
        fn CryptMsgClose(hCryptMsg: LPVOID) -> BOOL;
    }

    #[link(name = "version")]
    unsafe extern "system" {
        fn GetFileVersionInfoSizeW(lptstrFilename: LPCWSTR, lpdwHandle: *mut DWORD) -> DWORD;
        fn GetFileVersionInfoW(
            lptstrFilename: LPCWSTR,
            dwHandle: DWORD,
            dwLen: DWORD,
            lpData: LPVOID,
        ) -> BOOL;
        fn VerQueryValueW(
            pBlock: LPVOID,
            lpSubBlock: LPCWSTR,
            lplpBuffer: *mut LPVOID,
            puLen: *mut u32,
        ) -> BOOL;
    }

    fn to_wide(s: &OsStr) -> Vec<u16> {
        s.encode_wide().chain(std::iter::once(0)).collect()
    }

    fn query_embedded_signer_cert_info(exe_wide: &[u16]) -> (Option<String>, Option<String>, Option<String>) {
        unsafe {
            let mut h_store: HANDLE = std::ptr::null_mut();
            let mut h_msg: LPVOID = std::ptr::null_mut();
            let ok = CryptQueryObject(
                CERT_QUERY_OBJECT_FILE,
                exe_wide.as_ptr() as LPVOID,
                CERT_QUERY_CONTENT_FLAG_PKCS7_SIGNED_EMBED,
                CERT_QUERY_FORMAT_FLAG_BINARY,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut h_store,
                &mut h_msg,
                std::ptr::null_mut(),
            );
            if ok == 0 || h_store.is_null() {
                return (None, None, None);
            }

            let cert = CertEnumCertificatesInStore(h_store, std::ptr::null());
            let res = if !cert.is_null() {
                let mut subj_buf = [0u16; 256];
                let slen = CertGetNameStringW(
                    cert,
                    CERT_NAME_SIMPLE_DISPLAY_TYPE,
                    0,
                    std::ptr::null_mut(),
                    subj_buf.as_mut_ptr(),
                    subj_buf.len() as DWORD,
                );
                let subject = if slen > 1 {
                    Some(String::from_utf16_lossy(&subj_buf[..(slen as usize - 1)]))
                } else {
                    None
                };

                let mut iss_buf = [0u16; 256];
                let ilen = CertGetNameStringW(
                    cert,
                    CERT_NAME_SIMPLE_DISPLAY_TYPE,
                    CERT_NAME_ISSUER_FLAG,
                    std::ptr::null_mut(),
                    iss_buf.as_mut_ptr(),
                    iss_buf.len() as DWORD,
                );
                let issuer = if ilen > 1 {
                    Some(String::from_utf16_lossy(&iss_buf[..(ilen as usize - 1)]))
                } else {
                    None
                };

                let mut hash_buf = [0u8; 20];
                let mut hash_len: DWORD = 20;
                let thumbprint = if CertGetCertificateContextProperty(
                    cert,
                    CERT_SHA1_HASH_PROP_ID,
                    hash_buf.as_mut_ptr() as LPVOID,
                    &mut hash_len,
                ) != 0
                {
                    Some(hex_encode(&hash_buf[..(hash_len as usize)]).to_ascii_uppercase())
                } else {
                    None
                };

                let _ = CertFreeCertificateContext(cert);
                (subject, issuer, thumbprint)
            } else {
                (None, None, None)
            };

            if !h_msg.is_null() {
                let _ = CryptMsgClose(h_msg);
            }
            let _ = CertCloseStore(h_store, 0);
            res
        }
    }

    pub fn verify_authenticode_win32(exe_path: &Path) -> AuthenticodeVerificationReport {
        if !exe_path.exists() {
            return AuthenticodeVerificationReport {
                signed: false,
                trust_status: AuthenticodeTrustStatus::TamperedOrInvalid,
                trusted_public_release: false,
                win_trust_error_code: 2,
                signer_subject: None,
                signer_issuer: None,
                signer_thumbprint: None,
                timestamp_status: "MISSING_FILE".to_string(),
                detail: format!("Target binary missing: {}", exe_path.display()),
            };
        }

        let wide_path = to_wide(exe_path.as_os_str());
        let (subject, issuer, thumbprint) = query_embedded_signer_cert_info(&wide_path);

        let mut action_guid = Guid {
            data1: 0x00AA_C56B,
            data2: 0xCD44,
            data3: 0x11d0,
            data4: [0x8C, 0xC2, 0x00, 0xC0, 0x4F, 0xC2, 0x95, 0xEE],
        };
        let mut file_info = WintrustFileInfo {
            cb_struct: std::mem::size_of::<WintrustFileInfo>() as DWORD,
            pcwsz_file_path: wide_path.as_ptr(),
            h_file: std::ptr::null_mut(),
            pg_known_subject: std::ptr::null(),
        };
        let mut wvt = WintrustData {
            cb_struct: std::mem::size_of::<WintrustData>() as DWORD,
            p_policy_callback_data: std::ptr::null_mut(),
            p_sip_client_data: std::ptr::null_mut(),
            dw_ui_choice: WTD_UI_NONE,
            fdw_revocation_checks: WTD_REVOKE_NONE,
            dw_union_choice: WTD_CHOICE_FILE,
            p_file: &mut file_info,
            dw_state_action: WTD_STATEACTION_VERIFY,
            h_wvt_state_data: std::ptr::null_mut(),
            pwsz_url_reference: std::ptr::null(),
            dw_prov_flags: WTD_CACHE_ONLY_URL_RETRIEVAL,
            dw_ui_context: 0,
            p_signature_settings: std::ptr::null_mut(),
        };

        let status = unsafe { WinVerifyTrust(std::ptr::null_mut(), &mut action_guid, &mut wvt) } as u32;
        wvt.dw_state_action = WTD_STATEACTION_CLOSE;
        unsafe {
            let _ = WinVerifyTrust(std::ptr::null_mut(), &mut action_guid, &mut wvt);
        }

        const TRUST_E_NOSIGNATURE: u32 = 0x800B_0100;
        const TRUST_E_EXPLICIT_DISTRUST: u32 = 0x800B_0111;
        const CERT_E_UNTRUSTEDROOT: u32 = 0x800B_0109;
        const CERT_E_CHAINING: u32 = 0x800B_010A;
        const CERT_E_EXPIRED: u32 = 0x800B_0101;
        const TRUST_E_TIME_STAMP: u32 = 0x8009_6005;
        const TRUST_E_BAD_DIGEST: u32 = 0x8009_6010;
        const CRYPT_E_NO_MATCH: u32 = 0x8009_2009;

        match status {
            0 => AuthenticodeVerificationReport {
                signed: true,
                trust_status: AuthenticodeTrustStatus::TrustedPublicChain,
                trusted_public_release: true,
                win_trust_error_code: 0,
                signer_subject: subject,
                signer_issuer: issuer,
                signer_thumbprint: thumbprint,
                timestamp_status: "VALID_OR_UNCHECKED".to_string(),
                detail: "WinVerifyTrust passed (ERROR_SUCCESS): valid Authenticode signature & trusted Windows chain.".to_string(),
            },
            TRUST_E_NOSIGNATURE => AuthenticodeVerificationReport {
                signed: false,
                trust_status: AuthenticodeTrustStatus::Unsigned,
                trusted_public_release: false,
                win_trust_error_code: status,
                signer_subject: None,
                signer_issuer: None,
                signer_thumbprint: None,
                timestamp_status: "NONE".to_string(),
                detail: "Unsigned Internal Build (TRUST_E_NOSIGNATURE 0x800B0100).".to_string(),
            },
            CERT_E_UNTRUSTEDROOT | CERT_E_CHAINING => AuthenticodeVerificationReport {
                signed: true,
                trust_status: AuthenticodeTrustStatus::SelfSignedUntrustedRoot,
                trusted_public_release: false,
                win_trust_error_code: status,
                signer_subject: subject,
                signer_issuer: issuer,
                signer_thumbprint: thumbprint,
                timestamp_status: "NONE_OR_QA".to_string(),
                detail: format!(
                    "Self-Signed / Untrusted Root (0x{:08X}): Authenticode PE hash intact, NOT trusted as Public Release.",
                    status
                ),
            },
            CERT_E_EXPIRED | TRUST_E_TIME_STAMP => AuthenticodeVerificationReport {
                signed: true,
                trust_status: AuthenticodeTrustStatus::ExpiredOrTimestampError,
                trusted_public_release: false,
                win_trust_error_code: status,
                signer_subject: subject,
                signer_issuer: issuer,
                signer_thumbprint: thumbprint,
                timestamp_status: format!("EXPIRED_OR_TIMESTAMP_FAIL (0x{:08X})", status),
                detail: format!("Certificate expired or timestamp failure (0x{:08X}).", status),
            },
            TRUST_E_BAD_DIGEST | TRUST_E_EXPLICIT_DISTRUST | CRYPT_E_NO_MATCH | _ => AuthenticodeVerificationReport {
                signed: subject.is_some() || thumbprint.is_some(),
                trust_status: AuthenticodeTrustStatus::TamperedOrInvalid,
                trusted_public_release: false,
                win_trust_error_code: status,
                signer_subject: subject,
                signer_issuer: issuer,
                signer_thumbprint: thumbprint,
                timestamp_status: format!("INVALID (0x{:08X})", status),
                detail: format!(
                    "AUTHENTICODE_INVALID (0x{:08X}): binary modified post-signing or signature corrupt!",
                    status
                ),
            },
        }
    }

    pub fn read_pe_version_info_win32(exe_path: &Path) -> Result<PeVersionInfoSpec> {
        let wide_path = to_wide(exe_path.as_os_str());
        let mut handle: DWORD = 0;
        let sz = unsafe { GetFileVersionInfoSizeW(wide_path.as_ptr(), &mut handle) };
        if sz == 0 {
            bail!("PE_VERSIONINFO_MISSING: GetFileVersionInfoSizeW returned 0 for {}", exe_path.display());
        }
        let mut buf = vec![0u8; sz as usize];
        let ok = unsafe { GetFileVersionInfoW(wide_path.as_ptr(), handle, sz, buf.as_mut_ptr() as LPVOID) };
        if ok == 0 {
            bail!("PE_VERSIONINFO_READ_FAILED on {}", exe_path.display());
        }

        let mut query_str = |key: &str| -> String {
            for lang_cp in ["040904B0", "040904b0", "000004B0"] {
                let sub = format!("\\StringFileInfo\\{}\\{}", lang_cp, key);
                let wsub: Vec<u16> = OsStr::new(&sub).encode_wide().chain(std::iter::once(0)).collect();
                let mut ptr: LPVOID = std::ptr::null_mut();
                let mut len: u32 = 0;
                let ok_q = unsafe { VerQueryValueW(buf.as_mut_ptr() as LPVOID, wsub.as_ptr(), &mut ptr, &mut len) };
                if ok_q != 0 && !ptr.is_null() && len > 0 {
                    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u16, len as usize) };
                    let s = String::from_utf16_lossy(slice)
                        .trim_end_matches('\0')
                        .to_string();
                    if !s.is_empty() {
                        return s;
                    }
                }
            }
            String::new()
        };

        Ok(PeVersionInfoSpec {
            company_name: query_str("CompanyName"),
            file_description: query_str("FileDescription"),
            file_version: query_str("FileVersion"),
            internal_name: query_str("InternalName"),
            legal_copyright: query_str("LegalCopyright"),
            original_filename: query_str("OriginalFilename"),
            product_name: query_str("ProductName"),
            product_version: query_str("ProductVersion"),
        })
    }
}

/// Verifies Windows Authenticode signature & trust chain natively via Win32 `WinVerifyTrust`.
pub fn verify_authenticode_pe(exe_path: &Path) -> AuthenticodeVerificationReport {
    #[cfg(windows)]
    {
        win_authenticode_and_version::verify_authenticode_win32(exe_path)
    }
    #[cfg(not(windows))]
    {
        let _ = exe_path;
        AuthenticodeVerificationReport {
            signed: false,
            trust_status: AuthenticodeTrustStatus::Unsigned,
            trusted_public_release: false,
            win_trust_error_code: 0,
            signer_subject: None,
            signer_issuer: None,
            signer_thumbprint: None,
            timestamp_status: "UNSUPPORTED_OS".to_string(),
            detail: "Authenticode verification requires Windows".to_string(),
        }
    }
}

/// Reads Windows PE `RT_VERSION` string fields (`CompanyName`, `ProductName`, etc.) natively via `version.dll`.
pub fn read_pe_version_info_from_exe(exe_path: &Path) -> Result<PeVersionInfoSpec> {
    #[cfg(windows)]
    {
        win_authenticode_and_version::read_pe_version_info_win32(exe_path)
    }
    #[cfg(not(windows))]
    {
        let _ = exe_path;
        bail!("PE VERSIONINFO reading requires Windows")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rfc8032_section_7_1_official_ed25519_test_vectors() {
        // IETF RFC 8032 Section 7.1 TEST 1 (Empty message "")
        let pub_hex_1 = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let sig_hex_1 = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";
        let pub_bytes = hex_decode(pub_hex_1).unwrap();
        let sig_bytes = hex_decode(sig_hex_1).unwrap();
        let vk = VerifyingKey::from_bytes(pub_bytes.as_slice().try_into().unwrap()).unwrap();
        let sig = Signature::from_bytes(sig_bytes.as_slice().try_into().unwrap());
        assert!(vk.verify(b"", &sig).is_ok());

        // IETF RFC 8032 Section 7.1 TEST 2 (1-byte message 0x72)
        let pub_hex_2 = "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";
        let sig_hex_2 = "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00";
        let vk2 = VerifyingKey::from_bytes(
            hex_decode(pub_hex_2).unwrap().as_slice().try_into().unwrap(),
        )
        .unwrap();
        let sig2 = Signature::from_bytes(
            hex_decode(sig_hex_2).unwrap().as_slice().try_into().unwrap(),
        );
        assert!(vk2.verify(&[0x72], &sig2).is_ok());
        assert!(vk2.verify(&[0x73], &sig2).is_err());
    }

    #[test]
    fn test_path_traversal_guard_blocks_escape() {
        let root = Path::new(r"C:\FarmerPro\Projects\jx2-mobile");
        assert!(validate_safe_relative_path(root, "templates/btn_login.png").is_ok());
        assert!(validate_safe_relative_path(root, "workflows/wf_main.json").is_ok());

        // Must block parent traversal, absolute paths, drive prefixes, and empty paths
        assert!(validate_safe_relative_path(root, "../secret.txt").is_err());
        assert!(validate_safe_relative_path(root, "templates/../../outside.png").is_err());
        assert!(validate_safe_relative_path(root, r"C:\Windows\System32\cmd.exe").is_err());
        assert!(validate_safe_relative_path(root, "/etc/passwd").is_err());
        assert!(validate_safe_relative_path(root, "   ").is_err());
    }

    #[test]
    fn test_five_dimension_compatibility_and_downgrade_guards() {
        use fp_bundle_contract::{
            check_sqlite_downgrade_compatibility, CompatibilityVersions, GameProjectManifest,
            MiniLayoutPreset, SemVer,
        };
        let manifest = ReleaseBundleManifest {
            compatibility: CompatibilityVersions::default(),
            project: GameProjectManifest {
                compatibility: CompatibilityVersions::default(),
                project_id: "p-compat".to_string(),
                slug: "compat-game".to_string(),
                name: "Compat Game".to_string(),
                game_package_id: "com.compat.game".to_string(),
                version: "1.0.0".to_string(),
                exe_name: "Auto_Compat.exe".to_string(),
                window_title: "Compat".to_string(),
                icon_rel_path: "icon.ico".to_string(),
                target_resolution: (1280, 720),
                packaged_modules: vec![],
                workflow_files: vec![],
                template_files: vec![],
                bundle_adb: false,
            },
            module_states: vec![],
            workflows_count: 1,
            templates_count: 0,
            built_at_utc: "2026-11-22T00:00:00Z".to_string(),
            publisher: None,
            signing: None,
        };
        let ui = MiniToolUiSchema {
            schema_version: 1,
            project_slug: "compat-game".to_string(),
            window_title: "Compat".to_string(),
            version_badge: "v1.0.0".to_string(),
            theme_accent_hex: "#1878F5".to_string(),
            header_bg_hex: "#121E2A".to_string(),
            layout_preset: MiniLayoutPreset::Compact1Col,
            default_window_size: (680, 480),
            sections_order: vec![],
            hidden_components: vec![],
            device_columns: vec![],
            task_categories: vec![],
            tasks: vec![],
            utilities: vec![],
            initial_runtime_vars: Default::default(),
        };
        let profile = RuntimeCompatibilityProfile::default();
        assert!(verify_bundle_compatibility_gate(&manifest, &ui, &[1], &profile).is_ok());

        // 1) Breaking Runtime Version requirement
        let mut bad_rt = manifest.clone();
        bad_rt.compatibility.min_runtime_version = "9.9.0".to_string();
        assert!(verify_bundle_compatibility_gate(&bad_rt, &ui, &[1], &profile)
            .unwrap_err()
            .to_string()
            .contains("RUNTIME_VERSION_INCOMPATIBLE"));

        // 2) Unsupported Workflow Schema Version
        assert!(verify_bundle_compatibility_gate(&manifest, &ui, &[99], &profile)
            .unwrap_err()
            .to_string()
            .contains("WORKFLOW_SCHEMA_INCOMPATIBLE"));

        // 3) Unsupported UI Schema Version
        let mut bad_ui = ui.clone();
        bad_ui.schema_version = 7;
        assert!(verify_bundle_compatibility_gate(&manifest, &bad_ui, &[1], &profile)
            .unwrap_err()
            .to_string()
            .contains("UI_SCHEMA_INCOMPATIBLE"));

        // 4) Unsupported Module API Major Version
        let mut bad_mod = manifest.clone();
        bad_mod.compatibility.module_api_version = "2.0.0".to_string();
        assert!(verify_bundle_compatibility_gate(&bad_mod, &ui, &[1], &profile)
            .unwrap_err()
            .to_string()
            .contains("MODULE_API_INCOMPATIBLE"));

        // 5) Unsupported SQLite Schema Version & Downgrade Guard
        let mut bad_sql = manifest.clone();
        bad_sql.compatibility.sqlite_schema_version = 99;
        assert!(verify_bundle_compatibility_gate(&bad_sql, &ui, &[1], &profile)
            .unwrap_err()
            .to_string()
            .contains("SQLITE_SCHEMA_INCOMPATIBLE"));
        assert!(check_sqlite_downgrade_compatibility(4, 3)
            .unwrap_err()
            .to_string()
            .contains("SQLITE_DOWNGRADE_INCOMPATIBLE"));
        assert!(check_sqlite_downgrade_compatibility(3, 4).is_ok());
        assert!(SemVer::parse("1.1.0").unwrap() > SemVer::parse("1.0.0").unwrap());
    }
}

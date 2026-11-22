//! Verification-only crate for FarmerPro Mini Auto-Tool bundles.
//!
//! ARCHITECTURAL GUARANTEE:
//! - This crate is linked by `fp-mini-runtime.exe`.
//! - It depends ONLY on `ed25519_dalek::{VerifyingKey, Signature, Verifier}` and `sha2::Sha256`.
//! - It contains ZERO signing functions (`SigningKey` is never used or imported here)
//!   and ZERO private key material.

use anyhow::{bail, Context, Result};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use fp_bundle_contract::{
    evaluate_bundle_compatibility, CompatibilityReport, MiniToolUiSchema, ReleaseBundleManifest,
    RuntimeCompatibilityProfile,
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

pub mod dpapi;
pub mod keystore;
pub mod wincred;

use anyhow::{bail, Result};
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub use keystore::{
    PortableProducerKeyRecoveryBundle, ProducerKeyBackupBundle, ProducerKeyMetadata,
    ProducerKeystore,
};
pub use wincred::{
    parse_cred_uri_to_target, validate_segment, CredentialMetadata, SecretNamespace,
    CRED_MGR_URI_PREFIX,
};

pub struct SecretRedactor {
    pattern: Regex,
}

impl Default for SecretRedactor {
    fn default() -> Self {
        // Redacts patterns matching tokens, bearer keys, passwords
        let pattern = Regex::new(
            r#"(?i)(password|token|secret|key|authorization|bearer)\s*[:=]\s*["']?([A-Za-z0-9_\-\.]{6,})["']?"#,
        )
        .unwrap();
        Self { pattern }
    }
}

impl SecretRedactor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn redact(&self, input: &str) -> String {
        self.pattern
            .replace_all(input, "$1=***REDACTED***")
            .to_string()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VaultBackendKind {
    WindowsCredentialManager,
    EphemeralMemoryForTest,
}

#[derive(Clone)]
pub struct CredentialVault {
    backend: VaultBackendKind,
    ephemeral_store: Arc<Mutex<HashMap<String, String>>>,
}

impl Default for CredentialVault {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialVault {
    /// Production constructor backed by Windows Credential Manager (`advapi32.dll`).
    pub fn new() -> Self {
        #[cfg(windows)]
        let backend = VaultBackendKind::WindowsCredentialManager;
        #[cfg(not(windows))]
        let backend = VaultBackendKind::EphemeralMemoryForTest;

        Self {
            backend,
            ephemeral_store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Explicit in-memory vault strictly for isolated non-OS tests if ever needed.
    pub fn new_ephemeral_for_test() -> Self {
        Self {
            backend: VaultBackendKind::EphemeralMemoryForTest,
            ephemeral_store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn backend_kind(&self) -> &VaultBackendKind {
        &self.backend
    }

    pub fn backend_label(&self) -> &'static str {
        match self.backend {
            VaultBackendKind::WindowsCredentialManager => "WindowsCredentialManager",
            VaultBackendKind::EphemeralMemoryForTest => "EphemeralMemoryForTest",
        }
    }

    pub fn set_secret_namespaced(
        &self,
        ns: &SecretNamespace,
        key: &str,
        secret: &str,
    ) -> Result<String> {
        let target_name = ns.target_name(key)?;
        let uri = format!("{}{}", CRED_MGR_URI_PREFIX, target_name);
        match self.backend {
            #[cfg(windows)]
            VaultBackendKind::WindowsCredentialManager => {
                wincred::os_wincred::write_generic_secret(
                    &target_name,
                    &ns.label(),
                    "FarmerPro Studio Namespaced Secret",
                    secret,
                )?;
            }
            #[cfg(not(windows))]
            VaultBackendKind::WindowsCredentialManager => {
                bail!("Windows Credential Manager is only available on Windows");
            }
            VaultBackendKind::EphemeralMemoryForTest => {
                let mut map = self.ephemeral_store.lock().unwrap();
                map.insert(target_name, secret.to_string());
            }
        }
        Ok(uri)
    }

    pub fn get_secret_namespaced(
        &self,
        ns: &SecretNamespace,
        key: &str,
    ) -> Result<Option<String>> {
        let target_name = ns.target_name(key)?;
        self.get_secret_by_target(&target_name)
    }

    pub fn resolve_secret_uri(&self, uri_or_key: &str) -> Result<Option<String>> {
        let target_name = parse_cred_uri_to_target(uri_or_key)?;
        self.get_secret_by_target(&target_name)
    }

    pub fn delete_secret_namespaced(&self, ns: &SecretNamespace, key: &str) -> Result<bool> {
        let target_name = ns.target_name(key)?;
        match self.backend {
            #[cfg(windows)]
            VaultBackendKind::WindowsCredentialManager => {
                wincred::os_wincred::delete_generic_secret(&target_name)
            }
            #[cfg(not(windows))]
            VaultBackendKind::WindowsCredentialManager => {
                bail!("Windows Credential Manager is only available on Windows");
            }
            VaultBackendKind::EphemeralMemoryForTest => {
                let mut map = self.ephemeral_store.lock().unwrap();
                Ok(map.remove(&target_name).is_some())
            }
        }
    }

    pub fn list_secrets_namespaced(&self, ns: &SecretNamespace) -> Result<Vec<CredentialMetadata>> {
        let prefix = ns.prefix()?;
        let targets = match self.backend {
            #[cfg(windows)]
            VaultBackendKind::WindowsCredentialManager => {
                wincred::os_wincred::enumerate_generic_targets(&prefix)?
            }
            #[cfg(not(windows))]
            VaultBackendKind::WindowsCredentialManager => {
                bail!("Windows Credential Manager is only available on Windows");
            }
            VaultBackendKind::EphemeralMemoryForTest => {
                let map = self.ephemeral_store.lock().unwrap();
                let mut keys: Vec<String> = map
                    .keys()
                    .filter(|k| k.starts_with(&prefix))
                    .cloned()
                    .collect();
                keys.sort();
                keys
            }
        };

        let mut list = Vec::with_capacity(targets.len());
        for target_name in targets {
            let key = target_name
                .strip_prefix(&prefix)
                .unwrap_or(&target_name)
                .to_string();
            list.push(CredentialMetadata {
                cred_uri: format!("{}{}", CRED_MGR_URI_PREFIX, target_name),
                target_name,
                namespace: ns.label(),
                key,
                backend: self.backend_label().to_string(),
            });
        }
        Ok(list)
    }

    fn get_secret_by_target(&self, target_name: &str) -> Result<Option<String>> {
        match self.backend {
            #[cfg(windows)]
            VaultBackendKind::WindowsCredentialManager => {
                wincred::os_wincred::read_generic_secret(target_name)
            }
            #[cfg(not(windows))]
            VaultBackendKind::WindowsCredentialManager => {
                bail!("Windows Credential Manager is only available on Windows");
            }
            VaultBackendKind::EphemeralMemoryForTest => {
                let map = self.ephemeral_store.lock().unwrap();
                Ok(map.get(target_name).cloned())
            }
        }
    }

    /// Convenience wrapper storing under `SecretNamespace::System`
    pub fn set_secret(&self, key: &str, secret: &str) -> Result<String> {
        self.set_secret_namespaced(&SecretNamespace::System, key, secret)
    }

    /// Convenience wrapper reading from `SecretNamespace::System` or `CRED_MGR://` URI
    pub fn get_secret(&self, key_or_uri: &str) -> Result<Option<String>> {
        if key_or_uri.starts_with(CRED_MGR_URI_PREFIX) || key_or_uri.starts_with("FarmerPro/") {
            self.resolve_secret_uri(key_or_uri)
        } else {
            self.get_secret_namespaced(&SecretNamespace::System, key_or_uri)
        }
    }
}

pub struct DestructiveActionGuard;

impl DestructiveActionGuard {
    pub fn verify_destructive_permission(
        action_name: &str,
        confirmation_token: Option<&str>,
    ) -> Result<()> {
        match action_name {
            "clear_app_data" | "reboot_device" | "wipe_cache" => {
                if let Some(token) = confirmation_token {
                    if token == "CONFIRM_DESTRUCTIVE_ACTION" {
                        return Ok(());
                    }
                }
                bail!(
                    "Destructive operation '{}' blocked: requires explicit confirmation token",
                    action_name
                );
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_redaction() {
        let redactor = SecretRedactor::new();
        let log = "Connecting with token: my_secret_token_123456 and password=super_secret_pass";
        let redacted = redactor.redact(log);
        assert!(!redacted.contains("my_secret_token_123456"));
        assert!(!redacted.contains("super_secret_pass"));
        assert!(redacted.contains("***REDACTED***"));
    }

    #[test]
    fn test_destructive_action_guard() {
        let res = DestructiveActionGuard::verify_destructive_permission("clear_app_data", None);
        assert!(res.is_err());

        let res_ok = DestructiveActionGuard::verify_destructive_permission(
            "clear_app_data",
            Some("CONFIRM_DESTRUCTIVE_ACTION"),
        );
        assert!(res_ok.is_ok());
    }

    #[test]
    fn test_dpapi_roundtrip_and_tamper_detection() {
        let secret = b"ed25519-test-seed-0123456789abcdef";
        let entropy = b"FarmerPro.UnitTest.Entropy";
        let protected =
            dpapi::protect_base64(secret, Some(entropy), "FarmerPro DPAPI Unit Test").unwrap();
        assert!(!protected.is_empty());
        assert_ne!(protected.as_bytes(), secret);

        // Correct entropy decrypts cleanly
        let recovered = dpapi::unprotect_base64(&protected, Some(entropy)).unwrap();
        assert_eq!(recovered.as_slice(), secret);

        // Wrong entropy fails closed
        let wrong_entropy = dpapi::unprotect_base64(&protected, Some(b"WrongEntropy"));
        assert!(wrong_entropy.is_err());

        // Corrupted ciphertext fails closed
        let mut raw_cipher = dpapi::protect_bytes(secret, Some(entropy), "tamper").unwrap();
        let mid = raw_cipher.len() / 2;
        raw_cipher[mid] ^= 0xFF;
        assert!(dpapi::unprotect_bytes(&raw_cipher, Some(entropy)).is_err());
    }

    #[test]
    fn test_windows_credential_manager_namespaced_crud_and_isolation() {
        let vault = CredentialVault::new();
        assert_eq!(vault.backend_label(), "WindowsCredentialManager");

        let run_tag = uuid::Uuid::new_v4().to_string();
        let proj_a_slug = format!("p2-test-a-{}", &run_tag[..8]);
        let proj_b_slug = format!("p2-test-b-{}", &run_tag[..8]);
        let tool_a_slug = format!("auto-a-{}", &run_tag[..8]);

        let ns_proj_a = SecretNamespace::Project(proj_a_slug.clone());
        let ns_proj_b = SecretNamespace::Project(proj_b_slug.clone());
        let ns_tool_a = SecretNamespace::MiniTool(tool_a_slug.clone());
        let ns_acct_a = SecretNamespace::Account(proj_a_slug.clone());

        // Ensure cleanup even on early panic
        let _cleanup = scopeguard_cleanup(&vault, &[
            (ns_proj_a.clone(), "api_token"),
            (ns_proj_b.clone(), "api_token"),
            (ns_tool_a.clone(), "proxy_pass"),
            (ns_acct_a.clone(), "acc_01"),
        ]);

        let uri_a = vault
            .set_secret_namespaced(&ns_proj_a, "api_token", "secret-project-A-!@#")
            .unwrap();
        assert_eq!(
            uri_a,
            format!("CRED_MGR://FarmerPro/project/{}/api_token", proj_a_slug)
        );

        let uri_b = vault
            .set_secret_namespaced(&ns_proj_b, "api_token", "secret-project-B-$%^")
            .unwrap();
        let uri_tool = vault
            .set_secret_namespaced(&ns_tool_a, "proxy_pass", "socks5-tool-pass")
            .unwrap();
        let uri_acct = vault
            .set_secret_namespaced(&ns_acct_a, "acc_01", "jx2-player-pass-999")
            .unwrap();

        // Verify exact values and isolation across namespaces
        assert_eq!(
            vault
                .get_secret_namespaced(&ns_proj_a, "api_token")
                .unwrap()
                .as_deref(),
            Some("secret-project-A-!@#")
        );
        assert_eq!(
            vault
                .get_secret_namespaced(&ns_proj_b, "api_token")
                .unwrap()
                .as_deref(),
            Some("secret-project-B-$%^")
        );
        assert_eq!(
            vault
                .get_secret_namespaced(&ns_tool_a, "proxy_pass")
                .unwrap()
                .as_deref(),
            Some("socks5-tool-pass")
        );
        assert_eq!(
            vault
                .resolve_secret_uri(&uri_acct)
                .unwrap()
                .as_deref(),
            Some("jx2-player-pass-999")
        );
        // Project B must NOT see Project A's account or Mini Tool's proxy_pass
        assert_eq!(
            vault
                .get_secret_namespaced(&ns_proj_b, "proxy_pass")
                .unwrap(),
            None
        );

        // List enumerations stay strictly inside their namespace
        let listed_a = vault.list_secrets_namespaced(&ns_proj_a).unwrap();
        assert_eq!(listed_a.len(), 1);
        assert_eq!(listed_a[0].key, "api_token");
        assert_eq!(listed_a[0].cred_uri, uri_a);

        let listed_b = vault.list_secrets_namespaced(&ns_proj_b).unwrap();
        assert_eq!(listed_b.len(), 1);
        assert_eq!(listed_b[0].cred_uri, uri_b);

        let listed_tool = vault.list_secrets_namespaced(&ns_tool_a).unwrap();
        assert_eq!(listed_tool.len(), 1);
        assert_eq!(listed_tool[0].cred_uri, uri_tool);

        // Path traversal / wildcard injection blocked
        assert!(SecretNamespace::Project("../escape".to_string())
            .prefix()
            .is_err());
        assert!(vault
            .set_secret_namespaced(&ns_proj_a, "bad*key", "x")
            .is_err());

        // Delete & verify removal
        assert!(vault
            .delete_secret_namespaced(&ns_proj_a, "api_token")
            .unwrap());
        assert_eq!(
            vault
                .get_secret_namespaced(&ns_proj_a, "api_token")
                .unwrap(),
            None
        );
        // Project B untouched after Project A delete
        assert_eq!(
            vault
                .get_secret_namespaced(&ns_proj_b, "api_token")
                .unwrap()
                .as_deref(),
            Some("secret-project-B-$%^")
        );
    }

    #[test]
    fn test_dpapi_producer_keystore_create_backup_rotate_and_restore() {
        let temp_dir = std::env::temp_dir().join(format!(
            "fp_dpapi_keystore_test_{}",
            uuid::Uuid::new_v4()
        ));
        let ks_dir = temp_dir.join("keystore");
        let backup_path = temp_dir.join("backups").join("producer_key_backup.json");

        // 1. Create fresh DPAPI-protected Ed25519 keypair
        let (sk1, meta1) = ProducerKeystore::load_or_create_signing_key(&ks_dir).unwrap();
        assert_eq!(meta1.format_version, 2);
        assert_eq!(meta1.protection_scheme, "WIN32_DPAPI_LOCAL_USER");

        // Ensure plaintext seed is NEVER stored on disk
        let raw_json = std::fs::read_to_string(&meta1.key_store_path).unwrap();
        assert!(!raw_json.contains("secretSeedHex"));
        assert!(raw_json.contains("dpapiProtectedSeedB64"));

        // 2. Reload yields identical keypair
        let (sk1_reloaded, meta1_reloaded) =
            ProducerKeystore::load_or_create_signing_key(&ks_dir).unwrap();
        assert_eq!(sk1.to_bytes(), sk1_reloaded.to_bytes());
        assert_eq!(meta1.public_key_hex, meta1_reloaded.public_key_hex);

        // 3. Export DPAPI-encrypted backup
        let backup = ProducerKeystore::backup_signing_key(&ks_dir, &backup_path).unwrap();
        assert_eq!(backup.key_id, meta1.key_id);
        assert_eq!(backup.public_key_hex, meta1.public_key_hex);

        // 4. Rotate key -> new key_id and public key, old key archived
        let (_sk2, meta2) = ProducerKeystore::rotate_producer_key(&ks_dir, "p2_rotation").unwrap();
        assert_ne!(meta1.key_id, meta2.key_id);
        assert_ne!(meta1.public_key_hex, meta2.public_key_hex);

        // 5. Tampered backup must be rejected
        let tampered_backup_path = temp_dir.join("backups").join("tampered_backup.json");
        let mut tampered_bundle = backup.clone();
        tampered_bundle.public_key_hex = "00".repeat(32);
        std::fs::write(
            &tampered_backup_path,
            serde_json::to_vec_pretty(&tampered_bundle).unwrap(),
        )
        .unwrap();
        assert!(ProducerKeystore::restore_signing_key(&tampered_backup_path, &ks_dir).is_err());

        // 6. Valid backup restores original key1
        let restored_meta = ProducerKeystore::restore_signing_key(&backup_path, &ks_dir).unwrap();
        assert_eq!(restored_meta.key_id, meta1.key_id);
        assert_eq!(restored_meta.public_key_hex, meta1.public_key_hex);
        let (sk_restored, _) = ProducerKeystore::load_or_create_signing_key(&ks_dir).unwrap();
        assert_eq!(sk_restored.to_bytes(), sk1.to_bytes());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_dpapi_legacy_v1_keystore_auto_upgrade() {
        let temp_dir = std::env::temp_dir().join(format!(
            "fp_dpapi_legacy_upgrade_{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let key_file = temp_dir.join("active_producer_ed25519.json");

        // RFC 8032 test vector 1 seed
        let seed_hex = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
        let pub_hex = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let legacy_json = serde_json::json!({
            "keyId": "fp-ed25519-d75a980182b1",
            "secretSeedHex": seed_hex,
            "publicKeyHex": pub_hex,
            "createdAtUtc": "2026-11-21T00:00:00Z"
        });
        std::fs::write(&key_file, serde_json::to_vec_pretty(&legacy_json).unwrap()).unwrap();

        let (sk, meta) = ProducerKeystore::load_or_create_signing_key(&temp_dir).unwrap();
        assert_eq!(meta.format_version, 2);
        assert_eq!(meta.protection_scheme, "WIN32_DPAPI_LOCAL_USER");
        assert_eq!(meta.public_key_hex, pub_hex);
        assert_eq!(keystore::hex_encode(&sk.to_bytes()), seed_hex);

        // Verify file on disk has been scrubbed of plaintext secretSeedHex and encrypted with DPAPI
        let upgraded_raw = std::fs::read_to_string(&key_file).unwrap();
        assert!(!upgraded_raw.contains(seed_hex));
        assert!(!upgraded_raw.contains("secretSeedHex"));
        assert!(upgraded_raw.contains("dpapiProtectedSeedB64"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_portable_passphrase_recovery_bundle_roundtrip_wrong_pass_and_tamper() {
        let temp_dir = std::env::temp_dir().join(format!(
            "fp_portable_recovery_test_{}",
            uuid::Uuid::new_v4()
        ));
        let ks_machine_a = temp_dir.join("machine_a_ks");
        let ks_machine_b = temp_dir.join("machine_b_ks");
        let recovery_file = temp_dir.join("offline_recovery.json");

        let (sk_a, meta_a) = ProducerKeystore::load_or_create_signing_key(&ks_machine_a).unwrap();
        let seed_hex = keystore::hex_encode(&sk_a.to_bytes());

        // Weak passphrase rejected
        assert!(ProducerKeystore::export_portable_recovery_bundle(
            &ks_machine_a,
            &recovery_file,
            "short"
        )
        .is_err());

        // Export with strong passphrase
        let bundle = ProducerKeystore::export_portable_recovery_bundle(
            &ks_machine_a,
            &recovery_file,
            "FarmerPro-Offline-Recovery-2026!",
        )
        .unwrap();
        assert_eq!(bundle.protection_scheme, "PASSPHRASE_PBKDF2_SHA256_ETM_V1");
        assert_eq!(bundle.kdf_iterations, 200_000);
        assert_eq!(bundle.key_id, meta_a.key_id);

        // Ensure recovery bundle JSON contains ZERO plaintext seed
        let raw_bundle = std::fs::read_to_string(&recovery_file).unwrap();
        assert!(!raw_bundle.contains(&seed_hex));
        assert!(!raw_bundle.contains("secretSeedHex"));

        // Wrong passphrase must fail closed
        let wrong_res = ProducerKeystore::import_portable_recovery_bundle(
            &recovery_file,
            &ks_machine_b,
            "Wrong-Passphrase-999",
        );
        assert!(wrong_res.is_err());
        assert!(wrong_res
            .unwrap_err()
            .to_string()
            .contains("RECOVERY_AUTH_FAILED"));

        // Tampered metadata/ciphertext must fail closed
        let tampered_path = temp_dir.join("tampered_recovery.json");
        let mut tampered = bundle.clone();
        tampered.public_key_hex = "ff".repeat(32);
        std::fs::write(&tampered_path, serde_json::to_vec_pretty(&tampered).unwrap()).unwrap();
        assert!(ProducerKeystore::import_portable_recovery_bundle(
            &tampered_path,
            &ks_machine_b,
            "FarmerPro-Offline-Recovery-2026!"
        )
        .is_err());

        // Valid passphrase imports into Machine B's DPAPI keystore cleanly
        let imported_meta = ProducerKeystore::import_portable_recovery_bundle(
            &recovery_file,
            &ks_machine_b,
            "FarmerPro-Offline-Recovery-2026!",
        )
        .unwrap();
        assert_eq!(imported_meta.key_id, meta_a.key_id);
        assert_eq!(imported_meta.public_key_hex, meta_a.public_key_hex);
        assert_eq!(imported_meta.protection_scheme, "WIN32_DPAPI_LOCAL_USER");

        let (sk_b, _) = ProducerKeystore::load_or_create_signing_key(&ks_machine_b).unwrap();
        assert_eq!(sk_a.to_bytes(), sk_b.to_bytes());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    struct CleanupGuard<'a> {
        vault: &'a CredentialVault,
        entries: Vec<(SecretNamespace, String)>,
    }

    impl Drop for CleanupGuard<'_> {
        fn drop(&mut self) {
            for (ns, k) in &self.entries {
                let _ = self.vault.delete_secret_namespaced(ns, k);
            }
        }
    }

    fn scopeguard_cleanup<'a>(
        vault: &'a CredentialVault,
        items: &[(SecretNamespace, &str)],
    ) -> CleanupGuard<'a> {
        CleanupGuard {
            vault,
            entries: items
                .iter()
                .map(|(ns, k)| (ns.clone(), (*k).to_string()))
                .collect(),
        }
    }
}

use crate::dpapi;
use anyhow::{bail, Context, Result};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const PRODUCER_KEY_DPAPI_ENTROPY: &[u8] = b"FarmerPro.ProducerEd25519.Keystore.v2";
const PRODUCER_KEY_BACKUP_ENTROPY: &[u8] = b"FarmerPro.ProducerEd25519.Backup.v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProducerKeyMetadata {
    pub format_version: u32,
    pub protection_scheme: String,
    pub key_id: String,
    pub public_key_hex: String,
    pub created_at_utc: String,
    pub key_store_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredProducerKeyFile {
    #[serde(default = "default_format_v1")]
    pub format_version: u32,
    #[serde(default = "default_scheme_legacy")]
    pub protection_scheme: String,
    pub key_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dpapi_protected_seed_b64: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_seed_hex: Option<String>,
    pub public_key_hex: String,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProducerKeyBackupBundle {
    pub format_version: u32,
    pub protection_scheme: String,
    pub key_id: String,
    pub public_key_hex: String,
    pub created_at_utc: String,
    pub backed_up_at_utc: String,
    pub dpapi_protected_seed_b64: String,
    pub payload_sha256: String,
}

fn default_format_v1() -> u32 {
    1
}

fn default_scheme_legacy() -> String {
    "LEGACY_PLAINTEXT_V1".to_string()
}

pub fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

pub fn hex_decode(hex_str: &str) -> Result<Vec<u8>> {
    let s = hex_str.trim();
    if s.len() % 2 != 0 {
        bail!("Invalid hex string length: {}", s.len());
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(&s[i..i + 2], 16)
            .with_context(|| format!("Invalid hex byte at offset {}", i))?;
        out.push(byte);
    }
    Ok(out)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_encode(&hasher.finalize())
}

fn compute_backup_digest(
    key_id: &str,
    public_key_hex: &str,
    created_at_utc: &str,
    backed_up_at_utc: &str,
    dpapi_protected_seed_b64: &str,
) -> String {
    let canonical = format!(
        "v2|{}|{}|{}|{}|{}",
        key_id, public_key_hex, created_at_utc, backed_up_at_utc, dpapi_protected_seed_b64
    );
    sha256_hex(canonical.as_bytes())
}

pub struct ProducerKeystore;

impl ProducerKeystore {
    /// Resolves the default external key store directory in `%LOCALAPPDATA%\FarmerPro\ProducerKeys`
    pub fn default_external_keystore_dir() -> PathBuf {
        let base = std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir());
        base.join("FarmerPro").join("ProducerKeys")
    }

    fn generate_fresh_seed(salt_label: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(uuid::Uuid::new_v4().as_bytes());
        hasher.update(uuid::Uuid::new_v4().as_bytes());
        hasher.update(
            chrono::Utc::now()
                .timestamp_nanos_opt()
                .unwrap_or(0)
                .to_le_bytes(),
        );
        hasher.update(std::process::id().to_le_bytes());
        hasher.update(salt_label.as_bytes());
        let digest = hasher.finalize();
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&digest);
        seed
    }

    fn write_dpapi_key_file(
        key_file: &Path,
        key_id: &str,
        seed: &[u8; 32],
        public_key_hex: &str,
        created_at_utc: &str,
    ) -> Result<()> {
        let protected_b64 = dpapi::protect_base64(
            seed,
            Some(PRODUCER_KEY_DPAPI_ENTROPY),
            "FarmerPro Ed25519 Producer Signing Key",
        )?;

        let stored = StoredProducerKeyFile {
            format_version: 2,
            protection_scheme: "WIN32_DPAPI_LOCAL_USER".to_string(),
            key_id: key_id.to_string(),
            dpapi_protected_seed_b64: Some(protected_b64),
            secret_seed_hex: None,
            public_key_hex: public_key_hex.to_string(),
            created_at_utc: created_at_utc.to_string(),
        };
        std::fs::write(key_file, serde_json::to_vec_pretty(&stored)?)?;
        Ok(())
    }

    fn decode_signing_key_from_stored(stored: &StoredProducerKeyFile, key_file: &Path) -> Result<(SigningKey, bool)> {
        if let Some(ref b64) = stored.dpapi_protected_seed_b64 {
            let mut seed_vec = dpapi::unprotect_base64(b64, Some(PRODUCER_KEY_DPAPI_ENTROPY))?;
            if seed_vec.len() != 32 {
                seed_vec.fill(0);
                bail!("Invalid DPAPI-decrypted Ed25519 seed length in {}", key_file.display());
            }
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&seed_vec);
            seed_vec.fill(0);
            let sk = SigningKey::from_bytes(&seed);
            seed.fill(0);
            let derived_pub = hex_encode(sk.verifying_key().as_bytes());
            if !derived_pub.eq_ignore_ascii_case(&stored.public_key_hex) {
                bail!(
                    "Keystore integrity mismatch in {}: derived public key '{}' != stored '{}'",
                    key_file.display(),
                    derived_pub,
                    stored.public_key_hex
                );
            }
            return Ok((sk, false));
        }

        if let Some(ref legacy_hex) = stored.secret_seed_hex {
            let mut seed_vec = hex_decode(legacy_hex)?;
            if seed_vec.len() != 32 {
                seed_vec.fill(0);
                bail!("Invalid legacy Ed25519 seed length in {}", key_file.display());
            }
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&seed_vec);
            seed_vec.fill(0);
            let sk = SigningKey::from_bytes(&seed);
            // Auto-upgrade legacy plaintext key to DPAPI v2 without changing keypair
            let pub_hex = hex_encode(sk.verifying_key().as_bytes());
            Self::write_dpapi_key_file(key_file, &stored.key_id, &seed, &pub_hex, &stored.created_at_utc)?;
            seed.fill(0);
            return Ok((sk, true));
        }

        bail!("Keystore file {} contains neither DPAPI seed nor legacy seed", key_file.display());
    }

    pub fn inspect_active_metadata(keystore_dir: &Path) -> Result<Option<ProducerKeyMetadata>> {
        let key_file = keystore_dir.join("active_producer_ed25519.json");
        if !key_file.exists() {
            return Ok(None);
        }
        let raw = std::fs::read(&key_file)?;
        let stored: StoredProducerKeyFile = serde_json::from_slice(&raw)?;
        Ok(Some(ProducerKeyMetadata {
            format_version: stored.format_version,
            protection_scheme: stored.protection_scheme,
            key_id: stored.key_id,
            public_key_hex: stored.public_key_hex,
            created_at_utc: stored.created_at_utc,
            key_store_path: key_file.to_string_lossy().to_string(),
        }))
    }

    pub fn load_or_create_signing_key(
        keystore_dir: &Path,
    ) -> Result<(SigningKey, ProducerKeyMetadata)> {
        std::fs::create_dir_all(keystore_dir)?;
        let key_file = keystore_dir.join("active_producer_ed25519.json");

        if key_file.exists() {
            let raw = std::fs::read(&key_file)?;
            let stored: StoredProducerKeyFile = serde_json::from_slice(&raw)?;
            let (signing_key, upgraded) = Self::decode_signing_key_from_stored(&stored, &key_file)?;
            let pub_hex = hex_encode(signing_key.verifying_key().as_bytes());

            let meta = ProducerKeyMetadata {
                format_version: if upgraded { 2 } else { stored.format_version },
                protection_scheme: if upgraded {
                    "WIN32_DPAPI_LOCAL_USER".to_string()
                } else {
                    stored.protection_scheme
                },
                key_id: stored.key_id,
                public_key_hex: pub_hex,
                created_at_utc: stored.created_at_utc,
                key_store_path: key_file.to_string_lossy().to_string(),
            };
            return Ok((signing_key, meta));
        }

        Self::rotate_producer_key(keystore_dir, "initial")
    }

    pub fn rotate_producer_key(
        keystore_dir: &Path,
        reason: &str,
    ) -> Result<(SigningKey, ProducerKeyMetadata)> {
        std::fs::create_dir_all(keystore_dir)?;
        let key_file = keystore_dir.join("active_producer_ed25519.json");

        if key_file.exists() {
            let archive_name = format!(
                "archived_producer_ed25519_{}.json",
                chrono::Utc::now().format("%Y%m%d_%H%M%S_%f")
            );
            std::fs::rename(&key_file, keystore_dir.join(archive_name))?;
        }

        let mut seed = Self::generate_fresh_seed(reason);
        let signing_key = SigningKey::from_bytes(&seed);
        let pub_hex = hex_encode(signing_key.verifying_key().as_bytes());
        let short_pub = &pub_hex[..12];
        let key_id = format!("fp-ed25519-{}", short_pub);
        let now = chrono::Utc::now().to_rfc3339();

        Self::write_dpapi_key_file(&key_file, &key_id, &seed, &pub_hex, &now)?;
        seed.fill(0);

        let meta = ProducerKeyMetadata {
            format_version: 2,
            protection_scheme: "WIN32_DPAPI_LOCAL_USER".to_string(),
            key_id,
            public_key_hex: pub_hex,
            created_at_utc: now,
            key_store_path: key_file.to_string_lossy().to_string(),
        };
        Ok((signing_key, meta))
    }

    pub fn backup_signing_key(
        keystore_dir: &Path,
        backup_file_path: &Path,
    ) -> Result<ProducerKeyBackupBundle> {
        let (signing_key, meta) = Self::load_or_create_signing_key(keystore_dir)?;
        let mut seed_bytes = signing_key.to_bytes();
        let backed_up_at_utc = chrono::Utc::now().to_rfc3339();
        let dpapi_protected_seed_b64 = dpapi::protect_base64(
            &seed_bytes,
            Some(PRODUCER_KEY_BACKUP_ENTROPY),
            "FarmerPro Ed25519 Producer Key Backup",
        )?;
        seed_bytes.fill(0);

        let payload_sha256 = compute_backup_digest(
            &meta.key_id,
            &meta.public_key_hex,
            &meta.created_at_utc,
            &backed_up_at_utc,
            &dpapi_protected_seed_b64,
        );

        let bundle = ProducerKeyBackupBundle {
            format_version: 2,
            protection_scheme: "WIN32_DPAPI_LOCAL_USER_BACKUP".to_string(),
            key_id: meta.key_id,
            public_key_hex: meta.public_key_hex,
            created_at_utc: meta.created_at_utc,
            backed_up_at_utc,
            dpapi_protected_seed_b64,
            payload_sha256,
        };

        if let Some(parent) = backup_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(backup_file_path, serde_json::to_vec_pretty(&bundle)?)?;
        Ok(bundle)
    }

    pub fn restore_signing_key(
        backup_file_path: &Path,
        keystore_dir: &Path,
    ) -> Result<ProducerKeyMetadata> {
        let raw = std::fs::read(backup_file_path)
            .with_context(|| format!("Failed to read key backup: {}", backup_file_path.display()))?;
        let bundle: ProducerKeyBackupBundle = serde_json::from_slice(&raw)?;

        let expected_sha = compute_backup_digest(
            &bundle.key_id,
            &bundle.public_key_hex,
            &bundle.created_at_utc,
            &bundle.backed_up_at_utc,
            &bundle.dpapi_protected_seed_b64,
        );
        if expected_sha != bundle.payload_sha256 {
            bail!(
                "KEY_BACKUP_TAMPERED: Checksum mismatch (expected {}, got {})",
                expected_sha,
                bundle.payload_sha256
            );
        }

        let mut seed_vec = dpapi::unprotect_base64(
            &bundle.dpapi_protected_seed_b64,
            Some(PRODUCER_KEY_BACKUP_ENTROPY),
        )?;
        if seed_vec.len() != 32 {
            seed_vec.fill(0);
            bail!("Invalid restored Ed25519 seed length");
        }
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&seed_vec);
        seed_vec.fill(0);

        let sk = SigningKey::from_bytes(&seed);
        let derived_pub = hex_encode(sk.verifying_key().as_bytes());
        if !derived_pub.eq_ignore_ascii_case(&bundle.public_key_hex) {
            seed.fill(0);
            bail!("Restored public key mismatch against backup metadata");
        }

        // Verify signature roundtrip before committing restore
        let probe_sig = sk.sign(b"farmerpro-restore-probe");
        let vk = VerifyingKey::from_bytes(sk.verifying_key().as_bytes())?;
        vk.verify_strict(b"farmerpro-restore-probe", &probe_sig)?;

        std::fs::create_dir_all(keystore_dir)?;
        let key_file = keystore_dir.join("active_producer_ed25519.json");
        if key_file.exists() {
            let archive_name = format!(
                "pre_restore_producer_ed25519_{}.json",
                chrono::Utc::now().format("%Y%m%d_%H%M%S_%f")
            );
            std::fs::rename(&key_file, keystore_dir.join(archive_name))?;
        }

        Self::write_dpapi_key_file(
            &key_file,
            &bundle.key_id,
            &seed,
            &bundle.public_key_hex,
            &bundle.created_at_utc,
        )?;
        seed.fill(0);

        Ok(ProducerKeyMetadata {
            format_version: 2,
            protection_scheme: "WIN32_DPAPI_LOCAL_USER".to_string(),
            key_id: bundle.key_id,
            public_key_hex: bundle.public_key_hex,
            created_at_utc: bundle.created_at_utc,
            key_store_path: key_file.to_string_lossy().to_string(),
        })
    }
}

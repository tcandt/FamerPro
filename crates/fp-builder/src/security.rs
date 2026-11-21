//! Producer Key Lifecycle Management & RFC 8032 Ed25519 Bundle Signer.
//!
//! KEY MANAGEMENT MODEL (Requirement 1):
//! - Neither `FarmerPro.exe` (Studio) nor `fp-mini-runtime.exe` embeds a shared static private signing key in source code.
//! - Each Studio producer machine generates its own unique 32-byte Ed25519 seed stored outside the repository/binaries
//!   at `%LOCALAPPDATA%\FarmerPro\ProducerKeys\producer_ed25519.key` (or an explicitly configured external key path).
//! - When building a Mini Tool release, `fp-builder` signs the bundle with the producer's external `SigningKey`
//!   and writes the corresponding 32-byte `VerifyingKey` (Public Key) into `bundle/trusted_publisher.pub`.
//! - Supports full Key Rotation (`rotate_producer_key`): archives the old keypair with timestamp and generates a new keypair.

use anyhow::{bail, Result};
use ed25519_dalek::{Signer, SigningKey};
use fp_bundle_verify::{
    canonical_signing_message, hex_decode, hex_encode, sha256_hex, SignedBundleEnvelope,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProducerKeyMetadata {
    pub key_id: String,
    pub public_key_hex: String,
    pub created_at_utc: String,
    pub key_store_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredProducerKeyFile {
    pub key_id: String,
    pub secret_seed_hex: String,
    pub public_key_hex: String,
    pub created_at_utc: String,
}

pub struct ProducerKeyManager;

impl ProducerKeyManager {
    /// Resolves the default external key store directory in `%LOCALAPPDATA%\FarmerPro\ProducerKeys`
    pub fn default_external_keystore_dir() -> PathBuf {
        let base = std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir());
        base.join("FarmerPro").join("ProducerKeys")
    }

    /// Generates a fresh 32-byte Ed25519 seed using OS entropy (`Uuid::new_v4()` x 2 + timestamp + process ID hashed via SHA-256)
    fn generate_fresh_seed(salt_label: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(uuid::Uuid::new_v4().as_bytes());
        hasher.update(uuid::Uuid::new_v4().as_bytes());
        hasher.update(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0).to_le_bytes());
        hasher.update(std::process::id().to_le_bytes());
        hasher.update(salt_label.as_bytes());
        let digest = hasher.finalize();
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&digest);
        seed
    }

    /// Loads the active producer Ed25519 `SigningKey` from an external keystore directory,
    /// or generates and saves a new one on first run.
    pub fn load_or_create_signing_key(
        keystore_dir: &Path,
    ) -> Result<(SigningKey, ProducerKeyMetadata)> {
        std::fs::create_dir_all(keystore_dir)?;
        let key_file = keystore_dir.join("active_producer_ed25519.json");

        if key_file.exists() {
            let raw = std::fs::read(&key_file)?;
            let stored: StoredProducerKeyFile = serde_json::from_slice(&raw)?;
            let seed_vec = hex_decode(&stored.secret_seed_hex)?;
            if seed_vec.len() != 32 {
                bail!("Invalid stored Ed25519 seed length in {}", key_file.display());
            }
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&seed_vec);
            let signing_key = SigningKey::from_bytes(&seed);
            let pub_hex = hex_encode(signing_key.verifying_key().as_bytes());

            let meta = ProducerKeyMetadata {
                key_id: stored.key_id,
                public_key_hex: pub_hex,
                created_at_utc: stored.created_at_utc,
                key_store_path: key_file.to_string_lossy().to_string(),
            };
            return Ok((signing_key, meta));
        }

        Self::rotate_producer_key(keystore_dir, "initial")
    }

    /// Rotates the producer's Ed25519 signing key: archives any existing key file and writes a brand-new keypair.
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

        let seed = Self::generate_fresh_seed(reason);
        let signing_key = SigningKey::from_bytes(&seed);
        let pub_hex = hex_encode(signing_key.verifying_key().as_bytes());
        let short_pub = &pub_hex[..12];
        let key_id = format!("fp-ed25519-{}", short_pub);
        let now = chrono::Utc::now().to_rfc3339();

        let stored = StoredProducerKeyFile {
            key_id: key_id.clone(),
            secret_seed_hex: hex_encode(&seed),
            public_key_hex: pub_hex.clone(),
            created_at_utc: now.clone(),
        };
        std::fs::write(&key_file, serde_json::to_vec_pretty(&stored)?)?;

        let meta = ProducerKeyMetadata {
            key_id,
            public_key_hex: pub_hex,
            created_at_utc: now,
            key_store_path: key_file.to_string_lossy().to_string(),
        };
        Ok((signing_key, meta))
    }

    /// Signs a bundle's manifest SHA-256 and file checksums table using RFC 8032 Ed25519 (`ed25519-dalek`)
    pub fn sign_bundle_ed25519(
        signing_key: &SigningKey,
        key_id: &str,
        manifest_sha256: String,
        file_checksums: BTreeMap<String, String>,
    ) -> SignedBundleEnvelope {
        let canonical_table = serde_json::to_string(&file_checksums).unwrap_or_default();
        let checksums_table_sha256 = sha256_hex(canonical_table.as_bytes());
        let msg = canonical_signing_message(key_id, &manifest_sha256, &checksums_table_sha256);

        let signature = signing_key.sign(&msg);
        let pub_hex = hex_encode(signing_key.verifying_key().as_bytes());
        let sig_hex = hex_encode(&signature.to_bytes());

        SignedBundleEnvelope {
            algorithm: "Ed25519-RFC8032-SHA256".to_string(),
            key_id: key_id.to_string(),
            publisher_public_key_hex: pub_hex,
            manifest_sha256,
            checksums_table_sha256,
            file_checksums,
            signature_ed25519_hex: sig_hex,
            signed_at_utc: chrono::Utc::now().to_rfc3339(),
        }
    }
}

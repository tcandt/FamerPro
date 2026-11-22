//! Producer Key Lifecycle Management & RFC 8032 Ed25519 Bundle Signer.
//!
//! KEY MANAGEMENT MODEL (Requirement 1 + P2 DPAPI Hardening):
//! - Neither `FarmerPro.exe` (Studio) nor `fp-mini-runtime.exe` embeds a shared static private signing key in source code.
//! - Each Studio producer machine generates its own unique 32-byte Ed25519 seed protected by Windows DPAPI (`CryptProtectData`)
//!   and stored outside the repository/binaries at `%LOCALAPPDATA%\FarmerPro\ProducerKeys\active_producer_ed25519.json`.
//! - When building a Mini Tool release, `fp-builder` signs the bundle with the producer's external `SigningKey`
//!   and embeds the corresponding 32-byte `VerifyingKey` (Public Key) in PE `RT_RCDATA #7777` + `bundle/trusted_publisher.pub`.
//! - Supports full Key Rotation (`rotate_producer_key`), DPAPI-protected Backup (`backup_signing_key`), and Restore (`restore_signing_key`).

use anyhow::Result;
use ed25519_dalek::{Signer, SigningKey};
use fp_bundle_verify::{
    canonical_signing_message, hex_encode, sha256_hex, SignedBundleEnvelope,
};
use fp_security::{ProducerKeyBackupBundle, ProducerKeystore};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub use fp_security::ProducerKeyMetadata;

pub struct ProducerKeyManager;

impl ProducerKeyManager {
    /// Resolves the default external key store directory in `%LOCALAPPDATA%\FarmerPro\ProducerKeys`
    pub fn default_external_keystore_dir() -> PathBuf {
        ProducerKeystore::default_external_keystore_dir()
    }

    /// Loads the active producer Ed25519 `SigningKey` (DPAPI-protected) from an external keystore directory,
    /// or generates and saves a new DPAPI-protected one on first run.
    pub fn load_or_create_signing_key(
        keystore_dir: &Path,
    ) -> Result<(SigningKey, ProducerKeyMetadata)> {
        ProducerKeystore::load_or_create_signing_key(keystore_dir)
    }

    /// Rotates the producer's Ed25519 signing key: archives any existing key file and writes a brand-new DPAPI-protected keypair.
    pub fn rotate_producer_key(
        keystore_dir: &Path,
        reason: &str,
    ) -> Result<(SigningKey, ProducerKeyMetadata)> {
        ProducerKeystore::rotate_producer_key(keystore_dir, reason)
    }

    /// Exports a DPAPI-protected, SHA-256 checksummed backup of the active signing key.
    pub fn backup_signing_key(
        keystore_dir: &Path,
        backup_file_path: &Path,
    ) -> Result<ProducerKeyBackupBundle> {
        ProducerKeystore::backup_signing_key(keystore_dir, backup_file_path)
    }

    /// Restores a DPAPI-protected signing key backup after verifying checksum and cryptographic roundtrip.
    pub fn restore_signing_key(
        backup_file_path: &Path,
        keystore_dir: &Path,
    ) -> Result<ProducerKeyMetadata> {
        ProducerKeystore::restore_signing_key(backup_file_path, keystore_dir)
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

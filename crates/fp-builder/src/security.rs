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
use chrono::{Duration, Utc};
use ed25519_dalek::{Signer, SigningKey};
use fp_bundle_contract::{canonical_key_rotation_message, KeyRotationDelegation};
use fp_bundle_verify::{canonical_signing_message, hex_encode, sha256_hex, SignedBundleEnvelope};
use fp_security::{PortableProducerKeyRecoveryBundle, ProducerKeyBackupBundle, ProducerKeystore};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

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

    /// Exports a passphrase-encrypted portable recovery bundle for cross-machine/cross-user recovery.
    pub fn export_portable_recovery_bundle(
        keystore_dir: &Path,
        backup_file_path: &Path,
        passphrase: &str,
    ) -> Result<PortableProducerKeyRecoveryBundle> {
        ProducerKeystore::export_portable_recovery_bundle(
            keystore_dir,
            backup_file_path,
            passphrase,
        )
    }

    /// Imports a passphrase-encrypted portable recovery bundle into the local Windows DPAPI keystore.
    pub fn import_portable_recovery_bundle(
        backup_file_path: &Path,
        keystore_dir: &Path,
        passphrase: &str,
    ) -> Result<ProducerKeyMetadata> {
        ProducerKeystore::import_portable_recovery_bundle(
            backup_file_path,
            keystore_dir,
            passphrase,
        )
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

    /// Creates a forward-signed `KeyRotationDelegation` signed by `old_signing_key`, authorizing
    /// `new_pubkey_hex` for `(publisher_id, product_slug)` within `[now, now + valid_days]`.
    pub fn sign_key_rotation_delegation(
        old_signing_key: &SigningKey,
        new_pubkey_hex: &str,
        publisher_id: &str,
        product_slug: &str,
        valid_days: i64,
    ) -> KeyRotationDelegation {
        let old_pub_hex = hex_encode(old_signing_key.verifying_key().as_bytes());
        let now = Utc::now();
        let issued_at_utc = now.to_rfc3339();
        let expires_at_utc = (now + Duration::days(valid_days.max(1))).to_rfc3339();
        let nonce = format!("rot-{}", Uuid::new_v4().simple());
        let msg = canonical_key_rotation_message(
            publisher_id,
            product_slug,
            &old_pub_hex,
            new_pubkey_hex,
            &issued_at_utc,
            &expires_at_utc,
            &nonce,
        );
        let sig = old_signing_key.sign(&msg);
        KeyRotationDelegation {
            publisher_id: publisher_id.to_string(),
            product_slug: product_slug.to_string(),
            old_pubkey_hex: old_pub_hex,
            new_pubkey_hex: new_pubkey_hex.trim().to_ascii_lowercase(),
            issued_at_utc,
            expires_at_utc,
            nonce,
            signature_by_old_key_hex: hex_encode(&sig.to_bytes()),
        }
    }
}

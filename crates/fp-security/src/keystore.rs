use crate::dpapi;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{bail, Context, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use zeroize::{Zeroize, Zeroizing};

const PRODUCER_KEY_DPAPI_ENTROPY: &[u8] = b"FarmerPro.ProducerEd25519.Keystore.v2";
const PRODUCER_KEY_BACKUP_ENTROPY: &[u8] = b"FarmerPro.ProducerEd25519.Backup.v2";

pub const PORTABLE_RECOVERY_FORMAT_VERSION: u32 = 3;
pub const PORTABLE_RECOVERY_SCHEME: &str = "ARGON2ID_AES_256_GCM_V3";
pub const PORTABLE_RECOVERY_AEAD: &str = "AES-256-GCM";
pub const PORTABLE_RECOVERY_ARGON2_ALG: &str = "argon2id";
pub const PORTABLE_RECOVERY_ARGON2_VER: &str = "0x13";
pub const PORTABLE_RECOVERY_ARGON2_MEM_KIB: u32 = 65_536; // 64 MiB
pub const PORTABLE_RECOVERY_ARGON2_ITERS: u32 = 3;
pub const PORTABLE_RECOVERY_ARGON2_PARALLELISM: u32 = 1;
pub const PORTABLE_RECOVERY_RETIRED_EXPERIMENTAL_SCHEME: &str = "PASSPHRASE_PBKDF2_SHA256_ETM_V1";

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Argon2idKdfParams {
    pub algorithm: String,
    pub version: String,
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
    pub output_len_bytes: u32,
}

impl Default for Argon2idKdfParams {
    fn default() -> Self {
        Self {
            algorithm: PORTABLE_RECOVERY_ARGON2_ALG.to_string(),
            version: PORTABLE_RECOVERY_ARGON2_VER.to_string(),
            memory_kib: PORTABLE_RECOVERY_ARGON2_MEM_KIB,
            iterations: PORTABLE_RECOVERY_ARGON2_ITERS,
            parallelism: PORTABLE_RECOVERY_ARGON2_PARALLELISM,
            output_len_bytes: 32,
        }
    }
}

/// Cross-machine / cross-Windows-user passphrase-encrypted offline recovery bundle.
/// Uses standard Argon2id (`v=0x13`, 64 MiB, 3 iterations, p=1) + AES-256-GCM AEAD (96-bit CSPRNG nonce)
/// with full metadata bound as Authenticated Additional Data (AAD).
/// Never stores plaintext seed; imports into target machine/user's Windows DPAPI keystore.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableProducerKeyRecoveryBundle {
    pub format_version: u32,
    pub protection_scheme: String,
    pub aead_algorithm: String,
    pub kdf_params: Argon2idKdfParams,
    pub key_id: String,
    pub public_key_hex: String,
    pub created_at_utc: String,
    pub backed_up_at_utc: String,
    pub salt_b64: String,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PortableRecoveryAad<'a> {
    format_version: u32,
    protection_scheme: &'a str,
    aead_algorithm: &'a str,
    kdf_params: &'a Argon2idKdfParams,
    key_id: &'a str,
    public_key_hex: &'a str,
    created_at_utc: &'a str,
    backed_up_at_utc: &'a str,
    salt_b64: &'a str,
    nonce_b64: &'a str,
}

fn build_portable_recovery_aad(bundle: &PortableProducerKeyRecoveryBundle) -> Result<Vec<u8>> {
    let aad = PortableRecoveryAad {
        format_version: bundle.format_version,
        protection_scheme: &bundle.protection_scheme,
        aead_algorithm: &bundle.aead_algorithm,
        kdf_params: &bundle.kdf_params,
        key_id: &bundle.key_id,
        public_key_hex: &bundle.public_key_hex,
        created_at_utc: &bundle.created_at_utc,
        backed_up_at_utc: &bundle.backed_up_at_utc,
        salt_b64: &bundle.salt_b64,
        nonce_b64: &bundle.nonce_b64,
    };
    serde_json::to_vec(&aad).context("Failed serializing recovery AEAD associated data")
}

fn derive_argon2id_aes_key(
    passphrase: &str,
    salt: &[u8],
    kdf: &Argon2idKdfParams,
) -> Result<Zeroizing<[u8; 32]>> {
    let params = Params::new(
        kdf.memory_kib,
        kdf.iterations,
        kdf.parallelism,
        Some(kdf.output_len_bytes as usize),
    )
    .map_err(|e| anyhow::anyhow!("RECOVERY_INVALID_METADATA: Invalid Argon2id params: {}", e))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0u8; 32]);
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut *key)
        .map_err(|e| anyhow::anyhow!("Argon2id key derivation failed: {}", e))?;
    Ok(key)
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

    fn generate_fresh_seed(_label: &str) -> Zeroizing<[u8; 32]> {
        let mut seed = Zeroizing::new([0u8; 32]);
        OsRng.fill_bytes(&mut *seed);
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
        seed.zeroize();

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

    /// Exports a passphrase-encrypted offline recovery bundle portable across Windows users and machines.
    /// Uses Argon2id (v=0x13, 64 MiB, t=3, p=1) + standard AES-256-GCM AEAD (96-bit OsRng nonce)
    /// with all bundle metadata authenticated as AEAD Associated Data (AAD).
    /// Never writes plaintext Ed25519 seed to disk, JSON, or logs.
    pub fn export_portable_recovery_bundle(
        keystore_dir: &Path,
        backup_file_path: &Path,
        passphrase: &str,
    ) -> Result<PortableProducerKeyRecoveryBundle> {
        if passphrase.trim().len() < 8 {
            bail!("RECOVERY_PASSPHRASE_WEAK: Recovery passphrase must be at least 8 characters");
        }

        let (signing_key, meta) = Self::load_or_create_signing_key(keystore_dir)?;
        let mut seed_bytes = Zeroizing::new(signing_key.to_bytes());
        let backed_up_at_utc = chrono::Utc::now().to_rfc3339();

        let mut salt = [0u8; 16];
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut salt);
        OsRng.fill_bytes(&mut nonce_bytes);

        let kdf_params = Argon2idKdfParams::default();
        let aes_key = derive_argon2id_aes_key(passphrase, &salt, &kdf_params)?;

        let salt_b64 = B64.encode(salt);
        let nonce_b64 = B64.encode(nonce_bytes);

        let mut bundle = PortableProducerKeyRecoveryBundle {
            format_version: PORTABLE_RECOVERY_FORMAT_VERSION,
            protection_scheme: PORTABLE_RECOVERY_SCHEME.to_string(),
            aead_algorithm: PORTABLE_RECOVERY_AEAD.to_string(),
            kdf_params,
            key_id: meta.key_id,
            public_key_hex: meta.public_key_hex,
            created_at_utc: meta.created_at_utc,
            backed_up_at_utc,
            salt_b64,
            nonce_b64,
            ciphertext_b64: String::new(),
        };

        let aad = build_portable_recovery_aad(&bundle)?;
        let cipher = Aes256Gcm::new((&*aes_key).into());
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce_bytes),
                Payload {
                    msg: &*seed_bytes,
                    aad: &aad,
                },
            )
            .map_err(|_| anyhow::anyhow!("AES-256-GCM encryption failed"))?;
        seed_bytes.zeroize();

        bundle.ciphertext_b64 = B64.encode(&ciphertext);

        if let Some(parent) = backup_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(backup_file_path, serde_json::to_vec_pretty(&bundle)?)?;
        Ok(bundle)
    }

    /// Imports an Argon2id + AES-256-GCM portable recovery bundle into the target machine/user's
    /// Windows DPAPI keystore (`WIN32_DPAPI_LOCAL_USER`). Fails closed on wrong passphrase, tampered
    /// metadata/ciphertext, invalid parameters, or retired experimental formats.
    pub fn import_portable_recovery_bundle(
        backup_file_path: &Path,
        keystore_dir: &Path,
        passphrase: &str,
    ) -> Result<ProducerKeyMetadata> {
        if passphrase.is_empty() {
            bail!("RECOVERY_PASSPHRASE_EMPTY: Recovery passphrase is required");
        }
        let raw = std::fs::read(backup_file_path).with_context(|| {
            format!(
                "Failed to read portable key recovery bundle: {}",
                backup_file_path.display()
            )
        })?;

        let raw_val: serde_json::Value = serde_json::from_slice(&raw)
            .context("RECOVERY_INVALID_METADATA: Recovery bundle is not valid JSON")?;
        if let Some(scheme) = raw_val.get("protectionScheme").and_then(|v| v.as_str()) {
            if scheme == PORTABLE_RECOVERY_RETIRED_EXPERIMENTAL_SCHEME {
                bail!(
                    "RECOVERY_RETIRED_EXPERIMENTAL_FORMAT: Experimental format '{}' (SHA-256-CTR) is retired and blocked in production; re-export using {}",
                    scheme,
                    PORTABLE_RECOVERY_SCHEME
                );
            }
        }

        let bundle: PortableProducerKeyRecoveryBundle = serde_json::from_value(raw_val)
            .context("RECOVERY_INVALID_METADATA: Missing or malformed V3 recovery bundle fields")?;

        if bundle.format_version != PORTABLE_RECOVERY_FORMAT_VERSION
            || bundle.protection_scheme != PORTABLE_RECOVERY_SCHEME
        {
            bail!(
                "RECOVERY_UNSUPPORTED_SCHEME: Expected format v{} / '{}', got v{} / '{}'",
                PORTABLE_RECOVERY_FORMAT_VERSION,
                PORTABLE_RECOVERY_SCHEME,
                bundle.format_version,
                bundle.protection_scheme
            );
        }
        if bundle.aead_algorithm != PORTABLE_RECOVERY_AEAD {
            bail!(
                "RECOVERY_INVALID_METADATA: Unsupported AEAD algorithm '{}', expected '{}'",
                bundle.aead_algorithm,
                PORTABLE_RECOVERY_AEAD
            );
        }
        if bundle.kdf_params.algorithm != PORTABLE_RECOVERY_ARGON2_ALG
            || bundle.kdf_params.version != PORTABLE_RECOVERY_ARGON2_VER
            || bundle.kdf_params.output_len_bytes != 32
        {
            bail!(
                "RECOVERY_INVALID_METADATA: Unsupported KDF algorithm/version/output_len ({} / {} / {})",
                bundle.kdf_params.algorithm,
                bundle.kdf_params.version,
                bundle.kdf_params.output_len_bytes
            );
        }
        if bundle.kdf_params.memory_kib < 19_456
            || bundle.kdf_params.memory_kib > 524_288
            || bundle.kdf_params.iterations < 2
            || bundle.kdf_params.iterations > 20
            || bundle.kdf_params.parallelism < 1
            || bundle.kdf_params.parallelism > 16
        {
            bail!("RECOVERY_INVALID_METADATA: Argon2id parameters outside safe policy bounds");
        }
        if bundle.key_id.trim().is_empty() || hex_decode(&bundle.public_key_hex)?.len() != 32 {
            bail!("RECOVERY_INVALID_METADATA: Invalid keyId or publicKeyHex metadata");
        }

        let salt_vec = B64
            .decode(bundle.salt_b64.trim())
            .context("RECOVERY_INVALID_METADATA: Invalid salt base64")?;
        let nonce_vec = B64
            .decode(bundle.nonce_b64.trim())
            .context("RECOVERY_INVALID_METADATA: Invalid nonce base64")?;
        let ciphertext = B64
            .decode(bundle.ciphertext_b64.trim())
            .context("RECOVERY_INVALID_METADATA: Invalid ciphertext base64")?;
        if salt_vec.len() < 16
            || salt_vec.len() > 64
            || nonce_vec.len() != 12
            || ciphertext.len() != 48
        {
            bail!("RECOVERY_MALFORMED_PAYLOAD: Unexpected salt, 96-bit nonce, or AES-GCM ciphertext+tag length");
        }

        let aes_key = derive_argon2id_aes_key(passphrase, &salt_vec, &bundle.kdf_params)?;
        let aad = build_portable_recovery_aad(&bundle)?;
        let cipher = Aes256Gcm::new((&*aes_key).into());
        let plain_vec = Zeroizing::new(
            cipher
                .decrypt(
                    Nonce::from_slice(&nonce_vec),
                    Payload {
                        msg: &ciphertext,
                        aad: &aad,
                    },
                )
                .map_err(|_| {
                    anyhow::anyhow!(
                        "RECOVERY_AUTH_FAILED: Wrong passphrase, tampered ciphertext, or tampered metadata (AES-256-GCM auth failed)"
                    )
                })?,
        );

        if plain_vec.len() != 32 {
            bail!("Invalid decrypted Ed25519 seed length");
        }
        let mut seed = Zeroizing::new([0u8; 32]);
        seed.copy_from_slice(&plain_vec);

        let sk = SigningKey::from_bytes(&seed);
        let derived_pub = hex_encode(sk.verifying_key().as_bytes());
        if !derived_pub.eq_ignore_ascii_case(&bundle.public_key_hex) {
            bail!("Restored public key mismatch against portable recovery metadata");
        }

        let probe_sig = sk.sign(b"farmerpro-portable-recovery-probe");
        let vk = VerifyingKey::from_bytes(sk.verifying_key().as_bytes())?;
        vk.verify_strict(b"farmerpro-portable-recovery-probe", &probe_sig)?;

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
        seed.zeroize();

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

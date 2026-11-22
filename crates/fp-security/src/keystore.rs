use crate::dpapi;
use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const PRODUCER_KEY_DPAPI_ENTROPY: &[u8] = b"FarmerPro.ProducerEd25519.Keystore.v2";
const PRODUCER_KEY_BACKUP_ENTROPY: &[u8] = b"FarmerPro.ProducerEd25519.Backup.v2";
const PORTABLE_RECOVERY_KDF_ITERATIONS: u32 = 200_000;
const PORTABLE_RECOVERY_SCHEME: &str = "PASSPHRASE_PBKDF2_SHA256_ETM_V1";
const PORTABLE_RECOVERY_DOMAIN: &[u8] = b"FarmerPro.ProducerEd25519.PortableRecovery.v1";

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

/// Cross-machine / cross-Windows-user passphrase-encrypted offline recovery bundle.
/// Uses PBKDF2-HMAC-SHA256 (200,000 iterations) + SHA-256-CTR + Encrypt-then-MAC (HMAC-SHA256).
/// Never stores plaintext seed; imports into target machine's Windows DPAPI keystore.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableProducerKeyRecoveryBundle {
    pub format_version: u32,
    pub protection_scheme: String,
    pub kdf_iterations: u32,
    pub key_id: String,
    pub public_key_hex: String,
    pub created_at_utc: String,
    pub backed_up_at_utc: String,
    pub salt_b64: String,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
    pub hmac_sha256: String,
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;
    let mut key_block = [0u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        let mut h = Sha256::new();
        h.update(key);
        key_block[..32].copy_from_slice(&h.finalize());
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; BLOCK_SIZE];
    let mut opad = [0x5cu8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        ipad[i] ^= key_block[i];
        opad[i] ^= key_block[i];
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(data);
    let inner_digest = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_digest);
    let out = outer.finalize();

    key_block.fill(0);
    ipad.fill(0);
    opad.fill(0);

    let mut res = [0u8; 32];
    res.copy_from_slice(&out);
    res
}

fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: u32, out: &mut [u8]) {
    let mut block_index = 1u32;
    let mut offset = 0usize;
    while offset < out.len() {
        let mut u_input = Vec::with_capacity(salt.len() + 4);
        u_input.extend_from_slice(salt);
        u_input.extend_from_slice(&block_index.to_be_bytes());
        let mut u = hmac_sha256(password, &u_input);
        let mut t = u;
        for _ in 1..iterations {
            u = hmac_sha256(password, &u);
            for j in 0..32 {
                t[j] ^= u[j];
            }
        }
        let take = (out.len() - offset).min(32);
        out[offset..offset + take].copy_from_slice(&t[..take]);
        u.fill(0);
        t.fill(0);
        offset += take;
        block_index += 1;
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn xor_sha256_ctr_stream(enc_key: &[u8; 32], nonce: &[u8; 16], input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut counter = 0u32;
    for chunk in input.chunks(32) {
        let mut h = Sha256::new();
        h.update(PORTABLE_RECOVERY_DOMAIN);
        h.update(enc_key);
        h.update(nonce);
        h.update(counter.to_le_bytes());
        let mut ks = h.finalize();
        for (i, &byte) in chunk.iter().enumerate() {
            out.push(byte ^ ks[i]);
        }
        ks.fill(0);
        counter += 1;
    }
    out
}

fn compute_portable_mac(
    mac_key: &[u8; 32],
    key_id: &str,
    public_key_hex: &str,
    created_at_utc: &str,
    backed_up_at_utc: &str,
    kdf_iterations: u32,
    salt_b64: &str,
    nonce_b64: &str,
    ciphertext_b64: &str,
) -> String {
    let canonical = format!(
        "v1|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        PORTABLE_RECOVERY_SCHEME,
        kdf_iterations,
        key_id,
        public_key_hex,
        created_at_utc,
        backed_up_at_utc,
        salt_b64,
        nonce_b64,
        ciphertext_b64
    );
    hex_encode(&hmac_sha256(mac_key, canonical.as_bytes()))
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

    /// Exports a passphrase-encrypted offline recovery bundle portable across Windows users and machines.
    /// Uses PBKDF2-HMAC-SHA256 (200,000 iterations) + SHA-256-CTR + Encrypt-then-MAC (HMAC-SHA256).
    /// Never writes plaintext Ed25519 seed to disk or logs.
    pub fn export_portable_recovery_bundle(
        keystore_dir: &Path,
        backup_file_path: &Path,
        passphrase: &str,
    ) -> Result<PortableProducerKeyRecoveryBundle> {
        if passphrase.trim().len() < 8 {
            bail!("RECOVERY_PASSPHRASE_WEAK: Recovery passphrase must be at least 8 characters");
        }

        let (signing_key, meta) = Self::load_or_create_signing_key(keystore_dir)?;
        let mut seed_bytes = signing_key.to_bytes();
        let backed_up_at_utc = chrono::Utc::now().to_rfc3339();

        // uuid::Uuid::new_v4() uses OS CSPRNG (getrandom / BCryptGenRandom)
        let salt = *uuid::Uuid::new_v4().as_bytes();
        let nonce = *uuid::Uuid::new_v4().as_bytes();

        let mut derived = [0u8; 64];
        pbkdf2_hmac_sha256(
            passphrase.as_bytes(),
            &salt,
            PORTABLE_RECOVERY_KDF_ITERATIONS,
            &mut derived,
        );
        let mut enc_key = [0u8; 32];
        let mut mac_key = [0u8; 32];
        enc_key.copy_from_slice(&derived[..32]);
        mac_key.copy_from_slice(&derived[32..64]);
        derived.fill(0);

        let ciphertext = xor_sha256_ctr_stream(&enc_key, &nonce, &seed_bytes);
        seed_bytes.fill(0);
        enc_key.fill(0);

        let salt_b64 = B64.encode(salt);
        let nonce_b64 = B64.encode(nonce);
        let ciphertext_b64 = B64.encode(&ciphertext);
        let hmac_sha256 = compute_portable_mac(
            &mac_key,
            &meta.key_id,
            &meta.public_key_hex,
            &meta.created_at_utc,
            &backed_up_at_utc,
            PORTABLE_RECOVERY_KDF_ITERATIONS,
            &salt_b64,
            &nonce_b64,
            &ciphertext_b64,
        );
        mac_key.fill(0);

        let bundle = PortableProducerKeyRecoveryBundle {
            format_version: 2,
            protection_scheme: PORTABLE_RECOVERY_SCHEME.to_string(),
            kdf_iterations: PORTABLE_RECOVERY_KDF_ITERATIONS,
            key_id: meta.key_id,
            public_key_hex: meta.public_key_hex,
            created_at_utc: meta.created_at_utc,
            backed_up_at_utc,
            salt_b64,
            nonce_b64,
            ciphertext_b64,
            hmac_sha256,
        };

        if let Some(parent) = backup_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(backup_file_path, serde_json::to_vec_pretty(&bundle)?)?;
        Ok(bundle)
    }

    /// Imports a passphrase-encrypted portable recovery bundle into the target machine/user's
    /// Windows DPAPI keystore (`WIN32_DPAPI_LOCAL_USER`). Fails closed on wrong passphrase or tampering.
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
        let bundle: PortableProducerKeyRecoveryBundle = serde_json::from_slice(&raw)?;

        if bundle.protection_scheme != PORTABLE_RECOVERY_SCHEME {
            bail!(
                "RECOVERY_UNSUPPORTED_SCHEME: Unsupported protection scheme '{}'",
                bundle.protection_scheme
            );
        }
        if bundle.kdf_iterations < 10_000 {
            bail!("RECOVERY_WEAK_KDF: KDF iterations below minimum safety threshold");
        }

        let salt_vec = B64.decode(bundle.salt_b64.trim()).context("Invalid salt base64")?;
        let nonce_vec = B64.decode(bundle.nonce_b64.trim()).context("Invalid nonce base64")?;
        let ciphertext = B64
            .decode(bundle.ciphertext_b64.trim())
            .context("Invalid ciphertext base64")?;
        if salt_vec.len() < 16 || nonce_vec.len() != 16 || ciphertext.len() != 32 {
            bail!("RECOVERY_MALFORMED_PAYLOAD: Unexpected salt, nonce, or ciphertext length");
        }
        let mut nonce = [0u8; 16];
        nonce.copy_from_slice(&nonce_vec);

        let mut derived = [0u8; 64];
        pbkdf2_hmac_sha256(
            passphrase.as_bytes(),
            &salt_vec,
            bundle.kdf_iterations,
            &mut derived,
        );
        let mut enc_key = [0u8; 32];
        let mut mac_key = [0u8; 32];
        enc_key.copy_from_slice(&derived[..32]);
        mac_key.copy_from_slice(&derived[32..64]);
        derived.fill(0);

        let expected_mac = compute_portable_mac(
            &mac_key,
            &bundle.key_id,
            &bundle.public_key_hex,
            &bundle.created_at_utc,
            &bundle.backed_up_at_utc,
            bundle.kdf_iterations,
            &bundle.salt_b64,
            &bundle.nonce_b64,
            &bundle.ciphertext_b64,
        );
        mac_key.fill(0);

        if !constant_time_eq(expected_mac.as_bytes(), bundle.hmac_sha256.as_bytes()) {
            enc_key.fill(0);
            bail!("RECOVERY_AUTH_FAILED: Wrong passphrase or tampered recovery bundle (HMAC mismatch)");
        }

        let mut plain_vec = xor_sha256_ctr_stream(&enc_key, &nonce, &ciphertext);
        enc_key.fill(0);
        if plain_vec.len() != 32 {
            plain_vec.fill(0);
            bail!("Invalid decrypted Ed25519 seed length");
        }
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&plain_vec);
        plain_vec.fill(0);

        let sk = SigningKey::from_bytes(&seed);
        let derived_pub = hex_encode(sk.verifying_key().as_bytes());
        if !derived_pub.eq_ignore_ascii_case(&bundle.public_key_hex) {
            seed.fill(0);
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

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::Engine as _;
use rand::RngCore;
use sha2::Sha256;
use zeroize::ZeroizeOnDrop;

const PBKDF2_ITERATIONS: u32 = 600_000;
const SALT_LEN: usize = 16;
const IV_LEN: usize = 12;
const KEY_LEN: usize = 32;

#[derive(Debug)]
pub enum CryptoError {
    EncryptionFailed(String),
    DecryptionFailed(String),
    InvalidData(String),
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EncryptionFailed(m) => write!(f, "Encryption failed: {m}"),
            Self::DecryptionFailed(m) => write!(f, "Decryption failed: {m}"),
            Self::InvalidData(m) => write!(f, "Invalid data: {m}"),
        }
    }
}

fn derive_key(passphrase: &str, salt: &[u8]) -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    pbkdf2::pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
    key
}

/// A cached encryption key — derives the expensive PBKDF2 key once, reuses for all saves.
/// The salt is fixed per session; a fresh random IV is generated for each encryption.
/// The passphrase is retained so assets encrypted with different salts can be decrypted
/// by re-deriving the key from the asset's stored salt.
/// Key material is securely zeroed when dropped.
#[derive(Clone, ZeroizeOnDrop)]
pub struct CachedKey {
    #[zeroize(skip)]
    salt: [u8; SALT_LEN],
    key: [u8; KEY_LEN],
    passphrase: String,
}

impl std::fmt::Debug for CachedKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedKey")
            .field("key", &"[REDACTED]")
            .field("salt", &self.salt)
            .finish()
    }
}

impl CachedKey {
    /// Derive and cache the key from a passphrase (expensive — call once at unlock/create time).
    pub fn derive(passphrase: &str) -> Self {
        let mut salt = [0u8; SALT_LEN];
        rand::thread_rng().fill_bytes(&mut salt);
        let key = derive_key(passphrase, &salt);
        CachedKey { key, salt, passphrase: passphrase.to_string() }
    }

    /// Construct from already-derived key material (zero cost).
    pub fn from_raw(key: [u8; KEY_LEN], salt: [u8; SALT_LEN], passphrase: &str) -> Self {
        CachedKey { key, salt, passphrase: passphrase.to_string() }
    }
}

/// Fast encryption using a pre-derived cached key. No PBKDF2 on each call.
pub fn encrypt_vault_fast(plaintext: &str, cached: &CachedKey) -> Result<String, CryptoError> {
    encrypt_with_key(plaintext, &cached.key, &cached.salt)
}

fn encrypt_with_key(
    plaintext: &str,
    key: &[u8; KEY_LEN],
    salt: &[u8; SALT_LEN],
) -> Result<String, CryptoError> {
    let mut iv = [0u8; IV_LEN];
    rand::thread_rng().fill_bytes(&mut iv);

    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
    let nonce = Nonce::from_slice(&iv);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    let mut combined = Vec::with_capacity(SALT_LEN + IV_LEN + ciphertext.len());
    combined.extend_from_slice(salt);
    combined.extend_from_slice(&iv);
    combined.extend_from_slice(&ciphertext);

    let b64 = base64::engine::general_purpose::STANDARD.encode(&combined);
    Ok(format!(r#"{{"encrypted":true,"data":"{b64}"}}"#))
}

/// Decrypt vault and derive a fresh CachedKey for future saves.
pub fn decrypt_vault_returning_key(
    encrypted_json: &str,
    passphrase: &str,
) -> Result<(String, CachedKey), CryptoError> {
    let envelope: serde_json::Value =
        serde_json::from_str(encrypted_json).map_err(|e| CryptoError::InvalidData(e.to_string()))?;

    let is_encrypted = envelope
        .get("encrypted")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !is_encrypted {
        let plaintext = envelope
            .get("data")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| serde_json::to_string(&envelope).ok())
            .ok_or_else(|| CryptoError::InvalidData("No data field".into()))?;
        let cached = CachedKey::derive(passphrase);
        return Ok((plaintext, cached));
    }

    let data_b64 = envelope
        .get("data")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CryptoError::InvalidData("Missing 'data' field".into()))?;

    let combined = base64::engine::general_purpose::STANDARD
        .decode(data_b64)
        .map_err(|e| CryptoError::InvalidData(e.to_string()))?;

    if combined.len() < SALT_LEN + IV_LEN + 1 {
        return Err(CryptoError::InvalidData("Data too short".into()));
    }

    let salt = &combined[..SALT_LEN];
    let iv = &combined[SALT_LEN..SALT_LEN + IV_LEN];
    let ciphertext = &combined[SALT_LEN + IV_LEN..];

    let key = derive_key(passphrase, salt);
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;
    let nonce = Nonce::from_slice(iv);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::DecryptionFailed("Wrong passphrase or corrupted data".into()))?;

    let plaintext_str =
        String::from_utf8(plaintext).map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

    let cached = CachedKey::derive(passphrase);

    Ok((plaintext_str, cached))
}

/// Encrypt binary asset data using a pre-derived cached key.
pub fn encrypt_asset(data: &[u8], cached: &CachedKey) -> Result<String, CryptoError> {
    let mut iv = [0u8; IV_LEN];
    rand::thread_rng().fill_bytes(&mut iv);

    let cipher = Aes256Gcm::new_from_slice(&cached.key)
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
    let nonce = Nonce::from_slice(&iv);
    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    let mut combined = Vec::with_capacity(SALT_LEN + IV_LEN + ciphertext.len());
    combined.extend_from_slice(&cached.salt);
    combined.extend_from_slice(&iv);
    combined.extend_from_slice(&ciphertext);

    let b64 = base64::engine::general_purpose::STANDARD.encode(&combined);
    Ok(format!(r#"{{"encrypted":true,"data":"{b64}"}}"#))
}

/// Decrypt binary asset bytes using a cached key.
pub fn decrypt_asset(data: &[u8], cached: &CachedKey) -> Result<Vec<u8>, CryptoError> {
    let Ok(as_text) = std::str::from_utf8(data) else {
        return Ok(data.to_vec());
    };

    let Ok(envelope) = serde_json::from_str::<serde_json::Value>(as_text) else {
        return Ok(data.to_vec());
    };

    let is_encrypted = envelope
        .get("encrypted")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !is_encrypted {
        return Ok(data.to_vec());
    }

    let data_b64 = envelope
        .get("data")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CryptoError::InvalidData("Missing encrypted asset data".into()))?;

    let combined = base64::engine::general_purpose::STANDARD
        .decode(data_b64)
        .map_err(|e| CryptoError::InvalidData(e.to_string()))?;

    if combined.len() < SALT_LEN + IV_LEN + 1 {
        return Err(CryptoError::InvalidData("Encrypted asset data too short".into()));
    }

    let asset_salt = &combined[..SALT_LEN];
    let iv = &combined[SALT_LEN..SALT_LEN + IV_LEN];
    let ciphertext = &combined[SALT_LEN + IV_LEN..];

    let key = derive_key(&cached.passphrase, asset_salt);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;
    let nonce = Nonce::from_slice(iv);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::DecryptionFailed("Wrong key or corrupted asset".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unencrypted_fallback() {
        let envelope = r#"{"encrypted":false,"data":"{\"tree\":[]}"}"#;
        let (result, _key) = decrypt_vault_returning_key(envelope, "any").expect("decrypt");
        assert_eq!(result, r#"{"tree":[]}"#);
    }

    #[test]
    fn cached_key_roundtrip() {
        let plaintext = r#"{"tree":[],"trash":[]}"#;
        let pass = "cached-key-test";
        let cached = CachedKey::derive(pass);
        let encrypted = encrypt_vault_fast(plaintext, &cached).expect("encrypt");
        assert!(encrypted.contains("\"encrypted\":true"));
        let (decrypted, _key) = decrypt_vault_returning_key(&encrypted, pass).expect("decrypt");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_returning_key_roundtrip() {
        let plaintext = r#"{"tree":[],"trash":[]}"#;
        let pass = "roundtrip-test-pass";
        let cached = CachedKey::derive(pass);
        let encrypted = encrypt_vault_fast(plaintext, &cached).expect("encrypt");
        let (decrypted, reused_key) =
            decrypt_vault_returning_key(&encrypted, pass).expect("decrypt");
        assert_eq!(decrypted, plaintext);
        let re_encrypted = encrypt_vault_fast(plaintext, &reused_key).expect("re-encrypt");
        let (re_decrypted, _) = decrypt_vault_returning_key(&re_encrypted, pass).expect("re-decrypt");
        assert_eq!(re_decrypted, plaintext);
    }
}

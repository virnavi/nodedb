use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use sha2::{Sha256, Digest};
use x25519_dalek::{StaticSecret as X25519Secret, PublicKey as X25519Public};

use hkdf::Hkdf;

use crate::error::CryptoError;

/// Derive a 32-byte shared secret from X25519 key exchange, then hash with SHA-256.
pub fn derive_shared_secret(my_secret: &X25519Secret, their_public: &X25519Public) -> [u8; 32] {
    let shared = my_secret.diffie_hellman(their_public);
    let mut hasher = Sha256::new();
    hasher.update(shared.as_bytes());
    let hash = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash);
    key
}

/// Encrypt plaintext with AES-256-GCM. Returns (nonce, ciphertext).
pub fn aes_256_gcm_encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CryptoError::Encryption(e.to_string()))?;

    let mut nonce_bytes = [0u8; 12];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CryptoError::Encryption(e.to_string()))?;

    Ok((nonce_bytes.to_vec(), ciphertext))
}

/// Decrypt ciphertext with AES-256-GCM.
pub fn aes_256_gcm_decrypt(
    key: &[u8; 32],
    nonce: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    if nonce.len() != 12 {
        return Err(CryptoError::Decryption(
            format!("expected 12-byte nonce, got {}", nonce.len()),
        ));
    }
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CryptoError::Decryption(e.to_string()))?;

    let nonce = Nonce::from_slice(nonce);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CryptoError::Decryption(e.to_string()))
}

/// Derive a 256-bit key using HKDF-SHA256.
///
/// - `master_key`: the database DEK (32 bytes)
/// - `info`: context string, e.g. `"prefs:theme"` or `"prefs:locale"`
///
/// Returns a 32-byte derived key suitable for AES-256-GCM.
pub fn hkdf_derive_key(master_key: &[u8; 32], info: &str) -> Result<[u8; 32], CryptoError> {
    let hk = Hkdf::<Sha256>::new(None, master_key);
    let mut okm = [0u8; 32];
    hk.expand(info.as_bytes(), &mut okm)
        .map_err(|e| CryptoError::Encryption(format!("HKDF expand failed: {}", e)))?;
    Ok(okm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = [42u8; 32];
        let plaintext = b"hello, world!";
        let (nonce, ciphertext) = aes_256_gcm_encrypt(&key, plaintext).unwrap();
        assert_ne!(ciphertext, plaintext);
        let decrypted = aes_256_gcm_decrypt(&key, &nonce, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_with_wrong_key_fails() {
        let key1 = [1u8; 32];
        let key2 = [2u8; 32];
        let (nonce, ciphertext) = aes_256_gcm_encrypt(&key1, b"secret").unwrap();
        let result = aes_256_gcm_decrypt(&key2, &nonce, &ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn decrypt_with_wrong_nonce_fails() {
        let key = [1u8; 32];
        let (_, ciphertext) = aes_256_gcm_encrypt(&key, b"secret").unwrap();
        let wrong_nonce = [0u8; 12];
        let result = aes_256_gcm_decrypt(&key, &wrong_nonce, &ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_nonce_length() {
        let key = [1u8; 32];
        let result = aes_256_gcm_decrypt(&key, &[0u8; 8], &[]);
        assert!(result.is_err());
    }

    #[test]
    fn shared_secret_derivation() {
        use x25519_dalek::StaticSecret;
        let secret_a = StaticSecret::random_from_rng(rand::thread_rng());
        let public_a = x25519_dalek::PublicKey::from(&secret_a);
        let secret_b = StaticSecret::random_from_rng(rand::thread_rng());
        let public_b = x25519_dalek::PublicKey::from(&secret_b);

        let shared_ab = derive_shared_secret(&secret_a, &public_b);
        let shared_ba = derive_shared_secret(&secret_b, &public_a);
        assert_eq!(shared_ab, shared_ba);
    }

    #[test]
    fn hkdf_deterministic() {
        let master = [1u8; 32];
        let k1 = hkdf_derive_key(&master, "prefs:theme").unwrap();
        let k2 = hkdf_derive_key(&master, "prefs:theme").unwrap();
        assert_eq!(k1, k2);
    }

    #[test]
    fn hkdf_different_info_different_keys() {
        let master = [1u8; 32];
        let k1 = hkdf_derive_key(&master, "prefs:theme").unwrap();
        let k2 = hkdf_derive_key(&master, "prefs:locale").unwrap();
        assert_ne!(k1, k2);
    }

    #[test]
    fn hkdf_different_master_different_keys() {
        let k1 = hkdf_derive_key(&[1u8; 32], "prefs:theme").unwrap();
        let k2 = hkdf_derive_key(&[2u8; 32], "prefs:theme").unwrap();
        assert_ne!(k1, k2);
    }

    #[test]
    fn hkdf_encrypt_decrypt_roundtrip() {
        let master = [42u8; 32];
        let derived = hkdf_derive_key(&master, "prefs:secret").unwrap();
        let plaintext = b"preference value";
        let (nonce, ct) = aes_256_gcm_encrypt(&derived, plaintext).unwrap();
        let pt = aes_256_gcm_decrypt(&derived, &nonce, &ct).unwrap();
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn hkdf_empty_info() {
        let master = [1u8; 32];
        let k = hkdf_derive_key(&master, "").unwrap();
        assert_eq!(k.len(), 32);
        // Different from non-empty info
        let k2 = hkdf_derive_key(&master, "something").unwrap();
        assert_ne!(k, k2);
    }

    #[test]
    fn shared_secret_encrypt_decrypt() {
        use x25519_dalek::StaticSecret;
        let secret_a = StaticSecret::random_from_rng(rand::thread_rng());
        let public_a = x25519_dalek::PublicKey::from(&secret_a);
        let secret_b = StaticSecret::random_from_rng(rand::thread_rng());
        let public_b = x25519_dalek::PublicKey::from(&secret_b);

        let key = derive_shared_secret(&secret_a, &public_b);
        let (nonce, ct) = aes_256_gcm_encrypt(&key, b"federation data").unwrap();

        let key2 = derive_shared_secret(&secret_b, &public_a);
        let pt = aes_256_gcm_decrypt(&key2, &nonce, &ct).unwrap();
        assert_eq!(pt, b"federation data");
    }
}

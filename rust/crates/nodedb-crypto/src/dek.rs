use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use ed25519_dalek::{SigningKey, VerifyingKey};
use sha2::{Sha256, Sha512, Digest};
use x25519_dalek::{StaticSecret as X25519Secret, PublicKey as X25519Public};

use crate::error::CryptoError;

/// Seal a 256-bit Data Encryption Key (DEK) to an owner's Ed25519 public key using ECIES.
///
/// Uses an ephemeral X25519 keypair for key agreement with the owner's public key
/// (converted from Ed25519 to X25519 via `to_montgomery()`). The shared secret is
/// hashed with SHA-256 to derive an AES-256-GCM key, which encrypts the DEK.
///
/// Returns: `[ephemeral_public_key (32) | nonce (12) | ciphertext]`
pub fn seal_dek(
    owner_public_key_bytes: &[u8; 32],
    dek: &[u8; 32],
) -> Result<Vec<u8>, CryptoError> {
    // Convert owner Ed25519 public key to X25519
    let vk = VerifyingKey::from_bytes(owner_public_key_bytes)
        .map_err(|e| CryptoError::InvalidKeyMaterial(e.to_string()))?;
    let owner_x25519 = X25519Public::from(vk.to_montgomery().to_bytes());

    // Generate ephemeral X25519 keypair
    let ephemeral_secret = X25519Secret::random_from_rng(rand::thread_rng());
    let ephemeral_public = X25519Public::from(&ephemeral_secret);

    // DH → SHA-256 → AES key
    let shared = ephemeral_secret.diffie_hellman(&owner_x25519);
    let mut hasher = Sha256::new();
    hasher.update(shared.as_bytes());
    let aes_key: [u8; 32] = hasher.finalize().into();

    // AES-256-GCM encrypt the DEK
    let cipher = Aes256Gcm::new_from_slice(&aes_key)
        .map_err(|e| CryptoError::Encryption(e.to_string()))?;
    let mut nonce_bytes = [0u8; 12];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, dek.as_ref())
        .map_err(|e| CryptoError::Encryption(e.to_string()))?;

    // Pack: [ephemeral_public (32) | nonce (12) | ciphertext]
    let mut sealed = Vec::with_capacity(32 + 12 + ciphertext.len());
    sealed.extend_from_slice(ephemeral_public.as_bytes());
    sealed.extend_from_slice(&nonce_bytes);
    sealed.extend_from_slice(&ciphertext);

    Ok(sealed)
}

/// Unseal a DEK that was sealed to an owner's Ed25519 private key using ECIES.
///
/// Parses the ephemeral public key and nonce from the sealed blob, derives the
/// same shared secret using the owner's private key, and decrypts the DEK.
pub fn unseal_dek(
    owner_private_key_bytes: &[u8; 32],
    sealed: &[u8],
) -> Result<[u8; 32], CryptoError> {
    // Minimum: 32 (ephemeral pub) + 12 (nonce) + 32 (DEK) + 16 (GCM tag)
    if sealed.len() < 92 {
        return Err(CryptoError::Decryption(
            format!("sealed DEK too short: {} bytes, expected >= 92", sealed.len()),
        ));
    }

    // Parse components
    let ephemeral_pub_bytes: [u8; 32] = sealed[..32]
        .try_into()
        .map_err(|_| CryptoError::Decryption("invalid ephemeral public key".into()))?;
    let nonce_bytes = &sealed[32..44];
    let ciphertext = &sealed[44..];

    let ephemeral_public = X25519Public::from(ephemeral_pub_bytes);

    // Convert owner Ed25519 private key to X25519 secret
    let signing_key = SigningKey::from_bytes(owner_private_key_bytes);
    let mut hasher = Sha512::new();
    hasher.update(signing_key.to_bytes());
    let hash = hasher.finalize();
    let mut secret_bytes = [0u8; 32];
    secret_bytes.copy_from_slice(&hash[..32]);
    let owner_x25519_secret = X25519Secret::from(secret_bytes);

    // DH → SHA-256 → AES key
    let shared = owner_x25519_secret.diffie_hellman(&ephemeral_public);
    let mut hasher = Sha256::new();
    hasher.update(shared.as_bytes());
    let aes_key: [u8; 32] = hasher.finalize().into();

    // AES-256-GCM decrypt
    let cipher = Aes256Gcm::new_from_slice(&aes_key)
        .map_err(|e| CryptoError::Decryption(e.to_string()))?;
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CryptoError::Decryption(e.to_string()))?;

    if plaintext.len() != 32 {
        return Err(CryptoError::Decryption(
            format!("decrypted DEK has wrong length: {}, expected 32", plaintext.len()),
        ));
    }

    let mut dek = [0u8; 32];
    dek.copy_from_slice(&plaintext);
    Ok(dek)
}

/// Compute a fingerprint of an Ed25519 public key (SHA-256, hex-encoded).
pub fn fingerprint(public_key_bytes: &[u8; 32]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(public_key_bytes);
    let hash = hasher.finalize();
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NodeIdentity;

    #[test]
    fn seal_unseal_roundtrip() {
        let identity = NodeIdentity::generate();
        let dek = [42u8; 32];

        let sealed = seal_dek(&identity.verifying_key_bytes(), &dek).unwrap();
        let unsealed = unseal_dek(&identity.signing_key_bytes(), &sealed).unwrap();

        assert_eq!(unsealed, dek);
    }

    #[test]
    fn wrong_key_fails_unseal() {
        let owner = NodeIdentity::generate();
        let wrong = NodeIdentity::generate();
        let dek = [99u8; 32];

        let sealed = seal_dek(&owner.verifying_key_bytes(), &dek).unwrap();
        let result = unseal_dek(&wrong.signing_key_bytes(), &sealed);

        assert!(result.is_err());
    }

    #[test]
    fn sealed_blob_has_expected_structure() {
        let identity = NodeIdentity::generate();
        let dek = [0u8; 32];

        let sealed = seal_dek(&identity.verifying_key_bytes(), &dek).unwrap();

        // 32 (ephemeral pub) + 12 (nonce) + 32 (DEK) + 16 (GCM auth tag) = 92
        assert_eq!(sealed.len(), 92);
    }

    #[test]
    fn different_seals_produce_different_blobs() {
        let identity = NodeIdentity::generate();
        let dek = [1u8; 32];

        let sealed1 = seal_dek(&identity.verifying_key_bytes(), &dek).unwrap();
        let sealed2 = seal_dek(&identity.verifying_key_bytes(), &dek).unwrap();

        // Ephemeral key and nonce differ each time
        assert_ne!(sealed1, sealed2);

        // But both unseal to the same DEK
        let d1 = unseal_dek(&identity.signing_key_bytes(), &sealed1).unwrap();
        let d2 = unseal_dek(&identity.signing_key_bytes(), &sealed2).unwrap();
        assert_eq!(d1, dek);
        assert_eq!(d2, dek);
    }

    #[test]
    fn too_short_sealed_fails() {
        let result = unseal_dek(&[0u8; 32], &[0u8; 50]);
        assert!(result.is_err());
    }

    #[test]
    fn fingerprint_deterministic() {
        let key = [7u8; 32];
        let fp1 = fingerprint(&key);
        let fp2 = fingerprint(&key);
        assert_eq!(fp1, fp2);
        assert_eq!(fp1.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn fingerprint_different_keys_differ() {
        let fp1 = fingerprint(&[1u8; 32]);
        let fp2 = fingerprint(&[2u8; 32]);
        assert_ne!(fp1, fp2);
    }
}

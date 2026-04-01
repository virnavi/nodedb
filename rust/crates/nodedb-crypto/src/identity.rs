use ed25519_dalek::{Signer, Verifier, Signature, SigningKey, VerifyingKey};
use sha2::{Sha512, Digest};
use x25519_dalek::{StaticSecret as X25519Secret, PublicKey as X25519Public};

use crate::error::CryptoError;
use crate::types::{NodeIdentity, PublicIdentity};

impl NodeIdentity {
    /// Generate a new random identity.
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let signing_key = SigningKey::generate(&mut rng);
        let verifying_key = signing_key.verifying_key();
        let peer_id = hex::encode(verifying_key.as_bytes());
        NodeIdentity {
            signing_key,
            verifying_key,
            peer_id,
        }
    }

    /// Reconstruct an identity from raw signing key bytes (32 bytes).
    pub fn from_signing_key_bytes(bytes: &[u8; 32]) -> Result<Self, CryptoError> {
        let signing_key = SigningKey::from_bytes(bytes);
        let verifying_key = signing_key.verifying_key();
        let peer_id = hex::encode(verifying_key.as_bytes());
        Ok(NodeIdentity {
            signing_key,
            verifying_key,
            peer_id,
        })
    }

    /// Get the hex-encoded peer ID (public key).
    pub fn peer_id(&self) -> &str {
        &self.peer_id
    }

    /// Get the raw signing key bytes.
    pub fn signing_key_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Get the raw verifying (public) key bytes.
    pub fn verifying_key_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Extract the public identity (safe to transmit).
    pub fn to_public(&self) -> PublicIdentity {
        PublicIdentity {
            peer_id: self.peer_id.clone(),
            public_key_bytes: self.verifying_key.as_bytes().to_vec(),
        }
    }

    /// Sign arbitrary data with this identity's Ed25519 key.
    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        let sig = self.signing_key.sign(data);
        sig.to_bytes().to_vec()
    }

    /// Convert Ed25519 signing key to X25519 static secret for key exchange.
    pub fn to_x25519_secret(&self) -> X25519Secret {
        // Ed25519 signing key -> SHA-512 -> first 32 bytes clamped = X25519 secret
        let mut hasher = Sha512::new();
        hasher.update(self.signing_key.to_bytes());
        let hash = hasher.finalize();
        let mut secret_bytes = [0u8; 32];
        secret_bytes.copy_from_slice(&hash[..32]);
        X25519Secret::from(secret_bytes)
    }
}

impl PublicIdentity {
    /// Reconstruct a PublicIdentity from raw public key bytes.
    pub fn from_public_key_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() != 32 {
            return Err(CryptoError::InvalidKeyMaterial(
                format!("expected 32 bytes, got {}", bytes.len()),
            ));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        let verifying_key = VerifyingKey::from_bytes(&arr)
            .map_err(|e| CryptoError::InvalidKeyMaterial(e.to_string()))?;
        let peer_id = hex::encode(verifying_key.as_bytes());
        Ok(PublicIdentity {
            peer_id,
            public_key_bytes: bytes.to_vec(),
        })
    }

    /// Verify a signature against this public identity.
    pub fn verify(&self, data: &[u8], signature: &[u8]) -> Result<(), CryptoError> {
        if signature.len() != 64 {
            return Err(CryptoError::Verification(
                format!("expected 64-byte signature, got {}", signature.len()),
            ));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&self.public_key_bytes);
        let verifying_key = VerifyingKey::from_bytes(&arr)
            .map_err(|e| CryptoError::Verification(e.to_string()))?;

        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(signature);
        let sig = Signature::from_bytes(&sig_bytes);

        verifying_key
            .verify(data, &sig)
            .map_err(|e| CryptoError::Verification(e.to_string()))
    }

    /// Convert Ed25519 public key to X25519 public key for key exchange.
    pub fn to_x25519_public(&self) -> Result<X25519Public, CryptoError> {
        use ed25519_dalek::VerifyingKey;
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&self.public_key_bytes);
        let vk = VerifyingKey::from_bytes(&arr)
            .map_err(|e| CryptoError::InvalidKeyMaterial(e.to_string()))?;
        let ep = vk.to_montgomery();
        Ok(X25519Public::from(ep.to_bytes()))
    }
}

/// Helper to hex-encode bytes (avoids pulling in hex crate).
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_produces_valid_identity() {
        let id = NodeIdentity::generate();
        assert_eq!(id.peer_id().len(), 64); // 32 bytes hex = 64 chars
        assert_eq!(id.verifying_key_bytes().len(), 32);
        assert_eq!(id.signing_key_bytes().len(), 32);
    }

    #[test]
    fn from_signing_key_bytes_roundtrip() {
        let id1 = NodeIdentity::generate();
        let bytes = id1.signing_key_bytes();
        let id2 = NodeIdentity::from_signing_key_bytes(&bytes).unwrap();
        assert_eq!(id1.peer_id(), id2.peer_id());
        assert_eq!(id1.verifying_key_bytes(), id2.verifying_key_bytes());
    }

    #[test]
    fn sign_and_verify() {
        let id = NodeIdentity::generate();
        let data = b"hello world";
        let sig = id.sign(data);
        assert_eq!(sig.len(), 64);

        let public = id.to_public();
        public.verify(data, &sig).unwrap();
    }

    #[test]
    fn verify_wrong_data_fails() {
        let id = NodeIdentity::generate();
        let sig = id.sign(b"correct data");
        let public = id.to_public();
        let result = public.verify(b"wrong data", &sig);
        assert!(result.is_err());
    }

    #[test]
    fn verify_wrong_key_fails() {
        let id1 = NodeIdentity::generate();
        let id2 = NodeIdentity::generate();
        let sig = id1.sign(b"data");
        let result = id2.to_public().verify(b"data", &sig);
        assert!(result.is_err());
    }

    #[test]
    fn to_public_identity() {
        let id = NodeIdentity::generate();
        let public = id.to_public();
        assert_eq!(public.peer_id, id.peer_id());
        assert_eq!(public.public_key_bytes, id.verifying_key_bytes().to_vec());
    }

    #[test]
    fn public_identity_from_bytes() {
        let id = NodeIdentity::generate();
        let pi = PublicIdentity::from_public_key_bytes(&id.verifying_key_bytes()).unwrap();
        assert_eq!(pi.peer_id, id.peer_id());
    }

    #[test]
    fn public_identity_from_invalid_bytes() {
        let result = PublicIdentity::from_public_key_bytes(&[0u8; 16]);
        assert!(result.is_err());
    }

    #[test]
    fn x25519_conversion() {
        let id = NodeIdentity::generate();
        let _secret = id.to_x25519_secret();
        let _public = id.to_public().to_x25519_public().unwrap();
    }
}

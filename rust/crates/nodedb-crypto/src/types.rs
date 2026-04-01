use ed25519_dalek::{SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

/// A node's cryptographic identity derived from an Ed25519 key pair.
/// The signing key is held in memory only and never persisted.
/// Clone is supported but Serialize is NOT — the private key must never leave memory.
#[derive(Clone)]
pub struct NodeIdentity {
    pub(crate) signing_key: SigningKey,
    pub(crate) verifying_key: VerifyingKey,
    pub(crate) peer_id: String,
}

/// The public portion of a NodeIdentity, safe to transmit and persist.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PublicIdentity {
    pub peer_id: String,
    pub public_key_bytes: Vec<u8>,
}

/// An encrypted envelope for per-message encryption.
/// Contains the ciphertext, a random nonce, and the sender's public key
/// so the recipient can derive the shared secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedEnvelope {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub sender_public_key: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_identity_serde_roundtrip() {
        let pi = PublicIdentity {
            peer_id: "abcd1234".to_string(),
            public_key_bytes: vec![1, 2, 3, 4, 5],
        };
        let bytes = rmp_serde::to_vec(&pi).unwrap();
        let decoded: PublicIdentity = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded, pi);
    }

    #[test]
    fn encrypted_envelope_serde_roundtrip() {
        let env = EncryptedEnvelope {
            nonce: vec![0u8; 12],
            ciphertext: vec![1, 2, 3],
            sender_public_key: vec![4, 5, 6],
        };
        let bytes = rmp_serde::to_vec(&env).unwrap();
        let decoded: EncryptedEnvelope = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.nonce, env.nonce);
        assert_eq!(decoded.ciphertext, env.ciphertext);
        assert_eq!(decoded.sender_public_key, env.sender_public_key);
    }
}

use crate::encryption::{aes_256_gcm_decrypt, aes_256_gcm_encrypt, derive_shared_secret};
use crate::error::CryptoError;
use crate::types::{EncryptedEnvelope, NodeIdentity, PublicIdentity};

/// Encrypt plaintext for a specific recipient using X25519 + AES-256-GCM.
///
/// The sender's Ed25519 key is converted to X25519, a shared secret is derived
/// with the recipient's public key, and the plaintext is encrypted with AES-256-GCM.
pub fn seal_envelope(
    sender: &NodeIdentity,
    recipient: &PublicIdentity,
    plaintext: &[u8],
) -> Result<EncryptedEnvelope, CryptoError> {
    let my_secret = sender.to_x25519_secret();
    let their_public = recipient.to_x25519_public()?;
    let shared_key = derive_shared_secret(&my_secret, &their_public);

    let (nonce, ciphertext) = aes_256_gcm_encrypt(&shared_key, plaintext)?;

    Ok(EncryptedEnvelope {
        nonce,
        ciphertext,
        sender_public_key: sender.verifying_key_bytes().to_vec(),
    })
}

/// Decrypt an envelope using the recipient's identity.
///
/// The sender's public key (embedded in the envelope) is used with the recipient's
/// private key to derive the same shared secret, then AES-256-GCM decrypts.
pub fn open_envelope(
    recipient: &NodeIdentity,
    envelope: &EncryptedEnvelope,
) -> Result<Vec<u8>, CryptoError> {
    let sender_public = PublicIdentity::from_public_key_bytes(&envelope.sender_public_key)?;
    let their_public = sender_public.to_x25519_public()?;
    let my_secret = recipient.to_x25519_secret();
    let shared_key = derive_shared_secret(&my_secret, &their_public);

    aes_256_gcm_decrypt(&shared_key, &envelope.nonce, &envelope.ciphertext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_open_roundtrip() {
        let alice = NodeIdentity::generate();
        let bob = NodeIdentity::generate();
        let plaintext = b"secret message from alice to bob";

        let envelope = seal_envelope(&alice, &bob.to_public(), plaintext).unwrap();

        // Bob can decrypt
        let decrypted = open_envelope(&bob, &envelope).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_recipient_cannot_open() {
        let alice = NodeIdentity::generate();
        let bob = NodeIdentity::generate();
        let eve = NodeIdentity::generate();

        let envelope = seal_envelope(&alice, &bob.to_public(), b"for bob only").unwrap();

        // Eve cannot decrypt
        let result = open_envelope(&eve, &envelope);
        assert!(result.is_err());
    }

    #[test]
    fn bidirectional_communication() {
        let alice = NodeIdentity::generate();
        let bob = NodeIdentity::generate();

        // Alice -> Bob
        let env1 = seal_envelope(&alice, &bob.to_public(), b"hello bob").unwrap();
        let msg1 = open_envelope(&bob, &env1).unwrap();
        assert_eq!(msg1, b"hello bob");

        // Bob -> Alice
        let env2 = seal_envelope(&bob, &alice.to_public(), b"hello alice").unwrap();
        let msg2 = open_envelope(&alice, &env2).unwrap();
        assert_eq!(msg2, b"hello alice");
    }

    #[test]
    fn envelope_contains_sender_public_key() {
        let alice = NodeIdentity::generate();
        let bob = NodeIdentity::generate();

        let envelope = seal_envelope(&alice, &bob.to_public(), b"data").unwrap();
        assert_eq!(envelope.sender_public_key, alice.verifying_key_bytes().to_vec());
    }

    #[test]
    fn empty_plaintext() {
        let alice = NodeIdentity::generate();
        let bob = NodeIdentity::generate();

        let envelope = seal_envelope(&alice, &bob.to_public(), b"").unwrap();
        let decrypted = open_envelope(&bob, &envelope).unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn large_plaintext() {
        let alice = NodeIdentity::generate();
        let bob = NodeIdentity::generate();
        let plaintext = vec![0xAB; 1024 * 1024]; // 1MB

        let envelope = seal_envelope(&alice, &bob.to_public(), &plaintext).unwrap();
        let decrypted = open_envelope(&bob, &envelope).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}

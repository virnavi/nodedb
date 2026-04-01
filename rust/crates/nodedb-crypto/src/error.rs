use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("key generation failed: {0}")]
    KeyGeneration(String),

    #[error("signing error: {0}")]
    Signing(String),

    #[error("verification failed: {0}")]
    Verification(String),

    #[error("encryption failed: {0}")]
    Encryption(String),

    #[error("decryption failed: {0}")]
    Decryption(String),

    #[error("invalid key material: {0}")]
    InvalidKeyMaterial(String),
}

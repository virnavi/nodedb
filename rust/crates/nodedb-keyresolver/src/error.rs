use nodedb_storage::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum KeyResolverError {
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("key not found for pki_id={0}, user_id={1}")]
    KeyNotFound(String, String),

    #[error("entry not found: {0}")]
    EntryNotFound(i64),

    #[error("invalid public key hex: {0}")]
    InvalidPublicKeyHex(String),

    #[error("key expired for pki_id={0}, user_id={1}")]
    KeyExpired(String, String),
}

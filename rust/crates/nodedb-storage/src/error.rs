use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("key not found")]
    NotFound,

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("storage backend error: {0}")]
    Backend(String),

    #[error("transaction error: {0}")]
    Transaction(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("encryption error: {0}")]
    Encryption(String),

    #[error("decryption error: {0}")]
    Decryption(String),

    #[error("invalid database name: {0}")]
    InvalidDatabaseName(String),
}

impl From<sled::Error> for StorageError {
    fn from(e: sled::Error) -> Self {
        StorageError::Backend(e.to_string())
    }
}

impl From<rmp_serde::encode::Error> for StorageError {
    fn from(e: rmp_serde::encode::Error) -> Self {
        StorageError::Serialization(e.to_string())
    }
}

impl From<rmp_serde::decode::Error> for StorageError {
    fn from(e: rmp_serde::decode::Error) -> Self {
        StorageError::Serialization(e.to_string())
    }
}

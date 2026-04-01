use thiserror::Error;

#[derive(Error, Debug)]
pub enum DacError {
    #[error("access rule not found: id={0}")]
    RuleNotFound(i64),

    #[error("invalid collection: {0}")]
    InvalidCollection(String),

    #[error("invalid document: expected map")]
    InvalidDocument,

    #[error("storage error: {0}")]
    Storage(#[from] nodedb_storage::StorageError),

    #[error("serialization error: {0}")]
    Serialization(String),
}

impl From<rmp_serde::encode::Error> for DacError {
    fn from(e: rmp_serde::encode::Error) -> Self {
        DacError::Serialization(e.to_string())
    }
}

impl From<rmp_serde::decode::Error> for DacError {
    fn from(e: rmp_serde::decode::Error) -> Self {
        DacError::Serialization(e.to_string())
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum VectorError {
    #[error("vector not found: id={0}")]
    VectorNotFound(i64),

    #[error("dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("invalid dimension: {0}")]
    InvalidDimension(usize),

    #[error("invalid distance metric: {0}")]
    InvalidMetric(String),

    #[error("index error: {0}")]
    Index(String),

    #[error("storage error: {0}")]
    Storage(#[from] nodedb_storage::StorageError),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("search error: {0}")]
    Search(String),
}

impl From<rmp_serde::encode::Error> for VectorError {
    fn from(e: rmp_serde::encode::Error) -> Self {
        VectorError::Serialization(e.to_string())
    }
}

impl From<rmp_serde::decode::Error> for VectorError {
    fn from(e: rmp_serde::decode::Error) -> Self {
        VectorError::Serialization(e.to_string())
    }
}

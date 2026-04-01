use nodedb_storage::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum ProvenanceError {
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("invalid confidence: {0}")]
    InvalidConfidence(String),

    #[error("verification error: {0}")]
    Verification(String),

    #[error("canonical serialization error: {0}")]
    Canonical(String),

    #[error("provenance envelope not found: {0}")]
    NotFound(i64),
}

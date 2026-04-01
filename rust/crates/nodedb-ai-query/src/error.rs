use thiserror::Error;

#[derive(Error, Debug)]
pub enum AiQueryError {
    #[error("provenance error: {0}")]
    Provenance(#[from] nodedb_provenance::ProvenanceError),

    #[error("nosql error: {0}")]
    NoSql(#[from] nodedb_nosql::NoSqlError),

    #[error("schema validation failed: {0}")]
    SchemaValidation(String),

    #[error("confidence {0} below threshold {1}")]
    ConfidenceBelowThreshold(f64, f64),

    #[error("collection not enabled: {0}")]
    CollectionNotEnabled(String),

    #[error("storage error: {0}")]
    Storage(#[from] nodedb_storage::StorageError),
}

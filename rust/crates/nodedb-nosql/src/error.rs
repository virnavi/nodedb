use thiserror::Error;

#[derive(Error, Debug)]
pub enum NoSqlError {
    #[error("document not found: id={0}")]
    DocumentNotFound(i64),

    #[error("collection not found: {0}")]
    CollectionNotFound(String),

    #[error("storage error: {0}")]
    Storage(#[from] nodedb_storage::StorageError),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("invalid query: {0}")]
    InvalidQuery(String),

    #[error("invalid schema name: {0}")]
    InvalidSchema(String),

    #[error("schema not empty: {0}")]
    SchemaNotEmpty(String),

    #[error("remote database: {0}")]
    RemoteDatabase(String),

    #[error("trigger aborted: {0}")]
    TriggerAbort(String),

    #[error("singleton delete not permitted: {0}")]
    SingletonDeleteNotPermitted(String),

    #[error("singleton clear not permitted: {0}")]
    SingletonClearNotPermitted(String),

    #[error("reserved schema write not permitted: {0}")]
    ReservedSchemaWriteNotPermitted(String),

    #[error("preference not found: {0}")]
    PreferenceNotFound(String),

    #[error("preference error: {0}")]
    PreferenceError(String),

    #[error("access history error: {0}")]
    AccessHistoryError(String),

    #[error("trim not permitted (never-trim): {0}")]
    TrimNeverTrim(String),

    #[error("invalid trim policy: {0}")]
    TrimPolicyInvalid(String),

    #[error("trim aborted: {0}")]
    TrimAborted(String),

    #[error("invalid cache config: {0}")]
    CacheConfigInvalid(String),
}

impl From<rmp_serde::encode::Error> for NoSqlError {
    fn from(e: rmp_serde::encode::Error) -> Self {
        NoSqlError::Serialization(e.to_string())
    }
}

impl From<rmp_serde::decode::Error> for NoSqlError {
    fn from(e: rmp_serde::decode::Error) -> Self {
        NoSqlError::Serialization(e.to_string())
    }
}

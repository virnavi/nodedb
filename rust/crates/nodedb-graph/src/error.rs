use thiserror::Error;

#[derive(Error, Debug)]
pub enum GraphError {
    #[error("node not found: id={0}")]
    NodeNotFound(i64),

    #[error("edge not found: id={0}")]
    EdgeNotFound(i64),

    #[error("cannot delete node: edges still connected")]
    DeleteRestricted,

    #[error("invalid source node: id={0}")]
    InvalidSource(i64),

    #[error("invalid target node: id={0}")]
    InvalidTarget(i64),

    #[error("storage error: {0}")]
    Storage(#[from] nodedb_storage::StorageError),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("traversal error: {0}")]
    Traversal(String),

    #[error("algorithm error: {0}")]
    Algorithm(String),
}

impl From<rmp_serde::encode::Error> for GraphError {
    fn from(e: rmp_serde::encode::Error) -> Self {
        GraphError::Serialization(e.to_string())
    }
}

impl From<rmp_serde::decode::Error> for GraphError {
    fn from(e: rmp_serde::decode::Error) -> Self {
        GraphError::Serialization(e.to_string())
    }
}

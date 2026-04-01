use thiserror::Error;

#[derive(Error, Debug)]
pub enum FederationError {
    #[error("peer not found: id={0}")]
    PeerNotFound(i64),

    #[error("peer not found: name={0}")]
    PeerNotFoundByName(String),

    #[error("group not found: id={0}")]
    GroupNotFound(i64),

    #[error("group not found: name={0}")]
    GroupNotFoundByName(String),

    #[error("duplicate peer name: {0}")]
    DuplicatePeerName(String),

    #[error("duplicate group name: {0}")]
    DuplicateGroupName(String),

    #[error("invalid member peer: id={0}")]
    InvalidMemberPeer(i64),

    #[error("storage error: {0}")]
    Storage(#[from] nodedb_storage::StorageError),

    #[error("serialization error: {0}")]
    Serialization(String),
}

impl From<rmp_serde::encode::Error> for FederationError {
    fn from(e: rmp_serde::encode::Error) -> Self {
        FederationError::Serialization(e.to_string())
    }
}

impl From<rmp_serde::decode::Error> for FederationError {
    fn from(e: rmp_serde::decode::Error) -> Self {
        FederationError::Serialization(e.to_string())
    }
}

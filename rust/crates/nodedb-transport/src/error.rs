use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransportError {
    #[error("connection failed: {0}")]
    Connection(String),

    #[error("TLS error: {0}")]
    Tls(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("handshake failed: {0}")]
    Handshake(String),

    #[error("peer rejected: {0}")]
    PeerRejected(String),

    #[error("message send failed: {0}")]
    Send(String),

    #[error("message receive failed: {0}")]
    Receive(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("crypto error: {0}")]
    Crypto(#[from] nodedb_crypto::CryptoError),

    #[error("federation error: {0}")]
    Federation(#[from] nodedb_federation::FederationError),

    #[error("storage error: {0}")]
    Storage(#[from] nodedb_storage::StorageError),

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("discovery error: {0}")]
    Discovery(String),

    #[error("gossip error: {0}")]
    Gossip(String),

    #[error("audit error: {0}")]
    Audit(String),

    #[error("peer not connected: {0}")]
    PeerNotConnected(String),

    #[error("mesh error: {0}")]
    Mesh(String),

    #[error("pairing required: {0}")]
    PairingRequired(String),

    #[error("pairing error: {0}")]
    Pairing(String),

    #[error("pairing verification failed: {0}")]
    PairingVerificationFailed(String),

    #[error("shutdown")]
    Shutdown,
}

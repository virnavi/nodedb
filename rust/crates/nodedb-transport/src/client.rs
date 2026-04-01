use std::sync::Arc;

use futures_util::StreamExt;
use tracing::info;

use nodedb_crypto::NodeIdentity;

use crate::connection::PeerConnection;
use crate::connection_pool::ConnectionPool;
use crate::error::TransportError;

/// Connect to a remote peer as a client.
/// Performs TLS connect → WS upgrade → Hello/HelloAck handshake.
/// Adds the connection to the pool on success.
pub async fn connect_to_peer(
    endpoint: &str,
    tls_config: Arc<tokio_rustls::rustls::ClientConfig>,
    identity: Arc<NodeIdentity>,
    my_endpoint: &str,
    pool: Arc<ConnectionPool>,
    user_id: &str,
    device_name: &str,
) -> Result<String, TransportError> {
    // Build URL — ensure wss:// prefix
    let url = if endpoint.starts_with("wss://") || endpoint.starts_with("ws://") {
        endpoint.to_string()
    } else {
        format!("wss://{}", endpoint)
    };

    // Use tokio-tungstenite's connect with our custom TLS config
    let connector = tokio_tungstenite::Connector::Rustls(tls_config);
    let (mut ws_stream, _) = tokio_tungstenite::connect_async_tls_with_config(
        &url,
        None,
        false,
        Some(connector),
    )
    .await
    .map_err(|e| TransportError::WebSocket(e.to_string()))?;

    // Handshake (initiator side)
    let result = crate::handshake::handshake_initiator(
        &mut ws_stream,
        &identity,
        my_endpoint,
        user_id,
        device_name,
    )
    .await?;

    let peer_id = result.peer_public.peer_id.clone();
    let shared_key = result.shared_key;

    // Split and create connection
    let (sink, stream) = ws_stream.split();
    let conn = PeerConnection::new_client(
        peer_id.clone(),
        endpoint.to_string(),
        sink,
        Some(shared_key),
    );

    pool.add(conn, crate::connection::PeerReceiver::Client(stream));
    info!("Connected to peer {}", peer_id);

    Ok(peer_id)
}

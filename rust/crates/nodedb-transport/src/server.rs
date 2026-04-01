use std::sync::Arc;

use futures_util::StreamExt;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio_rustls::TlsAcceptor;
use tracing::{info, warn};

use nodedb_crypto::NodeIdentity;

use crate::connection::PeerConnection;
use crate::connection_pool::ConnectionPool;
use crate::credential::CredentialStore;
use crate::error::TransportError;
use crate::pairing::PairingStore;

/// Start the WebSocket server.
/// Listens for incoming TCP connections, performs TLS + WS upgrade + handshake,
/// and adds accepted peers to the connection pool.
/// Returns when shutdown is signalled.
pub async fn start_server(
    listen_addr: &str,
    tls_acceptor: TlsAcceptor,
    identity: Arc<NodeIdentity>,
    pool: Arc<ConnectionPool>,
    credential_store: Arc<CredentialStore>,
    pairing_store: Option<Arc<PairingStore>>,
    user_id: String,
    device_name: String,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), TransportError> {
    let listener = TcpListener::bind(listen_addr)
        .await
        .map_err(|e| TransportError::Connection(e.to_string()))?;

    info!("Transport server listening on {}", listen_addr);

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                let (tcp_stream, addr) = match accept_result {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("TCP accept error: {}", e);
                        continue;
                    }
                };

                let tls_acceptor = tls_acceptor.clone();
                let identity = identity.clone();
                let pool = pool.clone();
                let credential_store = credential_store.clone();
                let pairing_store = pairing_store.clone();
                let user_id = user_id.clone();
                let device_name = device_name.clone();
                let endpoint = format!("wss://{}", addr);

                tokio::spawn(async move {
                    if let Err(e) = handle_inbound(
                        tcp_stream, tls_acceptor, identity, pool, credential_store,
                        pairing_store, &user_id, &device_name, &endpoint,
                    ).await {
                        warn!("Inbound connection from {} failed: {}", addr, e);
                    }
                });
            }
            _ = shutdown.changed() => {
                info!("Transport server shutting down");
                break;
            }
        }
    }

    Ok(())
}

/// Handle a single inbound connection: TLS → WS → Handshake → Pool.
async fn handle_inbound(
    tcp_stream: tokio::net::TcpStream,
    tls_acceptor: TlsAcceptor,
    identity: Arc<NodeIdentity>,
    pool: Arc<ConnectionPool>,
    credential_store: Arc<CredentialStore>,
    pairing_store: Option<Arc<PairingStore>>,
    user_id: &str,
    device_name: &str,
    endpoint: &str,
) -> Result<(), TransportError> {
    // TLS accept
    let tls_stream = tls_acceptor
        .accept(tcp_stream)
        .await
        .map_err(|e| TransportError::Tls(e.to_string()))?;

    // WebSocket upgrade
    let mut ws_stream = tokio_tungstenite::accept_async(tls_stream)
        .await
        .map_err(|e| TransportError::WebSocket(e.to_string()))?;

    // Handshake (acceptor side)
    let result = crate::handshake::handshake_acceptor(
        &mut ws_stream,
        &identity,
        endpoint,
        &credential_store,
        user_id,
        device_name,
        pairing_store.as_deref(),
    )
    .await?;

    let peer_id = result.peer_public.peer_id.clone();
    let peer_endpoint = result.peer_endpoint;
    let shared_key = result.shared_key;
    let final_endpoint = if peer_endpoint.is_empty() {
        endpoint.to_string()
    } else {
        peer_endpoint
    };

    // Split and create connection
    let (sink, stream) = ws_stream.split();
    let conn = PeerConnection::new_server(
        peer_id.clone(),
        final_endpoint,
        sink,
        Some(shared_key),
    );

    pool.add(conn, crate::connection::PeerReceiver::Server(stream));
    info!("Accepted peer {}", peer_id);

    Ok(())
}

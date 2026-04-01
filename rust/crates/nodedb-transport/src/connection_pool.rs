use std::sync::Arc;

use dashmap::DashMap;
use tracing::{info, warn};

use crate::connection::{recv_message, PeerConnection, PeerReceiver};
use crate::error::TransportError;
use crate::types::WireMessage;

/// Manages active peer connections.
pub struct ConnectionPool {
    connections: DashMap<String, Arc<PeerConnection>>,
    /// Channel to receive incoming messages from all connected peers.
    incoming_tx: tokio::sync::mpsc::Sender<(String, WireMessage)>,
    incoming_rx: tokio::sync::Mutex<tokio::sync::mpsc::Receiver<(String, WireMessage)>>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(1024);
        ConnectionPool {
            connections: DashMap::new(),
            incoming_tx: tx,
            incoming_rx: tokio::sync::Mutex::new(rx),
        }
    }

    /// Add a peer connection and spawn a read loop for its receiver.
    pub fn add(&self, conn: PeerConnection, receiver: PeerReceiver) {
        let peer_id = conn.peer_id.clone();
        let arc_conn = Arc::new(conn);
        self.connections.insert(peer_id.clone(), arc_conn);

        let tx = self.incoming_tx.clone();
        let pid = peer_id.clone();
        tokio::spawn(async move {
            Self::read_loop(pid, receiver, tx).await;
        });

        info!("Peer {} added to connection pool", peer_id);
    }

    /// Background read loop for a single peer.
    async fn read_loop(
        peer_id: String,
        mut receiver: PeerReceiver,
        tx: tokio::sync::mpsc::Sender<(String, WireMessage)>,
    ) {
        loop {
            match recv_message(&mut receiver).await {
                Ok(Some(msg)) => {
                    if tx.send((peer_id.clone(), msg)).await.is_err() {
                        break; // pool dropped
                    }
                }
                Ok(None) => {
                    info!("Peer {} disconnected", peer_id);
                    break;
                }
                Err(e) => {
                    warn!("Read error from peer {}: {}", peer_id, e);
                    break;
                }
            }
        }
    }

    /// Get a connection by peer_id.
    pub fn get(&self, peer_id: &str) -> Option<Arc<PeerConnection>> {
        self.connections.get(peer_id).map(|r| r.value().clone())
    }

    /// Remove a peer connection.
    pub fn remove(&self, peer_id: &str) {
        self.connections.remove(peer_id);
        info!("Peer {} removed from connection pool", peer_id);
    }

    /// Send a message to a specific peer.
    pub async fn send(&self, peer_id: &str, msg: &WireMessage) -> Result<(), TransportError> {
        let conn = self
            .connections
            .get(peer_id)
            .ok_or_else(|| TransportError::PeerNotConnected(peer_id.to_string()))?;
        conn.value().send_message(msg).await
    }

    /// Broadcast a message to all connected peers.
    pub async fn broadcast(&self, msg: &WireMessage) {
        for entry in self.connections.iter() {
            if let Err(e) = entry.value().send_message(msg).await {
                warn!("Broadcast to {} failed: {}", entry.key(), e);
            }
        }
    }

    /// Get list of connected peer IDs.
    pub fn connected_peer_ids(&self) -> Vec<String> {
        self.connections.iter().map(|e| e.key().clone()).collect()
    }

    /// Number of connected peers.
    pub fn peer_count(&self) -> usize {
        self.connections.len()
    }

    /// Receive the next incoming message from any peer.
    pub async fn recv(&self) -> Option<(String, WireMessage)> {
        self.incoming_rx.lock().await.recv().await
    }
}

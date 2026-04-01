use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::{mpsc, oneshot};
use tracing::warn;

use crate::connection_pool::ConnectionPool;
use crate::error::TransportError;
use crate::types::{FederatedQueryPolicy, WireMessage, WireMessageType};

/// Routes federated queries to peers based on policy.
pub struct FederatedRouter {
    policy: FederatedQueryPolicy,
    pool: Arc<ConnectionPool>,
    /// Pending single-response queries: msg_id → response channel.
    pending: Arc<DashMap<String, oneshot::Sender<Vec<u8>>>>,
    /// Pending multi-response queries: msg_id → (response sender, expected count).
    pending_multi: Arc<DashMap<String, mpsc::Sender<Vec<u8>>>>,
    my_peer_id: String,
}

impl FederatedRouter {
    pub fn new(
        policy: FederatedQueryPolicy,
        pool: Arc<ConnectionPool>,
        my_peer_id: &str,
    ) -> Self {
        FederatedRouter {
            policy,
            pool,
            pending: Arc::new(DashMap::new()),
            pending_multi: Arc::new(DashMap::new()),
            my_peer_id: my_peer_id.to_string(),
        }
    }

    /// Get the current query policy.
    pub fn policy(&self) -> FederatedQueryPolicy {
        self.policy
    }

    /// Send a query to peers and wait for the first response.
    /// Returns None if policy is LocalOnly or no peers respond.
    pub async fn query(
        &self,
        query_payload: Vec<u8>,
        timeout_secs: u64,
    ) -> Result<Option<Vec<u8>>, TransportError> {
        if self.policy == FederatedQueryPolicy::LocalOnly {
            return Ok(None);
        }

        let peers = self.pool.connected_peer_ids();
        if peers.is_empty() {
            return Ok(None);
        }

        let msg_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        self.pending.insert(msg_id.clone(), tx);

        let msg = WireMessage {
            version: 1,
            msg_id: msg_id.clone(),
            msg_type: WireMessageType::QueryRequest,
            sender_id: self.my_peer_id.clone(),
            payload: query_payload,
        };

        // Fan out to all connected peers
        for peer_id in &peers {
            if let Err(e) = self.pool.send(peer_id, &msg).await {
                warn!("Query fan-out to {} failed: {}", peer_id, e);
            }
        }

        // Wait for first response with timeout
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            rx,
        )
        .await;

        // Clean up pending entry
        self.pending.remove(&msg_id);

        match result {
            Ok(Ok(data)) => Ok(Some(data)),
            Ok(Err(_)) => Ok(None), // channel closed, no response
            Err(_) => Ok(None),     // timeout, no response
        }
    }

    /// Send a query to all peers and collect all responses (with timeout).
    /// Returns a Vec of response payloads from each responding peer.
    pub async fn query_all(
        &self,
        query_payload: Vec<u8>,
        timeout_secs: u64,
    ) -> Result<Vec<Vec<u8>>, TransportError> {
        if self.policy == FederatedQueryPolicy::LocalOnly {
            return Ok(vec![]);
        }

        let peers = self.pool.connected_peer_ids();
        if peers.is_empty() {
            return Ok(vec![]);
        }

        let peer_count = peers.len();
        let msg_id = uuid::Uuid::new_v4().to_string();
        let (tx, mut rx) = mpsc::channel(peer_count);

        self.pending_multi.insert(msg_id.clone(), tx);

        let msg = WireMessage {
            version: 1,
            msg_id: msg_id.clone(),
            msg_type: WireMessageType::QueryRequest,
            sender_id: self.my_peer_id.clone(),
            payload: query_payload,
        };

        for peer_id in &peers {
            if let Err(e) = self.pool.send(peer_id, &msg).await {
                warn!("Query fan-out to {} failed: {}", peer_id, e);
            }
        }

        // Collect responses until timeout
        let mut results = Vec::new();
        let deadline = tokio::time::Instant::now()
            + std::time::Duration::from_secs(timeout_secs);

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Some(data)) => {
                    results.push(data);
                    if results.len() >= peer_count {
                        break;
                    }
                }
                _ => break,
            }
        }

        self.pending_multi.remove(&msg_id);
        Ok(results)
    }

    /// Handle an incoming query response. Matches it to a pending request.
    pub fn handle_response(&self, msg: &WireMessage) {
        // Check single-response pending first
        if let Some((_, tx)) = self.pending.remove(&msg.msg_id) {
            let _ = tx.send(msg.payload.clone());
            return;
        }
        // Check multi-response pending
        if let Some(entry) = self.pending_multi.get(&msg.msg_id) {
            let _ = entry.value().try_send(msg.payload.clone());
        }
    }

    /// Forward a query to connected peers, excluding those in `exclude_peers`.
    /// Used by intermediate hops to propagate queries to unvisited peers.
    /// Skips policy check — forwarding always proceeds regardless of local policy.
    pub async fn forward_query(
        &self,
        envelope_bytes: Vec<u8>,
        exclude_peers: &[String],
        timeout_secs: u64,
    ) -> Result<Vec<Vec<u8>>, TransportError> {
        let peers: Vec<String> = self
            .pool
            .connected_peer_ids()
            .into_iter()
            .filter(|pid| !exclude_peers.contains(pid))
            .collect();

        if peers.is_empty() {
            return Ok(vec![]);
        }

        let peer_count = peers.len();
        let msg_id = uuid::Uuid::new_v4().to_string();
        let (tx, mut rx) = mpsc::channel(peer_count);

        self.pending_multi.insert(msg_id.clone(), tx);

        let msg = WireMessage {
            version: 1,
            msg_id: msg_id.clone(),
            msg_type: WireMessageType::QueryRequest,
            sender_id: self.my_peer_id.clone(),
            payload: envelope_bytes,
        };

        for peer_id in &peers {
            if let Err(e) = self.pool.send(peer_id, &msg).await {
                warn!("Forward query to {} failed: {}", peer_id, e);
            }
        }

        // Collect responses until timeout
        let mut results = Vec::new();
        let deadline =
            tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Some(data)) => {
                    results.push(data);
                    if results.len() >= peer_count {
                        break;
                    }
                }
                _ => break,
            }
        }

        self.pending_multi.remove(&msg_id);
        Ok(results)
    }

    /// Send a query to specific peers (by peer_id) and collect all responses.
    /// Used for targeted mesh queries where the MeshRouter has identified which peers host a database.
    pub async fn query_peers(
        &self,
        peer_ids: &[String],
        query_payload: Vec<u8>,
        timeout_secs: u64,
    ) -> Result<Vec<Vec<u8>>, TransportError> {
        if peer_ids.is_empty() {
            return Ok(vec![]);
        }

        let peer_count = peer_ids.len();
        let msg_id = uuid::Uuid::new_v4().to_string();
        let (tx, mut rx) = mpsc::channel(peer_count);

        self.pending_multi.insert(msg_id.clone(), tx);

        let msg = WireMessage {
            version: 1,
            msg_id: msg_id.clone(),
            msg_type: WireMessageType::QueryRequest,
            sender_id: self.my_peer_id.clone(),
            payload: query_payload,
        };

        for peer_id in peer_ids {
            if let Err(e) = self.pool.send(peer_id, &msg).await {
                warn!("Mesh query to {} failed: {}", peer_id, e);
            }
        }

        let mut results = Vec::new();
        let deadline =
            tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Some(data)) => {
                    results.push(data);
                    if results.len() >= peer_count {
                        break;
                    }
                }
                _ => break,
            }
        }

        self.pending_multi.remove(&msg_id);
        Ok(results)
    }

    /// Handle an incoming query request from a peer.
    /// Returns the response payload (if any) to send back.
    /// The actual query execution is delegated to the caller.
    pub fn build_response(
        &self,
        request: &WireMessage,
        response_payload: Vec<u8>,
    ) -> WireMessage {
        WireMessage {
            version: 1,
            msg_id: request.msg_id.clone(),
            msg_type: WireMessageType::QueryResponse,
            sender_id: self.my_peer_id.clone(),
            payload: response_payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_only_returns_none() {
        let pool = Arc::new(ConnectionPool::new());
        let router = FederatedRouter::new(
            FederatedQueryPolicy::LocalOnly,
            pool,
            "self",
        );
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(router.query(vec![1, 2, 3], 5)).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn no_peers_returns_none() {
        let pool = Arc::new(ConnectionPool::new());
        let router = FederatedRouter::new(
            FederatedQueryPolicy::QueryPeersAlways,
            pool,
            "self",
        );
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(router.query(vec![1, 2, 3], 5)).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn build_response_matches_msg_id() {
        let pool = Arc::new(ConnectionPool::new());
        let router = FederatedRouter::new(
            FederatedQueryPolicy::QueryPeersAlways,
            pool,
            "self",
        );
        let request = WireMessage {
            version: 1,
            msg_id: "req-123".to_string(),
            msg_type: WireMessageType::QueryRequest,
            sender_id: "peer-a".to_string(),
            payload: vec![],
        };
        let response = router.build_response(&request, vec![42]);
        assert_eq!(response.msg_id, "req-123");
        assert_eq!(response.msg_type, WireMessageType::QueryResponse);
        assert_eq!(response.payload, vec![42]);
    }

    #[test]
    fn handle_response_resolves_pending() {
        let pool = Arc::new(ConnectionPool::new());
        let router = FederatedRouter::new(
            FederatedQueryPolicy::QueryPeersAlways,
            pool,
            "self",
        );

        let (tx, mut rx) = oneshot::channel();
        router.pending.insert("test-id".to_string(), tx);

        let msg = WireMessage {
            version: 1,
            msg_id: "test-id".to_string(),
            msg_type: WireMessageType::QueryResponse,
            sender_id: "peer-b".to_string(),
            payload: vec![99],
        };
        router.handle_response(&msg);

        let result = rx.try_recv().unwrap();
        assert_eq!(result, vec![99]);
    }

    #[test]
    fn query_all_local_only_returns_empty() {
        let pool = Arc::new(ConnectionPool::new());
        let router = FederatedRouter::new(
            FederatedQueryPolicy::LocalOnly,
            pool,
            "self",
        );
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(router.query_all(vec![1, 2, 3], 5)).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn query_all_no_peers_returns_empty() {
        let pool = Arc::new(ConnectionPool::new());
        let router = FederatedRouter::new(
            FederatedQueryPolicy::QueryPeersAlways,
            pool,
            "self",
        );
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(router.query_all(vec![1, 2, 3], 5)).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn forward_query_no_peers_returns_empty() {
        let pool = Arc::new(ConnectionPool::new());
        let router = FederatedRouter::new(
            FederatedQueryPolicy::QueryPeersAlways,
            pool,
            "self",
        );
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt
            .block_on(router.forward_query(vec![1, 2, 3], &[], 5))
            .unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn forward_query_all_excluded_returns_empty() {
        let pool = Arc::new(ConnectionPool::new());
        let router = FederatedRouter::new(
            FederatedQueryPolicy::QueryPeersAlways,
            pool,
            "self",
        );
        let rt = tokio::runtime::Runtime::new().unwrap();
        // Even if there were peers, excluding them all should give empty
        let result = rt
            .block_on(router.forward_query(vec![1], &["peer-a".to_string()], 5))
            .unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn handle_response_resolves_multi_pending() {
        let pool = Arc::new(ConnectionPool::new());
        let router = FederatedRouter::new(
            FederatedQueryPolicy::QueryPeersAlways,
            pool,
            "self",
        );

        let (tx, mut rx) = mpsc::channel(4);
        router.pending_multi.insert("multi-id".to_string(), tx);

        // Send two responses
        for payload in [vec![10], vec![20]] {
            let msg = WireMessage {
                version: 1,
                msg_id: "multi-id".to_string(),
                msg_type: WireMessageType::QueryResponse,
                sender_id: "peer-c".to_string(),
                payload,
            };
            router.handle_response(&msg);
        }

        let r1 = rx.try_recv().unwrap();
        let r2 = rx.try_recv().unwrap();
        assert_eq!(r1, vec![10]);
        assert_eq!(r2, vec![20]);
    }
}

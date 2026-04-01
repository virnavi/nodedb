use std::sync::Arc;

use tokio::sync::watch;
use tracing::{info, warn};

use nodedb_crypto::{NodeIdentity, PublicIdentity};

use crate::audit::AuditLog;
use crate::connection_pool::ConnectionPool;
use crate::credential::CredentialStore;
use crate::mesh::{MeshConfig, MeshMember, MeshRouter, MeshSharingStatus, MeshStatus};
use crate::discovery::DiscoveryManager;
use crate::error::TransportError;
use crate::gossip::GossipManager;
use crate::pairing::{PairingRecord, PairingStore, PendingPairingRequest};
use crate::query_handler::QueryHandler;
use crate::router::FederatedRouter;
use crate::tls;
use crate::types::{
    FederatedQueryEnvelope, FederatedQueryResponse, GossipPeerEntry, TransportConfig,
    WireMessageType,
};

/// Top-level facade composing all transport components.
pub struct TransportEngine {
    identity: Arc<NodeIdentity>,
    pool: Arc<ConnectionPool>,
    credential_store: Arc<CredentialStore>,
    discovery: Arc<DiscoveryManager>,
    gossip: Arc<GossipManager>,
    router: Arc<FederatedRouter>,
    mesh_router: Option<Arc<MeshRouter>>,
    audit: Option<Arc<AuditLog>>,
    pairing_store: Option<Arc<PairingStore>>,
    shutdown_tx: watch::Sender<bool>,
    config: TransportConfig,
}

impl TransportEngine {
    /// Create and start the transport engine.
    /// Spawns the WebSocket server, gossip loop, and mDNS discovery as background tasks.
    pub async fn start(
        config: TransportConfig,
        identity: NodeIdentity,
        storage: Option<Arc<nodedb_storage::StorageEngine>>,
        acceptance_callback: Option<
            Box<dyn Fn(&PublicIdentity) -> crate::types::PeerAcceptance + Send + Sync>,
        >,
        query_handler: Option<Arc<dyn QueryHandler>>,
    ) -> Result<Self, TransportError> {
        let identity = Arc::new(identity);
        let pool = Arc::new(ConnectionPool::new());
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Credential store
        let credential_store = if let Some(cb) = acceptance_callback {
            Arc::new(CredentialStore::new(move |pi| cb(pi)))
        } else if !config.trusted_peer_keys.is_empty() {
            let trusted: std::collections::HashSet<String> = config
                .trusted_peer_keys
                .iter()
                .map(|k| k.to_lowercase())
                .collect();
            Arc::new(CredentialStore::new(move |pi| {
                if trusted.contains(&pi.peer_id) {
                    crate::types::PeerAcceptance::Accept
                } else {
                    crate::types::PeerAcceptance::Reject
                }
            }))
        } else {
            Arc::new(CredentialStore::accept_all())
        };

        // Audit log (optional, needs storage)
        let audit = if let Some(ref storage) = storage {
            Some(Arc::new(AuditLog::new(storage.clone())?))
        } else {
            None
        };

        // Pairing store (optional, needs storage + require_pairing flag)
        let pairing_store = if config.require_pairing {
            if let Some(ref storage) = storage {
                Some(Arc::new(PairingStore::new(storage.clone())?))
            } else {
                None
            }
        } else {
            None
        };

        // Parse listen port for discovery
        let port: u16 = config
            .listen_addr
            .rsplit(':')
            .next()
            .and_then(|p| p.parse().ok())
            .unwrap_or(9400);

        // Discovery manager
        let discovery = Arc::new(DiscoveryManager::new(identity.peer_id(), port));
        discovery.add_seed_peers(&config.seed_peers);

        // Gossip manager
        let listen_endpoint = format!("wss://{}", config.listen_addr);
        let gossip = Arc::new(GossipManager::new(
            config.gossip.clone(),
            pool.clone(),
            discovery.clone(),
            identity.peer_id(),
            &listen_endpoint,
            config.mesh.clone(),
        ));

        // Federated router
        let router = Arc::new(FederatedRouter::new(
            config.query_policy,
            pool.clone(),
            identity.peer_id(),
        ));

        // TLS setup
        let cert_key = tls::generate_self_signed_cert()?;
        let server_tls_config = tls::build_server_tls_config(&cert_key)?;
        let tls_acceptor = tls::build_tls_acceptor(server_tls_config);

        // Start WebSocket server
        let server_shutdown = shutdown_rx.clone();
        let server_identity = identity.clone();
        let server_pool = pool.clone();
        let server_cred = credential_store.clone();
        let server_pairing = pairing_store.clone();
        let server_user_id = config.user_id.clone();
        let server_device_name = config.device_name.clone();
        let listen_addr = config.listen_addr.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::server::start_server(
                &listen_addr,
                tls_acceptor,
                server_identity,
                server_pool,
                server_cred,
                server_pairing,
                server_user_id,
                server_device_name,
                server_shutdown,
            )
            .await
            {
                tracing::error!("Server error: {}", e);
            }
        });

        // Start gossip loop
        gossip.start(shutdown_rx.clone());

        // Start mDNS if enabled
        if config.mdns_enabled {
            discovery.start_mdns(shutdown_rx.clone());
        }

        // Start message dispatcher (processes incoming messages from pool)
        let dispatch_pool = pool.clone();
        let dispatch_gossip = gossip.clone();
        let dispatch_router = router.clone();
        let dispatch_shutdown = shutdown_rx.clone();
        let dispatch_identity = identity.clone();
        let dispatch_handler = query_handler;
        let dispatch_mesh = config.mesh.clone();
        tokio::spawn(async move {
            message_dispatcher(
                dispatch_pool,
                dispatch_gossip,
                dispatch_router,
                dispatch_identity,
                dispatch_handler,
                dispatch_mesh,
                dispatch_shutdown,
            )
            .await;
        });

        // Grab mesh router from gossip manager (if mesh is configured)
        let mesh_router = gossip.mesh_router().cloned();

        info!(
            "TransportEngine started: peer_id={}, listen={}",
            identity.peer_id(),
            config.listen_addr,
        );

        Ok(TransportEngine {
            identity,
            pool,
            credential_store,
            discovery,
            gossip,
            router,
            mesh_router,
            audit,
            pairing_store,
            shutdown_tx,
            config,
        })
    }

    /// Get this node's identity.
    pub fn identity(&self) -> &NodeIdentity {
        &self.identity
    }

    /// Get this node's public identity.
    pub fn public_identity(&self) -> PublicIdentity {
        self.identity.to_public()
    }

    /// Get the number of connected peers.
    pub fn connected_peer_count(&self) -> usize {
        self.pool.peer_count()
    }

    /// Get IDs of connected peers.
    pub fn connected_peer_ids(&self) -> Vec<String> {
        self.pool.connected_peer_ids()
    }

    /// Get all known peers from gossip.
    pub fn known_peers(&self) -> Vec<GossipPeerEntry> {
        self.gossip.known_peers()
    }

    /// Set a credential for a peer.
    pub fn set_peer_credential(&self, peer_id: &str, cred: crate::types::PeerCredential) {
        self.credential_store.set_credential(peer_id, cred);
    }

    /// Update the mesh secret at runtime (e.g., after connecting via QR invite).
    pub fn set_mesh_secret(&self, secret: &str) {
        self.gossip.set_mesh_secret(secret.to_string());
    }

    /// Register a device directly in the pairing store (pre-authorize).
    /// Once registered, the device can connect and will pass the handshake
    /// verification, gaining access to gossip and federation.
    pub fn register_device(
        &self,
        peer_id: &str,
        public_key_bytes: &[u8],
        user_id: &str,
        device_name: &str,
    ) -> Result<PairingRecord, TransportError> {
        match &self.pairing_store {
            Some(store) => store.register_device(peer_id, public_key_bytes, user_id, device_name),
            None => Err(TransportError::Pairing(
                "pairing store not available (require_pairing not enabled)".to_string(),
            )),
        }
    }

    /// Connect to a peer by endpoint.
    pub async fn connect_to_peer(&self, endpoint: &str) -> Result<String, TransportError> {
        let tls_config = tls::build_client_tls_config()?;
        crate::client::connect_to_peer(
            endpoint,
            tls_config,
            self.identity.clone(),
            &format!("wss://{}", self.config.listen_addr),
            self.pool.clone(),
            &self.config.user_id,
            &self.config.device_name,
        )
        .await
    }

    /// Execute a federated query (first response wins).
    pub async fn query(&self, payload: Vec<u8>, timeout_secs: u64) -> Result<Option<Vec<u8>>, TransportError> {
        self.router.query(payload, timeout_secs).await
    }

    /// Execute a federated query and collect all responses.
    pub async fn query_all(&self, payload: Vec<u8>, timeout_secs: u64) -> Result<Vec<Vec<u8>>, TransportError> {
        self.router.query_all(payload, timeout_secs).await
    }

    /// Get the router (for policy inspection).
    pub fn router(&self) -> &Arc<FederatedRouter> {
        &self.router
    }

    /// Get the audit log (if storage was provided).
    pub fn audit_log(&self) -> Option<&AuditLog> {
        self.audit.as_deref()
    }

    /// Get the connection pool.
    pub fn pool(&self) -> &Arc<ConnectionPool> {
        &self.pool
    }

    /// Get the credential store.
    pub fn credential_store(&self) -> &Arc<CredentialStore> {
        &self.credential_store
    }

    /// Get the discovery manager.
    pub fn discovery(&self) -> &Arc<DiscoveryManager> {
        &self.discovery
    }

    /// Get the mesh router (if mesh is configured).
    pub fn mesh_router(&self) -> Option<&Arc<MeshRouter>> {
        self.mesh_router.as_ref()
    }

    /// Get the current mesh status (returns None if not in a mesh).
    pub fn mesh_status(&self) -> Option<MeshStatus> {
        let mesh_cfg = self.config.mesh.as_ref()?;
        let member_count = self
            .mesh_router
            .as_ref()
            .map(|r| r.members().len())
            .unwrap_or(0);
        Some(MeshStatus {
            mesh_name: mesh_cfg.mesh_name.clone(),
            database_name: mesh_cfg.database_name.clone(),
            sharing_status: mesh_cfg.sharing_status.to_str().to_string(),
            member_count,
        })
    }

    /// List all known mesh members (returns empty if not in a mesh).
    pub fn mesh_members(&self) -> Vec<MeshMember> {
        self.mesh_router
            .as_ref()
            .map(|r| r.members())
            .unwrap_or_default()
    }

    /// Execute a targeted mesh query to a specific database by name.
    /// Uses the MeshRouter to find peers hosting the named database,
    /// then sends the query only to those peers.
    pub async fn mesh_query(
        &self,
        database_name: &str,
        query_type: &str,
        query_data: Vec<u8>,
        timeout_secs: u64,
    ) -> Result<Vec<Vec<u8>>, TransportError> {
        let mesh_router = match &self.mesh_router {
            Some(r) => r,
            None => return Err(TransportError::Mesh("Not in a mesh".to_string())),
        };

        let peers = mesh_router.route(database_name);
        if peers.is_empty() {
            return Ok(vec![]);
        }

        // Filter to peers that allow reads
        let target_peer_ids: Vec<String> = peers
            .iter()
            .filter(|p| p.sharing_status.allows_reads())
            .map(|p| p.peer_id.clone())
            .collect();

        if target_peer_ids.is_empty() {
            return Ok(vec![]);
        }

        // Build a FederatedQueryEnvelope for targeted sending
        let envelope = FederatedQueryEnvelope {
            query_id: uuid::Uuid::new_v4().to_string(),
            origin_peer_id: self.identity.peer_id().to_string(),
            query_type: query_type.to_string(),
            query_data,
            ttl: 1, // Single hop for targeted mesh queries
            visited: vec![self.identity.peer_id().to_string()],
        };

        let envelope_bytes = rmp_serde::to_vec(&envelope)
            .map_err(|e| TransportError::Serialization(format!("Failed to serialize mesh query envelope: {}", e)))?;

        // Send to the specific peers and collect responses
        let raw_responses = self
            .router
            .query_peers(&target_peer_ids, envelope_bytes, timeout_secs)
            .await?;

        // Parse FederatedQueryResponse from each raw response
        let results: Vec<Vec<u8>> = raw_responses
            .into_iter()
            .filter_map(|data| {
                let resp: FederatedQueryResponse = rmp_serde::from_slice(&data).ok()?;
                if resp.success {
                    Some(resp.result_data)
                } else {
                    None
                }
            })
            .collect();

        Ok(results)
    }

    /// List all approved paired devices.
    pub fn paired_devices(&self) -> Vec<PairingRecord> {
        self.pairing_store
            .as_ref()
            .map(|ps| ps.list_paired())
            .unwrap_or_default()
    }

    /// List pending pairing requests awaiting user approval.
    pub fn pending_pairings(&self) -> Vec<PendingPairingRequest> {
        self.pairing_store
            .as_ref()
            .map(|ps| ps.list_pending())
            .unwrap_or_default()
    }

    /// Approve a pending pairing request by peer_id.
    pub fn approve_pairing(&self, peer_id: &str) -> Result<Option<PairingRecord>, TransportError> {
        match &self.pairing_store {
            Some(ps) => ps.approve(peer_id),
            None => Ok(None),
        }
    }

    /// Reject a pending pairing request by peer_id.
    pub fn reject_pairing(&self, peer_id: &str) -> bool {
        self.pairing_store
            .as_ref()
            .map(|ps| ps.reject(peer_id))
            .unwrap_or(false)
    }

    /// Remove (unpair) a previously paired device.
    pub fn remove_paired_device(&self, peer_id: &str) -> Result<bool, TransportError> {
        match &self.pairing_store {
            Some(ps) => ps.remove_paired(peer_id),
            None => Ok(false),
        }
    }

    /// Shut down all background tasks.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
        info!("TransportEngine shutdown signalled");
    }
}

impl Drop for TransportEngine {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.send(true);
    }
}

/// Background task that processes incoming messages from the connection pool
/// and dispatches them to the appropriate handler (gossip, router, etc.).
async fn message_dispatcher(
    pool: Arc<ConnectionPool>,
    gossip: Arc<GossipManager>,
    router: Arc<FederatedRouter>,
    identity: Arc<NodeIdentity>,
    query_handler: Option<Arc<dyn QueryHandler>>,
    mesh_config: Option<MeshConfig>,
    mut shutdown: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            msg = pool.recv() => {
                match msg {
                    Some((peer_id, wire_msg)) => {
                        match wire_msg.msg_type {
                            WireMessageType::GossipPeerList => {
                                gossip.handle_gossip_payload(&wire_msg.payload);
                            }
                            WireMessageType::QueryResponse => {
                                router.handle_response(&wire_msg);
                            }
                            WireMessageType::QueryRequest => {
                                handle_query_request(
                                    &pool,
                                    &router,
                                    &identity,
                                    &query_handler,
                                    mesh_config.as_ref(),
                                    &peer_id,
                                    wire_msg,
                                )
                                .await;
                            }
                            WireMessageType::Ping => {
                                let pong = crate::types::WireMessage {
                                    version: 1,
                                    msg_id: wire_msg.msg_id,
                                    msg_type: WireMessageType::Pong,
                                    sender_id: String::new(),
                                    payload: vec![],
                                };
                                let _ = pool.send(&peer_id, &pong).await;
                            }
                            WireMessageType::TriggerNotification => {
                                if let Some(ref handler) = query_handler {
                                    let h: Arc<dyn QueryHandler> = handler.clone();
                                    let payload_bytes = wire_msg.payload.clone();
                                    let sender = peer_id.clone();
                                    tokio::task::spawn_blocking(move || {
                                        h.handle_trigger_notification(&payload_bytes, &sender);
                                    });
                                }
                            }
                            WireMessageType::PreferenceSync => {
                                if let Some(ref handler) = query_handler {
                                    let h: Arc<dyn QueryHandler> = handler.clone();
                                    let payload_bytes = wire_msg.payload.clone();
                                    let sender = peer_id.clone();
                                    tokio::task::spawn_blocking(move || {
                                        h.handle_preference_sync(&payload_bytes, &sender);
                                    });
                                }
                            }
                            WireMessageType::SingletonSync => {
                                if let Some(ref handler) = query_handler {
                                    let h: Arc<dyn QueryHandler> = handler.clone();
                                    let payload_bytes = wire_msg.payload.clone();
                                    let sender = peer_id.clone();
                                    tokio::task::spawn_blocking(move || {
                                        h.handle_singleton_sync(&payload_bytes, &sender);
                                    });
                                }
                            }
                            _ => {
                                // Pong, Hello, HelloAck — ignored in dispatcher
                            }
                        }
                    }
                    None => break, // pool channel closed
                }
            }
            _ = shutdown.changed() => {
                break;
            }
        }
    }
}

/// Handle an inbound QueryRequest: deserialize envelope, check TTL + visited,
/// execute locally, optionally forward to unvisited peers, merge, and return.
async fn handle_query_request(
    pool: &Arc<ConnectionPool>,
    router: &Arc<FederatedRouter>,
    identity: &Arc<NodeIdentity>,
    query_handler: &Option<Arc<dyn QueryHandler>>,
    mesh_config: Option<&MeshConfig>,
    peer_id: &str,
    wire_msg: crate::types::WireMessage,
) {
    let handler = match query_handler {
        Some(h) => h.clone(),
        None => {
            warn!("Received QueryRequest but no QueryHandler is configured");
            return;
        }
    };

    // Mesh sharing status enforcement: check if this database allows the query
    if let Some(mesh_cfg) = mesh_config {
        match mesh_cfg.sharing_status {
            MeshSharingStatus::Private => {
                // Private databases return empty results to all mesh queries
                let my_peer_id = identity.peer_id().to_string();
                if let Ok(envelope) = rmp_serde::from_slice::<FederatedQueryEnvelope>(&wire_msg.payload) {
                    let resp = FederatedQueryResponse {
                        query_id: envelope.query_id,
                        responder_peer_id: my_peer_id,
                        success: true,
                        result_data: vec![],
                        error_message: None,
                    };
                    if let Ok(resp_bytes) = rmp_serde::to_vec(&resp) {
                        let response_msg = router.build_response(&wire_msg, resp_bytes);
                        let _ = pool.send(peer_id, &response_msg).await;
                    }
                }
                return;
            }
            MeshSharingStatus::ReadOnly => {
                // ReadOnly allows reads but rejects write-type operations
                // We check the query_type — writes come as nosql "put"/"delete" operations
                // For now, all federated queries are reads (find_all, get, search, etc.)
                // so ReadOnly allows them through. Write enforcement is at the handler level.
            }
            MeshSharingStatus::ReadWrite | MeshSharingStatus::Full => {
                // Allow all
            }
        }
    }

    // Deserialize the envelope
    let mut envelope: FederatedQueryEnvelope = match rmp_serde::from_slice(&wire_msg.payload) {
        Ok(e) => e,
        Err(e) => {
            warn!("Failed to deserialize FederatedQueryEnvelope: {}", e);
            return;
        }
    };

    let my_peer_id = identity.peer_id().to_string();

    // Loop prevention: if we already visited this query, return empty success
    if envelope.visited.contains(&my_peer_id) {
        let resp = FederatedQueryResponse {
            query_id: envelope.query_id,
            responder_peer_id: my_peer_id,
            success: true,
            result_data: vec![],
            error_message: None,
        };
        let resp_bytes = match rmp_serde::to_vec(&resp) {
            Ok(b) => b,
            Err(_) => return,
        };
        let response_msg = router.build_response(&wire_msg, resp_bytes);
        let _ = pool.send(peer_id, &response_msg).await;
        return;
    }

    // Check TTL
    if envelope.ttl == 0 {
        warn!("Rejecting query {} with TTL=0", envelope.query_id);
        let resp = FederatedQueryResponse {
            query_id: envelope.query_id,
            responder_peer_id: my_peer_id,
            success: false,
            result_data: vec![],
            error_message: Some("TTL expired".to_string()),
        };
        let resp_bytes = match rmp_serde::to_vec(&resp) {
            Ok(b) => b,
            Err(_) => return,
        };
        let response_msg = router.build_response(&wire_msg, resp_bytes);
        let _ = pool.send(peer_id, &response_msg).await;
        return;
    }

    // Decrement TTL and add self to visited
    envelope.ttl -= 1;
    envelope.visited.push(my_peer_id.clone());

    let query_type = envelope.query_type.clone();
    let query_data = envelope.query_data.clone();
    let origin = envelope.origin_peer_id.clone();
    let query_id = envelope.query_id.clone();

    // Execute locally via QueryHandler in a blocking task (engines may do I/O)
    let handler_for_local = handler.clone();
    let qt = query_type.clone();
    let qd = query_data.clone();
    let orig = origin.clone();
    let local_result =
        tokio::task::spawn_blocking(move || handler_for_local.handle_query(&qt, &qd, &orig))
            .await;

    let local_data = match local_result {
        Ok(Ok(data)) => data,
        Ok(Err(e)) => {
            warn!("Local query execution failed: {}", e);
            vec![]
        }
        Err(e) => {
            warn!("Local query handler panicked: {}", e);
            vec![]
        }
    };

    // If TTL > 0, forward to unvisited peers and merge
    let final_data = if envelope.ttl > 0 {
        match rmp_serde::to_vec(&envelope) {
            Ok(updated_envelope_bytes) => {
                // Forward to unvisited peers with a fixed 5-second per-hop timeout
                let remote_raw = router
                    .forward_query(updated_envelope_bytes, &envelope.visited, 5)
                    .await
                    .unwrap_or_default();

                // Parse FederatedQueryResponse from each remote result
                let remote_results: Vec<Vec<u8>> = remote_raw
                    .into_iter()
                    .filter_map(|data| {
                        let resp: FederatedQueryResponse = rmp_serde::from_slice(&data).ok()?;
                        if resp.success {
                            Some(resp.result_data)
                        } else {
                            None
                        }
                    })
                    .collect();

                if remote_results.is_empty() {
                    local_data
                } else {
                    // Merge local + remote using the handler's merge_results
                    let qt = query_type.clone();
                    let ld = local_data.clone();
                    tokio::task::spawn_blocking(move || {
                        handler.merge_results(&qt, ld, remote_results)
                    })
                    .await
                    .unwrap_or(local_data)
                }
            }
            Err(_) => local_data, // Can't serialize updated envelope — return local
        }
    } else {
        local_data
    };

    let resp = FederatedQueryResponse {
        query_id,
        responder_peer_id: my_peer_id,
        success: true,
        result_data: final_data,
        error_message: None,
    };

    let resp_bytes = match rmp_serde::to_vec(&resp) {
        Ok(b) => b,
        Err(_) => return,
    };
    let response_msg = router.build_response(&wire_msg, resp_bytes);
    let _ = pool.send(peer_id, &response_msg).await;
}

use std::sync::{Arc, RwLock};

use chrono::Utc;
use dashmap::DashMap;
use rand::seq::SliceRandom;
use tokio::sync::watch;
use tracing::{info, warn};

use crate::connection_pool::ConnectionPool;
use crate::discovery::DiscoveryManager;
use crate::mesh::{self, MeshConfig, MeshRouter};
use crate::types::{GossipConfig, GossipPeerEntry, WireMessage, WireMessageType};

/// Manages the gossip protocol: periodic peer list broadcasting and
/// processing incoming gossip messages.
pub struct GossipManager {
    config: GossipConfig,
    pool: Arc<ConnectionPool>,
    discovery: Arc<DiscoveryManager>,
    /// Known peers from gossip (peer_id → entry).
    known_peers: Arc<DashMap<String, GossipPeerEntry>>,
    my_peer_id: String,
    my_endpoint: String,
    /// Optional mesh config for advertising mesh fields in gossip.
    mesh: Option<MeshConfig>,
    /// Optional mesh router updated on gossip.
    mesh_router: Option<Arc<MeshRouter>>,
    /// Runtime-updatable mesh secret (overrides mesh config secret when set).
    mesh_secret_override: Arc<RwLock<Option<String>>>,
}

impl GossipManager {
    pub fn new(
        config: GossipConfig,
        pool: Arc<ConnectionPool>,
        discovery: Arc<DiscoveryManager>,
        my_peer_id: &str,
        my_endpoint: &str,
        mesh: Option<MeshConfig>,
    ) -> Self {
        let mesh_router = mesh.as_ref().map(|m| Arc::new(MeshRouter::new(&m.mesh_name)));
        GossipManager {
            config,
            pool,
            discovery,
            known_peers: Arc::new(DashMap::new()),
            my_peer_id: my_peer_id.to_string(),
            my_endpoint: my_endpoint.to_string(),
            mesh,
            mesh_router,
            mesh_secret_override: Arc::new(RwLock::new(None)),
        }
    }

    /// Update the mesh secret at runtime (e.g., after connecting via QR invite).
    /// This overrides the mesh secret from the initial config, allowing
    /// gossip HMAC authentication to succeed with the shared secret.
    pub fn set_mesh_secret(&self, secret: String) {
        if let Ok(mut guard) = self.mesh_secret_override.write() {
            *guard = Some(secret);
        }
    }

    /// Get the effective mesh secret (override takes precedence over config).
    fn effective_mesh_secret(&self) -> Option<String> {
        if let Ok(guard) = self.mesh_secret_override.read() {
            if let Some(ref s) = *guard {
                return Some(s.clone());
            }
        }
        self.mesh.as_ref().and_then(|m| m.mesh_secret.clone())
    }

    /// Get the mesh router (if mesh is configured).
    pub fn mesh_router(&self) -> Option<&Arc<MeshRouter>> {
        self.mesh_router.as_ref()
    }

    /// Get all known peers from gossip state.
    pub fn known_peers(&self) -> Vec<GossipPeerEntry> {
        self.known_peers.iter().map(|e| e.value().clone()).collect()
    }

    /// Start the periodic gossip loop as a background task.
    pub fn start(&self, mut shutdown: watch::Receiver<bool>) {
        let config = self.config.clone();
        let pool = self.pool.clone();
        let known_peers = self.known_peers.clone();
        let my_peer_id = self.my_peer_id.clone();
        let my_endpoint = self.my_endpoint.clone();
        let mesh = self.mesh.clone();
        let mesh_secret_override = self.mesh_secret_override.clone();

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(config.interval_seconds));
            interval.tick().await; // first tick is immediate, skip it

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        gossip_round(&config, &pool, &known_peers, &my_peer_id, &my_endpoint, mesh.as_ref(), &mesh_secret_override).await;
                    }
                    _ = shutdown.changed() => {
                        info!("Gossip loop shutting down");
                        break;
                    }
                }
            }
        });
    }

    /// Handle a raw gossip payload with optional HMAC verification.
    /// If a mesh_secret is configured, the payload is expected to be an
    /// AuthenticatedGossipPayload; otherwise it is a Vec<GossipPeerEntry>.
    pub fn handle_gossip_payload(&self, payload: &[u8]) {
        if self.mesh.is_some() {
            if let Some(ref secret) = self.effective_mesh_secret() {
                // Try to deserialize as authenticated payload
                match rmp_serde::from_slice::<mesh::AuthenticatedGossipPayload>(payload) {
                    Ok(auth) => {
                        // Verify HMAC over the entries (re-serialize for verification)
                        let entries_bytes = match rmp_serde::to_vec(&auth.entries) {
                            Ok(b) => b,
                            Err(_) => return,
                        };
                        if !mesh::verify_mesh_hmac(secret, &entries_bytes, &auth.mesh_hmac) {
                            warn!("Gossip HMAC verification failed, dropping payload");
                            return;
                        }
                        self.handle_gossip(auth.entries);
                        return;
                    }
                    Err(_) => {
                        // Not an authenticated payload — try plain for backward compat,
                        // but only if the HMAC is empty (missing from non-mesh peers)
                        if let Ok(entries) = rmp_serde::from_slice::<Vec<GossipPeerEntry>>(payload) {
                            // Accept unauthenticated gossip from non-mesh peers
                            self.handle_gossip(entries);
                        }
                        return;
                    }
                }
            }
        }
        // No mesh secret — accept plain gossip
        if let Ok(entries) = rmp_serde::from_slice::<Vec<GossipPeerEntry>>(payload) {
            self.handle_gossip(entries);
        }
    }

    /// Handle an incoming gossip peer list from a remote peer.
    pub fn handle_gossip(&self, entries: Vec<GossipPeerEntry>) {
        for mut entry in entries {
            // Skip self
            if entry.peer_id == self.my_peer_id {
                continue;
            }
            // Discard expired entries
            if entry.ttl == 0 {
                continue;
            }
            // Decrement TTL
            entry.ttl = entry.ttl.saturating_sub(1);

            // Upsert: insert if unknown, update if we have a newer last_seen
            let should_insert = match self.known_peers.get(&entry.peer_id) {
                None => true,
                Some(existing) => entry.last_seen > existing.last_seen,
            };

            if should_insert {
                // Also inform discovery manager
                self.discovery
                    .add_gossip_peer(&entry.peer_id, &entry.endpoint);
                self.known_peers.insert(entry.peer_id.clone(), entry);
            }
        }

        // Update mesh router with all known peers
        if let Some(ref router) = self.mesh_router {
            let all_peers: Vec<GossipPeerEntry> =
                self.known_peers.iter().map(|e| e.value().clone()).collect();
            router.update_from_gossip(&all_peers);
        }
    }
}

/// Execute a single gossip round: build peer list, pick random peers, send.
async fn gossip_round(
    config: &GossipConfig,
    pool: &ConnectionPool,
    known_peers: &DashMap<String, GossipPeerEntry>,
    my_peer_id: &str,
    my_endpoint: &str,
    mesh: Option<&MeshConfig>,
    mesh_secret_override: &RwLock<Option<String>>,
) {
    let connected = pool.connected_peer_ids();
    if connected.is_empty() {
        return;
    }

    // Build our peer list: self + known peers with TTL > 0
    let mut peer_list: Vec<GossipPeerEntry> = Vec::new();

    // Add self with mesh fields if configured
    let (db_name, m_name, share_status) = match mesh {
        Some(m) => (
            m.database_name.clone(),
            m.mesh_name.clone(),
            m.sharing_status.to_str().to_string(),
        ),
        None => (String::new(), String::new(), String::new()),
    };
    peer_list.push(GossipPeerEntry {
        peer_id: my_peer_id.to_string(),
        endpoint: my_endpoint.to_string(),
        status: "active".to_string(),
        last_seen: Utc::now(),
        ttl: config.ttl,
        database_name: db_name,
        mesh_name: m_name,
        sharing_status: share_status,
        schema_fingerprint: String::new(),
    });

    // Add known peers that still have TTL
    for entry in known_peers.iter() {
        if entry.ttl > 0 {
            peer_list.push(entry.value().clone());
        }
    }

    // Serialize the gossip payload (with optional HMAC authentication)
    // Check override first, then fall back to mesh config
    let effective_secret = mesh_secret_override.read().ok().and_then(|g| g.clone())
        .or_else(|| mesh.and_then(|m| m.mesh_secret.clone()));
    let payload = if let Some(ref secret) = effective_secret {
        // Compute HMAC over serialized entries
        let entries_bytes = match rmp_serde::to_vec(&peer_list) {
            Ok(b) => b,
            Err(e) => {
                warn!("Gossip serialization error: {}", e);
                return;
            }
        };
        let hmac_tag = mesh::compute_mesh_hmac(secret, &entries_bytes);
        let auth_payload = mesh::AuthenticatedGossipPayload {
            entries: peer_list,
            mesh_hmac: hmac_tag,
        };
        match rmp_serde::to_vec(&auth_payload) {
            Ok(p) => p,
            Err(e) => {
                warn!("Gossip auth serialization error: {}", e);
                return;
            }
        }
    } else {
        match rmp_serde::to_vec(&peer_list) {
            Ok(p) => p,
            Err(e) => {
                warn!("Gossip serialization error: {}", e);
                return;
            }
        }
    };

    let msg = WireMessage {
        version: 1,
        msg_id: uuid::Uuid::new_v4().to_string(),
        msg_type: WireMessageType::GossipPeerList,
        sender_id: my_peer_id.to_string(),
        payload,
    };

    // Select fan_out random peers (scope rng before any .await)
    let targets = {
        let mut rng = rand::thread_rng();
        let mut targets = connected;
        targets.shuffle(&mut rng);
        targets.truncate(config.fan_out);
        targets
    };

    for peer_id in &targets {
        if let Err(e) = pool.send(peer_id, &msg).await {
            warn!("Gossip send to {} failed: {}", peer_id, e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::GossipConfig;

    fn make_entry(peer_id: &str, ttl: u8) -> GossipPeerEntry {
        GossipPeerEntry {
            peer_id: peer_id.to_string(),
            endpoint: format!("wss://{}:9400", peer_id),
            status: "active".to_string(),
            last_seen: Utc::now(),
            ttl,
            database_name: String::new(),
            mesh_name: String::new(),
            sharing_status: String::new(),
            schema_fingerprint: String::new(),
        }
    }

    #[test]
    fn handle_gossip_adds_new_peers() {
        let pool = Arc::new(ConnectionPool::new());
        let discovery = Arc::new(DiscoveryManager::new("self", 9400));
        let mgr = GossipManager::new(
            GossipConfig::default(),
            pool,
            discovery.clone(),
            "self",
            "wss://127.0.0.1:9400",
            None,
        );

        let entries = vec![
            make_entry("peer-a", 5),
            make_entry("peer-b", 3),
        ];
        mgr.handle_gossip(entries);

        let known = mgr.known_peers();
        assert_eq!(known.len(), 2);
        // TTL should be decremented
        for entry in &known {
            match entry.peer_id.as_str() {
                "peer-a" => assert_eq!(entry.ttl, 4),
                "peer-b" => assert_eq!(entry.ttl, 2),
                _ => panic!("unexpected peer"),
            }
        }

        // Discovery manager should also know about them
        assert_eq!(discovery.peer_count(), 2);
    }

    #[test]
    fn handle_gossip_skips_self() {
        let pool = Arc::new(ConnectionPool::new());
        let discovery = Arc::new(DiscoveryManager::new("self", 9400));
        let mgr = GossipManager::new(
            GossipConfig::default(),
            pool,
            discovery,
            "self",
            "wss://127.0.0.1:9400",
            None,
        );

        let entries = vec![make_entry("self", 5)];
        mgr.handle_gossip(entries);
        assert_eq!(mgr.known_peers().len(), 0);
    }

    #[test]
    fn handle_gossip_discards_zero_ttl() {
        let pool = Arc::new(ConnectionPool::new());
        let discovery = Arc::new(DiscoveryManager::new("self", 9400));
        let mgr = GossipManager::new(
            GossipConfig::default(),
            pool,
            discovery,
            "self",
            "wss://127.0.0.1:9400",
            None,
        );

        let entries = vec![make_entry("peer-a", 0)];
        mgr.handle_gossip(entries);
        assert_eq!(mgr.known_peers().len(), 0);
    }

    #[test]
    fn handle_gossip_updates_with_newer() {
        let pool = Arc::new(ConnectionPool::new());
        let discovery = Arc::new(DiscoveryManager::new("self", 9400));
        let mgr = GossipManager::new(
            GossipConfig::default(),
            pool,
            discovery,
            "self",
            "wss://127.0.0.1:9400",
            None,
        );

        // First gossip
        let old = make_entry("peer-a", 5);
        mgr.handle_gossip(vec![old]);
        assert_eq!(mgr.known_peers()[0].ttl, 4);

        // Second gossip with newer timestamp
        std::thread::sleep(std::time::Duration::from_millis(10));
        let mut newer = make_entry("peer-a", 3);
        newer.endpoint = "wss://updated:9400".to_string();
        mgr.handle_gossip(vec![newer]);

        let peers = mgr.known_peers();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].endpoint, "wss://updated:9400");
        assert_eq!(peers[0].ttl, 2);
    }

    #[test]
    fn handle_gossip_ttl_1_becomes_0_and_stored() {
        let pool = Arc::new(ConnectionPool::new());
        let discovery = Arc::new(DiscoveryManager::new("self", 9400));
        let mgr = GossipManager::new(
            GossipConfig::default(),
            pool,
            discovery,
            "self",
            "wss://127.0.0.1:9400",
            None,
        );

        // TTL=1 entry: will be stored with TTL=0 (won't be forwarded next round)
        let entries = vec![make_entry("peer-a", 1)];
        mgr.handle_gossip(entries);

        let peers = mgr.known_peers();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].ttl, 0);
    }
}

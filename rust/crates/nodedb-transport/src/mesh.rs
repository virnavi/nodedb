use std::time::Duration;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::types::GossipPeerEntry;

type HmacSha256 = Hmac<Sha256>;

/// Controls how a database participates in the mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeshSharingStatus {
    /// Data is not shared. Visible in topology but returns no data.
    Private,
    /// Other mesh databases may read data (subject to DAC rules).
    ReadOnly,
    /// Other mesh databases may read and this database may write (subject to DAC rules).
    ReadWrite,
    /// Full participant: read, write, and relay.
    Full,
}

impl MeshSharingStatus {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "private" => Some(MeshSharingStatus::Private),
            "read_only" => Some(MeshSharingStatus::ReadOnly),
            "read_write" => Some(MeshSharingStatus::ReadWrite),
            "full" => Some(MeshSharingStatus::Full),
            _ => None,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            MeshSharingStatus::Private => "private",
            MeshSharingStatus::ReadOnly => "read_only",
            MeshSharingStatus::ReadWrite => "read_write",
            MeshSharingStatus::Full => "full",
        }
    }

    /// Returns true if this status allows read queries from mesh peers.
    pub fn allows_reads(&self) -> bool {
        matches!(self, MeshSharingStatus::ReadOnly | MeshSharingStatus::ReadWrite | MeshSharingStatus::Full)
    }

    /// Returns true if this status allows write queries from mesh peers.
    pub fn allows_writes(&self) -> bool {
        matches!(self, MeshSharingStatus::ReadWrite | MeshSharingStatus::Full)
    }
}

/// Configuration for mesh participation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshConfig {
    /// The name of the mesh this database joins.
    pub mesh_name: String,
    /// This database's name within the mesh.
    pub database_name: String,
    /// How this database shares data with the mesh.
    pub sharing_status: MeshSharingStatus,
    /// Maximum mesh peers to maintain connections to.
    #[serde(default = "default_max_mesh_peers")]
    pub max_mesh_peers: usize,
    /// Optional shared secret for HMAC mesh authentication.
    #[serde(default)]
    pub mesh_secret: Option<String>,
}

fn default_max_mesh_peers() -> usize {
    16
}

/// Compute HMAC-SHA256 tag over payload using the mesh secret.
pub fn compute_mesh_hmac(secret: &str, payload: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(payload);
    mac.finalize().into_bytes().to_vec()
}

/// Verify HMAC-SHA256 tag over payload using the mesh secret.
pub fn verify_mesh_hmac(secret: &str, payload: &[u8], tag: &[u8]) -> bool {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(payload);
    mac.verify_slice(tag).is_ok()
}

/// Wrapper for gossip payloads with optional HMAC authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedGossipPayload {
    pub entries: Vec<GossipPeerEntry>,
    #[serde(default)]
    pub mesh_hmac: Vec<u8>,
}

/// Information about a peer hosting a specific database in the mesh.
#[derive(Debug, Clone)]
pub struct MeshPeerInfo {
    pub peer_id: String,
    pub endpoint: String,
    pub sharing_status: MeshSharingStatus,
    pub last_seen: DateTime<Utc>,
}

/// Current mesh status for this node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshStatus {
    pub mesh_name: String,
    pub database_name: String,
    pub sharing_status: String,
    pub member_count: usize,
}

/// Summary of a known mesh member (for listing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshMember {
    pub database_name: String,
    pub peer_id: String,
    pub sharing_status: String,
    pub last_seen: DateTime<Utc>,
}

/// Routes queries to the correct database peer based on gossip-learned topology.
pub struct MeshRouter {
    /// My mesh name (only entries with matching mesh_name are tracked).
    mesh_name: String,
    /// database_name → list of peers hosting that database.
    routes: DashMap<String, Vec<MeshPeerInfo>>,
}

impl MeshRouter {
    pub fn new(mesh_name: &str) -> Self {
        MeshRouter {
            mesh_name: mesh_name.to_string(),
            routes: DashMap::new(),
        }
    }

    /// Update routing table from gossip entries.
    /// Only entries matching our mesh_name with non-empty database_name are tracked.
    pub fn update_from_gossip(&self, entries: &[GossipPeerEntry]) {
        for entry in entries {
            if entry.mesh_name != self.mesh_name || entry.database_name.is_empty() {
                continue;
            }
            let status = MeshSharingStatus::from_str(&entry.sharing_status)
                .unwrap_or(MeshSharingStatus::Private);
            let info = MeshPeerInfo {
                peer_id: entry.peer_id.clone(),
                endpoint: entry.endpoint.clone(),
                sharing_status: status,
                last_seen: entry.last_seen,
            };
            let mut peers = self.routes.entry(entry.database_name.clone()).or_default();
            // Update existing or insert new
            if let Some(existing) = peers.iter_mut().find(|p| p.peer_id == info.peer_id) {
                *existing = info;
            } else {
                peers.push(info);
            }
        }
    }

    /// Get peers hosting the named database.
    pub fn route(&self, database_name: &str) -> Vec<MeshPeerInfo> {
        self.routes
            .get(database_name)
            .map(|peers| peers.clone())
            .unwrap_or_default()
    }

    /// List all known mesh members.
    pub fn members(&self) -> Vec<MeshMember> {
        let mut result = Vec::new();
        for entry in self.routes.iter() {
            let db_name = entry.key().clone();
            for peer in entry.value() {
                result.push(MeshMember {
                    database_name: db_name.clone(),
                    peer_id: peer.peer_id.clone(),
                    sharing_status: peer.sharing_status.to_str().to_string(),
                    last_seen: peer.last_seen,
                });
            }
        }
        result
    }

    /// Remove entries older than max_age.
    pub fn remove_stale(&self, max_age: Duration) {
        let cutoff = Utc::now() - chrono::Duration::from_std(max_age).unwrap_or(chrono::Duration::hours(1));
        self.routes.retain(|_, peers| {
            peers.retain(|p| p.last_seen > cutoff);
            !peers.is_empty()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_gossip_entry(peer_id: &str, db_name: &str, mesh_name: &str, status: &str) -> GossipPeerEntry {
        GossipPeerEntry {
            peer_id: peer_id.to_string(),
            endpoint: format!("wss://{}:9400", peer_id),
            status: "active".to_string(),
            last_seen: Utc::now(),
            ttl: 5,
            database_name: db_name.to_string(),
            mesh_name: mesh_name.to_string(),
            sharing_status: status.to_string(),
            schema_fingerprint: String::new(),
        }
    }

    #[test]
    fn mesh_router_update_and_route() {
        let router = MeshRouter::new("corp-mesh");
        let entries = vec![
            make_gossip_entry("peer-a", "warehouse", "corp-mesh", "read_write"),
            make_gossip_entry("peer-b", "analytics", "corp-mesh", "read_only"),
            make_gossip_entry("peer-c", "warehouse", "corp-mesh", "full"),
        ];
        router.update_from_gossip(&entries);

        let warehouse_peers = router.route("warehouse");
        assert_eq!(warehouse_peers.len(), 2);
        let analytics_peers = router.route("analytics");
        assert_eq!(analytics_peers.len(), 1);
        assert_eq!(analytics_peers[0].peer_id, "peer-b");
        assert!(router.route("unknown").is_empty());
    }

    #[test]
    fn mesh_router_filters_by_mesh_name() {
        let router = MeshRouter::new("mesh-a");
        let entries = vec![
            make_gossip_entry("peer-1", "db1", "mesh-a", "full"),
            make_gossip_entry("peer-2", "db2", "mesh-b", "full"), // different mesh
            make_gossip_entry("peer-3", "db3", "", "full"),       // no mesh
        ];
        router.update_from_gossip(&entries);

        assert_eq!(router.members().len(), 1);
        assert_eq!(router.route("db1").len(), 1);
        assert!(router.route("db2").is_empty());
    }

    #[test]
    fn mesh_router_updates_existing_peer() {
        let router = MeshRouter::new("mesh");
        let entries1 = vec![make_gossip_entry("peer-a", "db", "mesh", "read_only")];
        router.update_from_gossip(&entries1);
        assert_eq!(router.route("db")[0].sharing_status, MeshSharingStatus::ReadOnly);

        let entries2 = vec![make_gossip_entry("peer-a", "db", "mesh", "full")];
        router.update_from_gossip(&entries2);
        let peers = router.route("db");
        assert_eq!(peers.len(), 1); // not duplicated
        assert_eq!(peers[0].sharing_status, MeshSharingStatus::Full);
    }

    #[test]
    fn mesh_router_members_lists_all() {
        let router = MeshRouter::new("mesh");
        let entries = vec![
            make_gossip_entry("peer-a", "db1", "mesh", "full"),
            make_gossip_entry("peer-b", "db2", "mesh", "read_only"),
        ];
        router.update_from_gossip(&entries);

        let members = router.members();
        assert_eq!(members.len(), 2);
    }

    #[test]
    fn mesh_router_remove_stale() {
        let router = MeshRouter::new("mesh");
        let mut old_entry = make_gossip_entry("peer-old", "db", "mesh", "full");
        old_entry.last_seen = Utc::now() - chrono::Duration::hours(2);
        router.update_from_gossip(&[old_entry]);

        let fresh = make_gossip_entry("peer-new", "db2", "mesh", "full");
        router.update_from_gossip(&[fresh]);

        router.remove_stale(Duration::from_secs(3600)); // 1 hour
        assert!(router.route("db").is_empty());
        assert_eq!(router.route("db2").len(), 1);
    }

    #[test]
    fn hmac_roundtrip() {
        let secret = "my-mesh-secret";
        let payload = b"hello gossip data";
        let tag = compute_mesh_hmac(secret, payload);
        assert!(verify_mesh_hmac(secret, payload, &tag));
    }

    #[test]
    fn hmac_wrong_secret_fails() {
        let tag = compute_mesh_hmac("secret-a", b"data");
        assert!(!verify_mesh_hmac("secret-b", b"data", &tag));
    }

    #[test]
    fn hmac_tampered_payload_fails() {
        let tag = compute_mesh_hmac("secret", b"original");
        assert!(!verify_mesh_hmac("secret", b"tampered", &tag));
    }

    #[test]
    fn authenticated_gossip_payload_serde_roundtrip() {
        let entries = vec![make_gossip_entry("peer-a", "db", "mesh", "full")];
        let entries_bytes = rmp_serde::to_vec(&entries).unwrap();
        let hmac_tag = compute_mesh_hmac("secret", &entries_bytes);
        let payload = AuthenticatedGossipPayload {
            entries: entries.clone(),
            mesh_hmac: hmac_tag.clone(),
        };
        let bytes = rmp_serde::to_vec(&payload).unwrap();
        let decoded: AuthenticatedGossipPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.entries.len(), 1);
        assert_eq!(decoded.mesh_hmac, hmac_tag);
    }

    #[test]
    fn mesh_sharing_status_serde_roundtrip() {
        for status in [
            MeshSharingStatus::Private,
            MeshSharingStatus::ReadOnly,
            MeshSharingStatus::ReadWrite,
            MeshSharingStatus::Full,
        ] {
            let bytes = rmp_serde::to_vec(&status).unwrap();
            let decoded: MeshSharingStatus = rmp_serde::from_slice(&bytes).unwrap();
            assert_eq!(decoded, status);
        }
    }

    #[test]
    fn mesh_sharing_status_str_roundtrip() {
        for status in [
            MeshSharingStatus::Private,
            MeshSharingStatus::ReadOnly,
            MeshSharingStatus::ReadWrite,
            MeshSharingStatus::Full,
        ] {
            let s = status.to_str();
            let parsed = MeshSharingStatus::from_str(s).unwrap();
            assert_eq!(parsed, status);
        }
        assert!(MeshSharingStatus::from_str("invalid").is_none());
    }

    #[test]
    fn mesh_sharing_status_permissions() {
        assert!(!MeshSharingStatus::Private.allows_reads());
        assert!(!MeshSharingStatus::Private.allows_writes());

        assert!(MeshSharingStatus::ReadOnly.allows_reads());
        assert!(!MeshSharingStatus::ReadOnly.allows_writes());

        assert!(MeshSharingStatus::ReadWrite.allows_reads());
        assert!(MeshSharingStatus::ReadWrite.allows_writes());

        assert!(MeshSharingStatus::Full.allows_reads());
        assert!(MeshSharingStatus::Full.allows_writes());
    }

    #[test]
    fn mesh_config_serde_roundtrip() {
        let config = MeshConfig {
            mesh_name: "corp-mesh".to_string(),
            database_name: "warehouse".to_string(),
            sharing_status: MeshSharingStatus::ReadWrite,
            max_mesh_peers: 16,
            mesh_secret: Some("secret123".to_string()),
        };
        let bytes = rmp_serde::to_vec(&config).unwrap();
        let decoded: MeshConfig = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.mesh_name, "corp-mesh");
        assert_eq!(decoded.database_name, "warehouse");
        assert_eq!(decoded.sharing_status, MeshSharingStatus::ReadWrite);
        assert_eq!(decoded.max_mesh_peers, 16);
        assert_eq!(decoded.mesh_secret.unwrap(), "secret123");
    }

    #[test]
    fn mesh_config_defaults() {
        let config = MeshConfig {
            mesh_name: "test".to_string(),
            database_name: "db".to_string(),
            sharing_status: MeshSharingStatus::Full,
            max_mesh_peers: default_max_mesh_peers(),
            mesh_secret: None,
        };
        assert_eq!(config.max_mesh_peers, 16);
        assert!(config.mesh_secret.is_none());
    }
}

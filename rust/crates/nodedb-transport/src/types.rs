use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Configuration for the transport layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    pub storage_path: String,
    pub listen_addr: String,
    pub tls_cert_pem: Option<String>,
    pub tls_key_pem: Option<String>,
    pub seed_peers: Vec<String>,
    pub mdns_enabled: bool,
    pub gossip: GossipConfig,
    pub query_policy: FederatedQueryPolicy,
    /// Optional mesh configuration for database mesh networking.
    #[serde(default)]
    pub mesh: Option<crate::mesh::MeshConfig>,
    /// Trusted peer public keys (hex-encoded Ed25519 public keys).
    /// If non-empty, only peers whose peer_id matches a key in this set
    /// will be accepted during handshake. If empty, accept all peers.
    #[serde(default)]
    pub trusted_peer_keys: Vec<String>,
    /// When true, unknown peers must go through the pairing approval flow
    /// before they are accepted. Requires persistent storage.
    #[serde(default)]
    pub require_pairing: bool,
    /// This user's globally unique ID (UUID). Exchanged during handshake
    /// and stored in the pairing record on the remote device.
    #[serde(default)]
    pub user_id: String,
    /// Human-readable device name sent to peers during handshake.
    #[serde(default)]
    pub device_name: String,
}

impl Default for TransportConfig {
    fn default() -> Self {
        TransportConfig {
            storage_path: String::new(),
            listen_addr: "0.0.0.0:9400".to_string(),
            tls_cert_pem: None,
            tls_key_pem: None,
            seed_peers: vec![],
            mdns_enabled: true,
            gossip: GossipConfig::default(),
            query_policy: FederatedQueryPolicy::QueryPeersOnMiss,
            mesh: None,
            trusted_peer_keys: vec![],
            require_pairing: false,
            user_id: String::new(),
            device_name: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipConfig {
    pub interval_seconds: u64,
    pub fan_out: usize,
    pub ttl: u8,
}

impl Default for GossipConfig {
    fn default() -> Self {
        GossipConfig {
            interval_seconds: 30,
            fan_out: 3,
            ttl: 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FederatedQueryPolicy {
    LocalOnly,
    QueryPeersOnMiss,
    QueryPeersAlways,
    QueryPeersExplicitly,
}

/// The wire protocol envelope for all messages between peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireMessage {
    pub version: u8,
    pub msg_id: String,
    pub msg_type: WireMessageType,
    pub sender_id: String,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WireMessageType {
    Hello,
    HelloAck,
    GossipPeerList,
    QueryRequest,
    QueryResponse,
    Ping,
    Pong,
    TriggerNotification,
    PreferenceSync,
    SingletonSync,
}

/// Gossip payload: peer topology metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipPeerEntry {
    pub peer_id: String,
    pub endpoint: String,
    pub status: String,
    pub last_seen: DateTime<Utc>,
    pub ttl: u8,
    /// Database name for mesh routing (empty if not in a mesh).
    #[serde(default)]
    pub database_name: String,
    /// Mesh name this peer belongs to (empty if not in a mesh).
    #[serde(default)]
    pub mesh_name: String,
    /// Sharing status as string (empty if not in a mesh).
    #[serde(default)]
    pub sharing_status: String,
    /// Schema fingerprint (placeholder for v1.2).
    #[serde(default)]
    pub schema_fingerprint: String,
}

/// Per-peer credential stored in memory only.
#[derive(Debug, Clone)]
pub enum PeerCredential {
    BearerToken(String),
    MtlsCert(Vec<u8>),
    Custom(Vec<u8>),
}

/// Callback result for peer acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerAcceptance {
    Accept,
    Reject,
}

/// Source of peer discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscoverySource {
    Mdns,
    Seed,
    Gossip,
}

/// A discovered peer (before connection).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPeer {
    pub peer_id: String,
    pub endpoint: String,
    pub source: DiscoverySource,
    pub discovered_at: DateTime<Utc>,
}

/// Audit log entry for outbound shares.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeShareAuditEntry {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub peer_id: String,
    pub action: String,
    pub collection: Option<String>,
    pub record_count: u64,
    pub content_hash: String,
}

/// Structured envelope for federated query requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedQueryEnvelope {
    pub query_type: String,
    pub query_id: String,
    pub origin_peer_id: String,
    pub ttl: u8,
    pub query_data: Vec<u8>,
    /// Peer IDs already visited by this query (for loop prevention in multi-hop).
    /// Defaults to empty for backward compatibility with old peers.
    #[serde(default)]
    pub visited: Vec<String>,
}

/// Configuration for bounded multi-hop search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Maximum hop depth for federated queries (default 3).
    pub max_depth: u8,
    /// Overall timeout in seconds for the originator (default 10).
    pub timeout_secs: u64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            max_depth: 3,
            timeout_secs: 10,
        }
    }
}

/// Payload for cross-database trigger notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerNotificationPayload {
    /// The source database name.
    pub source_database: String,
    /// The collection that was modified (FQN).
    pub collection: String,
    /// The event type: "insert", "update", "delete".
    pub event: String,
    /// The old record (serialized Document, None for inserts).
    pub old_record: Option<Vec<u8>>,
    /// The new record (serialized Document, None for deletes).
    pub new_record: Option<Vec<u8>>,
}

/// Payload for preference sync between mesh peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferenceSyncPayload {
    /// The source database name.
    pub source_database: String,
    /// The preference store name.
    pub preference_store: String,
    /// The preference key.
    pub key: String,
    /// Encrypted value bytes.
    pub encrypted_value: Vec<u8>,
    /// Value type hint (e.g. "string", "integer").
    pub value_type: String,
    /// When this preference was last updated (millis since epoch).
    pub updated_at: i64,
    /// Conflict resolution strategy.
    pub conflict_resolution: String,
    /// Optional confidence score.
    #[serde(default)]
    pub confidence: Option<f64>,
}

/// Payload for singleton sync between mesh peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingletonSyncPayload {
    /// The source database name.
    pub source_database: String,
    /// The singleton collection name.
    pub collection: String,
    /// Serialized record data.
    pub record_data: Vec<u8>,
    /// When this record was last updated (millis since epoch).
    pub updated_at: i64,
    /// Conflict resolution strategy.
    pub conflict_resolution: String,
}

/// Structured response for federated queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedQueryResponse {
    pub query_id: String,
    pub responder_peer_id: String,
    pub success: bool,
    pub result_data: Vec<u8>,
    pub error_message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_config_serde_roundtrip() {
        let config = TransportConfig::default();
        let bytes = rmp_serde::to_vec(&config).unwrap();
        let decoded: TransportConfig = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.listen_addr, "0.0.0.0:9400");
        assert!(decoded.mdns_enabled);
        assert!(decoded.mesh.is_none());
    }

    #[test]
    fn transport_config_backward_compat_no_mesh() {
        // Simulate old config without mesh field
        #[derive(Serialize)]
        struct OldConfig {
            storage_path: String,
            listen_addr: String,
            tls_cert_pem: Option<String>,
            tls_key_pem: Option<String>,
            seed_peers: Vec<String>,
            mdns_enabled: bool,
            gossip: GossipConfig,
            query_policy: FederatedQueryPolicy,
        }
        let old = OldConfig {
            storage_path: String::new(),
            listen_addr: "0.0.0.0:9400".to_string(),
            tls_cert_pem: None,
            tls_key_pem: None,
            seed_peers: vec![],
            mdns_enabled: true,
            gossip: GossipConfig::default(),
            query_policy: FederatedQueryPolicy::QueryPeersOnMiss,
        };
        let bytes = rmp_serde::to_vec(&old).unwrap();
        let decoded: TransportConfig = rmp_serde::from_slice(&bytes).unwrap();
        assert!(decoded.mesh.is_none());
    }

    #[test]
    fn gossip_config_defaults() {
        let gc = GossipConfig::default();
        assert_eq!(gc.interval_seconds, 30);
        assert_eq!(gc.fan_out, 3);
        assert_eq!(gc.ttl, 5);
    }

    #[test]
    fn wire_message_serde_roundtrip() {
        let msg = WireMessage {
            version: 1,
            msg_id: "test-123".to_string(),
            msg_type: WireMessageType::Hello,
            sender_id: "abc".to_string(),
            payload: vec![1, 2, 3],
        };
        let bytes = rmp_serde::to_vec(&msg).unwrap();
        let decoded: WireMessage = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.msg_type, WireMessageType::Hello);
        assert_eq!(decoded.payload, vec![1, 2, 3]);
    }

    #[test]
    fn gossip_peer_entry_serde_roundtrip() {
        let entry = GossipPeerEntry {
            peer_id: "peer1".to_string(),
            endpoint: "wss://localhost:9400".to_string(),
            status: "active".to_string(),
            last_seen: Utc::now(),
            ttl: 5,
            database_name: "my-db".to_string(),
            mesh_name: "test-mesh".to_string(),
            sharing_status: "read_write".to_string(),
            schema_fingerprint: String::new(),
        };
        let bytes = rmp_serde::to_vec(&entry).unwrap();
        let decoded: GossipPeerEntry = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.peer_id, "peer1");
        assert_eq!(decoded.ttl, 5);
        assert_eq!(decoded.database_name, "my-db");
        assert_eq!(decoded.mesh_name, "test-mesh");
        assert_eq!(decoded.sharing_status, "read_write");
    }

    #[test]
    fn gossip_peer_entry_backward_compat_no_mesh_fields() {
        // Simulate an old entry without mesh fields
        #[derive(Serialize)]
        struct OldGossipPeerEntry {
            peer_id: String,
            endpoint: String,
            status: String,
            last_seen: DateTime<Utc>,
            ttl: u8,
        }
        let old = OldGossipPeerEntry {
            peer_id: "peer-old".to_string(),
            endpoint: "wss://old:9400".to_string(),
            status: "active".to_string(),
            last_seen: Utc::now(),
            ttl: 3,
        };
        let bytes = rmp_serde::to_vec(&old).unwrap();
        let decoded: GossipPeerEntry = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.peer_id, "peer-old");
        assert_eq!(decoded.ttl, 3);
        assert!(decoded.database_name.is_empty());
        assert!(decoded.mesh_name.is_empty());
        assert!(decoded.sharing_status.is_empty());
        assert!(decoded.schema_fingerprint.is_empty());
    }

    #[test]
    fn federated_query_policy_serde_roundtrip() {
        for policy in [
            FederatedQueryPolicy::LocalOnly,
            FederatedQueryPolicy::QueryPeersOnMiss,
            FederatedQueryPolicy::QueryPeersAlways,
            FederatedQueryPolicy::QueryPeersExplicitly,
        ] {
            let bytes = rmp_serde::to_vec(&policy).unwrap();
            let decoded: FederatedQueryPolicy = rmp_serde::from_slice(&bytes).unwrap();
            assert_eq!(decoded, policy);
        }
    }

    #[test]
    fn wire_message_type_all_variants() {
        for mt in [
            WireMessageType::Hello,
            WireMessageType::HelloAck,
            WireMessageType::GossipPeerList,
            WireMessageType::QueryRequest,
            WireMessageType::QueryResponse,
            WireMessageType::Ping,
            WireMessageType::Pong,
            WireMessageType::TriggerNotification,
            WireMessageType::PreferenceSync,
            WireMessageType::SingletonSync,
        ] {
            let bytes = rmp_serde::to_vec(&mt).unwrap();
            let decoded: WireMessageType = rmp_serde::from_slice(&bytes).unwrap();
            assert_eq!(decoded, mt);
        }
    }

    #[test]
    fn audit_entry_serde_roundtrip() {
        let entry = NodeShareAuditEntry {
            id: 1,
            timestamp: Utc::now(),
            peer_id: "peer1".to_string(),
            action: "query_response".to_string(),
            collection: Some("users".to_string()),
            record_count: 5,
            content_hash: "abc123".to_string(),
        };
        let bytes = rmp_serde::to_vec(&entry).unwrap();
        let decoded: NodeShareAuditEntry = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.record_count, 5);
    }

    #[test]
    fn discovery_source_serde_roundtrip() {
        for src in [DiscoverySource::Mdns, DiscoverySource::Seed, DiscoverySource::Gossip] {
            let bytes = rmp_serde::to_vec(&src).unwrap();
            let decoded: DiscoverySource = rmp_serde::from_slice(&bytes).unwrap();
            assert_eq!(decoded, src);
        }
    }

    #[test]
    fn federated_query_envelope_serde_roundtrip() {
        let envelope = FederatedQueryEnvelope {
            query_type: "nosql".to_string(),
            query_id: "q-123".to_string(),
            origin_peer_id: "peer-abc".to_string(),
            ttl: 3,
            query_data: vec![10, 20, 30],
            visited: vec!["peer-abc".to_string()],
        };
        let bytes = rmp_serde::to_vec(&envelope).unwrap();
        let decoded: FederatedQueryEnvelope = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.query_type, "nosql");
        assert_eq!(decoded.query_id, "q-123");
        assert_eq!(decoded.origin_peer_id, "peer-abc");
        assert_eq!(decoded.ttl, 3);
        assert_eq!(decoded.query_data, vec![10, 20, 30]);
        assert_eq!(decoded.visited, vec!["peer-abc"]);
    }

    #[test]
    fn federated_query_envelope_backward_compat_no_visited() {
        // Simulate an old envelope without the `visited` field by serializing
        // a struct that lacks it, then deserializing into the new struct.
        #[derive(Serialize)]
        struct OldEnvelope {
            query_type: String,
            query_id: String,
            origin_peer_id: String,
            ttl: u8,
            query_data: Vec<u8>,
        }

        let old = OldEnvelope {
            query_type: "graph".to_string(),
            query_id: "q-old".to_string(),
            origin_peer_id: "peer-old".to_string(),
            ttl: 2,
            query_data: vec![1],
        };
        let bytes = rmp_serde::to_vec(&old).unwrap();
        let decoded: FederatedQueryEnvelope = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.query_type, "graph");
        assert_eq!(decoded.ttl, 2);
        assert!(decoded.visited.is_empty(), "visited should default to empty vec");
    }

    #[test]
    fn search_config_defaults() {
        let config = SearchConfig::default();
        assert_eq!(config.max_depth, 3);
        assert_eq!(config.timeout_secs, 10);
    }

    #[test]
    fn search_config_serde_roundtrip() {
        let config = SearchConfig {
            max_depth: 5,
            timeout_secs: 30,
        };
        let bytes = rmp_serde::to_vec(&config).unwrap();
        let decoded: SearchConfig = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.max_depth, 5);
        assert_eq!(decoded.timeout_secs, 30);
    }

    #[test]
    fn federated_query_response_serde_roundtrip() {
        let resp = FederatedQueryResponse {
            query_id: "q-123".to_string(),
            responder_peer_id: "peer-xyz".to_string(),
            success: true,
            result_data: vec![1, 2, 3],
            error_message: None,
        };
        let bytes = rmp_serde::to_vec(&resp).unwrap();
        let decoded: FederatedQueryResponse = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.query_id, "q-123");
        assert!(decoded.success);
        assert!(decoded.error_message.is_none());

        let resp_err = FederatedQueryResponse {
            query_id: "q-456".to_string(),
            responder_peer_id: "peer-xyz".to_string(),
            success: false,
            result_data: vec![],
            error_message: Some("not found".to_string()),
        };
        let bytes = rmp_serde::to_vec(&resp_err).unwrap();
        let decoded: FederatedQueryResponse = rmp_serde::from_slice(&bytes).unwrap();
        assert!(!decoded.success);
        assert_eq!(decoded.error_message.unwrap(), "not found");
    }

    #[test]
    fn trigger_notification_payload_serde_roundtrip() {
        let payload = TriggerNotificationPayload {
            source_database: "warehouse".to_string(),
            collection: "public.products".to_string(),
            event: "insert".to_string(),
            old_record: None,
            new_record: Some(vec![1, 2, 3]),
        };
        let bytes = rmp_serde::to_vec(&payload).unwrap();
        let decoded: TriggerNotificationPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.source_database, "warehouse");
        assert_eq!(decoded.collection, "public.products");
        assert_eq!(decoded.event, "insert");
        assert!(decoded.old_record.is_none());
        assert_eq!(decoded.new_record.unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn trigger_notification_payload_delete_event() {
        let payload = TriggerNotificationPayload {
            source_database: "warehouse".to_string(),
            collection: "public.products".to_string(),
            event: "delete".to_string(),
            old_record: Some(vec![4, 5, 6]),
            new_record: None,
        };
        let bytes = rmp_serde::to_vec(&payload).unwrap();
        let decoded: TriggerNotificationPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.event, "delete");
        assert_eq!(decoded.old_record.unwrap(), vec![4, 5, 6]);
        assert!(decoded.new_record.is_none());
    }

    #[test]
    fn preference_sync_payload_serde_roundtrip() {
        let payload = PreferenceSyncPayload {
            source_database: "warehouse".to_string(),
            preference_store: "app_prefs".to_string(),
            key: "theme".to_string(),
            encrypted_value: vec![1, 2, 3, 4],
            value_type: "string".to_string(),
            updated_at: 1234567890,
            conflict_resolution: "last_write_wins".to_string(),
            confidence: Some(0.95),
        };
        let bytes = rmp_serde::to_vec(&payload).unwrap();
        let decoded: PreferenceSyncPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.source_database, "warehouse");
        assert_eq!(decoded.preference_store, "app_prefs");
        assert_eq!(decoded.key, "theme");
        assert_eq!(decoded.encrypted_value, vec![1, 2, 3, 4]);
        assert_eq!(decoded.value_type, "string");
        assert_eq!(decoded.updated_at, 1234567890);
        assert_eq!(decoded.conflict_resolution, "last_write_wins");
        assert_eq!(decoded.confidence, Some(0.95));
    }

    #[test]
    fn singleton_sync_payload_serde_roundtrip() {
        let payload = SingletonSyncPayload {
            source_database: "warehouse".to_string(),
            collection: "settings".to_string(),
            record_data: vec![10, 20, 30],
            updated_at: 9876543210,
            conflict_resolution: "remote_wins".to_string(),
        };
        let bytes = rmp_serde::to_vec(&payload).unwrap();
        let decoded: SingletonSyncPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.source_database, "warehouse");
        assert_eq!(decoded.collection, "settings");
        assert_eq!(decoded.record_data, vec![10, 20, 30]);
        assert_eq!(decoded.updated_at, 9876543210);
        assert_eq!(decoded.conflict_resolution, "remote_wins");
    }

    #[test]
    fn preference_sync_payload_no_confidence() {
        let payload = PreferenceSyncPayload {
            source_database: "db".to_string(),
            preference_store: "prefs".to_string(),
            key: "k".to_string(),
            encrypted_value: vec![],
            value_type: "nil".to_string(),
            updated_at: 0,
            conflict_resolution: "local_wins".to_string(),
            confidence: None,
        };
        let bytes = rmp_serde::to_vec(&payload).unwrap();
        let decoded: PreferenceSyncPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert!(decoded.confidence.is_none());
    }
}

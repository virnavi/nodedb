use chrono::{DateTime, Utc};
use rmpv::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerStatus {
    Active,
    Inactive,
    Banned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePeer {
    pub id: i64,
    pub name: String,
    pub endpoint: Option<String>,
    pub public_key: Option<String>,
    pub status: PeerStatus,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeGroup {
    pub id: i64,
    pub name: String,
    pub members: Vec<i64>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_status_serde_roundtrip() {
        for status in [PeerStatus::Active, PeerStatus::Inactive, PeerStatus::Banned] {
            let bytes = rmp_serde::to_vec(&status).unwrap();
            let decoded: PeerStatus = rmp_serde::from_slice(&bytes).unwrap();
            assert_eq!(decoded, status);
        }
    }

    #[test]
    fn node_peer_serde_roundtrip() {
        let peer = NodePeer {
            id: 1,
            name: "alice".to_string(),
            endpoint: Some("ws://localhost:8080".to_string()),
            public_key: None,
            status: PeerStatus::Active,
            metadata: Value::Map(vec![
                (Value::String("role".into()), Value::String("admin".into())),
            ]),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let bytes = rmp_serde::to_vec(&peer).unwrap();
        let decoded: NodePeer = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, peer.id);
        assert_eq!(decoded.name, peer.name);
        assert_eq!(decoded.endpoint, peer.endpoint);
        assert_eq!(decoded.status, peer.status);
    }

    #[test]
    fn node_group_serde_roundtrip() {
        let group = NodeGroup {
            id: 1,
            name: "admins".to_string(),
            members: vec![1, 2, 3],
            metadata: Value::Nil,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let bytes = rmp_serde::to_vec(&group).unwrap();
        let decoded: NodeGroup = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, group.id);
        assert_eq!(decoded.name, group.name);
        assert_eq!(decoded.members, group.members);
    }
}

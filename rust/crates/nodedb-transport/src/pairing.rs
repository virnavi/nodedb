use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use nodedb_storage::StorageEngine;

use crate::error::TransportError;

/// A persistently stored pairing record for an approved device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingRecord {
    /// The paired peer's Ed25519 public key hex (64 chars).
    pub peer_id: String,
    /// Raw Ed25519 public key bytes (32 bytes).
    pub public_key_bytes: Vec<u8>,
    /// The paired user's globally unique ID (UUID).
    pub user_id: String,
    /// Human-readable device name.
    pub device_name: String,
    /// When the pairing was approved.
    pub paired_at: DateTime<Utc>,
    /// When this peer was last successfully verified.
    pub last_verified_at: DateTime<Utc>,
}

/// An ephemeral pending pairing request (lives only in memory).
#[derive(Debug, Clone)]
pub struct PendingPairingRequest {
    pub peer_id: String,
    pub public_key_bytes: Vec<u8>,
    pub user_id: String,
    pub device_name: String,
    pub endpoint: String,
    pub received_at: DateTime<Utc>,
}

/// Persistent pairing store backed by sled + in-memory pending requests.
pub struct PairingStore {
    tree: nodedb_storage::StorageTree,
    pending: DashMap<String, PendingPairingRequest>,
}

impl PairingStore {
    /// Open the pairing store backed by a sled tree.
    pub fn new(engine: Arc<StorageEngine>) -> Result<Self, TransportError> {
        let tree = engine
            .open_tree("__pairing__")
            .map_err(|e| TransportError::Pairing(e.to_string()))?;
        Ok(PairingStore {
            tree,
            pending: DashMap::new(),
        })
    }

    /// Check if a peer_id is an approved paired device.
    pub fn is_paired(&self, peer_id: &str) -> bool {
        self.tree.contains_key(peer_id.as_bytes()).unwrap_or(false)
    }

    /// Get the pairing record for a peer_id (if paired).
    pub fn get_paired(&self, peer_id: &str) -> Option<PairingRecord> {
        self.tree
            .get(peer_id.as_bytes())
            .ok()
            .flatten()
            .and_then(|v| rmp_serde::from_slice(&v).ok())
    }

    /// List all approved pairings.
    pub fn list_paired(&self) -> Vec<PairingRecord> {
        self.tree
            .iter()
            .filter_map(|r| r.ok())
            .filter_map(|(_, v)| rmp_serde::from_slice(&v).ok())
            .collect()
    }

    /// Add a pending pairing request (from an unknown peer during handshake).
    pub fn add_pending(&self, request: PendingPairingRequest) {
        self.pending.insert(request.peer_id.clone(), request);
    }

    /// List all pending requests.
    pub fn list_pending(&self) -> Vec<PendingPairingRequest> {
        self.pending.iter().map(|r| r.value().clone()).collect()
    }

    /// Approve a pending request: moves from pending to sled.
    pub fn approve(&self, peer_id: &str) -> Result<Option<PairingRecord>, TransportError> {
        let request = match self.pending.remove(peer_id) {
            Some((_, r)) => r,
            None => return Ok(None),
        };

        let now = Utc::now();
        let record = PairingRecord {
            peer_id: request.peer_id,
            public_key_bytes: request.public_key_bytes,
            user_id: request.user_id,
            device_name: request.device_name,
            paired_at: now,
            last_verified_at: now,
        };

        let value = rmp_serde::to_vec(&record)
            .map_err(|e| TransportError::Pairing(e.to_string()))?;
        self.tree
            .insert(record.peer_id.as_bytes(), &value)
            .map_err(|e| TransportError::Pairing(e.to_string()))?;

        Ok(Some(record))
    }

    /// Reject a pending request: removes from pending map.
    pub fn reject(&self, peer_id: &str) -> bool {
        self.pending.remove(peer_id).is_some()
    }

    /// Remove a paired device (unpair).
    pub fn remove_paired(&self, peer_id: &str) -> Result<bool, TransportError> {
        let existed = self
            .tree
            .remove(peer_id.as_bytes())
            .map_err(|e| TransportError::Pairing(e.to_string()))?
            .is_some();
        Ok(existed)
    }

    /// Update last_verified_at timestamp for a paired device.
    pub fn touch_verified(&self, peer_id: &str) -> Result<(), TransportError> {
        if let Some(mut record) = self.get_paired(peer_id) {
            record.last_verified_at = Utc::now();
            let value = rmp_serde::to_vec(&record)
                .map_err(|e| TransportError::Pairing(e.to_string()))?;
            self.tree
                .insert(record.peer_id.as_bytes(), &value)
                .map_err(|e| TransportError::Pairing(e.to_string()))?;
        }
        Ok(())
    }

    /// Register a device directly (pre-authorize without pending queue).
    /// Used by the auth layer to register verified devices so they can
    /// connect and join gossip/federation.
    pub fn register_device(
        &self,
        peer_id: &str,
        public_key_bytes: &[u8],
        user_id: &str,
        device_name: &str,
    ) -> Result<PairingRecord, TransportError> {
        let now = Utc::now();
        let record = PairingRecord {
            peer_id: peer_id.to_string(),
            public_key_bytes: public_key_bytes.to_vec(),
            user_id: user_id.to_string(),
            device_name: device_name.to_string(),
            paired_at: now,
            last_verified_at: now,
        };

        let value = rmp_serde::to_vec(&record)
            .map_err(|e| TransportError::Pairing(e.to_string()))?;
        self.tree
            .insert(record.peer_id.as_bytes(), &value)
            .map_err(|e| TransportError::Pairing(e.to_string()))?;

        Ok(record)
    }

    /// Verify that a reconnecting peer's public_key_bytes and user_id
    /// match the stored pairing record.
    pub fn verify_reconnect(
        &self,
        peer_id: &str,
        public_key_bytes: &[u8],
        user_id: &str,
    ) -> Result<bool, TransportError> {
        match self.get_paired(peer_id) {
            Some(record) => {
                if record.public_key_bytes != public_key_bytes {
                    return Err(TransportError::PairingVerificationFailed(
                        format!("public key mismatch for peer {}", peer_id),
                    ));
                }
                if !user_id.is_empty() && !record.user_id.is_empty() && record.user_id != user_id {
                    return Err(TransportError::PairingVerificationFailed(
                        format!("user_id mismatch for peer {}", peer_id),
                    ));
                }
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn open_store() -> (TempDir, PairingStore) {
        let dir = TempDir::new().unwrap();
        let storage = Arc::new(StorageEngine::open(&dir.path().join("db")).unwrap());
        let store = PairingStore::new(storage).unwrap();
        (dir, store)
    }

    fn make_pending(peer_id: &str, user_id: &str) -> PendingPairingRequest {
        PendingPairingRequest {
            peer_id: peer_id.to_string(),
            public_key_bytes: vec![1, 2, 3, 4],
            user_id: user_id.to_string(),
            device_name: "Test Device".to_string(),
            endpoint: "wss://127.0.0.1:9400".to_string(),
            received_at: Utc::now(),
        }
    }

    #[test]
    fn pairing_store_open_empty() {
        let (_dir, store) = open_store();
        assert!(store.list_paired().is_empty());
        assert!(store.list_pending().is_empty());
        assert!(!store.is_paired("nonexistent"));
    }

    #[test]
    fn add_pending_and_approve() {
        let (_dir, store) = open_store();
        store.add_pending(make_pending("peer-abc", "user-123"));
        assert_eq!(store.list_pending().len(), 1);
        assert!(!store.is_paired("peer-abc"));

        let record = store.approve("peer-abc").unwrap().unwrap();
        assert_eq!(record.peer_id, "peer-abc");
        assert_eq!(record.user_id, "user-123");
        assert!(store.is_paired("peer-abc"));
        assert!(store.list_pending().is_empty());
    }

    #[test]
    fn add_pending_and_reject() {
        let (_dir, store) = open_store();
        store.add_pending(make_pending("peer-xyz", "user-456"));
        assert_eq!(store.list_pending().len(), 1);

        assert!(store.reject("peer-xyz"));
        assert!(store.list_pending().is_empty());
        assert!(!store.is_paired("peer-xyz"));
    }

    #[test]
    fn list_paired() {
        let (_dir, store) = open_store();
        store.add_pending(make_pending("peer-a", "user-1"));
        store.add_pending(make_pending("peer-b", "user-2"));
        store.approve("peer-a").unwrap();
        store.approve("peer-b").unwrap();

        let paired = store.list_paired();
        assert_eq!(paired.len(), 2);
        let ids: Vec<&str> = paired.iter().map(|r| r.peer_id.as_str()).collect();
        assert!(ids.contains(&"peer-a"));
        assert!(ids.contains(&"peer-b"));
    }

    #[test]
    fn remove_paired() {
        let (_dir, store) = open_store();
        store.add_pending(make_pending("peer-rm", "user-rm"));
        store.approve("peer-rm").unwrap();
        assert!(store.is_paired("peer-rm"));

        assert!(store.remove_paired("peer-rm").unwrap());
        assert!(!store.is_paired("peer-rm"));
        assert!(!store.remove_paired("peer-rm").unwrap());
    }

    #[test]
    fn verify_reconnect_success() {
        let (_dir, store) = open_store();
        store.add_pending(make_pending("peer-v", "user-v"));
        store.approve("peer-v").unwrap();

        let ok = store
            .verify_reconnect("peer-v", &[1, 2, 3, 4], "user-v")
            .unwrap();
        assert!(ok);
    }

    #[test]
    fn verify_reconnect_wrong_user_id() {
        let (_dir, store) = open_store();
        store.add_pending(make_pending("peer-u", "user-correct"));
        store.approve("peer-u").unwrap();

        let result = store.verify_reconnect("peer-u", &[1, 2, 3, 4], "user-wrong");
        assert!(result.is_err());
    }

    #[test]
    fn verify_reconnect_wrong_key() {
        let (_dir, store) = open_store();
        store.add_pending(make_pending("peer-k", "user-k"));
        store.approve("peer-k").unwrap();

        let result = store.verify_reconnect("peer-k", &[9, 9, 9, 9], "user-k");
        assert!(result.is_err());
    }

    #[test]
    fn touch_verified_updates_timestamp() {
        let (_dir, store) = open_store();
        store.add_pending(make_pending("peer-t", "user-t"));
        store.approve("peer-t").unwrap();

        let before = store.get_paired("peer-t").unwrap().last_verified_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        store.touch_verified("peer-t").unwrap();
        let after = store.get_paired("peer-t").unwrap().last_verified_at;
        assert!(after > before);
    }

    #[test]
    fn persistence_across_reopen() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("db");

        // First open: add and approve a pairing
        {
            let storage = Arc::new(StorageEngine::open(&db_path).unwrap());
            let store = PairingStore::new(storage).unwrap();
            store.add_pending(make_pending("peer-persist", "user-persist"));
            store.approve("peer-persist").unwrap();
        }

        // Second open: verify pairing survives
        {
            let storage = Arc::new(StorageEngine::open(&db_path).unwrap());
            let store = PairingStore::new(storage).unwrap();
            assert!(store.is_paired("peer-persist"));
            let record = store.get_paired("peer-persist").unwrap();
            assert_eq!(record.user_id, "user-persist");
            // Pending is in-memory only, so it should be empty
            assert!(store.list_pending().is_empty());
        }
    }
}

use std::sync::Arc;

use nodedb_storage::{IdGenerator, StorageEngine};

use crate::error::TransportError;
use crate::types::NodeShareAuditEntry;

/// Append-only audit log for outbound shares.
/// Backed by a sled tree, entries are never updated or deleted.
pub struct AuditLog {
    tree: nodedb_storage::StorageTree,
    id_gen: Arc<IdGenerator>,
}

impl AuditLog {
    pub fn new(engine: Arc<StorageEngine>) -> Result<Self, TransportError> {
        let tree = engine
            .open_tree("__audit_log__")
            .map_err(|e| TransportError::Audit(e.to_string()))?;
        let id_gen = Arc::new(
            IdGenerator::new(&engine)
                .map_err(|e| TransportError::Audit(e.to_string()))?,
        );
        Ok(AuditLog { tree, id_gen })
    }

    /// Append a new audit entry. The id field is auto-assigned.
    pub fn append(&self, mut entry: NodeShareAuditEntry) -> Result<NodeShareAuditEntry, TransportError> {
        let id = self
            .id_gen
            .next_id("audit")
            .map_err(|e| TransportError::Audit(e.to_string()))?;
        entry.id = id;

        let key = nodedb_storage::encode_id(id);
        let value = rmp_serde::to_vec(&entry)
            .map_err(|e| TransportError::Serialization(e.to_string()))?;
        self.tree
            .insert(&key, &value)
            .map_err(|e| TransportError::Audit(e.to_string()))?;

        Ok(entry)
    }

    /// Get recent audit entries with offset and limit.
    pub fn recent(&self, offset: usize, limit: usize) -> Result<Vec<NodeShareAuditEntry>, TransportError> {
        let entries: Vec<NodeShareAuditEntry> = self
            .tree
            .iter()
            .filter_map(|r| r.ok())
            .filter_map(|(_, v)| rmp_serde::from_slice(&v).ok())
            .skip(offset)
            .take(limit)
            .collect();
        Ok(entries)
    }

    /// Get total audit entry count.
    pub fn count(&self) -> usize {
        self.tree.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    fn open_audit() -> (TempDir, AuditLog) {
        let dir = TempDir::new().unwrap();
        let storage = Arc::new(StorageEngine::open(&dir.path().join("db")).unwrap());
        let audit = AuditLog::new(storage).unwrap();
        (dir, audit)
    }

    #[test]
    fn append_and_read() {
        let (_dir, audit) = open_audit();
        let entry = NodeShareAuditEntry {
            id: 0,
            timestamp: Utc::now(),
            peer_id: "peer1".to_string(),
            action: "query_response".to_string(),
            collection: Some("users".to_string()),
            record_count: 5,
            content_hash: "abc123".to_string(),
        };
        let stored = audit.append(entry).unwrap();
        assert_eq!(stored.id, 1);

        let entries = audit.recent(0, 10).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].peer_id, "peer1");
        assert_eq!(entries[0].record_count, 5);
    }

    #[test]
    fn auto_increment_ids() {
        let (_dir, audit) = open_audit();
        for i in 0..3 {
            let entry = NodeShareAuditEntry {
                id: 0,
                timestamp: Utc::now(),
                peer_id: format!("peer{}", i),
                action: "share".to_string(),
                collection: None,
                record_count: i as u64,
                content_hash: "hash".to_string(),
            };
            let stored = audit.append(entry).unwrap();
            assert_eq!(stored.id, (i + 1) as i64);
        }
        assert_eq!(audit.count(), 3);
    }

    #[test]
    fn offset_and_limit() {
        let (_dir, audit) = open_audit();
        for i in 0..5 {
            let entry = NodeShareAuditEntry {
                id: 0,
                timestamp: Utc::now(),
                peer_id: format!("peer{}", i),
                action: "share".to_string(),
                collection: None,
                record_count: 0,
                content_hash: "hash".to_string(),
            };
            audit.append(entry).unwrap();
        }

        let page = audit.recent(2, 2).unwrap();
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].peer_id, "peer2");
        assert_eq!(page[1].peer_id, "peer3");
    }
}

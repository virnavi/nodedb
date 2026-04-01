use std::sync::Arc;

use chrono::{DateTime, Utc};
use rmpv::Value;
use serde::{Deserialize, Serialize};

use crate::collection::Collection;
use crate::error::NoSqlError;

/// Type of access event recorded in the access history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessEventType {
    Read,
    Write,
    Watch,
    FederatedRead,
    AiWrite,
}

impl AccessEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccessEventType::Read => "read",
            AccessEventType::Write => "write",
            AccessEventType::Watch => "watch",
            AccessEventType::FederatedRead => "federated_read",
            AccessEventType::AiWrite => "ai_write",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "read" => Some(AccessEventType::Read),
            "write" => Some(AccessEventType::Write),
            "watch" => Some(AccessEventType::Watch),
            "federated_read" => Some(AccessEventType::FederatedRead),
            "ai_write" => Some(AccessEventType::AiWrite),
            _ => None,
        }
    }
}

/// Scope under which the access occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryScope {
    Local,
    Federated,
    AiQuery,
    FederatedAndAi,
}

impl QueryScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            QueryScope::Local => "local",
            QueryScope::Federated => "federated",
            QueryScope::AiQuery => "ai_query",
            QueryScope::FederatedAndAi => "federated_and_ai",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "local" => Some(QueryScope::Local),
            "federated" => Some(QueryScope::Federated),
            "ai_query" => Some(QueryScope::AiQuery),
            "federated_and_ai" => Some(QueryScope::FederatedAndAi),
            _ => None,
        }
    }
}

/// Configuration for access history behaviour.
#[derive(Debug, Clone)]
pub struct AccessHistoryConfig {
    /// How long to retain access history entries. Default: 365 days.
    pub retention_period_secs: i64,
    /// Whether to automatically trim old entries. Default: true.
    pub auto_trim: bool,
    /// Interval between auto-trim runs. Default: 24 hours.
    pub auto_trim_interval_secs: i64,
    /// Collections to exclude from access history recording.
    pub exclude_collections: Vec<String>,
    /// Whether to track watch events. Default: false.
    pub track_watch_events: bool,
}

impl Default for AccessHistoryConfig {
    fn default() -> Self {
        AccessHistoryConfig {
            retention_period_secs: 365 * 24 * 3600, // 365 days
            auto_trim: true,
            auto_trim_interval_secs: 24 * 3600, // 24 hours
            exclude_collections: vec![
                "__access_history__".to_string(),
            ],
            track_watch_events: false,
        }
    }
}

/// Append-only store for access history entries.
///
/// Each entry is stored as a Document in the `__access_history__` collection.
pub struct AccessHistoryStore {
    collection: Arc<Collection>,
}

impl AccessHistoryStore {
    pub fn new(collection: Arc<Collection>) -> Self {
        AccessHistoryStore { collection }
    }

    /// Record an access event.
    pub fn record(
        &self,
        collection: &str,
        record_id: i64,
        event_type: AccessEventType,
        accessor_id: &str,
        query_scope: QueryScope,
        federation_hop_count: Option<i32>,
        cache_hit: bool,
    ) -> Result<(), NoSqlError> {
        let now = Utc::now();
        let mut fields = vec![
            (Value::String("collection".into()), Value::String(collection.into())),
            (Value::String("record_id".into()), Value::Integer(record_id.into())),
            (Value::String("event_type".into()), Value::String(event_type.as_str().into())),
            (Value::String("accessed_at_utc".into()), Value::String(now.to_rfc3339().into())),
            (Value::String("accessor_id".into()), Value::String(accessor_id.into())),
            (Value::String("query_scope".into()), Value::String(query_scope.as_str().into())),
            (Value::String("cache_hit".into()), Value::Boolean(cache_hit)),
        ];

        if let Some(hop) = federation_hop_count {
            fields.push((Value::String("federation_hop_count".into()), Value::Integer(hop.into())));
        }

        self.collection.put(Value::Map(fields))?;
        Ok(())
    }

    /// Record access events for multiple records at once (batch).
    pub fn record_batch(
        &self,
        collection: &str,
        record_ids: &[i64],
        event_type: AccessEventType,
        accessor_id: &str,
        query_scope: QueryScope,
    ) -> Result<(), NoSqlError> {
        for &id in record_ids {
            self.record(collection, id, event_type, accessor_id, query_scope, None, false)?;
        }
        Ok(())
    }

    /// Query access history entries for a specific collection and record.
    pub fn query_history(
        &self,
        collection: Option<&str>,
        record_id: Option<i64>,
        event_type: Option<AccessEventType>,
        since: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Result<Vec<Value>, NoSqlError> {
        let all = self.collection.find_all(None, None)?;
        let mut results: Vec<Value> = Vec::new();

        for doc in all {
            let data = &doc.data;

            // Filter by collection
            if let Some(col) = collection {
                let doc_col = data_field_str(data, "collection").unwrap_or_default();
                if doc_col != col {
                    continue;
                }
            }

            // Filter by record_id
            if let Some(rid) = record_id {
                let doc_rid = data_field_i64(data, "record_id").unwrap_or(-1);
                if doc_rid != rid {
                    continue;
                }
            }

            // Filter by event_type
            if let Some(ref et) = event_type {
                let doc_et = data_field_str(data, "event_type").unwrap_or_default();
                if doc_et != et.as_str() {
                    continue;
                }
            }

            // Filter by since
            if let Some(ref since_dt) = since {
                let doc_ts = data_field_str(data, "accessed_at_utc").unwrap_or_default();
                if let Ok(ts) = DateTime::parse_from_rfc3339(&doc_ts) {
                    if ts.with_timezone(&Utc) < *since_dt {
                        continue;
                    }
                }
            }

            // Build result entry including the doc ID
            let mut entry_fields = vec![
                (Value::String("id".into()), Value::Integer(doc.id.into())),
            ];
            if let Value::Map(ref fields) = doc.data {
                for (k, v) in fields {
                    entry_fields.push((k.clone(), v.clone()));
                }
            }
            results.push(Value::Map(entry_fields));

            if let Some(lim) = limit {
                if results.len() >= lim {
                    break;
                }
            }
        }

        Ok(results)
    }

    /// Get the most recent access time for a specific record.
    pub fn last_access_time(
        &self,
        collection: &str,
        record_id: i64,
    ) -> Result<Option<DateTime<Utc>>, NoSqlError> {
        let all = self.collection.find_all(None, None)?;
        let mut latest: Option<DateTime<Utc>> = None;

        for doc in all {
            let doc_col = data_field_str(&doc.data, "collection").unwrap_or_default();
            let doc_rid = data_field_i64(&doc.data, "record_id").unwrap_or(-1);

            if doc_col == collection && doc_rid == record_id {
                let doc_ts = data_field_str(&doc.data, "accessed_at_utc").unwrap_or_default();
                if let Ok(ts) = DateTime::parse_from_rfc3339(&doc_ts) {
                    let utc_ts = ts.with_timezone(&Utc);
                    match latest {
                        None => latest = Some(utc_ts),
                        Some(prev) if utc_ts > prev => latest = Some(utc_ts),
                        _ => {}
                    }
                }
            }
        }

        Ok(latest)
    }

    /// Trim access history entries older than the given retention period (in seconds).
    pub fn trim_old_entries(&self, retention_period_secs: i64) -> Result<usize, NoSqlError> {
        let cutoff = Utc::now() - chrono::Duration::seconds(retention_period_secs);
        let all = self.collection.find_all(None, None)?;
        let mut deleted = 0;

        for doc in all {
            let doc_ts = data_field_str(&doc.data, "accessed_at_utc").unwrap_or_default();
            if let Ok(ts) = DateTime::parse_from_rfc3339(&doc_ts) {
                if ts.with_timezone(&Utc) < cutoff {
                    self.collection.delete(doc.id)?;
                    deleted += 1;
                }
            }
        }

        Ok(deleted)
    }

    /// Count total access history entries.
    pub fn count(&self) -> Result<usize, NoSqlError> {
        Ok(self.collection.count())
    }

    /// Get a reference to the underlying collection.
    pub fn collection(&self) -> &Arc<Collection> {
        &self.collection
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Extract a string field from a Value::Map slice (for use in trim evaluation).
pub fn data_field_str_from_map<'a>(map: &'a [(Value, Value)], key: &str) -> Option<&'a str> {
    for (k, v) in map {
        if let Value::String(s) = k {
            if s.as_str() == Some(key) {
                if let Value::String(vs) = v {
                    return vs.as_str();
                }
            }
        }
    }
    None
}

fn data_field_str<'a>(data: &'a Value, key: &str) -> Option<&'a str> {
    match data {
        Value::Map(fields) => {
            for (k, v) in fields {
                if let Value::String(s) = k {
                    if s.as_str() == Some(key) {
                        if let Value::String(vs) = v {
                            return vs.as_str();
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn data_field_i64(data: &Value, key: &str) -> Option<i64> {
    match data {
        Value::Map(fields) => {
            for (k, v) in fields {
                if let Value::String(s) = k {
                    if s.as_str() == Some(key) {
                        if let Value::Integer(i) = v {
                            return i.as_i64();
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use tempfile::TempDir;

    #[test]
    fn test_access_event_type_roundtrip() {
        let types = vec![
            AccessEventType::Read,
            AccessEventType::Write,
            AccessEventType::Watch,
            AccessEventType::FederatedRead,
            AccessEventType::AiWrite,
        ];
        for t in types {
            assert_eq!(AccessEventType::from_str(t.as_str()), Some(t));
        }
        assert_eq!(AccessEventType::from_str("invalid"), None);
    }

    #[test]
    fn test_query_scope_roundtrip() {
        let scopes = vec![
            QueryScope::Local,
            QueryScope::Federated,
            QueryScope::AiQuery,
            QueryScope::FederatedAndAi,
        ];
        for s in scopes {
            assert_eq!(QueryScope::from_str(s.as_str()), Some(s));
        }
        assert_eq!(QueryScope::from_str("invalid"), None);
    }

    #[test]
    fn test_record_and_query() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let col = db.collection("__access_history__").unwrap();
        let store = AccessHistoryStore::new(col);

        store.record("users", 1, AccessEventType::Read, "local", QueryScope::Local, None, false).unwrap();
        store.record("users", 2, AccessEventType::Write, "local", QueryScope::Local, None, false).unwrap();
        store.record("posts", 1, AccessEventType::Read, "peer:alice", QueryScope::Federated, Some(1), false).unwrap();

        // Query all
        let all = store.query_history(None, None, None, None, None).unwrap();
        assert_eq!(all.len(), 3);

        // Query by collection
        let users = store.query_history(Some("users"), None, None, None, None).unwrap();
        assert_eq!(users.len(), 2);

        // Query by record_id
        let r1 = store.query_history(Some("users"), Some(1), None, None, None).unwrap();
        assert_eq!(r1.len(), 1);

        // Query by event_type
        let writes = store.query_history(None, None, Some(AccessEventType::Write), None, None).unwrap();
        assert_eq!(writes.len(), 1);
    }

    #[test]
    fn test_record_batch() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let col = db.collection("__access_history__").unwrap();
        let store = AccessHistoryStore::new(col);

        store.record_batch("users", &[1, 2, 3], AccessEventType::Read, "local", QueryScope::Local).unwrap();
        assert_eq!(store.count().unwrap(), 3);
    }

    #[test]
    fn test_last_access_time() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let col = db.collection("__access_history__").unwrap();
        let store = AccessHistoryStore::new(col);

        // No history yet
        assert!(store.last_access_time("users", 1).unwrap().is_none());

        store.record("users", 1, AccessEventType::Read, "local", QueryScope::Local, None, false).unwrap();
        let t1 = store.last_access_time("users", 1).unwrap();
        assert!(t1.is_some());

        store.record("users", 1, AccessEventType::Write, "local", QueryScope::Local, None, false).unwrap();
        let t2 = store.last_access_time("users", 1).unwrap();
        assert!(t2.unwrap() >= t1.unwrap());
    }

    #[test]
    fn test_trim_old_entries() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let col = db.collection("__access_history__").unwrap();
        let store = AccessHistoryStore::new(col);

        store.record("users", 1, AccessEventType::Read, "local", QueryScope::Local, None, false).unwrap();
        store.record("users", 2, AccessEventType::Write, "local", QueryScope::Local, None, false).unwrap();

        // Trim with 0 retention (everything is "old")
        let deleted = store.trim_old_entries(0).unwrap();
        assert_eq!(deleted, 2);
        assert_eq!(store.count().unwrap(), 0);
    }

    #[test]
    fn test_trim_old_entries_retains_recent() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let col = db.collection("__access_history__").unwrap();
        let store = AccessHistoryStore::new(col);

        store.record("users", 1, AccessEventType::Read, "local", QueryScope::Local, None, false).unwrap();

        // Trim with large retention (nothing is old)
        let deleted = store.trim_old_entries(365 * 24 * 3600).unwrap();
        assert_eq!(deleted, 0);
        assert_eq!(store.count().unwrap(), 1);
    }

    #[test]
    fn test_config_defaults() {
        let config = AccessHistoryConfig::default();
        assert_eq!(config.retention_period_secs, 365 * 24 * 3600);
        assert!(config.auto_trim);
        assert_eq!(config.auto_trim_interval_secs, 24 * 3600);
        assert!(!config.track_watch_events);
    }

    #[test]
    fn test_query_with_limit() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let col = db.collection("__access_history__").unwrap();
        let store = AccessHistoryStore::new(col);

        for i in 1..=10 {
            store.record("users", i, AccessEventType::Read, "local", QueryScope::Local, None, false).unwrap();
        }

        let limited = store.query_history(None, None, None, None, Some(3)).unwrap();
        assert_eq!(limited.len(), 3);
    }
}

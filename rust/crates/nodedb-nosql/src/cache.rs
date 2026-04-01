use chrono::{DateTime, Utc};
use rmpv::Value;
use serde::{Deserialize, Serialize};

use crate::document::Document;
use crate::error::NoSqlError;
use nodedb_storage::StorageTree;

/// Cache expiry mode for a record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CacheMode {
    /// TTL is measured from the document's `updated_at` timestamp.
    ExpireAfterWrite,
    /// TTL is measured from the document's `created_at` timestamp.
    ExpireAfterCreate,
}

impl CacheMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            CacheMode::ExpireAfterWrite => "expire_after_write",
            CacheMode::ExpireAfterCreate => "expire_after_create",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, NoSqlError> {
        match s {
            "expire_after_write" => Ok(CacheMode::ExpireAfterWrite),
            "expire_after_create" => Ok(CacheMode::ExpireAfterCreate),
            _ => Err(NoSqlError::CacheConfigInvalid(format!(
                "unknown cache mode: {}",
                s
            ))),
        }
    }
}

/// Per-record cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub mode: CacheMode,
    pub ttl_secs: i64,
}

impl CacheConfig {
    /// Parse from a MessagePack Value map: `{mode: "expire_after_write", ttl_secs: 3600}`.
    pub fn from_value(val: &Value) -> Result<Self, NoSqlError> {
        let map = val.as_map().ok_or_else(|| {
            NoSqlError::CacheConfigInvalid("expected map".to_string())
        })?;

        let mode_str = map_field_str(map, "mode").ok_or_else(|| {
            NoSqlError::CacheConfigInvalid("missing 'mode' field".to_string())
        })?;
        let mode = CacheMode::from_str(mode_str)?;

        let ttl_secs = map_field_i64(map, "ttl_secs").ok_or_else(|| {
            NoSqlError::CacheConfigInvalid("missing 'ttl_secs' field".to_string())
        })?;

        if ttl_secs <= 0 {
            return Err(NoSqlError::CacheConfigInvalid(
                "ttl_secs must be positive".to_string(),
            ));
        }

        Ok(CacheConfig { mode, ttl_secs })
    }

    /// Serialize to a MessagePack Value map.
    pub fn to_value(&self) -> Value {
        Value::Map(vec![
            (
                Value::String("mode".into()),
                Value::String(self.mode.as_str().into()),
            ),
            (
                Value::String("ttl_secs".into()),
                Value::Integer(self.ttl_secs.into()),
            ),
        ])
    }

    /// Check whether a document has expired according to this cache config.
    pub fn is_expired(&self, doc: &Document, now: DateTime<Utc>) -> bool {
        let reference_time = match self.mode {
            CacheMode::ExpireAfterWrite => doc.updated_at,
            CacheMode::ExpireAfterCreate => doc.created_at,
        };
        let elapsed = now.signed_duration_since(reference_time).num_seconds();
        elapsed >= self.ttl_secs
    }
}

/// Manages per-record cache configurations in a dedicated sled tree.
///
/// Key format: `"{meta_key}\0{record_id}"` (same pattern as TrimConfig record overrides).
/// Value: msgpack-serialized `CacheConfig`.
pub struct RecordCacheStore {
    tree: StorageTree,
}

impl RecordCacheStore {
    pub fn new(tree: StorageTree) -> Self {
        RecordCacheStore { tree }
    }

    /// Set cache configuration for a specific record.
    pub fn set(
        &self,
        meta_key: &str,
        record_id: i64,
        config: &CacheConfig,
    ) -> Result<(), NoSqlError> {
        let key = cache_key(meta_key, record_id);
        let bytes =
            rmp_serde::to_vec(config).map_err(|e| NoSqlError::Serialization(e.to_string()))?;
        self.tree.insert(key.as_bytes(), &bytes)?;
        Ok(())
    }

    /// Get cache configuration for a specific record.
    pub fn get(
        &self,
        meta_key: &str,
        record_id: i64,
    ) -> Result<Option<CacheConfig>, NoSqlError> {
        let key = cache_key(meta_key, record_id);
        match self.tree.get(key.as_bytes())? {
            Some(bytes) => {
                let config: CacheConfig = rmp_serde::from_slice(&bytes)
                    .map_err(|e| NoSqlError::Serialization(e.to_string()))?;
                Ok(Some(config))
            }
            None => Ok(None),
        }
    }

    /// Remove cache configuration for a specific record.
    pub fn remove(&self, meta_key: &str, record_id: i64) -> Result<(), NoSqlError> {
        let key = cache_key(meta_key, record_id);
        self.tree.remove(key.as_bytes())?;
        Ok(())
    }

    /// Get all cache configs for a collection by scanning the prefix.
    pub fn all_for_collection(
        &self,
        meta_key: &str,
    ) -> Result<Vec<(i64, CacheConfig)>, NoSqlError> {
        let prefix = format!("{}\0", meta_key);
        let mut results = Vec::new();

        for item in self.tree.scan_prefix(prefix.as_bytes()) {
            let (key_bytes, val_bytes) = item?;
            let key_str = std::str::from_utf8(&key_bytes)
                .map_err(|e| NoSqlError::Serialization(e.to_string()))?;

            // Extract record_id from key after the null separator
            if let Some(id_str) = key_str.split('\0').nth(1) {
                if let Ok(record_id) = id_str.parse::<i64>() {
                    let config: CacheConfig = rmp_serde::from_slice(&val_bytes)
                        .map_err(|e| NoSqlError::Serialization(e.to_string()))?;
                    results.push((record_id, config));
                }
            }
        }

        Ok(results)
    }

    /// Sweep expired records in a specific collection.
    ///
    /// Looks up each cached record, checks if it's expired, and deletes both
    /// the record and its cache config entry. Returns the count of deleted records.
    ///
    /// `get_doc` should return the Document for a given record_id, or None if not found.
    /// `delete_doc` should delete the record by id.
    pub fn sweep<F, D>(
        &self,
        meta_key: &str,
        now: DateTime<Utc>,
        get_doc: F,
        delete_doc: D,
    ) -> Result<usize, NoSqlError>
    where
        F: Fn(i64) -> Result<Option<Document>, NoSqlError>,
        D: Fn(i64) -> Result<(), NoSqlError>,
    {
        let entries = self.all_for_collection(meta_key)?;
        let mut deleted = 0;

        for (record_id, config) in entries {
            match get_doc(record_id)? {
                Some(doc) => {
                    if config.is_expired(&doc, now) {
                        delete_doc(record_id)?;
                        self.remove(meta_key, record_id)?;
                        deleted += 1;
                    }
                }
                None => {
                    // Record already gone, clean up stale cache entry
                    self.remove(meta_key, record_id)?;
                }
            }
        }

        Ok(deleted)
    }
}

fn cache_key(meta_key: &str, record_id: i64) -> String {
    format!("{}\0{}", meta_key, record_id)
}

// ── Value helpers ───────────────────────────────────────────────────────

fn map_field_str<'a>(map: &'a [(Value, Value)], key: &str) -> Option<&'a str> {
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

fn map_field_i64(map: &[(Value, Value)], key: &str) -> Option<i64> {
    for (k, v) in map {
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use nodedb_storage::StorageEngine;
    use tempfile::TempDir;

    fn open_cache_store(dir: &TempDir) -> RecordCacheStore {
        let engine = StorageEngine::open(dir.path()).unwrap();
        let tree = engine.open_tree("__record_cache_config__").unwrap();
        RecordCacheStore::new(tree)
    }

    fn make_doc(id: i64, created_at: DateTime<Utc>, updated_at: DateTime<Utc>) -> Document {
        Document {
            id,
            collection: "test".to_string(),
            data: rmpv::Value::Map(vec![]),
            created_at,
            updated_at,
        }
    }

    #[test]
    fn test_cache_mode_roundtrip() {
        assert_eq!(
            CacheMode::from_str(CacheMode::ExpireAfterWrite.as_str()).unwrap(),
            CacheMode::ExpireAfterWrite
        );
        assert_eq!(
            CacheMode::from_str(CacheMode::ExpireAfterCreate.as_str()).unwrap(),
            CacheMode::ExpireAfterCreate
        );
    }

    #[test]
    fn test_cache_mode_invalid() {
        assert!(CacheMode::from_str("bogus").is_err());
    }

    #[test]
    fn test_cache_config_from_value() {
        let val = Value::Map(vec![
            (
                Value::String("mode".into()),
                Value::String("expire_after_write".into()),
            ),
            (Value::String("ttl_secs".into()), Value::Integer(3600.into())),
        ]);
        let config = CacheConfig::from_value(&val).unwrap();
        assert_eq!(config.mode, CacheMode::ExpireAfterWrite);
        assert_eq!(config.ttl_secs, 3600);
    }

    #[test]
    fn test_cache_config_value_roundtrip() {
        let config = CacheConfig {
            mode: CacheMode::ExpireAfterCreate,
            ttl_secs: 7200,
        };
        let val = config.to_value();
        let parsed = CacheConfig::from_value(&val).unwrap();
        assert_eq!(parsed.mode, CacheMode::ExpireAfterCreate);
        assert_eq!(parsed.ttl_secs, 7200);
    }

    #[test]
    fn test_cache_config_invalid_ttl() {
        let val = Value::Map(vec![
            (
                Value::String("mode".into()),
                Value::String("expire_after_write".into()),
            ),
            (Value::String("ttl_secs".into()), Value::Integer(0.into())),
        ]);
        assert!(CacheConfig::from_value(&val).is_err());
    }

    #[test]
    fn test_cache_config_missing_fields() {
        let val = Value::Map(vec![]);
        assert!(CacheConfig::from_value(&val).is_err());
    }

    #[test]
    fn test_is_expired_after_write() {
        let now = Utc::now();
        let config = CacheConfig {
            mode: CacheMode::ExpireAfterWrite,
            ttl_secs: 60,
        };

        // Created 120s ago, updated 120s ago → expired
        let doc = make_doc(
            1,
            now - Duration::seconds(120),
            now - Duration::seconds(120),
        );
        assert!(config.is_expired(&doc, now));

        // Updated just now → not expired
        let doc = make_doc(1, now - Duration::seconds(120), now);
        assert!(!config.is_expired(&doc, now));
    }

    #[test]
    fn test_is_expired_after_create() {
        let now = Utc::now();
        let config = CacheConfig {
            mode: CacheMode::ExpireAfterCreate,
            ttl_secs: 60,
        };

        // Created 120s ago → expired regardless of updated_at
        let doc = make_doc(1, now - Duration::seconds(120), now);
        assert!(config.is_expired(&doc, now));

        // Created just now → not expired
        let doc = make_doc(1, now, now);
        assert!(!config.is_expired(&doc, now));
    }

    #[test]
    fn test_store_set_get_remove() {
        let dir = TempDir::new().unwrap();
        let store = open_cache_store(&dir);

        let config = CacheConfig {
            mode: CacheMode::ExpireAfterWrite,
            ttl_secs: 300,
        };

        // Initially empty
        assert!(store.get("public::users", 1).unwrap().is_none());

        // Set and get
        store.set("public::users", 1, &config).unwrap();
        let fetched = store.get("public::users", 1).unwrap().unwrap();
        assert_eq!(fetched.mode, CacheMode::ExpireAfterWrite);
        assert_eq!(fetched.ttl_secs, 300);

        // Remove
        store.remove("public::users", 1).unwrap();
        assert!(store.get("public::users", 1).unwrap().is_none());
    }

    #[test]
    fn test_store_all_for_collection() {
        let dir = TempDir::new().unwrap();
        let store = open_cache_store(&dir);

        let config = CacheConfig {
            mode: CacheMode::ExpireAfterWrite,
            ttl_secs: 60,
        };
        store.set("public::users", 1, &config).unwrap();
        store.set("public::users", 2, &config).unwrap();
        store.set("public::orders", 1, &config).unwrap();

        let entries = store.all_for_collection("public::users").unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_store_sweep() {
        let dir = TempDir::new().unwrap();
        let store = open_cache_store(&dir);
        let now = Utc::now();

        let config = CacheConfig {
            mode: CacheMode::ExpireAfterWrite,
            ttl_secs: 60,
        };
        store.set("public::users", 1, &config).unwrap();
        store.set("public::users", 2, &config).unwrap();

        // Doc 1 expired, doc 2 not expired
        let deleted = store
            .sweep(
                "public::users",
                now,
                |id| {
                    Ok(Some(make_doc(
                        id,
                        now - Duration::seconds(120),
                        if id == 1 {
                            now - Duration::seconds(120)
                        } else {
                            now
                        },
                    )))
                },
                |_id| Ok(()),
            )
            .unwrap();

        assert_eq!(deleted, 1);
        // Doc 1 cache entry removed
        assert!(store.get("public::users", 1).unwrap().is_none());
        // Doc 2 cache entry still there
        assert!(store.get("public::users", 2).unwrap().is_some());
    }

    #[test]
    fn test_store_sweep_stale_entry() {
        let dir = TempDir::new().unwrap();
        let store = open_cache_store(&dir);
        let now = Utc::now();

        let config = CacheConfig {
            mode: CacheMode::ExpireAfterWrite,
            ttl_secs: 60,
        };
        store.set("public::users", 99, &config).unwrap();

        // Doc doesn't exist → stale cache entry cleaned up
        let deleted = store
            .sweep("public::users", now, |_id| Ok(None), |_id| Ok(()))
            .unwrap();

        assert_eq!(deleted, 0); // Not counted as a "deleted record"
        assert!(store.get("public::users", 99).unwrap().is_none());
    }
}

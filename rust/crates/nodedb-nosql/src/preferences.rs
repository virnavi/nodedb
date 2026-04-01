use std::sync::Arc;

use rmpv::Value;

use crate::collection::Collection;
use crate::conflict::{resolve_conflict, ConflictContext, ConflictOutcome, ConflictResolution};
use crate::document::Document;
use crate::error::NoSqlError;
use nodedb_storage::StorageTree;

/// A per-key encrypted key-value store backed by a Collection.
///
/// Each preference is stored as a Document with encrypted value.
/// An index tree maps preference keys to document IDs for fast lookup.
pub struct PreferencesStore {
    collection: Arc<Collection>,
    index_tree: StorageTree,
    dek: Option<[u8; 32]>,
}

impl PreferencesStore {
    /// Create a new PreferencesStore backed by the given collection and index tree.
    pub fn new(
        collection: Arc<Collection>,
        index_tree: StorageTree,
        dek: Option<[u8; 32]>,
    ) -> Self {
        PreferencesStore {
            collection,
            index_tree,
            dek,
        }
    }

    /// Encrypt a value using a per-key HKDF-derived key.
    /// If no DEK, returns the value as-is serialized.
    fn encrypt_value(&self, key: &str, value: &Value) -> Result<Vec<u8>, NoSqlError> {
        let plain_bytes = rmp_serde::to_vec(value)?;
        match self.dek {
            Some(ref master) => {
                let derived = nodedb_crypto::hkdf_derive_key(master, &format!("prefs:{}", key))
                    .map_err(|e| NoSqlError::PreferenceError(e.to_string()))?;
                let (nonce, ct) = nodedb_crypto::encryption::aes_256_gcm_encrypt(&derived, &plain_bytes)
                    .map_err(|e| NoSqlError::PreferenceError(e.to_string()))?;
                // Pack: [nonce_len(1) | nonce | ciphertext]
                let mut packed = Vec::with_capacity(1 + nonce.len() + ct.len());
                packed.push(nonce.len() as u8);
                packed.extend_from_slice(&nonce);
                packed.extend_from_slice(&ct);
                Ok(packed)
            }
            None => Ok(plain_bytes),
        }
    }

    /// Decrypt a value using a per-key HKDF-derived key.
    fn decrypt_value(&self, key: &str, encrypted: &[u8]) -> Result<Value, NoSqlError> {
        match self.dek {
            Some(ref master) => {
                if encrypted.is_empty() {
                    return Err(NoSqlError::PreferenceError("empty encrypted value".into()));
                }
                let nonce_len = encrypted[0] as usize;
                if encrypted.len() < 1 + nonce_len {
                    return Err(NoSqlError::PreferenceError("encrypted value too short".into()));
                }
                let nonce = &encrypted[1..1 + nonce_len];
                let ct = &encrypted[1 + nonce_len..];
                let derived = nodedb_crypto::hkdf_derive_key(master, &format!("prefs:{}", key))
                    .map_err(|e| NoSqlError::PreferenceError(e.to_string()))?;
                let plain_bytes = nodedb_crypto::encryption::aes_256_gcm_decrypt(&derived, nonce, ct)
                    .map_err(|e| NoSqlError::PreferenceError(e.to_string()))?;
                let value: Value = rmp_serde::from_slice(&plain_bytes)?;
                Ok(value)
            }
            None => {
                let value: Value = rmp_serde::from_slice(encrypted)?;
                Ok(value)
            }
        }
    }

    /// Set a preference key-value pair.
    ///
    /// - `key`: the preference name (e.g. "theme", "locale")
    /// - `value`: the value to store
    /// - `shareable`: whether this preference should be synced to peers
    /// - `conflict_resolution`: strategy for sync conflicts
    pub fn set(
        &self,
        key: &str,
        value: Value,
        shareable: bool,
        conflict_resolution: ConflictResolution,
    ) -> Result<Document, NoSqlError> {
        let enc_bytes = self.encrypt_value(key, &value)?;
        let enc_value = Value::Binary(enc_bytes);

        let value_type = match &value {
            Value::String(_) => "string",
            Value::Integer(_) => "integer",
            Value::Boolean(_) => "boolean",
            Value::F32(_) | Value::F64(_) => "float",
            Value::Map(_) => "map",
            Value::Array(_) => "array",
            Value::Binary(_) => "binary",
            Value::Nil => "nil",
            Value::Ext(_, _) => "ext",
        };

        let doc_data = Value::Map(vec![
            (Value::String("key".into()), Value::String(key.into())),
            (Value::String("enc_value".into()), enc_value),
            (Value::String("value_type".into()), Value::String(value_type.into())),
            (Value::String("shareable".into()), Value::Boolean(shareable)),
            (
                Value::String("conflict_resolution".into()),
                Value::String(conflict_resolution.as_str().into()),
            ),
            (
                Value::String("updated_at".into()),
                Value::Integer(chrono::Utc::now().timestamp_millis().into()),
            ),
        ]);

        // Check if key already exists → update, otherwise insert
        let existing_id = self.lookup_id(key)?;
        let doc = match existing_id {
            Some(id) => self.collection.update(id, doc_data)?,
            None => {
                let doc = self.collection.put(doc_data)?;
                // Store key→ID mapping
                self.index_tree
                    .insert(key.as_bytes(), &doc.id.to_be_bytes())?;
                doc
            }
        };

        Ok(doc)
    }

    /// Get a preference value by key.
    pub fn get(&self, key: &str) -> Result<Option<Value>, NoSqlError> {
        let id = match self.lookup_id(key)? {
            Some(id) => id,
            None => return Ok(None),
        };
        let doc = match self.collection.get(id) {
            Ok(d) => d,
            Err(NoSqlError::DocumentNotFound(_)) => return Ok(None),
            Err(e) => return Err(e),
        };
        let enc_bytes = self.extract_enc_bytes(&doc)?;
        let value = self.decrypt_value(key, &enc_bytes)?;
        Ok(Some(value))
    }

    /// Get a preference value along with the full document metadata.
    pub fn get_with_metadata(&self, key: &str) -> Result<Option<(Value, Document)>, NoSqlError> {
        let id = match self.lookup_id(key)? {
            Some(id) => id,
            None => return Ok(None),
        };
        let doc = match self.collection.get(id) {
            Ok(d) => d,
            Err(NoSqlError::DocumentNotFound(_)) => return Ok(None),
            Err(e) => return Err(e),
        };
        let enc_bytes = self.extract_enc_bytes(&doc)?;
        let value = self.decrypt_value(key, &enc_bytes)?;
        Ok(Some((value, doc)))
    }

    /// Remove a preference by key.
    pub fn remove(&self, key: &str) -> Result<bool, NoSqlError> {
        let id = match self.lookup_id(key)? {
            Some(id) => id,
            None => return Ok(false),
        };
        self.collection.delete(id)?;
        self.index_tree.remove(key.as_bytes())?;
        Ok(true)
    }

    /// List all preference keys.
    pub fn keys(&self) -> Result<Vec<String>, NoSqlError> {
        let mut keys = Vec::new();
        for result in self.index_tree.iter() {
            let (key_bytes, _) = result.map_err(NoSqlError::Storage)?;
            let key = String::from_utf8(key_bytes)
                .map_err(|e| NoSqlError::Serialization(e.to_string()))?;
            keys.push(key);
        }
        Ok(keys)
    }

    /// Return all shareable preferences (for sync).
    pub fn shareable_entries(&self) -> Result<Vec<(String, Document)>, NoSqlError> {
        let all_docs = self.collection.find_all(None, None)?;
        let mut result = Vec::new();
        for doc in all_docs {
            let shareable = doc
                .get_field("shareable")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if shareable {
                let key = doc
                    .get_field("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                result.push((key, doc));
            }
        }
        Ok(result)
    }

    /// Apply a remote preference update with conflict resolution.
    pub fn apply_remote(
        &self,
        key: &str,
        remote_value: Value,
        remote_updated_at: i64,
        remote_confidence: Option<f64>,
    ) -> Result<ConflictOutcome, NoSqlError> {
        let existing = self.get_with_metadata(key)?;
        match existing {
            None => {
                // No local value — accept remote
                self.set(key, remote_value, true, ConflictResolution::default())?;
                Ok(ConflictOutcome::AcceptRemote)
            }
            Some((_local_value, doc)) => {
                let local_updated_at = doc
                    .get_field("updated_at")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let strategy_str = doc
                    .get_field("conflict_resolution")
                    .and_then(|v| v.as_str())
                    .unwrap_or("last_write_wins");
                let strategy = ConflictResolution::from_str(strategy_str)
                    .unwrap_or(ConflictResolution::LastWriteWins);

                let ctx = ConflictContext {
                    local_updated_at,
                    remote_updated_at,
                    local_confidence: None,
                    remote_confidence,
                };

                let outcome = resolve_conflict(strategy, &ctx);
                if outcome == ConflictOutcome::AcceptRemote {
                    self.set(key, remote_value, true, strategy)?;
                }
                Ok(outcome)
            }
        }
    }

    // ── Helpers ────────────────────────────────────────────────────────

    fn lookup_id(&self, key: &str) -> Result<Option<i64>, NoSqlError> {
        match self.index_tree.get(key.as_bytes())? {
            Some(bytes) => {
                if bytes.len() != 8 {
                    return Err(NoSqlError::PreferenceError(
                        "corrupt index entry".to_string(),
                    ));
                }
                let id = i64::from_be_bytes(bytes[..8].try_into().unwrap());
                Ok(Some(id))
            }
            None => Ok(None),
        }
    }

    fn extract_enc_bytes(&self, doc: &Document) -> Result<Vec<u8>, NoSqlError> {
        match doc.get_field("enc_value") {
            Some(Value::Binary(bytes)) => Ok(bytes.clone()),
            _ => Err(NoSqlError::PreferenceError(
                "missing or invalid enc_value field".to_string(),
            )),
        }
    }
}

/// Extension methods on Database for creating PreferencesStore instances.
impl crate::database::Database {
    /// Get or create a PreferencesStore for the given store name.
    ///
    /// Creates a collection with `collection_type = "preferences"` and an index tree.
    pub fn preferences(&self, name: &str) -> Result<PreferencesStore, NoSqlError> {
        let col = self.collection(name)?;

        // Mark collection_type if not already set
        let qn = crate::schema::QualifiedName::parse(name);
        let meta_key = qn.meta_key();
        if let Some(entry_bytes) = self.engine().open_tree("__collections_meta__")
            .ok()
            .and_then(|tree| tree.get(meta_key.as_bytes()).ok().flatten())
        {
            if let Ok(mut entry) = rmp_serde::from_slice::<crate::schema::SchemaEntry>(&entry_bytes) {
                if entry.collection_type.is_none() {
                    entry.collection_type = Some("preferences".to_string());
                    if let Ok(bytes) = rmp_serde::to_vec(&entry) {
                        let meta_tree = self.engine().open_tree("__collections_meta__").ok();
                        if let Some(tree) = meta_tree {
                            let _ = tree.insert(meta_key.as_bytes(), &bytes);
                        }
                    }
                }
            }
        }

        let index_tree_name = format!("__pref_idx_{}__", meta_key);
        let index_tree = self.engine().open_tree(&index_tree_name)?;
        let dek = self.engine().dek();

        Ok(PreferencesStore::new(col, index_tree, dek))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use tempfile::TempDir;

    #[test]
    fn test_pref_set_get_string() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();

        prefs.set("theme", Value::String("dark".into()), false, ConflictResolution::LastWriteWins).unwrap();
        let val = prefs.get("theme").unwrap().unwrap();
        assert_eq!(val, Value::String("dark".into()));
    }

    #[test]
    fn test_pref_set_get_int() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();

        prefs.set("font_size", Value::Integer(14.into()), false, ConflictResolution::default()).unwrap();
        let val = prefs.get("font_size").unwrap().unwrap();
        assert_eq!(val, Value::Integer(14.into()));
    }

    #[test]
    fn test_pref_set_get_bool() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();

        prefs.set("notifications", Value::Boolean(true), false, ConflictResolution::default()).unwrap();
        let val = prefs.get("notifications").unwrap().unwrap();
        assert_eq!(val, Value::Boolean(true));
    }

    #[test]
    fn test_pref_set_get_map() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();

        let map_val = Value::Map(vec![
            (Value::String("x".into()), Value::Integer(10.into())),
            (Value::String("y".into()), Value::Integer(20.into())),
        ]);
        prefs.set("window_pos", map_val.clone(), false, ConflictResolution::default()).unwrap();
        let val = prefs.get("window_pos").unwrap().unwrap();
        assert_eq!(val, map_val);
    }

    #[test]
    fn test_pref_get_nonexistent() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();
        assert!(prefs.get("nonexistent").unwrap().is_none());
    }

    #[test]
    fn test_pref_overwrite() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();

        prefs.set("theme", Value::String("light".into()), false, ConflictResolution::default()).unwrap();
        prefs.set("theme", Value::String("dark".into()), false, ConflictResolution::default()).unwrap();
        let val = prefs.get("theme").unwrap().unwrap();
        assert_eq!(val, Value::String("dark".into()));
    }

    #[test]
    fn test_pref_remove() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();

        prefs.set("theme", Value::String("dark".into()), false, ConflictResolution::default()).unwrap();
        assert!(prefs.remove("theme").unwrap());
        assert!(prefs.get("theme").unwrap().is_none());
        assert!(!prefs.remove("theme").unwrap()); // already removed
    }

    #[test]
    fn test_pref_keys() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();

        prefs.set("theme", Value::String("dark".into()), false, ConflictResolution::default()).unwrap();
        prefs.set("lang", Value::String("en".into()), false, ConflictResolution::default()).unwrap();
        prefs.set("size", Value::Integer(14.into()), false, ConflictResolution::default()).unwrap();

        let mut keys = prefs.keys().unwrap();
        keys.sort();
        assert_eq!(keys, vec!["lang", "size", "theme"]);
    }

    #[test]
    fn test_pref_shareable_entries() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();

        prefs.set("theme", Value::String("dark".into()), true, ConflictResolution::default()).unwrap();
        prefs.set("secret", Value::String("hidden".into()), false, ConflictResolution::default()).unwrap();
        prefs.set("lang", Value::String("en".into()), true, ConflictResolution::default()).unwrap();

        let shareable = prefs.shareable_entries().unwrap();
        assert_eq!(shareable.len(), 2);
        let keys: Vec<&str> = shareable.iter().map(|(k, _)| k.as_str()).collect();
        assert!(keys.contains(&"theme"));
        assert!(keys.contains(&"lang"));
    }

    #[test]
    fn test_pref_encrypted_roundtrip() {
        let dir = TempDir::new().unwrap();
        let db = Database::open_encrypted(dir.path(), [42u8; 32]).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();

        prefs.set("theme", Value::String("dark".into()), false, ConflictResolution::default()).unwrap();
        let val = prefs.get("theme").unwrap().unwrap();
        assert_eq!(val, Value::String("dark".into()));
    }

    #[test]
    fn test_pref_encrypted_value_not_plaintext() {
        let dir = TempDir::new().unwrap();
        let db = Database::open_encrypted(dir.path(), [42u8; 32]).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();

        prefs.set("theme", Value::String("dark".into()), false, ConflictResolution::default()).unwrap();

        // Read raw document — enc_value should not contain plaintext "dark"
        let col = db.collection("app_prefs").unwrap();
        let doc = col.get(1).unwrap();
        let enc_bytes = doc.get_field("enc_value");
        if let Some(Value::Binary(bytes)) = enc_bytes {
            // The encrypted bytes should not contain the msgpack encoding of "dark"
            let plain = rmp_serde::to_vec(&Value::String("dark".into())).unwrap();
            assert_ne!(bytes, &plain);
        }
    }

    #[test]
    fn test_pref_different_keys_different_encryption() {
        let dir = TempDir::new().unwrap();
        let db = Database::open_encrypted(dir.path(), [42u8; 32]).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();

        // Set same value with different keys
        let val = Value::String("same_value".into());
        prefs.set("key_a", val.clone(), false, ConflictResolution::default()).unwrap();
        prefs.set("key_b", val, false, ConflictResolution::default()).unwrap();

        // Read raw enc_values
        let col = db.collection("app_prefs").unwrap();
        let doc_a = col.get(1).unwrap();
        let doc_b = col.get(2).unwrap();
        let enc_a = doc_a.get_field("enc_value");
        let enc_b = doc_b.get_field("enc_value");
        // Different keys derive different encryption keys, so ciphertext should differ
        // (also random nonce differs, but even same nonce + different key = different ct)
        assert_ne!(enc_a, enc_b);
    }

    #[test]
    fn test_pref_persistence() {
        let dir = TempDir::new().unwrap();
        {
            let db = Database::open(dir.path()).unwrap();
            let prefs = db.preferences("app_prefs").unwrap();
            prefs.set("theme", Value::String("dark".into()), false, ConflictResolution::default()).unwrap();
            db.flush().unwrap();
        }
        {
            let db = Database::open(dir.path()).unwrap();
            let prefs = db.preferences("app_prefs").unwrap();
            let val = prefs.get("theme").unwrap().unwrap();
            assert_eq!(val, Value::String("dark".into()));
        }
    }

    #[test]
    fn test_pref_conflict_lww_remote_wins() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();

        prefs.set("theme", Value::String("light".into()), true, ConflictResolution::LastWriteWins).unwrap();

        // Remote value with newer timestamp
        let outcome = prefs
            .apply_remote("theme", Value::String("dark".into()), i64::MAX, None)
            .unwrap();
        assert_eq!(outcome, ConflictOutcome::AcceptRemote);
        assert_eq!(prefs.get("theme").unwrap().unwrap(), Value::String("dark".into()));
    }

    #[test]
    fn test_pref_conflict_local_wins() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();

        prefs.set("theme", Value::String("light".into()), true, ConflictResolution::LocalWins).unwrap();

        let outcome = prefs
            .apply_remote("theme", Value::String("dark".into()), i64::MAX, None)
            .unwrap();
        assert_eq!(outcome, ConflictOutcome::KeepLocal);
        assert_eq!(prefs.get("theme").unwrap().unwrap(), Value::String("light".into()));
    }

    #[test]
    fn test_pref_conflict_remote_wins_strategy() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();

        prefs.set("theme", Value::String("light".into()), true, ConflictResolution::RemoteWins).unwrap();

        let outcome = prefs
            .apply_remote("theme", Value::String("dark".into()), 0, None)
            .unwrap();
        assert_eq!(outcome, ConflictOutcome::AcceptRemote);
        assert_eq!(prefs.get("theme").unwrap().unwrap(), Value::String("dark".into()));
    }

    #[test]
    fn test_pref_conflict_highest_confidence() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();

        prefs.set("theme", Value::String("light".into()), true, ConflictResolution::HighestConfidence).unwrap();

        // local_confidence is None, remote_confidence is Some(0.99)
        // HighestConfidence falls back to LWW when one side is None
        // Use future timestamp so remote wins via LWW fallback
        let outcome = prefs
            .apply_remote("theme", Value::String("dark".into()), i64::MAX, Some(0.99))
            .unwrap();
        assert_eq!(outcome, ConflictOutcome::AcceptRemote);
    }

    #[test]
    fn test_pref_apply_remote_no_local() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();

        let outcome = prefs
            .apply_remote("theme", Value::String("dark".into()), 100, None)
            .unwrap();
        assert_eq!(outcome, ConflictOutcome::AcceptRemote);
        assert_eq!(prefs.get("theme").unwrap().unwrap(), Value::String("dark".into()));
    }

    #[test]
    fn test_pref_multiple_stores_coexist() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();

        let prefs_a = db.preferences("store_a").unwrap();
        let prefs_b = db.preferences("store_b").unwrap();

        prefs_a.set("key", Value::String("from_a".into()), false, ConflictResolution::default()).unwrap();
        prefs_b.set("key", Value::String("from_b".into()), false, ConflictResolution::default()).unwrap();

        assert_eq!(prefs_a.get("key").unwrap().unwrap(), Value::String("from_a".into()));
        assert_eq!(prefs_b.get("key").unwrap().unwrap(), Value::String("from_b".into()));
    }

    #[test]
    fn test_pref_unencrypted_db_works() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();

        prefs.set("theme", Value::String("dark".into()), false, ConflictResolution::default()).unwrap();
        let val = prefs.get("theme").unwrap().unwrap();
        assert_eq!(val, Value::String("dark".into()));
    }

    #[test]
    fn test_pref_get_with_metadata() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let prefs = db.preferences("app_prefs").unwrap();

        prefs.set("theme", Value::String("dark".into()), true, ConflictResolution::LocalWins).unwrap();
        let (val, doc) = prefs.get_with_metadata("theme").unwrap().unwrap();
        assert_eq!(val, Value::String("dark".into()));
        assert_eq!(doc.get_field("shareable"), Some(&Value::Boolean(true)));
        assert_eq!(
            doc.get_field("conflict_resolution"),
            Some(&Value::String("local_wins".into()))
        );
    }
}

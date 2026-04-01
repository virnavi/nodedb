use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};

use rmpv::Value;

use crate::access_history::{AccessHistoryConfig, AccessHistoryStore};
use crate::collection::Collection;
use crate::document::Document;
use crate::error::NoSqlError;
use crate::schema::{
    CollectionSchema, QualifiedName, SchemaEntry, SchemaMetadata, DEFAULT_SCHEMA,
    META_KEY_SEPARATOR, SECURITY_SCHEMA, is_reserved_schema,
};
use crate::trigger::{TriggerContext, TriggerEvent, TriggerRegistry, TriggerResult, TriggerTiming};
use crate::cache::RecordCacheStore;
use crate::trim::TrimConfig;
use nodedb_storage::{IdGenerator, OwnerKeyStatus, StorageEngine, StorageTree};

/// Maximum trigger reentrancy depth to prevent infinite loops.
const MAX_TRIGGER_DEPTH: u8 = 8;

thread_local! {
    static TRIGGER_DEPTH: Cell<u8> = const { Cell::new(0) };
}

pub struct Database {
    path: PathBuf,
    engine: Arc<StorageEngine>,
    id_gen: Arc<IdGenerator>,
    /// Keyed by meta_key format: `"{schema}::{collection}"`.
    collections: Mutex<HashMap<String, Arc<Collection>>>,
    meta_tree: StorageTree,
    schema_meta: StorageTree,
    schemas: Mutex<HashMap<String, SchemaMetadata>>,
    database_name: Option<String>,
    trigger_registry: TriggerRegistry,
    /// Weak self-reference set after wrapping in Arc. Used for TriggerContext.
    self_ref: RwLock<Weak<Database>>,
    /// Stores default values for singleton collections (keyed by meta_key).
    singleton_defaults_tree: StorageTree,
    /// Set of meta_keys that are singleton collections.
    singletons: Mutex<HashSet<String>>,
    /// Access history store for tracking record access events.
    access_history: AccessHistoryStore,
    /// Access history configuration.
    access_history_config: AccessHistoryConfig,
    /// Trim configuration (runtime + record-level overrides).
    trim_config: TrimConfig,
    /// Per-record cache configuration store.
    record_cache: RecordCacheStore,
    /// Monotonic counter incremented on every write (local or remote-applied).
    /// Dart polls this to detect changes without FFI callbacks.
    sync_version: AtomicU64,
}

const META_TREE_NAME: &str = "__collections_meta__";
const SCHEMA_META_TREE: &str = "__schema_meta__";
const SINGLETON_DEFAULTS_TREE: &str = "__singleton_defaults__";
const ACCESS_HISTORY_TREE: &str = "__access_history__";
const TRIM_CONFIG_TREE: &str = "__trim_config__";
const TRIM_RECORD_OVERRIDES_TREE: &str = "__trim_record_overrides__";
const RECORD_CACHE_TREE: &str = "__record_cache_config__";

/// Validate a schema name: lowercase alphanumeric + hyphens + underscores, max 64 chars.
fn validate_schema_name(name: &str) -> Result<(), NoSqlError> {
    if name.is_empty() || name.len() > 64 {
        return Err(NoSqlError::InvalidSchema(format!(
            "schema name must be 1-64 chars, got {}",
            name.len()
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        return Err(NoSqlError::InvalidSchema(format!(
            "schema name '{}' contains invalid characters (allowed: a-z, 0-9, -, _)",
            name
        )));
    }
    Ok(())
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, NoSqlError> {
        let engine = Arc::new(StorageEngine::open(path)?);
        Self::from_engine(path, engine)
    }

    /// Open a database with transparent encryption using a pre-unsealed DEK.
    pub fn open_encrypted(path: &Path, dek: [u8; 32]) -> Result<Self, NoSqlError> {
        let engine = Arc::new(StorageEngine::open_encrypted(path, dek)?);
        Self::from_engine(path, engine)
    }

    /// Open a database in mismatch mode (all reads empty, writes no-op).
    pub fn open_mismatch(path: &Path) -> Result<Self, NoSqlError> {
        let engine = Arc::new(StorageEngine::open_mismatch(path)?);
        Self::from_engine(path, engine)
    }

    /// Open a database with a pre-configured StorageEngine.
    pub fn open_with_engine(path: &Path, engine: Arc<StorageEngine>) -> Result<Self, NoSqlError> {
        Self::from_engine(path, engine)
    }

    fn from_engine(path: &Path, engine: Arc<StorageEngine>) -> Result<Self, NoSqlError> {
        let id_gen = Arc::new(IdGenerator::new(&engine)?);
        let meta_tree = engine.open_tree(META_TREE_NAME)?;
        let schema_meta = engine.open_tree(SCHEMA_META_TREE)?;
        let singleton_defaults_tree = engine.open_tree(SINGLETON_DEFAULTS_TREE)?;

        let database_name = engine.database_name().unwrap_or(None);

        // Access history — internal collection, bypasses schema checks
        let ah_tree = engine.open_tree(ACCESS_HISTORY_TREE)?;
        let ah_id_gen = Arc::clone(&id_gen);
        let ah_engine = Arc::clone(&engine);
        let ah_col = Arc::new(Collection::new(
            "__access_history__",
            ah_tree,
            ah_id_gen,
            ah_engine,
        ));
        let access_history = AccessHistoryStore::new(ah_col);

        // Trim config
        let trim_col_tree = engine.open_tree(TRIM_CONFIG_TREE)?;
        let trim_rec_tree = engine.open_tree(TRIM_RECORD_OVERRIDES_TREE)?;
        let trim_config = TrimConfig::new(trim_col_tree, trim_rec_tree);

        // Record cache config
        let cache_tree = engine.open_tree(RECORD_CACHE_TREE)?;
        let record_cache = RecordCacheStore::new(cache_tree);

        let db = Database {
            path: path.to_path_buf(),
            engine,
            id_gen,
            collections: Mutex::new(HashMap::new()),
            meta_tree,
            schema_meta,
            schemas: Mutex::new(HashMap::new()),
            database_name,
            trigger_registry: TriggerRegistry::new(),
            self_ref: RwLock::new(Weak::new()),
            singleton_defaults_tree,
            singletons: Mutex::new(HashSet::new()),
            access_history,
            access_history_config: AccessHistoryConfig::default(),
            trim_config,
            record_cache,
            sync_version: AtomicU64::new(0),
        };

        db.migrate_legacy_meta()?;
        db.load_schemas()?;
        db.ensure_public_schema()?;
        db.ensure_security_schema()?;
        db.load_existing_collections()?;
        Ok(db)
    }

    /// Get the owner key status.
    pub fn owner_key_status(&self) -> OwnerKeyStatus {
        self.engine.owner_key_status()
    }

    /// Get the current sync version counter.
    pub fn sync_version(&self) -> u64 {
        self.sync_version.load(Ordering::Acquire)
    }

    /// Increment the sync version counter (called after every successful write).
    pub fn increment_sync_version(&self) {
        self.sync_version.fetch_add(1, Ordering::Release);
    }

    /// Get the database name (if set in storage header).
    pub fn database_name(&self) -> Option<&str> {
        self.database_name.as_deref()
    }

    // ── Legacy Migration ─────────────────────────────────────────────

    /// Detect old-format meta entries (keys without `::`) and migrate to new format.
    fn migrate_legacy_meta(&self) -> Result<(), NoSqlError> {
        let mut legacy_entries: Vec<(String, CollectionSchema)> = Vec::new();

        for result in self.meta_tree.iter() {
            let (key_bytes, value_bytes) = result.map_err(NoSqlError::Storage)?;
            let key = String::from_utf8(key_bytes)
                .map_err(|e| NoSqlError::Serialization(e.to_string()))?;

            // New-format keys contain `::`; old-format keys do not.
            if !key.contains(META_KEY_SEPARATOR) {
                // Try to deserialize as old CollectionSchema
                if let Ok(old_schema) = rmp_serde::from_slice::<CollectionSchema>(&value_bytes) {
                    legacy_entries.push((key, old_schema));
                }
            }
        }

        if legacy_entries.is_empty() {
            return Ok(());
        }

        for (old_key, old_schema) in &legacy_entries {
            let entry = SchemaEntry {
                schema_name: DEFAULT_SCHEMA.to_string(),
                collection_name: old_key.clone(),
                tree_name: old_key.clone(), // Keep original sled tree name
                sharing_status: None,
                singleton: false,
                collection_type: None,
                created_at: old_schema.created_at,
            };
            let new_key = format!("{}{}{}", DEFAULT_SCHEMA, META_KEY_SEPARATOR, old_key);
            let entry_bytes = rmp_serde::to_vec(&entry)?;
            self.meta_tree.insert(new_key.as_bytes(), &entry_bytes)?;
            self.meta_tree.remove(old_key.as_bytes())?;
        }

        Ok(())
    }

    // ── Schema Registry ──────────────────────────────────────────────

    fn load_schemas(&self) -> Result<(), NoSqlError> {
        let mut schemas = self.schemas.lock().unwrap();
        for result in self.schema_meta.iter() {
            let (key_bytes, value_bytes) = result.map_err(NoSqlError::Storage)?;
            let name = String::from_utf8(key_bytes)
                .map_err(|e| NoSqlError::Serialization(e.to_string()))?;
            let meta: SchemaMetadata = rmp_serde::from_slice(&value_bytes)?;
            schemas.insert(name, meta);
        }
        Ok(())
    }

    fn ensure_public_schema(&self) -> Result<(), NoSqlError> {
        let mut schemas = self.schemas.lock().unwrap();
        if !schemas.contains_key(DEFAULT_SCHEMA) {
            let meta = SchemaMetadata {
                name: DEFAULT_SCHEMA.to_string(),
                sharing_status: None,
                created_at: chrono::Utc::now(),
            };
            let bytes = rmp_serde::to_vec(&meta)?;
            self.schema_meta
                .insert(DEFAULT_SCHEMA.as_bytes(), &bytes)?;
            schemas.insert(DEFAULT_SCHEMA.to_string(), meta);
        }
        Ok(())
    }

    fn ensure_security_schema(&self) -> Result<(), NoSqlError> {
        let mut schemas = self.schemas.lock().unwrap();
        if !schemas.contains_key(SECURITY_SCHEMA) {
            let meta = SchemaMetadata {
                name: SECURITY_SCHEMA.to_string(),
                sharing_status: Some("private".to_string()),
                created_at: chrono::Utc::now(),
            };
            let bytes = rmp_serde::to_vec(&meta)?;
            self.schema_meta
                .insert(SECURITY_SCHEMA.as_bytes(), &bytes)?;
            schemas.insert(SECURITY_SCHEMA.to_string(), meta);
        }
        Ok(())
    }

    /// Create a new schema.
    pub fn create_schema(
        &self,
        name: &str,
        sharing_status: Option<&str>,
    ) -> Result<(), NoSqlError> {
        validate_schema_name(name)?;

        if is_reserved_schema(name) {
            return Err(NoSqlError::InvalidSchema(
                format!("cannot create reserved schema '{}'", name),
            ));
        }

        let mut schemas = self.schemas.lock().unwrap();
        if schemas.contains_key(name) {
            return Ok(()); // Idempotent
        }

        let meta = SchemaMetadata {
            name: name.to_string(),
            sharing_status: sharing_status.map(|s| s.to_string()),
            created_at: chrono::Utc::now(),
        };
        let bytes = rmp_serde::to_vec(&meta)?;
        self.schema_meta.insert(name.as_bytes(), &bytes)?;
        schemas.insert(name.to_string(), meta);
        Ok(())
    }

    /// Drop a schema. Fails if the schema has collections or is "public".
    pub fn drop_schema(&self, name: &str) -> Result<bool, NoSqlError> {
        if name == DEFAULT_SCHEMA {
            return Err(NoSqlError::InvalidSchema(
                "cannot drop the 'public' schema".to_string(),
            ));
        }

        if is_reserved_schema(name) {
            return Err(NoSqlError::InvalidSchema(
                format!("cannot drop reserved schema '{}'", name),
            ));
        }

        // Check for collections in this schema
        let collections = self.collections.lock().unwrap();
        let prefix = format!("{}{}", name, META_KEY_SEPARATOR);
        if collections.keys().any(|k| k.starts_with(&prefix)) {
            return Err(NoSqlError::SchemaNotEmpty(name.to_string()));
        }
        drop(collections);

        let mut schemas = self.schemas.lock().unwrap();
        if schemas.remove(name).is_some() {
            self.schema_meta.remove(name.as_bytes())?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// List all schemas.
    pub fn list_schemas(&self) -> Vec<SchemaMetadata> {
        let schemas = self.schemas.lock().unwrap();
        schemas.values().cloned().collect()
    }

    /// Get info about a specific schema.
    pub fn schema_info(&self, name: &str) -> Option<SchemaMetadata> {
        let schemas = self.schemas.lock().unwrap();
        schemas.get(name).cloned()
    }

    // ── Collection Access ────────────────────────────────────────────

    fn load_existing_collections(&self) -> Result<(), NoSqlError> {
        let mut collections = self.collections.lock().unwrap();
        let mut singletons = self.singletons.lock().unwrap();
        for result in self.meta_tree.iter() {
            let (key_bytes, value_bytes) = result.map_err(NoSqlError::Storage)?;
            let meta_key = String::from_utf8(key_bytes)
                .map_err(|e| NoSqlError::Serialization(e.to_string()))?;

            if collections.contains_key(&meta_key) {
                continue;
            }

            let entry: SchemaEntry = rmp_serde::from_slice(&value_bytes)?;
            if entry.singleton {
                singletons.insert(meta_key.clone());
            }
            let tree = self.engine.open_tree(&entry.tree_name)?;
            let col = Arc::new(Collection::new(
                &entry.collection_name,
                tree,
                Arc::clone(&self.id_gen),
                Arc::clone(&self.engine),
            ));
            collections.insert(meta_key, col);
        }
        Ok(())
    }

    /// Get or create a collection by name. Accepts bare names, schema.collection, or
    /// database.schema.collection formats.
    pub fn collection(&self, name: &str) -> Result<Arc<Collection>, NoSqlError> {
        let qn = QualifiedName::parse(name);

        if !qn.is_local(self.database_name.as_deref()) {
            return Err(NoSqlError::RemoteDatabase(format!(
                "collection '{}' refers to remote database '{}'",
                name,
                qn.database.as_deref().unwrap_or("unknown")
            )));
        }

        let meta_key = qn.meta_key();
        let mut collections = self.collections.lock().unwrap();

        if let Some(col) = collections.get(&meta_key) {
            return Ok(Arc::clone(col));
        }

        // Guard: prevent auto-creation of collections in reserved schemas
        if is_reserved_schema(&qn.schema) {
            return Err(NoSqlError::ReservedSchemaWriteNotPermitted(
                format!("cannot create collection in reserved schema '{}'", qn.schema),
            ));
        }

        // Auto-create schema if it doesn't exist
        {
            let mut schemas = self.schemas.lock().unwrap();
            if !schemas.contains_key(&qn.schema) {
                let schema_meta = SchemaMetadata {
                    name: qn.schema.clone(),
                    sharing_status: None,
                    created_at: chrono::Utc::now(),
                };
                let bytes = rmp_serde::to_vec(&schema_meta)?;
                self.schema_meta
                    .insert(qn.schema.as_bytes(), &bytes)?;
                schemas.insert(qn.schema.clone(), schema_meta);
            }
        }

        // Create collection lazily with schema-prefixed tree name
        let tree_name = meta_key.clone(); // e.g. "analytics::page_views"
        let tree = self.engine.open_tree(&tree_name)?;
        let col = Arc::new(Collection::new(
            &qn.collection,
            tree,
            Arc::clone(&self.id_gen),
            Arc::clone(&self.engine),
        ));

        // Register in metadata
        let entry = SchemaEntry {
            schema_name: qn.schema.clone(),
            collection_name: qn.collection.clone(),
            tree_name,
            sharing_status: None,
            singleton: false,
            collection_type: None,
            created_at: chrono::Utc::now(),
        };
        let entry_bytes = rmp_serde::to_vec(&entry)?;
        self.meta_tree.insert(meta_key.as_bytes(), &entry_bytes)?;

        collections.insert(meta_key, Arc::clone(&col));
        Ok(col)
    }

    /// Returns collection names as `"{schema}.{collection}"` format.
    pub fn collection_names(&self) -> Vec<String> {
        let collections = self.collections.lock().unwrap();
        collections
            .keys()
            .map(|meta_key| {
                // Convert "schema::collection" to "schema.collection"
                meta_key.replacen(META_KEY_SEPARATOR, ".", 1)
            })
            .collect()
    }

    /// Returns collection names in a specific schema.
    pub fn collection_names_in_schema(&self, schema: &str) -> Vec<String> {
        let collections = self.collections.lock().unwrap();
        let prefix = format!("{}{}", schema, META_KEY_SEPARATOR);
        collections
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .map(|meta_key| {
                meta_key
                    .strip_prefix(&prefix)
                    .unwrap_or(meta_key)
                    .to_string()
            })
            .collect()
    }

    /// Drop a collection by name (accepts FQN or bare name).
    pub fn drop_collection(&self, name: &str) -> Result<bool, NoSqlError> {
        let qn = QualifiedName::parse(name);

        if is_reserved_schema(&qn.schema) {
            return Err(NoSqlError::ReservedSchemaWriteNotPermitted(
                format!("cannot drop collection in reserved schema '{}'", qn.schema),
            ));
        }

        let meta_key = qn.meta_key();
        let mut collections = self.collections.lock().unwrap();

        // Look up the SchemaEntry to get the actual tree name
        let tree_name = match self.meta_tree.get(meta_key.as_bytes())? {
            Some(bytes) => {
                let entry: SchemaEntry = rmp_serde::from_slice(&bytes)?;
                entry.tree_name
            }
            None => {
                // Not in meta — check if it's in the collections map anyway
                if collections.remove(&meta_key).is_none() {
                    return Ok(false);
                }
                meta_key.clone()
            }
        };

        collections.remove(&meta_key);

        // Clear all data in the tree
        let tree = self.engine.open_tree(&tree_name)?;
        let keys: Vec<Vec<u8>> = tree
            .iter()
            .filter_map(|r| r.ok().map(|(k, _)| k))
            .collect();
        for key in &keys {
            tree.remove(key)?;
        }

        // Remove from metadata
        self.meta_tree.remove(meta_key.as_bytes())?;
        Ok(true)
    }

    /// Get the SchemaEntry for a collection (for sharing status resolution).
    pub fn collection_schema_entry(&self, name: &str) -> Option<SchemaEntry> {
        let qn = QualifiedName::parse(name);
        let meta_key = qn.meta_key();
        self.meta_tree
            .get(meta_key.as_bytes())
            .ok()
            .flatten()
            .and_then(|bytes| rmp_serde::from_slice(&bytes).ok())
    }

    /// Resolve the effective sharing status for a collection using the cascade:
    /// collection-level → schema-level → db_default → "full".
    pub fn effective_sharing_status(
        &self,
        name: &str,
        db_default: Option<&str>,
    ) -> String {
        let qn = QualifiedName::parse(name);
        let meta_key = qn.meta_key();

        // 1. Collection-level override
        if let Some(entry_bytes) = self.meta_tree.get(meta_key.as_bytes()).ok().flatten() {
            if let Ok(entry) = rmp_serde::from_slice::<SchemaEntry>(&entry_bytes) {
                if let Some(ref status) = entry.sharing_status {
                    return status.clone();
                }
            }
        }

        // 2. Schema-level override
        {
            let schemas = self.schemas.lock().unwrap();
            if let Some(schema_meta) = schemas.get(&qn.schema) {
                if let Some(ref status) = schema_meta.sharing_status {
                    return status.clone();
                }
            }
        }

        // 3. Database-level default
        if let Some(status) = db_default {
            return status.to_string();
        }

        // 4. Default: full
        "full".to_string()
    }

    // ── Schema Migration Operations ──────────────────────────────────

    /// Move a collection from one schema to another.
    pub fn move_collection(&self, from_fqn: &str, to_schema: &str) -> Result<(), NoSqlError> {
        validate_schema_name(to_schema)?;
        let from_qn = QualifiedName::parse(from_fqn);

        if is_reserved_schema(to_schema) {
            return Err(NoSqlError::InvalidSchema(
                format!("cannot move collection into reserved schema '{}'", to_schema),
            ));
        }
        if is_reserved_schema(&from_qn.schema) {
            return Err(NoSqlError::InvalidSchema(
                format!("cannot move collection out of reserved schema '{}'", from_qn.schema),
            ));
        }

        let from_meta_key = from_qn.meta_key();

        // Read the existing entry
        let entry_bytes = self
            .meta_tree
            .get(from_meta_key.as_bytes())?
            .ok_or_else(|| {
                NoSqlError::CollectionNotFound(from_fqn.to_string())
            })?;
        let old_entry: SchemaEntry = rmp_serde::from_slice(&entry_bytes)?;

        let new_meta_key = format!(
            "{}{}{}",
            to_schema, META_KEY_SEPARATOR, from_qn.collection
        );
        let new_tree_name = new_meta_key.clone();

        // Copy data from old tree to new tree
        let old_tree = self.engine.open_tree(&old_entry.tree_name)?;
        let new_tree = self.engine.open_tree(&new_tree_name)?;
        for result in old_tree.iter() {
            let (k, v) = result.map_err(NoSqlError::Storage)?;
            new_tree.insert(&k, &v)?;
        }

        // Create new SchemaEntry
        let new_entry = SchemaEntry {
            schema_name: to_schema.to_string(),
            collection_name: from_qn.collection.clone(),
            tree_name: new_tree_name,
            sharing_status: old_entry.sharing_status,
            singleton: old_entry.singleton,
            collection_type: old_entry.collection_type,
            created_at: old_entry.created_at,
        };
        let new_entry_bytes = rmp_serde::to_vec(&new_entry)?;
        self.meta_tree
            .insert(new_meta_key.as_bytes(), &new_entry_bytes)?;

        // Remove old meta entry
        self.meta_tree.remove(from_meta_key.as_bytes())?;

        // Clear old tree data
        let keys: Vec<Vec<u8>> = old_tree
            .iter()
            .filter_map(|r| r.ok().map(|(k, _)| k))
            .collect();
        for key in &keys {
            old_tree.remove(key)?;
        }

        // Update collections map
        {
            let mut collections = self.collections.lock().unwrap();
            collections.remove(&from_meta_key);
            // Re-open with new tree
            let tree = self.engine.open_tree(&new_entry.tree_name)?;
            let col = Arc::new(Collection::new(
                &from_qn.collection,
                tree,
                Arc::clone(&self.id_gen),
                Arc::clone(&self.engine),
            ));
            collections.insert(new_meta_key, col);
        }

        // Auto-create target schema if needed
        self.create_schema(to_schema, None)?;

        Ok(())
    }

    /// Rename a schema. Cannot rename "public".
    pub fn rename_schema(&self, old_name: &str, new_name: &str) -> Result<(), NoSqlError> {
        if old_name == DEFAULT_SCHEMA {
            return Err(NoSqlError::InvalidSchema(
                "cannot rename the 'public' schema".to_string(),
            ));
        }
        validate_schema_name(new_name)?;

        if is_reserved_schema(old_name) {
            return Err(NoSqlError::InvalidSchema(
                format!("cannot rename reserved schema '{}'", old_name),
            ));
        }
        if is_reserved_schema(new_name) {
            return Err(NoSqlError::InvalidSchema(
                format!("cannot rename into reserved schema '{}'", new_name),
            ));
        }

        // Find all collections in old schema
        let old_prefix = format!("{}{}", old_name, META_KEY_SEPARATOR);
        let collection_names: Vec<String> = {
            let collections = self.collections.lock().unwrap();
            collections
                .keys()
                .filter(|k| k.starts_with(&old_prefix))
                .map(|k| {
                    k.strip_prefix(&old_prefix)
                        .unwrap_or(k)
                        .to_string()
                })
                .collect()
        };

        // Move each collection
        for col_name in &collection_names {
            let from_fqn = format!("{}.{}", old_name, col_name);
            self.move_collection(&from_fqn, new_name)?;
        }

        // Update schema_meta: copy old to new, remove old
        {
            let mut schemas = self.schemas.lock().unwrap();
            if let Some(mut meta) = schemas.remove(old_name) {
                meta.name = new_name.to_string();
                let bytes = rmp_serde::to_vec(&meta)?;
                self.schema_meta.insert(new_name.as_bytes(), &bytes)?;
                self.schema_meta.remove(old_name.as_bytes())?;
                schemas.insert(new_name.to_string(), meta);
            }
        }

        Ok(())
    }

    // ── Fingerprint ──────────────────────────────────────────────────

    /// Compute a schema fingerprint: SHA-256 of sorted `"{schema}::{collection}\n"` lines.
    pub fn schema_fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        let collections = self.collections.lock().unwrap();
        let mut keys: Vec<&String> = collections.keys().collect();
        keys.sort();

        let mut hasher = Sha256::new();
        for key in keys {
            hasher.update(key.as_bytes());
            hasher.update(b"\n");
        }
        let result = hasher.finalize();
        hex::encode(result)
    }

    // ── Trigger System ────────────────────────────────────────────────

    /// Must be called after wrapping in Arc to enable trigger context.
    pub fn set_self_ref(self: &Arc<Self>) {
        let mut self_ref = self.self_ref.write().unwrap();
        *self_ref = Arc::downgrade(self);
    }

    /// Access the trigger registry.
    pub fn triggers(&self) -> &TriggerRegistry {
        &self.trigger_registry
    }

    fn weak_self(&self) -> Weak<Database> {
        self.self_ref.read().unwrap().clone()
    }

    /// Check and increment the reentrancy depth guard.
    fn enter_trigger() -> Result<(), NoSqlError> {
        TRIGGER_DEPTH.with(|d| {
            let depth = d.get();
            if depth >= MAX_TRIGGER_DEPTH {
                return Err(NoSqlError::TriggerAbort(
                    "maximum trigger depth exceeded".to_string(),
                ));
            }
            d.set(depth + 1);
            Ok(())
        })
    }

    /// Decrement the reentrancy depth guard.
    fn exit_trigger() {
        TRIGGER_DEPTH.with(|d| {
            let depth = d.get();
            if depth > 0 {
                d.set(depth - 1);
            }
        });
    }

    /// Insert a document into a collection, firing triggers.
    pub fn trigger_put(
        &self,
        collection_name: &str,
        data: Value,
    ) -> Result<Document, NoSqlError> {
        self.trigger_put_internal(collection_name, 0, data, false, None)
    }

    /// Insert with explicit ID, firing triggers.
    pub fn trigger_put_with_id(
        &self,
        collection_name: &str,
        id: i64,
        data: Value,
    ) -> Result<Document, NoSqlError> {
        self.trigger_put_internal(collection_name, id, data, false, None)
    }

    /// Insert with mesh origin metadata.
    pub fn trigger_put_from_mesh(
        &self,
        collection_name: &str,
        data: Value,
        source_database_name: &str,
    ) -> Result<Document, NoSqlError> {
        self.trigger_put_internal(
            collection_name,
            0,
            data,
            true,
            Some(source_database_name.to_string()),
        )
    }

    fn trigger_put_internal(
        &self,
        collection_name: &str,
        id: i64,
        data: Value,
        from_mesh: bool,
        source_database_name: Option<String>,
    ) -> Result<Document, NoSqlError> {
        Self::enter_trigger()?;
        let result = self.trigger_put_impl(collection_name, id, data, from_mesh, source_database_name);
        Self::exit_trigger();
        if result.is_ok() {
            self.increment_sync_version();
        }
        result
    }

    fn trigger_put_impl(
        &self,
        collection_name: &str,
        id: i64,
        data: Value,
        from_mesh: bool,
        source_database_name: Option<String>,
    ) -> Result<Document, NoSqlError> {
        let qn = QualifiedName::parse(collection_name);
        if is_reserved_schema(&qn.schema) {
            return Err(NoSqlError::ReservedSchemaWriteNotPermitted(qn.display_name()));
        }
        let collection = self.collection(collection_name)?;
        let meta_key = qn.meta_key();
        let display = qn.display_name();

        // Determine if this is an insert or update (for correct event matching)
        let (event, old_record) = if id != 0 {
            match collection.get(id) {
                Ok(doc) => (TriggerEvent::Update, Some(doc)),
                Err(NoSqlError::DocumentNotFound(_)) => (TriggerEvent::Insert, None),
                Err(e) => return Err(e),
            }
        } else {
            (TriggerEvent::Insert, None)
        };

        // ── "instead" triggers ──
        let instead_triggers = self.trigger_registry.matching(&meta_key, event, TriggerTiming::Instead);
        if !instead_triggers.is_empty() {
            let trigger = instead_triggers.last().unwrap();
            let ctx = TriggerContext {
                db: self.weak_self(),
                qualified_name: display,
                event,
                old_record,
                new_record: Some(Document::new(id, &qn.collection, data.clone())),
                from_mesh,
                source_database_name,
            };
            return match (trigger.handler)(&ctx) {
                TriggerResult::Proceed(Some(modified_data)) => {
                    collection.put_with_id(id, modified_data)
                }
                TriggerResult::Proceed(None) => collection.put_with_id(id, data),
                TriggerResult::Abort(reason) => Err(NoSqlError::TriggerAbort(reason)),
                TriggerResult::Skip => Err(NoSqlError::TriggerAbort(
                    "instead trigger skipped the write".to_string(),
                )),
            };
        }

        // ── "before" triggers ──
        let before_triggers = self.trigger_registry.matching(&meta_key, event, TriggerTiming::Before);
        let mut current_data = data;
        for trigger in &before_triggers {
            let ctx = TriggerContext {
                db: self.weak_self(),
                qualified_name: display.clone(),
                event,
                old_record: old_record.clone(),
                new_record: Some(Document::new(0, &qn.collection, current_data.clone())),
                from_mesh,
                source_database_name: source_database_name.clone(),
            };
            match (trigger.handler)(&ctx) {
                TriggerResult::Proceed(Some(modified)) => {
                    current_data = modified;
                }
                TriggerResult::Proceed(None) => {}
                TriggerResult::Abort(reason) => return Err(NoSqlError::TriggerAbort(reason)),
                TriggerResult::Skip => {} // no-op for before triggers
            }
        }

        // ── Execute the write ──
        let doc = collection.put_with_id(id, current_data)?;

        // ── "after" triggers ──
        let after_triggers = self.trigger_registry.matching(&meta_key, event, TriggerTiming::After);
        for trigger in &after_triggers {
            let ctx = TriggerContext {
                db: self.weak_self(),
                qualified_name: display.clone(),
                event,
                old_record: old_record.clone(),
                new_record: Some(doc.clone()),
                from_mesh,
                source_database_name: source_database_name.clone(),
            };
            match (trigger.handler)(&ctx) {
                TriggerResult::Proceed(_) | TriggerResult::Skip => {}
                TriggerResult::Abort(_) => {} // after triggers cannot abort
            }
        }

        Ok(doc)
    }

    /// Update a document, firing triggers.
    pub fn trigger_update(
        &self,
        collection_name: &str,
        id: i64,
        data: Value,
    ) -> Result<Document, NoSqlError> {
        self.trigger_update_internal(collection_name, id, data, false, None)
    }

    fn trigger_update_internal(
        &self,
        collection_name: &str,
        id: i64,
        data: Value,
        from_mesh: bool,
        source_database_name: Option<String>,
    ) -> Result<Document, NoSqlError> {
        Self::enter_trigger()?;
        let result = self.trigger_update_impl(collection_name, id, data, from_mesh, source_database_name);
        Self::exit_trigger();
        if result.is_ok() {
            self.increment_sync_version();
        }
        result
    }

    fn trigger_update_impl(
        &self,
        collection_name: &str,
        id: i64,
        data: Value,
        from_mesh: bool,
        source_database_name: Option<String>,
    ) -> Result<Document, NoSqlError> {
        let qn = QualifiedName::parse(collection_name);
        if is_reserved_schema(&qn.schema) {
            return Err(NoSqlError::ReservedSchemaWriteNotPermitted(qn.display_name()));
        }
        let collection = self.collection(collection_name)?;
        let meta_key = qn.meta_key();
        let display = qn.display_name();

        let old_doc = collection.get(id)?;

        // ── "instead" triggers ──
        let instead_triggers = self.trigger_registry.matching(
            &meta_key, TriggerEvent::Update, TriggerTiming::Instead,
        );
        if !instead_triggers.is_empty() {
            let trigger = instead_triggers.last().unwrap();
            let ctx = TriggerContext {
                db: self.weak_self(),
                qualified_name: display,
                event: TriggerEvent::Update,
                old_record: Some(old_doc.clone()),
                new_record: Some(Document::new(id, &qn.collection, data.clone())),
                from_mesh,
                source_database_name,
            };
            return match (trigger.handler)(&ctx) {
                TriggerResult::Proceed(Some(modified_data)) => {
                    collection.update(id, modified_data)
                }
                TriggerResult::Proceed(None) => collection.update(id, data),
                TriggerResult::Abort(reason) => Err(NoSqlError::TriggerAbort(reason)),
                TriggerResult::Skip => Err(NoSqlError::TriggerAbort(
                    "instead trigger skipped the write".to_string(),
                )),
            };
        }

        // ── "before" triggers ──
        let before_triggers = self.trigger_registry.matching(
            &meta_key, TriggerEvent::Update, TriggerTiming::Before,
        );
        let mut current_data = data;
        for trigger in &before_triggers {
            let ctx = TriggerContext {
                db: self.weak_self(),
                qualified_name: display.clone(),
                event: TriggerEvent::Update,
                old_record: Some(old_doc.clone()),
                new_record: Some(Document::new(id, &qn.collection, current_data.clone())),
                from_mesh,
                source_database_name: source_database_name.clone(),
            };
            match (trigger.handler)(&ctx) {
                TriggerResult::Proceed(Some(modified)) => {
                    current_data = modified;
                }
                TriggerResult::Proceed(None) => {}
                TriggerResult::Abort(reason) => return Err(NoSqlError::TriggerAbort(reason)),
                TriggerResult::Skip => {}
            }
        }

        // ── Execute the write ──
        let doc = collection.update(id, current_data)?;

        // ── "after" triggers ──
        let after_triggers = self.trigger_registry.matching(
            &meta_key, TriggerEvent::Update, TriggerTiming::After,
        );
        for trigger in &after_triggers {
            let ctx = TriggerContext {
                db: self.weak_self(),
                qualified_name: display.clone(),
                event: TriggerEvent::Update,
                old_record: Some(old_doc.clone()),
                new_record: Some(doc.clone()),
                from_mesh,
                source_database_name: source_database_name.clone(),
            };
            match (trigger.handler)(&ctx) {
                TriggerResult::Proceed(_) | TriggerResult::Skip => {}
                TriggerResult::Abort(_) => {}
            }
        }

        Ok(doc)
    }

    /// Delete a document, firing triggers.
    pub fn trigger_delete(
        &self,
        collection_name: &str,
        id: i64,
    ) -> Result<bool, NoSqlError> {
        self.trigger_delete_internal(collection_name, id, false, None)
    }

    fn trigger_delete_internal(
        &self,
        collection_name: &str,
        id: i64,
        from_mesh: bool,
        source_database_name: Option<String>,
    ) -> Result<bool, NoSqlError> {
        Self::enter_trigger()?;
        let result = self.trigger_delete_impl(collection_name, id, from_mesh, source_database_name);
        Self::exit_trigger();
        if let Ok(true) = result {
            self.increment_sync_version();
        }
        result
    }

    fn trigger_delete_impl(
        &self,
        collection_name: &str,
        id: i64,
        from_mesh: bool,
        source_database_name: Option<String>,
    ) -> Result<bool, NoSqlError> {
        let qn = QualifiedName::parse(collection_name);

        // Guard: reserved schemas cannot be written to
        if is_reserved_schema(&qn.schema) {
            return Err(NoSqlError::ReservedSchemaWriteNotPermitted(qn.display_name()));
        }

        // Guard: singletons cannot be deleted
        let meta_key = qn.meta_key();
        if self.is_singleton_meta_key(&meta_key) {
            return Err(NoSqlError::SingletonDeleteNotPermitted(
                qn.display_name(),
            ));
        }

        let collection = self.collection(collection_name)?;
        let display = qn.display_name();

        // Fetch old record for context
        let old_doc = match collection.get(id) {
            Ok(doc) => doc,
            Err(NoSqlError::DocumentNotFound(_)) => return Ok(false),
            Err(e) => return Err(e),
        };

        // ── "instead" triggers ──
        let instead_triggers = self.trigger_registry.matching(
            &meta_key, TriggerEvent::Delete, TriggerTiming::Instead,
        );
        if !instead_triggers.is_empty() {
            let trigger = instead_triggers.last().unwrap();
            let ctx = TriggerContext {
                db: self.weak_self(),
                qualified_name: display,
                event: TriggerEvent::Delete,
                old_record: Some(old_doc),
                new_record: None,
                from_mesh,
                source_database_name,
            };
            return match (trigger.handler)(&ctx) {
                TriggerResult::Proceed(_) => collection.delete(id),
                TriggerResult::Abort(reason) => Err(NoSqlError::TriggerAbort(reason)),
                TriggerResult::Skip => Ok(false),
            };
        }

        // ── "before" triggers ──
        let before_triggers = self.trigger_registry.matching(
            &meta_key, TriggerEvent::Delete, TriggerTiming::Before,
        );
        for trigger in &before_triggers {
            let ctx = TriggerContext {
                db: self.weak_self(),
                qualified_name: display.clone(),
                event: TriggerEvent::Delete,
                old_record: Some(old_doc.clone()),
                new_record: None,
                from_mesh,
                source_database_name: source_database_name.clone(),
            };
            match (trigger.handler)(&ctx) {
                TriggerResult::Proceed(_) => {}
                TriggerResult::Abort(reason) => return Err(NoSqlError::TriggerAbort(reason)),
                TriggerResult::Skip => {}
            }
        }

        // ── Execute the delete ──
        let deleted = collection.delete(id)?;

        // ── "after" triggers ──
        if deleted {
            let after_triggers = self.trigger_registry.matching(
                &meta_key, TriggerEvent::Delete, TriggerTiming::After,
            );
            for trigger in &after_triggers {
                let ctx = TriggerContext {
                    db: self.weak_self(),
                    qualified_name: display.clone(),
                    event: TriggerEvent::Delete,
                    old_record: Some(old_doc.clone()),
                    new_record: None,
                    from_mesh,
                    source_database_name: source_database_name.clone(),
                };
                match (trigger.handler)(&ctx) {
                    TriggerResult::Proceed(_) | TriggerResult::Skip => {}
                    TriggerResult::Abort(_) => {}
                }
            }
        }

        Ok(deleted)
    }

    /// Trigger-aware clear: fires delete triggers for each record.
    pub fn trigger_clear(
        &self,
        collection_name: &str,
    ) -> Result<usize, NoSqlError> {
        let qn = QualifiedName::parse(collection_name);

        // Guard: reserved schemas cannot be written to
        if is_reserved_schema(&qn.schema) {
            return Err(NoSqlError::ReservedSchemaWriteNotPermitted(qn.display_name()));
        }

        // Guard: singletons cannot be cleared
        if self.is_singleton_meta_key(&qn.meta_key()) {
            return Err(NoSqlError::SingletonClearNotPermitted(
                qn.display_name(),
            ));
        }

        let collection = self.collection(collection_name)?;
        let all_docs = collection.find_all(None, None)?;
        let mut count = 0;
        for doc in all_docs {
            match self.trigger_delete(collection_name, doc.id) {
                Ok(true) => count += 1,
                Ok(false) => {}
                Err(NoSqlError::TriggerAbort(_)) => {} // skip aborted
                Err(e) => return Err(e),
            }
        }
        Ok(count)
    }

    // ── Existing Public API ──────────────────────────────────────────

    pub fn write_txn<F>(&self, f: F) -> Result<(), NoSqlError>
    where
        F: FnOnce(&Self) -> Result<(), NoSqlError>,
    {
        f(self)
    }

    pub fn flush(&self) -> Result<(), NoSqlError> {
        self.engine.flush()?;
        Ok(())
    }

    pub fn close(self) -> Result<(), NoSqlError> {
        self.flush()?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn engine(&self) -> &StorageEngine {
        &self.engine
    }

    // ── Access History ────────────────────────────────────────────────

    /// Get a reference to the access history store.
    pub fn access_history(&self) -> &AccessHistoryStore {
        &self.access_history
    }

    /// Get a reference to the access history config.
    pub fn access_history_config(&self) -> &AccessHistoryConfig {
        &self.access_history_config
    }

    /// Set the access history configuration.
    pub fn set_access_history_config(&mut self, config: AccessHistoryConfig) {
        self.access_history_config = config;
    }

    /// Check if a collection should be excluded from access history recording.
    pub fn should_record_access(&self, collection: &str) -> bool {
        !self.access_history_config.exclude_collections.contains(&collection.to_string())
    }

    // ── Trim Configuration ────────────────────────────────────────────

    /// Get a reference to the trim configuration.
    pub fn trim_config(&self) -> &TrimConfig {
        &self.trim_config
    }

    // ── Record Cache ─────────────────────────────────────────────────

    /// Get a reference to the record cache store.
    pub fn record_cache(&self) -> &RecordCacheStore {
        &self.record_cache
    }

    /// Set cache configuration for a specific record.
    pub fn set_record_cache(
        &self,
        collection_name: &str,
        record_id: i64,
        config: &crate::cache::CacheConfig,
    ) -> Result<(), NoSqlError> {
        let qn = QualifiedName::parse(collection_name);
        self.record_cache.set(&qn.meta_key(), record_id, config)
    }

    /// Get cache configuration for a specific record.
    pub fn get_record_cache(
        &self,
        collection_name: &str,
        record_id: i64,
    ) -> Result<Option<crate::cache::CacheConfig>, NoSqlError> {
        let qn = QualifiedName::parse(collection_name);
        self.record_cache.get(&qn.meta_key(), record_id)
    }

    /// Clear cache configuration for a specific record.
    pub fn clear_record_cache(
        &self,
        collection_name: &str,
        record_id: i64,
    ) -> Result<(), NoSqlError> {
        let qn = QualifiedName::parse(collection_name);
        self.record_cache.remove(&qn.meta_key(), record_id)
    }

    /// Sweep expired cached records in a specific collection.
    /// Returns count of deleted records.
    pub fn sweep_expired(&self, collection_name: &str) -> Result<usize, NoSqlError> {
        let qn = QualifiedName::parse(collection_name);
        let meta_key = qn.meta_key();
        let now = chrono::Utc::now();
        let col = self.collection(collection_name)?;

        self.record_cache.sweep(
            &meta_key,
            now,
            |id| match col.get(id) {
                Ok(doc) => Ok(Some(doc)),
                Err(NoSqlError::DocumentNotFound(_)) => Ok(None),
                Err(e) => Err(e),
            },
            |id| {
                col.delete(id)?;
                Ok(())
            },
        )
    }

    /// Sweep expired cached records across all collections.
    /// Returns total count of deleted records.
    pub fn sweep_all_expired(&self) -> Result<usize, NoSqlError> {
        let names = self.collection_names();
        let mut total = 0;
        for name in names {
            total += self.sweep_expired(&name)?;
        }
        Ok(total)
    }

    // ── Singleton Collections ─────────────────────────────────────────

    fn is_singleton_meta_key(&self, meta_key: &str) -> bool {
        let singletons = self.singletons.lock().unwrap();
        singletons.contains(meta_key)
    }

    /// Check if a collection is a singleton.
    pub fn is_singleton(&self, name: &str) -> bool {
        let qn = QualifiedName::parse(name);
        self.is_singleton_meta_key(&qn.meta_key())
    }

    /// Create a singleton collection with default data.
    /// The default record is inserted at ID=1 and stored for `singleton_reset`.
    /// Idempotent: if already created, returns the existing collection.
    pub fn singleton_collection(
        &self,
        name: &str,
        defaults: Value,
    ) -> Result<Arc<Collection>, NoSqlError> {
        let qn = QualifiedName::parse(name);
        let meta_key = qn.meta_key();

        // If already a singleton, return existing
        {
            let singletons = self.singletons.lock().unwrap();
            if singletons.contains(&meta_key) {
                let collections = self.collections.lock().unwrap();
                if let Some(col) = collections.get(&meta_key) {
                    return Ok(Arc::clone(col));
                }
            }
        }

        // Create the collection (this auto-creates schema if needed)
        let col = self.collection(name)?;

        // Mark as singleton in SchemaEntry
        let entry_bytes = self.meta_tree.get(meta_key.as_bytes())?
            .ok_or_else(|| NoSqlError::CollectionNotFound(name.to_string()))?;
        let mut entry: SchemaEntry = rmp_serde::from_slice(&entry_bytes)?;
        entry.singleton = true;
        let updated_bytes = rmp_serde::to_vec(&entry)?;
        self.meta_tree.insert(meta_key.as_bytes(), &updated_bytes)?;

        // Store defaults
        let defaults_bytes = rmp_serde::to_vec(&defaults)?;
        self.singleton_defaults_tree.insert(meta_key.as_bytes(), &defaults_bytes)?;

        // Insert default record at ID=1 if not already present
        match col.get(1) {
            Err(NoSqlError::DocumentNotFound(_)) => {
                col.put_with_id(1, defaults)?;
            }
            Ok(_) => {} // already exists
            Err(e) => return Err(e),
        }

        // Register in singletons set
        {
            let mut singletons = self.singletons.lock().unwrap();
            singletons.insert(meta_key);
        }

        Ok(col)
    }

    /// Put data into a singleton collection (always upserts at ID=1, fires triggers).
    pub fn singleton_put(
        &self,
        name: &str,
        data: Value,
    ) -> Result<Document, NoSqlError> {
        let qn = QualifiedName::parse(name);
        let meta_key = qn.meta_key();
        if !self.is_singleton_meta_key(&meta_key) {
            return Err(NoSqlError::CollectionNotFound(format!(
                "'{}' is not a singleton collection", name
            )));
        }
        self.trigger_put_with_id(name, 1, data)
    }

    /// Get the singleton record (ID=1).
    pub fn singleton_get(
        &self,
        name: &str,
    ) -> Result<Document, NoSqlError> {
        let qn = QualifiedName::parse(name);
        let meta_key = qn.meta_key();
        if !self.is_singleton_meta_key(&meta_key) {
            return Err(NoSqlError::CollectionNotFound(format!(
                "'{}' is not a singleton collection", name
            )));
        }
        let col = self.collection(name)?;
        col.get(1)
    }

    /// Reset a singleton to its default values.
    pub fn singleton_reset(
        &self,
        name: &str,
    ) -> Result<Document, NoSqlError> {
        let qn = QualifiedName::parse(name);
        let meta_key = qn.meta_key();
        if !self.is_singleton_meta_key(&meta_key) {
            return Err(NoSqlError::CollectionNotFound(format!(
                "'{}' is not a singleton collection", name
            )));
        }

        let defaults_bytes = self.singleton_defaults_tree
            .get(meta_key.as_bytes())?
            .ok_or_else(|| NoSqlError::CollectionNotFound(format!(
                "no defaults found for singleton '{}'", name
            )))?;
        let defaults: Value = rmp_serde::from_slice(&defaults_bytes)?;
        self.trigger_put_with_id(name, 1, defaults)
    }

    // ── Trim Operations ─────────────────────────────────────────────

    /// Trim records in a single collection according to the given policy.
    ///
    /// Records protected by never-trim (at record or collection level) are skipped.
    /// Returns a TrimReport describing what was (or would be, if dry_run) deleted.
    pub fn trim(
        &self,
        collection_name: &str,
        policy: &crate::trim::TrimPolicy,
        dry_run: bool,
    ) -> Result<crate::trim::TrimReport, NoSqlError> {
        let qn = QualifiedName::parse(collection_name);
        let meta_key = qn.meta_key();

        // Check collection-level never-trim
        if self.trim_config.is_never_trim(&meta_key)? {
            return Ok(crate::trim::TrimReport::empty(collection_name, dry_run));
        }

        let col = self.collection(collection_name)?;
        let all_docs = col.find_all(None, None)?;
        let now = chrono::Utc::now();

        let mut report = crate::trim::TrimReport::empty(collection_name, dry_run);
        report.candidate_count = all_docs.len();

        for doc in &all_docs {
            // Check record-level never-trim
            if self.trim_config.is_record_never_trim(&meta_key, doc.id)? {
                report.never_trim_skipped_count += 1;
                continue;
            }

            let should_trim = self.evaluate_trim_policy(
                policy, collection_name, doc.id, &now,
            )?;

            if !should_trim {
                report.skipped_count += 1;
                continue;
            }

            if dry_run {
                report.deleted_count += 1;
                report.deleted_record_ids.push((collection_name.to_string(), doc.id));
            } else {
                // Use trigger_delete so triggers can abort
                match self.trigger_delete(collection_name, doc.id) {
                    Ok(_) => {
                        report.deleted_count += 1;
                        report.deleted_record_ids.push((collection_name.to_string(), doc.id));
                    }
                    Err(NoSqlError::TriggerAbort(_)) => {
                        report.trigger_aborted_count += 1;
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        Ok(report)
    }

    /// Trim across all collections according to the given policy.
    pub fn trim_all(
        &self,
        policy: &crate::trim::TrimPolicy,
        dry_run: bool,
    ) -> Result<crate::trim::TrimReport, NoSqlError> {
        let names = self.collection_names();
        let mut combined = crate::trim::TrimReport::empty("all", dry_run);

        for name in &names {
            // Skip internal collections
            if name.starts_with("__") || name.contains("__") {
                continue;
            }
            let report = self.trim(name, policy, dry_run)?;
            combined.candidate_count += report.candidate_count;
            combined.deleted_count += report.deleted_count;
            combined.skipped_count += report.skipped_count;
            combined.never_trim_skipped_count += report.never_trim_skipped_count;
            combined.trigger_aborted_count += report.trigger_aborted_count;
            combined.deleted_record_ids.extend(report.deleted_record_ids);
        }

        Ok(combined)
    }

    /// Generate a trim recommendation without executing any deletions.
    pub fn recommend_trim(
        &self,
        policy: &crate::trim::TrimPolicy,
        exclude_collections: &[String],
    ) -> Result<crate::trim::TrimRecommendation, NoSqlError> {
        use crate::trim::{TrimCandidate, TrimCollectionRecommendation, TrimRecommendation};

        let names = self.collection_names();
        let now = chrono::Utc::now();
        let mut by_collection = Vec::new();
        let mut total = 0;

        for name in &names {
            if name.starts_with("__") || name.contains("__") {
                continue;
            }
            if exclude_collections.contains(name) {
                continue;
            }

            let qn = QualifiedName::parse(name);
            let meta_key = qn.meta_key();

            if self.trim_config.is_never_trim(&meta_key)? {
                continue;
            }

            let col = self.collection(name)?;
            let all_docs = col.find_all(None, None)?;
            let mut candidates = Vec::new();

            for doc in &all_docs {
                if self.trim_config.is_record_never_trim(&meta_key, doc.id)? {
                    continue;
                }

                let should_trim = self.evaluate_trim_policy(
                    policy, name, doc.id, &now,
                )?;

                if should_trim {
                    let last_access = self.access_history.last_access_time(name, doc.id)?;
                    let age = last_access.map(|t| (now - t).num_seconds());

                    candidates.push(TrimCandidate {
                        collection: name.clone(),
                        record_id: doc.id,
                        last_accessed_at_utc: last_access,
                        age_since_last_access_secs: age,
                        ai_originated: false, // TODO: check provenance
                        confidence: None,     // TODO: check provenance
                        never_trim_protected: false,
                        reasons: vec![policy.as_str().to_string()],
                    });
                }
            }

            let count = candidates.len();
            if count > 0 {
                total += count;
                by_collection.push(TrimCollectionRecommendation {
                    collection: name.clone(),
                    candidate_count: count,
                    candidates,
                });
            }
        }

        Ok(TrimRecommendation {
            total_candidate_count: total,
            by_collection,
            generated_at_utc: now,
            policy: policy.clone(),
        })
    }

    /// Execute a user-approved trim.
    pub fn trim_approved(
        &self,
        approval: &crate::trim::UserApprovedTrim,
    ) -> Result<crate::trim::TrimReport, NoSqlError> {
        let mut report = crate::trim::TrimReport::empty("approved", false);

        for (collection, record_id) in &approval.confirmed_record_ids {
            let qn = QualifiedName::parse(collection);
            let meta_key = qn.meta_key();

            // Re-validate: skip if now protected
            if self.trim_config.is_record_never_trim(&meta_key, *record_id)? {
                report.never_trim_skipped_count += 1;
                continue;
            }

            report.candidate_count += 1;

            match self.trigger_delete(collection, *record_id) {
                Ok(_) => {
                    report.deleted_count += 1;
                    report.deleted_record_ids.push((collection.clone(), *record_id));
                }
                Err(NoSqlError::TriggerAbort(_)) => {
                    report.trigger_aborted_count += 1;
                }
                Err(NoSqlError::DocumentNotFound(_)) => {
                    report.skipped_count += 1; // already deleted
                }
                Err(e) => return Err(e),
            }
        }

        Ok(report)
    }

    /// Evaluate whether a record matches a trim policy.
    fn evaluate_trim_policy(
        &self,
        policy: &crate::trim::TrimPolicy,
        collection: &str,
        record_id: i64,
        now: &chrono::DateTime<chrono::Utc>,
    ) -> Result<bool, NoSqlError> {
        use crate::access_history::AccessEventType;
        use crate::trim::TrimPolicy;

        match policy {
            TrimPolicy::NotAccessedSince(secs) => {
                let cutoff = *now - chrono::Duration::seconds(*secs);
                match self.access_history.last_access_time(collection, record_id)? {
                    Some(t) => Ok(t < cutoff),
                    None => Ok(true), // never accessed = eligible
                }
            }
            TrimPolicy::NotReadSince(secs) => {
                let cutoff = *now - chrono::Duration::seconds(*secs);
                let reads = self.access_history.query_history(
                    Some(collection),
                    Some(record_id),
                    Some(AccessEventType::Read),
                    None,
                    None,
                )?;
                if reads.is_empty() {
                    return Ok(true); // never read = eligible
                }
                // Check if the most recent read is before cutoff
                // The query returns all matching entries; check the latest
                let latest = reads.last(); // find_all returns in insertion order
                if let Some(entry) = latest {
                    if let Some(ts_str) = entry.as_map().and_then(|m| {
                        crate::access_history::data_field_str_from_map(m, "accessed_at_utc")
                    }) {
                        if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(ts_str) {
                            return Ok(ts.with_timezone(&chrono::Utc) < cutoff);
                        }
                    }
                }
                Ok(true)
            }
            TrimPolicy::AiOriginatedOnly => {
                // Check if the record was written by AI
                let writes = self.access_history.query_history(
                    Some(collection),
                    Some(record_id),
                    Some(AccessEventType::AiWrite),
                    None,
                    Some(1),
                )?;
                Ok(!writes.is_empty())
            }
            TrimPolicy::ConfidenceBelow(_threshold) => {
                // TODO: integrate with provenance engine
                Ok(false)
            }
            TrimPolicy::ToTargetBytes(_max_bytes) => {
                // This policy is evaluated at collection level, not per-record
                // For per-record, always return true (caller handles ordering)
                Ok(true)
            }
            TrimPolicy::KeepMostRecentlyAccessed(_count) => {
                // This policy is evaluated at collection level
                Ok(true)
            }
            TrimPolicy::Compound(policies) => {
                // ALL must match
                for p in policies {
                    if !self.evaluate_trim_policy(p, collection, record_id, now)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            TrimPolicy::Any(policies) => {
                // At least one must match
                for p in policies {
                    if self.evaluate_trim_policy(p, collection, record_id, now)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmpv::Value;
    use tempfile::TempDir;

    fn sample_data(name: &str, age: i64) -> Value {
        Value::Map(vec![
            (Value::String("name".into()), Value::String(name.into())),
            (Value::String("age".into()), Value::Integer(age.into())),
        ])
    }

    #[test]
    fn test_open_and_collection() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();

        let users = db.collection("users").unwrap();
        users.put(sample_data("Alice", 30)).unwrap();

        assert_eq!(users.count(), 1);
        assert!(db.collection_names().contains(&"public.users".to_string()));
    }

    #[test]
    fn test_multiple_collections() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();

        let users = db.collection("users").unwrap();
        let posts = db.collection("posts").unwrap();

        users.put(sample_data("Alice", 30)).unwrap();
        posts
            .put(Value::Map(vec![(
                Value::String("title".into()),
                Value::String("Hello".into()),
            )]))
            .unwrap();

        assert_eq!(users.count(), 1);
        assert_eq!(posts.count(), 1);
    }

    #[test]
    fn test_persistence() {
        let dir = TempDir::new().unwrap();

        {
            let db = Database::open(dir.path()).unwrap();
            let users = db.collection("users").unwrap();
            users.put(sample_data("Alice", 30)).unwrap();
            users.put(sample_data("Bob", 25)).unwrap();
            db.close().unwrap();
        }

        {
            let db = Database::open(dir.path()).unwrap();
            let users = db.collection("users").unwrap();
            assert_eq!(users.count(), 2);
            let alice = users.get(1).unwrap();
            assert_eq!(
                alice.get_field("name"),
                Some(&Value::String("Alice".into()))
            );
        }
    }

    #[test]
    fn test_drop_collection() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();

        let users = db.collection("users").unwrap();
        users.put(sample_data("Alice", 30)).unwrap();
        drop(users);

        assert!(db.drop_collection("users").unwrap());
        assert!(!db.collection_names().contains(&"public.users".to_string()));

        // Can recreate
        let users = db.collection("users").unwrap();
        assert_eq!(users.count(), 0);
    }

    #[test]
    fn test_write_txn() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();

        db.write_txn(|db| {
            let users = db.collection("users")?;
            users.put(sample_data("Alice", 30))?;
            users.put(sample_data("Bob", 25))?;
            Ok(())
        })
        .unwrap();

        let users = db.collection("users").unwrap();
        assert_eq!(users.count(), 2);
    }

    // ── Schema-aware tests ───────────────────────────────────────────

    #[test]
    fn test_schema_collection_in_custom_schema() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();

        let events = db.collection("analytics.events").unwrap();
        events.put(sample_data("click", 1)).unwrap();
        assert_eq!(events.count(), 1);

        // Also accessible via the same FQN
        let events2 = db.collection("analytics.events").unwrap();
        assert_eq!(events2.count(), 1);

        // Schema was auto-created
        assert!(db.schema_info("analytics").is_some());
    }

    #[test]
    fn test_same_name_different_schemas() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();

        let public_users = db.collection("users").unwrap();
        let internal_users = db.collection("internal.users").unwrap();

        public_users.put(sample_data("Alice", 30)).unwrap();
        internal_users.put(sample_data("Bob", 25)).unwrap();
        internal_users.put(sample_data("Carol", 35)).unwrap();

        assert_eq!(public_users.count(), 1);
        assert_eq!(internal_users.count(), 2);
    }

    #[test]
    fn test_collection_names_in_schema() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();

        db.collection("users").unwrap();
        db.collection("posts").unwrap();
        db.collection("analytics.events").unwrap();
        db.collection("analytics.sessions").unwrap();

        let public_cols = db.collection_names_in_schema("public");
        assert_eq!(public_cols.len(), 2);
        assert!(public_cols.contains(&"users".to_string()));
        assert!(public_cols.contains(&"posts".to_string()));

        let analytics_cols = db.collection_names_in_schema("analytics");
        assert_eq!(analytics_cols.len(), 2);
    }

    #[test]
    fn test_create_and_drop_schema() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();

        db.create_schema("analytics", Some("read_only")).unwrap();
        let schemas = db.list_schemas();
        assert!(schemas.iter().any(|s| s.name == "analytics"));

        let info = db.schema_info("analytics").unwrap();
        assert_eq!(info.sharing_status, Some("read_only".to_string()));

        assert!(db.drop_schema("analytics").unwrap());
        assert!(db.schema_info("analytics").is_none());
    }

    #[test]
    fn test_cannot_drop_public_schema() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        assert!(db.drop_schema("public").is_err());
    }

    #[test]
    fn test_cannot_drop_non_empty_schema() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();

        db.collection("analytics.events").unwrap();
        assert!(db.drop_schema("analytics").is_err());
    }

    #[test]
    fn test_public_schema_exists_by_default() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        assert!(db.schema_info("public").is_some());
    }

    #[test]
    fn test_remote_database_rejected() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let result = db.collection("warehouse.public.products");
        assert!(result.is_err());
    }

    #[test]
    fn test_legacy_migration() {
        let dir = TempDir::new().unwrap();

        // Simulate old-format database by writing CollectionSchema directly
        {
            let engine = Arc::new(StorageEngine::open(dir.path()).unwrap());
            let meta_tree = engine.open_tree(META_TREE_NAME).unwrap();
            let id_gen = Arc::new(IdGenerator::new(&engine).unwrap());

            // Write old-format entries
            let old_schema = CollectionSchema::new("users");
            let bytes = rmp_serde::to_vec(&old_schema).unwrap();
            meta_tree.insert("users".as_bytes(), &bytes).unwrap();

            // Write some data to the "users" tree
            let tree = engine.open_tree("users").unwrap();
            let col = Collection::new("users", tree, Arc::clone(&id_gen), Arc::clone(&engine));
            col.put(sample_data("Alice", 30)).unwrap();
            col.put(sample_data("Bob", 25)).unwrap();

            engine.flush().unwrap();
        }

        // Reopen — migration should happen
        {
            let db = Database::open(dir.path()).unwrap();

            // Collection should be accessible by bare name (resolves to public.users)
            let users = db.collection("users").unwrap();
            assert_eq!(users.count(), 2);

            // Also accessible by FQN
            let users2 = db.collection("public.users").unwrap();
            assert_eq!(users2.count(), 2);

            // Verify data is intact
            let alice = users.get(1).unwrap();
            assert_eq!(
                alice.get_field("name"),
                Some(&Value::String("Alice".into()))
            );

            // collection_names now returns schema-qualified names
            let names = db.collection_names();
            assert!(names.contains(&"public.users".to_string()));
        }
    }

    #[test]
    fn test_move_collection() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();

        let users = db.collection("users").unwrap();
        users.put(sample_data("Alice", 30)).unwrap();
        users.put(sample_data("Bob", 25)).unwrap();
        drop(users);

        db.move_collection("public.users", "analytics").unwrap();

        // Old location is gone
        assert!(db.collection_names_in_schema("public").is_empty()
            || !db.collection_names_in_schema("public").contains(&"users".to_string()));

        // New location has the data
        let users = db.collection("analytics.users").unwrap();
        assert_eq!(users.count(), 2);
    }

    #[test]
    fn test_rename_schema() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();

        db.collection("internal.secrets").unwrap()
            .put(sample_data("key", 1))
            .unwrap();

        db.rename_schema("internal", "private").unwrap();

        assert!(db.schema_info("internal").is_none());
        assert!(db.schema_info("private").is_some());

        let secrets = db.collection("private.secrets").unwrap();
        assert_eq!(secrets.count(), 1);
    }

    #[test]
    fn test_cannot_rename_public() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        assert!(db.rename_schema("public", "main").is_err());
    }

    #[test]
    fn test_effective_sharing_status_cascade() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();

        // Default: "full"
        db.collection("users").unwrap();
        assert_eq!(db.effective_sharing_status("users", None), "full");

        // With db default
        assert_eq!(
            db.effective_sharing_status("users", Some("read_only")),
            "read_only"
        );

        // Schema-level override
        db.create_schema("secure", Some("private")).unwrap();
        db.collection("secure.data").unwrap();
        assert_eq!(
            db.effective_sharing_status("secure.data", Some("full")),
            "private"
        );
    }

    // ── Trigger-aware write tests ───────────────────────────────────

    fn make_arc_db() -> (Arc<Database>, TempDir) {
        let dir = TempDir::new().unwrap();
        let db = Arc::new(Database::open(dir.path()).unwrap());
        db.set_self_ref();
        (db, dir)
    }

    #[test]
    fn test_trigger_no_triggers_passthrough() {
        let (db, _dir) = make_arc_db();
        let doc = db.trigger_put("users", sample_data("Alice", 30)).unwrap();
        assert_eq!(doc.id, 1);
        assert_eq!(db.collection("users").unwrap().count(), 1);

        let updated = db.trigger_update("users", 1, sample_data("Alice", 31)).unwrap();
        assert_eq!(updated.id, 1);

        assert!(db.trigger_delete("users", 1).unwrap());
        assert_eq!(db.collection("users").unwrap().count(), 0);
    }

    #[test]
    fn test_trigger_before_insert_modifies_data() {
        let (db, _dir) = make_arc_db();

        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Insert,
            crate::trigger::TriggerTiming::Before,
            Some("trim_name".to_string()),
            std::sync::Arc::new(|ctx| {
                if let Some(ref new_rec) = ctx.new_record {
                    let mut data = new_rec.data.clone();
                    // Replace name with "MODIFIED"
                    if let Value::Map(ref mut map) = data {
                        for (k, v) in map.iter_mut() {
                            if k.as_str() == Some("name") {
                                *v = Value::String("MODIFIED".into());
                            }
                        }
                    }
                    crate::trigger::TriggerResult::Proceed(Some(data))
                } else {
                    crate::trigger::TriggerResult::Proceed(None)
                }
            }),
        );

        let doc = db.trigger_put("users", sample_data("Alice", 30)).unwrap();
        let fetched = db.collection("users").unwrap().get(doc.id).unwrap();
        assert_eq!(
            fetched.get_field("name"),
            Some(&Value::String("MODIFIED".into()))
        );
    }

    #[test]
    fn test_trigger_before_insert_abort() {
        let (db, _dir) = make_arc_db();

        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Insert,
            crate::trigger::TriggerTiming::Before,
            None,
            std::sync::Arc::new(|_ctx| {
                crate::trigger::TriggerResult::Abort("validation failed".to_string())
            }),
        );

        let result = db.trigger_put("users", sample_data("Alice", 30));
        assert!(result.is_err());
        assert!(matches!(result, Err(NoSqlError::TriggerAbort(_))));
        assert_eq!(db.collection("users").unwrap().count(), 0);
    }

    #[test]
    fn test_trigger_after_insert_side_effect() {
        let (db, _dir) = make_arc_db();

        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Insert,
            crate::trigger::TriggerTiming::After,
            None,
            std::sync::Arc::new(|ctx| {
                // Write to an audit collection as a side effect
                if let Some(db) = ctx.db.upgrade() {
                    let audit = db.collection("audit").unwrap();
                    let entry = Value::Map(vec![
                        (Value::String("action".into()), Value::String("insert".into())),
                        (
                            Value::String("collection".into()),
                            Value::String(ctx.qualified_name.clone().into()),
                        ),
                    ]);
                    audit.put(entry).unwrap();
                }
                crate::trigger::TriggerResult::Proceed(None)
            }),
        );

        db.trigger_put("users", sample_data("Alice", 30)).unwrap();
        let audit = db.collection("audit").unwrap();
        assert_eq!(audit.count(), 1);
    }

    #[test]
    fn test_trigger_instead_insert() {
        let (db, _dir) = make_arc_db();

        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Insert,
            crate::trigger::TriggerTiming::Instead,
            None,
            std::sync::Arc::new(|_ctx| {
                // Replace with completely different data
                let data = Value::Map(vec![
                    (Value::String("name".into()), Value::String("REPLACED".into())),
                    (Value::String("age".into()), Value::Integer(99.into())),
                ]);
                crate::trigger::TriggerResult::Proceed(Some(data))
            }),
        );

        let doc = db.trigger_put("users", sample_data("Alice", 30)).unwrap();
        assert_eq!(
            doc.get_field("name"),
            Some(&Value::String("REPLACED".into()))
        );
    }

    #[test]
    fn test_trigger_before_update_modify() {
        let (db, _dir) = make_arc_db();

        // Insert without triggers
        let col = db.collection("users").unwrap();
        col.put(sample_data("Alice", 30)).unwrap();

        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Update,
            crate::trigger::TriggerTiming::Before,
            None,
            std::sync::Arc::new(|_ctx| {
                let data = Value::Map(vec![
                    (Value::String("name".into()), Value::String("UPDATED".into())),
                    (Value::String("age".into()), Value::Integer(99.into())),
                ]);
                crate::trigger::TriggerResult::Proceed(Some(data))
            }),
        );

        let updated = db.trigger_update("users", 1, sample_data("Alice", 31)).unwrap();
        assert_eq!(
            updated.get_field("name"),
            Some(&Value::String("UPDATED".into()))
        );
    }

    #[test]
    fn test_trigger_before_update_abort() {
        let (db, _dir) = make_arc_db();

        let col = db.collection("users").unwrap();
        col.put(sample_data("Alice", 30)).unwrap();

        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Update,
            crate::trigger::TriggerTiming::Before,
            None,
            std::sync::Arc::new(|_ctx| {
                crate::trigger::TriggerResult::Abort("read only".to_string())
            }),
        );

        let result = db.trigger_update("users", 1, sample_data("Bob", 25));
        assert!(result.is_err());
        // Original data preserved
        let doc = db.collection("users").unwrap().get(1).unwrap();
        assert_eq!(
            doc.get_field("name"),
            Some(&Value::String("Alice".into()))
        );
    }

    #[test]
    fn test_trigger_after_update() {
        let (db, _dir) = make_arc_db();
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

        let col = db.collection("users").unwrap();
        col.put(sample_data("Alice", 30)).unwrap();

        let counter_clone = counter.clone();
        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Update,
            crate::trigger::TriggerTiming::After,
            None,
            std::sync::Arc::new(move |_ctx| {
                counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                crate::trigger::TriggerResult::Proceed(None)
            }),
        );

        db.trigger_update("users", 1, sample_data("Alice", 31)).unwrap();
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn test_trigger_before_delete_abort() {
        let (db, _dir) = make_arc_db();

        let col = db.collection("users").unwrap();
        col.put(sample_data("Alice", 30)).unwrap();

        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Delete,
            crate::trigger::TriggerTiming::Before,
            None,
            std::sync::Arc::new(|_ctx| {
                crate::trigger::TriggerResult::Abort("cannot delete".to_string())
            }),
        );

        let result = db.trigger_delete("users", 1);
        assert!(result.is_err());
        assert_eq!(db.collection("users").unwrap().count(), 1);
    }

    #[test]
    fn test_trigger_after_delete() {
        let (db, _dir) = make_arc_db();
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

        let col = db.collection("users").unwrap();
        col.put(sample_data("Alice", 30)).unwrap();

        let counter_clone = counter.clone();
        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Delete,
            crate::trigger::TriggerTiming::After,
            None,
            std::sync::Arc::new(move |_ctx| {
                counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                crate::trigger::TriggerResult::Proceed(None)
            }),
        );

        assert!(db.trigger_delete("users", 1).unwrap());
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn test_trigger_context_fields_insert() {
        let (db, _dir) = make_arc_db();
        let captured_event = std::sync::Arc::new(std::sync::Mutex::new(None));
        let captured_qn = std::sync::Arc::new(std::sync::Mutex::new(String::new()));

        let ce = captured_event.clone();
        let cq = captured_qn.clone();
        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Insert,
            crate::trigger::TriggerTiming::Before,
            None,
            std::sync::Arc::new(move |ctx| {
                *ce.lock().unwrap() = Some(ctx.event);
                *cq.lock().unwrap() = ctx.qualified_name.clone();
                assert!(ctx.old_record.is_none());
                assert!(ctx.new_record.is_some());
                assert!(!ctx.from_mesh);
                crate::trigger::TriggerResult::Proceed(None)
            }),
        );

        db.trigger_put("users", sample_data("Alice", 30)).unwrap();
        assert_eq!(
            *captured_event.lock().unwrap(),
            Some(crate::trigger::TriggerEvent::Insert)
        );
        assert_eq!(*captured_qn.lock().unwrap(), "public.users");
    }

    #[test]
    fn test_trigger_context_fields_update() {
        let (db, _dir) = make_arc_db();
        let had_old = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        let col = db.collection("users").unwrap();
        col.put(sample_data("Alice", 30)).unwrap();

        let had_old_clone = had_old.clone();
        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Update,
            crate::trigger::TriggerTiming::Before,
            None,
            std::sync::Arc::new(move |ctx| {
                had_old_clone.store(
                    ctx.old_record.is_some(),
                    std::sync::atomic::Ordering::SeqCst,
                );
                crate::trigger::TriggerResult::Proceed(None)
            }),
        );

        db.trigger_update("users", 1, sample_data("Alice", 31)).unwrap();
        assert!(had_old.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_trigger_context_fields_delete() {
        let (db, _dir) = make_arc_db();
        let had_old = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let had_new = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));

        let col = db.collection("users").unwrap();
        col.put(sample_data("Alice", 30)).unwrap();

        let ho = had_old.clone();
        let hn = had_new.clone();
        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Delete,
            crate::trigger::TriggerTiming::Before,
            None,
            std::sync::Arc::new(move |ctx| {
                ho.store(ctx.old_record.is_some(), std::sync::atomic::Ordering::SeqCst);
                hn.store(ctx.new_record.is_some(), std::sync::atomic::Ordering::SeqCst);
                crate::trigger::TriggerResult::Proceed(None)
            }),
        );

        db.trigger_delete("users", 1).unwrap();
        assert!(had_old.load(std::sync::atomic::Ordering::SeqCst));
        assert!(!had_new.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_trigger_execution_order() {
        let (db, _dir) = make_arc_db();
        let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        let o1 = order.clone();
        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Insert,
            crate::trigger::TriggerTiming::Before,
            Some("first".to_string()),
            std::sync::Arc::new(move |_ctx| {
                o1.lock().unwrap().push("before-1");
                crate::trigger::TriggerResult::Proceed(None)
            }),
        );
        let o2 = order.clone();
        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Insert,
            crate::trigger::TriggerTiming::Before,
            Some("second".to_string()),
            std::sync::Arc::new(move |_ctx| {
                o2.lock().unwrap().push("before-2");
                crate::trigger::TriggerResult::Proceed(None)
            }),
        );
        let o3 = order.clone();
        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Insert,
            crate::trigger::TriggerTiming::After,
            None,
            std::sync::Arc::new(move |_ctx| {
                o3.lock().unwrap().push("after-1");
                crate::trigger::TriggerResult::Proceed(None)
            }),
        );

        db.trigger_put("users", sample_data("Alice", 30)).unwrap();
        let captured = order.lock().unwrap();
        assert_eq!(*captured, vec!["before-1", "before-2", "after-1"]);
    }

    #[test]
    fn test_trigger_disabled_not_fired() {
        let (db, _dir) = make_arc_db();
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

        let cc = counter.clone();
        let id = db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Insert,
            crate::trigger::TriggerTiming::Before,
            None,
            std::sync::Arc::new(move |_ctx| {
                cc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                crate::trigger::TriggerResult::Proceed(None)
            }),
        );

        db.triggers().set_enabled(id, false);
        db.trigger_put("users", sample_data("Alice", 30)).unwrap();
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[test]
    fn test_trigger_clear_fires_delete_triggers() {
        let (db, _dir) = make_arc_db();
        let count = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

        let col = db.collection("users").unwrap();
        col.put(sample_data("Alice", 30)).unwrap();
        col.put(sample_data("Bob", 25)).unwrap();
        col.put(sample_data("Carol", 35)).unwrap();

        let cc = count.clone();
        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Delete,
            crate::trigger::TriggerTiming::After,
            None,
            std::sync::Arc::new(move |_ctx| {
                cc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                crate::trigger::TriggerResult::Proceed(None)
            }),
        );

        let cleared = db.trigger_clear("users").unwrap();
        assert_eq!(cleared, 3);
        assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[test]
    fn test_trigger_clear_abort_skips_record() {
        let (db, _dir) = make_arc_db();

        let col = db.collection("users").unwrap();
        col.put(sample_data("Alice", 30)).unwrap(); // id=1
        col.put(sample_data("Bob", 25)).unwrap(); // id=2

        // Abort deletion of id=1 only
        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Delete,
            crate::trigger::TriggerTiming::Before,
            None,
            std::sync::Arc::new(|ctx| {
                if let Some(ref old) = ctx.old_record {
                    if old.id == 1 {
                        return crate::trigger::TriggerResult::Abort("protected".to_string());
                    }
                }
                crate::trigger::TriggerResult::Proceed(None)
            }),
        );

        let cleared = db.trigger_clear("users").unwrap();
        assert_eq!(cleared, 1); // only id=2 deleted
        assert_eq!(db.collection("users").unwrap().count(), 1);
        assert!(db.collection("users").unwrap().get(1).is_ok()); // id=1 survived
    }

    #[test]
    fn test_trigger_multiple_before_modify_chain() {
        let (db, _dir) = make_arc_db();

        // First trigger appends "-1" to name
        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Insert,
            crate::trigger::TriggerTiming::Before,
            None,
            std::sync::Arc::new(|ctx| {
                if let Some(ref new_rec) = ctx.new_record {
                    let mut data = new_rec.data.clone();
                    if let Value::Map(ref mut map) = data {
                        for (k, v) in map.iter_mut() {
                            if k.as_str() == Some("name") {
                                if let Some(s) = v.as_str() {
                                    *v = Value::String(format!("{}-1", s).into());
                                }
                            }
                        }
                    }
                    crate::trigger::TriggerResult::Proceed(Some(data))
                } else {
                    crate::trigger::TriggerResult::Proceed(None)
                }
            }),
        );

        // Second trigger appends "-2" to name
        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Insert,
            crate::trigger::TriggerTiming::Before,
            None,
            std::sync::Arc::new(|ctx| {
                if let Some(ref new_rec) = ctx.new_record {
                    let mut data = new_rec.data.clone();
                    if let Value::Map(ref mut map) = data {
                        for (k, v) in map.iter_mut() {
                            if k.as_str() == Some("name") {
                                if let Some(s) = v.as_str() {
                                    *v = Value::String(format!("{}-2", s).into());
                                }
                            }
                        }
                    }
                    crate::trigger::TriggerResult::Proceed(Some(data))
                } else {
                    crate::trigger::TriggerResult::Proceed(None)
                }
            }),
        );

        let doc = db.trigger_put("users", sample_data("Alice", 30)).unwrap();
        assert_eq!(
            doc.get_field("name"),
            Some(&Value::String("Alice-1-2".into()))
        );
    }

    #[test]
    fn test_trigger_instead_skips_before_after() {
        let (db, _dir) = make_arc_db();
        let before_ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let after_ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        let br = before_ran.clone();
        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Insert,
            crate::trigger::TriggerTiming::Before,
            None,
            std::sync::Arc::new(move |_ctx| {
                br.store(true, std::sync::atomic::Ordering::SeqCst);
                crate::trigger::TriggerResult::Proceed(None)
            }),
        );

        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Insert,
            crate::trigger::TriggerTiming::Instead,
            None,
            std::sync::Arc::new(|_ctx| {
                crate::trigger::TriggerResult::Proceed(None)
            }),
        );

        let ar = after_ran.clone();
        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Insert,
            crate::trigger::TriggerTiming::After,
            None,
            std::sync::Arc::new(move |_ctx| {
                ar.store(true, std::sync::atomic::Ordering::SeqCst);
                crate::trigger::TriggerResult::Proceed(None)
            }),
        );

        db.trigger_put("users", sample_data("Alice", 30)).unwrap();
        assert!(!before_ran.load(std::sync::atomic::Ordering::SeqCst));
        assert!(!after_ran.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_trigger_reentrancy_depth_limit() {
        let (db, _dir) = make_arc_db();

        // Register a trigger that recursively inserts, causing reentrancy
        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Insert,
            crate::trigger::TriggerTiming::After,
            None,
            std::sync::Arc::new(|ctx| {
                if let Some(db) = ctx.db.upgrade() {
                    // This will recurse and eventually hit the depth limit
                    let _ = db.trigger_put("users", Value::Map(vec![
                        (Value::String("name".into()), Value::String("recursive".into())),
                    ]));
                }
                crate::trigger::TriggerResult::Proceed(None)
            }),
        );

        // Should not panic or hang — depth limit prevents infinite recursion
        let result = db.trigger_put("users", sample_data("Alice", 30));
        // The initial put succeeds (the abort happens in the recursive after-trigger)
        assert!(result.is_ok());
        // There should be some records but not infinite
        let count = db.collection("users").unwrap().count();
        assert!(count > 0 && count <= 10); // bounded by MAX_TRIGGER_DEPTH
    }

    #[test]
    fn test_trigger_put_detects_update_event() {
        let (db, _dir) = make_arc_db();
        let captured_event = std::sync::Arc::new(std::sync::Mutex::new(None));

        let col = db.collection("users").unwrap();
        col.put(sample_data("Alice", 30)).unwrap(); // id=1

        let ce = captured_event.clone();
        db.triggers().register(
            "public::users".to_string(),
            crate::trigger::TriggerEvent::Update,
            crate::trigger::TriggerTiming::Before,
            None,
            std::sync::Arc::new(move |ctx| {
                *ce.lock().unwrap() = Some(ctx.event);
                crate::trigger::TriggerResult::Proceed(None)
            }),
        );

        // put_with_id for existing record should fire Update event
        db.trigger_put_with_id("users", 1, sample_data("Bob", 25)).unwrap();
        assert_eq!(
            *captured_event.lock().unwrap(),
            Some(crate::trigger::TriggerEvent::Update)
        );
    }

    // ── Singleton Tests ──────────────────────────────────────────────

    fn singleton_defaults() -> Value {
        Value::Map(vec![
            (Value::String("theme".into()), Value::String("light".into())),
            (Value::String("lang".into()), Value::String("en".into())),
        ])
    }

    #[test]
    fn test_singleton_create_populates_default() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        db.singleton_collection("settings", singleton_defaults()).unwrap();

        let doc = db.singleton_get("settings").unwrap();
        assert_eq!(doc.id, 1);
        assert_eq!(doc.get_field("theme"), Some(&Value::String("light".into())));
    }

    #[test]
    fn test_singleton_put_upserts_id1() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        db.singleton_collection("settings", singleton_defaults()).unwrap();

        let new_data = Value::Map(vec![
            (Value::String("theme".into()), Value::String("dark".into())),
            (Value::String("lang".into()), Value::String("fr".into())),
        ]);
        let doc = db.singleton_put("settings", new_data).unwrap();
        assert_eq!(doc.id, 1);
        assert_eq!(doc.get_field("theme"), Some(&Value::String("dark".into())));

        // Count should still be 1
        let col = db.collection("settings").unwrap();
        assert_eq!(col.count(), 1);
    }

    #[test]
    fn test_singleton_delete_rejected() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        db.singleton_collection("settings", singleton_defaults()).unwrap();

        let result = db.trigger_delete("settings", 1);
        assert!(result.is_err());
        match result.unwrap_err() {
            NoSqlError::SingletonDeleteNotPermitted(_) => {}
            other => panic!("expected SingletonDeleteNotPermitted, got {:?}", other),
        }
    }

    #[test]
    fn test_singleton_clear_rejected() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        db.singleton_collection("settings", singleton_defaults()).unwrap();

        let result = db.trigger_clear("settings");
        assert!(result.is_err());
        match result.unwrap_err() {
            NoSqlError::SingletonClearNotPermitted(_) => {}
            other => panic!("expected SingletonClearNotPermitted, got {:?}", other),
        }
    }

    #[test]
    fn test_singleton_reset_restores_defaults() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        db.singleton_collection("settings", singleton_defaults()).unwrap();

        // Modify
        db.singleton_put("settings", Value::Map(vec![
            (Value::String("theme".into()), Value::String("dark".into())),
        ])).unwrap();

        // Reset
        let doc = db.singleton_reset("settings").unwrap();
        assert_eq!(doc.get_field("theme"), Some(&Value::String("light".into())));
        assert_eq!(doc.get_field("lang"), Some(&Value::String("en".into())));
    }

    #[test]
    fn test_singleton_persistence_across_reopen() {
        let dir = TempDir::new().unwrap();
        {
            let db = Database::open(dir.path()).unwrap();
            db.singleton_collection("settings", singleton_defaults()).unwrap();
            db.singleton_put("settings", Value::Map(vec![
                (Value::String("theme".into()), Value::String("dark".into())),
            ])).unwrap();
            db.flush().unwrap();
        }
        {
            let db = Database::open(dir.path()).unwrap();
            assert!(db.is_singleton("settings"));
            let doc = db.singleton_get("settings").unwrap();
            assert_eq!(doc.get_field("theme"), Some(&Value::String("dark".into())));

            // Reset should still work (defaults persisted)
            let doc = db.singleton_reset("settings").unwrap();
            assert_eq!(doc.get_field("theme"), Some(&Value::String("light".into())));
        }
    }

    #[test]
    fn test_singleton_idempotent_creation() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        db.singleton_collection("settings", singleton_defaults()).unwrap();
        // Second call should not error and not overwrite data
        db.singleton_put("settings", Value::Map(vec![
            (Value::String("theme".into()), Value::String("dark".into())),
        ])).unwrap();

        db.singleton_collection("settings", singleton_defaults()).unwrap();
        let doc = db.singleton_get("settings").unwrap();
        assert_eq!(doc.get_field("theme"), Some(&Value::String("dark".into())));
    }

    #[test]
    fn test_singleton_is_singleton() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        assert!(!db.is_singleton("settings"));
        db.singleton_collection("settings", singleton_defaults()).unwrap();
        assert!(db.is_singleton("settings"));
        assert!(!db.is_singleton("users"));
    }

    #[test]
    fn test_singleton_count_always_one() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        db.singleton_collection("settings", singleton_defaults()).unwrap();
        let col = db.collection("settings").unwrap();
        assert_eq!(col.count(), 1);

        db.singleton_put("settings", Value::Map(vec![
            (Value::String("x".into()), Value::Integer(1.into())),
        ])).unwrap();
        assert_eq!(col.count(), 1);
    }

    #[test]
    fn test_singleton_put_on_non_singleton_fails() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        db.collection("users").unwrap();
        let result = db.singleton_put("users", Value::Map(vec![]));
        assert!(result.is_err());
    }

    #[test]
    fn test_singleton_triggers_fire() {
        let dir = TempDir::new().unwrap();
        let db = Arc::new(Database::open(dir.path()).unwrap());
        db.set_self_ref();
        db.singleton_collection("settings", singleton_defaults()).unwrap();

        let fired = std::sync::Arc::new(Mutex::new(false));
        let f = fired.clone();
        db.triggers().register(
            "public::settings".to_string(),
            crate::trigger::TriggerEvent::Update,
            crate::trigger::TriggerTiming::After,
            Some("test_singleton_trigger".to_string()),
            std::sync::Arc::new(move |_ctx| {
                *f.lock().unwrap() = true;
                crate::trigger::TriggerResult::Proceed(None)
            }),
        );

        db.singleton_put("settings", Value::Map(vec![
            (Value::String("theme".into()), Value::String("dark".into())),
        ])).unwrap();
        assert!(*fired.lock().unwrap());
    }

    #[test]
    fn test_regular_collection_not_singleton() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(dir.path()).unwrap();
        db.collection("users").unwrap();
        assert!(!db.is_singleton("users"));
        // Delete should still work on regular collections
        let col = db.collection("users").unwrap();
        col.put(sample_data("Alice", 30)).unwrap();
        assert!(db.trigger_delete("users", 1).unwrap());
    }
}

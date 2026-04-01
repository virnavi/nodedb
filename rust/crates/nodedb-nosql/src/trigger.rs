use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use rmpv::Value;
use serde::{Deserialize, Serialize};

use crate::document::Document;

// ── Enums ───────────────────────────────────────────────────────────

/// The write event that fires a trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TriggerEvent {
    Insert,
    Update,
    Delete,
}

impl TriggerEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            TriggerEvent::Insert => "insert",
            TriggerEvent::Update => "update",
            TriggerEvent::Delete => "delete",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "insert" => Some(TriggerEvent::Insert),
            "update" => Some(TriggerEvent::Update),
            "delete" => Some(TriggerEvent::Delete),
            _ => None,
        }
    }
}

/// When the trigger fires relative to the write operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TriggerTiming {
    Before,
    After,
    Instead,
}

impl TriggerTiming {
    pub fn as_str(&self) -> &'static str {
        match self {
            TriggerTiming::Before => "before",
            TriggerTiming::After => "after",
            TriggerTiming::Instead => "instead",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "before" => Some(TriggerTiming::Before),
            "after" => Some(TriggerTiming::After),
            "instead" => Some(TriggerTiming::Instead),
            _ => None,
        }
    }
}

// ── TriggerContext ──────────────────────────────────────────────────

/// Context passed to trigger callback functions.
pub struct TriggerContext {
    /// Weak reference to the Database for side-effect access.
    pub db: std::sync::Weak<crate::database::Database>,

    /// Fully qualified collection name (e.g., "public.users").
    pub qualified_name: String,

    /// The event that triggered this callback.
    pub event: TriggerEvent,

    /// The document before the write (None for inserts).
    pub old_record: Option<Document>,

    /// The document after the write / proposed record (None for deletes).
    pub new_record: Option<Document>,

    /// Whether this write originated from a mesh peer.
    pub from_mesh: bool,

    /// If from_mesh is true, the name of the originating database.
    pub source_database_name: Option<String>,
}

// ── TriggerResult ──────────────────────────────────────────────────

/// The result returned by a trigger callback.
pub enum TriggerResult {
    /// Proceed with the write. Optionally provides modified data to
    /// replace the record being written (only meaningful for "before" triggers).
    Proceed(Option<Value>),

    /// Abort the write. The reason string is surfaced as a TriggerAbort error.
    Abort(String),

    /// Skip this trigger (no-op). Only valid for "after" triggers.
    Skip,
}

// ── Trigger callback type ──────────────────────────────────────────

/// A trigger callback function. Must be Send + Sync for thread safety.
pub type TriggerFn = Arc<dyn Fn(&TriggerContext) -> TriggerResult + Send + Sync>;

// ── MeshTriggerSource ──────────────────────────────────────────────

/// Identifies the remote source for a mesh trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshTriggerSource {
    pub source_database: String,
    pub source_collection: String,
}

// ── TriggerRegistration ────────────────────────────────────────────

static TRIGGER_ID_SEQ: AtomicU64 = AtomicU64::new(1);

/// A registered trigger with its metadata.
pub struct TriggerRegistration {
    /// Unique trigger ID.
    pub id: u64,
    /// The collection this trigger is bound to (meta_key format: "schema::collection").
    pub collection: String,
    /// The event this trigger fires on.
    pub event: TriggerEvent,
    /// When this trigger fires.
    pub timing: TriggerTiming,
    /// Whether this trigger is currently enabled.
    pub enabled: AtomicBool,
    /// Human-readable name (optional).
    pub name: Option<String>,
    /// If Some, this is a mesh trigger watching a remote database + collection.
    pub mesh_source: Option<MeshTriggerSource>,
    /// The callback.
    pub handler: TriggerFn,
}

impl TriggerRegistration {
    pub fn new(
        collection: String,
        event: TriggerEvent,
        timing: TriggerTiming,
        name: Option<String>,
        handler: TriggerFn,
    ) -> Self {
        TriggerRegistration {
            id: TRIGGER_ID_SEQ.fetch_add(1, Ordering::SeqCst),
            collection,
            event,
            timing,
            enabled: AtomicBool::new(true),
            name,
            mesh_source: None,
            handler,
        }
    }

    pub fn new_mesh(
        collection: String,
        event: TriggerEvent,
        timing: TriggerTiming,
        name: Option<String>,
        mesh_source: MeshTriggerSource,
        handler: TriggerFn,
    ) -> Self {
        TriggerRegistration {
            id: TRIGGER_ID_SEQ.fetch_add(1, Ordering::SeqCst),
            collection,
            event,
            timing,
            enabled: AtomicBool::new(true),
            name,
            mesh_source: Some(mesh_source),
            handler,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
}

impl fmt::Debug for TriggerRegistration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TriggerRegistration")
            .field("id", &self.id)
            .field("collection", &self.collection)
            .field("event", &self.event)
            .field("timing", &self.timing)
            .field("enabled", &self.is_enabled())
            .field("name", &self.name)
            .field("mesh_source", &self.mesh_source)
            .finish()
    }
}

// ── TriggerInfo ────────────────────────────────────────────────────

/// Serializable trigger information (no callback).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerInfo {
    pub id: u64,
    pub collection: String,
    pub event: String,
    pub timing: String,
    pub enabled: bool,
    pub name: Option<String>,
}

// ── TriggerRegistry ────────────────────────────────────────────────

/// Registry holding all registered triggers.
pub struct TriggerRegistry {
    triggers: RwLock<Vec<Arc<TriggerRegistration>>>,
}

impl TriggerRegistry {
    pub fn new() -> Self {
        TriggerRegistry {
            triggers: RwLock::new(Vec::new()),
        }
    }

    /// Register a new local trigger. Returns the trigger ID.
    pub fn register(
        &self,
        collection: String,
        event: TriggerEvent,
        timing: TriggerTiming,
        name: Option<String>,
        handler: TriggerFn,
    ) -> u64 {
        let reg = Arc::new(TriggerRegistration::new(
            collection, event, timing, name, handler,
        ));
        let id = reg.id;
        self.triggers.write().unwrap().push(reg);
        id
    }

    /// Register a mesh trigger watching a remote database + collection.
    pub fn register_mesh(
        &self,
        collection: String,
        event: TriggerEvent,
        timing: TriggerTiming,
        name: Option<String>,
        mesh_source: MeshTriggerSource,
        handler: TriggerFn,
    ) -> u64 {
        let reg = Arc::new(TriggerRegistration::new_mesh(
            collection, event, timing, name, mesh_source, handler,
        ));
        let id = reg.id;
        self.triggers.write().unwrap().push(reg);
        id
    }

    /// Unregister a trigger by ID. Returns true if found and removed.
    pub fn unregister(&self, id: u64) -> bool {
        let mut triggers = self.triggers.write().unwrap();
        let len_before = triggers.len();
        triggers.retain(|t| t.id != id);
        triggers.len() < len_before
    }

    /// List all triggers (serializable info, no callbacks).
    pub fn list(&self) -> Vec<TriggerInfo> {
        let triggers = self.triggers.read().unwrap();
        triggers
            .iter()
            .map(|t| TriggerInfo {
                id: t.id,
                collection: t.collection.clone(),
                event: t.event.as_str().to_string(),
                timing: t.timing.as_str().to_string(),
                enabled: t.is_enabled(),
                name: t.name.clone(),
            })
            .collect()
    }

    /// Set the enabled state of a trigger by ID. Returns true if found.
    pub fn set_enabled(&self, id: u64, enabled: bool) -> bool {
        let triggers = self.triggers.read().unwrap();
        if let Some(t) = triggers.iter().find(|t| t.id == id) {
            t.set_enabled(enabled);
            true
        } else {
            false
        }
    }

    /// Get all enabled local triggers matching a collection + event + timing,
    /// in registration order.
    pub fn matching(
        &self,
        collection: &str,
        event: TriggerEvent,
        timing: TriggerTiming,
    ) -> Vec<Arc<TriggerRegistration>> {
        let triggers = self.triggers.read().unwrap();
        triggers
            .iter()
            .filter(|t| {
                t.collection == collection
                    && t.event == event
                    && t.timing == timing
                    && t.is_enabled()
                    && t.mesh_source.is_none()
            })
            .cloned()
            .collect()
    }

    /// Get all enabled mesh triggers matching a source database + collection + event.
    pub fn matching_mesh(
        &self,
        source_database: &str,
        source_collection: &str,
        event: TriggerEvent,
    ) -> Vec<Arc<TriggerRegistration>> {
        let triggers = self.triggers.read().unwrap();
        triggers
            .iter()
            .filter(|t| {
                if let Some(ref ms) = t.mesh_source {
                    ms.source_database == source_database
                        && ms.source_collection == source_collection
                        && t.event == event
                        && t.is_enabled()
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noop_handler() -> TriggerFn {
        Arc::new(|_ctx| TriggerResult::Proceed(None))
    }

    #[test]
    fn test_trigger_event_as_str() {
        assert_eq!(TriggerEvent::Insert.as_str(), "insert");
        assert_eq!(TriggerEvent::Update.as_str(), "update");
        assert_eq!(TriggerEvent::Delete.as_str(), "delete");
    }

    #[test]
    fn test_trigger_event_from_str() {
        assert_eq!(TriggerEvent::from_str("insert"), Some(TriggerEvent::Insert));
        assert_eq!(TriggerEvent::from_str("update"), Some(TriggerEvent::Update));
        assert_eq!(TriggerEvent::from_str("delete"), Some(TriggerEvent::Delete));
        assert_eq!(TriggerEvent::from_str("unknown"), None);
    }

    #[test]
    fn test_trigger_timing_as_str() {
        assert_eq!(TriggerTiming::Before.as_str(), "before");
        assert_eq!(TriggerTiming::After.as_str(), "after");
        assert_eq!(TriggerTiming::Instead.as_str(), "instead");
    }

    #[test]
    fn test_trigger_timing_from_str() {
        assert_eq!(TriggerTiming::from_str("before"), Some(TriggerTiming::Before));
        assert_eq!(TriggerTiming::from_str("after"), Some(TriggerTiming::After));
        assert_eq!(TriggerTiming::from_str("instead"), Some(TriggerTiming::Instead));
        assert_eq!(TriggerTiming::from_str("unknown"), None);
    }

    #[test]
    fn test_trigger_registration_unique_ids() {
        let r1 = TriggerRegistration::new(
            "public::users".to_string(),
            TriggerEvent::Insert,
            TriggerTiming::Before,
            None,
            noop_handler(),
        );
        let r2 = TriggerRegistration::new(
            "public::users".to_string(),
            TriggerEvent::Insert,
            TriggerTiming::Before,
            None,
            noop_handler(),
        );
        assert_ne!(r1.id, r2.id);
    }

    #[test]
    fn test_trigger_registration_enable_disable() {
        let reg = TriggerRegistration::new(
            "public::users".to_string(),
            TriggerEvent::Insert,
            TriggerTiming::Before,
            None,
            noop_handler(),
        );
        assert!(reg.is_enabled());
        reg.set_enabled(false);
        assert!(!reg.is_enabled());
        reg.set_enabled(true);
        assert!(reg.is_enabled());
    }

    #[test]
    fn test_registry_register_and_list() {
        let registry = TriggerRegistry::new();
        let id1 = registry.register(
            "public::users".to_string(),
            TriggerEvent::Insert,
            TriggerTiming::Before,
            Some("validate".to_string()),
            noop_handler(),
        );
        let id2 = registry.register(
            "public::posts".to_string(),
            TriggerEvent::Delete,
            TriggerTiming::After,
            None,
            noop_handler(),
        );

        let list = registry.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, id1);
        assert_eq!(list[0].collection, "public::users");
        assert_eq!(list[0].event, "insert");
        assert_eq!(list[0].timing, "before");
        assert!(list[0].enabled);
        assert_eq!(list[0].name, Some("validate".to_string()));
        assert_eq!(list[1].id, id2);
    }

    #[test]
    fn test_registry_unregister() {
        let registry = TriggerRegistry::new();
        let id = registry.register(
            "public::users".to_string(),
            TriggerEvent::Insert,
            TriggerTiming::Before,
            None,
            noop_handler(),
        );
        assert!(registry.unregister(id));
        assert_eq!(registry.list().len(), 0);
        assert!(!registry.unregister(id)); // already removed
    }

    #[test]
    fn test_registry_matching_filters_correctly() {
        let registry = TriggerRegistry::new();
        registry.register(
            "public::users".to_string(),
            TriggerEvent::Insert,
            TriggerTiming::Before,
            None,
            noop_handler(),
        );
        registry.register(
            "public::users".to_string(),
            TriggerEvent::Insert,
            TriggerTiming::After,
            None,
            noop_handler(),
        );
        registry.register(
            "public::users".to_string(),
            TriggerEvent::Delete,
            TriggerTiming::Before,
            None,
            noop_handler(),
        );
        registry.register(
            "public::posts".to_string(),
            TriggerEvent::Insert,
            TriggerTiming::Before,
            None,
            noop_handler(),
        );

        let matches = registry.matching("public::users", TriggerEvent::Insert, TriggerTiming::Before);
        assert_eq!(matches.len(), 1);

        let matches = registry.matching("public::users", TriggerEvent::Insert, TriggerTiming::After);
        assert_eq!(matches.len(), 1);

        let matches = registry.matching("public::users", TriggerEvent::Delete, TriggerTiming::Before);
        assert_eq!(matches.len(), 1);

        let matches = registry.matching("public::users", TriggerEvent::Update, TriggerTiming::Before);
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_registry_matching_excludes_disabled() {
        let registry = TriggerRegistry::new();
        let id = registry.register(
            "public::users".to_string(),
            TriggerEvent::Insert,
            TriggerTiming::Before,
            None,
            noop_handler(),
        );

        assert_eq!(
            registry.matching("public::users", TriggerEvent::Insert, TriggerTiming::Before).len(),
            1
        );

        registry.set_enabled(id, false);

        assert_eq!(
            registry.matching("public::users", TriggerEvent::Insert, TriggerTiming::Before).len(),
            0
        );
    }

    #[test]
    fn test_registry_set_enabled() {
        let registry = TriggerRegistry::new();
        let id = registry.register(
            "public::users".to_string(),
            TriggerEvent::Insert,
            TriggerTiming::Before,
            None,
            noop_handler(),
        );

        assert!(registry.set_enabled(id, false));
        assert!(!registry.list()[0].enabled);

        assert!(registry.set_enabled(id, true));
        assert!(registry.list()[0].enabled);

        assert!(!registry.set_enabled(99999, false)); // not found
    }

    #[test]
    fn test_registry_matching_preserves_order() {
        let registry = TriggerRegistry::new();
        let id1 = registry.register(
            "public::users".to_string(),
            TriggerEvent::Insert,
            TriggerTiming::Before,
            Some("first".to_string()),
            noop_handler(),
        );
        let id2 = registry.register(
            "public::users".to_string(),
            TriggerEvent::Insert,
            TriggerTiming::Before,
            Some("second".to_string()),
            noop_handler(),
        );

        let matches = registry.matching("public::users", TriggerEvent::Insert, TriggerTiming::Before);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].id, id1);
        assert_eq!(matches[1].id, id2);
    }

    #[test]
    fn test_registry_matching_excludes_mesh_triggers() {
        let registry = TriggerRegistry::new();
        registry.register(
            "public::users".to_string(),
            TriggerEvent::Insert,
            TriggerTiming::After,
            None,
            noop_handler(),
        );
        registry.register_mesh(
            "public::users".to_string(),
            TriggerEvent::Insert,
            TriggerTiming::After,
            None,
            MeshTriggerSource {
                source_database: "warehouse".to_string(),
                source_collection: "public.users".to_string(),
            },
            noop_handler(),
        );

        // matching() excludes mesh triggers
        let local = registry.matching("public::users", TriggerEvent::Insert, TriggerTiming::After);
        assert_eq!(local.len(), 1);
    }

    #[test]
    fn test_registry_matching_mesh() {
        let registry = TriggerRegistry::new();
        registry.register_mesh(
            "public::cache".to_string(),
            TriggerEvent::Insert,
            TriggerTiming::After,
            None,
            MeshTriggerSource {
                source_database: "warehouse".to_string(),
                source_collection: "public.products".to_string(),
            },
            noop_handler(),
        );
        registry.register_mesh(
            "public::cache".to_string(),
            TriggerEvent::Update,
            TriggerTiming::After,
            None,
            MeshTriggerSource {
                source_database: "warehouse".to_string(),
                source_collection: "public.products".to_string(),
            },
            noop_handler(),
        );

        let matches = registry.matching_mesh("warehouse", "public.products", TriggerEvent::Insert);
        assert_eq!(matches.len(), 1);

        let matches = registry.matching_mesh("warehouse", "public.products", TriggerEvent::Update);
        assert_eq!(matches.len(), 1);

        let matches = registry.matching_mesh("other", "public.products", TriggerEvent::Insert);
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_trigger_info_serialization() {
        let info = TriggerInfo {
            id: 42,
            collection: "public::users".to_string(),
            event: "insert".to_string(),
            timing: "before".to_string(),
            enabled: true,
            name: Some("validate".to_string()),
        };
        let bytes = rmp_serde::to_vec(&info).unwrap();
        let decoded: TriggerInfo = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, 42);
        assert_eq!(decoded.collection, "public::users");
        assert_eq!(decoded.event, "insert");
        assert!(decoded.enabled);
    }
}

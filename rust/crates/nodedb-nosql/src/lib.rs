pub mod document;
pub mod schema;
pub mod error;
pub mod collection;
pub mod filter;
pub mod query;
pub mod database;
pub mod trigger;
pub mod conflict;
pub mod preferences;
pub mod access_history;
pub mod trim;
pub mod cache;

pub use document::Document;
pub use schema::{CollectionSchema, QualifiedName, SchemaEntry, SchemaMetadata, DEFAULT_SCHEMA, META_KEY_SEPARATOR};
pub use error::NoSqlError;
pub use collection::Collection;
pub use filter::{Filter, FilterCondition};
pub use query::Query;
pub use database::Database;
pub use trigger::{
    TriggerEvent, TriggerTiming, TriggerContext, TriggerResult,
    TriggerFn, TriggerRegistry, TriggerInfo, TriggerRegistration,
    MeshTriggerSource,
};
pub use conflict::{ConflictResolution, ConflictContext, ConflictOutcome, resolve_conflict};
pub use preferences::PreferencesStore;
pub use access_history::{AccessEventType, QueryScope, AccessHistoryConfig, AccessHistoryStore};
pub use trim::{TrimPolicy, TrimConfig, TrimReport, TrimRecommendation, TrimCandidate, TrimCollectionRecommendation, UserApprovedTrim};
pub use cache::{CacheMode, CacheConfig, RecordCacheStore};

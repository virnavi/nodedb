pub mod types;
pub mod query_handler;
pub mod merge;

use std::collections::HashMap;
use std::ffi::CString;
use std::panic::catch_unwind;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};

use nodedb_nosql::{Database, Query, Filter, FilterCondition};
use nodedb_nosql::query::{SortField, SortDirection};
use nodedb_graph::{GraphEngine, GraphError, DeleteBehaviour};
use nodedb_vector::{VectorEngine, VectorError, CollectionConfig, DistanceMetric};
use nodedb_dac::{DacEngine, DacError, AccessSubjectType, AccessPermission, DacSubject};
use nodedb_federation::{FederationEngine, FederationError, PeerStatus};
use nodedb_storage::StorageEngine;
use nodedb_transport::engine::TransportEngine;
use nodedb_transport::TransportError;
use nodedb_transport::QueryHandler;
use nodedb_crypto::NodeIdentity;
use nodedb_storage::{DbHeader, MigrationOp, MigrationRunner, OwnerKeyStatus};
use nodedb_provenance::{ProvenanceEngine, ProvenanceError, ProvenanceSourceType, ProvenanceVerificationStatus};
use nodedb_keyresolver::{KeyResolverEngine, KeyResolverError, KeyTrustLevel, KeyResolutionResult};
use nodedb_ai_provenance::{AiProvenanceEngine, AiProvenanceError, AiProvenanceConfig, AiProvenanceAssessment, AiConflictResolution, AiAnomalyFlag, AiSourceClassification, ConflictPreference};
use nodedb_ai_query::{AiQueryEngine, AiQueryError, AiQueryConfig, AiQueryResult, AiQuerySchema, SchemaPropertyType};
use types::*;

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

// Use a function to get the global handle map
fn handle_map() -> &'static RwLock<HashMap<DbHandle, Arc<Database>>> {
    use std::sync::OnceLock;
    static MAP: OnceLock<RwLock<HashMap<DbHandle, Arc<Database>>>> = OnceLock::new();
    MAP.get_or_init(|| RwLock::new(HashMap::new()))
}

fn graph_handle_map() -> &'static RwLock<HashMap<GraphHandle, Arc<GraphEngine>>> {
    use std::sync::OnceLock;
    static MAP: OnceLock<RwLock<HashMap<GraphHandle, Arc<GraphEngine>>>> = OnceLock::new();
    MAP.get_or_init(|| RwLock::new(HashMap::new()))
}

fn set_error(out_error: *mut NodeDbError, code: i32, msg: &str) {
    if !out_error.is_null() {
        unsafe {
            *out_error = NodeDbError::new(code, msg);
        }
    }
}

fn clear_error(out_error: *mut NodeDbError) {
    if !out_error.is_null() {
        unsafe {
            *out_error = NodeDbError::none();
        }
    }
}

fn get_db(handle: DbHandle) -> Option<Arc<Database>> {
    let map = handle_map().read().ok()?;
    map.get(&handle).cloned()
}

fn get_graph(handle: GraphHandle) -> Option<Arc<GraphEngine>> {
    let map = graph_handle_map().read().ok()?;
    map.get(&handle).cloned()
}

fn vector_handle_map() -> &'static RwLock<HashMap<VectorHandle, Arc<std::sync::Mutex<VectorEngine>>>> {
    use std::sync::OnceLock;
    static MAP: OnceLock<RwLock<HashMap<VectorHandle, Arc<std::sync::Mutex<VectorEngine>>>>> = OnceLock::new();
    MAP.get_or_init(|| RwLock::new(HashMap::new()))
}

fn get_vector(handle: VectorHandle) -> Option<Arc<std::sync::Mutex<VectorEngine>>> {
    let map = vector_handle_map().read().ok()?;
    map.get(&handle).cloned()
}

fn vector_error_code(e: &VectorError) -> i32 {
    match e {
        VectorError::VectorNotFound(_) => ERR_VECTOR_NOT_FOUND,
        VectorError::DimensionMismatch { .. } => ERR_VECTOR_DIMENSION_MISMATCH,
        VectorError::InvalidDimension(_) => ERR_VECTOR_DIMENSION_MISMATCH,
        VectorError::InvalidMetric(_) => ERR_INVALID_QUERY,
        VectorError::Index(_) => ERR_VECTOR_SEARCH,
        VectorError::Storage(_) => ERR_STORAGE,
        VectorError::Serialization(_) => ERR_SERIALIZATION,
        VectorError::Search(_) => ERR_VECTOR_SEARCH,
    }
}

fn graph_error_code(e: &GraphError) -> i32 {
    match e {
        GraphError::NodeNotFound(_) => ERR_GRAPH_NODE_NOT_FOUND,
        GraphError::EdgeNotFound(_) => ERR_GRAPH_EDGE_NOT_FOUND,
        GraphError::DeleteRestricted => ERR_GRAPH_DELETE_RESTRICTED,
        GraphError::InvalidSource(_) | GraphError::InvalidTarget(_) => ERR_INVALID_QUERY,
        GraphError::Storage(_) => ERR_STORAGE,
        GraphError::Serialization(_) => ERR_SERIALIZATION,
        GraphError::Traversal(_) => ERR_GRAPH_TRAVERSAL,
        GraphError::Algorithm(_) => ERR_GRAPH_TRAVERSAL,
    }
}

fn federation_handle_map() -> &'static RwLock<HashMap<FederationHandle, Arc<FederationEngine>>> {
    use std::sync::OnceLock;
    static MAP: OnceLock<RwLock<HashMap<FederationHandle, Arc<FederationEngine>>>> = OnceLock::new();
    MAP.get_or_init(|| RwLock::new(HashMap::new()))
}

fn get_federation(handle: FederationHandle) -> Option<Arc<FederationEngine>> {
    let map = federation_handle_map().read().ok()?;
    map.get(&handle).cloned()
}

fn federation_error_code(e: &FederationError) -> i32 {
    match e {
        FederationError::PeerNotFound(_) | FederationError::PeerNotFoundByName(_) => ERR_FEDERATION_PEER_NOT_FOUND,
        FederationError::GroupNotFound(_) | FederationError::GroupNotFoundByName(_) => ERR_FEDERATION_GROUP_NOT_FOUND,
        FederationError::DuplicatePeerName(_) | FederationError::DuplicateGroupName(_) => ERR_FEDERATION_DUPLICATE_NAME,
        FederationError::InvalidMemberPeer(_) => ERR_FEDERATION_INVALID_MEMBER,
        FederationError::Storage(_) => ERR_STORAGE,
        FederationError::Serialization(_) => ERR_SERIALIZATION,
    }
}

fn dac_handle_map() -> &'static RwLock<HashMap<DacHandle, Arc<DacEngine>>> {
    use std::sync::OnceLock;
    static MAP: OnceLock<RwLock<HashMap<DacHandle, Arc<DacEngine>>>> = OnceLock::new();
    MAP.get_or_init(|| RwLock::new(HashMap::new()))
}

fn get_dac(handle: DacHandle) -> Option<Arc<DacEngine>> {
    let map = dac_handle_map().read().ok()?;
    map.get(&handle).cloned()
}

fn dac_error_code(e: &DacError) -> i32 {
    match e {
        DacError::RuleNotFound(_) => ERR_DAC_RULE_NOT_FOUND,
        DacError::InvalidCollection(_) => ERR_DAC_INVALID_COLLECTION,
        DacError::InvalidDocument => ERR_DAC_INVALID_DOCUMENT,
        DacError::Storage(_) => ERR_STORAGE,
        DacError::Serialization(_) => ERR_SERIALIZATION,
    }
}

fn provenance_handle_map() -> &'static RwLock<HashMap<ProvenanceHandle, Arc<ProvenanceEngine>>> {
    use std::sync::OnceLock;
    static MAP: OnceLock<RwLock<HashMap<ProvenanceHandle, Arc<ProvenanceEngine>>>> = OnceLock::new();
    MAP.get_or_init(|| RwLock::new(HashMap::new()))
}

fn get_provenance(handle: ProvenanceHandle) -> Option<Arc<ProvenanceEngine>> {
    let map = provenance_handle_map().read().ok()?;
    map.get(&handle).cloned()
}

fn provenance_error_code(e: &ProvenanceError) -> i32 {
    match e {
        ProvenanceError::NotFound(_) => ERR_PROVENANCE_NOT_FOUND,
        ProvenanceError::InvalidConfidence(_) => ERR_PROVENANCE_INVALID_CONFIDENCE,
        ProvenanceError::Verification(_) => ERR_PROVENANCE_VERIFICATION,
        ProvenanceError::Canonical(_) => ERR_PROVENANCE_CANONICAL,
        ProvenanceError::Storage(_) => ERR_STORAGE,
    }
}

fn keyresolver_handle_map() -> &'static RwLock<HashMap<KeyResolverHandle, Arc<KeyResolverEngine>>> {
    use std::sync::OnceLock;
    static MAP: OnceLock<RwLock<HashMap<KeyResolverHandle, Arc<KeyResolverEngine>>>> = OnceLock::new();
    MAP.get_or_init(|| RwLock::new(HashMap::new()))
}

fn get_keyresolver(handle: KeyResolverHandle) -> Option<Arc<KeyResolverEngine>> {
    let map = keyresolver_handle_map().read().ok()?;
    map.get(&handle).cloned()
}

fn keyresolver_error_code(e: &KeyResolverError) -> i32 {
    match e {
        KeyResolverError::KeyNotFound(_, _) => ERR_KEYRESOLVER_NOT_FOUND,
        KeyResolverError::EntryNotFound(_) => ERR_KEYRESOLVER_ENTRY_NOT_FOUND,
        KeyResolverError::InvalidPublicKeyHex(_) => ERR_KEYRESOLVER_INVALID_HEX,
        KeyResolverError::KeyExpired(_, _) => ERR_KEYRESOLVER_EXPIRED,
        KeyResolverError::Storage(_) => ERR_STORAGE,
    }
}

/// Open a database. config is a MessagePack-encoded map with at minimum a "path" key.
/// Returns true on success, false on error.
#[no_mangle]
pub extern "C" fn nodedb_open(
    config_ptr: *const u8,
    config_len: usize,
    out_handle: *mut DbHandle,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if config_ptr.is_null() || out_handle.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let config_bytes = unsafe { std::slice::from_raw_parts(config_ptr, config_len) };

        // Decode config as MessagePack
        let config: rmpv::Value = match rmp_serde::from_slice(config_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &format!("invalid config: {}", e));
                return false;
            }
        };

        // Extract path
        let path_str = match config.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("path")))
            .and_then(|(_, v)| v.as_str())
        {
            Some(p) => p.to_string(),
            None => {
                set_error(out_error, ERR_SERIALIZATION, "config missing 'path' field");
                return false;
            }
        };

        let path = PathBuf::from(&path_str);

        // Optional owner_private_key_hex for keypair-bound encryption
        let owner_key_hex = config.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("owner_private_key_hex")))
            .and_then(|(_, v)| v.as_str())
            .map(|s| s.to_string());

        let db_result = if let Some(key_hex) = owner_key_hex {
            open_with_keypair(&path, &key_hex)
        } else {
            Database::open(&path).map_err(|e| e.to_string())
        };

        match db_result {
            Ok(db) => {
                let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
                let db = Arc::new(db);
                db.set_self_ref();
                let mut map = handle_map().write().unwrap();
                map.insert(handle, db);
                unsafe { *out_handle = handle; }
                clear_error(out_error);
                true
            }
            Err(e) => {
                set_error(out_error, ERR_STORAGE, &e);
                false
            }
        }
    });

    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_open");
            false
        }
    }
}

/// Close a database and release resources.
#[no_mangle]
pub extern "C" fn nodedb_close(handle: DbHandle) {
    let _ = catch_unwind(|| {
        let mut map = handle_map().write().unwrap();
        map.remove(&handle);
    });
}

/// Execute a query. request is a MessagePack-encoded query descriptor.
/// Response is a MessagePack-encoded result written to out_response.
#[no_mangle]
pub extern "C" fn nodedb_query(
    handle: DbHandle,
    request_ptr: *const u8,
    request_len: usize,
    out_response: *mut *mut u8,
    out_response_len: *mut usize,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if request_ptr.is_null() || out_response.is_null() || out_response_len.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let db = match get_db(handle) {
            Some(db) => db,
            None => {
                set_error(out_error, ERR_INVALID_HANDLE, "invalid database handle");
                return false;
            }
        };

        let request_bytes = unsafe { std::slice::from_raw_parts(request_ptr, request_len) };

        let request: rmpv::Value = match rmp_serde::from_slice(request_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &format!("invalid request: {}", e));
                return false;
            }
        };

        let action = request.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("action")))
            .and_then(|(_, v)| v.as_str())
            .unwrap_or("find_all");

        // Schema and trigger management actions (no collection needed)
        match action {
            "create_schema" | "drop_schema" | "list_schemas" | "schema_info"
            | "move_collection" | "rename_schema" | "schema_fingerprint"
            | "collection_names" | "collection_names_in_schema"
            | "register_trigger" | "register_mesh_trigger" | "unregister_trigger" | "list_triggers"
            | "set_trigger_enabled"
            | "singleton_create" | "singleton_get" | "singleton_put" | "singleton_reset" | "is_singleton"
            | "pref_get" | "pref_keys" | "pref_shareable" | "pref_set" | "pref_remove"
            | "access_history_query" | "access_history_count" | "access_history_last_access"
            | "access_history_trim"
            | "recommend_trim" | "trim" | "trim_all" | "trim_approved"
            | "trim_config_effective" | "trim_config_is_never_trim"
            | "trim_config_set" | "trim_config_reset"
            | "trim_config_set_record_never_trim" | "trim_config_clear_record_override"
            | "set_record_cache" | "get_record_cache" | "clear_record_cache"
            | "sweep_expired" | "sweep_all_expired"
            | "sync_version" => {
                let response_value: rmpv::Value = match action {
                    "create_schema" => {
                        let name = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("name")))
                            .and_then(|(_, v)| v.as_str())
                            .unwrap_or("");
                        let sharing_status = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("sharing_status")))
                            .and_then(|(_, v)| v.as_str());
                        match db.create_schema(name, sharing_status) {
                            Ok(()) => rmpv::Value::Boolean(true),
                            Err(e) => {
                                let code = match &e {
                                    nodedb_nosql::NoSqlError::ReservedSchemaWriteNotPermitted(_) |
                                    nodedb_nosql::NoSqlError::InvalidSchema(_) => ERR_RESERVED_SCHEMA_WRITE,
                                    _ => ERR_INVALID_QUERY,
                                };
                                set_error(out_error, code, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "drop_schema" => {
                        let name = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("name")))
                            .and_then(|(_, v)| v.as_str())
                            .unwrap_or("");
                        match db.drop_schema(name) {
                            Ok(dropped) => rmpv::Value::Boolean(dropped),
                            Err(e) => {
                                let code = match &e {
                                    nodedb_nosql::NoSqlError::ReservedSchemaWriteNotPermitted(_) => ERR_RESERVED_SCHEMA_WRITE,
                                    _ => ERR_INVALID_QUERY,
                                };
                                set_error(out_error, code, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "list_schemas" => {
                        let schemas = db.list_schemas();
                        let arr: Vec<rmpv::Value> = schemas.iter().map(|s| {
                            rmpv::Value::Map(vec![
                                (rmpv::Value::String("name".into()), rmpv::Value::String(s.name.clone().into())),
                                (rmpv::Value::String("sharing_status".into()), match &s.sharing_status {
                                    Some(ss) => rmpv::Value::String(ss.clone().into()),
                                    None => rmpv::Value::Nil,
                                }),
                            ])
                        }).collect();
                        rmpv::Value::Array(arr)
                    }
                    "schema_info" => {
                        let name = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("name")))
                            .and_then(|(_, v)| v.as_str())
                            .unwrap_or("");
                        match db.schema_info(name) {
                            Some(s) => rmpv::Value::Map(vec![
                                (rmpv::Value::String("name".into()), rmpv::Value::String(s.name.into())),
                                (rmpv::Value::String("sharing_status".into()), match s.sharing_status {
                                    Some(ss) => rmpv::Value::String(ss.into()),
                                    None => rmpv::Value::Nil,
                                }),
                            ]),
                            None => rmpv::Value::Nil,
                        }
                    }
                    "move_collection" => {
                        let from = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("from")))
                            .and_then(|(_, v)| v.as_str())
                            .unwrap_or("");
                        let to_schema = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("to_schema")))
                            .and_then(|(_, v)| v.as_str())
                            .unwrap_or("");
                        match db.move_collection(from, to_schema) {
                            Ok(()) => rmpv::Value::Boolean(true),
                            Err(e) => {
                                let code = match &e {
                                    nodedb_nosql::NoSqlError::ReservedSchemaWriteNotPermitted(_) => ERR_RESERVED_SCHEMA_WRITE,
                                    _ => ERR_STORAGE,
                                };
                                set_error(out_error, code, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "rename_schema" => {
                        let from = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("from")))
                            .and_then(|(_, v)| v.as_str())
                            .unwrap_or("");
                        let to = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("to")))
                            .and_then(|(_, v)| v.as_str())
                            .unwrap_or("");
                        match db.rename_schema(from, to) {
                            Ok(()) => rmpv::Value::Boolean(true),
                            Err(e) => {
                                let code = match &e {
                                    nodedb_nosql::NoSqlError::ReservedSchemaWriteNotPermitted(_) => ERR_RESERVED_SCHEMA_WRITE,
                                    _ => ERR_STORAGE,
                                };
                                set_error(out_error, code, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "schema_fingerprint" => {
                        let fp = db.schema_fingerprint();
                        rmpv::Value::String(fp.into())
                    }
                    "collection_names" => {
                        let names = db.collection_names();
                        rmpv::Value::Array(names.into_iter().map(|n| rmpv::Value::String(n.into())).collect())
                    }
                    "collection_names_in_schema" => {
                        let schema = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("schema")))
                            .and_then(|(_, v)| v.as_str())
                            .unwrap_or("public");
                        let names = db.collection_names_in_schema(schema);
                        rmpv::Value::Array(names.into_iter().map(|n| rmpv::Value::String(n.into())).collect())
                    }
                    "register_trigger" => {
                        let collection = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("collection")))
                            .and_then(|(_, v)| v.as_str())
                            .unwrap_or("").to_string();
                        let event_str = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("event")))
                            .and_then(|(_, v)| v.as_str())
                            .unwrap_or("");
                        let timing_str = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("timing")))
                            .and_then(|(_, v)| v.as_str())
                            .unwrap_or("");
                        let name = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("name")))
                            .and_then(|(_, v)| v.as_str())
                            .map(|s| s.to_string());

                        let event = match nodedb_nosql::TriggerEvent::from_str(event_str) {
                            Some(e) => e,
                            None => {
                                set_error(out_error, ERR_INVALID_QUERY, &format!("invalid trigger event: {}", event_str));
                                return false;
                            }
                        };
                        let timing = match nodedb_nosql::TriggerTiming::from_str(timing_str) {
                            Some(t) => t,
                            None => {
                                set_error(out_error, ERR_INVALID_QUERY, &format!("invalid trigger timing: {}", timing_str));
                                return false;
                            }
                        };

                        // Parse collection name to get meta_key
                        let qn = nodedb_nosql::QualifiedName::parse(&collection);
                        let meta_key = qn.meta_key();

                        let trigger_id = db.triggers().register(
                            meta_key,
                            event,
                            timing,
                            name,
                            std::sync::Arc::new(|_ctx| nodedb_nosql::TriggerResult::Proceed(None)),
                        );

                        rmpv::Value::Map(vec![
                            (rmpv::Value::String("trigger_id".into()), rmpv::Value::Integer(rmpv::Integer::from(trigger_id as i64))),
                        ])
                    }
                    "register_mesh_trigger" => {
                        let source_database = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("source_database")))
                            .and_then(|(_, v)| v.as_str())
                            .unwrap_or("").to_string();
                        let collection = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("collection")))
                            .and_then(|(_, v)| v.as_str())
                            .unwrap_or("").to_string();
                        let event_str = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("event")))
                            .and_then(|(_, v)| v.as_str())
                            .unwrap_or("");
                        let timing_str = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("timing")))
                            .and_then(|(_, v)| v.as_str())
                            .unwrap_or("after");
                        let name = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("name")))
                            .and_then(|(_, v)| v.as_str())
                            .map(|s| s.to_string());

                        let event = match nodedb_nosql::TriggerEvent::from_str(event_str) {
                            Some(e) => e,
                            None => {
                                set_error(out_error, ERR_INVALID_QUERY, &format!("invalid trigger event: {}", event_str));
                                return false;
                            }
                        };
                        let timing = match nodedb_nosql::TriggerTiming::from_str(timing_str) {
                            Some(t) => t,
                            None => {
                                set_error(out_error, ERR_INVALID_QUERY, &format!("invalid trigger timing: {}", timing_str));
                                return false;
                            }
                        };

                        let mesh_source = nodedb_nosql::MeshTriggerSource {
                            source_database: source_database,
                            source_collection: collection.clone(),
                        };

                        let trigger_id = db.triggers().register_mesh(
                            collection,
                            event,
                            timing,
                            name,
                            mesh_source,
                            std::sync::Arc::new(|_ctx| nodedb_nosql::TriggerResult::Proceed(None)),
                        );

                        rmpv::Value::Map(vec![
                            (rmpv::Value::String("trigger_id".into()), rmpv::Value::Integer(rmpv::Integer::from(trigger_id as i64))),
                        ])
                    }
                    "unregister_trigger" => {
                        let trigger_id = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("trigger_id")))
                            .and_then(|(_, v)| v.as_u64())
                            .unwrap_or(0);
                        let removed = db.triggers().unregister(trigger_id);
                        rmpv::Value::Map(vec![
                            (rmpv::Value::String("removed".into()), rmpv::Value::Boolean(removed)),
                        ])
                    }
                    "list_triggers" => {
                        let triggers = db.triggers().list();
                        let arr: Vec<rmpv::Value> = triggers.iter().map(|t| {
                            rmpv::Value::Map(vec![
                                (rmpv::Value::String("id".into()), rmpv::Value::Integer(rmpv::Integer::from(t.id as i64))),
                                (rmpv::Value::String("collection".into()), rmpv::Value::String(t.collection.clone().into())),
                                (rmpv::Value::String("event".into()), rmpv::Value::String(t.event.clone().into())),
                                (rmpv::Value::String("timing".into()), rmpv::Value::String(t.timing.clone().into())),
                                (rmpv::Value::String("enabled".into()), rmpv::Value::Boolean(t.enabled)),
                                (rmpv::Value::String("name".into()), match &t.name {
                                    Some(n) => rmpv::Value::String(n.clone().into()),
                                    None => rmpv::Value::Nil,
                                }),
                            ])
                        }).collect();
                        rmpv::Value::Array(arr)
                    }
                    "set_trigger_enabled" => {
                        let trigger_id = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("trigger_id")))
                            .and_then(|(_, v)| v.as_u64())
                            .unwrap_or(0);
                        let enabled = request.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("enabled")))
                            .and_then(|(_, v)| v.as_bool())
                            .unwrap_or(true);
                        let found = db.triggers().set_enabled(trigger_id, enabled);
                        rmpv::Value::Map(vec![
                            (rmpv::Value::String("found".into()), rmpv::Value::Boolean(found)),
                        ])
                    }

                    // ── Singleton Actions ──────────────────────────────────
                    "singleton_create" => {
                        let name = map_field(&request, "collection")
                            .or_else(|| map_field(&request, "name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("settings");
                        let defaults = map_field(&request, "defaults")
                            .cloned()
                            .unwrap_or(rmpv::Value::Map(vec![]));
                        match db.singleton_collection(name, defaults) {
                            Ok(_) => rmpv::Value::Map(vec![
                                (rmpv::Value::String("ok".into()), rmpv::Value::Boolean(true)),
                            ]),
                            Err(e) => {
                                set_error(out_error, ERR_STORAGE, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "singleton_get" => {
                        let name = map_field(&request, "collection")
                            .and_then(|v| v.as_str())
                            .unwrap_or("settings");
                        match db.singleton_get(name) {
                            Ok(doc) => {
                                rmpv::Value::Map(vec![
                                    (rmpv::Value::String("id".into()), rmpv::Value::Integer(doc.id.into())),
                                    (rmpv::Value::String("data".into()), doc.data.clone()),
                                    (rmpv::Value::String("updated_at".into()), rmpv::Value::String(doc.updated_at.to_rfc3339().into())),
                                ])
                            }
                            Err(e) => {
                                set_error(out_error, ERR_NOT_FOUND, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "singleton_put" => {
                        let name = map_field(&request, "collection")
                            .and_then(|v| v.as_str())
                            .unwrap_or("settings");
                        let data = map_field(&request, "data")
                            .cloned()
                            .unwrap_or(rmpv::Value::Map(vec![]));
                        match db.singleton_put(name, data) {
                            Ok(doc) => rmpv::Value::Map(vec![
                                (rmpv::Value::String("id".into()), rmpv::Value::Integer(doc.id.into())),
                                (rmpv::Value::String("data".into()), doc.data.clone()),
                            ]),
                            Err(e) => {
                                set_error(out_error, ERR_STORAGE, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "singleton_reset" => {
                        let name = map_field(&request, "collection")
                            .and_then(|v| v.as_str())
                            .unwrap_or("settings");
                        match db.singleton_reset(name) {
                            Ok(doc) => rmpv::Value::Map(vec![
                                (rmpv::Value::String("id".into()), rmpv::Value::Integer(doc.id.into())),
                                (rmpv::Value::String("data".into()), doc.data.clone()),
                            ]),
                            Err(e) => {
                                set_error(out_error, ERR_NOT_FOUND, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "is_singleton" => {
                        let name = map_field(&request, "collection")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        rmpv::Value::Map(vec![
                            (rmpv::Value::String("is_singleton".into()), rmpv::Value::Boolean(db.is_singleton(name))),
                        ])
                    }

                    // ── Preferences Actions ──────────────────────────────
                    "pref_set" => {
                        let store_name = map_field(&request, "store")
                            .or_else(|| map_field(&request, "name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("preferences");
                        let key = map_field(&request, "key")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let value = map_field(&request, "value")
                            .cloned()
                            .unwrap_or(rmpv::Value::Nil);
                        let shareable = map_field(&request, "shareable")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        let conflict_str = map_field(&request, "conflict_resolution")
                            .and_then(|v| v.as_str())
                            .unwrap_or("last_write_wins");
                        let conflict = nodedb_nosql::ConflictResolution::from_str(conflict_str)
                            .unwrap_or(nodedb_nosql::ConflictResolution::LastWriteWins);
                        match db.preferences(store_name) {
                            Ok(prefs) => match prefs.set(key, value, shareable, conflict) {
                                Ok(doc) => rmpv::Value::Map(vec![
                                    (rmpv::Value::String("id".into()), rmpv::Value::Integer(doc.id.into())),
                                    (rmpv::Value::String("ok".into()), rmpv::Value::Boolean(true)),
                                ]),
                                Err(e) => {
                                    set_error(out_error, ERR_PREFERENCE_ERROR, &e.to_string());
                                    return false;
                                }
                            },
                            Err(e) => {
                                set_error(out_error, ERR_PREFERENCE_ERROR, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "pref_get" => {
                        let store_name = map_field(&request, "store")
                            .or_else(|| map_field(&request, "name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("preferences");
                        let key = map_field(&request, "key")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        match db.preferences(store_name) {
                            Ok(prefs) => match prefs.get(key) {
                                Ok(Some(val)) => rmpv::Value::Map(vec![
                                    (rmpv::Value::String("value".into()), val),
                                    (rmpv::Value::String("found".into()), rmpv::Value::Boolean(true)),
                                ]),
                                Ok(None) => rmpv::Value::Map(vec![
                                    (rmpv::Value::String("found".into()), rmpv::Value::Boolean(false)),
                                ]),
                                Err(e) => {
                                    set_error(out_error, ERR_PREFERENCE_ERROR, &e.to_string());
                                    return false;
                                }
                            },
                            Err(e) => {
                                set_error(out_error, ERR_PREFERENCE_ERROR, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "pref_keys" => {
                        let store_name = map_field(&request, "store")
                            .or_else(|| map_field(&request, "name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("preferences");
                        match db.preferences(store_name) {
                            Ok(prefs) => match prefs.keys() {
                                Ok(keys) => {
                                    let arr: Vec<rmpv::Value> = keys.into_iter()
                                        .map(|k| rmpv::Value::String(k.into()))
                                        .collect();
                                    rmpv::Value::Map(vec![
                                        (rmpv::Value::String("keys".into()), rmpv::Value::Array(arr)),
                                    ])
                                }
                                Err(e) => {
                                    set_error(out_error, ERR_PREFERENCE_ERROR, &e.to_string());
                                    return false;
                                }
                            },
                            Err(e) => {
                                set_error(out_error, ERR_PREFERENCE_ERROR, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "pref_remove" => {
                        let store_name = map_field(&request, "store")
                            .or_else(|| map_field(&request, "name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("preferences");
                        let key = map_field(&request, "key")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        match db.preferences(store_name) {
                            Ok(prefs) => match prefs.remove(key) {
                                Ok(removed) => rmpv::Value::Map(vec![
                                    (rmpv::Value::String("removed".into()), rmpv::Value::Boolean(removed)),
                                ]),
                                Err(e) => {
                                    set_error(out_error, ERR_PREFERENCE_ERROR, &e.to_string());
                                    return false;
                                }
                            },
                            Err(e) => {
                                set_error(out_error, ERR_PREFERENCE_ERROR, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "pref_shareable" => {
                        let store_name = map_field(&request, "store")
                            .or_else(|| map_field(&request, "name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("preferences");
                        match db.preferences(store_name) {
                            Ok(prefs) => match prefs.shareable_entries() {
                                Ok(entries) => {
                                    let arr: Vec<rmpv::Value> = entries.into_iter()
                                        .map(|(key, doc)| rmpv::Value::Map(vec![
                                            (rmpv::Value::String("key".into()), rmpv::Value::String(key.into())),
                                            (rmpv::Value::String("id".into()), rmpv::Value::Integer(doc.id.into())),
                                            (rmpv::Value::String("data".into()), doc.data.clone()),
                                        ]))
                                        .collect();
                                    rmpv::Value::Map(vec![
                                        (rmpv::Value::String("entries".into()), rmpv::Value::Array(arr)),
                                    ])
                                }
                                Err(e) => {
                                    set_error(out_error, ERR_PREFERENCE_ERROR, &e.to_string());
                                    return false;
                                }
                            },
                            Err(e) => {
                                set_error(out_error, ERR_PREFERENCE_ERROR, &e.to_string());
                                return false;
                            }
                        }
                    }
                    // ── Access History ─────────────────────────────────
                    "access_history_query" => {
                        let collection = map_field(&request, "collection").and_then(|v| v.as_str());
                        let record_id = map_field(&request, "record_id").and_then(|v| v.as_i64());
                        let event_type = map_field(&request, "event_type")
                            .and_then(|v| v.as_str())
                            .and_then(nodedb_nosql::AccessEventType::from_str);
                        let limit = map_field(&request, "limit").and_then(|v| v.as_u64()).map(|v| v as usize);
                        match db.access_history().query_history(collection, record_id, event_type, None, limit) {
                            Ok(entries) => rmpv::Value::Array(entries),
                            Err(e) => {
                                set_error(out_error, ERR_ACCESS_HISTORY, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "access_history_count" => {
                        match db.access_history().count() {
                            Ok(count) => rmpv::Value::Integer((count as i64).into()),
                            Err(e) => {
                                set_error(out_error, ERR_ACCESS_HISTORY, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "access_history_last_access" => {
                        let collection = map_field(&request, "collection")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let record_id = map_field(&request, "record_id")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        match db.access_history().last_access_time(collection, record_id) {
                            Ok(Some(ts)) => rmpv::Value::String(ts.to_rfc3339().into()),
                            Ok(None) => rmpv::Value::Nil,
                            Err(e) => {
                                set_error(out_error, ERR_ACCESS_HISTORY, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "access_history_trim" => {
                        let retention_secs = map_field(&request, "retention_secs")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(365 * 24 * 3600);
                        match db.access_history().trim_old_entries(retention_secs) {
                            Ok(deleted) => rmpv::Value::Integer((deleted as i64).into()),
                            Err(e) => {
                                set_error(out_error, ERR_ACCESS_HISTORY, &e.to_string());
                                return false;
                            }
                        }
                    }
                    // ── Trim ──────────────────────────────────────────────
                    "recommend_trim" => {
                        let policy_val = map_field(&request, "policy");
                        let policy = match policy_val {
                            Some(v) => match nodedb_nosql::TrimPolicy::from_value(v) {
                                Ok(p) => p,
                                Err(e) => {
                                    set_error(out_error, ERR_TRIM_POLICY_INVALID, &e.to_string());
                                    return false;
                                }
                            },
                            None => {
                                set_error(out_error, ERR_TRIM_POLICY_INVALID, "missing 'policy'");
                                return false;
                            }
                        };
                        let exclude: Vec<String> = map_field(&request, "exclude_collections")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_default();
                        match db.recommend_trim(&policy, &exclude) {
                            Ok(rec) => rec.to_value(),
                            Err(e) => {
                                set_error(out_error, ERR_ACCESS_HISTORY, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "trim" => {
                        let collection = map_field(&request, "collection")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let policy_val = map_field(&request, "policy");
                        let policy = match policy_val {
                            Some(v) => match nodedb_nosql::TrimPolicy::from_value(v) {
                                Ok(p) => p,
                                Err(e) => {
                                    set_error(out_error, ERR_TRIM_POLICY_INVALID, &e.to_string());
                                    return false;
                                }
                            },
                            None => {
                                set_error(out_error, ERR_TRIM_POLICY_INVALID, "missing 'policy'");
                                return false;
                            }
                        };
                        let dry_run = map_field(&request, "dry_run")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        match db.trim(collection, &policy, dry_run) {
                            Ok(report) => report.to_value(),
                            Err(e) => {
                                let code = match &e {
                                    nodedb_nosql::NoSqlError::TrimNeverTrim(_) => ERR_TRIM_NEVER_TRIM,
                                    nodedb_nosql::NoSqlError::TrimAborted(_) => ERR_TRIM_ABORTED,
                                    _ => ERR_ACCESS_HISTORY,
                                };
                                set_error(out_error, code, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "trim_all" => {
                        let policy_val = map_field(&request, "policy");
                        let policy = match policy_val {
                            Some(v) => match nodedb_nosql::TrimPolicy::from_value(v) {
                                Ok(p) => p,
                                Err(e) => {
                                    set_error(out_error, ERR_TRIM_POLICY_INVALID, &e.to_string());
                                    return false;
                                }
                            },
                            None => {
                                set_error(out_error, ERR_TRIM_POLICY_INVALID, "missing 'policy'");
                                return false;
                            }
                        };
                        let dry_run = map_field(&request, "dry_run")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        match db.trim_all(&policy, dry_run) {
                            Ok(report) => report.to_value(),
                            Err(e) => {
                                let code = match &e {
                                    nodedb_nosql::NoSqlError::TrimNeverTrim(_) => ERR_TRIM_NEVER_TRIM,
                                    nodedb_nosql::NoSqlError::TrimAborted(_) => ERR_TRIM_ABORTED,
                                    _ => ERR_ACCESS_HISTORY,
                                };
                                set_error(out_error, code, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "trim_approved" => {
                        let approval = match nodedb_nosql::UserApprovedTrim::from_value(&request) {
                            Ok(a) => a,
                            Err(e) => {
                                set_error(out_error, ERR_TRIM_POLICY_INVALID, &e.to_string());
                                return false;
                            }
                        };
                        match db.trim_approved(&approval) {
                            Ok(report) => report.to_value(),
                            Err(e) => {
                                set_error(out_error, ERR_ACCESS_HISTORY, &e.to_string());
                                return false;
                            }
                        }
                    }
                    // ── Trim Config ───────────────────────────────────────
                    "trim_config_effective" => {
                        let collection = map_field(&request, "collection")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let qn = nodedb_nosql::QualifiedName::parse(collection);
                        let meta_key = qn.meta_key();
                        match db.trim_config().get_collection_override(&meta_key) {
                            Ok(Some(policy)) => policy.to_value(),
                            Ok(None) => rmpv::Value::Nil,
                            Err(e) => {
                                set_error(out_error, ERR_ACCESS_HISTORY, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "trim_config_is_never_trim" => {
                        let collection = map_field(&request, "collection")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let qn = nodedb_nosql::QualifiedName::parse(collection);
                        let meta_key = qn.meta_key();
                        match db.trim_config().is_never_trim(&meta_key) {
                            Ok(v) => rmpv::Value::Boolean(v),
                            Err(e) => {
                                set_error(out_error, ERR_ACCESS_HISTORY, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "trim_config_set" => {
                        let collection = map_field(&request, "collection")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let policy_val = map_field(&request, "policy");
                        let policy = match policy_val {
                            Some(v) => match nodedb_nosql::TrimPolicy::from_value(v) {
                                Ok(p) => p,
                                Err(e) => {
                                    set_error(out_error, ERR_TRIM_POLICY_INVALID, &e.to_string());
                                    return false;
                                }
                            },
                            None => {
                                set_error(out_error, ERR_TRIM_POLICY_INVALID, "missing 'policy'");
                                return false;
                            }
                        };
                        let qn = nodedb_nosql::QualifiedName::parse(collection);
                        let meta_key = qn.meta_key();
                        match db.trim_config().set_trim_policy(&meta_key, &policy) {
                            Ok(()) => rmpv::Value::Boolean(true),
                            Err(e) => {
                                set_error(out_error, ERR_ACCESS_HISTORY, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "trim_config_reset" => {
                        let collection = map_field(&request, "collection")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let qn = nodedb_nosql::QualifiedName::parse(collection);
                        let meta_key = qn.meta_key();
                        match db.trim_config().reset_to_annotation_default(&meta_key) {
                            Ok(()) => rmpv::Value::Boolean(true),
                            Err(e) => {
                                set_error(out_error, ERR_ACCESS_HISTORY, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "trim_config_set_record_never_trim" => {
                        let collection = map_field(&request, "collection")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let record_id = map_field(&request, "record_id")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        let qn = nodedb_nosql::QualifiedName::parse(collection);
                        let meta_key = qn.meta_key();
                        match db.trim_config().set_record_never_trim(&meta_key, record_id) {
                            Ok(()) => rmpv::Value::Boolean(true),
                            Err(e) => {
                                set_error(out_error, ERR_ACCESS_HISTORY, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "trim_config_clear_record_override" => {
                        let collection = map_field(&request, "collection")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let record_id = map_field(&request, "record_id")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        let qn = nodedb_nosql::QualifiedName::parse(collection);
                        let meta_key = qn.meta_key();
                        match db.trim_config().clear_record_override(&meta_key, record_id) {
                            Ok(()) => rmpv::Value::Boolean(true),
                            Err(e) => {
                                set_error(out_error, ERR_ACCESS_HISTORY, &e.to_string());
                                return false;
                            }
                        }
                    }

                    // ── Record Cache actions ─────────────────────────────
                    "set_record_cache" => {
                        let collection = map_field(&request, "collection")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let record_id = map_field(&request, "record_id")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        let cache_val = map_field(&request, "cache");
                        let config = match cache_val {
                            Some(v) => match nodedb_nosql::CacheConfig::from_value(v) {
                                Ok(c) => c,
                                Err(e) => {
                                    set_error(out_error, ERR_CACHE_CONFIG_INVALID, &e.to_string());
                                    return false;
                                }
                            },
                            None => {
                                set_error(out_error, ERR_CACHE_CONFIG_INVALID, "missing 'cache'");
                                return false;
                            }
                        };
                        match db.set_record_cache(collection, record_id, &config) {
                            Ok(()) => rmpv::Value::Boolean(true),
                            Err(e) => {
                                set_error(out_error, ERR_STORAGE, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "get_record_cache" => {
                        let collection = map_field(&request, "collection")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let record_id = map_field(&request, "record_id")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        match db.get_record_cache(collection, record_id) {
                            Ok(Some(config)) => config.to_value(),
                            Ok(None) => rmpv::Value::Nil,
                            Err(e) => {
                                set_error(out_error, ERR_STORAGE, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "clear_record_cache" => {
                        let collection = map_field(&request, "collection")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let record_id = map_field(&request, "record_id")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        match db.clear_record_cache(collection, record_id) {
                            Ok(()) => rmpv::Value::Boolean(true),
                            Err(e) => {
                                set_error(out_error, ERR_STORAGE, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "sweep_expired" => {
                        let collection = map_field(&request, "collection")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        match db.sweep_expired(collection) {
                            Ok(count) => rmpv::Value::Integer(rmpv::Integer::from(count as i64)),
                            Err(e) => {
                                set_error(out_error, ERR_STORAGE, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "sweep_all_expired" => {
                        match db.sweep_all_expired() {
                            Ok(count) => rmpv::Value::Integer(rmpv::Integer::from(count as i64)),
                            Err(e) => {
                                set_error(out_error, ERR_STORAGE, &e.to_string());
                                return false;
                            }
                        }
                    }
                    "sync_version" => {
                        rmpv::Value::Integer(rmpv::Integer::from(db.sync_version() as i64))
                    }
                    _ => unreachable!(),
                };

                let response_bytes = match rmp_serde::to_vec(&response_value) {
                    Ok(b) => b,
                    Err(e) => {
                        set_error(out_error, ERR_SERIALIZATION, &e.to_string());
                        return false;
                    }
                };

                let len = response_bytes.len();
                let ptr = response_bytes.as_ptr();
                std::mem::forget(response_bytes);

                unsafe {
                    *out_response = ptr as *mut u8;
                    *out_response_len = len;
                }

                clear_error(out_error);
                return true;
            }
            _ => {} // Fall through to collection-based actions
        }

        // Extract collection name and query parameters
        let collection_name = match request.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("collection")))
            .and_then(|(_, v)| v.as_str())
        {
            Some(c) => c.to_string(),
            None => {
                set_error(out_error, ERR_INVALID_QUERY, "missing 'collection' field");
                return false;
            }
        };

        let collection = match db.collection(&collection_name) {
            Ok(c) => c,
            Err(e) => {
                set_error(out_error, ERR_NOT_FOUND, &e.to_string());
                return false;
            }
        };

        let response_value: rmpv::Value = match action {
            "get" => {
                let id = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("id")))
                    .and_then(|(_, v)| v.as_i64())
                    .unwrap_or(0);
                match collection.get(id) {
                    Ok(doc) => {
                        // Lazy eviction: check if record has expired cache config
                        let qn = nodedb_nosql::QualifiedName::parse(&collection_name);
                        let mk = qn.meta_key();
                        if let Ok(Some(cache_cfg)) = db.record_cache().get(&mk, doc.id) {
                            if cache_cfg.is_expired(&doc, chrono::Utc::now()) {
                                // Delete expired record and its cache entry
                                let _ = collection.delete(doc.id);
                                let _ = db.record_cache().remove(&mk, doc.id);
                                set_error(out_error, ERR_NOT_FOUND, &format!("document not found: id={}", id));
                                return false;
                            }
                        }
                        match rmpv::ext::to_value(&doc) {
                            Ok(v) => v,
                            Err(e) => {
                                set_error(out_error, ERR_SERIALIZATION, &e.to_string());
                                return false;
                            }
                        }
                    }
                    Err(e) => {
                        set_error(out_error, ERR_NOT_FOUND, &e.to_string());
                        return false;
                    }
                }
            }
            "count" => {
                rmpv::Value::Integer(rmpv::Integer::from(collection.count() as i64))
            }
            "query" => {
                // Parse the Query from the "query" field map
                let query_val = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("query")))
                    .map(|(_, v)| v.clone())
                    .unwrap_or(rmpv::Value::Nil);
                let query: Query = if query_val.is_nil() {
                    Query::default()
                } else {
                    match parse_query_from_value(&query_val) {
                        Ok(q) => q,
                        Err(e) => {
                            set_error(out_error, ERR_INVALID_QUERY, &format!("invalid query: {}", e));
                            return false;
                        }
                    }
                };
                match collection.query(&query) {
                    Ok(docs) => {
                        match rmpv::ext::to_value(&docs) {
                            Ok(v) => v,
                            Err(e) => {
                                set_error(out_error, ERR_SERIALIZATION, &e.to_string());
                                return false;
                            }
                        }
                    }
                    Err(e) => {
                        set_error(out_error, ERR_STORAGE, &e.to_string());
                        return false;
                    }
                }
            }
            "clear" => {
                match collection.clear() {
                    Ok(count) => {
                        rmpv::Value::Integer(rmpv::Integer::from(count as i64))
                    }
                    Err(e) => {
                        set_error(out_error, ERR_STORAGE, &e.to_string());
                        return false;
                    }
                }
            }
            "batch_put" => {
                // Fast bulk insert/update using atomic sled::Batch.
                // Bypasses triggers, notifications, and access history.
                let items = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("items")))
                    .and_then(|(_, v)| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let parsed: Vec<(i64, rmpv::Value)> = items.iter().map(|item| {
                    let id = item.as_map()
                        .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("id")))
                        .and_then(|(_, v)| v.as_i64())
                        .unwrap_or(0);
                    let data = item.as_map()
                        .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("data")))
                        .map(|(_, v)| v.clone())
                        .unwrap_or(rmpv::Value::Nil);
                    (id, data)
                }).collect();
                match collection.batch_put_with_ids(&parsed) {
                    Ok(count) => rmpv::Value::Integer(rmpv::Integer::from(count as i64)),
                    Err(e) => {
                        set_error(out_error, ERR_STORAGE, &e.to_string());
                        return false;
                    }
                }
            }
            "batch_delete" => {
                // Fast bulk delete using atomic sled::Batch.
                // Bypasses triggers, notifications, and access history.
                let ids: Vec<i64> = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("ids")))
                    .and_then(|(_, v)| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
                    .unwrap_or_default();
                match collection.batch_delete(&ids) {
                    Ok(count) => rmpv::Value::Integer(rmpv::Integer::from(count as i64)),
                    Err(e) => {
                        set_error(out_error, ERR_STORAGE, &e.to_string());
                        return false;
                    }
                }
            }
            "find_all" | _ => {
                let offset = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("offset")))
                    .and_then(|(_, v)| v.as_u64())
                    .map(|v| v as usize);
                let limit = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("limit")))
                    .and_then(|(_, v)| v.as_u64())
                    .map(|v| v as usize);

                match collection.find_all(offset, limit) {
                    Ok(docs) => {
                        match rmpv::ext::to_value(&docs) {
                            Ok(v) => v,
                            Err(e) => {
                                set_error(out_error, ERR_SERIALIZATION, &e.to_string());
                                return false;
                            }
                        }
                    }
                    Err(e) => {
                        set_error(out_error, ERR_STORAGE, &e.to_string());
                        return false;
                    }
                }
            }
        };

        // Record access history for read operations (best-effort, ignore errors)
        if db.should_record_access(&collection_name) {
            match action {
                "get" => {
                    let id = request.as_map()
                        .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("id")))
                        .and_then(|(_, v)| v.as_i64())
                        .unwrap_or(0);
                    let _ = db.access_history().record(
                        &collection_name, id,
                        nodedb_nosql::AccessEventType::Read,
                        "local", nodedb_nosql::QueryScope::Local,
                        None, false,
                    );
                }
                "query" | "find_all" => {
                    // Record access for each returned document
                    if let Some(arr) = response_value.as_array() {
                        let ids: Vec<i64> = arr.iter()
                            .filter_map(|v| {
                                // Documents are serialized as positional arrays by rmpv::ext::to_value
                                // The first field is id (i64)
                                v.as_array().and_then(|a| a.first()).and_then(|id| id.as_i64())
                            })
                            .collect();
                        let _ = db.access_history().record_batch(
                            &collection_name, &ids,
                            nodedb_nosql::AccessEventType::Read,
                            "local", nodedb_nosql::QueryScope::Local,
                        );
                    }
                }
                _ => {} // count doesn't need recording
            }
        }

        // Serialize response
        let response_bytes = match rmp_serde::to_vec(&response_value) {
            Ok(b) => b,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &e.to_string());
                return false;
            }
        };

        let len = response_bytes.len();
        let ptr = response_bytes.as_ptr();
        std::mem::forget(response_bytes);

        unsafe {
            *out_response = ptr as *mut u8;
            *out_response_len = len;
        }

        clear_error(out_error);
        true
    });

    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_query");
            false
        }
    }
}

/// Execute a write transaction. ops is a MessagePack-encoded array of operations.
#[no_mangle]
pub extern "C" fn nodedb_write_txn(
    handle: DbHandle,
    ops_ptr: *const u8,
    ops_len: usize,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if ops_ptr.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let db = match get_db(handle) {
            Some(db) => db,
            None => {
                set_error(out_error, ERR_INVALID_HANDLE, "invalid database handle");
                return false;
            }
        };

        let ops_bytes = unsafe { std::slice::from_raw_parts(ops_ptr, ops_len) };

        let ops: rmpv::Value = match rmp_serde::from_slice(ops_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &format!("invalid ops: {}", e));
                return false;
            }
        };

        let ops_array = match ops.as_array() {
            Some(a) => a,
            None => {
                set_error(out_error, ERR_SERIALIZATION, "ops must be an array");
                return false;
            }
        };

        for op in ops_array {
            let collection_name = match op.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("collection")))
                .and_then(|(_, v)| v.as_str())
            {
                Some(c) => c.to_string(),
                None => {
                    set_error(out_error, ERR_INVALID_QUERY, "op missing 'collection'");
                    return false;
                }
            };

            let action = match op.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("action")))
                .and_then(|(_, v)| v.as_str())
            {
                Some(a) => a.to_string(),
                None => {
                    set_error(out_error, ERR_INVALID_QUERY, "op missing 'action'");
                    return false;
                }
            };

            // Check if any triggers are registered for this collection (fast path)
            let meta_key = nodedb_nosql::QualifiedName::parse(&collection_name).meta_key();
            let has_triggers = !db.triggers().matching(
                &meta_key,
                nodedb_nosql::TriggerEvent::Insert,
                nodedb_nosql::TriggerTiming::Before,
            ).is_empty()
                || !db.triggers().matching(
                    &meta_key,
                    nodedb_nosql::TriggerEvent::Update,
                    nodedb_nosql::TriggerTiming::Before,
                ).is_empty()
                || !db.triggers().matching(
                    &meta_key,
                    nodedb_nosql::TriggerEvent::Delete,
                    nodedb_nosql::TriggerTiming::Before,
                ).is_empty();

            match action.as_str() {
                "put" => {
                    let data = op.as_map()
                        .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("data")))
                        .map(|(_, v)| v.clone())
                        .unwrap_or(rmpv::Value::Nil);
                    let id = op.as_map()
                        .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("id")))
                        .and_then(|(_, v)| v.as_i64())
                        .unwrap_or(0);

                    // Parse optional cache config
                    let cache_config = op.as_map()
                        .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("cache")))
                        .and_then(|(_, v)| if v.is_nil() { None } else { Some(v) })
                        .map(|v| nodedb_nosql::CacheConfig::from_value(v));

                    // Validate cache config early
                    let cache_config = match cache_config {
                        Some(Ok(c)) => Some(c),
                        Some(Err(e)) => {
                            set_error(out_error, ERR_CACHE_CONFIG_INVALID, &e.to_string());
                            return false;
                        }
                        None => None,
                    };

                    let written_doc_id: i64;

                    if has_triggers {
                        // Slow path: check for old doc, fire triggers, emit notifications
                        let old_doc = db.collection(&collection_name)
                            .ok()
                            .and_then(|c| c.get(id).ok());
                        let event_str = if old_doc.is_some() { "update" } else { "insert" };
                        match db.trigger_put_with_id(&collection_name, id, data) {
                            Ok(new_doc) => {
                                written_doc_id = new_doc.id;
                                emit_trigger_notification(
                                    handle, &collection_name, event_str,
                                    old_doc.as_ref(), Some(&new_doc),
                                );
                                if db.should_record_access(&collection_name) {
                                    let _ = db.access_history().record(
                                        &collection_name, new_doc.id,
                                        nodedb_nosql::AccessEventType::Write,
                                        "local", nodedb_nosql::QueryScope::Local,
                                        None, false,
                                    );
                                }
                            }
                            Err(e) => {
                                let code = match &e {
                                    nodedb_nosql::NoSqlError::TriggerAbort(_) => ERR_TRIGGER_ABORT,
                                    nodedb_nosql::NoSqlError::ReservedSchemaWriteNotPermitted(_) => ERR_RESERVED_SCHEMA_WRITE,
                                    _ => ERR_STORAGE,
                                };
                                set_error(out_error, code, &e.to_string());
                                return false;
                            }
                        }
                    } else {
                        // Fast path: direct collection write, no trigger overhead
                        match db.collection(&collection_name) {
                            Ok(col) => {
                                match col.put_with_id(id, data) {
                                    Ok(doc) => {
                                        written_doc_id = doc.id;
                                        db.increment_sync_version();
                                    }
                                    Err(e) => {
                                        set_error(out_error, ERR_STORAGE, &e.to_string());
                                        return false;
                                    }
                                }
                            }
                            Err(e) => {
                                set_error(out_error, ERR_STORAGE, &e.to_string());
                                return false;
                            }
                        }
                    }

                    // Apply cache config if provided
                    if let Some(config) = cache_config {
                        if let Err(e) = db.set_record_cache(&collection_name, written_doc_id, &config) {
                            set_error(out_error, ERR_STORAGE, &e.to_string());
                            return false;
                        }
                    }
                }
                "delete" => {
                    let id = op.as_map()
                        .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("id")))
                        .and_then(|(_, v)| v.as_i64())
                        .unwrap_or(0);

                    if has_triggers {
                        // Slow path: fetch old doc, fire triggers, emit notifications
                        let old_doc = db.collection(&collection_name)
                            .ok()
                            .and_then(|c| c.get(id).ok());
                        match db.trigger_delete(&collection_name, id) {
                            Ok(_) => {
                                emit_trigger_notification(
                                    handle, &collection_name, "delete",
                                    old_doc.as_ref(), None,
                                );
                                if db.should_record_access(&collection_name) {
                                    let _ = db.access_history().record(
                                        &collection_name, id,
                                        nodedb_nosql::AccessEventType::Write,
                                        "local", nodedb_nosql::QueryScope::Local,
                                        None, false,
                                    );
                                }
                            }
                            Err(e) => {
                                let code = match &e {
                                    nodedb_nosql::NoSqlError::TriggerAbort(_) => ERR_TRIGGER_ABORT,
                                    nodedb_nosql::NoSqlError::SingletonDeleteNotPermitted(_) => ERR_SINGLETON_DELETE,
                                    nodedb_nosql::NoSqlError::SingletonClearNotPermitted(_) => ERR_SINGLETON_CLEAR,
                                    nodedb_nosql::NoSqlError::ReservedSchemaWriteNotPermitted(_) => ERR_RESERVED_SCHEMA_WRITE,
                                    _ => ERR_STORAGE,
                                };
                                set_error(out_error, code, &e.to_string());
                                return false;
                            }
                        }
                    } else {
                        // Fast path: direct collection delete, no trigger overhead
                        // Still check singleton guard
                        if db.is_singleton(&collection_name) {
                            set_error(out_error, ERR_SINGLETON_DELETE,
                                &format!("cannot delete from singleton collection: {}", collection_name));
                            return false;
                        }
                        match db.collection(&collection_name) {
                            Ok(col) => {
                                match col.delete(id) {
                                    Ok(true) => { db.increment_sync_version(); }
                                    Ok(false) => {}
                                    Err(e) => {
                                        set_error(out_error, ERR_STORAGE, &e.to_string());
                                        return false;
                                    }
                                }
                            }
                            Err(e) => {
                                set_error(out_error, ERR_STORAGE, &e.to_string());
                                return false;
                            }
                        }
                    }
                }
                other => {
                    set_error(out_error, ERR_INVALID_QUERY, &format!("unknown action: {}", other));
                    return false;
                }
            }
        }

        clear_error(out_error);
        true
    });

    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_write_txn");
            false
        }
    }
}

/// Free a buffer previously returned by nodedb_query.
#[no_mangle]
pub extern "C" fn nodedb_free_buffer(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(ptr, len, len);
        }
    }
}

/// Return the FFI version number. Encoding: major*1_000_000 + minor*1_000 + patch.
/// v0.0.1 = 1.
#[no_mangle]
pub extern "C" fn nodedb_ffi_version() -> u32 {
    1
}

/// Free an error message string.
#[no_mangle]
pub extern "C" fn nodedb_free_error(error: *mut NodeDbError) {
    if !error.is_null() {
        unsafe {
            let err = &mut *error;
            if !err.message.is_null() {
                let _ = CString::from_raw(err.message);
                err.message = std::ptr::null_mut();
            }
        }
    }
}

// =============================================================================
// Database-level actions (owner key, migrations)
// =============================================================================

/// Open a database with owner keypair binding.
/// On first open: generates DEK, seals to owner public key, writes header.
/// On subsequent opens: verifies fingerprint, unseals DEK or enters mismatch mode.
fn open_with_keypair(path: &std::path::Path, private_key_hex: &str) -> Result<Database, String> {
    // Decode the private key hex
    let key_bytes = hex_decode_32(private_key_hex)
        .map_err(|e| format!("invalid owner_private_key_hex: {}", e))?;

    // Derive the public key and fingerprint from the private key
    let identity = NodeIdentity::from_signing_key_bytes(&key_bytes)
        .map_err(|e| format!("invalid signing key: {}", e))?;
    let public_key_bytes = identity.verifying_key_bytes();
    let expected_fingerprint = nodedb_crypto::fingerprint(&public_key_bytes);

    // Open the raw storage engine (no encryption yet) to check/write header
    let mut engine = StorageEngine::open(path)
        .map_err(|e| format!("storage open failed: {}", e))?;

    let header = engine.get_db_header()
        .map_err(|e| format!("header read failed: {}", e))?;

    match header {
        Some(h) => {
            // Existing database — verify fingerprint
            if h.owner_fingerprint == expected_fingerprint {
                // Unseal DEK
                let dek = nodedb_crypto::unseal_dek(&key_bytes, &h.sealed_dek)
                    .map_err(|e| format!("DEK unseal failed: {}", e))?;
                engine.set_dek(dek);
                // OwnerKeyStatus::Verified is now implicit (dek is set)
            } else {
                // Key mismatch — silent inaccessibility
                engine.set_mismatch(true);
            }
        }
        None => {
            // First open — generate DEK and seal it
            let mut dek = [0u8; 32];
            rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut dek);
            let sealed = nodedb_crypto::seal_dek(&public_key_bytes, &dek)
                .map_err(|e| format!("DEK seal failed: {}", e))?;
            let new_header = DbHeader {
                sealed_dek: sealed,
                owner_fingerprint: expected_fingerprint,
                db_version: 0,
                database_name: None,
            };
            engine.put_db_header(&new_header)
                .map_err(|e| format!("header write failed: {}", e))?;
            engine.flush().map_err(|e| format!("flush failed: {}", e))?;
            engine.set_dek(dek);
        }
    }

    Database::open_with_engine(path, Arc::new(engine))
        .map_err(|e| format!("database open failed: {}", e))
}

/// Decode a hex string to a 32-byte array.
fn hex_decode_32(hex: &str) -> Result<[u8; 32], String> {
    if hex.len() != 64 {
        return Err(format!("expected 64 hex chars, got {}", hex.len()));
    }
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|e| format!("invalid hex at pos {}: {}", i * 2, e))?;
    }
    Ok(bytes)
}

/// Encode a byte slice as hex string.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Parse a Query from a rmpv::Value map (as sent from Dart/client).
/// Expects a map with optional keys: "filter", "sort", "offset", "limit".
fn parse_query_from_value(val: &rmpv::Value) -> Result<Query, String> {
    let map = val.as_map().ok_or("query must be a map")?;

    let mut query = Query::new();

    for (k, v) in map {
        match k.as_str().unwrap_or("") {
            "filter" => {
                if !v.is_nil() {
                    query.filter = Some(parse_filter(v)?);
                }
            }
            "sort" => {
                if let Some(arr) = v.as_array() {
                    for item in arr {
                        let field = item.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("field")))
                            .and_then(|(_, v)| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let dir = item.as_map()
                            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("direction")))
                            .and_then(|(_, v)| v.as_str())
                            .unwrap_or("Asc");
                        let direction = match dir {
                            "Desc" | "desc" => SortDirection::Desc,
                            _ => SortDirection::Asc,
                        };
                        query.sort.push(SortField { field, direction });
                    }
                }
            }
            "offset" => {
                if let Some(n) = v.as_u64() {
                    query.offset = Some(n as usize);
                }
            }
            "limit" => {
                if let Some(n) = v.as_u64() {
                    query.limit = Some(n as usize);
                }
            }
            _ => {}
        }
    }

    Ok(query)
}

/// Parse a Filter from a rmpv::Value map.
/// Expects: {"Condition": {"GreaterThan": {"field": "age", "value": 25}}}
///      or: {"And": [filter1, filter2, ...]}
///      or: {"Or": [filter1, filter2, ...]}
fn parse_filter(val: &rmpv::Value) -> Result<Filter, String> {
    let map = val.as_map().ok_or("filter must be a map")?;
    if map.is_empty() {
        return Err("empty filter map".into());
    }

    let (key, inner) = &map[0];
    let key_str = key.as_str().ok_or("filter key must be a string")?;

    match key_str {
        "Condition" => {
            let cond = parse_filter_condition(inner)?;
            Ok(Filter::Condition(cond))
        }
        "And" => {
            let arr = inner.as_array().ok_or("And must contain an array")?;
            let filters: Result<Vec<Filter>, String> = arr.iter().map(parse_filter).collect();
            Ok(Filter::And(filters?))
        }
        "Or" => {
            let arr = inner.as_array().ok_or("Or must contain an array")?;
            let filters: Result<Vec<Filter>, String> = arr.iter().map(parse_filter).collect();
            Ok(Filter::Or(filters?))
        }
        _ => Err(format!("unknown filter type: {}", key_str)),
    }
}

/// Parse a FilterCondition from a rmpv::Value map.
/// Expects: {"GreaterThan": {"field": "age", "value": 25}}
fn parse_filter_condition(val: &rmpv::Value) -> Result<FilterCondition, String> {
    let map = val.as_map().ok_or("condition must be a map")?;
    if map.is_empty() {
        return Err("empty condition map".into());
    }

    let (key, inner) = &map[0];
    let key_str = key.as_str().ok_or("condition key must be a string")?;

    let get_field = |v: &rmpv::Value| -> String {
        v.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("field")))
            .and_then(|(_, v)| v.as_str())
            .unwrap_or("")
            .to_string()
    };

    let get_value = |v: &rmpv::Value| -> rmpv::Value {
        v.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("value")))
            .map(|(_, v)| v.clone())
            .unwrap_or(rmpv::Value::Nil)
    };

    let get_str_value = |v: &rmpv::Value| -> String {
        v.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("value")))
            .and_then(|(_, v)| v.as_str())
            .unwrap_or("")
            .to_string()
    };

    match key_str {
        "EqualTo" => Ok(FilterCondition::EqualTo { field: get_field(inner), value: get_value(inner) }),
        "NotEqualTo" => Ok(FilterCondition::NotEqualTo { field: get_field(inner), value: get_value(inner) }),
        "GreaterThan" => Ok(FilterCondition::GreaterThan { field: get_field(inner), value: get_value(inner) }),
        "GreaterThanOrEqual" => Ok(FilterCondition::GreaterThanOrEqual { field: get_field(inner), value: get_value(inner) }),
        "LessThan" => Ok(FilterCondition::LessThan { field: get_field(inner), value: get_value(inner) }),
        "LessThanOrEqual" => Ok(FilterCondition::LessThanOrEqual { field: get_field(inner), value: get_value(inner) }),
        "Contains" => Ok(FilterCondition::Contains { field: get_field(inner), value: get_str_value(inner) }),
        "StartsWith" => Ok(FilterCondition::StartsWith { field: get_field(inner), value: get_str_value(inner) }),
        "EndsWith" => Ok(FilterCondition::EndsWith { field: get_field(inner), value: get_str_value(inner) }),
        "IsNull" => Ok(FilterCondition::IsNull { field: get_field(inner) }),
        "IsNotNull" => Ok(FilterCondition::IsNotNull { field: get_field(inner) }),
        "Between" => {
            let field = get_field(inner);
            let low = inner.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("low")))
                .map(|(_, v)| v.clone())
                .unwrap_or(rmpv::Value::Nil);
            let high = inner.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("high")))
                .map(|(_, v)| v.clone())
                .unwrap_or(rmpv::Value::Nil);
            Ok(FilterCondition::Between { field, low, high })
        }
        "JsonPathEquals" => {
            let field = get_field(inner);
            let path = inner.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("path")))
                .and_then(|(_, v)| v.as_str())
                .unwrap_or("")
                .to_string();
            let value = get_value(inner);
            Ok(FilterCondition::JsonPathEquals { field, path, value })
        }
        "JsonHasKey" => {
            let field = get_field(inner);
            let path = inner.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("path")))
                .and_then(|(_, v)| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(FilterCondition::JsonHasKey { field, path })
        }
        "JsonContains" => Ok(FilterCondition::JsonContains {
            field: get_field(inner),
            value: get_value(inner),
        }),
        "ArrayContains" => Ok(FilterCondition::ArrayContains {
            field: get_field(inner),
            value: get_value(inner),
        }),
        "ArrayOverlap" => {
            let field = get_field(inner);
            let values = inner.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("values")))
                .and_then(|(_, v)| v.as_array())
                .map(|arr| arr.to_vec())
                .unwrap_or_default();
            Ok(FilterCondition::ArrayOverlap { field, values })
        }
        _ => Err(format!("unknown condition type: {}", key_str)),
    }
}

/// Execute a database-level operation (not collection-specific).
/// Actions: owner_key_status, rotate_owner_key
#[no_mangle]
pub extern "C" fn nodedb_db_execute(
    handle: DbHandle,
    request_ptr: *const u8,
    request_len: usize,
    out_response: *mut *mut u8,
    out_response_len: *mut usize,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if request_ptr.is_null() || out_response.is_null() || out_response_len.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let db = match get_db(handle) {
            Some(db) => db,
            None => {
                set_error(out_error, ERR_INVALID_HANDLE, "invalid database handle");
                return false;
            }
        };

        let request_bytes = unsafe { std::slice::from_raw_parts(request_ptr, request_len) };
        let request: rmpv::Value = match rmp_serde::from_slice(request_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &format!("invalid request: {}", e));
                return false;
            }
        };

        let action = match request.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("action")))
            .and_then(|(_, v)| v.as_str())
        {
            Some(a) => a,
            None => {
                set_error(out_error, ERR_INVALID_QUERY, "missing 'action' field");
                return false;
            }
        };

        let response_value: rmpv::Value = match action {
            "owner_key_status" => {
                let status = match db.owner_key_status() {
                    OwnerKeyStatus::Verified => "verified",
                    OwnerKeyStatus::Mismatch => "mismatch",
                    OwnerKeyStatus::Unbound => "unbound",
                };
                rmpv::Value::Map(vec![
                    (rmpv::Value::String("status".into()), rmpv::Value::String(status.into())),
                ])
            }
            "rotate_owner_key" => {
                let current_key_hex = match request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("current_private_key_hex")))
                    .and_then(|(_, v)| v.as_str())
                {
                    Some(k) => k,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'current_private_key_hex'");
                        return false;
                    }
                };
                let new_key_hex = match request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("new_private_key_hex")))
                    .and_then(|(_, v)| v.as_str())
                {
                    Some(k) => k,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'new_private_key_hex'");
                        return false;
                    }
                };

                // Verify current key
                let current_bytes = match hex_decode_32(current_key_hex) {
                    Ok(b) => b,
                    Err(e) => {
                        set_error(out_error, ERR_INVALID_QUERY, &format!("invalid current key: {}", e));
                        return false;
                    }
                };

                // Read current header
                let header = match db.engine().get_db_header() {
                    Ok(Some(h)) => h,
                    Ok(None) => {
                        set_error(out_error, ERR_STORAGE, "no database header (unbound database)");
                        return false;
                    }
                    Err(e) => {
                        set_error(out_error, ERR_STORAGE, &e.to_string());
                        return false;
                    }
                };

                // Unseal with current key to get DEK
                let dek = match nodedb_crypto::unseal_dek(&current_bytes, &header.sealed_dek) {
                    Ok(d) => d,
                    Err(e) => {
                        set_error(out_error, ERR_STORAGE, &format!("current key mismatch: {}", e));
                        return false;
                    }
                };

                // Derive new public key and seal DEK under it
                let new_bytes = match hex_decode_32(new_key_hex) {
                    Ok(b) => b,
                    Err(e) => {
                        set_error(out_error, ERR_INVALID_QUERY, &format!("invalid new key: {}", e));
                        return false;
                    }
                };
                let new_identity = match NodeIdentity::from_signing_key_bytes(&new_bytes) {
                    Ok(id) => id,
                    Err(e) => {
                        set_error(out_error, ERR_INVALID_QUERY, &format!("invalid new signing key: {}", e));
                        return false;
                    }
                };
                let new_public = new_identity.verifying_key_bytes();
                let new_fingerprint = nodedb_crypto::fingerprint(&new_public);
                let new_sealed = match nodedb_crypto::seal_dek(&new_public, &dek) {
                    Ok(s) => s,
                    Err(e) => {
                        set_error(out_error, ERR_STORAGE, &format!("seal failed: {}", e));
                        return false;
                    }
                };

                // Write updated header
                let new_header = DbHeader {
                    sealed_dek: new_sealed,
                    owner_fingerprint: new_fingerprint.clone(),
                    db_version: header.db_version,
                    database_name: header.database_name.clone(),
                };
                if let Err(e) = db.engine().put_db_header(&new_header) {
                    set_error(out_error, ERR_STORAGE, &e.to_string());
                    return false;
                }
                if let Err(e) = db.engine().flush() {
                    set_error(out_error, ERR_STORAGE, &e.to_string());
                    return false;
                }

                rmpv::Value::Map(vec![
                    (rmpv::Value::String("status".into()), rmpv::Value::String("rotated".into())),
                    (rmpv::Value::String("new_fingerprint".into()), rmpv::Value::String(new_fingerprint.into())),
                ])
            }
            "migrate" => {
                let target_version = match request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("target_version")))
                    .and_then(|(_, v)| v.as_u64())
                {
                    Some(v) => v as u32,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'target_version'");
                        return false;
                    }
                };

                let ops_array = match request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("operations")))
                    .and_then(|(_, v)| v.as_array())
                {
                    Some(a) => a,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'operations' array");
                        return false;
                    }
                };

                let mut ops = Vec::new();
                for op_val in ops_array {
                    let op_type = op_val.as_map()
                        .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("type")))
                        .and_then(|(_, v)| v.as_str())
                        .unwrap_or("");

                    match op_type {
                        "rename_tree" => {
                            let from = op_val.as_map()
                                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("from")))
                                .and_then(|(_, v)| v.as_str())
                                .unwrap_or("").to_string();
                            let to = op_val.as_map()
                                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("to")))
                                .and_then(|(_, v)| v.as_str())
                                .unwrap_or("").to_string();
                            ops.push(MigrationOp::RenameTree { from, to });
                        }
                        "drop_tree" => {
                            let name = op_val.as_map()
                                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("name")))
                                .and_then(|(_, v)| v.as_str())
                                .unwrap_or("").to_string();
                            ops.push(MigrationOp::DropTree(name));
                        }
                        other => {
                            set_error(out_error, ERR_INVALID_QUERY, &format!("unknown migration op: {}", other));
                            return false;
                        }
                    }
                }

                if let Err(e) = MigrationRunner::run(db.engine(), target_version, ops) {
                    set_error(out_error, ERR_STORAGE, &e.to_string());
                    return false;
                }

                rmpv::Value::Map(vec![
                    (rmpv::Value::String("status".into()), rmpv::Value::String("migrated".into())),
                    (rmpv::Value::String("version".into()), rmpv::Value::Integer((target_version as i64).into())),
                ])
            }
            "generate_keypair" => {
                let identity = NodeIdentity::generate();
                let private_hex = hex_encode(&identity.signing_key_bytes());
                let public_hex = hex_encode(&identity.verifying_key_bytes());
                rmpv::Value::Map(vec![
                    (rmpv::Value::String("private_key_hex".into()), rmpv::Value::String(private_hex.into())),
                    (rmpv::Value::String("public_key_hex".into()), rmpv::Value::String(public_hex.into())),
                ])
            }
            "sign" => {
                let private_key_hex = match request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("private_key_hex")))
                    .and_then(|(_, v)| v.as_str())
                {
                    Some(k) => k.to_string(),
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'private_key_hex'");
                        return false;
                    }
                };
                let payload_utf8 = match request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("payload_utf8")))
                    .and_then(|(_, v)| v.as_str())
                {
                    Some(p) => p.to_string(),
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'payload_utf8'");
                        return false;
                    }
                };
                let key_bytes = match hex_decode_32(&private_key_hex) {
                    Ok(b) => b,
                    Err(e) => {
                        set_error(out_error, ERR_INVALID_QUERY, &format!("invalid key: {}", e));
                        return false;
                    }
                };
                let identity = match NodeIdentity::from_signing_key_bytes(&key_bytes) {
                    Ok(id) => id,
                    Err(e) => {
                        set_error(out_error, ERR_INVALID_QUERY, &format!("invalid signing key: {}", e));
                        return false;
                    }
                };
                let sig_bytes = identity.sign(payload_utf8.as_bytes());
                let sig_hex = hex_encode(&sig_bytes);
                rmpv::Value::Map(vec![
                    (rmpv::Value::String("signature_hex".into()), rmpv::Value::String(sig_hex.into())),
                ])
            }
            other => {
                set_error(out_error, ERR_INVALID_QUERY, &format!("unknown db action: {}", other));
                return false;
            }
        };

        let response_bytes = match rmp_serde::to_vec(&response_value) {
            Ok(b) => b,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &e.to_string());
                return false;
            }
        };
        let len = response_bytes.len();
        let ptr = response_bytes.as_ptr();
        std::mem::forget(response_bytes);
        unsafe {
            *out_response = ptr as *mut u8;
            *out_response_len = len;
        }
        clear_error(out_error);
        true
    });

    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_db_execute");
            false
        }
    }
}

// =============================================================================
// Graph FFI
// =============================================================================

/// Open a graph engine. config is a MessagePack-encoded map with a "path" key.
#[no_mangle]
pub extern "C" fn nodedb_graph_open(
    config_ptr: *const u8,
    config_len: usize,
    out_handle: *mut GraphHandle,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if config_ptr.is_null() || out_handle.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let config_bytes = unsafe { std::slice::from_raw_parts(config_ptr, config_len) };

        let config: rmpv::Value = match rmp_serde::from_slice(config_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &format!("invalid config: {}", e));
                return false;
            }
        };

        let path_str = match config.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("path")))
            .and_then(|(_, v)| v.as_str())
        {
            Some(p) => p.to_string(),
            None => {
                set_error(out_error, ERR_SERIALIZATION, "config missing 'path' field");
                return false;
            }
        };

        let path = PathBuf::from(&path_str);
        let storage = match StorageEngine::open(&path) {
            Ok(e) => Arc::new(e),
            Err(e) => {
                set_error(out_error, ERR_STORAGE, &e.to_string());
                return false;
            }
        };

        match GraphEngine::new(storage) {
            Ok(graph) => {
                let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
                let mut map = graph_handle_map().write().unwrap();
                map.insert(handle, Arc::new(graph));
                unsafe { *out_handle = handle; }
                clear_error(out_error);
                true
            }
            Err(e) => {
                set_error(out_error, ERR_STORAGE, &e.to_string());
                false
            }
        }
    });

    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_graph_open");
            false
        }
    }
}

/// Close a graph engine and release resources.
#[no_mangle]
pub extern "C" fn nodedb_graph_close(handle: GraphHandle) {
    let _ = catch_unwind(|| {
        let mut map = graph_handle_map().write().unwrap();
        map.remove(&handle);
    });
}

/// Execute a graph operation. request is MessagePack with an "action" field.
/// Response is MessagePack written to out_response.
#[no_mangle]
pub extern "C" fn nodedb_graph_execute(
    handle: GraphHandle,
    request_ptr: *const u8,
    request_len: usize,
    out_response: *mut *mut u8,
    out_response_len: *mut usize,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if request_ptr.is_null() || out_response.is_null() || out_response_len.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let graph = match get_graph(handle) {
            Some(g) => g,
            None => {
                set_error(out_error, ERR_INVALID_HANDLE, "invalid graph handle");
                return false;
            }
        };

        let request_bytes = unsafe { std::slice::from_raw_parts(request_ptr, request_len) };

        let request: rmpv::Value = match rmp_serde::from_slice(request_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &format!("invalid request: {}", e));
                return false;
            }
        };

        let action = match request.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("action")))
            .and_then(|(_, v)| v.as_str())
        {
            Some(a) => a.to_string(),
            None => {
                set_error(out_error, ERR_INVALID_QUERY, "missing 'action' field");
                return false;
            }
        };

        let get_str = |key: &str| -> Option<String> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_str())
                .map(|s| s.to_string())
        };

        let get_i64 = |key: &str| -> Option<i64> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_i64())
        };

        let get_f64 = |key: &str| -> Option<f64> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_f64())
        };

        let get_value = |key: &str| -> rmpv::Value {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .map(|(_, v)| v.clone())
                .unwrap_or(rmpv::Value::Nil)
        };

        let get_u64 = |key: &str| -> Option<u64> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_u64())
        };

        let response_value: rmpv::Value = match action.as_str() {
            "add_node" => {
                let label = get_str("label").unwrap_or_default();
                let data = get_value("data");
                match graph.add_node(&label, data) {
                    Ok(node) => match rmpv::ext::to_value(&node) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            "get_node" => {
                let id = get_i64("id").unwrap_or(0);
                match graph.get_node(id) {
                    Ok(node) => match rmpv::ext::to_value(&node) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            "update_node" => {
                let id = get_i64("id").unwrap_or(0);
                let data = get_value("data");
                match graph.update_node(id, data) {
                    Ok(node) => match rmpv::ext::to_value(&node) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            "delete_node" => {
                let id = get_i64("id").unwrap_or(0);
                let behaviour = match get_str("behaviour").as_deref() {
                    Some("restrict") => DeleteBehaviour::Restrict,
                    Some("cascade") => DeleteBehaviour::Cascade,
                    Some("nullify") => DeleteBehaviour::Nullify,
                    _ => DeleteBehaviour::Detach,
                };
                match graph.delete_node(id, behaviour) {
                    Ok(()) => rmpv::Value::Boolean(true),
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            "all_nodes" => {
                match graph.all_nodes() {
                    Ok(nodes) => match rmpv::ext::to_value(&nodes) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            "node_count" => {
                rmpv::Value::Integer((graph.node_count() as i64).into())
            }
            "add_edge" => {
                let label = get_str("label").unwrap_or_default();
                let source = get_i64("source").unwrap_or(0);
                let target = get_i64("target").unwrap_or(0);
                let weight = get_f64("weight").unwrap_or(1.0);
                let data = get_value("data");
                match graph.add_edge(&label, source, target, weight, data) {
                    Ok(edge) => match rmpv::ext::to_value(&edge) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            "get_edge" => {
                let id = get_i64("id").unwrap_or(0);
                match graph.get_edge(id) {
                    Ok(edge) => match rmpv::ext::to_value(&edge) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            "update_edge" => {
                let id = get_i64("id").unwrap_or(0);
                let data = get_value("data");
                match graph.update_edge(id, data) {
                    Ok(edge) => match rmpv::ext::to_value(&edge) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            "delete_edge" => {
                let id = get_i64("id").unwrap_or(0);
                match graph.delete_edge(id) {
                    Ok(()) => rmpv::Value::Boolean(true),
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            "edges_from" => {
                let id = get_i64("id").unwrap_or(0);
                match graph.edges_from(id) {
                    Ok(edges) => match rmpv::ext::to_value(&edges) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            "edges_to" => {
                let id = get_i64("id").unwrap_or(0);
                match graph.edges_to(id) {
                    Ok(edges) => match rmpv::ext::to_value(&edges) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            "neighbors" => {
                let id = get_i64("id").unwrap_or(0);
                match graph.neighbors(id) {
                    Ok(nodes) => match rmpv::ext::to_value(&nodes) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            "bfs" => {
                let id = get_i64("id").unwrap_or(0);
                let max_depth = get_u64("max_depth").map(|v| v as usize);
                match nodedb_graph::bfs(&graph, id, max_depth) {
                    Ok(r) => rmpv::Value::Map(vec![
                        (rmpv::Value::String("nodes".into()), rmpv::ext::to_value(&r.nodes).unwrap_or(rmpv::Value::Nil)),
                        (rmpv::Value::String("edges".into()), rmpv::ext::to_value(&r.edges).unwrap_or(rmpv::Value::Nil)),
                    ]),
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            "dfs" => {
                let id = get_i64("id").unwrap_or(0);
                let max_depth = get_u64("max_depth").map(|v| v as usize);
                match nodedb_graph::dfs(&graph, id, max_depth) {
                    Ok(r) => rmpv::Value::Map(vec![
                        (rmpv::Value::String("nodes".into()), rmpv::ext::to_value(&r.nodes).unwrap_or(rmpv::Value::Nil)),
                        (rmpv::Value::String("edges".into()), rmpv::ext::to_value(&r.edges).unwrap_or(rmpv::Value::Nil)),
                    ]),
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            "shortest_path" => {
                let from = get_i64("from").unwrap_or(0);
                let to = get_i64("to").unwrap_or(0);
                match nodedb_graph::shortest_path(&graph, from, to) {
                    Ok(Some(r)) => rmpv::Value::Map(vec![
                        (rmpv::Value::String("path".into()), rmpv::ext::to_value(&r.path).unwrap_or(rmpv::Value::Nil)),
                        (rmpv::Value::String("total_weight".into()), rmpv::Value::F64(r.total_weight)),
                        (rmpv::Value::String("edges".into()), rmpv::ext::to_value(&r.edges).unwrap_or(rmpv::Value::Nil)),
                    ]),
                    Ok(None) => rmpv::Value::Nil,
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            "pagerank" => {
                let damping = get_f64("damping").unwrap_or(0.85);
                let iterations = get_u64("iterations").unwrap_or(20) as usize;
                match nodedb_graph::pagerank(&graph, damping, iterations) {
                    Ok(ranks) => {
                        let entries: Vec<(rmpv::Value, rmpv::Value)> = ranks.into_iter()
                            .map(|(id, rank)| (rmpv::Value::Integer(id.into()), rmpv::Value::F64(rank)))
                            .collect();
                        rmpv::Value::Map(entries)
                    }
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            "connected_components" => {
                match nodedb_graph::connected_components(&graph) {
                    Ok(components) => match rmpv::ext::to_value(&components) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            "has_cycle" => {
                match nodedb_graph::has_cycle(&graph) {
                    Ok(v) => rmpv::Value::Boolean(v),
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            "find_cycles" => {
                match nodedb_graph::find_cycles(&graph) {
                    Ok(cycles) => match rmpv::ext::to_value(&cycles) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, graph_error_code(&e), &e.to_string()); return false; }
                }
            }
            other => {
                set_error(out_error, ERR_INVALID_QUERY, &format!("unknown graph action: {}", other));
                return false;
            }
        };

        // Serialize response
        let response_bytes = match rmp_serde::to_vec(&response_value) {
            Ok(b) => b,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &e.to_string());
                return false;
            }
        };

        let len = response_bytes.len();
        let ptr = response_bytes.as_ptr();
        std::mem::forget(response_bytes);

        unsafe {
            *out_response = ptr as *mut u8;
            *out_response_len = len;
        }

        clear_error(out_error);
        true
    });

    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_graph_execute");
            false
        }
    }
}

// =============================================================================
// Vector FFI
// =============================================================================

/// Open a vector engine. config is a MessagePack-encoded map with:
/// "path" (string), "dimension" (int), optional: "metric" (string: cosine|euclidean|dotproduct),
/// "max_elements" (int), "ef_construction" (int), "max_nb_connection" (int).
#[no_mangle]
pub extern "C" fn nodedb_vector_open(
    config_ptr: *const u8,
    config_len: usize,
    out_handle: *mut VectorHandle,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if config_ptr.is_null() || out_handle.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let config_bytes = unsafe { std::slice::from_raw_parts(config_ptr, config_len) };

        let config: rmpv::Value = match rmp_serde::from_slice(config_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &format!("invalid config: {}", e));
                return false;
            }
        };

        let path_str = match config.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("path")))
            .and_then(|(_, v)| v.as_str())
        {
            Some(p) => p.to_string(),
            None => {
                set_error(out_error, ERR_SERIALIZATION, "config missing 'path' field");
                return false;
            }
        };

        let dimension = match config.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("dimension")))
            .and_then(|(_, v)| v.as_u64())
        {
            Some(d) => d as usize,
            None => {
                set_error(out_error, ERR_SERIALIZATION, "config missing 'dimension' field");
                return false;
            }
        };

        let metric_str = config.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("metric")))
            .and_then(|(_, v)| v.as_str())
            .unwrap_or("cosine");

        let metric = match DistanceMetric::from_str(metric_str) {
            Some(m) => m,
            None => {
                set_error(out_error, ERR_INVALID_QUERY, &format!("unknown metric: {}", metric_str));
                return false;
            }
        };

        let mut coll_config = CollectionConfig::new(dimension);
        coll_config.metric = metric;

        if let Some(v) = config.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("max_elements")))
            .and_then(|(_, v)| v.as_u64())
        {
            coll_config.max_elements = v as usize;
        }
        if let Some(v) = config.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("ef_construction")))
            .and_then(|(_, v)| v.as_u64())
        {
            coll_config.ef_construction = v as usize;
        }
        if let Some(v) = config.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("max_nb_connection")))
            .and_then(|(_, v)| v.as_u64())
        {
            coll_config.max_nb_connection = v as usize;
        }

        let path = PathBuf::from(&path_str);
        let storage = match StorageEngine::open(&path) {
            Ok(e) => Arc::new(e),
            Err(e) => {
                set_error(out_error, ERR_STORAGE, &e.to_string());
                return false;
            }
        };

        let hnsw_dir = path.join("hnsw_data");
        match VectorEngine::open(storage, coll_config, &hnsw_dir) {
            Ok(ve) => {
                let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
                let mut map = vector_handle_map().write().unwrap();
                map.insert(handle, Arc::new(std::sync::Mutex::new(ve)));
                unsafe { *out_handle = handle; }
                clear_error(out_error);
                true
            }
            Err(e) => {
                set_error(out_error, vector_error_code(&e), &e.to_string());
                false
            }
        }
    });

    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_vector_open");
            false
        }
    }
}

/// Close a vector engine and release resources.
#[no_mangle]
pub extern "C" fn nodedb_vector_close(handle: VectorHandle) {
    let _ = catch_unwind(|| {
        let mut map = vector_handle_map().write().unwrap();
        map.remove(&handle);
    });
}

/// Execute a vector operation. request is MessagePack with an "action" field.
/// Response is MessagePack written to out_response.
#[no_mangle]
pub extern "C" fn nodedb_vector_execute(
    handle: VectorHandle,
    request_ptr: *const u8,
    request_len: usize,
    out_response: *mut *mut u8,
    out_response_len: *mut usize,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if request_ptr.is_null() || out_response.is_null() || out_response_len.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let ve_arc = match get_vector(handle) {
            Some(v) => v,
            None => {
                set_error(out_error, ERR_INVALID_HANDLE, "invalid vector handle");
                return false;
            }
        };

        let request_bytes = unsafe { std::slice::from_raw_parts(request_ptr, request_len) };

        let request: rmpv::Value = match rmp_serde::from_slice(request_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &format!("invalid request: {}", e));
                return false;
            }
        };

        let action = match request.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("action")))
            .and_then(|(_, v)| v.as_str())
        {
            Some(a) => a.to_string(),
            None => {
                set_error(out_error, ERR_INVALID_QUERY, "missing 'action' field");
                return false;
            }
        };

        let get_i64 = |key: &str| -> Option<i64> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_i64())
        };

        let get_u64 = |key: &str| -> Option<u64> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_u64())
        };

        let get_value = |key: &str| -> rmpv::Value {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .map(|(_, v)| v.clone())
                .unwrap_or(rmpv::Value::Nil)
        };

        let get_f32_array = |key: &str| -> Option<Vec<f32>> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_array())
                .map(|arr| arr.iter().map(|x| x.as_f64().unwrap_or(0.0) as f32).collect())
        };

        let mut ve = ve_arc.lock().unwrap();

        let response_value: rmpv::Value = match action.as_str() {
            "insert" => {
                let vector = match get_f32_array("vector") {
                    Some(v) => v,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'vector' field");
                        return false;
                    }
                };
                let metadata = get_value("metadata");
                match ve.insert(&vector, metadata) {
                    Ok(record) => match rmpv::ext::to_value(&record) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, vector_error_code(&e), &e.to_string()); return false; }
                }
            }
            "get" => {
                let id = get_i64("id").unwrap_or(0);
                match ve.get(id) {
                    Ok((record, vector)) => {
                        let vec_val: Vec<rmpv::Value> = vector.iter().map(|&f| rmpv::Value::F64(f as f64)).collect();
                        rmpv::Value::Map(vec![
                            (rmpv::Value::String("record".into()), rmpv::ext::to_value(&record).unwrap_or(rmpv::Value::Nil)),
                            (rmpv::Value::String("vector".into()), rmpv::Value::Array(vec_val)),
                        ])
                    }
                    Err(e) => { set_error(out_error, vector_error_code(&e), &e.to_string()); return false; }
                }
            }
            "delete" => {
                let id = get_i64("id").unwrap_or(0);
                match ve.delete(id) {
                    Ok(()) => rmpv::Value::Boolean(true),
                    Err(e) => { set_error(out_error, vector_error_code(&e), &e.to_string()); return false; }
                }
            }
            "update_metadata" => {
                let id = get_i64("id").unwrap_or(0);
                let metadata = get_value("metadata");
                match ve.update_metadata(id, metadata) {
                    Ok(record) => match rmpv::ext::to_value(&record) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, vector_error_code(&e), &e.to_string()); return false; }
                }
            }
            "search" => {
                let query = match get_f32_array("query") {
                    Some(v) => v,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'query' field");
                        return false;
                    }
                };
                let k = get_u64("k").unwrap_or(10) as usize;
                let ef_search = get_u64("ef_search").unwrap_or(64) as usize;
                match ve.search(&query, k, ef_search) {
                    Ok(results) => {
                        let arr: Vec<rmpv::Value> = results.iter().map(|r| {
                            rmpv::Value::Map(vec![
                                (rmpv::Value::String("id".into()), rmpv::Value::Integer((r.id).into())),
                                (rmpv::Value::String("distance".into()), rmpv::Value::F64(r.distance as f64)),
                                (rmpv::Value::String("metadata".into()), r.metadata.clone()),
                            ])
                        }).collect();
                        rmpv::Value::Array(arr)
                    }
                    Err(e) => { set_error(out_error, vector_error_code(&e), &e.to_string()); return false; }
                }
            }
            "count" => {
                rmpv::Value::Integer((ve.count() as i64).into())
            }
            "flush" => {
                match ve.flush() {
                    Ok(()) => rmpv::Value::Boolean(true),
                    Err(e) => { set_error(out_error, vector_error_code(&e), &e.to_string()); return false; }
                }
            }
            other => {
                set_error(out_error, ERR_INVALID_QUERY, &format!("unknown vector action: {}", other));
                return false;
            }
        };

        // Serialize response
        let response_bytes = match rmp_serde::to_vec(&response_value) {
            Ok(b) => b,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &e.to_string());
                return false;
            }
        };

        let len = response_bytes.len();
        let ptr = response_bytes.as_ptr();
        std::mem::forget(response_bytes);

        unsafe {
            *out_response = ptr as *mut u8;
            *out_response_len = len;
        }

        clear_error(out_error);
        true
    });

    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_vector_execute");
            false
        }
    }
}

// =============================================================================
// Federation FFI
// =============================================================================

/// Open a federation engine. config is a MessagePack-encoded map with a "path" key.
#[no_mangle]
pub extern "C" fn nodedb_federation_open(
    config_ptr: *const u8,
    config_len: usize,
    out_handle: *mut FederationHandle,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if config_ptr.is_null() || out_handle.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let config_bytes = unsafe { std::slice::from_raw_parts(config_ptr, config_len) };

        let config: rmpv::Value = match rmp_serde::from_slice(config_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &format!("invalid config: {}", e));
                return false;
            }
        };

        let path_str = match config.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("path")))
            .and_then(|(_, v)| v.as_str())
        {
            Some(p) => p.to_string(),
            None => {
                set_error(out_error, ERR_SERIALIZATION, "config missing 'path' field");
                return false;
            }
        };

        let path = PathBuf::from(&path_str);
        let storage = match StorageEngine::open(&path) {
            Ok(e) => Arc::new(e),
            Err(e) => {
                set_error(out_error, ERR_STORAGE, &e.to_string());
                return false;
            }
        };

        match FederationEngine::new(storage) {
            Ok(fed) => {
                let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
                let mut map = federation_handle_map().write().unwrap();
                map.insert(handle, Arc::new(fed));
                unsafe { *out_handle = handle; }
                clear_error(out_error);
                true
            }
            Err(e) => {
                set_error(out_error, federation_error_code(&e), &e.to_string());
                false
            }
        }
    });

    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_federation_open");
            false
        }
    }
}

/// Close a federation engine and release resources.
#[no_mangle]
pub extern "C" fn nodedb_federation_close(handle: FederationHandle) {
    let _ = catch_unwind(|| {
        let mut map = federation_handle_map().write().unwrap();
        map.remove(&handle);
    });
}

/// Execute a federation operation. request is MessagePack with an "action" field.
/// Response is MessagePack written to out_response.
#[no_mangle]
pub extern "C" fn nodedb_federation_execute(
    handle: FederationHandle,
    request_ptr: *const u8,
    request_len: usize,
    out_response: *mut *mut u8,
    out_response_len: *mut usize,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if request_ptr.is_null() || out_response.is_null() || out_response_len.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let fed = match get_federation(handle) {
            Some(f) => f,
            None => {
                set_error(out_error, ERR_INVALID_HANDLE, "invalid federation handle");
                return false;
            }
        };

        let request_bytes = unsafe { std::slice::from_raw_parts(request_ptr, request_len) };

        let request: rmpv::Value = match rmp_serde::from_slice(request_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &format!("invalid request: {}", e));
                return false;
            }
        };

        let action = match request.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("action")))
            .and_then(|(_, v)| v.as_str())
        {
            Some(a) => a.to_string(),
            None => {
                set_error(out_error, ERR_INVALID_QUERY, "missing 'action' field");
                return false;
            }
        };

        let get_str = |key: &str| -> Option<String> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_str())
                .map(|s| s.to_string())
        };

        let get_i64 = |key: &str| -> Option<i64> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_i64())
        };

        let get_value = |key: &str| -> rmpv::Value {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .map(|(_, v)| v.clone())
                .unwrap_or(rmpv::Value::Nil)
        };

        let get_optional_str = |key: &str| -> Option<Option<String>> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .map(|(_, v)| v.as_str().map(|s| s.to_string()))
        };

        let parse_status = |s: &str| -> Option<PeerStatus> {
            match s {
                "active" => Some(PeerStatus::Active),
                "inactive" => Some(PeerStatus::Inactive),
                "banned" => Some(PeerStatus::Banned),
                _ => None,
            }
        };

        let response_value: rmpv::Value = match action.as_str() {
            "add_peer" => {
                let name = match get_str("name") {
                    Some(n) => n,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'name' field");
                        return false;
                    }
                };
                let endpoint = get_str("endpoint");
                let public_key = get_str("public_key");
                let status = get_str("status")
                    .and_then(|s| parse_status(&s))
                    .unwrap_or(PeerStatus::Active);
                let metadata = get_value("metadata");
                match fed.add_peer(&name, endpoint, public_key, status, metadata) {
                    Ok(peer) => match rmpv::ext::to_value(&peer) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, federation_error_code(&e), &e.to_string()); return false; }
                }
            }
            "get_peer" => {
                let id = get_i64("id").unwrap_or(0);
                match fed.get_peer(id) {
                    Ok(peer) => match rmpv::ext::to_value(&peer) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, federation_error_code(&e), &e.to_string()); return false; }
                }
            }
            "get_peer_by_name" => {
                let name = match get_str("name") {
                    Some(n) => n,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'name' field");
                        return false;
                    }
                };
                match fed.get_peer_by_name(&name) {
                    Ok(peer) => match rmpv::ext::to_value(&peer) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, federation_error_code(&e), &e.to_string()); return false; }
                }
            }
            "update_peer" => {
                let id = get_i64("id").unwrap_or(0);
                let endpoint = get_optional_str("endpoint");
                let public_key = get_optional_str("public_key");
                let status = get_str("status").and_then(|s| parse_status(&s));
                let metadata = if request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("metadata")))
                    .is_some()
                {
                    Some(get_value("metadata"))
                } else {
                    None
                };
                match fed.update_peer(id, endpoint, public_key, status, metadata) {
                    Ok(peer) => match rmpv::ext::to_value(&peer) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, federation_error_code(&e), &e.to_string()); return false; }
                }
            }
            "delete_peer" => {
                let id = get_i64("id").unwrap_or(0);
                match fed.delete_peer(id) {
                    Ok(peer) => match rmpv::ext::to_value(&peer) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, federation_error_code(&e), &e.to_string()); return false; }
                }
            }
            "all_peers" => {
                match fed.all_peers() {
                    Ok(peers) => {
                        let arr: Vec<rmpv::Value> = peers.into_iter()
                            .filter_map(|p| rmpv::ext::to_value(&p).ok())
                            .collect();
                        rmpv::Value::Array(arr)
                    }
                    Err(e) => { set_error(out_error, federation_error_code(&e), &e.to_string()); return false; }
                }
            }
            "peer_count" => {
                rmpv::Value::Integer((fed.peer_count() as i64).into())
            }
            "add_group" => {
                let name = match get_str("name") {
                    Some(n) => n,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'name' field");
                        return false;
                    }
                };
                let metadata = get_value("metadata");
                match fed.add_group(&name, metadata) {
                    Ok(group) => match rmpv::ext::to_value(&group) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, federation_error_code(&e), &e.to_string()); return false; }
                }
            }
            "get_group" => {
                let id = get_i64("id").unwrap_or(0);
                match fed.get_group(id) {
                    Ok(group) => match rmpv::ext::to_value(&group) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, federation_error_code(&e), &e.to_string()); return false; }
                }
            }
            "get_group_by_name" => {
                let name = match get_str("name") {
                    Some(n) => n,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'name' field");
                        return false;
                    }
                };
                match fed.get_group_by_name(&name) {
                    Ok(group) => match rmpv::ext::to_value(&group) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, federation_error_code(&e), &e.to_string()); return false; }
                }
            }
            "update_group" => {
                let id = get_i64("id").unwrap_or(0);
                let metadata = if request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("metadata")))
                    .is_some()
                {
                    Some(get_value("metadata"))
                } else {
                    None
                };
                match fed.update_group(id, metadata) {
                    Ok(group) => match rmpv::ext::to_value(&group) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, federation_error_code(&e), &e.to_string()); return false; }
                }
            }
            "delete_group" => {
                let id = get_i64("id").unwrap_or(0);
                match fed.delete_group(id) {
                    Ok(group) => match rmpv::ext::to_value(&group) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, federation_error_code(&e), &e.to_string()); return false; }
                }
            }
            "all_groups" => {
                match fed.all_groups() {
                    Ok(groups) => {
                        let arr: Vec<rmpv::Value> = groups.into_iter()
                            .filter_map(|g| rmpv::ext::to_value(&g).ok())
                            .collect();
                        rmpv::Value::Array(arr)
                    }
                    Err(e) => { set_error(out_error, federation_error_code(&e), &e.to_string()); return false; }
                }
            }
            "group_count" => {
                rmpv::Value::Integer((fed.group_count() as i64).into())
            }
            "add_member" => {
                let group_id = match get_i64("group_id") {
                    Some(id) => id,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'group_id' field");
                        return false;
                    }
                };
                let peer_id = match get_i64("peer_id") {
                    Some(id) => id,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'peer_id' field");
                        return false;
                    }
                };
                match fed.add_member(group_id, peer_id) {
                    Ok(group) => match rmpv::ext::to_value(&group) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, federation_error_code(&e), &e.to_string()); return false; }
                }
            }
            "remove_member" => {
                let group_id = match get_i64("group_id") {
                    Some(id) => id,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'group_id' field");
                        return false;
                    }
                };
                let peer_id = match get_i64("peer_id") {
                    Some(id) => id,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'peer_id' field");
                        return false;
                    }
                };
                match fed.remove_member(group_id, peer_id) {
                    Ok(group) => match rmpv::ext::to_value(&group) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, federation_error_code(&e), &e.to_string()); return false; }
                }
            }
            "groups_for_peer" => {
                let peer_id = match get_i64("peer_id") {
                    Some(id) => id,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'peer_id' field");
                        return false;
                    }
                };
                match fed.groups_for_peer(peer_id) {
                    Ok(ids) => {
                        let arr: Vec<rmpv::Value> = ids.into_iter()
                            .map(|id| rmpv::Value::Integer(id.into()))
                            .collect();
                        rmpv::Value::Array(arr)
                    }
                    Err(e) => { set_error(out_error, federation_error_code(&e), &e.to_string()); return false; }
                }
            }
            other => {
                set_error(out_error, ERR_INVALID_QUERY, &format!("unknown federation action: {}", other));
                return false;
            }
        };

        // Serialize response
        let response_bytes = match rmp_serde::to_vec(&response_value) {
            Ok(b) => b,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &e.to_string());
                return false;
            }
        };

        let len = response_bytes.len();
        let ptr = response_bytes.as_ptr();
        std::mem::forget(response_bytes);

        unsafe {
            *out_response = ptr as *mut u8;
            *out_response_len = len;
        }

        clear_error(out_error);
        true
    });

    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_federation_execute");
            false
        }
    }
}

// =============================================================================
// DAC FFI
// =============================================================================

/// Open a DAC engine. config is a MessagePack-encoded map with a "path" key.
#[no_mangle]
pub extern "C" fn nodedb_dac_open(
    config_ptr: *const u8,
    config_len: usize,
    out_handle: *mut DacHandle,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if config_ptr.is_null() || out_handle.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let config_bytes = unsafe { std::slice::from_raw_parts(config_ptr, config_len) };

        let config: rmpv::Value = match rmp_serde::from_slice(config_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &format!("invalid config: {}", e));
                return false;
            }
        };

        let path_str = match config.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("path")))
            .and_then(|(_, v)| v.as_str())
        {
            Some(p) => p.to_string(),
            None => {
                set_error(out_error, ERR_SERIALIZATION, "config missing 'path' field");
                return false;
            }
        };

        let path = PathBuf::from(&path_str);
        let storage = match StorageEngine::open(&path) {
            Ok(e) => Arc::new(e),
            Err(e) => {
                set_error(out_error, ERR_STORAGE, &e.to_string());
                return false;
            }
        };

        match DacEngine::new(storage) {
            Ok(dac) => {
                let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
                let mut map = dac_handle_map().write().unwrap();
                map.insert(handle, Arc::new(dac));
                unsafe { *out_handle = handle; }
                clear_error(out_error);
                true
            }
            Err(e) => {
                set_error(out_error, dac_error_code(&e), &e.to_string());
                false
            }
        }
    });

    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_dac_open");
            false
        }
    }
}

/// Close a DAC engine and release resources.
#[no_mangle]
pub extern "C" fn nodedb_dac_close(handle: DacHandle) {
    let _ = catch_unwind(|| {
        let mut map = dac_handle_map().write().unwrap();
        map.remove(&handle);
    });
}

/// Execute a DAC operation. request is MessagePack with an "action" field.
#[no_mangle]
pub extern "C" fn nodedb_dac_execute(
    handle: DacHandle,
    request_ptr: *const u8,
    request_len: usize,
    out_response: *mut *mut u8,
    out_response_len: *mut usize,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if request_ptr.is_null() || out_response.is_null() || out_response_len.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let dac = match get_dac(handle) {
            Some(d) => d,
            None => {
                set_error(out_error, ERR_INVALID_HANDLE, "invalid dac handle");
                return false;
            }
        };

        let request_bytes = unsafe { std::slice::from_raw_parts(request_ptr, request_len) };

        let request: rmpv::Value = match rmp_serde::from_slice(request_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &format!("invalid request: {}", e));
                return false;
            }
        };

        let action = match request.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("action")))
            .and_then(|(_, v)| v.as_str())
        {
            Some(a) => a.to_string(),
            None => {
                set_error(out_error, ERR_INVALID_QUERY, "missing 'action' field");
                return false;
            }
        };

        let get_str = |key: &str| -> Option<String> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_str())
                .map(|s| s.to_string())
        };

        let get_i64 = |key: &str| -> Option<i64> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_i64())
        };

        let get_value = |key: &str| -> rmpv::Value {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .map(|(_, v)| v.clone())
                .unwrap_or(rmpv::Value::Nil)
        };

        let parse_subject_type = |s: &str| -> Option<AccessSubjectType> {
            match s {
                "peer" => Some(AccessSubjectType::Peer),
                "group" => Some(AccessSubjectType::Group),
                _ => None,
            }
        };

        let parse_permission = |s: &str| -> Option<AccessPermission> {
            match s {
                "allow" => Some(AccessPermission::Allow),
                "deny" => Some(AccessPermission::Deny),
                "redact" => Some(AccessPermission::Redact),
                _ => None,
            }
        };

        let response_value: rmpv::Value = match action.as_str() {
            "add_rule" => {
                let collection = match get_str("collection") {
                    Some(c) => c,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'collection' field");
                        return false;
                    }
                };
                let subject_type = match get_str("subject_type").and_then(|s| parse_subject_type(&s)) {
                    Some(st) => st,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing or invalid 'subject_type' (peer/group)");
                        return false;
                    }
                };
                let subject_id = match get_str("subject_id") {
                    Some(s) => s,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'subject_id' field");
                        return false;
                    }
                };
                let permission = match get_str("permission").and_then(|s| parse_permission(&s)) {
                    Some(p) => p,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing or invalid 'permission' (allow/deny/redact)");
                        return false;
                    }
                };
                let field = get_str("field");
                let record_id = get_str("record_id");
                let expires_at = get_str("expires_at").and_then(|s| {
                    chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&chrono::Utc))
                });
                let created_by = get_str("created_by");

                match dac.add_rule(&collection, field, record_id, subject_type, &subject_id, permission, expires_at, created_by) {
                    Ok(rule) => match rmpv::ext::to_value(&rule) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, dac_error_code(&e), &e.to_string()); return false; }
                }
            }
            "get_rule" => {
                let id = get_i64("id").unwrap_or(0);
                match dac.get_rule(id) {
                    Ok(rule) => match rmpv::ext::to_value(&rule) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, dac_error_code(&e), &e.to_string()); return false; }
                }
            }
            "update_rule" => {
                let id = get_i64("id").unwrap_or(0);
                let permission = get_str("permission").and_then(|s| parse_permission(&s));
                let expires_at = if request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("expires_at")))
                    .is_some()
                {
                    let ea = get_str("expires_at").and_then(|s| {
                        chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&chrono::Utc))
                    });
                    Some(ea)
                } else {
                    None
                };
                match dac.update_rule(id, permission, expires_at) {
                    Ok(rule) => match rmpv::ext::to_value(&rule) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, dac_error_code(&e), &e.to_string()); return false; }
                }
            }
            "delete_rule" => {
                let id = get_i64("id").unwrap_or(0);
                match dac.delete_rule(id) {
                    Ok(rule) => match rmpv::ext::to_value(&rule) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, dac_error_code(&e), &e.to_string()); return false; }
                }
            }
            "all_rules" => {
                match dac.all_rules() {
                    Ok(rules) => {
                        let arr: Vec<rmpv::Value> = rules.into_iter()
                            .filter_map(|r| rmpv::ext::to_value(&r).ok())
                            .collect();
                        rmpv::Value::Array(arr)
                    }
                    Err(e) => { set_error(out_error, dac_error_code(&e), &e.to_string()); return false; }
                }
            }
            "rules_for_collection" => {
                let collection = match get_str("collection") {
                    Some(c) => c,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'collection' field");
                        return false;
                    }
                };
                match dac.rules_for_collection(&collection) {
                    Ok(rules) => {
                        let arr: Vec<rmpv::Value> = rules.into_iter()
                            .filter_map(|r| rmpv::ext::to_value(&r).ok())
                            .collect();
                        rmpv::Value::Array(arr)
                    }
                    Err(e) => { set_error(out_error, dac_error_code(&e), &e.to_string()); return false; }
                }
            }
            "rule_count" => {
                rmpv::Value::Integer((dac.rule_count() as i64).into())
            }
            "filter_document" => {
                let collection = match get_str("collection") {
                    Some(c) => c,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'collection' field");
                        return false;
                    }
                };
                let document = get_value("document");
                let peer_id = match get_str("peer_id") {
                    Some(p) => p,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'peer_id' field");
                        return false;
                    }
                };
                let group_ids: Vec<String> = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("group_ids")))
                    .and_then(|(_, v)| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();
                let record_id = get_str("record_id");
                let subject = DacSubject { peer_id, group_ids };

                match dac.filter_document(&collection, &document, &subject, record_id.as_deref()) {
                    Ok(filtered) => filtered,
                    Err(e) => { set_error(out_error, dac_error_code(&e), &e.to_string()); return false; }
                }
            }
            other => {
                set_error(out_error, ERR_INVALID_QUERY, &format!("unknown dac action: {}", other));
                return false;
            }
        };

        // Serialize response
        let response_bytes = match rmp_serde::to_vec(&response_value) {
            Ok(b) => b,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &e.to_string());
                return false;
            }
        };

        let len = response_bytes.len();
        let ptr = response_bytes.as_ptr();
        std::mem::forget(response_bytes);

        unsafe {
            *out_response = ptr as *mut u8;
            *out_response_len = len;
        }

        clear_error(out_error);
        true
    });

    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_dac_execute");
            false
        }
    }
}

// ── Transport FFI ───────────────────────────────────────────────────────────

/// Global lazily-initialized tokio runtime for async transport operations.
fn global_runtime() -> &'static tokio::runtime::Runtime {
    use std::sync::OnceLock;
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to build tokio runtime")
    })
}

fn transport_handle_map() -> &'static RwLock<HashMap<TransportHandle, Arc<TransportEngine>>> {
    use std::sync::OnceLock;
    static MAP: OnceLock<RwLock<HashMap<TransportHandle, Arc<TransportEngine>>>> = OnceLock::new();
    MAP.get_or_init(|| RwLock::new(HashMap::new()))
}

fn get_transport(handle: TransportHandle) -> Option<Arc<TransportEngine>> {
    let map = transport_handle_map().read().ok()?;
    map.get(&handle).cloned()
}

/// Emit a TriggerNotification to all mesh peers (fire-and-forget).
/// Called after successful trigger-aware writes when a transport is linked.
fn emit_trigger_notification(
    db_handle: DbHandle,
    collection: &str,
    event: &str,
    old_record: Option<&nodedb_nosql::Document>,
    new_record: Option<&nodedb_nosql::Document>,
) {
    let transport_handle = {
        let map = db_transport_link_map().read().unwrap();
        match map.get(&db_handle) {
            Some(h) => *h,
            None => return, // no transport linked
        }
    };
    let engine = match get_transport(transport_handle) {
        Some(e) => e,
        None => return,
    };

    let payload = nodedb_transport::TriggerNotificationPayload {
        source_database: format!("db_{}", db_handle),
        collection: collection.to_string(),
        event: event.to_string(),
        old_record: old_record.and_then(|d| rmp_serde::to_vec(d).ok()),
        new_record: new_record.and_then(|d| rmp_serde::to_vec(d).ok()),
    };

    let payload_bytes = match rmp_serde::to_vec(&payload) {
        Ok(b) => b,
        Err(_) => return,
    };

    let msg = nodedb_transport::WireMessage {
        version: 1,
        msg_id: format!("trigger-{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()),
        msg_type: nodedb_transport::WireMessageType::TriggerNotification,
        sender_id: engine.identity().peer_id().to_string(),
        payload: payload_bytes,
    };

    let pool = engine.pool().clone();
    global_runtime().spawn(async move {
        pool.broadcast(&msg).await;
    });
}

/// Maps DbHandle → TransportHandle for mesh trigger notification delivery.
fn db_transport_link_map() -> &'static RwLock<HashMap<DbHandle, TransportHandle>> {
    use std::sync::OnceLock;
    static MAP: OnceLock<RwLock<HashMap<DbHandle, TransportHandle>>> = OnceLock::new();
    MAP.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Link a transport handle to a database handle for mesh trigger delivery.
#[no_mangle]
pub extern "C" fn nodedb_link_transport(
    db_handle: DbHandle,
    transport_handle: TransportHandle,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if get_db(db_handle).is_none() {
            set_error(out_error, ERR_INVALID_HANDLE, "invalid database handle");
            return false;
        }
        if get_transport(transport_handle).is_none() {
            set_error(out_error, ERR_INVALID_HANDLE, "invalid transport handle");
            return false;
        }
        let mut map = db_transport_link_map().write().unwrap();
        map.insert(db_handle, transport_handle);
        clear_error(out_error);
        true
    });
    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_link_transport");
            false
        }
    }
}

fn transport_error_code(e: &TransportError) -> i32 {
    match e {
        TransportError::Connection(_) => ERR_TRANSPORT_CONNECTION,
        TransportError::Tls(_) => ERR_TRANSPORT_CONNECTION,
        TransportError::WebSocket(_) => ERR_TRANSPORT_CONNECTION,
        TransportError::Handshake(_) => ERR_TRANSPORT_HANDSHAKE,
        TransportError::PeerRejected(_) => ERR_TRANSPORT_PEER_REJECTED,
        TransportError::Send(_) => ERR_TRANSPORT_SEND,
        TransportError::Receive(_) => ERR_TRANSPORT_SEND,
        TransportError::Timeout(_) => ERR_TRANSPORT_TIMEOUT,
        TransportError::Crypto(_) => ERR_TRANSPORT_CRYPTO,
        TransportError::PeerNotConnected(_) => ERR_TRANSPORT_CONNECTION,
        TransportError::PairingRequired(_) => ERR_PAIRING_REQUIRED,
        TransportError::PairingVerificationFailed(_) => ERR_PAIRING_VERIFICATION_FAILED,
        TransportError::Pairing(_) => ERR_PAIRING_ERROR,
        _ => ERR_INTERNAL,
    }
}

/// Open a transport engine.
/// Config is MessagePack: {path, listen_addr, mdns_enabled, seed_peers, gossip_interval_seconds, gossip_fan_out, gossip_ttl, query_policy, identity_key}
#[no_mangle]
pub extern "C" fn nodedb_transport_open(
    config_ptr: *const u8,
    config_len: usize,
    out_handle: *mut TransportHandle,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if config_ptr.is_null() || out_handle.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let config_bytes = unsafe { std::slice::from_raw_parts(config_ptr, config_len) };

        let config: rmpv::Value = match rmp_serde::from_slice(config_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &format!("invalid config: {}", e));
                return false;
            }
        };

        let get_str = |key: &str| -> Option<String> {
            config.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_str())
                .map(|s| s.to_string())
        };

        let get_u64 = |key: &str| -> Option<u64> {
            config.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_u64())
        };

        let get_bool = |key: &str| -> Option<bool> {
            config.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_bool())
        };

        let path_str = get_str("path").unwrap_or_default();
        let listen_addr = get_str("listen_addr").unwrap_or_else(|| "0.0.0.0:9400".to_string());
        let mdns_enabled = get_bool("mdns_enabled").unwrap_or(true);

        let seed_peers: Vec<String> = config.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("seed_peers")))
            .and_then(|(_, v)| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let trusted_peer_keys: Vec<String> = config.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("trusted_peer_keys")))
            .and_then(|(_, v)| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let require_pairing = get_bool("require_pairing").unwrap_or(false);
        let user_id = get_str("user_id").unwrap_or_default();
        let device_name = get_str("device_name").unwrap_or_default();

        let query_policy_str = get_str("query_policy").unwrap_or_else(|| "query_peers_on_miss".to_string());
        let query_policy = match query_policy_str.as_str() {
            "local_only" => nodedb_transport::FederatedQueryPolicy::LocalOnly,
            "query_peers_always" => nodedb_transport::FederatedQueryPolicy::QueryPeersAlways,
            "query_peers_explicitly" => nodedb_transport::FederatedQueryPolicy::QueryPeersExplicitly,
            _ => nodedb_transport::FederatedQueryPolicy::QueryPeersOnMiss,
        };

        let gossip = nodedb_transport::GossipConfig {
            interval_seconds: get_u64("gossip_interval_seconds").unwrap_or(30),
            fan_out: get_u64("gossip_fan_out").unwrap_or(3) as usize,
            ttl: get_u64("gossip_ttl").unwrap_or(5) as u8,
        };

        // Mesh config (optional)
        let mesh = get_str("mesh_name").map(|mesh_name| {
            let database_name = get_str("mesh_database_name").unwrap_or_default();
            let sharing_status_str = get_str("mesh_sharing_status").unwrap_or_else(|| "full".to_string());
            let sharing_status = nodedb_transport::MeshSharingStatus::from_str(&sharing_status_str)
                .unwrap_or(nodedb_transport::MeshSharingStatus::Full);
            let max_mesh_peers = get_u64("mesh_max_peers").unwrap_or(16) as usize;
            let mesh_secret = get_str("mesh_secret");
            nodedb_transport::MeshConfig {
                mesh_name,
                database_name,
                sharing_status,
                max_mesh_peers,
                mesh_secret,
            }
        });

        // Extract sharing status before mesh is moved into config
        let mesh_sharing_status_str = mesh.as_ref().map(|m| m.sharing_status.to_str().to_string());

        let transport_config = nodedb_transport::TransportConfig {
            storage_path: path_str.clone(),
            listen_addr,
            tls_cert_pem: None,
            tls_key_pem: None,
            seed_peers,
            mdns_enabled,
            gossip,
            query_policy,
            mesh,
            trusted_peer_keys,
            require_pairing,
            user_id,
            device_name,
        };

        // Identity: from provided key bytes or generate new
        let identity = if let Some(hex_key) = get_str("identity_key") {
            let bytes: Vec<u8> = (0..hex_key.len())
                .step_by(2)
                .filter_map(|i| u8::from_str_radix(&hex_key[i..i + 2], 16).ok())
                .collect();
            if bytes.len() != 32 {
                set_error(out_error, ERR_TRANSPORT_CRYPTO, "identity_key must be 64 hex chars (32 bytes)");
                return false;
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            match NodeIdentity::from_signing_key_bytes(&arr) {
                Ok(id) => id,
                Err(e) => {
                    set_error(out_error, ERR_TRANSPORT_CRYPTO, &e.to_string());
                    return false;
                }
            }
        } else {
            NodeIdentity::generate()
        };

        // Storage (optional)
        let storage = if !path_str.is_empty() {
            match StorageEngine::open(&PathBuf::from(&path_str)) {
                Ok(e) => Some(Arc::new(e)),
                Err(e) => {
                    set_error(out_error, ERR_STORAGE, &e.to_string());
                    return false;
                }
            }
        } else {
            None
        };

        // Build query handler from optional engine handles in config
        let query_handler: Option<Arc<dyn nodedb_transport::QueryHandler>> = {
            let nosql_handle = get_u64("nosql_handle");
            let graph_handle = get_u64("graph_handle");
            let vector_handle = get_u64("vector_handle");
            let dac_handle = get_u64("dac_handle");
            let federation_handle = get_u64("federation_handle");

            let nosql = nosql_handle.and_then(get_db);
            let graph = graph_handle.and_then(get_graph);
            let vector = vector_handle.and_then(get_vector);
            let dac = dac_handle.and_then(get_dac);
            let federation = federation_handle.and_then(get_federation);

            if nosql.is_some() || graph.is_some() || vector.is_some() {
                let mut handler = query_handler::FfiQueryHandler::new(
                    nosql, graph, vector, dac, federation,
                );
                // Pass mesh sharing status for schema-level cascade
                handler.set_db_sharing_status(mesh_sharing_status_str.clone());
                Some(Arc::new(handler) as Arc<dyn nodedb_transport::QueryHandler>)
            } else {
                None
            }
        };

        let rt = global_runtime();
        // Spawn on a worker thread to avoid stack overflow on Android's
        // main thread — TLS cert generation + network binding are stack-heavy.
        let join_handle = rt.spawn(TransportEngine::start(transport_config, identity, storage, None, query_handler));
        match rt.block_on(join_handle) {
            Ok(Ok(engine)) => {
                let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
                let mut map = transport_handle_map().write().unwrap();
                map.insert(handle, Arc::new(engine));
                unsafe { *out_handle = handle; }
                clear_error(out_error);
                true
            }
            Ok(Err(e)) => {
                set_error(out_error, transport_error_code(&e), &e.to_string());
                false
            }
            Err(e) => {
                set_error(out_error, ERR_INTERNAL, &format!("transport spawn failed: {}", e));
                false
            }
        }
    });

    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_transport_open");
            false
        }
    }
}

/// Close a transport engine, signalling shutdown of all background tasks.
#[no_mangle]
pub extern "C" fn nodedb_transport_close(handle: TransportHandle) {
    let _ = catch_unwind(|| {
        let engine = {
            let mut map = transport_handle_map().write().unwrap();
            map.remove(&handle)
        };
        if let Some(e) = engine {
            e.shutdown();
        }
    });
}

/// Execute a transport operation. request is MessagePack with an "action" field.
#[no_mangle]
pub extern "C" fn nodedb_transport_execute(
    handle: TransportHandle,
    request_ptr: *const u8,
    request_len: usize,
    out_response: *mut *mut u8,
    out_response_len: *mut usize,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if request_ptr.is_null() || out_response.is_null() || out_response_len.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let engine = match get_transport(handle) {
            Some(e) => e,
            None => {
                set_error(out_error, ERR_INVALID_HANDLE, "invalid transport handle");
                return false;
            }
        };

        let request_bytes = unsafe { std::slice::from_raw_parts(request_ptr, request_len) };

        let request: rmpv::Value = match rmp_serde::from_slice(request_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &format!("invalid request: {}", e));
                return false;
            }
        };

        let action = match request.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("action")))
            .and_then(|(_, v)| v.as_str())
        {
            Some(a) => a.to_string(),
            None => {
                set_error(out_error, ERR_INVALID_QUERY, "missing 'action' field");
                return false;
            }
        };

        let response: rmpv::Value = match action.as_str() {
            "identity" => {
                let public = engine.public_identity();
                rmpv::Value::Map(vec![
                    (rmpv::Value::String("peer_id".into()), rmpv::Value::String(public.peer_id.into())),
                    (rmpv::Value::String("public_key_bytes".into()), rmpv::Value::Binary(public.public_key_bytes)),
                ])
            }

            "connected_peers" => {
                let ids = engine.connected_peer_ids();
                rmpv::Value::Map(vec![
                    (rmpv::Value::String("count".into()), rmpv::Value::Integer(ids.len().into())),
                    (rmpv::Value::String("peer_ids".into()), rmpv::Value::Array(
                        ids.into_iter().map(|id| rmpv::Value::String(id.into())).collect()
                    )),
                ])
            }

            "known_peers" => {
                let peers = engine.known_peers();
                let entries: Vec<rmpv::Value> = peers.into_iter().map(|p| {
                    rmpv::Value::Map(vec![
                        (rmpv::Value::String("peer_id".into()), rmpv::Value::String(p.peer_id.into())),
                        (rmpv::Value::String("endpoint".into()), rmpv::Value::String(p.endpoint.into())),
                        (rmpv::Value::String("status".into()), rmpv::Value::String(p.status.into())),
                        (rmpv::Value::String("ttl".into()), rmpv::Value::Integer(p.ttl.into())),
                    ])
                }).collect();
                rmpv::Value::Map(vec![
                    (rmpv::Value::String("peers".into()), rmpv::Value::Array(entries)),
                ])
            }

            "discovered_peers" => {
                let peers = engine.discovery().discovered_peers();
                let entries: Vec<rmpv::Value> = peers.into_iter().map(|p| {
                    let source_str = match p.source {
                        nodedb_transport::DiscoverySource::Mdns => "mdns",
                        nodedb_transport::DiscoverySource::Seed => "seed",
                        nodedb_transport::DiscoverySource::Gossip => "gossip",
                    };
                    rmpv::Value::Map(vec![
                        (rmpv::Value::String("peer_id".into()), rmpv::Value::String(p.peer_id.into())),
                        (rmpv::Value::String("endpoint".into()), rmpv::Value::String(p.endpoint.into())),
                        (rmpv::Value::String("source".into()), rmpv::Value::String(source_str.into())),
                    ])
                }).collect();
                rmpv::Value::Map(vec![
                    (rmpv::Value::String("peers".into()), rmpv::Value::Array(entries)),
                ])
            }

            "set_credential" => {
                let peer_id = match request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("peer_id")))
                    .and_then(|(_, v)| v.as_str())
                {
                    Some(p) => p.to_string(),
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'peer_id'");
                        return false;
                    }
                };
                let token = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("token")))
                    .and_then(|(_, v)| v.as_str())
                    .map(|s| s.to_string());

                if let Some(t) = token {
                    engine.set_peer_credential(&peer_id, nodedb_transport::PeerCredential::BearerToken(t));
                }
                rmpv::Value::Map(vec![
                    (rmpv::Value::String("ok".into()), rmpv::Value::Boolean(true)),
                ])
            }

            "set_mesh_secret" => {
                let secret = match request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("secret")))
                    .and_then(|(_, v)| v.as_str())
                {
                    Some(s) => s.to_string(),
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'secret'");
                        return false;
                    }
                };
                engine.set_mesh_secret(&secret);
                rmpv::Value::Map(vec![
                    (rmpv::Value::String("ok".into()), rmpv::Value::Boolean(true)),
                ])
            }

            "register_device" => {
                let peer_id = match request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("peer_id")))
                    .and_then(|(_, v)| v.as_str())
                {
                    Some(p) => p.to_string(),
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'peer_id'");
                        return false;
                    }
                };
                let public_key_hex = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("public_key_hex")))
                    .and_then(|(_, v)| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let user_id = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("user_id")))
                    .and_then(|(_, v)| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let device_name = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("device_name")))
                    .and_then(|(_, v)| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // Decode hex public key to bytes
                let public_key_bytes: Vec<u8> = if public_key_hex.len() == 64 {
                    (0..32).filter_map(|i| {
                        u8::from_str_radix(&public_key_hex[i*2..i*2+2], 16).ok()
                    }).collect()
                } else {
                    vec![]
                };

                match engine.register_device(&peer_id, &public_key_bytes, &user_id, &device_name) {
                    Ok(record) => {
                        rmpv::Value::Map(vec![
                            (rmpv::Value::String("ok".into()), rmpv::Value::Boolean(true)),
                            (rmpv::Value::String("peer_id".into()), rmpv::Value::String(record.peer_id.into())),
                        ])
                    }
                    Err(e) => {
                        set_error(out_error, ERR_INVALID_QUERY, &format!("register_device failed: {}", e));
                        return false;
                    }
                }
            }

            "connect" => {
                let endpoint = match request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("endpoint")))
                    .and_then(|(_, v)| v.as_str())
                {
                    Some(e) => e.to_string(),
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'endpoint'");
                        return false;
                    }
                };
                let rt = global_runtime();
                match rt.block_on(engine.connect_to_peer(&endpoint)) {
                    Ok(peer_id) => {
                        rmpv::Value::Map(vec![
                            (rmpv::Value::String("peer_id".into()), rmpv::Value::String(peer_id.into())),
                        ])
                    }
                    Err(e) => {
                        set_error(out_error, transport_error_code(&e), &e.to_string());
                        return false;
                    }
                }
            }

            "query" => {
                let payload = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("payload")))
                    .and_then(|(_, v)| match v {
                        rmpv::Value::Binary(b) => Some(b.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                let timeout = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("timeout_secs")))
                    .and_then(|(_, v)| v.as_u64())
                    .unwrap_or(10);

                let rt = global_runtime();
                match rt.block_on(engine.query(payload, timeout)) {
                    Ok(Some(data)) => {
                        rmpv::Value::Map(vec![
                            (rmpv::Value::String("result".into()), rmpv::Value::Binary(data)),
                        ])
                    }
                    Ok(None) => {
                        rmpv::Value::Map(vec![
                            (rmpv::Value::String("result".into()), rmpv::Value::Nil),
                        ])
                    }
                    Err(e) => {
                        set_error(out_error, transport_error_code(&e), &e.to_string());
                        return false;
                    }
                }
            }

            "audit_log" => {
                let offset = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("offset")))
                    .and_then(|(_, v)| v.as_u64())
                    .unwrap_or(0) as usize;
                let limit = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("limit")))
                    .and_then(|(_, v)| v.as_u64())
                    .unwrap_or(50) as usize;

                if let Some(audit) = engine.audit_log() {
                    let entries = match audit.recent(offset, limit) {
                        Ok(e) => e,
                        Err(e) => {
                            set_error(out_error, ERR_STORAGE, &e.to_string());
                            return false;
                        }
                    };
                    let arr: Vec<rmpv::Value> = entries.into_iter().map(|e| {
                        rmpv::Value::Map(vec![
                            (rmpv::Value::String("id".into()), rmpv::Value::Integer(e.id.into())),
                            (rmpv::Value::String("peer_id".into()), rmpv::Value::String(e.peer_id.into())),
                            (rmpv::Value::String("action".into()), rmpv::Value::String(e.action.into())),
                            (rmpv::Value::String("record_count".into()), rmpv::Value::Integer((e.record_count as i64).into())),
                        ])
                    }).collect();
                    rmpv::Value::Map(vec![
                        (rmpv::Value::String("count".into()), rmpv::Value::Integer(audit.count().into())),
                        (rmpv::Value::String("entries".into()), rmpv::Value::Array(arr)),
                    ])
                } else {
                    rmpv::Value::Map(vec![
                        (rmpv::Value::String("count".into()), rmpv::Value::Integer(0.into())),
                        (rmpv::Value::String("entries".into()), rmpv::Value::Array(vec![])),
                    ])
                }
            }

            "federated_query" => {
                let query_type = match request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("query_type")))
                    .and_then(|(_, v)| v.as_str())
                {
                    Some(t) => t.to_string(),
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'query_type'");
                        return false;
                    }
                };

                let query_data_value = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("query_data")))
                    .map(|(_, v)| v.clone())
                    .unwrap_or(rmpv::Value::Nil);

                let query_data_bytes = match rmp_serde::to_vec(&query_data_value) {
                    Ok(b) => b,
                    Err(e) => {
                        set_error(out_error, ERR_SERIALIZATION, &format!("query_data serialization: {}", e));
                        return false;
                    }
                };

                let timeout = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("timeout_secs")))
                    .and_then(|(_, v)| v.as_u64())
                    .unwrap_or(10);

                let ttl = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("ttl")))
                    .and_then(|(_, v)| v.as_u64())
                    .unwrap_or(3) as u8;

                let k_param = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("k")))
                    .and_then(|(_, v)| v.as_u64())
                    .unwrap_or(10) as usize;

                // Resolve engine handles from request for local execution
                let nosql_handle = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("nosql_handle")))
                    .and_then(|(_, v)| v.as_u64());
                let graph_handle = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("graph_handle")))
                    .and_then(|(_, v)| v.as_u64());
                let vector_handle = request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("vector_handle")))
                    .and_then(|(_, v)| v.as_u64());

                // Step 1: Execute locally
                let local_result = {
                    let handler = query_handler::FfiQueryHandler::new(
                        nosql_handle.and_then(get_db),
                        graph_handle.and_then(get_graph),
                        vector_handle.and_then(get_vector),
                        None, // No DAC for local queries (we are the originator)
                        None,
                    );
                    handler.handle_query(&query_type, &query_data_bytes, "local")
                        .unwrap_or_default()
                };

                // Step 2: Fan out to remote peers
                let envelope = nodedb_transport::FederatedQueryEnvelope {
                    query_type: query_type.clone(),
                    query_id: uuid::Uuid::new_v4().to_string(),
                    origin_peer_id: engine.identity().peer_id().to_string(),
                    ttl,
                    query_data: query_data_bytes,
                    visited: vec![engine.identity().peer_id().to_string()],
                };

                let envelope_bytes = match rmp_serde::to_vec(&envelope) {
                    Ok(b) => b,
                    Err(e) => {
                        set_error(out_error, ERR_SERIALIZATION, &format!("envelope serialization: {}", e));
                        return false;
                    }
                };

                let rt = global_runtime();
                let remote_results = match rt.block_on(engine.query_all(envelope_bytes, timeout)) {
                    Ok(results) => {
                        // Each result is a serialized FederatedQueryResponse
                        results.into_iter().filter_map(|data| {
                            let resp: nodedb_transport::FederatedQueryResponse =
                                rmp_serde::from_slice(&data).ok()?;
                            if resp.success {
                                Some(resp.result_data)
                            } else {
                                None
                            }
                        }).collect::<Vec<_>>()
                    }
                    Err(_) => vec![],
                };

                // Step 3: Merge local + remote results
                let merged = match query_type.as_str() {
                    "nosql" => merge::merge_nosql_results(&local_result, &remote_results),
                    "vector" => merge::merge_vector_results(&local_result, &remote_results, k_param),
                    "graph" => merge::merge_graph_results(&local_result, &remote_results),
                    _ => local_result,
                };

                // Return raw merged bytes as binary
                rmpv::Value::Map(vec![
                    (rmpv::Value::String("result".into()), rmpv::Value::Binary(merged)),
                    (rmpv::Value::String("remote_count".into()), rmpv::Value::Integer(remote_results.len().into())),
                ])
            }

            "mesh_status" => {
                match engine.mesh_status() {
                    Some(status) => {
                        rmpv::Value::Map(vec![
                            (rmpv::Value::String("mesh_name".into()), rmpv::Value::String(status.mesh_name.into())),
                            (rmpv::Value::String("database_name".into()), rmpv::Value::String(status.database_name.into())),
                            (rmpv::Value::String("sharing_status".into()), rmpv::Value::String(status.sharing_status.into())),
                            (rmpv::Value::String("member_count".into()), rmpv::Value::Integer((status.member_count as u64).into())),
                        ])
                    }
                    None => rmpv::Value::Nil,
                }
            }

            "mesh_members" => {
                let members = engine.mesh_members();
                let arr: Vec<rmpv::Value> = members.into_iter().map(|m| {
                    rmpv::Value::Map(vec![
                        (rmpv::Value::String("database_name".into()), rmpv::Value::String(m.database_name.into())),
                        (rmpv::Value::String("peer_id".into()), rmpv::Value::String(m.peer_id.into())),
                        (rmpv::Value::String("sharing_status".into()), rmpv::Value::String(m.sharing_status.into())),
                        (rmpv::Value::String("last_seen".into()), rmpv::Value::String(m.last_seen.to_rfc3339().into())),
                    ])
                }).collect();
                rmpv::Value::Array(arr)
            }

            "mesh_query" => {
                let database = map_field(&request, "database")
                    .and_then(|v| v.as_str().map(|s| s.to_string()));
                let database = match database {
                    Some(d) => d,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "mesh_query requires 'database' field");
                        return false;
                    }
                };
                let query_type = map_field(&request, "query_type")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| "nosql".to_string());
                let query_data_val = map_field(&request, "query_data").cloned().unwrap_or(rmpv::Value::Nil);
                let query_data = match rmp_serde::to_vec(&query_data_val) {
                    Ok(b) => b,
                    Err(e) => {
                        set_error(out_error, ERR_SERIALIZATION, &e.to_string());
                        return false;
                    }
                };
                let timeout_secs = map_field(&request, "timeout_secs")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10);

                let rt = tokio::runtime::Runtime::new().unwrap();
                match rt.block_on(engine.mesh_query(&database, &query_type, query_data, timeout_secs)) {
                    Ok(results) => {
                        let arr: Vec<rmpv::Value> = results.into_iter()
                            .map(|data| rmpv::Value::Binary(data))
                            .collect();
                        rmpv::Value::Map(vec![
                            (rmpv::Value::String("results".into()), rmpv::Value::Array(arr)),
                        ])
                    }
                    Err(e) => {
                        set_error(out_error, ERR_TRANSPORT_SEND, &e.to_string());
                        return false;
                    }
                }
            }

            "paired_devices" => {
                let devices: Vec<rmpv::Value> = engine.paired_devices().into_iter().map(|r| {
                    rmpv::Value::Map(vec![
                        (rmpv::Value::String("peer_id".into()), rmpv::Value::String(r.peer_id.into())),
                        (rmpv::Value::String("user_id".into()), rmpv::Value::String(r.user_id.into())),
                        (rmpv::Value::String("device_name".into()), rmpv::Value::String(r.device_name.into())),
                        (rmpv::Value::String("paired_at".into()), rmpv::Value::String(r.paired_at.to_rfc3339().into())),
                        (rmpv::Value::String("last_verified_at".into()), rmpv::Value::String(r.last_verified_at.to_rfc3339().into())),
                    ])
                }).collect();
                rmpv::Value::Map(vec![
                    (rmpv::Value::String("devices".into()), rmpv::Value::Array(devices)),
                ])
            }

            "pending_pairings" => {
                let pending: Vec<rmpv::Value> = engine.pending_pairings().into_iter().map(|r| {
                    rmpv::Value::Map(vec![
                        (rmpv::Value::String("peer_id".into()), rmpv::Value::String(r.peer_id.into())),
                        (rmpv::Value::String("user_id".into()), rmpv::Value::String(r.user_id.into())),
                        (rmpv::Value::String("device_name".into()), rmpv::Value::String(r.device_name.into())),
                        (rmpv::Value::String("endpoint".into()), rmpv::Value::String(r.endpoint.into())),
                        (rmpv::Value::String("received_at".into()), rmpv::Value::String(r.received_at.to_rfc3339().into())),
                    ])
                }).collect();
                rmpv::Value::Map(vec![
                    (rmpv::Value::String("pending".into()), rmpv::Value::Array(pending)),
                ])
            }

            "approve_pairing" => {
                let peer_id = match request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("peer_id")))
                    .and_then(|(_, v)| v.as_str())
                {
                    Some(p) => p.to_string(),
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'peer_id'");
                        return false;
                    }
                };
                match engine.approve_pairing(&peer_id) {
                    Ok(Some(record)) => {
                        rmpv::Value::Map(vec![
                            (rmpv::Value::String("ok".into()), rmpv::Value::Boolean(true)),
                            (rmpv::Value::String("peer_id".into()), rmpv::Value::String(record.peer_id.into())),
                            (rmpv::Value::String("user_id".into()), rmpv::Value::String(record.user_id.into())),
                        ])
                    }
                    Ok(None) => {
                        set_error(out_error, ERR_PAIRING_NOT_FOUND, "pending pairing not found");
                        return false;
                    }
                    Err(e) => {
                        set_error(out_error, ERR_PAIRING_ERROR, &e.to_string());
                        return false;
                    }
                }
            }

            "reject_pairing" => {
                let peer_id = match request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("peer_id")))
                    .and_then(|(_, v)| v.as_str())
                {
                    Some(p) => p.to_string(),
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'peer_id'");
                        return false;
                    }
                };
                let removed = engine.reject_pairing(&peer_id);
                rmpv::Value::Map(vec![
                    (rmpv::Value::String("removed".into()), rmpv::Value::Boolean(removed)),
                ])
            }

            "remove_paired_device" => {
                let peer_id = match request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("peer_id")))
                    .and_then(|(_, v)| v.as_str())
                {
                    Some(p) => p.to_string(),
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'peer_id'");
                        return false;
                    }
                };
                match engine.remove_paired_device(&peer_id) {
                    Ok(removed) => {
                        rmpv::Value::Map(vec![
                            (rmpv::Value::String("removed".into()), rmpv::Value::Boolean(removed)),
                        ])
                    }
                    Err(e) => {
                        set_error(out_error, ERR_STORAGE, &e.to_string());
                        return false;
                    }
                }
            }

            other => {
                set_error(out_error, ERR_INVALID_QUERY, &format!("unknown transport action: {}", other));
                return false;
            }
        };

        let response_bytes = match rmp_serde::to_vec(&response) {
            Ok(b) => b,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &e.to_string());
                return false;
            }
        };

        let len = response_bytes.len();
        let ptr = response_bytes.as_ptr();
        std::mem::forget(response_bytes);

        unsafe {
            *out_response = ptr as *mut u8;
            *out_response_len = len;
        }

        clear_error(out_error);
        true
    });

    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_transport_execute");
            false
        }
    }
}

// ── Provenance FFI ──────────────────────────────────────────────────────────

/// Open a provenance engine. config is MessagePack with "path" key.
#[no_mangle]
pub extern "C" fn nodedb_provenance_open(
    config_ptr: *const u8,
    config_len: usize,
    out_handle: *mut ProvenanceHandle,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if config_ptr.is_null() || out_handle.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let config_bytes = unsafe { std::slice::from_raw_parts(config_ptr, config_len) };

        let config: rmpv::Value = match rmp_serde::from_slice(config_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &format!("invalid config: {}", e));
                return false;
            }
        };

        let path_str = match config.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("path")))
            .and_then(|(_, v)| v.as_str())
        {
            Some(p) => p.to_string(),
            None => {
                set_error(out_error, ERR_SERIALIZATION, "config missing 'path' field");
                return false;
            }
        };

        let path = PathBuf::from(&path_str);
        let storage = match StorageEngine::open(&path) {
            Ok(e) => Arc::new(e),
            Err(e) => {
                set_error(out_error, ERR_STORAGE, &e.to_string());
                return false;
            }
        };

        match ProvenanceEngine::new(storage) {
            Ok(prov) => {
                let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
                let mut map = provenance_handle_map().write().unwrap();
                map.insert(handle, Arc::new(prov));
                unsafe { *out_handle = handle; }
                clear_error(out_error);
                true
            }
            Err(e) => {
                set_error(out_error, provenance_error_code(&e), &e.to_string());
                false
            }
        }
    });

    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_provenance_open");
            false
        }
    }
}

/// Close a provenance engine and release resources.
#[no_mangle]
pub extern "C" fn nodedb_provenance_close(handle: ProvenanceHandle) {
    let _ = catch_unwind(|| {
        let mut map = provenance_handle_map().write().unwrap();
        map.remove(&handle);
    });
}

/// Execute a provenance operation. request is MessagePack with an "action" field.
#[no_mangle]
pub extern "C" fn nodedb_provenance_execute(
    handle: ProvenanceHandle,
    request_ptr: *const u8,
    request_len: usize,
    out_response: *mut *mut u8,
    out_response_len: *mut usize,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if request_ptr.is_null() || out_response.is_null() || out_response_len.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let prov = match get_provenance(handle) {
            Some(p) => p,
            None => {
                set_error(out_error, ERR_INVALID_HANDLE, "invalid provenance handle");
                return false;
            }
        };

        let request_bytes = unsafe { std::slice::from_raw_parts(request_ptr, request_len) };

        let request: rmpv::Value = match rmp_serde::from_slice(request_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &format!("invalid request: {}", e));
                return false;
            }
        };

        let action = match request.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("action")))
            .and_then(|(_, v)| v.as_str())
        {
            Some(a) => a.to_string(),
            None => {
                set_error(out_error, ERR_INVALID_QUERY, "missing 'action' field");
                return false;
            }
        };

        let get_str = |key: &str| -> Option<String> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_str())
                .map(|s| s.to_string())
        };

        let get_i64 = |key: &str| -> Option<i64> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_i64())
        };

        let get_f64 = |key: &str| -> Option<f64> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_f64())
        };

        let get_bool = |key: &str| -> Option<bool> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_bool())
        };

        let get_value = |key: &str| -> rmpv::Value {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .map(|(_, v)| v.clone())
                .unwrap_or(rmpv::Value::Nil)
        };

        let get_u64 = |key: &str| -> Option<u64> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_u64())
        };

        let response_value: rmpv::Value = match action.as_str() {
            "attach" => {
                let collection = match get_str("collection") {
                    Some(c) => c,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'collection' field");
                        return false;
                    }
                };
                let record_id = match get_i64("record_id") {
                    Some(id) => id,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'record_id' field");
                        return false;
                    }
                };
                let source_id = match get_str("source_id") {
                    Some(s) => s,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'source_id' field");
                        return false;
                    }
                };
                let source_type = ProvenanceSourceType::from_str(
                    &get_str("source_type").unwrap_or_else(|| "unknown".to_string())
                );
                let content_hash = match get_str("content_hash") {
                    Some(h) => h,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'content_hash' field");
                        return false;
                    }
                };
                let pki_signature = get_str("pki_signature");
                let pki_id = get_str("pki_id");
                let user_id = get_str("user_id");
                let is_signed = get_bool("is_signed").unwrap_or(pki_signature.is_some());
                let hops = get_u64("hops").unwrap_or(0) as u8;
                let created_at_utc = get_str("created_at_utc");
                let data_updated_at_utc = get_str("data_updated_at_utc");
                let local_id = get_str("local_id");
                let global_id = get_str("global_id");

                match prov.attach(&collection, record_id, &source_id, source_type, content_hash, pki_signature, pki_id, user_id, is_signed, hops, created_at_utc, data_updated_at_utc, local_id, global_id) {
                    Ok(env) => match rmpv::ext::to_value(&env) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, provenance_error_code(&e), &e.to_string()); return false; }
                }
            }
            "get" => {
                let id = get_i64("id").unwrap_or(0);
                match prov.get(id) {
                    Ok(Some(env)) => match rmpv::ext::to_value(&env) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Ok(None) => {
                        set_error(out_error, ERR_PROVENANCE_NOT_FOUND, &format!("envelope {} not found", id));
                        return false;
                    }
                    Err(e) => { set_error(out_error, provenance_error_code(&e), &e.to_string()); return false; }
                }
            }
            "get_for_record" => {
                let collection = match get_str("collection") {
                    Some(c) => c,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'collection' field");
                        return false;
                    }
                };
                let record_id = match get_i64("record_id") {
                    Some(id) => id,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'record_id' field");
                        return false;
                    }
                };
                match prov.get_for_record(&collection, record_id) {
                    Ok(envs) => {
                        let arr: Vec<rmpv::Value> = envs.into_iter()
                            .filter_map(|e| rmpv::ext::to_value(&e).ok())
                            .collect();
                        rmpv::Value::Array(arr)
                    }
                    Err(e) => { set_error(out_error, provenance_error_code(&e), &e.to_string()); return false; }
                }
            }
            "corroborate" => {
                let id = match get_i64("id") {
                    Some(id) => id,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'id' field");
                        return false;
                    }
                };
                let new_source_confidence = match get_f64("new_source_confidence") {
                    Some(c) => c,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'new_source_confidence' field");
                        return false;
                    }
                };
                match prov.corroborate(id, new_source_confidence) {
                    Ok(env) => match rmpv::ext::to_value(&env) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, provenance_error_code(&e), &e.to_string()); return false; }
                }
            }
            "verify" => {
                let id = match get_i64("id") {
                    Some(id) => id,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'id' field");
                        return false;
                    }
                };
                let public_key_hex = match get_str("public_key") {
                    Some(k) => k,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'public_key' field");
                        return false;
                    }
                };
                let public_key_bytes = match hex_decode_ffi(&public_key_hex) {
                    Ok(b) => b,
                    Err(e) => {
                        set_error(out_error, ERR_PROVENANCE_VERIFICATION, &format!("invalid public key hex: {}", e));
                        return false;
                    }
                };
                match prov.verify(id, &public_key_bytes) {
                    Ok(env) => match rmpv::ext::to_value(&env) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, provenance_error_code(&e), &e.to_string()); return false; }
                }
            }
            "update_confidence" => {
                let id = match get_i64("id") {
                    Some(id) => id,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'id' field");
                        return false;
                    }
                };
                let confidence = match get_f64("confidence") {
                    Some(c) => c,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'confidence' field");
                        return false;
                    }
                };
                match prov.update_confidence(id, confidence) {
                    Ok(env) => match rmpv::ext::to_value(&env) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, provenance_error_code(&e), &e.to_string()); return false; }
                }
            }
            "delete" => {
                let id = get_i64("id").unwrap_or(0);
                match prov.delete(id) {
                    Ok(deleted) => rmpv::Value::Boolean(deleted),
                    Err(e) => { set_error(out_error, provenance_error_code(&e), &e.to_string()); return false; }
                }
            }
            "query" => {
                let collection = get_str("collection");
                let source_type = get_str("source_type").map(|s| ProvenanceSourceType::from_str(&s));
                let verification_status = get_str("verification_status").map(|s| ProvenanceVerificationStatus::from_str(&s));
                let min_confidence = get_f64("min_confidence");

                match prov.query(
                    collection.as_deref(),
                    source_type.as_ref(),
                    verification_status.as_ref(),
                    min_confidence,
                ) {
                    Ok(envs) => {
                        let arr: Vec<rmpv::Value> = envs.into_iter()
                            .filter_map(|e| rmpv::ext::to_value(&e).ok())
                            .collect();
                        rmpv::Value::Array(arr)
                    }
                    Err(e) => { set_error(out_error, provenance_error_code(&e), &e.to_string()); return false; }
                }
            }
            "count" => {
                match prov.envelope_count() {
                    Ok(n) => rmpv::Value::Integer((n as i64).into()),
                    Err(e) => { set_error(out_error, provenance_error_code(&e), &e.to_string()); return false; }
                }
            }
            "compute_hash" => {
                let data = get_value("data");
                match prov.compute_hash(&data) {
                    Ok(hash) => rmpv::Value::String(hash.into()),
                    Err(e) => { set_error(out_error, provenance_error_code(&e), &e.to_string()); return false; }
                }
            }
            other => {
                set_error(out_error, ERR_INVALID_QUERY, &format!("unknown provenance action: {}", other));
                return false;
            }
        };

        // Serialize response
        let response_bytes = match rmp_serde::to_vec(&response_value) {
            Ok(b) => b,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &e.to_string());
                return false;
            }
        };

        let len = response_bytes.len();
        let ptr = response_bytes.as_ptr();
        std::mem::forget(response_bytes);

        unsafe {
            *out_response = ptr as *mut u8;
            *out_response_len = len;
        }

        clear_error(out_error);
        true
    });

    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_provenance_execute");
            false
        }
    }
}

fn hex_decode_ffi(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err("odd length".to_string());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| e.to_string())
        })
        .collect()
}

// ── Key Resolver FFI ──────────────────────────────────────────────────

/// Open a key resolver engine. Config is MessagePack with a "path" field.
#[no_mangle]
pub extern "C" fn nodedb_keyresolver_open(
    config_ptr: *const u8,
    config_len: usize,
    out_handle: *mut KeyResolverHandle,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if config_ptr.is_null() || out_handle.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let config_bytes = unsafe { std::slice::from_raw_parts(config_ptr, config_len) };

        let config: rmpv::Value = match rmp_serde::from_slice(config_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &format!("invalid config: {}", e));
                return false;
            }
        };

        let path_str = match config.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("path")))
            .and_then(|(_, v)| v.as_str())
        {
            Some(p) => p.to_string(),
            None => {
                set_error(out_error, ERR_SERIALIZATION, "config missing 'path' field");
                return false;
            }
        };

        let path = PathBuf::from(&path_str);
        let storage = match StorageEngine::open(&path) {
            Ok(e) => Arc::new(e),
            Err(e) => {
                set_error(out_error, ERR_STORAGE, &e.to_string());
                return false;
            }
        };

        let id_gen = match nodedb_storage::IdGenerator::new(&storage) {
            Ok(g) => Arc::new(g),
            Err(e) => {
                set_error(out_error, ERR_STORAGE, &e.to_string());
                return false;
            }
        };

        match KeyResolverEngine::new(storage, id_gen) {
            Ok(kr) => {
                let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
                let mut map = keyresolver_handle_map().write().unwrap();
                map.insert(handle, Arc::new(kr));
                unsafe { *out_handle = handle; }
                clear_error(out_error);
                true
            }
            Err(e) => {
                set_error(out_error, keyresolver_error_code(&e), &e.to_string());
                false
            }
        }
    });

    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_keyresolver_open");
            false
        }
    }
}

/// Close a key resolver engine and release resources.
#[no_mangle]
pub extern "C" fn nodedb_keyresolver_close(handle: KeyResolverHandle) {
    let _ = catch_unwind(|| {
        let mut map = keyresolver_handle_map().write().unwrap();
        map.remove(&handle);
    });
}

/// Execute a key resolver operation. Request is MessagePack with an "action" field.
#[no_mangle]
pub extern "C" fn nodedb_keyresolver_execute(
    handle: KeyResolverHandle,
    request_ptr: *const u8,
    request_len: usize,
    out_response: *mut *mut u8,
    out_response_len: *mut usize,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if request_ptr.is_null() || out_response.is_null() || out_response_len.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let kr = match get_keyresolver(handle) {
            Some(k) => k,
            None => {
                set_error(out_error, ERR_INVALID_HANDLE, "invalid keyresolver handle");
                return false;
            }
        };

        let request_bytes = unsafe { std::slice::from_raw_parts(request_ptr, request_len) };

        let request: rmpv::Value = match rmp_serde::from_slice(request_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &format!("invalid request: {}", e));
                return false;
            }
        };

        let action = match request.as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("action")))
            .and_then(|(_, v)| v.as_str())
        {
            Some(a) => a.to_string(),
            None => {
                set_error(out_error, ERR_INVALID_QUERY, "missing 'action' field");
                return false;
            }
        };

        let get_str = |key: &str| -> Option<String> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_str())
                .map(|s| s.to_string())
        };

        let get_i64 = |key: &str| -> Option<i64> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_i64())
        };

        let get_bool = |key: &str| -> Option<bool> {
            request.as_map()
                .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
                .and_then(|(_, v)| v.as_bool())
        };

        let response_value: rmpv::Value = match action.as_str() {
            "supply_key" => {
                let pki_id = match get_str("pki_id") {
                    Some(s) => s,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'pki_id' field");
                        return false;
                    }
                };
                let user_id = match get_str("user_id") {
                    Some(s) => s,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'user_id' field");
                        return false;
                    }
                };
                let public_key_hex = match get_str("public_key_hex") {
                    Some(s) => s,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'public_key_hex' field");
                        return false;
                    }
                };
                let trust_level = KeyTrustLevel::from_str(
                    &get_str("trust_level").unwrap_or_else(|| "explicit".to_string())
                );
                let expires_at_utc = get_str("expires_at_utc");

                match kr.supply_key(&pki_id, &user_id, &public_key_hex, trust_level, expires_at_utc) {
                    Ok(entry) => match rmpv::ext::to_value(&entry) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, keyresolver_error_code(&e), &e.to_string()); return false; }
                }
            }
            "revoke_key" => {
                let pki_id = match get_str("pki_id") {
                    Some(s) => s,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'pki_id' field");
                        return false;
                    }
                };
                let user_id = match get_str("user_id") {
                    Some(s) => s,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'user_id' field");
                        return false;
                    }
                };
                match kr.revoke_key(&pki_id, &user_id) {
                    Ok(entry) => match rmpv::ext::to_value(&entry) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, keyresolver_error_code(&e), &e.to_string()); return false; }
                }
            }
            "get_key" => {
                let pki_id = match get_str("pki_id") {
                    Some(s) => s,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'pki_id' field");
                        return false;
                    }
                };
                let user_id = match get_str("user_id") {
                    Some(s) => s,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'user_id' field");
                        return false;
                    }
                };
                match kr.get_key(&pki_id, &user_id) {
                    Ok(entry) => match rmpv::ext::to_value(&entry) {
                        Ok(v) => v,
                        Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                    },
                    Err(e) => { set_error(out_error, keyresolver_error_code(&e), &e.to_string()); return false; }
                }
            }
            "all_keys" => {
                match kr.all_keys() {
                    Ok(keys) => {
                        let arr: Vec<rmpv::Value> = keys.into_iter()
                            .filter_map(|e| rmpv::ext::to_value(&e).ok())
                            .collect();
                        rmpv::Value::Array(arr)
                    }
                    Err(e) => { set_error(out_error, keyresolver_error_code(&e), &e.to_string()); return false; }
                }
            }
            "key_count" => {
                match kr.key_count() {
                    Ok(n) => rmpv::Value::Integer((n as i64).into()),
                    Err(e) => { set_error(out_error, keyresolver_error_code(&e), &e.to_string()); return false; }
                }
            }
            "delete_key" => {
                let id = match get_i64("id") {
                    Some(id) => id,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'id' field");
                        return false;
                    }
                };
                match kr.delete_key(id) {
                    Ok(()) => rmpv::Value::Boolean(true),
                    Err(e) => { set_error(out_error, keyresolver_error_code(&e), &e.to_string()); return false; }
                }
            }
            "set_trust_all" => {
                let enabled = get_bool("enabled").unwrap_or(false);
                kr.set_trust_all(enabled);
                rmpv::Value::Boolean(true)
            }
            "set_trust_all_for_peer" => {
                let peer_id = match get_str("peer_id") {
                    Some(s) => s,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'peer_id' field");
                        return false;
                    }
                };
                let enabled = get_bool("enabled").unwrap_or(false);
                kr.set_trust_all_for_peer(&peer_id, enabled);
                rmpv::Value::Boolean(true)
            }
            "is_trust_all_active" => {
                rmpv::Value::Boolean(kr.is_trust_all_active())
            }
            "verify_with_cache" => {
                let provenance_handle = match request.as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("provenance_handle")))
                    .and_then(|(_, v)| v.as_u64())
                {
                    Some(h) => h,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'provenance_handle' field");
                        return false;
                    }
                };
                let envelope_id = match get_i64("envelope_id") {
                    Some(id) => id,
                    None => {
                        set_error(out_error, ERR_INVALID_QUERY, "missing 'envelope_id' field");
                        return false;
                    }
                };

                let prov = match get_provenance(provenance_handle) {
                    Some(p) => p,
                    None => {
                        set_error(out_error, ERR_INVALID_HANDLE, "invalid provenance_handle");
                        return false;
                    }
                };

                // Get the envelope
                let envelope = match prov.get(envelope_id) {
                    Ok(Some(e)) => e,
                    Ok(None) => {
                        set_error(out_error, ERR_PROVENANCE_NOT_FOUND, &format!("envelope {} not found", envelope_id));
                        return false;
                    }
                    Err(e) => {
                        set_error(out_error, provenance_error_code(&e), &e.to_string());
                        return false;
                    }
                };

                let pki_id = match &envelope.pki_id {
                    Some(id) => id.clone(),
                    None => {
                        // No PKI info — can't verify, set to Unverified
                        match rmpv::ext::to_value(&envelope) {
                            Ok(v) => return write_response(out_response, out_response_len, out_error, v),
                            Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                        }
                    }
                };
                let user_id = envelope.user_id.clone().unwrap_or_default();

                // Resolve key from cache
                match kr.resolve_for_verification(&pki_id, &user_id) {
                    Ok(KeyResolutionResult::Found(key_entry)) => {
                        if key_entry.trust_level == KeyTrustLevel::Revoked {
                            // Revoked key → Failed + confidence 0.0
                            let _ = prov.update_confidence(envelope_id, 0.0);
                            match prov.get(envelope_id) {
                                Ok(Some(updated)) => match rmpv::ext::to_value(&updated) {
                                    Ok(v) => v,
                                    Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                                },
                                _ => { set_error(out_error, ERR_INTERNAL, "failed to re-read envelope"); return false; }
                            }
                        } else {
                            // Explicit or TrustAll key — verify signature
                            let key_bytes = match hex_decode_ffi(&key_entry.public_key_hex) {
                                Ok(b) => b,
                                Err(e) => {
                                    set_error(out_error, ERR_KEYRESOLVER_INVALID_HEX, &format!("invalid key hex: {}", e));
                                    return false;
                                }
                            };
                            match prov.verify(envelope_id, &key_bytes) {
                                Ok(env) => match rmpv::ext::to_value(&env) {
                                    Ok(v) => v,
                                    Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                                },
                                Err(e) => { set_error(out_error, provenance_error_code(&e), &e.to_string()); return false; }
                            }
                        }
                    }
                    Ok(KeyResolutionResult::NotFound) => {
                        // Check trust-all mode
                        if kr.is_trust_all_for_peer(&pki_id) {
                            // Trust-all → set status to TrustAll
                            let updated = nodedb_provenance::ProvenanceEnvelope {
                                verification_status: ProvenanceVerificationStatus::TrustAll,
                                ..envelope
                            };
                            // Persist the status update via update_confidence (keeps current confidence)
                            // We need a way to update verification status — use verify lifecycle
                            // For trust-all, just return the envelope with TrustAll status
                            match rmpv::ext::to_value(&updated) {
                                Ok(v) => v,
                                Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                            }
                        } else {
                            // No key, no trust-all → KeyRequested
                            let updated = nodedb_provenance::ProvenanceEnvelope {
                                verification_status: ProvenanceVerificationStatus::KeyRequested,
                                ..envelope
                            };
                            match rmpv::ext::to_value(&updated) {
                                Ok(v) => v,
                                Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                            }
                        }
                    }
                    Ok(KeyResolutionResult::Expired) => {
                        // Expired key → KeyRequested
                        let updated = nodedb_provenance::ProvenanceEnvelope {
                            verification_status: ProvenanceVerificationStatus::KeyRequested,
                            ..envelope
                        };
                        match rmpv::ext::to_value(&updated) {
                            Ok(v) => v,
                            Err(e) => { set_error(out_error, ERR_SERIALIZATION, &e.to_string()); return false; }
                        }
                    }
                    Err(e) => {
                        set_error(out_error, keyresolver_error_code(&e), &e.to_string());
                        return false;
                    }
                }
            }
            other => {
                set_error(out_error, ERR_INVALID_QUERY, &format!("unknown keyresolver action: {}", other));
                return false;
            }
        };

        // Serialize response
        let response_bytes = match rmp_serde::to_vec(&response_value) {
            Ok(b) => b,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &e.to_string());
                return false;
            }
        };

        let len = response_bytes.len();
        let ptr = response_bytes.as_ptr();
        std::mem::forget(response_bytes);

        unsafe {
            *out_response = ptr as *mut u8;
            *out_response_len = len;
        }

        clear_error(out_error);
        true
    });

    match result {
        Ok(v) => v,
        Err(_) => {
            set_error(out_error, ERR_INTERNAL, "panic in nodedb_keyresolver_execute");
            false
        }
    }
}

fn ai_provenance_handle_map() -> &'static RwLock<HashMap<AiProvenanceHandle, Arc<AiProvenanceEngine>>> {
    use std::sync::OnceLock;
    static MAP: OnceLock<RwLock<HashMap<AiProvenanceHandle, Arc<AiProvenanceEngine>>>> = OnceLock::new();
    MAP.get_or_init(|| RwLock::new(HashMap::new()))
}

fn get_ai_provenance(handle: AiProvenanceHandle) -> Option<Arc<AiProvenanceEngine>> {
    let map = ai_provenance_handle_map().read().ok()?;
    map.get(&handle).cloned()
}

fn ai_provenance_error_code(e: &AiProvenanceError) -> i32 {
    match e {
        AiProvenanceError::EnvelopeNotFound(_) => ERR_AI_PROVENANCE_ENVELOPE_NOT_FOUND,
        AiProvenanceError::InvalidConfidence(_) => ERR_AI_PROVENANCE_INVALID_CONFIDENCE,
        AiProvenanceError::CollectionNotEnabled(_) => ERR_AI_PROVENANCE_COLLECTION_NOT_ENABLED,
        AiProvenanceError::ConfigError(_) => ERR_AI_PROVENANCE_CONFIG,
        AiProvenanceError::Provenance(_) => ERR_STORAGE,
    }
}

fn map_field<'a>(val: &'a rmpv::Value, key: &str) -> Option<&'a rmpv::Value> {
    val.as_map()
        .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)))
        .map(|(_, v)| v)
}

/// Open an AI provenance engine. Config must include "provenance_handle" (u64).
#[no_mangle]
pub extern "C" fn nodedb_ai_provenance_open(
    config_ptr: *const u8,
    config_len: usize,
    out_handle: *mut AiProvenanceHandle,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if config_ptr.is_null() || out_handle.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let config_bytes = unsafe { std::slice::from_raw_parts(config_ptr, config_len) };
        let config: rmpv::Value = match rmp_serde::from_slice(config_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &e.to_string());
                return false;
            }
        };

        let prov_handle = map_field(&config, "provenance_handle")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let prov = match get_provenance(prov_handle) {
            Some(p) => p,
            None => {
                set_error(out_error, ERR_INVALID_HANDLE, "invalid provenance_handle");
                return false;
            }
        };

        let ai_blend_weight = map_field(&config, "ai_blend_weight")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.3);

        let enabled_collections: Vec<String> = map_field(&config, "enabled_collections")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let response_timeout_secs = map_field(&config, "response_timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(5);

        let silent_on_error = map_field(&config, "silent_on_error")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let rate_limit_per_minute = map_field(&config, "rate_limit_per_minute")
            .and_then(|v| v.as_u64())
            .unwrap_or(60) as u32;

        let ai_config = AiProvenanceConfig {
            ai_blend_weight,
            enabled_collections,
            response_timeout_secs,
            silent_on_error,
            rate_limit_per_minute,
        };

        let engine = AiProvenanceEngine::new(prov, ai_config);
        let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
        ai_provenance_handle_map().write().unwrap().insert(handle, Arc::new(engine));
        unsafe { *out_handle = handle; }
        clear_error(out_error);
        true
    });
    result.unwrap_or_else(|_| {
        set_error(out_error, ERR_INTERNAL, "panic in nodedb_ai_provenance_open");
        false
    })
}

#[no_mangle]
pub extern "C" fn nodedb_ai_provenance_close(handle: AiProvenanceHandle) {
    ai_provenance_handle_map().write().unwrap().remove(&handle);
}

#[no_mangle]
pub extern "C" fn nodedb_ai_provenance_execute(
    handle: AiProvenanceHandle,
    request_ptr: *const u8,
    request_len: usize,
    out_response: *mut *mut u8,
    out_response_len: *mut usize,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if request_ptr.is_null() || out_response.is_null() || out_response_len.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let engine = match get_ai_provenance(handle) {
            Some(e) => e,
            None => {
                set_error(out_error, ERR_INVALID_HANDLE, "invalid ai_provenance handle");
                return false;
            }
        };

        let request_bytes = unsafe { std::slice::from_raw_parts(request_ptr, request_len) };
        let request: rmpv::Value = match rmp_serde::from_slice(request_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &e.to_string());
                return false;
            }
        };

        let action = map_field(&request, "action").and_then(|v| v.as_str()).unwrap_or("");

        match action {
            "apply_assessment" => {
                let envelope_id = map_field(&request, "envelope_id").and_then(|v| v.as_i64()).unwrap_or(0);
                let suggested_confidence = map_field(&request, "suggested_confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let source_type = map_field(&request, "source_type").and_then(|v| v.as_str()).map(|s| s.to_string());
                let reasoning = map_field(&request, "reasoning").and_then(|v| v.as_str()).map(|s| s.to_string());
                let tags: Option<std::collections::HashMap<String, String>> = map_field(&request, "tags")
                    .and_then(|v| v.as_map())
                    .map(|entries| {
                        entries.iter()
                            .filter_map(|(k, v)| {
                                Some((k.as_str()?.to_string(), v.as_str()?.to_string()))
                            })
                            .collect()
                    });

                let assessment = AiProvenanceAssessment {
                    envelope_id,
                    suggested_confidence,
                    source_type,
                    reasoning,
                    tags,
                };

                match engine.apply_assessment(&assessment) {
                    Ok(()) => {
                        let resp = rmpv::Value::Map(vec![
                            (rmpv::Value::String("ok".into()), rmpv::Value::Boolean(true)),
                        ]);
                        write_response(out_response, out_response_len, out_error, resp)
                    }
                    Err(e) => {
                        set_error(out_error, ai_provenance_error_code(&e), &e.to_string());
                        false
                    }
                }
            }

            "apply_conflict_resolution" => {
                let envelope_id_a = map_field(&request, "envelope_id_a").and_then(|v| v.as_i64()).unwrap_or(0);
                let envelope_id_b = map_field(&request, "envelope_id_b").and_then(|v| v.as_i64()).unwrap_or(0);
                let confidence_delta_a = map_field(&request, "confidence_delta_a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let confidence_delta_b = map_field(&request, "confidence_delta_b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let preference_str = map_field(&request, "preference").and_then(|v| v.as_str()).unwrap_or("prefer_neither");
                let reasoning = map_field(&request, "reasoning").and_then(|v| v.as_str()).map(|s| s.to_string());

                let resolution = AiConflictResolution {
                    envelope_id_a,
                    envelope_id_b,
                    confidence_delta_a,
                    confidence_delta_b,
                    preference: ConflictPreference::from_str(preference_str),
                    reasoning,
                };

                match engine.apply_conflict_resolution(&resolution) {
                    Ok(()) => {
                        let resp = rmpv::Value::Map(vec![
                            (rmpv::Value::String("ok".into()), rmpv::Value::Boolean(true)),
                        ]);
                        write_response(out_response, out_response_len, out_error, resp)
                    }
                    Err(e) => {
                        set_error(out_error, ai_provenance_error_code(&e), &e.to_string());
                        false
                    }
                }
            }

            "apply_anomaly_flags" => {
                let collection = map_field(&request, "collection").and_then(|v| v.as_str()).unwrap_or("");
                let flags_val = map_field(&request, "flags").and_then(|v| v.as_array());

                let flags: Vec<AiAnomalyFlag> = match flags_val {
                    Some(arr) => arr.iter().map(|f| {
                        AiAnomalyFlag {
                            record_id: map_field(f, "record_id").and_then(|v| v.as_i64()).unwrap_or(0),
                            confidence_penalty: map_field(f, "confidence_penalty").and_then(|v| v.as_f64()).unwrap_or(0.0),
                            reason: map_field(f, "reason").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            severity: map_field(f, "severity").and_then(|v| v.as_str()).unwrap_or("low").to_string(),
                        }
                    }).collect(),
                    None => Vec::new(),
                };

                match engine.apply_anomaly_flags(collection, &flags) {
                    Ok(affected) => {
                        let resp = rmpv::Value::Map(vec![
                            (rmpv::Value::String("ok".into()), rmpv::Value::Boolean(true)),
                            (rmpv::Value::String("affected".into()), rmpv::Value::Integer((affected as i64).into())),
                        ]);
                        write_response(out_response, out_response_len, out_error, resp)
                    }
                    Err(e) => {
                        set_error(out_error, ai_provenance_error_code(&e), &e.to_string());
                        false
                    }
                }
            }

            "apply_source_classification" => {
                let envelope_id = map_field(&request, "envelope_id").and_then(|v| v.as_i64()).unwrap_or(0);
                let source_type = map_field(&request, "source_type").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                let credibility_prior = map_field(&request, "credibility_prior").and_then(|v| v.as_f64()).unwrap_or(0.5);
                let reasoning = map_field(&request, "reasoning").and_then(|v| v.as_str()).map(|s| s.to_string());

                let classification = AiSourceClassification {
                    envelope_id,
                    source_type,
                    credibility_prior,
                    reasoning,
                };

                match engine.apply_source_classification(&classification) {
                    Ok(()) => {
                        let resp = rmpv::Value::Map(vec![
                            (rmpv::Value::String("ok".into()), rmpv::Value::Boolean(true)),
                        ]);
                        write_response(out_response, out_response_len, out_error, resp)
                    }
                    Err(e) => {
                        set_error(out_error, ai_provenance_error_code(&e), &e.to_string());
                        false
                    }
                }
            }

            "get_config" => {
                let cfg = engine.config();
                let enabled: Vec<rmpv::Value> = cfg.enabled_collections.iter()
                    .map(|s| rmpv::Value::String(s.clone().into()))
                    .collect();
                let resp = rmpv::Value::Map(vec![
                    (rmpv::Value::String("ai_blend_weight".into()), rmpv::Value::F64(cfg.ai_blend_weight)),
                    (rmpv::Value::String("enabled_collections".into()), rmpv::Value::Array(enabled)),
                    (rmpv::Value::String("response_timeout_secs".into()), rmpv::Value::Integer((cfg.response_timeout_secs as i64).into())),
                    (rmpv::Value::String("silent_on_error".into()), rmpv::Value::Boolean(cfg.silent_on_error)),
                    (rmpv::Value::String("rate_limit_per_minute".into()), rmpv::Value::Integer((cfg.rate_limit_per_minute as i64).into())),
                ]);
                write_response(out_response, out_response_len, out_error, resp)
            }

            _ => {
                set_error(out_error, ERR_INVALID_QUERY, &format!("unknown ai_provenance action: {}", action));
                false
            }
        }
    });
    result.unwrap_or_else(|_| {
        set_error(out_error, ERR_INTERNAL, "panic in nodedb_ai_provenance_execute");
        false
    })
}

// ── AI Query FFI ──────────────────────────────────────────────────

fn ai_query_handle_map() -> &'static RwLock<HashMap<AiQueryHandle, Arc<AiQueryEngine>>> {
    use std::sync::OnceLock;
    static MAP: OnceLock<RwLock<HashMap<AiQueryHandle, Arc<AiQueryEngine>>>> = OnceLock::new();
    MAP.get_or_init(|| RwLock::new(HashMap::new()))
}

fn get_ai_query(handle: AiQueryHandle) -> Option<Arc<AiQueryEngine>> {
    let map = ai_query_handle_map().read().ok()?;
    map.get(&handle).cloned()
}

fn ai_query_error_code(e: &AiQueryError) -> i32 {
    match e {
        AiQueryError::SchemaValidation(_) => ERR_AI_QUERY_SCHEMA_VALIDATION,
        AiQueryError::ConfidenceBelowThreshold(_, _) => ERR_AI_QUERY_CONFIDENCE_BELOW_THRESHOLD,
        AiQueryError::CollectionNotEnabled(_) => ERR_AI_QUERY_COLLECTION_NOT_ENABLED,
        AiQueryError::NoSql(_) => ERR_AI_QUERY_NOSQL,
        AiQueryError::Provenance(_) => ERR_STORAGE,
        AiQueryError::Storage(_) => ERR_STORAGE,
    }
}

/// Open an AI query engine. Config must include "nosql_handle" and "provenance_handle".
#[no_mangle]
pub extern "C" fn nodedb_ai_query_open(
    config_ptr: *const u8,
    config_len: usize,
    out_handle: *mut AiQueryHandle,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if config_ptr.is_null() || out_handle.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let config_bytes = unsafe { std::slice::from_raw_parts(config_ptr, config_len) };
        let config: rmpv::Value = match rmp_serde::from_slice(config_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &e.to_string());
                return false;
            }
        };

        let nosql_handle = map_field(&config, "nosql_handle")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let db = match get_db(nosql_handle) {
            Some(d) => d,
            None => {
                set_error(out_error, ERR_INVALID_HANDLE, "invalid nosql_handle");
                return false;
            }
        };

        let prov_handle = map_field(&config, "provenance_handle")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let prov = match get_provenance(prov_handle) {
            Some(p) => p,
            None => {
                set_error(out_error, ERR_INVALID_HANDLE, "invalid provenance_handle");
                return false;
            }
        };

        let minimum_write_confidence = map_field(&config, "minimum_write_confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.80);

        let max_results_per_query = map_field(&config, "max_results_per_query")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        let enabled_collections: Vec<String> = map_field(&config, "enabled_collections")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let report_write_decisions = map_field(&config, "report_write_decisions")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let rate_limit_per_minute = map_field(&config, "rate_limit_per_minute")
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as u32;

        let ai_config = AiQueryConfig {
            minimum_write_confidence,
            max_results_per_query,
            enabled_collections,
            report_write_decisions,
            rate_limit_per_minute,
        };

        let engine = AiQueryEngine::new(db, prov, ai_config);
        let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
        ai_query_handle_map().write().unwrap().insert(handle, Arc::new(engine));
        unsafe { *out_handle = handle; }
        clear_error(out_error);
        true
    });
    result.unwrap_or_else(|_| {
        set_error(out_error, ERR_INTERNAL, "panic in nodedb_ai_query_open");
        false
    })
}

#[no_mangle]
pub extern "C" fn nodedb_ai_query_close(handle: AiQueryHandle) {
    ai_query_handle_map().write().unwrap().remove(&handle);
}

#[no_mangle]
pub extern "C" fn nodedb_ai_query_execute(
    handle: AiQueryHandle,
    request_ptr: *const u8,
    request_len: usize,
    out_response: *mut *mut u8,
    out_response_len: *mut usize,
    out_error: *mut NodeDbError,
) -> bool {
    let result = catch_unwind(|| {
        if request_ptr.is_null() || out_response.is_null() || out_response_len.is_null() {
            set_error(out_error, ERR_NULL_POINTER, "null pointer argument");
            return false;
        }

        let engine = match get_ai_query(handle) {
            Some(e) => e,
            None => {
                set_error(out_error, ERR_INVALID_HANDLE, "invalid ai_query handle");
                return false;
            }
        };

        let request_bytes = unsafe { std::slice::from_raw_parts(request_ptr, request_len) };
        let request: rmpv::Value = match rmp_serde::from_slice(request_bytes) {
            Ok(v) => v,
            Err(e) => {
                set_error(out_error, ERR_SERIALIZATION, &e.to_string());
                return false;
            }
        };

        let action = map_field(&request, "action").and_then(|v| v.as_str()).unwrap_or("");

        match action {
            "process_results" => {
                let collection = map_field(&request, "collection")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                // Parse results array
                let results_val = map_field(&request, "results").and_then(|v| v.as_array());
                let results: Vec<AiQueryResult> = match results_val {
                    Some(arr) => arr.iter().map(|r| {
                        let data = map_field(r, "data").cloned().unwrap_or(rmpv::Value::Nil);
                        let confidence = map_field(r, "confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let source_explanation = map_field(r, "source_explanation")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let external_source_uri = map_field(r, "external_source_uri")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let tags: Option<std::collections::HashMap<String, String>> = map_field(r, "tags")
                            .and_then(|v| v.as_map())
                            .map(|entries| {
                                entries.iter()
                                    .filter_map(|(k, v)| {
                                        Some((k.as_str()?.to_string(), v.as_str()?.to_string()))
                                    })
                                    .collect()
                            });
                        AiQueryResult { data, confidence, source_explanation, external_source_uri, tags }
                    }).collect(),
                    None => Vec::new(),
                };

                // Parse optional schema
                let schema: Option<AiQuerySchema> = map_field(&request, "schema").and_then(|s| {
                    let required_fields: Vec<String> = map_field(s, "required_fields")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                        .unwrap_or_default();
                    let field_types: std::collections::HashMap<String, SchemaPropertyType> =
                        map_field(s, "field_types")
                            .and_then(|v| v.as_map())
                            .map(|entries| {
                                entries.iter()
                                    .filter_map(|(k, v)| {
                                        let key = k.as_str()?.to_string();
                                        let type_str = v.as_str().unwrap_or("any");
                                        Some((key, SchemaPropertyType::from_str(type_str)))
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                    Some(AiQuerySchema { required_fields, field_types })
                });

                match engine.process_results(collection, results, schema.as_ref()) {
                    Ok(decisions) => {
                        let decisions_val: Vec<rmpv::Value> = decisions.iter().map(|d| {
                            rmpv::Value::Map(vec![
                                (rmpv::Value::String("persisted".into()), rmpv::Value::Boolean(d.persisted)),
                                (rmpv::Value::String("record_id".into()), match d.record_id {
                                    Some(id) => rmpv::Value::Integer(id.into()),
                                    None => rmpv::Value::Nil,
                                }),
                                (rmpv::Value::String("confidence".into()), rmpv::Value::F64(d.confidence)),
                                (rmpv::Value::String("ai_origin_tag".into()), match &d.ai_origin_tag {
                                    Some(tag) => rmpv::Value::String(tag.clone().into()),
                                    None => rmpv::Value::Nil,
                                }),
                                (rmpv::Value::String("rejection_reason".into()), match &d.rejection_reason {
                                    Some(r) => rmpv::Value::String(r.clone().into()),
                                    None => rmpv::Value::Nil,
                                }),
                            ])
                        }).collect();
                        let resp = rmpv::Value::Array(decisions_val);
                        write_response(out_response, out_response_len, out_error, resp)
                    }
                    Err(e) => {
                        set_error(out_error, ai_query_error_code(&e), &e.to_string());
                        false
                    }
                }
            }

            "get_config" => {
                let cfg = engine.config();
                let enabled: Vec<rmpv::Value> = cfg.enabled_collections.iter()
                    .map(|s| rmpv::Value::String(s.clone().into()))
                    .collect();
                let resp = rmpv::Value::Map(vec![
                    (rmpv::Value::String("minimum_write_confidence".into()), rmpv::Value::F64(cfg.minimum_write_confidence)),
                    (rmpv::Value::String("max_results_per_query".into()), rmpv::Value::Integer((cfg.max_results_per_query as i64).into())),
                    (rmpv::Value::String("enabled_collections".into()), rmpv::Value::Array(enabled)),
                    (rmpv::Value::String("report_write_decisions".into()), rmpv::Value::Boolean(cfg.report_write_decisions)),
                    (rmpv::Value::String("rate_limit_per_minute".into()), rmpv::Value::Integer((cfg.rate_limit_per_minute as i64).into())),
                ]);
                write_response(out_response, out_response_len, out_error, resp)
            }

            _ => {
                set_error(out_error, ERR_INVALID_QUERY, &format!("unknown ai_query action: {}", action));
                false
            }
        }
    });
    result.unwrap_or_else(|_| {
        set_error(out_error, ERR_INTERNAL, "panic in nodedb_ai_query_execute");
        false
    })
}

fn write_response(out_response: *mut *mut u8, out_response_len: *mut usize, out_error: *mut NodeDbError, value: rmpv::Value) -> bool {
    let response_bytes = match rmp_serde::to_vec(&value) {
        Ok(b) => b,
        Err(e) => {
            set_error(out_error, ERR_SERIALIZATION, &e.to_string());
            return false;
        }
    };
    let len = response_bytes.len();
    let ptr = response_bytes.as_ptr();
    std::mem::forget(response_bytes);
    unsafe {
        *out_response = ptr as *mut u8;
        *out_response_len = len;
    }
    clear_error(out_error);
    true
}

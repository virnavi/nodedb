use std::sync::{Arc, Mutex};

use nodedb_dac::{DacEngine, DacSubject};
use nodedb_federation::FederationEngine;
use nodedb_graph::GraphEngine;
use nodedb_nosql::Database;
use nodedb_transport::error::TransportError;
use nodedb_transport::query_handler::QueryHandler;
use nodedb_vector::VectorEngine;

/// FFI-layer implementation of QueryHandler.
/// Holds direct Arc references to engines so it can execute queries
/// against local data when receiving federated requests from peers.
pub struct FfiQueryHandler {
    nosql: Option<Arc<Database>>,
    graph: Option<Arc<GraphEngine>>,
    vector: Option<Arc<Mutex<VectorEngine>>>,
    dac: Option<Arc<DacEngine>>,
    federation: Option<Arc<FederationEngine>>,
    /// Database-level sharing status from MeshConfig (string form).
    db_sharing_status: Option<String>,
}

impl FfiQueryHandler {
    pub fn new(
        nosql: Option<Arc<Database>>,
        graph: Option<Arc<GraphEngine>>,
        vector: Option<Arc<Mutex<VectorEngine>>>,
        dac: Option<Arc<DacEngine>>,
        federation: Option<Arc<FederationEngine>>,
    ) -> Self {
        FfiQueryHandler {
            nosql,
            graph,
            vector,
            dac,
            federation,
            db_sharing_status: None,
        }
    }

    /// Set the database-level sharing status (from MeshConfig).
    pub fn set_db_sharing_status(&mut self, status: Option<String>) {
        self.db_sharing_status = status;
    }

    /// Build a DacSubject from a peer_id by resolving group memberships via FederationEngine.
    fn build_dac_subject(&self, origin_peer_id: &str) -> Option<DacSubject> {
        let federation = self.federation.as_ref()?;
        let _dac = self.dac.as_ref()?;

        // Find the peer by looking through all peers for one with a matching public_key or name
        // In federated context, origin_peer_id is the hex peer_id from their Ed25519 key.
        // We need to find the corresponding peer in our federation engine.
        let peers = federation.all_peers().ok()?;
        let peer = peers.iter().find(|p| {
            p.public_key.as_deref() == Some(origin_peer_id)
                || p.name == origin_peer_id
        });

        match peer {
            Some(p) => {
                let group_ids = federation
                    .groups_for_peer(p.id)
                    .ok()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|gid| gid.to_string())
                    .collect();
                Some(DacSubject {
                    peer_id: origin_peer_id.to_string(),
                    group_ids,
                })
            }
            None => {
                // Unknown peer — give them no group memberships (most restrictive)
                Some(DacSubject {
                    peer_id: origin_peer_id.to_string(),
                    group_ids: vec![],
                })
            }
        }
    }

    /// Apply DAC filtering to a serialized response value for a given collection.
    fn apply_dac_filter(
        &self,
        collection: &str,
        response_value: &rmpv::Value,
        origin_peer_id: &str,
    ) -> rmpv::Value {
        let dac = match self.dac.as_ref() {
            Some(d) => d,
            None => return response_value.clone(),
        };

        let subject = match self.build_dac_subject(origin_peer_id) {
            Some(s) => s,
            None => return response_value.clone(),
        };

        // If the response is an array (list of documents), filter each one
        if let Some(arr) = response_value.as_array() {
            let filtered: Vec<rmpv::Value> = arr
                .iter()
                .filter_map(|doc| {
                    let record_id = extract_record_id(doc);
                    match dac.filter_document(
                        collection,
                        doc,
                        &subject,
                        record_id.as_deref(),
                    ) {
                        Ok(v) => Some(v),
                        Err(_) => None, // DAC denied — exclude
                    }
                })
                .collect();
            rmpv::Value::Array(filtered)
        } else {
            // Single document
            let record_id = extract_record_id(response_value);
            match dac.filter_document(
                collection,
                response_value,
                &subject,
                record_id.as_deref(),
            ) {
                Ok(v) => v,
                Err(_) => rmpv::Value::Nil,
            }
        }
    }

    fn handle_nosql(
        &self,
        query_data: &[u8],
        origin_peer_id: &str,
    ) -> Result<Vec<u8>, TransportError> {
        let db = self.nosql.as_ref().ok_or_else(|| {
            TransportError::Connection("NoSQL engine not available".into())
        })?;

        let request: rmpv::Value = rmp_serde::from_slice(query_data)
            .map_err(|e| TransportError::Serialization(e.to_string()))?;

        let collection_name = request
            .as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("collection")))
            .and_then(|(_, v)| v.as_str())
            .ok_or_else(|| TransportError::Serialization("missing 'collection' field".into()))?
            .to_string();

        // Check effective sharing status for this collection (schema-aware cascade)
        let effective_status = db.effective_sharing_status(
            &collection_name,
            self.db_sharing_status.as_deref(),
        );
        if effective_status == "private" {
            // Collection is private — return empty result for remote queries
            let empty = rmpv::Value::Array(vec![]);
            return rmp_serde::to_vec(&empty)
                .map_err(|e| TransportError::Serialization(e.to_string()));
        }

        let collection = db
            .collection(&collection_name)
            .map_err(|e| TransportError::Connection(e.to_string()))?;

        let action = request
            .as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("action")))
            .and_then(|(_, v)| v.as_str())
            .unwrap_or("find_all");

        let response_value: rmpv::Value = match action {
            "get" => {
                let id = request
                    .as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("id")))
                    .and_then(|(_, v)| v.as_i64())
                    .unwrap_or(0);
                let doc = collection
                    .get(id)
                    .map_err(|e| TransportError::Connection(e.to_string()))?;
                rmpv::ext::to_value(&doc)
                    .map_err(|e| TransportError::Serialization(e.to_string()))?
            }
            _ => {
                // find_all
                let offset = request
                    .as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("offset")))
                    .and_then(|(_, v)| v.as_u64())
                    .map(|v| v as usize);
                let limit = request
                    .as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("limit")))
                    .and_then(|(_, v)| v.as_u64())
                    .map(|v| v as usize);
                let docs = collection
                    .find_all(offset, limit)
                    .map_err(|e| TransportError::Connection(e.to_string()))?;
                rmpv::ext::to_value(&docs)
                    .map_err(|e| TransportError::Serialization(e.to_string()))?
            }
        };

        // Apply DAC filtering
        let filtered = self.apply_dac_filter(&collection_name, &response_value, origin_peer_id);

        rmp_serde::to_vec(&filtered)
            .map_err(|e| TransportError::Serialization(e.to_string()))
    }

    fn handle_graph(
        &self,
        query_data: &[u8],
        origin_peer_id: &str,
    ) -> Result<Vec<u8>, TransportError> {
        let graph = self.graph.as_ref().ok_or_else(|| {
            TransportError::Connection("Graph engine not available".into())
        })?;

        let request: rmpv::Value = rmp_serde::from_slice(query_data)
            .map_err(|e| TransportError::Serialization(e.to_string()))?;

        let action = request
            .as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("action")))
            .and_then(|(_, v)| v.as_str())
            .ok_or_else(|| TransportError::Serialization("missing 'action' field".into()))?;

        let response_value: rmpv::Value = match action {
            "get_node" => {
                let id = get_i64_field(&request, "id")?;
                let node = graph
                    .get_node(id)
                    .map_err(|e| TransportError::Connection(e.to_string()))?;
                rmpv::ext::to_value(&node)
                    .map_err(|e| TransportError::Serialization(e.to_string()))?
            }
            "all_nodes" => {
                let nodes = graph
                    .all_nodes()
                    .map_err(|e| TransportError::Connection(e.to_string()))?;
                rmpv::ext::to_value(&nodes)
                    .map_err(|e| TransportError::Serialization(e.to_string()))?
            }
            "get_edge" => {
                let id = get_i64_field(&request, "id")?;
                let edge = graph
                    .get_edge(id)
                    .map_err(|e| TransportError::Connection(e.to_string()))?;
                rmpv::ext::to_value(&edge)
                    .map_err(|e| TransportError::Serialization(e.to_string()))?
            }
            "edges_from" => {
                let id = get_i64_field(&request, "id")?;
                let edges = graph
                    .edges_from(id)
                    .map_err(|e| TransportError::Connection(e.to_string()))?;
                rmpv::ext::to_value(&edges)
                    .map_err(|e| TransportError::Serialization(e.to_string()))?
            }
            "edges_to" => {
                let id = get_i64_field(&request, "id")?;
                let edges = graph
                    .edges_to(id)
                    .map_err(|e| TransportError::Connection(e.to_string()))?;
                rmpv::ext::to_value(&edges)
                    .map_err(|e| TransportError::Serialization(e.to_string()))?
            }
            "neighbors" => {
                let id = get_i64_field(&request, "id")?;
                let neighbors = graph
                    .neighbors(id)
                    .map_err(|e| TransportError::Connection(e.to_string()))?;
                rmpv::ext::to_value(&neighbors)
                    .map_err(|e| TransportError::Serialization(e.to_string()))?
            }
            "bfs" => {
                let id = get_i64_field(&request, "id")?;
                let max_depth = request
                    .as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("max_depth")))
                    .and_then(|(_, v)| v.as_u64())
                    .map(|v| v as usize);
                let result = nodedb_graph::bfs(graph, id, max_depth)
                    .map_err(|e| TransportError::Connection(e.to_string()))?;
                rmpv::ext::to_value(&result)
                    .map_err(|e| TransportError::Serialization(e.to_string()))?
            }
            "dfs" => {
                let id = get_i64_field(&request, "id")?;
                let max_depth = request
                    .as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("max_depth")))
                    .and_then(|(_, v)| v.as_u64())
                    .map(|v| v as usize);
                let result = nodedb_graph::dfs(graph, id, max_depth)
                    .map_err(|e| TransportError::Connection(e.to_string()))?;
                rmpv::ext::to_value(&result)
                    .map_err(|e| TransportError::Serialization(e.to_string()))?
            }
            "shortest_path" => {
                let from = get_i64_field(&request, "from")?;
                let to = get_i64_field(&request, "to")?;
                let result = nodedb_graph::shortest_path(graph, from, to)
                    .map_err(|e| TransportError::Connection(e.to_string()))?;
                rmpv::ext::to_value(&result)
                    .map_err(|e| TransportError::Serialization(e.to_string()))?
            }
            _ => {
                return Err(TransportError::Serialization(format!(
                    "unsupported graph action for federated query: {}",
                    action
                )));
            }
        };

        // DAC filtering for graph nodes — filter node data fields
        let _ = origin_peer_id; // DAC on graph data is application-specific; return as-is for now
        rmp_serde::to_vec(&response_value)
            .map_err(|e| TransportError::Serialization(e.to_string()))
    }

    fn handle_vector(
        &self,
        query_data: &[u8],
        _origin_peer_id: &str,
    ) -> Result<Vec<u8>, TransportError> {
        let vector = self.vector.as_ref().ok_or_else(|| {
            TransportError::Connection("Vector engine not available".into())
        })?;

        let request: rmpv::Value = rmp_serde::from_slice(query_data)
            .map_err(|e| TransportError::Serialization(e.to_string()))?;

        let action = request
            .as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("action")))
            .and_then(|(_, v)| v.as_str())
            .ok_or_else(|| TransportError::Serialization("missing 'action' field".into()))?;

        let engine = vector
            .lock()
            .map_err(|_| TransportError::Connection("vector engine lock poisoned".into()))?;

        let response_value: rmpv::Value = match action {
            "search" => {
                let query_vec = extract_f32_vec(&request, "query")?;
                let k = request
                    .as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("k")))
                    .and_then(|(_, v)| v.as_u64())
                    .unwrap_or(10) as usize;
                let ef = request
                    .as_map()
                    .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("ef_search")))
                    .and_then(|(_, v)| v.as_u64())
                    .unwrap_or(64) as usize;
                let results = engine
                    .search(&query_vec, k, ef)
                    .map_err(|e| TransportError::Connection(e.to_string()))?;
                rmpv::ext::to_value(&results)
                    .map_err(|e| TransportError::Serialization(e.to_string()))?
            }
            "get" => {
                let id = get_i64_field(&request, "id")?;
                let (record, _vec) = engine
                    .get(id)
                    .map_err(|e| TransportError::Connection(e.to_string()))?;
                rmpv::ext::to_value(&record)
                    .map_err(|e| TransportError::Serialization(e.to_string()))?
            }
            "count" => {
                let count = engine.count();
                rmpv::Value::Integer(rmpv::Integer::from(count as i64))
            }
            _ => {
                return Err(TransportError::Serialization(format!(
                    "unsupported vector action for federated query: {}",
                    action
                )));
            }
        };

        rmp_serde::to_vec(&response_value)
            .map_err(|e| TransportError::Serialization(e.to_string()))
    }
}

impl QueryHandler for FfiQueryHandler {
    fn handle_query(
        &self,
        query_type: &str,
        query_data: &[u8],
        origin_peer_id: &str,
    ) -> Result<Vec<u8>, TransportError> {
        match query_type {
            "nosql" => self.handle_nosql(query_data, origin_peer_id),
            "graph" => self.handle_graph(query_data, origin_peer_id),
            "vector" => self.handle_vector(query_data, origin_peer_id),
            _ => Err(TransportError::Serialization(format!(
                "unknown query type: {}",
                query_type
            ))),
        }
    }

    fn merge_results(
        &self,
        query_type: &str,
        local_result: Vec<u8>,
        remote_results: Vec<Vec<u8>>,
    ) -> Vec<u8> {
        use crate::merge;
        match query_type {
            "nosql" => merge::merge_nosql_results(&local_result, &remote_results),
            "vector" => merge::merge_vector_results(&local_result, &remote_results, 10),
            "graph" => merge::merge_graph_results(&local_result, &remote_results),
            _ => local_result,
        }
    }

    fn handle_trigger_notification(
        &self,
        payload: &[u8],
        _origin_peer_id: &str,
    ) {
        let db = match self.nosql.as_ref() {
            Some(db) => db,
            None => return,
        };

        let notification: nodedb_transport::TriggerNotificationPayload =
            match rmp_serde::from_slice(payload) {
                Ok(n) => n,
                Err(_) => return,
            };

        let event = match nodedb_nosql::TriggerEvent::from_str(&notification.event) {
            Some(e) => e,
            None => return,
        };

        // Deserialize old/new records if present
        let old_record: Option<nodedb_nosql::Document> = notification
            .old_record
            .as_ref()
            .and_then(|bytes| rmp_serde::from_slice(bytes).ok());

        let new_record: Option<nodedb_nosql::Document> = notification
            .new_record
            .as_ref()
            .and_then(|bytes| rmp_serde::from_slice(bytes).ok());

        // ── Auto-apply remote changes to local DB ──
        let effective_status = db.effective_sharing_status(
            &notification.collection,
            self.db_sharing_status.as_deref(),
        );
        if effective_status != "private" {
            let _ = self.auto_apply_remote(db, &notification, event, &old_record, &new_record);
        }

        // ── Fire mesh triggers ──
        let mesh_triggers = db.triggers().matching_mesh(
            &notification.source_database,
            &notification.collection,
            event,
        );

        for trigger in &mesh_triggers {
            let ctx = nodedb_nosql::TriggerContext {
                db: std::sync::Arc::downgrade(db),
                qualified_name: notification.collection.clone(),
                event,
                old_record: old_record.clone(),
                new_record: new_record.clone(),
                from_mesh: true,
                source_database_name: Some(notification.source_database.clone()),
            };
            let _ = (trigger.handler)(&ctx);
        }
    }

}

impl FfiQueryHandler {
    /// Auto-apply a remote trigger notification to the local database.
    /// Uses last-write-wins (compare `updated_at`) for insert/update.
    /// Writes directly to collection (bypasses triggers to avoid infinite loops).
    fn auto_apply_remote(
        &self,
        db: &Arc<Database>,
        notification: &nodedb_transport::TriggerNotificationPayload,
        event: nodedb_nosql::TriggerEvent,
        _old_record: &Option<nodedb_nosql::Document>,
        new_record: &Option<nodedb_nosql::Document>,
    ) -> Result<(), nodedb_nosql::NoSqlError> {
        let collection = db.collection(&notification.collection)?;

        match event {
            nodedb_nosql::TriggerEvent::Insert | nodedb_nosql::TriggerEvent::Update => {
                let remote_doc = match new_record {
                    Some(doc) => doc,
                    None => return Ok(()),
                };

                // Try to find local record with same data['id'] or sled id
                let local_doc = self.find_local_record(db, &notification.collection, remote_doc);

                // Last-write-wins: only apply if remote is newer
                if let Some(ref local) = local_doc {
                    if local.updated_at >= remote_doc.updated_at {
                        return Ok(()); // Local is same age or newer, skip
                    }
                }

                // Apply: use local sled id if found, otherwise 0 (auto-assign)
                let sled_id = local_doc.map(|d| d.id).unwrap_or(0);
                collection.put_with_id(sled_id, remote_doc.data.clone())?;
                db.increment_sync_version();
            }
            nodedb_nosql::TriggerEvent::Delete => {
                // Find the local record to delete
                let old_doc = match _old_record {
                    Some(doc) => doc,
                    None => return Ok(()),
                };

                if let Some(local) = self.find_local_record(db, &notification.collection, old_doc) {
                    collection.delete(local.id)?;
                    db.increment_sync_version();
                }
            }
        }

        Ok(())
    }

    /// Find a local record matching a remote document by data['id'] field or sled ID.
    fn find_local_record(
        &self,
        db: &Arc<Database>,
        collection_name: &str,
        remote_doc: &nodedb_nosql::Document,
    ) -> Option<nodedb_nosql::Document> {
        let collection = db.collection(collection_name).ok()?;

        // First try: look up by data['id'] field (String UUID or int)
        if let Some(id_val) = remote_doc.data.as_map().and_then(|m| {
            m.iter().find(|(k, _)| k.as_str() == Some("id")).map(|(_, v)| v)
        }) {
            // If data['id'] is an integer, it might be the sled key
            if let Some(int_id) = id_val.as_i64() {
                if let Ok(doc) = collection.get(int_id) {
                    return Some(doc);
                }
            }

            // String ID: scan for matching data['id'] field
            if let Some(str_id) = id_val.as_str() {
                if let Ok(all) = collection.find_all(None, None) {
                    for doc in all {
                        if let Some(local_id_val) = doc.data.as_map().and_then(|m| {
                            m.iter().find(|(k, _)| k.as_str() == Some("id")).map(|(_, v)| v)
                        }) {
                            if local_id_val.as_str() == Some(str_id) {
                                return Some(doc);
                            }
                        }
                    }
                }
            }
        }

        // Fallback: try sled ID directly
        if remote_doc.id != 0 {
            if let Ok(doc) = collection.get(remote_doc.id) {
                return Some(doc);
            }
        }

        None
    }
}

/// Extract the record ID from a document value (looks for "id" field).
fn extract_record_id(doc: &rmpv::Value) -> Option<String> {
    doc.as_map()
        .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some("id")))
        .and_then(|(_, v)| v.as_i64())
        .map(|id| id.to_string())
}

fn get_i64_field(request: &rmpv::Value, field: &str) -> Result<i64, TransportError> {
    request
        .as_map()
        .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(field)))
        .and_then(|(_, v)| v.as_i64())
        .ok_or_else(|| TransportError::Serialization(format!("missing '{}' field", field)))
}

fn extract_f32_vec(request: &rmpv::Value, field: &str) -> Result<Vec<f32>, TransportError> {
    let arr = request
        .as_map()
        .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(field)))
        .and_then(|(_, v)| v.as_array())
        .ok_or_else(|| TransportError::Serialization(format!("missing '{}' array", field)))?;

    arr.iter()
        .map(|v| {
            v.as_f64()
                .map(|f| f as f32)
                .ok_or_else(|| TransportError::Serialization("invalid vector element".into()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodedb_transport::query_handler::QueryHandler;

    fn make_doc(id: i64, name: &str) -> rmpv::Value {
        rmpv::Value::Map(vec![
            (
                rmpv::Value::String("id".into()),
                rmpv::Value::Integer(rmpv::Integer::from(id)),
            ),
            (
                rmpv::Value::String("name".into()),
                rmpv::Value::String(name.into()),
            ),
        ])
    }

    fn make_search_result(id: i64, distance: f32) -> rmpv::Value {
        rmpv::Value::Map(vec![
            (
                rmpv::Value::String("id".into()),
                rmpv::Value::Integer(rmpv::Integer::from(id)),
            ),
            (
                rmpv::Value::String("distance".into()),
                rmpv::Value::F32(distance),
            ),
        ])
    }

    fn handler() -> FfiQueryHandler {
        FfiQueryHandler::new(None, None, None, None, None)
    }

    #[test]
    fn merge_results_nosql_dedup() {
        let h = handler();
        let local = rmp_serde::to_vec(&rmpv::Value::Array(vec![
            make_doc(1, "A"),
            make_doc(2, "B"),
        ]))
        .unwrap();
        let remote = rmp_serde::to_vec(&rmpv::Value::Array(vec![
            make_doc(2, "RemoteB"),
            make_doc(3, "C"),
        ]))
        .unwrap();

        let merged = h.merge_results("nosql", local, vec![remote]);
        let val: rmpv::Value = rmp_serde::from_slice(&merged).unwrap();
        let arr = val.as_array().unwrap();
        assert_eq!(arr.len(), 3); // deduped: 1, 2 (local), 3
    }

    #[test]
    fn merge_results_vector_sorts_by_distance() {
        let h = handler();
        let local = rmp_serde::to_vec(&rmpv::Value::Array(vec![
            make_search_result(1, 0.5),
        ]))
        .unwrap();
        let remote = rmp_serde::to_vec(&rmpv::Value::Array(vec![
            make_search_result(2, 0.1),
        ]))
        .unwrap();

        let merged = h.merge_results("vector", local, vec![remote]);
        let val: rmpv::Value = rmp_serde::from_slice(&merged).unwrap();
        let arr = val.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        // id=2 (0.1) should come before id=1 (0.5)
        let first_id = arr[0]
            .as_map()
            .unwrap()
            .iter()
            .find(|(k, _)| k.as_str() == Some("id"))
            .unwrap()
            .1
            .as_i64()
            .unwrap();
        assert_eq!(first_id, 2);
    }

    #[test]
    fn merge_results_graph_union() {
        let h = handler();
        let local = rmp_serde::to_vec(&rmpv::Value::Array(vec![make_doc(1, "NodeA")]))
            .unwrap();
        let remote = rmp_serde::to_vec(&rmpv::Value::Array(vec![make_doc(2, "NodeB")]))
            .unwrap();

        let merged = h.merge_results("graph", local, vec![remote]);
        let val: rmpv::Value = rmp_serde::from_slice(&merged).unwrap();
        assert_eq!(val.as_array().unwrap().len(), 2);
    }

    #[test]
    fn merge_results_unknown_type_returns_local() {
        let h = handler();
        let local = vec![1, 2, 3];
        let remote = vec![4, 5, 6];
        let result = h.merge_results("unknown", local.clone(), vec![remote]);
        assert_eq!(result, local);
    }
}

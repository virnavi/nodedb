use std::collections::HashMap;

/// Merge NoSQL results from local and remote peers.
/// Deduplicates documents by (collection, id). Local results take priority on conflict.
pub fn merge_nosql_results(local: &[u8], remotes: &[Vec<u8>]) -> Vec<u8> {
    let mut seen: HashMap<i64, rmpv::Value> = HashMap::new();

    // Parse local results first (they win on conflict)
    if let Ok(local_val) = rmp_serde::from_slice::<rmpv::Value>(local) {
        if let Some(arr) = local_val.as_array() {
            for doc in arr {
                if let Some(id) = extract_doc_id(doc) {
                    seen.insert(id, doc.clone());
                }
            }
        }
    }

    // Parse remote results — only insert if not already seen
    for remote in remotes {
        if let Ok(remote_val) = rmp_serde::from_slice::<rmpv::Value>(remote) {
            if let Some(arr) = remote_val.as_array() {
                for doc in arr {
                    if let Some(id) = extract_doc_id(doc) {
                        seen.entry(id).or_insert_with(|| doc.clone());
                    }
                }
            }
        }
    }

    // Collect all unique documents, sorted by id for deterministic output
    let mut docs: Vec<(i64, rmpv::Value)> = seen.into_iter().collect();
    docs.sort_by_key(|(id, _)| *id);
    let result: Vec<rmpv::Value> = docs.into_iter().map(|(_, v)| v).collect();

    rmp_serde::to_vec(&rmpv::Value::Array(result)).unwrap_or_default()
}

/// Merge vector search results from local and remote peers.
/// Sorts all results by distance (ascending) and takes the top-k.
pub fn merge_vector_results(local: &[u8], remotes: &[Vec<u8>], k: usize) -> Vec<u8> {
    let mut all_results: Vec<(i64, f32, rmpv::Value)> = Vec::new();

    // Parse results from all sources
    for data in std::iter::once(local).chain(remotes.iter().map(|v| v.as_slice())) {
        if let Ok(val) = rmp_serde::from_slice::<rmpv::Value>(data) {
            if let Some(arr) = val.as_array() {
                for item in arr {
                    let id = extract_search_id(item);
                    let distance = extract_search_distance(item);
                    if let (Some(id), Some(dist)) = (id, distance) {
                        all_results.push((id, dist, item.clone()));
                    }
                }
            }
        }
    }

    // Sort by distance ascending (closest first), dedup by id
    all_results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut seen = std::collections::HashSet::new();
    let mut deduped: Vec<rmpv::Value> = Vec::new();
    for (id, _dist, val) in all_results {
        if seen.insert(id) {
            deduped.push(val);
            if deduped.len() >= k {
                break;
            }
        }
    }

    rmp_serde::to_vec(&rmpv::Value::Array(deduped)).unwrap_or_default()
}

/// Merge graph results from local and remote peers.
/// Union nodes and edges by ID.
pub fn merge_graph_results(local: &[u8], remotes: &[Vec<u8>]) -> Vec<u8> {
    // Graph results can be arrays of nodes, edges, or traversal results.
    // For simple cases (arrays), deduplicate by id.
    let mut seen_ids: HashMap<i64, rmpv::Value> = HashMap::new();

    for data in std::iter::once(local).chain(remotes.iter().map(|v| v.as_slice())) {
        if let Ok(val) = rmp_serde::from_slice::<rmpv::Value>(data) {
            if let Some(arr) = val.as_array() {
                for item in arr {
                    if let Some(id) = extract_doc_id(item) {
                        seen_ids.entry(id).or_insert_with(|| item.clone());
                    }
                }
            } else {
                // Single result (e.g., traversal result) — merge fields
                // For traversal results, just return the local one if we can't merge
                if seen_ids.is_empty() {
                    return data.to_vec();
                }
            }
        }
    }

    let mut items: Vec<(i64, rmpv::Value)> = seen_ids.into_iter().collect();
    items.sort_by_key(|(id, _)| *id);
    let result: Vec<rmpv::Value> = items.into_iter().map(|(_, v)| v).collect();

    rmp_serde::to_vec(&rmpv::Value::Array(result)).unwrap_or_default()
}

/// Extract a document ID from an rmpv::Value.
/// Looks for "id" field in a map, or first element of positional array.
fn extract_doc_id(doc: &rmpv::Value) -> Option<i64> {
    // Try map-style first
    if let Some(map) = doc.as_map() {
        for (k, v) in map {
            if k.as_str() == Some("id") {
                return v.as_i64();
            }
        }
    }
    // Try positional array (rmpv::ext::to_value produces arrays for structs)
    if let Some(arr) = doc.as_array() {
        if !arr.is_empty() {
            return arr[0].as_i64();
        }
    }
    None
}

/// Extract search result ID — works for both map and positional formats.
fn extract_search_id(item: &rmpv::Value) -> Option<i64> {
    extract_doc_id(item)
}

/// Extract search result distance.
fn extract_search_distance(item: &rmpv::Value) -> Option<f32> {
    if let Some(map) = item.as_map() {
        for (k, v) in map {
            if k.as_str() == Some("distance") {
                return v.as_f64().map(|f| f as f32);
            }
        }
    }
    // Positional: SearchResult fields are [id, distance, metadata]
    if let Some(arr) = item.as_array() {
        if arr.len() >= 2 {
            return arr[1].as_f64().map(|f| f as f32);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

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
            (
                rmpv::Value::String("metadata".into()),
                rmpv::Value::Nil,
            ),
        ])
    }

    #[test]
    fn merge_nosql_dedup_local_wins() {
        let local_docs = vec![make_doc(1, "Alice"), make_doc(2, "Bob")];
        let local = rmp_serde::to_vec(&rmpv::Value::Array(local_docs)).unwrap();

        let remote_docs = vec![make_doc(2, "RemoteBob"), make_doc(3, "Charlie")];
        let remote = rmp_serde::to_vec(&rmpv::Value::Array(remote_docs)).unwrap();

        let merged = merge_nosql_results(&local, &[remote]);
        let val: rmpv::Value = rmp_serde::from_slice(&merged).unwrap();
        let arr = val.as_array().unwrap();

        assert_eq!(arr.len(), 3); // 1 (Alice), 2 (Bob), 3 (Charlie)

        // Doc 2 should be local "Bob", not remote "RemoteBob"
        let doc2 = &arr[1];
        let name = doc2
            .as_map()
            .unwrap()
            .iter()
            .find(|(k, _)| k.as_str() == Some("name"))
            .unwrap()
            .1
            .as_str()
            .unwrap();
        assert_eq!(name, "Bob");
    }

    #[test]
    fn merge_nosql_empty_remotes() {
        let local_docs = vec![make_doc(1, "Alice")];
        let local = rmp_serde::to_vec(&rmpv::Value::Array(local_docs)).unwrap();

        let merged = merge_nosql_results(&local, &[]);
        let val: rmpv::Value = rmp_serde::from_slice(&merged).unwrap();
        assert_eq!(val.as_array().unwrap().len(), 1);
    }

    #[test]
    fn merge_vector_top_k_by_distance() {
        let local_results = vec![
            make_search_result(1, 0.1),
            make_search_result(2, 0.5),
        ];
        let local = rmp_serde::to_vec(&rmpv::Value::Array(local_results)).unwrap();

        let remote_results = vec![
            make_search_result(3, 0.2),
            make_search_result(4, 0.05),
        ];
        let remote = rmp_serde::to_vec(&rmpv::Value::Array(remote_results)).unwrap();

        let merged = merge_vector_results(&local, &[remote], 3);
        let val: rmpv::Value = rmp_serde::from_slice(&merged).unwrap();
        let arr = val.as_array().unwrap();

        assert_eq!(arr.len(), 3);
        // Should be sorted: id=4 (0.05), id=1 (0.1), id=3 (0.2)
        assert_eq!(extract_search_id(&arr[0]), Some(4));
        assert_eq!(extract_search_id(&arr[1]), Some(1));
        assert_eq!(extract_search_id(&arr[2]), Some(3));
    }

    #[test]
    fn merge_vector_dedup_by_id() {
        let local_results = vec![make_search_result(1, 0.1)];
        let local = rmp_serde::to_vec(&rmpv::Value::Array(local_results)).unwrap();

        let remote_results = vec![make_search_result(1, 0.2)]; // same id, worse distance
        let remote = rmp_serde::to_vec(&rmpv::Value::Array(remote_results)).unwrap();

        let merged = merge_vector_results(&local, &[remote], 10);
        let val: rmpv::Value = rmp_serde::from_slice(&merged).unwrap();
        let arr = val.as_array().unwrap();
        assert_eq!(arr.len(), 1); // deduped
    }

    #[test]
    fn merge_graph_union_nodes() {
        let local_nodes = vec![make_doc(1, "NodeA"), make_doc(2, "NodeB")];
        let local = rmp_serde::to_vec(&rmpv::Value::Array(local_nodes)).unwrap();

        let remote_nodes = vec![make_doc(2, "RemoteB"), make_doc(3, "NodeC")];
        let remote = rmp_serde::to_vec(&rmpv::Value::Array(remote_nodes)).unwrap();

        let merged = merge_graph_results(&local, &[remote]);
        let val: rmpv::Value = rmp_serde::from_slice(&merged).unwrap();
        let arr = val.as_array().unwrap();
        assert_eq!(arr.len(), 3);
    }
}

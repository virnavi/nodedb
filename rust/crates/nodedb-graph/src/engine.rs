use std::sync::Arc;

use nodedb_storage::{StorageEngine, StorageTree, IdGenerator, encode_id, decode_id, to_msgpack, from_msgpack};
use rmpv::Value;

use crate::error::GraphError;
use crate::types::{GraphNode, GraphEdge, DeleteBehaviour};

pub struct GraphEngine {
    engine: Arc<StorageEngine>,
    nodes: StorageTree,
    edges: StorageTree,
    adj_out: StorageTree,
    adj_in: StorageTree,
    #[allow(dead_code)]
    meta: StorageTree,
    id_gen: Arc<IdGenerator>,
}

impl GraphEngine {
    pub fn new(engine: Arc<StorageEngine>) -> Result<Self, GraphError> {
        let nodes = engine.open_tree("__graph_nodes__")?;
        let edges = engine.open_tree("__graph_edges__")?;
        let adj_out = engine.open_tree("__graph_adj_out__")?;
        let adj_in = engine.open_tree("__graph_adj_in__")?;
        let meta = engine.open_tree("__graph_meta__")?;
        let id_gen = Arc::new(IdGenerator::new(&engine)?);

        Ok(GraphEngine {
            engine,
            nodes,
            edges,
            adj_out,
            adj_in,
            meta,
            id_gen,
        })
    }

    pub fn flush(&self) -> Result<(), GraphError> {
        self.engine.flush()?;
        Ok(())
    }

    fn encode_adj_key(node_id: i64, edge_id: i64) -> Vec<u8> {
        let mut key = Vec::with_capacity(16);
        key.extend_from_slice(&encode_id(node_id));
        key.extend_from_slice(&encode_id(edge_id));
        key
    }

    fn decode_adj_key(bytes: &[u8]) -> Result<(i64, i64), GraphError> {
        if bytes.len() != 16 {
            return Err(GraphError::Serialization("invalid adj key: expected 16 bytes".to_string()));
        }
        let node_id = decode_id(&bytes[0..8])?;
        let edge_id = decode_id(&bytes[8..16])?;
        Ok((node_id, edge_id))
    }

    // --- Node CRUD ---

    pub fn add_node(&self, label: &str, data: Value) -> Result<GraphNode, GraphError> {
        let id = self.id_gen.next_id("graph_nodes")?;
        let node = GraphNode::new(id, label, data);
        let bytes = to_msgpack(&node)?;
        self.nodes.insert(&encode_id(id), &bytes)?;
        Ok(node)
    }

    pub fn get_node(&self, id: i64) -> Result<GraphNode, GraphError> {
        match self.nodes.get(&encode_id(id))? {
            Some(bytes) => Ok(from_msgpack(&bytes)?),
            None => Err(GraphError::NodeNotFound(id)),
        }
    }

    pub fn update_node(&self, id: i64, data: Value) -> Result<GraphNode, GraphError> {
        let mut node = self.get_node(id)?;
        node.data = data;
        node.updated_at = chrono::Utc::now();
        let bytes = to_msgpack(&node)?;
        self.nodes.insert(&encode_id(id), &bytes)?;
        Ok(node)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn all_nodes(&self) -> Result<Vec<GraphNode>, GraphError> {
        let mut nodes = Vec::new();
        for item in self.nodes.iter() {
            let (_, bytes) = item?;
            let node: GraphNode = from_msgpack(&bytes)?;
            nodes.push(node);
        }
        Ok(nodes)
    }

    // --- Edge CRUD ---

    pub fn add_edge(
        &self,
        label: &str,
        source: i64,
        target: i64,
        weight: f64,
        data: Value,
    ) -> Result<GraphEdge, GraphError> {
        if weight.is_nan() {
            return Err(GraphError::Traversal("NaN weight not allowed".to_string()));
        }
        // Validate source and target exist
        if self.nodes.get(&encode_id(source))?.is_none() {
            return Err(GraphError::InvalidSource(source));
        }
        if self.nodes.get(&encode_id(target))?.is_none() {
            return Err(GraphError::InvalidTarget(target));
        }

        let id = self.id_gen.next_id("graph_edges")?;
        let edge = GraphEdge::new(id, label, source, target, weight, data);
        let bytes = to_msgpack(&edge)?;
        self.edges.insert(&encode_id(id), &bytes)?;

        // Insert adjacency entries
        self.adj_out.insert(&Self::encode_adj_key(source, id), &[])?;
        self.adj_in.insert(&Self::encode_adj_key(target, id), &[])?;

        Ok(edge)
    }

    pub fn get_edge(&self, id: i64) -> Result<GraphEdge, GraphError> {
        match self.edges.get(&encode_id(id))? {
            Some(bytes) => Ok(from_msgpack(&bytes)?),
            None => Err(GraphError::EdgeNotFound(id)),
        }
    }

    pub fn update_edge(&self, id: i64, data: Value) -> Result<GraphEdge, GraphError> {
        let mut edge = self.get_edge(id)?;
        edge.data = data;
        edge.updated_at = chrono::Utc::now();
        let bytes = to_msgpack(&edge)?;
        self.edges.insert(&encode_id(id), &bytes)?;
        Ok(edge)
    }

    pub fn delete_edge(&self, id: i64) -> Result<(), GraphError> {
        let edge = self.get_edge(id)?;
        self.edges.remove(&encode_id(id))?;
        self.adj_out.remove(&Self::encode_adj_key(edge.source, id))?;
        self.adj_in.remove(&Self::encode_adj_key(edge.target, id))?;
        Ok(())
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn all_edges(&self) -> Result<Vec<GraphEdge>, GraphError> {
        let mut edges = Vec::new();
        for item in self.edges.iter() {
            let (_, bytes) = item?;
            let edge: GraphEdge = from_msgpack(&bytes)?;
            edges.push(edge);
        }
        Ok(edges)
    }

    // --- Adjacency lookups ---

    pub fn edges_from(&self, node_id: i64) -> Result<Vec<GraphEdge>, GraphError> {
        let prefix = encode_id(node_id);
        let mut edges = Vec::new();
        for item in self.adj_out.scan_prefix(&prefix) {
            let (key, _) = item?;
            let (_, edge_id) = Self::decode_adj_key(&key)?;
            let edge = self.get_edge(edge_id)?;
            edges.push(edge);
        }
        Ok(edges)
    }

    pub fn edges_to(&self, node_id: i64) -> Result<Vec<GraphEdge>, GraphError> {
        let prefix = encode_id(node_id);
        let mut edges = Vec::new();
        for item in self.adj_in.scan_prefix(&prefix) {
            let (key, _) = item?;
            let (_, edge_id) = Self::decode_adj_key(&key)?;
            let edge = self.get_edge(edge_id)?;
            edges.push(edge);
        }
        Ok(edges)
    }

    pub fn neighbors(&self, node_id: i64) -> Result<Vec<GraphNode>, GraphError> {
        let mut neighbor_ids = std::collections::HashSet::new();
        for edge in self.edges_from(node_id)? {
            neighbor_ids.insert(edge.target);
        }
        for edge in self.edges_to(node_id)? {
            neighbor_ids.insert(edge.source);
        }
        neighbor_ids.remove(&node_id); // exclude self-loops from neighbor list
        let mut neighbors = Vec::new();
        for id in neighbor_ids {
            neighbors.push(self.get_node(id)?);
        }
        Ok(neighbors)
    }

    fn all_edge_ids_for_node(&self, node_id: i64) -> Result<Vec<i64>, GraphError> {
        let prefix = encode_id(node_id);
        let mut edge_ids = Vec::new();
        for item in self.adj_out.scan_prefix(&prefix) {
            let (key, _) = item?;
            let (_, edge_id) = Self::decode_adj_key(&key)?;
            edge_ids.push(edge_id);
        }
        for item in self.adj_in.scan_prefix(&prefix) {
            let (key, _) = item?;
            let (_, edge_id) = Self::decode_adj_key(&key)?;
            if !edge_ids.contains(&edge_id) {
                edge_ids.push(edge_id);
            }
        }
        Ok(edge_ids)
    }

    // --- Delete Behaviours ---

    pub fn delete_node(&self, id: i64, behaviour: DeleteBehaviour) -> Result<(), GraphError> {
        // Verify node exists
        self.get_node(id)?;

        match behaviour {
            DeleteBehaviour::Detach => self.delete_node_detach(id),
            DeleteBehaviour::Restrict => self.delete_node_restrict(id),
            DeleteBehaviour::Cascade => {
                let mut visited = std::collections::HashSet::new();
                self.delete_node_cascade(id, 0, 5, &mut visited)
            }
            DeleteBehaviour::Nullify => self.delete_node_nullify(id),
        }
    }

    fn delete_node_detach(&self, id: i64) -> Result<(), GraphError> {
        let edge_ids = self.all_edge_ids_for_node(id)?;
        for edge_id in edge_ids {
            self.delete_edge(edge_id)?;
        }
        self.nodes.remove(&encode_id(id))?;
        Ok(())
    }

    fn delete_node_restrict(&self, id: i64) -> Result<(), GraphError> {
        let edge_ids = self.all_edge_ids_for_node(id)?;
        if !edge_ids.is_empty() {
            return Err(GraphError::DeleteRestricted);
        }
        self.nodes.remove(&encode_id(id))?;
        Ok(())
    }

    fn delete_node_cascade(
        &self,
        id: i64,
        depth: usize,
        max_depth: usize,
        visited: &mut std::collections::HashSet<i64>,
    ) -> Result<(), GraphError> {
        if !visited.insert(id) {
            return Ok(()); // already visited
        }
        if depth > max_depth {
            return Ok(()); // depth limit reached
        }

        // Collect neighbor nodes via outgoing edges
        let outgoing = self.edges_from(id)?;
        let target_ids: Vec<i64> = outgoing.iter().map(|e| e.target).collect();

        // Delete all edges for this node
        let edge_ids = self.all_edge_ids_for_node(id)?;
        for edge_id in edge_ids {
            // Edge may have already been deleted by a prior cascade step
            if self.edges.get(&encode_id(edge_id))?.is_some() {
                self.delete_edge(edge_id)?;
            }
        }

        // Remove the node
        self.nodes.remove(&encode_id(id))?;

        // Recursively cascade to target nodes
        for target_id in target_ids {
            if self.nodes.get(&encode_id(target_id))?.is_some() {
                self.delete_node_cascade(target_id, depth + 1, max_depth, visited)?;
            }
        }

        Ok(())
    }

    fn delete_node_nullify(&self, id: i64) -> Result<(), GraphError> {
        let sentinel: i64 = -1;

        // Process outgoing edges: set source to -1
        let prefix = encode_id(id);
        let out_edge_ids: Vec<i64> = {
            let mut ids = Vec::new();
            for item in self.adj_out.scan_prefix(&prefix) {
                let (key, _) = item?;
                let (_, edge_id) = Self::decode_adj_key(&key)?;
                ids.push(edge_id);
            }
            ids
        };

        for edge_id in &out_edge_ids {
            let mut edge = self.get_edge(*edge_id)?;
            // Remove old adj entry
            self.adj_out.remove(&Self::encode_adj_key(id, *edge_id))?;
            // Update edge source
            edge.source = sentinel;
            edge.updated_at = chrono::Utc::now();
            let bytes = to_msgpack(&edge)?;
            self.edges.insert(&encode_id(*edge_id), &bytes)?;
            // Add new adj entry for sentinel
            self.adj_out.insert(&Self::encode_adj_key(sentinel, *edge_id), &[])?;
        }

        // Process incoming edges: set target to -1
        let in_edge_ids: Vec<i64> = {
            let mut ids = Vec::new();
            for item in self.adj_in.scan_prefix(&prefix) {
                let (key, _) = item?;
                let (_, edge_id) = Self::decode_adj_key(&key)?;
                ids.push(edge_id);
            }
            ids
        };

        for edge_id in &in_edge_ids {
            let mut edge = self.get_edge(*edge_id)?;
            // Remove old adj entry
            self.adj_in.remove(&Self::encode_adj_key(id, *edge_id))?;
            // Update edge target
            edge.target = sentinel;
            edge.updated_at = chrono::Utc::now();
            let bytes = to_msgpack(&edge)?;
            self.edges.insert(&encode_id(*edge_id), &bytes)?;
            // Add new adj entry for sentinel
            self.adj_in.insert(&Self::encode_adj_key(sentinel, *edge_id), &[])?;
        }

        // Remove the node
        self.nodes.remove(&encode_id(id))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp_engine() -> (Arc<StorageEngine>, TempDir) {
        let dir = TempDir::new().unwrap();
        let engine = Arc::new(StorageEngine::open(dir.path()).unwrap());
        (engine, dir)
    }

    // --- Node CRUD tests ---

    #[test]
    fn test_add_and_get_node() {
        let (engine, _dir) = tmp_engine();
        let graph = GraphEngine::new(engine).unwrap();

        let node = graph.add_node("person", Value::String("Alice".into())).unwrap();
        assert_eq!(node.id, 1);
        assert_eq!(node.label, "person");

        let fetched = graph.get_node(1).unwrap();
        assert_eq!(fetched.id, 1);
        assert_eq!(fetched.label, "person");
    }

    #[test]
    fn test_update_node() {
        let (engine, _dir) = tmp_engine();
        let graph = GraphEngine::new(engine).unwrap();

        graph.add_node("person", Value::String("Alice".into())).unwrap();
        let updated = graph.update_node(1, Value::String("Bob".into())).unwrap();
        assert_eq!(updated.data, Value::String("Bob".into()));

        let fetched = graph.get_node(1).unwrap();
        assert_eq!(fetched.data, Value::String("Bob".into()));
    }

    #[test]
    fn test_node_count_and_all_nodes() {
        let (engine, _dir) = tmp_engine();
        let graph = GraphEngine::new(engine).unwrap();

        assert_eq!(graph.node_count(), 0);
        graph.add_node("a", Value::Nil).unwrap();
        graph.add_node("b", Value::Nil).unwrap();
        graph.add_node("c", Value::Nil).unwrap();
        assert_eq!(graph.node_count(), 3);

        let all = graph.all_nodes().unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_get_node_not_found() {
        let (engine, _dir) = tmp_engine();
        let graph = GraphEngine::new(engine).unwrap();
        assert!(matches!(graph.get_node(999), Err(GraphError::NodeNotFound(999))));
    }

    // --- Edge CRUD tests ---

    #[test]
    fn test_add_and_get_edge() {
        let (engine, _dir) = tmp_engine();
        let graph = GraphEngine::new(engine).unwrap();

        let a = graph.add_node("person", Value::Nil).unwrap();
        let b = graph.add_node("person", Value::Nil).unwrap();

        let edge = graph.add_edge("knows", a.id, b.id, 1.0, Value::Nil).unwrap();
        assert_eq!(edge.source, a.id);
        assert_eq!(edge.target, b.id);
        assert_eq!(edge.weight, 1.0);

        let fetched = graph.get_edge(edge.id).unwrap();
        assert_eq!(fetched.label, "knows");
    }

    #[test]
    fn test_add_edge_invalid_source() {
        let (engine, _dir) = tmp_engine();
        let graph = GraphEngine::new(engine).unwrap();
        let b = graph.add_node("x", Value::Nil).unwrap();
        assert!(matches!(graph.add_edge("e", 999, b.id, 1.0, Value::Nil), Err(GraphError::InvalidSource(999))));
    }

    #[test]
    fn test_add_edge_invalid_target() {
        let (engine, _dir) = tmp_engine();
        let graph = GraphEngine::new(engine).unwrap();
        let a = graph.add_node("x", Value::Nil).unwrap();
        assert!(matches!(graph.add_edge("e", a.id, 999, 1.0, Value::Nil), Err(GraphError::InvalidTarget(999))));
    }

    #[test]
    fn test_add_edge_nan_weight() {
        let (engine, _dir) = tmp_engine();
        let graph = GraphEngine::new(engine).unwrap();
        let a = graph.add_node("x", Value::Nil).unwrap();
        let b = graph.add_node("x", Value::Nil).unwrap();
        assert!(graph.add_edge("e", a.id, b.id, f64::NAN, Value::Nil).is_err());
    }

    #[test]
    fn test_delete_edge_cleans_adjacency() {
        let (engine, _dir) = tmp_engine();
        let graph = GraphEngine::new(engine).unwrap();

        let a = graph.add_node("x", Value::Nil).unwrap();
        let b = graph.add_node("x", Value::Nil).unwrap();
        let edge = graph.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();

        assert_eq!(graph.edges_from(a.id).unwrap().len(), 1);
        assert_eq!(graph.edges_to(b.id).unwrap().len(), 1);

        graph.delete_edge(edge.id).unwrap();

        assert_eq!(graph.edges_from(a.id).unwrap().len(), 0);
        assert_eq!(graph.edges_to(b.id).unwrap().len(), 0);
        assert!(graph.get_edge(edge.id).is_err());
    }

    #[test]
    fn test_edges_from_and_to() {
        let (engine, _dir) = tmp_engine();
        let graph = GraphEngine::new(engine).unwrap();

        let a = graph.add_node("x", Value::Nil).unwrap();
        let b = graph.add_node("x", Value::Nil).unwrap();
        let c = graph.add_node("x", Value::Nil).unwrap();

        graph.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
        graph.add_edge("e", a.id, c.id, 1.0, Value::Nil).unwrap();

        assert_eq!(graph.edges_from(a.id).unwrap().len(), 2);
        assert_eq!(graph.edges_to(b.id).unwrap().len(), 1);
        assert_eq!(graph.edges_to(c.id).unwrap().len(), 1);
    }

    #[test]
    fn test_neighbors() {
        let (engine, _dir) = tmp_engine();
        let graph = GraphEngine::new(engine).unwrap();

        let a = graph.add_node("x", Value::Nil).unwrap();
        let b = graph.add_node("x", Value::Nil).unwrap();
        let c = graph.add_node("x", Value::Nil).unwrap();

        graph.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
        graph.add_edge("e", c.id, a.id, 1.0, Value::Nil).unwrap();

        let n = graph.neighbors(a.id).unwrap();
        assert_eq!(n.len(), 2);
        let ids: Vec<i64> = n.iter().map(|n| n.id).collect();
        assert!(ids.contains(&b.id));
        assert!(ids.contains(&c.id));
    }

    // --- Delete Behaviour tests ---

    #[test]
    fn test_delete_node_detach() {
        let (engine, _dir) = tmp_engine();
        let graph = GraphEngine::new(engine).unwrap();

        let a = graph.add_node("x", Value::Nil).unwrap();
        let b = graph.add_node("x", Value::Nil).unwrap();
        graph.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();

        graph.delete_node(a.id, DeleteBehaviour::Detach).unwrap();
        assert!(graph.get_node(a.id).is_err());
        assert_eq!(graph.edge_count(), 0);
        // b should still exist
        assert!(graph.get_node(b.id).is_ok());
    }

    #[test]
    fn test_delete_node_restrict_with_edges() {
        let (engine, _dir) = tmp_engine();
        let graph = GraphEngine::new(engine).unwrap();

        let a = graph.add_node("x", Value::Nil).unwrap();
        let b = graph.add_node("x", Value::Nil).unwrap();
        graph.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();

        assert!(matches!(
            graph.delete_node(a.id, DeleteBehaviour::Restrict),
            Err(GraphError::DeleteRestricted)
        ));
        // Node should still exist
        assert!(graph.get_node(a.id).is_ok());
    }

    #[test]
    fn test_delete_node_restrict_no_edges() {
        let (engine, _dir) = tmp_engine();
        let graph = GraphEngine::new(engine).unwrap();

        let a = graph.add_node("x", Value::Nil).unwrap();
        graph.delete_node(a.id, DeleteBehaviour::Restrict).unwrap();
        assert!(graph.get_node(a.id).is_err());
    }

    #[test]
    fn test_delete_node_cascade() {
        let (engine, _dir) = tmp_engine();
        let graph = GraphEngine::new(engine).unwrap();

        // a -> b -> c
        let a = graph.add_node("x", Value::Nil).unwrap();
        let b = graph.add_node("x", Value::Nil).unwrap();
        let c = graph.add_node("x", Value::Nil).unwrap();
        graph.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
        graph.add_edge("e", b.id, c.id, 1.0, Value::Nil).unwrap();

        graph.delete_node(a.id, DeleteBehaviour::Cascade).unwrap();
        assert!(graph.get_node(a.id).is_err());
        assert!(graph.get_node(b.id).is_err());
        assert!(graph.get_node(c.id).is_err());
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_delete_node_cascade_with_cycle() {
        let (engine, _dir) = tmp_engine();
        let graph = GraphEngine::new(engine).unwrap();

        // a -> b -> a (cycle)
        let a = graph.add_node("x", Value::Nil).unwrap();
        let b = graph.add_node("x", Value::Nil).unwrap();
        graph.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
        graph.add_edge("e", b.id, a.id, 1.0, Value::Nil).unwrap();

        graph.delete_node(a.id, DeleteBehaviour::Cascade).unwrap();
        assert!(graph.get_node(a.id).is_err());
        assert!(graph.get_node(b.id).is_err());
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_delete_node_nullify() {
        let (engine, _dir) = tmp_engine();
        let graph = GraphEngine::new(engine).unwrap();

        let a = graph.add_node("x", Value::Nil).unwrap();
        let b = graph.add_node("x", Value::Nil).unwrap();
        let edge = graph.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();

        graph.delete_node(a.id, DeleteBehaviour::Nullify).unwrap();
        assert!(graph.get_node(a.id).is_err());

        // Edge should still exist but source set to -1
        let e = graph.get_edge(edge.id).unwrap();
        assert_eq!(e.source, -1);
        assert_eq!(e.target, b.id);
    }

    #[test]
    fn test_update_edge() {
        let (engine, _dir) = tmp_engine();
        let graph = GraphEngine::new(engine).unwrap();

        let a = graph.add_node("x", Value::Nil).unwrap();
        let b = graph.add_node("x", Value::Nil).unwrap();
        let edge = graph.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();

        let updated = graph.update_edge(edge.id, Value::String("updated".into())).unwrap();
        assert_eq!(updated.data, Value::String("updated".into()));
    }
}

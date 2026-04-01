use chrono::{DateTime, Utc};
use rmpv::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: i64,
    pub label: String,
    pub data: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GraphNode {
    pub fn new(id: i64, label: &str, data: Value) -> Self {
        let now = Utc::now();
        GraphNode {
            id,
            label: label.to_string(),
            data,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: i64,
    pub label: String,
    pub source: i64,
    pub target: i64,
    pub weight: f64,
    pub data: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GraphEdge {
    pub fn new(id: i64, label: &str, source: i64, target: i64, weight: f64, data: Value) -> Self {
        let now = Utc::now();
        GraphEdge {
            id,
            label: label.to_string(),
            source,
            target,
            weight,
            data,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteBehaviour {
    Detach,
    Restrict,
    Cascade,
    Nullify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalResult {
    pub nodes: Vec<i64>,
    pub edges: Vec<i64>,
    pub path: Vec<i64>,
    pub total_weight: f64,
}

impl TraversalResult {
    pub fn new() -> Self {
        TraversalResult {
            nodes: Vec::new(),
            edges: Vec::new(),
            path: Vec::new(),
            total_weight: 0.0,
        }
    }
}

impl Default for TraversalResult {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_node_roundtrip() {
        let node = GraphNode::new(1, "person", Value::Map(vec![
            (Value::String("name".into()), Value::String("Alice".into())),
        ]));
        let bytes = rmp_serde::to_vec(&node).unwrap();
        let decoded: GraphNode = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.label, "person");
    }

    #[test]
    fn test_graph_edge_roundtrip() {
        let edge = GraphEdge::new(1, "knows", 1, 2, 1.0, Value::Nil);
        let bytes = rmp_serde::to_vec(&edge).unwrap();
        let decoded: GraphEdge = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.label, "knows");
        assert_eq!(decoded.source, 1);
        assert_eq!(decoded.target, 2);
        assert_eq!(decoded.weight, 1.0);
    }

    #[test]
    fn test_traversal_result_default() {
        let r = TraversalResult::default();
        assert!(r.nodes.is_empty());
        assert!(r.edges.is_empty());
        assert!(r.path.is_empty());
        assert_eq!(r.total_weight, 0.0);
    }
}

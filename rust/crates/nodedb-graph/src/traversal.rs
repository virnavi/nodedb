use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

use crate::engine::GraphEngine;
use crate::error::GraphError;
use crate::types::TraversalResult;

pub fn bfs(engine: &GraphEngine, start_id: i64, max_depth: Option<usize>) -> Result<TraversalResult, GraphError> {
    engine.get_node(start_id)?; // validate start exists

    let max_depth = max_depth.unwrap_or(usize::MAX);
    let mut visited = HashSet::new();
    let mut queue: VecDeque<(i64, usize)> = VecDeque::new();
    let mut result = TraversalResult::new();

    visited.insert(start_id);
    queue.push_back((start_id, 0));
    result.nodes.push(start_id);

    while let Some((node_id, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        let edges = engine.edges_from(node_id)?;
        for edge in edges {
            result.edges.push(edge.id);
            if visited.insert(edge.target) {
                result.nodes.push(edge.target);
                queue.push_back((edge.target, depth + 1));
            }
        }
    }

    Ok(result)
}

pub fn dfs(engine: &GraphEngine, start_id: i64, max_depth: Option<usize>) -> Result<TraversalResult, GraphError> {
    engine.get_node(start_id)?; // validate start exists

    let max_depth = max_depth.unwrap_or(usize::MAX);
    let mut visited = HashSet::new();
    let mut stack: Vec<(i64, usize)> = Vec::new();
    let mut result = TraversalResult::new();

    stack.push((start_id, 0));

    while let Some((node_id, depth)) = stack.pop() {
        if !visited.insert(node_id) {
            continue;
        }
        result.nodes.push(node_id);

        if depth >= max_depth {
            continue;
        }
        let edges = engine.edges_from(node_id)?;
        for edge in edges {
            result.edges.push(edge.id);
            if !visited.contains(&edge.target) {
                stack.push((edge.target, depth + 1));
            }
        }
    }

    Ok(result)
}

#[derive(Debug, Clone)]
struct OrdF64(f64);

impl PartialEq for OrdF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for OrdF64 {}

impl PartialOrd for OrdF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrdF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap behavior in BinaryHeap (which is a max-heap)
        other.0.partial_cmp(&self.0).unwrap_or(Ordering::Equal)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct DijkstraState {
    cost: OrdF64,
    node_id: i64,
}

impl Ord for DijkstraState {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cost.cmp(&other.cost)
    }
}

impl PartialOrd for DijkstraState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn shortest_path(
    engine: &GraphEngine,
    from_id: i64,
    to_id: i64,
) -> Result<Option<TraversalResult>, GraphError> {
    engine.get_node(from_id)?;
    engine.get_node(to_id)?;

    if from_id == to_id {
        let mut result = TraversalResult::new();
        result.nodes.push(from_id);
        result.path.push(from_id);
        return Ok(Some(result));
    }

    let mut dist: HashMap<i64, f64> = HashMap::new();
    let mut prev: HashMap<i64, (i64, i64)> = HashMap::new(); // node -> (prev_node, edge_id)
    let mut heap = BinaryHeap::new();

    dist.insert(from_id, 0.0);
    heap.push(DijkstraState {
        cost: OrdF64(0.0),
        node_id: from_id,
    });

    while let Some(DijkstraState { cost, node_id }) = heap.pop() {
        let current_cost = cost.0;

        if node_id == to_id {
            break;
        }

        if let Some(&best) = dist.get(&node_id) {
            if current_cost > best {
                continue;
            }
        }

        let edges = engine.edges_from(node_id)?;
        for edge in edges {
            let next_cost = current_cost + edge.weight;
            let is_better = match dist.get(&edge.target) {
                Some(&d) => next_cost < d,
                None => true,
            };
            if is_better {
                dist.insert(edge.target, next_cost);
                prev.insert(edge.target, (node_id, edge.id));
                heap.push(DijkstraState {
                    cost: OrdF64(next_cost),
                    node_id: edge.target,
                });
            }
        }
    }

    if !dist.contains_key(&to_id) {
        return Ok(None);
    }

    // Reconstruct path
    let mut result = TraversalResult::new();
    result.total_weight = dist[&to_id];

    let mut path = Vec::new();
    let mut edges = Vec::new();
    let mut current = to_id;
    while current != from_id {
        path.push(current);
        if let Some(&(prev_node, edge_id)) = prev.get(&current) {
            edges.push(edge_id);
            current = prev_node;
        } else {
            break;
        }
    }
    path.push(from_id);
    path.reverse();
    edges.reverse();

    result.path = path.clone();
    result.nodes = path;
    result.edges = edges;

    Ok(Some(result))
}

pub fn multi_hop(
    engine: &GraphEngine,
    start_id: i64,
    edge_labels: &[String],
    hops: usize,
) -> Result<TraversalResult, GraphError> {
    engine.get_node(start_id)?;

    let mut result = TraversalResult::new();
    let mut current_frontier: HashSet<i64> = HashSet::new();
    current_frontier.insert(start_id);
    result.nodes.push(start_id);

    for hop in 0..hops {
        let mut next_frontier: HashSet<i64> = HashSet::new();
        for &node_id in &current_frontier {
            let edges = engine.edges_from(node_id)?;
            for edge in edges {
                let label_matches = edge_labels.is_empty()
                    || edge_labels.iter().any(|l| l == &edge.label);
                if label_matches {
                    result.edges.push(edge.id);
                    if next_frontier.insert(edge.target) && !result.nodes.contains(&edge.target) {
                        result.nodes.push(edge.target);
                    }
                }
            }
        }
        if next_frontier.is_empty() {
            break;
        }
        current_frontier = next_frontier;

        // For exact hop count, only keep the last frontier's nodes on the final hop
        if hop == hops - 1 {
            result.path = current_frontier.into_iter().collect();
            break;
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodedb_storage::StorageEngine;
    use rmpv::Value;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn tmp_graph() -> (GraphEngine, TempDir) {
        let dir = TempDir::new().unwrap();
        let engine = Arc::new(StorageEngine::open(dir.path()).unwrap());
        let graph = GraphEngine::new(engine).unwrap();
        (graph, dir)
    }

    #[test]
    fn test_bfs_linear() {
        let (g, _dir) = tmp_graph();
        // a -> b -> c -> d
        let a = g.add_node("x", Value::Nil).unwrap();
        let b = g.add_node("x", Value::Nil).unwrap();
        let c = g.add_node("x", Value::Nil).unwrap();
        let d = g.add_node("x", Value::Nil).unwrap();
        g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
        g.add_edge("e", b.id, c.id, 1.0, Value::Nil).unwrap();
        g.add_edge("e", c.id, d.id, 1.0, Value::Nil).unwrap();

        let result = bfs(&g, a.id, None).unwrap();
        assert_eq!(result.nodes.len(), 4);
        assert_eq!(result.nodes[0], a.id);
    }

    #[test]
    fn test_bfs_depth_limit() {
        let (g, _dir) = tmp_graph();
        let a = g.add_node("x", Value::Nil).unwrap();
        let b = g.add_node("x", Value::Nil).unwrap();
        let c = g.add_node("x", Value::Nil).unwrap();
        g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
        g.add_edge("e", b.id, c.id, 1.0, Value::Nil).unwrap();

        let result = bfs(&g, a.id, Some(1)).unwrap();
        assert_eq!(result.nodes.len(), 2); // a, b only
        assert!(result.nodes.contains(&a.id));
        assert!(result.nodes.contains(&b.id));
    }

    #[test]
    fn test_dfs_linear() {
        let (g, _dir) = tmp_graph();
        let a = g.add_node("x", Value::Nil).unwrap();
        let b = g.add_node("x", Value::Nil).unwrap();
        let c = g.add_node("x", Value::Nil).unwrap();
        g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
        g.add_edge("e", b.id, c.id, 1.0, Value::Nil).unwrap();

        let result = dfs(&g, a.id, None).unwrap();
        assert_eq!(result.nodes.len(), 3);
    }

    #[test]
    fn test_dfs_depth_limit() {
        let (g, _dir) = tmp_graph();
        let a = g.add_node("x", Value::Nil).unwrap();
        let b = g.add_node("x", Value::Nil).unwrap();
        let c = g.add_node("x", Value::Nil).unwrap();
        g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
        g.add_edge("e", b.id, c.id, 1.0, Value::Nil).unwrap();

        let result = dfs(&g, a.id, Some(1)).unwrap();
        assert_eq!(result.nodes.len(), 2);
    }

    #[test]
    fn test_bfs_diamond() {
        let (g, _dir) = tmp_graph();
        //   a
        //  / \
        // b   c
        //  \ /
        //   d
        let a = g.add_node("x", Value::Nil).unwrap();
        let b = g.add_node("x", Value::Nil).unwrap();
        let c = g.add_node("x", Value::Nil).unwrap();
        let d = g.add_node("x", Value::Nil).unwrap();
        g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
        g.add_edge("e", a.id, c.id, 1.0, Value::Nil).unwrap();
        g.add_edge("e", b.id, d.id, 1.0, Value::Nil).unwrap();
        g.add_edge("e", c.id, d.id, 1.0, Value::Nil).unwrap();

        let result = bfs(&g, a.id, None).unwrap();
        assert_eq!(result.nodes.len(), 4);
    }

    #[test]
    fn test_shortest_path_simple() {
        let (g, _dir) = tmp_graph();
        let a = g.add_node("x", Value::Nil).unwrap();
        let b = g.add_node("x", Value::Nil).unwrap();
        let c = g.add_node("x", Value::Nil).unwrap();
        g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
        g.add_edge("e", b.id, c.id, 2.0, Value::Nil).unwrap();
        g.add_edge("e", a.id, c.id, 10.0, Value::Nil).unwrap();

        let result = shortest_path(&g, a.id, c.id).unwrap().unwrap();
        assert_eq!(result.total_weight, 3.0);
        assert_eq!(result.path, vec![a.id, b.id, c.id]);
    }

    #[test]
    fn test_shortest_path_no_path() {
        let (g, _dir) = tmp_graph();
        let a = g.add_node("x", Value::Nil).unwrap();
        let b = g.add_node("x", Value::Nil).unwrap();

        let result = shortest_path(&g, a.id, b.id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_shortest_path_same_node() {
        let (g, _dir) = tmp_graph();
        let a = g.add_node("x", Value::Nil).unwrap();

        let result = shortest_path(&g, a.id, a.id).unwrap().unwrap();
        assert_eq!(result.path, vec![a.id]);
        assert_eq!(result.total_weight, 0.0);
    }

    #[test]
    fn test_multi_hop_basic() {
        let (g, _dir) = tmp_graph();
        let a = g.add_node("x", Value::Nil).unwrap();
        let b = g.add_node("x", Value::Nil).unwrap();
        let c = g.add_node("x", Value::Nil).unwrap();
        g.add_edge("knows", a.id, b.id, 1.0, Value::Nil).unwrap();
        g.add_edge("knows", b.id, c.id, 1.0, Value::Nil).unwrap();

        let result = multi_hop(&g, a.id, &["knows".to_string()], 2).unwrap();
        assert!(result.nodes.contains(&c.id));
        assert!(result.path.contains(&c.id));
    }

    #[test]
    fn test_multi_hop_label_filter() {
        let (g, _dir) = tmp_graph();
        let a = g.add_node("x", Value::Nil).unwrap();
        let b = g.add_node("x", Value::Nil).unwrap();
        let c = g.add_node("x", Value::Nil).unwrap();
        g.add_edge("knows", a.id, b.id, 1.0, Value::Nil).unwrap();
        g.add_edge("likes", b.id, c.id, 1.0, Value::Nil).unwrap();

        // Only follow "knows" edges
        let result = multi_hop(&g, a.id, &["knows".to_string()], 2).unwrap();
        // Should reach b but not c (because b->c is "likes")
        assert!(result.nodes.contains(&b.id));
        assert!(!result.path.contains(&c.id));
    }

    #[test]
    fn test_bfs_start_not_found() {
        let (g, _dir) = tmp_graph();
        assert!(bfs(&g, 999, None).is_err());
    }

    #[test]
    fn test_dfs_start_not_found() {
        let (g, _dir) = tmp_graph();
        assert!(dfs(&g, 999, None).is_err());
    }
}

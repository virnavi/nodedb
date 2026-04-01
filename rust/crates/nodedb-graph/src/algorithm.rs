use std::collections::{HashMap, HashSet, VecDeque};

use crate::engine::GraphEngine;
use crate::error::GraphError;

pub fn pagerank(
    engine: &GraphEngine,
    damping: f64,
    iterations: usize,
) -> Result<HashMap<i64, f64>, GraphError> {
    let nodes = engine.all_nodes()?;
    let n = nodes.len();
    if n == 0 {
        return Ok(HashMap::new());
    }

    let node_ids: Vec<i64> = nodes.iter().map(|n| n.id).collect();
    let initial_rank = 1.0 / n as f64;

    let mut ranks: HashMap<i64, f64> = node_ids.iter().map(|&id| (id, initial_rank)).collect();

    // Pre-compute outgoing edges for each node
    let mut out_edges: HashMap<i64, Vec<i64>> = HashMap::new();
    for &id in &node_ids {
        let edges = engine.edges_from(id)?;
        out_edges.insert(id, edges.iter().map(|e| e.target).collect());
    }

    for _ in 0..iterations {
        let mut new_ranks: HashMap<i64, f64> = node_ids.iter().map(|&id| (id, (1.0 - damping) / n as f64)).collect();

        for &id in &node_ids {
            let out = &out_edges[&id];
            if out.is_empty() {
                // Dangling node: distribute rank equally
                let share = ranks[&id] / n as f64;
                for &nid in &node_ids {
                    *new_ranks.get_mut(&nid).unwrap() += damping * share;
                }
            } else {
                let share = ranks[&id] / out.len() as f64;
                for &target in out {
                    if let Some(r) = new_ranks.get_mut(&target) {
                        *r += damping * share;
                    }
                }
            }
        }

        ranks = new_ranks;
    }

    Ok(ranks)
}

pub fn connected_components(engine: &GraphEngine) -> Result<Vec<Vec<i64>>, GraphError> {
    let nodes = engine.all_nodes()?;
    let mut visited: HashSet<i64> = HashSet::new();
    let mut components: Vec<Vec<i64>> = Vec::new();

    for node in &nodes {
        if visited.contains(&node.id) {
            continue;
        }

        let mut component = Vec::new();
        let mut queue: VecDeque<i64> = VecDeque::new();
        queue.push_back(node.id);
        visited.insert(node.id);

        while let Some(current) = queue.pop_front() {
            component.push(current);

            // Treat as undirected: follow both outgoing and incoming edges
            let outgoing = engine.edges_from(current)?;
            for edge in &outgoing {
                if visited.insert(edge.target) {
                    queue.push_back(edge.target);
                }
            }
            let incoming = engine.edges_to(current)?;
            for edge in &incoming {
                if visited.insert(edge.source) {
                    queue.push_back(edge.source);
                }
            }
        }

        component.sort();
        components.push(component);
    }

    Ok(components)
}

#[derive(Clone, Copy, PartialEq)]
enum Color {
    White,
    Gray,
    Black,
}

pub fn has_cycle(engine: &GraphEngine) -> Result<bool, GraphError> {
    let nodes = engine.all_nodes()?;
    let mut colors: HashMap<i64, Color> = nodes.iter().map(|n| (n.id, Color::White)).collect();

    for node in &nodes {
        if colors[&node.id] == Color::White {
            if dfs_cycle_detect(engine, node.id, &mut colors)? {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

fn dfs_cycle_detect(
    engine: &GraphEngine,
    node_id: i64,
    colors: &mut HashMap<i64, Color>,
) -> Result<bool, GraphError> {
    // Use explicit stack to avoid deep recursion
    struct Frame {
        node_id: i64,
        edge_idx: usize,
        edges: Vec<i64>,
    }

    colors.insert(node_id, Color::Gray);
    let initial_edges: Vec<i64> = engine.edges_from(node_id)?.iter().map(|e| e.target).collect();

    let mut stack = vec![Frame {
        node_id,
        edge_idx: 0,
        edges: initial_edges,
    }];

    while let Some(frame) = stack.last_mut() {
        if frame.edge_idx >= frame.edges.len() {
            let finished_id = frame.node_id;
            colors.insert(finished_id, Color::Black);
            stack.pop();
            continue;
        }

        let target = frame.edges[frame.edge_idx];
        frame.edge_idx += 1;

        match colors.get(&target) {
            Some(&Color::Gray) => return Ok(true), // Back edge = cycle
            Some(&Color::White) => {
                colors.insert(target, Color::Gray);
                let target_edges: Vec<i64> = engine.edges_from(target)?.iter().map(|e| e.target).collect();
                stack.push(Frame {
                    node_id: target,
                    edge_idx: 0,
                    edges: target_edges,
                });
            }
            _ => {} // Black = already fully explored
        }
    }

    Ok(false)
}

pub fn find_cycles(engine: &GraphEngine) -> Result<Vec<Vec<i64>>, GraphError> {
    let nodes = engine.all_nodes()?;
    let mut colors: HashMap<i64, Color> = nodes.iter().map(|n| (n.id, Color::White)).collect();
    let mut cycles: Vec<Vec<i64>> = Vec::new();

    for node in &nodes {
        if colors[&node.id] == Color::White {
            dfs_find_cycle(engine, node.id, &mut colors, &mut Vec::new(), &mut cycles)?;
            if !cycles.is_empty() {
                return Ok(cycles);
            }
        }
    }

    Ok(cycles)
}

fn dfs_find_cycle(
    engine: &GraphEngine,
    node_id: i64,
    colors: &mut HashMap<i64, Color>,
    path: &mut Vec<i64>,
    cycles: &mut Vec<Vec<i64>>,
) -> Result<(), GraphError> {
    colors.insert(node_id, Color::Gray);
    path.push(node_id);

    let edges = engine.edges_from(node_id)?;
    for edge in &edges {
        if !cycles.is_empty() {
            break; // found one, stop
        }
        match colors.get(&edge.target) {
            Some(&Color::Gray) => {
                // Found a cycle: extract the cycle from path
                if let Some(pos) = path.iter().position(|&id| id == edge.target) {
                    let cycle: Vec<i64> = path[pos..].to_vec();
                    cycles.push(cycle);
                }
            }
            Some(&Color::White) => {
                dfs_find_cycle(engine, edge.target, colors, path, cycles)?;
            }
            _ => {}
        }
    }

    path.pop();
    colors.insert(node_id, Color::Black);
    Ok(())
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
    fn test_pagerank_simple() {
        let (g, _dir) = tmp_graph();
        let a = g.add_node("x", Value::Nil).unwrap();
        let b = g.add_node("x", Value::Nil).unwrap();
        let c = g.add_node("x", Value::Nil).unwrap();
        // a -> b -> c, a -> c
        g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
        g.add_edge("e", b.id, c.id, 1.0, Value::Nil).unwrap();
        g.add_edge("e", a.id, c.id, 1.0, Value::Nil).unwrap();

        let ranks = pagerank(&g, 0.85, 20).unwrap();
        assert_eq!(ranks.len(), 3);
        // c should have highest rank (receives most links)
        assert!(ranks[&c.id] > ranks[&a.id]);
        assert!(ranks[&c.id] > ranks[&b.id]);
    }

    #[test]
    fn test_pagerank_empty() {
        let (g, _dir) = tmp_graph();
        let ranks = pagerank(&g, 0.85, 20).unwrap();
        assert!(ranks.is_empty());
    }

    #[test]
    fn test_pagerank_star() {
        let (g, _dir) = tmp_graph();
        let center = g.add_node("x", Value::Nil).unwrap();
        let mut spokes = Vec::new();
        for _ in 0..5 {
            let s = g.add_node("x", Value::Nil).unwrap();
            g.add_edge("e", s.id, center.id, 1.0, Value::Nil).unwrap();
            spokes.push(s);
        }

        let ranks = pagerank(&g, 0.85, 20).unwrap();
        // Center should have highest rank
        for s in &spokes {
            assert!(ranks[&center.id] > ranks[&s.id]);
        }
    }

    #[test]
    fn test_connected_components_single() {
        let (g, _dir) = tmp_graph();
        let a = g.add_node("x", Value::Nil).unwrap();
        let b = g.add_node("x", Value::Nil).unwrap();
        g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();

        let components = connected_components(&g).unwrap();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), 2);
    }

    #[test]
    fn test_connected_components_disconnected() {
        let (g, _dir) = tmp_graph();
        let a = g.add_node("x", Value::Nil).unwrap();
        let b = g.add_node("x", Value::Nil).unwrap();
        let c = g.add_node("x", Value::Nil).unwrap();
        let d = g.add_node("x", Value::Nil).unwrap();
        g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
        g.add_edge("e", c.id, d.id, 1.0, Value::Nil).unwrap();

        let components = connected_components(&g).unwrap();
        assert_eq!(components.len(), 2);
    }

    #[test]
    fn test_has_cycle_true() {
        let (g, _dir) = tmp_graph();
        let a = g.add_node("x", Value::Nil).unwrap();
        let b = g.add_node("x", Value::Nil).unwrap();
        let c = g.add_node("x", Value::Nil).unwrap();
        g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
        g.add_edge("e", b.id, c.id, 1.0, Value::Nil).unwrap();
        g.add_edge("e", c.id, a.id, 1.0, Value::Nil).unwrap();

        assert!(has_cycle(&g).unwrap());
    }

    #[test]
    fn test_has_cycle_false() {
        let (g, _dir) = tmp_graph();
        let a = g.add_node("x", Value::Nil).unwrap();
        let b = g.add_node("x", Value::Nil).unwrap();
        let c = g.add_node("x", Value::Nil).unwrap();
        g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
        g.add_edge("e", b.id, c.id, 1.0, Value::Nil).unwrap();

        assert!(!has_cycle(&g).unwrap());
    }

    #[test]
    fn test_find_cycles() {
        let (g, _dir) = tmp_graph();
        let a = g.add_node("x", Value::Nil).unwrap();
        let b = g.add_node("x", Value::Nil).unwrap();
        let c = g.add_node("x", Value::Nil).unwrap();
        g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
        g.add_edge("e", b.id, c.id, 1.0, Value::Nil).unwrap();
        g.add_edge("e", c.id, a.id, 1.0, Value::Nil).unwrap();

        let cycles = find_cycles(&g).unwrap();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
    }

    #[test]
    fn test_find_cycles_none() {
        let (g, _dir) = tmp_graph();
        let a = g.add_node("x", Value::Nil).unwrap();
        let b = g.add_node("x", Value::Nil).unwrap();
        g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();

        let cycles = find_cycles(&g).unwrap();
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_has_cycle_empty() {
        let (g, _dir) = tmp_graph();
        assert!(!has_cycle(&g).unwrap());
    }

    #[test]
    fn test_connected_components_isolated() {
        let (g, _dir) = tmp_graph();
        g.add_node("x", Value::Nil).unwrap();
        g.add_node("x", Value::Nil).unwrap();
        g.add_node("x", Value::Nil).unwrap();

        let components = connected_components(&g).unwrap();
        assert_eq!(components.len(), 3);
    }
}

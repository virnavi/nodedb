use std::sync::Arc;

use nodedb_graph::{
    GraphEngine, GraphError, DeleteBehaviour,
    bfs, dfs, shortest_path, multi_hop,
    pagerank, connected_components, has_cycle, find_cycles,
};
use nodedb_storage::StorageEngine;
use rmpv::Value;
use tempfile::TempDir;

fn tmp_graph() -> (GraphEngine, TempDir) {
    let dir = TempDir::new().unwrap();
    let engine = Arc::new(StorageEngine::open(dir.path()).unwrap());
    let graph = GraphEngine::new(engine).unwrap();
    (graph, dir)
}

// --- Full workflow ---

#[test]
fn test_full_graph_workflow() {
    let (g, _dir) = tmp_graph();

    // Create a social network
    let alice = g.add_node("person", Value::String("Alice".into())).unwrap();
    let bob = g.add_node("person", Value::String("Bob".into())).unwrap();
    let carol = g.add_node("person", Value::String("Carol".into())).unwrap();
    let dave = g.add_node("person", Value::String("Dave".into())).unwrap();

    assert_eq!(g.node_count(), 4);

    // Add relationships
    let e1 = g.add_edge("knows", alice.id, bob.id, 1.0, Value::Nil).unwrap();
    let e2 = g.add_edge("knows", alice.id, carol.id, 2.0, Value::Nil).unwrap();
    let _e3 = g.add_edge("knows", bob.id, carol.id, 1.5, Value::Nil).unwrap();
    let _e4 = g.add_edge("knows", carol.id, dave.id, 1.0, Value::Nil).unwrap();

    assert_eq!(g.edge_count(), 4);

    // Traverse
    let bfs_result = bfs(&g, alice.id, None).unwrap();
    assert_eq!(bfs_result.nodes.len(), 4);

    let dfs_result = dfs(&g, alice.id, None).unwrap();
    assert_eq!(dfs_result.nodes.len(), 4);

    // Shortest path
    let sp = shortest_path(&g, alice.id, dave.id).unwrap().unwrap();
    assert_eq!(sp.path.len(), 3); // alice -> carol -> dave (weight 3.0)
    assert_eq!(sp.total_weight, 3.0);

    // Neighbors
    let n = g.neighbors(alice.id).unwrap();
    assert_eq!(n.len(), 2); // bob and carol

    // Update
    g.update_node(alice.id, Value::String("Alice Updated".into())).unwrap();
    let updated = g.get_node(alice.id).unwrap();
    assert_eq!(updated.data, Value::String("Alice Updated".into()));

    g.update_edge(e1.id, Value::String("strong".into())).unwrap();
    let updated_e = g.get_edge(e1.id).unwrap();
    assert_eq!(updated_e.data, Value::String("strong".into()));

    // Algorithms
    let ranks = pagerank(&g, 0.85, 20).unwrap();
    assert_eq!(ranks.len(), 4);

    let components = connected_components(&g).unwrap();
    assert_eq!(components.len(), 1);

    assert!(!has_cycle(&g).unwrap());

    // Delete edge
    g.delete_edge(e2.id).unwrap();
    assert_eq!(g.edge_count(), 3);

    // Delete node with detach
    g.delete_node(dave.id, DeleteBehaviour::Detach).unwrap();
    assert_eq!(g.node_count(), 3);
}

#[test]
fn test_persistence_across_reopen() {
    let dir = TempDir::new().unwrap();

    // First session: create data
    {
        let engine = Arc::new(StorageEngine::open(dir.path()).unwrap());
        let g = GraphEngine::new(engine.clone()).unwrap();

        let a = g.add_node("person", Value::String("Alice".into())).unwrap();
        let b = g.add_node("person", Value::String("Bob".into())).unwrap();
        g.add_edge("knows", a.id, b.id, 1.0, Value::Nil).unwrap();

        g.flush().unwrap();
    }

    // Second session: verify data persists
    {
        let engine = Arc::new(StorageEngine::open(dir.path()).unwrap());
        let g = GraphEngine::new(engine).unwrap();

        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 1);

        let alice = g.get_node(1).unwrap();
        assert_eq!(alice.label, "person");

        let edges = g.edges_from(1).unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].target, 2);
    }
}

#[test]
fn test_delete_behaviours_integration() {
    // Test Restrict
    let (g, _dir) = tmp_graph();
    let a = g.add_node("x", Value::Nil).unwrap();
    let b = g.add_node("x", Value::Nil).unwrap();
    g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();

    assert!(matches!(
        g.delete_node(a.id, DeleteBehaviour::Restrict),
        Err(GraphError::DeleteRestricted)
    ));

    // Test Nullify
    let (g, _dir) = tmp_graph();
    let a = g.add_node("x", Value::Nil).unwrap();
    let b = g.add_node("x", Value::Nil).unwrap();
    let c = g.add_node("x", Value::Nil).unwrap();
    let e1 = g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
    let e2 = g.add_edge("e", c.id, a.id, 1.0, Value::Nil).unwrap();

    g.delete_node(a.id, DeleteBehaviour::Nullify).unwrap();

    let edge1 = g.get_edge(e1.id).unwrap();
    assert_eq!(edge1.source, -1);
    assert_eq!(edge1.target, b.id);

    let edge2 = g.get_edge(e2.id).unwrap();
    assert_eq!(edge2.source, c.id);
    assert_eq!(edge2.target, -1);

    // Test Cascade
    let (g, _dir) = tmp_graph();
    let a = g.add_node("x", Value::Nil).unwrap();
    let b = g.add_node("x", Value::Nil).unwrap();
    let c = g.add_node("x", Value::Nil).unwrap();
    let d = g.add_node("x", Value::Nil).unwrap();
    g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
    g.add_edge("e", b.id, c.id, 1.0, Value::Nil).unwrap();
    g.add_edge("e", c.id, d.id, 1.0, Value::Nil).unwrap();

    g.delete_node(a.id, DeleteBehaviour::Cascade).unwrap();
    assert_eq!(g.node_count(), 0);
    assert_eq!(g.edge_count(), 0);
}

#[test]
fn test_traversal_on_cycle() {
    let (g, _dir) = tmp_graph();
    let a = g.add_node("x", Value::Nil).unwrap();
    let b = g.add_node("x", Value::Nil).unwrap();
    let c = g.add_node("x", Value::Nil).unwrap();
    g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
    g.add_edge("e", b.id, c.id, 1.0, Value::Nil).unwrap();
    g.add_edge("e", c.id, a.id, 1.0, Value::Nil).unwrap();

    // BFS/DFS should not infinite loop
    let bfs_result = bfs(&g, a.id, None).unwrap();
    assert_eq!(bfs_result.nodes.len(), 3);

    let dfs_result = dfs(&g, a.id, None).unwrap();
    assert_eq!(dfs_result.nodes.len(), 3);

    assert!(has_cycle(&g).unwrap());

    let cycles = find_cycles(&g).unwrap();
    assert!(!cycles.is_empty());
}

#[test]
fn test_multi_hop_integration() {
    let (g, _dir) = tmp_graph();
    let a = g.add_node("person", Value::Nil).unwrap();
    let b = g.add_node("person", Value::Nil).unwrap();
    let c = g.add_node("person", Value::Nil).unwrap();
    let d = g.add_node("person", Value::Nil).unwrap();

    g.add_edge("friend", a.id, b.id, 1.0, Value::Nil).unwrap();
    g.add_edge("friend", b.id, c.id, 1.0, Value::Nil).unwrap();
    g.add_edge("colleague", c.id, d.id, 1.0, Value::Nil).unwrap();

    // 2-hop friends-of-friends
    let result = multi_hop(&g, a.id, &["friend".to_string()], 2).unwrap();
    assert!(result.nodes.contains(&c.id));

    // All labels, 3 hops
    let result = multi_hop(&g, a.id, &[], 3).unwrap();
    assert!(result.nodes.contains(&d.id));
}

#[test]
fn test_weighted_shortest_path() {
    let (g, _dir) = tmp_graph();
    //   a --1--> b --1--> d
    //   a --5--> c --1--> d
    let a = g.add_node("x", Value::Nil).unwrap();
    let b = g.add_node("x", Value::Nil).unwrap();
    let c = g.add_node("x", Value::Nil).unwrap();
    let d = g.add_node("x", Value::Nil).unwrap();

    g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();
    g.add_edge("e", b.id, d.id, 1.0, Value::Nil).unwrap();
    g.add_edge("e", a.id, c.id, 5.0, Value::Nil).unwrap();
    g.add_edge("e", c.id, d.id, 1.0, Value::Nil).unwrap();

    let sp = shortest_path(&g, a.id, d.id).unwrap().unwrap();
    assert_eq!(sp.total_weight, 2.0);
    assert_eq!(sp.path, vec![a.id, b.id, d.id]);
}

#[test]
fn test_all_edges_and_all_nodes() {
    let (g, _dir) = tmp_graph();
    let a = g.add_node("x", Value::Nil).unwrap();
    let b = g.add_node("x", Value::Nil).unwrap();
    g.add_edge("e", a.id, b.id, 1.0, Value::Nil).unwrap();

    let nodes = g.all_nodes().unwrap();
    assert_eq!(nodes.len(), 2);

    let edges = g.all_edges().unwrap();
    assert_eq!(edges.len(), 1);
}

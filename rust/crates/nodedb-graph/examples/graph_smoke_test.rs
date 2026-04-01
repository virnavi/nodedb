use std::sync::Arc;

use nodedb_graph::{
    GraphEngine, DeleteBehaviour,
    bfs, dfs, shortest_path, multi_hop,
    pagerank, connected_components, has_cycle, find_cycles,
};
use nodedb_storage::StorageEngine;
use rmpv::Value;

fn main() {
    let dir = tempfile::TempDir::new().unwrap();
    let engine = Arc::new(StorageEngine::open(dir.path()).unwrap());
    let g = GraphEngine::new(engine).unwrap();

    println!("=== NodeDB Graph Engine v0.2 Smoke Test ===\n");

    // Create nodes
    let alice = g.add_node("person", Value::String("Alice".into())).unwrap();
    let bob = g.add_node("person", Value::String("Bob".into())).unwrap();
    let carol = g.add_node("person", Value::String("Carol".into())).unwrap();
    let dave = g.add_node("person", Value::String("Dave".into())).unwrap();
    let eve = g.add_node("person", Value::String("Eve".into())).unwrap();
    println!("Created 5 nodes: Alice({}), Bob({}), Carol({}), Dave({}), Eve({})",
        alice.id, bob.id, carol.id, dave.id, eve.id);

    // Create edges
    g.add_edge("knows", alice.id, bob.id, 1.0, Value::Nil).unwrap();
    g.add_edge("knows", alice.id, carol.id, 2.0, Value::Nil).unwrap();
    g.add_edge("knows", bob.id, carol.id, 1.5, Value::Nil).unwrap();
    g.add_edge("knows", carol.id, dave.id, 1.0, Value::Nil).unwrap();
    g.add_edge("works_with", dave.id, eve.id, 3.0, Value::Nil).unwrap();
    println!("Created 5 edges\n");

    // BFS
    let bfs_result = bfs(&g, alice.id, None).unwrap();
    println!("BFS from Alice: {:?}", bfs_result.nodes);

    // DFS
    let dfs_result = dfs(&g, alice.id, None).unwrap();
    println!("DFS from Alice: {:?}", dfs_result.nodes);

    // Shortest path
    let sp = shortest_path(&g, alice.id, eve.id).unwrap();
    match sp {
        Some(r) => println!("Shortest path Alice->Eve: {:?} (weight: {})", r.path, r.total_weight),
        None => println!("No path from Alice to Eve"),
    }

    // Multi-hop
    let mh = multi_hop(&g, alice.id, &["knows".to_string()], 2).unwrap();
    println!("2-hop 'knows' from Alice: {:?}", mh.nodes);

    // Neighbors
    let neighbors = g.neighbors(alice.id).unwrap();
    println!("Alice's neighbors: {:?}", neighbors.iter().map(|n| n.id).collect::<Vec<_>>());

    // PageRank
    let ranks = pagerank(&g, 0.85, 20).unwrap();
    println!("\nPageRank:");
    let mut rank_vec: Vec<_> = ranks.iter().collect();
    rank_vec.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
    for (id, rank) in &rank_vec {
        let node = g.get_node(**id).unwrap();
        println!("  {} ({}): {:.4}", node.label, node.id, rank);
    }

    // Connected components
    let components = connected_components(&g).unwrap();
    println!("\nConnected components: {}", components.len());

    // Cycle detection
    println!("Has cycle: {}", has_cycle(&g).unwrap());

    // Add a cycle and detect it
    g.add_edge("knows", eve.id, alice.id, 1.0, Value::Nil).unwrap();
    println!("Added Eve->Alice edge");
    println!("Has cycle now: {}", has_cycle(&g).unwrap());
    let cycles = find_cycles(&g).unwrap();
    if !cycles.is_empty() {
        println!("Cycle found: {:?}", cycles[0]);
    }

    // Delete behaviours
    println!("\n--- Delete Behaviours ---");
    let temp_node = g.add_node("temp", Value::Nil).unwrap();
    g.delete_node(temp_node.id, DeleteBehaviour::Restrict).unwrap();
    println!("Restrict delete (no edges): OK");

    println!("\nNode count: {}, Edge count: {}", g.node_count(), g.edge_count());
    println!("\n=== Smoke test passed! ===");
}

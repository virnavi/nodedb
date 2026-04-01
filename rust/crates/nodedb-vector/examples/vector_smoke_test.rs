use std::sync::Arc;
use nodedb_storage::StorageEngine;
use nodedb_vector::{VectorEngine, CollectionConfig, DistanceMetric};
use rmpv::Value;

fn main() {
    let dir = tempfile::TempDir::new().unwrap();
    let engine = Arc::new(StorageEngine::open(dir.path()).unwrap());
    let config = CollectionConfig {
        metric: DistanceMetric::Euclidean,
        ..CollectionConfig::new(4)
    };
    let mut ve = VectorEngine::open(engine, config, dir.path().join("hnsw")).unwrap();

    println!("=== NodeDB Vector Engine Smoke Test ===\n");

    // Insert vectors with metadata
    let vectors = vec![
        (vec![1.0, 0.0, 0.0, 0.0], "x-axis"),
        (vec![0.0, 1.0, 0.0, 0.0], "y-axis"),
        (vec![0.0, 0.0, 1.0, 0.0], "z-axis"),
        (vec![0.0, 0.0, 0.0, 1.0], "w-axis"),
        (vec![0.5, 0.5, 0.0, 0.0], "xy-diagonal"),
        (vec![0.0, 0.5, 0.5, 0.0], "yz-diagonal"),
    ];

    for (v, label) in &vectors {
        let record = ve.insert(v, Value::String((*label).into())).unwrap();
        println!("Inserted id={}: {}", record.id, label);
    }
    println!("\nTotal vectors: {}\n", ve.count());

    // k-NN search
    println!("--- k-NN Search (query near x-axis) ---");
    let results = ve.search(&[0.9, 0.1, 0.0, 0.0], 3, 16).unwrap();
    for r in &results {
        println!("  id={}, distance={:.4}, metadata={}", r.id, r.distance, r.metadata);
    }

    // Filtered search
    println!("\n--- Filtered Search (only diagonals) ---");
    let results = ve
        .search_filtered(&[0.3, 0.3, 0.3, 0.0], 3, 16, |_id, meta| {
            meta.as_str().map_or(false, |s| s.contains("diagonal"))
        })
        .unwrap();
    for r in &results {
        println!("  id={}, distance={:.4}, metadata={}", r.id, r.distance, r.metadata);
    }

    // Delete and search again
    println!("\n--- Delete id=1 (x-axis) and search ---");
    ve.delete(1).unwrap();
    let results = ve.search(&[1.0, 0.0, 0.0, 0.0], 3, 16).unwrap();
    for r in &results {
        println!("  id={}, distance={:.4}, metadata={}", r.id, r.distance, r.metadata);
    }

    // Flush and verify count
    ve.flush().unwrap();
    println!("\nAfter flush: {} vectors", ve.count());

    println!("\n=== Smoke test passed! ===");
}

use std::sync::Arc;
use nodedb_storage::StorageEngine;
use nodedb_vector::{VectorEngine, CollectionConfig, DistanceMetric, VectorError};
use rmpv::Value;
use tempfile::TempDir;

fn setup(dim: usize, metric: DistanceMetric) -> (VectorEngine, TempDir) {
    let dir = TempDir::new().unwrap();
    let engine = Arc::new(StorageEngine::open(dir.path()).unwrap());
    let config = CollectionConfig {
        metric,
        ..CollectionConfig::new(dim)
    };
    let ve = VectorEngine::open(engine, config, dir.path().join("hnsw")).unwrap();
    (ve, dir)
}

#[test]
fn test_full_workflow() {
    let (mut ve, _dir) = setup(3, DistanceMetric::Euclidean);

    // Insert vectors
    let r1 = ve.insert(&[1.0, 0.0, 0.0], Value::String("x-axis".into())).unwrap();
    let r2 = ve.insert(&[0.0, 1.0, 0.0], Value::String("y-axis".into())).unwrap();
    let r3 = ve.insert(&[0.0, 0.0, 1.0], Value::String("z-axis".into())).unwrap();
    assert_eq!(r1.id, 1);
    assert_eq!(r2.id, 2);
    assert_eq!(r3.id, 3);
    assert_eq!(ve.count(), 3);

    // Get
    let (record, vector) = ve.get(1).unwrap();
    assert_eq!(record.id, 1);
    assert_eq!(record.metadata, Value::String("x-axis".into()));
    assert_eq!(vector, vec![1.0, 0.0, 0.0]);

    // Update metadata
    let updated = ve.update_metadata(2, Value::String("updated-y".into())).unwrap();
    assert_eq!(updated.metadata, Value::String("updated-y".into()));

    // Search — nearest to x-axis should be r1
    let results = ve.search(&[0.9, 0.1, 0.0], 2, 16).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, 1);

    // Delete
    ve.delete(3).unwrap();
    assert_eq!(ve.count(), 2);
    assert!(matches!(ve.get(3), Err(VectorError::VectorNotFound(3))));

    // Search after delete — z-axis should not appear
    let results = ve.search(&[0.0, 0.0, 1.0], 3, 16).unwrap();
    for r in &results {
        assert_ne!(r.id, 3);
    }
}

#[test]
fn test_filtered_search() {
    let (mut ve, _dir) = setup(2, DistanceMetric::Euclidean);

    // Insert with category metadata
    let meta = |cat: &str| {
        Value::Map(vec![
            (Value::String("category".into()), Value::String(cat.into())),
        ])
    };

    ve.insert(&[1.0, 0.0], meta("animal")).unwrap();
    ve.insert(&[0.9, 0.1], meta("animal")).unwrap();
    ve.insert(&[0.0, 1.0], meta("plant")).unwrap();
    ve.insert(&[0.1, 0.9], meta("plant")).unwrap();

    // Filter: only animals
    let results = ve
        .search_filtered(&[0.5, 0.5], 10, 16, |_id, metadata| {
            metadata
                .as_map()
                .and_then(|m| {
                    m.iter()
                        .find(|(k, _)| k.as_str() == Some("category"))
                        .and_then(|(_, v)| v.as_str())
                })
                == Some("animal")
        })
        .unwrap();

    assert!(!results.is_empty());
    for r in &results {
        assert!(r.id == 1 || r.id == 2);
    }
}

#[test]
fn test_persistence_full_cycle() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("db");
    let hnsw_path = dir.path().join("hnsw");

    // Phase 1: create and populate
    {
        let engine = Arc::new(StorageEngine::open(&db_path).unwrap());
        let config = CollectionConfig {
            metric: DistanceMetric::Euclidean,
            ..CollectionConfig::new(4)
        };
        let mut ve = VectorEngine::open(engine, config, &hnsw_path).unwrap();

        for i in 0..20 {
            let v = vec![i as f32, (i * 2) as f32, (i * 3) as f32, (i * 4) as f32];
            ve.insert(&v, Value::Integer((i as i64).into())).unwrap();
        }

        ve.flush().unwrap();
    }

    // Phase 2: reopen and verify
    {
        let engine = Arc::new(StorageEngine::open(&db_path).unwrap());
        let config = CollectionConfig {
            metric: DistanceMetric::Euclidean,
            ..CollectionConfig::new(4)
        };
        let ve = VectorEngine::open(engine, config, &hnsw_path).unwrap();

        assert_eq!(ve.count(), 20);

        // All records accessible
        let records = ve.all_records().unwrap();
        assert_eq!(records.len(), 20);

        // Search works
        let results = ve.search(&[5.0, 10.0, 15.0, 20.0], 3, 32).unwrap();
        assert_eq!(results.len(), 3);
        // id=6 (vector [5,10,15,20]) should be closest
        assert_eq!(results[0].id, 6);
    }
}

#[test]
fn test_cosine_search() {
    let (mut ve, _dir) = setup(3, DistanceMetric::Cosine);

    // Vectors pointing in similar directions
    ve.insert(&[1.0, 0.0, 0.0], Value::Nil).unwrap();
    ve.insert(&[0.9, 0.1, 0.0], Value::Nil).unwrap();
    ve.insert(&[0.0, 0.0, 1.0], Value::Nil).unwrap();

    let results = ve.search(&[1.0, 0.0, 0.0], 2, 16).unwrap();
    assert_eq!(results.len(), 2);
    // The first result should be the exact match or very similar direction
    assert!(results[0].id == 1 || results[0].id == 2);
}

#[test]
fn test_dot_product_search() {
    let (mut ve, _dir) = setup(3, DistanceMetric::DotProduct);

    ve.insert(&[1.0, 0.0, 0.0], Value::Nil).unwrap();
    ve.insert(&[0.0, 1.0, 0.0], Value::Nil).unwrap();
    ve.insert(&[0.0, 0.0, 1.0], Value::Nil).unwrap();

    let results = ve.search(&[1.0, 0.0, 0.0], 1, 16).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, 1);
}

#[test]
fn test_delete_and_rebuild() {
    let (mut ve, _dir) = setup(2, DistanceMetric::Euclidean);

    // Insert 20 vectors
    for i in 1..=20 {
        ve.insert(&[i as f32, 0.0], Value::Nil).unwrap();
    }

    // Delete first 15 (75% tombstone ratio, well above 10% threshold)
    for i in 1..=15 {
        ve.delete(i).unwrap();
    }

    // Flush triggers rebuild
    ve.flush().unwrap();

    assert_eq!(ve.count(), 5);

    // Search should still work on remaining vectors
    let results = ve.search(&[16.0, 0.0], 3, 16).unwrap();
    assert!(!results.is_empty());
    // All results should be in 16..=20
    for r in &results {
        assert!(r.id >= 16 && r.id <= 20);
    }
}

#[test]
fn test_performance_1k_vectors() {
    let (mut ve, _dir) = setup(128, DistanceMetric::Cosine);

    // Insert 1000 random-ish vectors
    for i in 0..1000 {
        let v: Vec<f32> = (0..128)
            .map(|j| ((i * 7 + j * 13) % 100) as f32 / 100.0)
            .collect();
        ve.insert(&v, Value::Integer((i as i64).into())).unwrap();
    }

    // Search should complete quickly
    let query: Vec<f32> = (0..128).map(|j| (j % 50) as f32 / 50.0).collect();
    let start = std::time::Instant::now();
    let results = ve.search(&query, 10, 64).unwrap();
    let elapsed = start.elapsed();

    assert_eq!(results.len(), 10);
    assert!(elapsed.as_millis() < 1000, "Search took too long: {:?}", elapsed);
}

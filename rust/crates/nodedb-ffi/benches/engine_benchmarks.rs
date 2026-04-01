use criterion::{criterion_group, criterion_main, Criterion, BatchSize};
use nodedb_ffi::types::*;
use rmpv::Value;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_config(path: &str) -> Vec<u8> {
    let config = Value::Map(vec![(
        Value::String("path".into()),
        Value::String(path.into()),
    )]);
    rmp_serde::to_vec(&config).unwrap()
}

fn make_vector_config(path: &str, dim: u64) -> Vec<u8> {
    let config = Value::Map(vec![
        (Value::String("path".into()), Value::String(path.into())),
        (Value::String("dimension".into()), Value::Integer(dim.into())),
        (Value::String("metric".into()), Value::String("cosine".into())),
        (Value::String("max_elements".into()), Value::Integer(10000.into())),
    ]);
    rmp_serde::to_vec(&config).unwrap()
}

fn open_db(dir: &TempDir) -> DbHandle {
    let config = make_config(dir.path().to_str().unwrap());
    let mut handle: DbHandle = 0;
    let mut error = NodeDbError::none();
    let ok = nodedb_ffi::nodedb_open(config.as_ptr(), config.len(), &mut handle, &mut error);
    assert!(ok, "open_db failed: code={}", error.code);
    handle
}

fn open_graph(dir: &TempDir) -> GraphHandle {
    let config = make_config(dir.path().to_str().unwrap());
    let mut handle: GraphHandle = 0;
    let mut error = NodeDbError::none();
    let ok = nodedb_ffi::nodedb_graph_open(config.as_ptr(), config.len(), &mut handle, &mut error);
    assert!(ok, "open_graph failed: code={}", error.code);
    handle
}

fn open_vector(dir: &TempDir, dim: u64) -> VectorHandle {
    let config = make_vector_config(dir.path().to_str().unwrap(), dim);
    let mut handle: VectorHandle = 0;
    let mut error = NodeDbError::none();
    let ok = nodedb_ffi::nodedb_vector_open(config.as_ptr(), config.len(), &mut handle, &mut error);
    assert!(ok, "open_vector failed: code={}", error.code);
    handle
}

fn open_dac(dir: &TempDir) -> DacHandle {
    let config = make_config(dir.path().to_str().unwrap());
    let mut handle: DacHandle = 0;
    let mut error = NodeDbError::none();
    let ok = nodedb_ffi::nodedb_dac_open(config.as_ptr(), config.len(), &mut handle, &mut error);
    assert!(ok, "open_dac failed: code={}", error.code);
    handle
}

fn open_provenance(dir: &TempDir) -> ProvenanceHandle {
    let config = make_config(dir.path().to_str().unwrap());
    let mut handle: ProvenanceHandle = 0;
    let mut error = NodeDbError::none();
    let ok = nodedb_ffi::nodedb_provenance_open(config.as_ptr(), config.len(), &mut handle, &mut error);
    assert!(ok, "open_provenance failed: code={}", error.code);
    handle
}

fn open_keyresolver(dir: &TempDir) -> KeyResolverHandle {
    let config = make_config(dir.path().to_str().unwrap());
    let mut handle: KeyResolverHandle = 0;
    let mut error = NodeDbError::none();
    let ok = nodedb_ffi::nodedb_keyresolver_open(config.as_ptr(), config.len(), &mut handle, &mut error);
    assert!(ok, "open_keyresolver failed: code={}", error.code);
    handle
}

fn ffi_write(handle: DbHandle, ops: Value) {
    let bytes = rmp_serde::to_vec(&ops).unwrap();
    let mut error = NodeDbError::none();
    let ok = nodedb_ffi::nodedb_write_txn(handle, bytes.as_ptr(), bytes.len(), &mut error);
    assert!(ok, "ffi_write failed: code={}", error.code);
}

fn ffi_query(handle: DbHandle, request: Value) -> Value {
    let bytes = rmp_serde::to_vec(&request).unwrap();
    let mut resp_ptr: *mut u8 = std::ptr::null_mut();
    let mut resp_len: usize = 0;
    let mut error = NodeDbError::none();
    let ok = nodedb_ffi::nodedb_query(
        handle,
        bytes.as_ptr(), bytes.len(),
        &mut resp_ptr, &mut resp_len,
        &mut error,
    );
    assert!(ok, "ffi_query failed: code={}", error.code);
    let resp_bytes = unsafe { std::slice::from_raw_parts(resp_ptr, resp_len) };
    let val: Value = rmp_serde::from_slice(resp_bytes).unwrap();
    nodedb_ffi::nodedb_free_buffer(resp_ptr, resp_len);
    val
}

fn ffi_graph_execute(handle: GraphHandle, request: Value) -> Value {
    let bytes = rmp_serde::to_vec(&request).unwrap();
    let mut resp_ptr: *mut u8 = std::ptr::null_mut();
    let mut resp_len: usize = 0;
    let mut error = NodeDbError::none();
    let ok = nodedb_ffi::nodedb_graph_execute(
        handle,
        bytes.as_ptr(), bytes.len(),
        &mut resp_ptr, &mut resp_len,
        &mut error,
    );
    assert!(ok, "ffi_graph_execute failed: code={}", error.code);
    let resp_bytes = unsafe { std::slice::from_raw_parts(resp_ptr, resp_len) };
    let val: Value = rmp_serde::from_slice(resp_bytes).unwrap();
    nodedb_ffi::nodedb_free_buffer(resp_ptr, resp_len);
    val
}

fn ffi_vector_execute(handle: VectorHandle, request: Value) -> Value {
    let bytes = rmp_serde::to_vec(&request).unwrap();
    let mut resp_ptr: *mut u8 = std::ptr::null_mut();
    let mut resp_len: usize = 0;
    let mut error = NodeDbError::none();
    let ok = nodedb_ffi::nodedb_vector_execute(
        handle,
        bytes.as_ptr(), bytes.len(),
        &mut resp_ptr, &mut resp_len,
        &mut error,
    );
    assert!(ok, "ffi_vector_execute failed: code={}", error.code);
    let resp_bytes = unsafe { std::slice::from_raw_parts(resp_ptr, resp_len) };
    let val: Value = rmp_serde::from_slice(resp_bytes).unwrap();
    nodedb_ffi::nodedb_free_buffer(resp_ptr, resp_len);
    val
}

fn ffi_dac_execute(handle: DacHandle, request: Value) -> Value {
    let bytes = rmp_serde::to_vec(&request).unwrap();
    let mut resp_ptr: *mut u8 = std::ptr::null_mut();
    let mut resp_len: usize = 0;
    let mut error = NodeDbError::none();
    let ok = nodedb_ffi::nodedb_dac_execute(
        handle,
        bytes.as_ptr(), bytes.len(),
        &mut resp_ptr, &mut resp_len,
        &mut error,
    );
    assert!(ok, "ffi_dac_execute failed: code={}", error.code);
    let resp_bytes = unsafe { std::slice::from_raw_parts(resp_ptr, resp_len) };
    let val: Value = rmp_serde::from_slice(resp_bytes).unwrap();
    nodedb_ffi::nodedb_free_buffer(resp_ptr, resp_len);
    val
}

fn ffi_provenance_execute(handle: ProvenanceHandle, request: Value) -> Value {
    let bytes = rmp_serde::to_vec(&request).unwrap();
    let mut resp_ptr: *mut u8 = std::ptr::null_mut();
    let mut resp_len: usize = 0;
    let mut error = NodeDbError::none();
    let ok = nodedb_ffi::nodedb_provenance_execute(
        handle,
        bytes.as_ptr(), bytes.len(),
        &mut resp_ptr, &mut resp_len,
        &mut error,
    );
    assert!(ok, "ffi_provenance_execute failed: code={}", error.code);
    let resp_bytes = unsafe { std::slice::from_raw_parts(resp_ptr, resp_len) };
    let val: Value = rmp_serde::from_slice(resp_bytes).unwrap();
    nodedb_ffi::nodedb_free_buffer(resp_ptr, resp_len);
    val
}

fn ffi_keyresolver_execute(handle: KeyResolverHandle, request: Value) -> Value {
    let bytes = rmp_serde::to_vec(&request).unwrap();
    let mut resp_ptr: *mut u8 = std::ptr::null_mut();
    let mut resp_len: usize = 0;
    let mut error = NodeDbError::none();
    let ok = nodedb_ffi::nodedb_keyresolver_execute(
        handle,
        bytes.as_ptr(), bytes.len(),
        &mut resp_ptr, &mut resp_len,
        &mut error,
    );
    assert!(ok, "ffi_keyresolver_execute failed: code={}", error.code);
    let resp_bytes = unsafe { std::slice::from_raw_parts(resp_ptr, resp_len) };
    let val: Value = rmp_serde::from_slice(resp_bytes).unwrap();
    nodedb_ffi::nodedb_free_buffer(resp_ptr, resp_len);
    val
}

// ---------------------------------------------------------------------------
// NoSQL benchmarks
// ---------------------------------------------------------------------------

fn bench_nosql(c: &mut Criterion) {
    let mut group = c.benchmark_group("nosql");

    // --- single_write ---
    group.bench_function("single_write", |b| {
        let dir = TempDir::new().unwrap();
        let handle = open_db(&dir);
        let mut idx = 0u64;
        b.iter(|| {
            idx += 1;
            let ops = Value::Array(vec![Value::Map(vec![
                (Value::String("collection".into()), Value::String("bench".into())),
                (Value::String("action".into()), Value::String("put".into())),
                (Value::String("data".into()), Value::Map(vec![
                    (Value::String("name".into()), Value::String(format!("user_{}", idx).into())),
                    (Value::String("age".into()), Value::Integer((idx % 100).into())),
                ])),
            ])]);
            ffi_write(handle, ops);
        });
        nodedb_ffi::nodedb_close(handle);
    });

    // --- get_by_id ---
    group.bench_function("get_by_id", |b| {
        let dir = TempDir::new().unwrap();
        let handle = open_db(&dir);
        // Seed 100 documents
        for i in 0..100 {
            let ops = Value::Array(vec![Value::Map(vec![
                (Value::String("collection".into()), Value::String("bench".into())),
                (Value::String("action".into()), Value::String("put".into())),
                (Value::String("data".into()), Value::Map(vec![
                    (Value::String("name".into()), Value::String(format!("user_{}", i).into())),
                    (Value::String("age".into()), Value::Integer(i.into())),
                ])),
            ])]);
            ffi_write(handle, ops);
        }
        b.iter(|| {
            let req = Value::Map(vec![
                (Value::String("collection".into()), Value::String("bench".into())),
                (Value::String("action".into()), Value::String("get".into())),
                (Value::String("id".into()), Value::Integer(50.into())),
            ]);
            ffi_query(handle, req)
        });
        nodedb_ffi::nodedb_close(handle);
    });

    // --- filtered_query ---
    group.bench_function("filtered_query", |b| {
        let dir = TempDir::new().unwrap();
        let handle = open_db(&dir);
        // Seed 1000 documents
        for i in 0..1000 {
            let ops = Value::Array(vec![Value::Map(vec![
                (Value::String("collection".into()), Value::String("bench".into())),
                (Value::String("action".into()), Value::String("put".into())),
                (Value::String("data".into()), Value::Map(vec![
                    (Value::String("name".into()), Value::String(format!("user_{}", i).into())),
                    (Value::String("age".into()), Value::Integer((i % 100).into())),
                    (Value::String("active".into()), Value::Boolean(i % 2 == 0)),
                ])),
            ])]);
            ffi_write(handle, ops);
        }
        b.iter(|| {
            let req = Value::Map(vec![
                (Value::String("collection".into()), Value::String("bench".into())),
                (Value::String("action".into()), Value::String("find_all".into())),
            ]);
            ffi_query(handle, req)
        });
        nodedb_ffi::nodedb_close(handle);
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Graph benchmarks
// ---------------------------------------------------------------------------

fn bench_graph(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph");

    // --- add_node ---
    group.bench_function("add_node", |b| {
        let dir = TempDir::new().unwrap();
        let handle = open_graph(&dir);
        let mut idx = 0u64;
        b.iter(|| {
            idx += 1;
            let req = Value::Map(vec![
                (Value::String("action".into()), Value::String("add_node".into())),
                (Value::String("label".into()), Value::String("Person".into())),
                (Value::String("properties".into()), Value::Map(vec![
                    (Value::String("name".into()), Value::String(format!("node_{}", idx).into())),
                ])),
            ]);
            ffi_graph_execute(handle, req)
        });
        nodedb_ffi::nodedb_graph_close(handle);
    });

    // --- add_edge ---
    group.bench_function("add_edge", |b| {
        let dir = TempDir::new().unwrap();
        let handle = open_graph(&dir);
        // Seed 200 nodes
        for i in 0..200 {
            let req = Value::Map(vec![
                (Value::String("action".into()), Value::String("add_node".into())),
                (Value::String("label".into()), Value::String("Person".into())),
                (Value::String("properties".into()), Value::Map(vec![
                    (Value::String("name".into()), Value::String(format!("n_{}", i).into())),
                ])),
            ]);
            ffi_graph_execute(handle, req);
        }
        let mut idx = 0u64;
        b.iter(|| {
            idx += 1;
            let src = (idx % 200) + 1;
            let tgt = ((idx + 1) % 200) + 1;
            let req = Value::Map(vec![
                (Value::String("action".into()), Value::String("add_edge".into())),
                (Value::String("source".into()), Value::Integer(src.into())),
                (Value::String("target".into()), Value::Integer(tgt.into())),
                (Value::String("label".into()), Value::String("KNOWS".into())),
                (Value::String("properties".into()), Value::Map(vec![])),
            ]);
            ffi_graph_execute(handle, req)
        });
        nodedb_ffi::nodedb_graph_close(handle);
    });

    // --- traversal_depth3 ---
    group.bench_function("traversal_depth3", |b| {
        let dir = TempDir::new().unwrap();
        let handle = open_graph(&dir);
        // Build a small graph: 100 nodes, chain edges
        for i in 0..100 {
            let req = Value::Map(vec![
                (Value::String("action".into()), Value::String("add_node".into())),
                (Value::String("label".into()), Value::String("Person".into())),
                (Value::String("properties".into()), Value::Map(vec![
                    (Value::String("name".into()), Value::String(format!("n_{}", i).into())),
                ])),
            ]);
            ffi_graph_execute(handle, req);
        }
        for i in 1..100u64 {
            let req = Value::Map(vec![
                (Value::String("action".into()), Value::String("add_edge".into())),
                (Value::String("source".into()), Value::Integer(i.into())),
                (Value::String("target".into()), Value::Integer((i + 1).into())),
                (Value::String("label".into()), Value::String("NEXT".into())),
                (Value::String("properties".into()), Value::Map(vec![])),
            ]);
            ffi_graph_execute(handle, req);
        }
        b.iter(|| {
            let req = Value::Map(vec![
                (Value::String("action".into()), Value::String("traverse".into())),
                (Value::String("start_node".into()), Value::Integer(1.into())),
                (Value::String("max_depth".into()), Value::Integer(3.into())),
                (Value::String("direction".into()), Value::String("outgoing".into())),
            ]);
            ffi_graph_execute(handle, req)
        });
        nodedb_ffi::nodedb_graph_close(handle);
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Vector benchmarks
// ---------------------------------------------------------------------------

fn bench_vector(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector");
    let dim: u64 = 128;

    fn random_vector(dim: usize) -> Vec<Value> {
        (0..dim)
            .map(|i| Value::F64((((i * 7 + 13) % 100) as f64) / 100.0))
            .collect()
    }

    // --- insert_vector ---
    group.bench_function("insert_vector", |b| {
        let dir = TempDir::new().unwrap();
        let handle = open_vector(&dir, dim);
        let mut idx = 0u64;
        b.iter(|| {
            idx += 1;
            let req = Value::Map(vec![
                (Value::String("action".into()), Value::String("insert".into())),
                (Value::String("id".into()), Value::String(format!("vec_{}", idx).into())),
                (Value::String("vector".into()), Value::Array(random_vector(dim as usize))),
                (Value::String("metadata".into()), Value::Map(vec![
                    (Value::String("label".into()), Value::String("test".into())),
                ])),
            ]);
            ffi_vector_execute(handle, req)
        });
        nodedb_ffi::nodedb_vector_close(handle);
    });

    // --- knn_search ---
    group.bench_function("knn_search_k10", |b| {
        let dir = TempDir::new().unwrap();
        let handle = open_vector(&dir, dim);
        // Seed 500 vectors
        for i in 0..500 {
            let vec_data: Vec<Value> = (0..dim)
                .map(|j| Value::F64((((i * 7 + j * 3 + 13) % 100) as f64) / 100.0))
                .collect();
            let req = Value::Map(vec![
                (Value::String("action".into()), Value::String("insert".into())),
                (Value::String("id".into()), Value::String(format!("v_{}", i).into())),
                (Value::String("vector".into()), Value::Array(vec_data)),
                (Value::String("metadata".into()), Value::Map(vec![])),
            ]);
            ffi_vector_execute(handle, req);
        }
        let query_vec = random_vector(dim as usize);
        b.iter(|| {
            let req = Value::Map(vec![
                (Value::String("action".into()), Value::String("search".into())),
                (Value::String("vector".into()), Value::Array(query_vec.clone())),
                (Value::String("k".into()), Value::Integer(10.into())),
            ]);
            ffi_vector_execute(handle, req)
        });
        nodedb_ffi::nodedb_vector_close(handle);
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// DAC benchmarks
// ---------------------------------------------------------------------------

fn bench_dac(c: &mut Criterion) {
    let mut group = c.benchmark_group("dac");

    // --- add_rule ---
    group.bench_function("add_rule", |b| {
        let dir = TempDir::new().unwrap();
        let handle = open_dac(&dir);
        let mut idx = 0u64;
        b.iter(|| {
            idx += 1;
            let req = Value::Map(vec![
                (Value::String("action".into()), Value::String("add_rule".into())),
                (Value::String("collection".into()), Value::String("docs".into())),
                (Value::String("subject_type".into()), Value::String("user".into())),
                (Value::String("subject_id".into()), Value::String(format!("user_{}", idx).into())),
                (Value::String("permission".into()), Value::String("read".into())),
            ]);
            ffi_dac_execute(handle, req)
        });
        nodedb_ffi::nodedb_dac_close(handle);
    });

    // --- evaluate_rules ---
    group.bench_function("evaluate_100_rules", |b| {
        let dir = TempDir::new().unwrap();
        let handle = open_dac(&dir);
        // Seed 100 rules
        for i in 0..100 {
            let req = Value::Map(vec![
                (Value::String("action".into()), Value::String("add_rule".into())),
                (Value::String("collection".into()), Value::String("docs".into())),
                (Value::String("subject_type".into()), Value::String("user".into())),
                (Value::String("subject_id".into()), Value::String(format!("user_{}", i).into())),
                (Value::String("permission".into()), Value::String("read".into())),
            ]);
            ffi_dac_execute(handle, req);
        }
        b.iter(|| {
            let req = Value::Map(vec![
                (Value::String("action".into()), Value::String("check_access".into())),
                (Value::String("collection".into()), Value::String("docs".into())),
                (Value::String("subject_type".into()), Value::String("user".into())),
                (Value::String("subject_id".into()), Value::String("user_50".into())),
                (Value::String("permission".into()), Value::String("read".into())),
            ]);
            ffi_dac_execute(handle, req)
        });
        nodedb_ffi::nodedb_dac_close(handle);
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Provenance benchmarks
// ---------------------------------------------------------------------------

fn bench_provenance(c: &mut Criterion) {
    let mut group = c.benchmark_group("provenance");

    // --- attach_envelope ---
    group.bench_function("attach_envelope", |b| {
        let dir = TempDir::new().unwrap();
        let handle = open_provenance(&dir);
        let mut idx = 0u64;
        b.iter(|| {
            idx += 1;
            let req = Value::Map(vec![
                (Value::String("action".into()), Value::String("attach".into())),
                (Value::String("document_id".into()), Value::String(format!("doc_{}", idx).into())),
                (Value::String("collection".into()), Value::String("bench".into())),
                (Value::String("source_type".into()), Value::String("device".into())),
                (Value::String("source_id".into()), Value::String("device-1".into())),
                (Value::String("confidence".into()), Value::F64(0.95)),
            ]);
            ffi_provenance_execute(handle, req)
        });
        nodedb_ffi::nodedb_provenance_close(handle);
    });

    // --- compute_hash ---
    group.bench_function("compute_hash", |b| {
        let dir = TempDir::new().unwrap();
        let handle = open_provenance(&dir);
        // Seed one envelope
        let req = Value::Map(vec![
            (Value::String("action".into()), Value::String("attach".into())),
            (Value::String("document_id".into()), Value::String("doc_1".into())),
            (Value::String("collection".into()), Value::String("bench".into())),
            (Value::String("source_type".into()), Value::String("device".into())),
            (Value::String("source_id".into()), Value::String("device-1".into())),
            (Value::String("confidence".into()), Value::F64(0.9)),
        ]);
        ffi_provenance_execute(handle, req);
        b.iter(|| {
            let req = Value::Map(vec![
                (Value::String("action".into()), Value::String("compute_hash".into())),
                (Value::String("envelope_id".into()), Value::Integer(1.into())),
            ]);
            ffi_provenance_execute(handle, req)
        });
        nodedb_ffi::nodedb_provenance_close(handle);
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// KeyResolver benchmarks
// ---------------------------------------------------------------------------

fn bench_keyresolver(c: &mut Criterion) {
    let mut group = c.benchmark_group("keyresolver");

    // --- register_key ---
    group.bench_function("register_key", |b| {
        let dir = TempDir::new().unwrap();
        let handle = open_keyresolver(&dir);
        let mut idx = 0u64;
        b.iter(|| {
            idx += 1;
            // Generate a deterministic 32-byte hex key
            let key_hex = format!("{:064x}", idx);
            let req = Value::Map(vec![
                (Value::String("action".into()), Value::String("register".into())),
                (Value::String("public_key_hex".into()), Value::String(key_hex.into())),
                (Value::String("peer_id".into()), Value::String(format!("peer_{}", idx).into())),
                (Value::String("trust_level".into()), Value::String("verified".into())),
            ]);
            ffi_keyresolver_execute(handle, req)
        });
        nodedb_ffi::nodedb_keyresolver_close(handle);
    });

    // --- resolve_key (cache lookup) ---
    group.bench_function("resolve_key", |b| {
        let dir = TempDir::new().unwrap();
        let handle = open_keyresolver(&dir);
        // Register 100 keys
        for i in 0..100u64 {
            let key_hex = format!("{:064x}", i + 1);
            let req = Value::Map(vec![
                (Value::String("action".into()), Value::String("register".into())),
                (Value::String("public_key_hex".into()), Value::String(key_hex.into())),
                (Value::String("peer_id".into()), Value::String(format!("peer_{}", i).into())),
                (Value::String("trust_level".into()), Value::String("verified".into())),
            ]);
            ffi_keyresolver_execute(handle, req);
        }
        let lookup_key = format!("{:064x}", 50u64);
        b.iter(|| {
            let req = Value::Map(vec![
                (Value::String("action".into()), Value::String("resolve".into())),
                (Value::String("public_key_hex".into()), Value::String(lookup_key.clone().into())),
            ]);
            ffi_keyresolver_execute(handle, req)
        });
        nodedb_ffi::nodedb_keyresolver_close(handle);
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// FFI overhead benchmark (open -> query -> close cycle)
// ---------------------------------------------------------------------------

fn bench_ffi_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("ffi_overhead");

    group.bench_function("open_query_close", |b| {
        b.iter_batched(
            || TempDir::new().unwrap(),
            |dir| {
                let handle = open_db(&dir);
                let req = Value::Map(vec![
                    (Value::String("collection".into()), Value::String("test".into())),
                    (Value::String("action".into()), Value::String("find_all".into())),
                ]);
                let _ = ffi_query(handle, req);
                nodedb_ffi::nodedb_close(handle);
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Criterion main
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_nosql,
    bench_graph,
    bench_vector,
    bench_dac,
    bench_provenance,
    bench_keyresolver,
    bench_ffi_overhead,
);
criterion_main!(benches);

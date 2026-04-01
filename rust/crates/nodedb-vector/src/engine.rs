use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use hnsw_rs::prelude::*;
use nodedb_storage::{
    encode_id, decode_id, from_msgpack, to_msgpack, IdGenerator, StorageEngine, StorageTree,
};
use rmpv::Value;

use crate::error::VectorError;
use crate::types::{CollectionConfig, DistanceMetric, SearchResult, VectorRecord};

// ---------------------------------------------------------------------------
// HnswIndex — enum dispatch over distance metrics to avoid dyn dispatch
// ---------------------------------------------------------------------------

enum HnswIndex {
    Cosine(Hnsw<'static, f32, DistCosine>),
    Euclidean(Hnsw<'static, f32, DistL2>),
    DotProduct(Hnsw<'static, f32, DistDot>),
}

impl HnswIndex {
    fn new(config: &CollectionConfig) -> Self {
        match config.metric {
            DistanceMetric::Cosine => HnswIndex::Cosine(Hnsw::new(
                config.max_nb_connection,
                config.max_elements,
                config.max_layer,
                config.ef_construction,
                DistCosine,
            )),
            DistanceMetric::Euclidean => HnswIndex::Euclidean(Hnsw::new(
                config.max_nb_connection,
                config.max_elements,
                config.max_layer,
                config.ef_construction,
                DistL2,
            )),
            DistanceMetric::DotProduct => HnswIndex::DotProduct(Hnsw::new(
                config.max_nb_connection,
                config.max_elements,
                config.max_layer,
                config.ef_construction,
                DistDot,
            )),
        }
    }

    fn insert(&self, vector: &[f32], id: usize) {
        match self {
            HnswIndex::Cosine(h) => h.insert((vector, id)),
            HnswIndex::Euclidean(h) => h.insert((vector, id)),
            HnswIndex::DotProduct(h) => h.insert((vector, id)),
        }
    }

    fn search(&self, query: &[f32], k: usize, ef_search: usize) -> Vec<Neighbour> {
        match self {
            HnswIndex::Cosine(h) => h.search(query, k, ef_search),
            HnswIndex::Euclidean(h) => h.search(query, k, ef_search),
            HnswIndex::DotProduct(h) => h.search(query, k, ef_search),
        }
    }

    fn search_filter(
        &self,
        query: &[f32],
        k: usize,
        ef_search: usize,
        filter: &dyn FilterT,
    ) -> Vec<Neighbour> {
        match self {
            HnswIndex::Cosine(h) => h.search_filter(query, k, ef_search, Some(filter)),
            HnswIndex::Euclidean(h) => h.search_filter(query, k, ef_search, Some(filter)),
            HnswIndex::DotProduct(h) => h.search_filter(query, k, ef_search, Some(filter)),
        }
    }

    fn get_nb_point(&self) -> usize {
        match self {
            HnswIndex::Cosine(h) => h.get_nb_point(),
            HnswIndex::Euclidean(h) => h.get_nb_point(),
            HnswIndex::DotProduct(h) => h.get_nb_point(),
        }
    }

    fn file_dump(&self, path: &Path, basename: &str) -> Result<(), VectorError> {
        let result = match self {
            HnswIndex::Cosine(h) => h.file_dump(path, basename),
            HnswIndex::Euclidean(h) => h.file_dump(path, basename),
            HnswIndex::DotProduct(h) => h.file_dump(path, basename),
        };
        result.map(|_| ()).map_err(|e| VectorError::Index(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Filter adapters for hnsw_rs FilterT trait
// ---------------------------------------------------------------------------

struct TombstoneFilter<'a> {
    tombstones: &'a HashSet<usize>,
}

impl FilterT for TombstoneFilter<'_> {
    fn hnsw_filter(&self, id: &usize) -> bool {
        !self.tombstones.contains(id)
    }
}

struct MetadataFilter<'a, F: Fn(i64, &Value) -> bool> {
    filter_fn: &'a F,
    records: &'a StorageTree,
    tombstones: &'a HashSet<usize>,
}

impl<F: Fn(i64, &Value) -> bool> FilterT for MetadataFilter<'_, F> {
    fn hnsw_filter(&self, id: &usize) -> bool {
        if self.tombstones.contains(id) {
            return false;
        }
        let internal_id = *id as i64;
        match self.records.get(&encode_id(internal_id)) {
            Ok(Some(bytes)) => match from_msgpack::<VectorRecord>(&bytes) {
                Ok(record) => (self.filter_fn)(internal_id, &record.metadata),
                Err(_) => false,
            },
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// VectorEngine
// ---------------------------------------------------------------------------

pub struct VectorEngine {
    engine: Arc<StorageEngine>,
    #[allow(dead_code)]
    meta: StorageTree,
    records: StorageTree,
    data: StorageTree,
    id_gen: Arc<IdGenerator>,
    config: CollectionConfig,
    hnsw: HnswIndex,
    tombstones: HashSet<usize>,
    vectors: HashMap<usize, Vec<f32>>,
    data_dir: PathBuf,
}

impl VectorEngine {
    /// Open or create a vector engine.
    pub fn open(
        engine: Arc<StorageEngine>,
        config: CollectionConfig,
        data_dir: impl AsRef<Path>,
    ) -> Result<Self, VectorError> {
        if config.dimension == 0 {
            return Err(VectorError::InvalidDimension(0));
        }

        let data_dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| VectorError::Index(format!("cannot create data dir: {}", e)))?;

        let meta = engine.open_tree("__vector_meta__")?;
        let records = engine.open_tree("__vector_records__")?;
        let data = engine.open_tree("__vector_data__")?;
        let id_gen = Arc::new(IdGenerator::new(&engine)?);

        // Persist or validate config
        let config_key = b"config";
        if let Some(existing_bytes) = meta.get(config_key)? {
            let existing: CollectionConfig = from_msgpack(&existing_bytes)?;
            if existing.dimension != config.dimension {
                return Err(VectorError::DimensionMismatch {
                    expected: existing.dimension,
                    got: config.dimension,
                });
            }
            if existing.metric != config.metric {
                return Err(VectorError::InvalidMetric(format!(
                    "existing metric {:?} != requested {:?}",
                    existing.metric, config.metric
                )));
            }
        } else {
            let config_bytes = to_msgpack(&config)?;
            meta.insert(config_key, &config_bytes)?;
        }

        // Build HNSW index from sled data
        let hnsw = HnswIndex::new(&config);
        let mut vectors = HashMap::new();

        for item in data.iter() {
            let (key_bytes, val_bytes) = item?;
            let id = decode_id(&key_bytes)?;
            let uid = id as usize;
            let vector = bytes_to_f32_vec(&val_bytes);
            hnsw.insert(&vector, uid);
            vectors.insert(uid, vector);
        }

        Ok(VectorEngine {
            engine,
            meta,
            records,
            data,
            id_gen,
            config,
            hnsw,
            tombstones: HashSet::new(),
            vectors,
            data_dir,
        })
    }

    /// Insert a vector with metadata. Returns the new record.
    pub fn insert(
        &mut self,
        vector: &[f32],
        metadata: Value,
    ) -> Result<VectorRecord, VectorError> {
        if vector.len() != self.config.dimension {
            return Err(VectorError::DimensionMismatch {
                expected: self.config.dimension,
                got: vector.len(),
            });
        }

        let id = self.id_gen.next_id("vector")?;
        let uid = id as usize;
        let record = VectorRecord::new(id, metadata);

        // Store record metadata in sled
        let record_bytes = to_msgpack(&record)?;
        self.records.insert(&encode_id(id), &record_bytes)?;

        // Store raw vector bytes in sled
        let raw_bytes = f32_vec_to_bytes(vector);
        self.data.insert(&encode_id(id), &raw_bytes)?;

        // Insert into HNSW index
        self.hnsw.insert(vector, uid);
        self.vectors.insert(uid, vector.to_vec());

        Ok(record)
    }

    /// Get a vector record and its raw vector by ID.
    pub fn get(&self, id: i64) -> Result<(VectorRecord, Vec<f32>), VectorError> {
        let key = encode_id(id);
        let record_bytes = self
            .records
            .get(&key)?
            .ok_or(VectorError::VectorNotFound(id))?;
        let record: VectorRecord = from_msgpack(&record_bytes)?;

        let data_bytes = self
            .data
            .get(&key)?
            .ok_or(VectorError::VectorNotFound(id))?;
        let vector = bytes_to_f32_vec(&data_bytes);

        Ok((record, vector))
    }

    /// Update only the metadata for a vector (vector data unchanged).
    pub fn update_metadata(
        &self,
        id: i64,
        metadata: Value,
    ) -> Result<VectorRecord, VectorError> {
        let key = encode_id(id);
        let record_bytes = self
            .records
            .get(&key)?
            .ok_or(VectorError::VectorNotFound(id))?;
        let mut record: VectorRecord = from_msgpack(&record_bytes)?;
        record.metadata = metadata;
        record.updated_at = chrono::Utc::now();
        let new_bytes = to_msgpack(&record)?;
        self.records.insert(&key, &new_bytes)?;
        Ok(record)
    }

    /// Delete a vector by ID. Uses tombstone-based deletion for HNSW.
    pub fn delete(&mut self, id: i64) -> Result<(), VectorError> {
        let key = encode_id(id);
        if self.records.get(&key)?.is_none() {
            return Err(VectorError::VectorNotFound(id));
        }
        self.records.remove(&key)?;
        self.data.remove(&key)?;
        let uid = id as usize;
        self.tombstones.insert(uid);
        self.vectors.remove(&uid);
        Ok(())
    }

    /// Number of live (non-tombstoned) vectors.
    pub fn count(&self) -> usize {
        self.records.len()
    }

    /// Iterate all live vector records.
    pub fn all_records(&self) -> Result<Vec<VectorRecord>, VectorError> {
        let mut out = Vec::new();
        for item in self.records.iter() {
            let (_, bytes) = item?;
            let record: VectorRecord = from_msgpack(&bytes)?;
            out.push(record);
        }
        Ok(out)
    }

    // -----------------------------------------------------------------------
    // Search
    // -----------------------------------------------------------------------

    /// k-NN search. Returns up to k nearest neighbors.
    pub fn search(
        &self,
        query: &[f32],
        k: usize,
        ef_search: usize,
    ) -> Result<Vec<SearchResult>, VectorError> {
        if query.len() != self.config.dimension {
            return Err(VectorError::DimensionMismatch {
                expected: self.config.dimension,
                got: query.len(),
            });
        }

        if self.hnsw.get_nb_point() == 0 {
            return Ok(Vec::new());
        }

        let results = if self.tombstones.is_empty() {
            self.hnsw.search(query, k, ef_search)
        } else {
            let filter = TombstoneFilter {
                tombstones: &self.tombstones,
            };
            self.hnsw
                .search_filter(query, k + self.tombstones.len(), ef_search, &filter)
        };

        self.neighbours_to_results(results, k)
    }

    /// k-NN search with a metadata filter.
    /// `filter_fn(id, metadata) -> bool` — return true to include.
    pub fn search_filtered<F>(
        &self,
        query: &[f32],
        k: usize,
        ef_search: usize,
        filter_fn: F,
    ) -> Result<Vec<SearchResult>, VectorError>
    where
        F: Fn(i64, &Value) -> bool,
    {
        if query.len() != self.config.dimension {
            return Err(VectorError::DimensionMismatch {
                expected: self.config.dimension,
                got: query.len(),
            });
        }

        if self.hnsw.get_nb_point() == 0 {
            return Ok(Vec::new());
        }

        let filter = MetadataFilter {
            filter_fn: &filter_fn,
            records: &self.records,
            tombstones: &self.tombstones,
        };

        let results = self
            .hnsw
            .search_filter(query, k + self.tombstones.len(), ef_search, &filter);

        self.neighbours_to_results(results, k)
    }

    fn neighbours_to_results(
        &self,
        neighbours: Vec<Neighbour>,
        k: usize,
    ) -> Result<Vec<SearchResult>, VectorError> {
        let mut results = Vec::with_capacity(k.min(neighbours.len()));
        for n in neighbours.into_iter().take(k) {
            let id = n.d_id as i64;
            let metadata = match self.records.get(&encode_id(id))? {
                Some(bytes) => {
                    let record: VectorRecord = from_msgpack(&bytes)?;
                    record.metadata
                }
                None => Value::Nil,
            };
            results.push(SearchResult {
                id,
                distance: n.distance,
                metadata,
            });
        }
        Ok(results)
    }

    // -----------------------------------------------------------------------
    // Persistence
    // -----------------------------------------------------------------------

    /// Flush sled and dump HNSW index to files.
    /// If tombstone ratio > 10%, rebuilds the HNSW index first.
    pub fn flush(&mut self) -> Result<(), VectorError> {
        // Rebuild if tombstone ratio is high
        let total = self.hnsw.get_nb_point();
        if total > 0 && self.tombstones.len() * 10 > total {
            self.rebuild()?;
        }

        self.engine.flush()?;

        // Dump HNSW to files (best-effort; failure is non-fatal since we rebuild on open)
        let _ = self.hnsw.file_dump(&self.data_dir, "vector");
        Ok(())
    }

    /// Rebuild the HNSW index from all live vectors.
    fn rebuild(&mut self) -> Result<(), VectorError> {
        let new_hnsw = HnswIndex::new(&self.config);
        for (&uid, vector) in &self.vectors {
            new_hnsw.insert(vector, uid);
        }
        self.hnsw = new_hnsw;
        self.tombstones.clear();
        Ok(())
    }

    /// Get the collection config.
    pub fn config(&self) -> &CollectionConfig {
        &self.config
    }
}

// ---------------------------------------------------------------------------
// Helpers: raw f32 byte conversion (no msgpack overhead)
// ---------------------------------------------------------------------------

fn f32_vec_to_bytes(v: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for &f in v {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    bytes
}

fn bytes_to_f32_vec(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp_engine() -> (Arc<StorageEngine>, TempDir) {
        let dir = TempDir::new().unwrap();
        let engine = Arc::new(StorageEngine::open(dir.path()).unwrap());
        (engine, dir)
    }

    #[test]
    fn test_insert_and_get() {
        let (engine, dir) = tmp_engine();
        let config = CollectionConfig::new(3);
        let mut ve = VectorEngine::open(engine, config, dir.path().join("hnsw")).unwrap();

        let record = ve.insert(&[1.0, 2.0, 3.0], Value::String("hello".into())).unwrap();
        assert_eq!(record.id, 1);

        let (fetched, vector) = ve.get(1).unwrap();
        assert_eq!(fetched.id, 1);
        assert_eq!(vector, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_dimension_mismatch() {
        let (engine, dir) = tmp_engine();
        let config = CollectionConfig::new(3);
        let mut ve = VectorEngine::open(engine, config, dir.path().join("hnsw")).unwrap();

        let err = ve.insert(&[1.0, 2.0], Value::Nil).unwrap_err();
        assert!(matches!(err, VectorError::DimensionMismatch { expected: 3, got: 2 }));
    }

    #[test]
    fn test_update_metadata() {
        let (engine, dir) = tmp_engine();
        let config = CollectionConfig::new(3);
        let mut ve = VectorEngine::open(engine, config, dir.path().join("hnsw")).unwrap();

        ve.insert(&[1.0, 2.0, 3.0], Value::String("old".into())).unwrap();
        let updated = ve.update_metadata(1, Value::String("new".into())).unwrap();
        assert_eq!(updated.metadata, Value::String("new".into()));

        let (fetched, _) = ve.get(1).unwrap();
        assert_eq!(fetched.metadata, Value::String("new".into()));
    }

    #[test]
    fn test_delete() {
        let (engine, dir) = tmp_engine();
        let config = CollectionConfig::new(3);
        let mut ve = VectorEngine::open(engine, config, dir.path().join("hnsw")).unwrap();

        ve.insert(&[1.0, 2.0, 3.0], Value::Nil).unwrap();
        assert_eq!(ve.count(), 1);

        ve.delete(1).unwrap();
        assert_eq!(ve.count(), 0);
        assert!(ve.get(1).is_err());
    }

    #[test]
    fn test_delete_not_found() {
        let (engine, dir) = tmp_engine();
        let config = CollectionConfig::new(3);
        let mut ve = VectorEngine::open(engine, config, dir.path().join("hnsw")).unwrap();
        assert!(matches!(ve.delete(999), Err(VectorError::VectorNotFound(999))));
    }

    #[test]
    fn test_count_and_all_records() {
        let (engine, dir) = tmp_engine();
        let config = CollectionConfig::new(2);
        let mut ve = VectorEngine::open(engine, config, dir.path().join("hnsw")).unwrap();

        ve.insert(&[1.0, 0.0], Value::Nil).unwrap();
        ve.insert(&[0.0, 1.0], Value::Nil).unwrap();
        ve.insert(&[1.0, 1.0], Value::Nil).unwrap();

        assert_eq!(ve.count(), 3);
        assert_eq!(ve.all_records().unwrap().len(), 3);
    }

    #[test]
    fn test_search_basic() {
        let (engine, dir) = tmp_engine();
        let config = CollectionConfig {
            metric: DistanceMetric::Euclidean,
            ..CollectionConfig::new(2)
        };
        let mut ve = VectorEngine::open(engine, config, dir.path().join("hnsw")).unwrap();

        // Insert known vectors
        ve.insert(&[1.0, 0.0], Value::String("a".into())).unwrap();
        ve.insert(&[0.0, 1.0], Value::String("b".into())).unwrap();
        ve.insert(&[1.0, 1.0], Value::String("c".into())).unwrap();

        // Search near [1.0, 0.0] — should find "a" first
        let results = ve.search(&[1.0, 0.1], 2, 16).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 1); // closest to [1.0, 0.0]
    }

    #[test]
    fn test_search_after_delete() {
        let (engine, dir) = tmp_engine();
        let config = CollectionConfig {
            metric: DistanceMetric::Euclidean,
            ..CollectionConfig::new(2)
        };
        let mut ve = VectorEngine::open(engine, config, dir.path().join("hnsw")).unwrap();

        ve.insert(&[1.0, 0.0], Value::Nil).unwrap(); // id=1
        ve.insert(&[0.0, 1.0], Value::Nil).unwrap(); // id=2
        ve.insert(&[0.5, 0.5], Value::Nil).unwrap(); // id=3

        ve.delete(1).unwrap();

        let results = ve.search(&[1.0, 0.0], 2, 16).unwrap();
        // id=1 should not appear
        for r in &results {
            assert_ne!(r.id, 1);
        }
    }

    #[test]
    fn test_search_filtered() {
        let (engine, dir) = tmp_engine();
        let config = CollectionConfig {
            metric: DistanceMetric::Euclidean,
            ..CollectionConfig::new(2)
        };
        let mut ve = VectorEngine::open(engine, config, dir.path().join("hnsw")).unwrap();

        ve.insert(&[1.0, 0.0], Value::String("cat".into())).unwrap();
        ve.insert(&[0.0, 1.0], Value::String("dog".into())).unwrap();
        ve.insert(&[1.0, 1.0], Value::String("cat".into())).unwrap();

        // Filter: only "cat" metadata
        let results = ve
            .search_filtered(&[1.0, 0.5], 10, 16, |_id, meta| {
                meta.as_str() == Some("cat")
            })
            .unwrap();

        assert!(results.len() <= 2);
        for r in &results {
            assert_eq!(r.metadata, Value::String("cat".into()));
        }
    }

    #[test]
    fn test_search_empty() {
        let (engine, dir) = tmp_engine();
        let config = CollectionConfig::new(3);
        let ve = VectorEngine::open(engine, config, dir.path().join("hnsw")).unwrap();
        let results = ve.search(&[1.0, 2.0, 3.0], 5, 16).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_persistence_reopen() {
        let dir = TempDir::new().unwrap();
        let hnsw_dir = dir.path().join("hnsw");

        // Phase 1: insert and flush
        {
            let engine = Arc::new(StorageEngine::open(&dir.path().join("db")).unwrap());
            let config = CollectionConfig {
                metric: DistanceMetric::Euclidean,
                ..CollectionConfig::new(3)
            };
            let mut ve = VectorEngine::open(engine, config, &hnsw_dir).unwrap();
            ve.insert(&[1.0, 0.0, 0.0], Value::String("first".into())).unwrap();
            ve.insert(&[0.0, 1.0, 0.0], Value::String("second".into())).unwrap();
            ve.flush().unwrap();
        }

        // Phase 2: reopen and verify
        {
            let engine = Arc::new(StorageEngine::open(&dir.path().join("db")).unwrap());
            let config = CollectionConfig {
                metric: DistanceMetric::Euclidean,
                ..CollectionConfig::new(3)
            };
            let ve = VectorEngine::open(engine, config, &hnsw_dir).unwrap();
            assert_eq!(ve.count(), 2);

            let (record, vector) = ve.get(1).unwrap();
            assert_eq!(record.metadata, Value::String("first".into()));
            assert_eq!(vector, vec![1.0, 0.0, 0.0]);

            // Search should work after reopen
            let results = ve.search(&[1.0, 0.0, 0.0], 1, 16).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].id, 1);
        }
    }

    #[test]
    fn test_rebuild_on_flush() {
        let (engine, dir) = tmp_engine();
        let config = CollectionConfig {
            metric: DistanceMetric::Euclidean,
            ..CollectionConfig::new(2)
        };
        let mut ve = VectorEngine::open(engine, config, dir.path().join("hnsw")).unwrap();

        // Insert 10 vectors, delete 5 (50% > 10% threshold)
        for i in 0..10 {
            ve.insert(&[i as f32, 0.0], Value::Nil).unwrap();
        }
        for i in 1..=5 {
            ve.delete(i).unwrap();
        }

        assert_eq!(ve.tombstones.len(), 5);

        // Flush should trigger rebuild
        ve.flush().unwrap();

        assert_eq!(ve.tombstones.len(), 0);
        assert_eq!(ve.count(), 5);
    }

    #[test]
    fn test_f32_byte_roundtrip() {
        let original = vec![1.0f32, -2.5, 3.14, 0.0, f32::MAX];
        let bytes = f32_vec_to_bytes(&original);
        let recovered = bytes_to_f32_vec(&bytes);
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_invalid_dimension_zero() {
        let (engine, dir) = tmp_engine();
        let config = CollectionConfig::new(0);
        match VectorEngine::open(engine, config, dir.path().join("hnsw")) {
            Err(VectorError::InvalidDimension(0)) => {}
            other => panic!("expected InvalidDimension(0), got {:?}", other.err()),
        }
    }

    #[test]
    fn test_config_mismatch_on_reopen() {
        let dir = TempDir::new().unwrap();
        let hnsw_dir = dir.path().join("hnsw");

        {
            let engine = Arc::new(StorageEngine::open(&dir.path().join("db")).unwrap());
            let config = CollectionConfig::new(128);
            let mut ve = VectorEngine::open(engine, config, &hnsw_dir).unwrap();
            ve.insert(&vec![0.0; 128], Value::Nil).unwrap();
            ve.flush().unwrap();
        }

        {
            let engine = Arc::new(StorageEngine::open(&dir.path().join("db")).unwrap());
            let config = CollectionConfig::new(256); // different dimension
            match VectorEngine::open(engine, config, &hnsw_dir) {
                Err(VectorError::DimensionMismatch { .. }) => {}
                other => panic!("expected DimensionMismatch, got {:?}", other.err()),
            }
        }
    }
}

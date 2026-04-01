use std::sync::Arc;

use chrono::Utc;
use rmpv::Value;

use crate::document::Document;
use crate::error::NoSqlError;
use crate::query::Query;
use nodedb_storage::{IdGenerator, StorageEngine, StorageTree};

pub struct Collection {
    name: String,
    tree: StorageTree,
    id_gen: Arc<IdGenerator>,
    #[allow(dead_code)]
    engine: Arc<StorageEngine>,
}

impl Collection {
    pub fn new(name: &str, tree: StorageTree, id_gen: Arc<IdGenerator>, engine: Arc<StorageEngine>) -> Self {
        Collection {
            name: name.to_string(),
            tree,
            id_gen,
            engine,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn put(&self, data: Value) -> Result<Document, NoSqlError> {
        self.put_with_id(0, data)
    }

    pub fn put_with_id(&self, id: i64, data: Value) -> Result<Document, NoSqlError> {
        let id = if id == 0 {
            self.id_gen.next_id(&self.name)?
        } else {
            id
        };

        let doc = Document::new(id, &self.name, data);
        let key = nodedb_storage::encode_id(id);
        let value = rmp_serde::to_vec(&doc)?;
        self.tree.insert(&key, &value)?;
        Ok(doc)
    }

    pub fn put_all(&self, items: Vec<Value>) -> Result<Vec<Document>, NoSqlError> {
        let mut docs = Vec::with_capacity(items.len());
        for data in items {
            docs.push(self.put(data)?);
        }
        Ok(docs)
    }

    pub fn get(&self, id: i64) -> Result<Document, NoSqlError> {
        let key = nodedb_storage::encode_id(id);
        match self.tree.get(&key)? {
            Some(bytes) => {
                let doc: Document = rmp_serde::from_slice(&bytes)?;
                Ok(doc)
            }
            None => Err(NoSqlError::DocumentNotFound(id)),
        }
    }

    pub fn get_all(&self, ids: &[i64]) -> Result<Vec<Document>, NoSqlError> {
        let mut docs = Vec::with_capacity(ids.len());
        for &id in ids {
            match self.get(id) {
                Ok(doc) => docs.push(doc),
                Err(NoSqlError::DocumentNotFound(_)) => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(docs)
    }

    pub fn delete(&self, id: i64) -> Result<bool, NoSqlError> {
        let key = nodedb_storage::encode_id(id);
        let removed = self.tree.remove(&key)?;
        Ok(removed.is_some())
    }

    pub fn delete_all(&self, ids: &[i64]) -> Result<usize, NoSqlError> {
        let mut count = 0;
        for &id in ids {
            if self.delete(id)? {
                count += 1;
            }
        }
        Ok(count)
    }

    pub fn count(&self) -> usize {
        self.tree.len()
    }

    pub fn find_all(&self, offset: Option<usize>, limit: Option<usize>) -> Result<Vec<Document>, NoSqlError> {
        let iter = self.tree.iter();
        let mut docs = Vec::new();

        for result in iter {
            let (_key, value) = result.map_err(NoSqlError::Storage)?;
            let doc: Document = rmp_serde::from_slice(&value)?;
            docs.push(doc);
        }

        if let Some(off) = offset {
            docs = docs.into_iter().skip(off).collect();
        }
        if let Some(lim) = limit {
            docs.truncate(lim);
        }

        Ok(docs)
    }

    pub fn query(&self, query: &Query) -> Result<Vec<Document>, NoSqlError> {
        let all_docs = self.find_all(None, None)?;
        Ok(query.apply(all_docs))
    }

    pub fn clear(&self) -> Result<usize, NoSqlError> {
        let count = self.tree.len();
        self.tree.clear()?;
        Ok(count)
    }

    /// Fast batch insert/update with explicit IDs using atomic sled::Batch.
    /// Uses a single timestamp for all documents. Returns the count of written records.
    pub fn batch_put_with_ids(&self, items: &[(i64, Value)]) -> Result<usize, NoSqlError> {
        let now = Utc::now();
        let mut batch_items = Vec::with_capacity(items.len());
        for (id, data) in items {
            let doc = Document::new_with_timestamp(*id, &self.name, data.clone(), now);
            let key = nodedb_storage::encode_id(*id);
            let value = rmp_serde::to_vec(&doc)?;
            batch_items.push((key, Some(value)));
        }
        self.tree.apply_batch(&batch_items)?;
        Ok(items.len())
    }

    /// Fast batch delete using atomic sled::Batch. Returns the count of items in the batch.
    pub fn batch_delete(&self, ids: &[i64]) -> Result<usize, NoSqlError> {
        let batch_items: Vec<(Vec<u8>, Option<Vec<u8>>)> = ids
            .iter()
            .map(|id| (nodedb_storage::encode_id(*id), None))
            .collect();
        self.tree.apply_batch(&batch_items)?;
        Ok(ids.len())
    }

    pub fn update(&self, id: i64, data: Value) -> Result<Document, NoSqlError> {
        let existing = self.get(id)?;
        let mut doc = Document::new(id, &self.name, data);
        doc.created_at = existing.created_at;
        let key = nodedb_storage::encode_id(id);
        let value = rmp_serde::to_vec(&doc)?;
        self.tree.insert(&key, &value)?;
        Ok(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodedb_storage::StorageEngine;
    use tempfile::TempDir;

    fn setup() -> (Collection, TempDir) {
        let dir = TempDir::new().unwrap();
        let engine = Arc::new(StorageEngine::open(dir.path()).unwrap());
        let tree = engine.open_tree("users").unwrap();
        let id_gen = Arc::new(IdGenerator::new(&engine).unwrap());
        let col = Collection::new("users", tree, id_gen, Arc::clone(&engine));
        (col, dir)
    }

    fn sample_data(name: &str, age: i64) -> Value {
        Value::Map(vec![
            (Value::String("name".into()), Value::String(name.into())),
            (Value::String("age".into()), Value::Integer(age.into())),
        ])
    }

    #[test]
    fn test_put_and_get() {
        let (col, _dir) = setup();
        let doc = col.put(sample_data("Alice", 30)).unwrap();
        assert_eq!(doc.id, 1);

        let fetched = col.get(1).unwrap();
        assert_eq!(fetched.id, 1);
        assert_eq!(
            fetched.get_field("name"),
            Some(&Value::String("Alice".into()))
        );
    }

    #[test]
    fn test_auto_increment() {
        let (col, _dir) = setup();
        let d1 = col.put(sample_data("Alice", 30)).unwrap();
        let d2 = col.put(sample_data("Bob", 25)).unwrap();
        assert_eq!(d1.id, 1);
        assert_eq!(d2.id, 2);
    }

    #[test]
    fn test_delete() {
        let (col, _dir) = setup();
        col.put(sample_data("Alice", 30)).unwrap();
        assert!(col.delete(1).unwrap());
        assert!(col.get(1).is_err());
    }

    #[test]
    fn test_count() {
        let (col, _dir) = setup();
        assert_eq!(col.count(), 0);
        col.put(sample_data("Alice", 30)).unwrap();
        col.put(sample_data("Bob", 25)).unwrap();
        assert_eq!(col.count(), 2);
    }

    #[test]
    fn test_put_all() {
        let (col, _dir) = setup();
        let docs = col
            .put_all(vec![
                sample_data("Alice", 30),
                sample_data("Bob", 25),
                sample_data("Charlie", 35),
            ])
            .unwrap();
        assert_eq!(docs.len(), 3);
        assert_eq!(col.count(), 3);
    }

    #[test]
    fn test_find_all_pagination() {
        let (col, _dir) = setup();
        for i in 0..10 {
            col.put(sample_data(&format!("User{}", i), 20 + i)).unwrap();
        }
        let page = col.find_all(Some(3), Some(5)).unwrap();
        assert_eq!(page.len(), 5);
    }

    #[test]
    fn test_clear() {
        let (col, _dir) = setup();
        col.put(sample_data("Alice", 30)).unwrap();
        col.put(sample_data("Bob", 25)).unwrap();
        let cleared = col.clear().unwrap();
        assert_eq!(cleared, 2);
        assert_eq!(col.count(), 0);
    }

    #[test]
    fn test_get_not_found() {
        let (col, _dir) = setup();
        assert!(matches!(col.get(999), Err(NoSqlError::DocumentNotFound(999))));
    }
}

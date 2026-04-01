use crate::engine::StorageEngine;
use crate::error::StorageError;

pub struct IdGenerator {
    tree: sled::Tree,
}

impl IdGenerator {
    pub fn new(engine: &StorageEngine) -> Result<Self, StorageError> {
        let tree = engine.inner().open_tree("__id_gen__")?;
        Ok(IdGenerator { tree })
    }

    pub fn next_id(&self, namespace: &str) -> Result<i64, StorageError> {
        let key = namespace.as_bytes();

        // update_and_fetch returns the NEW value after the update
        let result = self
            .tree
            .update_and_fetch(key, |old| {
                let current = old
                    .map(|bytes| {
                        let arr: [u8; 8] = bytes.try_into().unwrap_or([0u8; 8]);
                        i64::from_be_bytes(arr)
                    })
                    .unwrap_or(0);
                let next = current + 1;
                Some(next.to_be_bytes().to_vec())
            })
            .map_err(StorageError::from)?;

        match result {
            Some(bytes) => {
                let arr: [u8; 8] = bytes.as_ref().try_into().map_err(|_| {
                    StorageError::Serialization("invalid id bytes".to_string())
                })?;
                Ok(i64::from_be_bytes(arr))
            }
            None => Err(StorageError::Backend("id generation failed".to_string())),
        }
    }

    pub fn current_id(&self, namespace: &str) -> Result<i64, StorageError> {
        let key = namespace.as_bytes();
        match self.tree.get(key)? {
            Some(bytes) => {
                let arr: [u8; 8] = bytes.as_ref().try_into().map_err(|_| {
                    StorageError::Serialization("invalid id bytes".to_string())
                })?;
                Ok(i64::from_be_bytes(arr))
            }
            None => Ok(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp_engine() -> (StorageEngine, TempDir) {
        let dir = TempDir::new().unwrap();
        let engine = StorageEngine::open(dir.path()).unwrap();
        (engine, dir)
    }

    #[test]
    fn test_sequential_ids() {
        let (engine, _dir) = tmp_engine();
        let gen = IdGenerator::new(&engine).unwrap();

        assert_eq!(gen.next_id("users").unwrap(), 1);
        assert_eq!(gen.next_id("users").unwrap(), 2);
        assert_eq!(gen.next_id("users").unwrap(), 3);
    }

    #[test]
    fn test_separate_namespaces() {
        let (engine, _dir) = tmp_engine();
        let gen = IdGenerator::new(&engine).unwrap();

        assert_eq!(gen.next_id("users").unwrap(), 1);
        assert_eq!(gen.next_id("posts").unwrap(), 1);
        assert_eq!(gen.next_id("users").unwrap(), 2);
        assert_eq!(gen.next_id("posts").unwrap(), 2);
    }

    #[test]
    fn test_current_id() {
        let (engine, _dir) = tmp_engine();
        let gen = IdGenerator::new(&engine).unwrap();

        assert_eq!(gen.current_id("users").unwrap(), 0);
        gen.next_id("users").unwrap();
        gen.next_id("users").unwrap();
        assert_eq!(gen.current_id("users").unwrap(), 2);
    }

    #[test]
    fn test_persistence() {
        let dir = TempDir::new().unwrap();

        {
            let engine = StorageEngine::open(dir.path()).unwrap();
            let gen = IdGenerator::new(&engine).unwrap();
            gen.next_id("users").unwrap();
            gen.next_id("users").unwrap();
            engine.flush().unwrap();
        }

        {
            let engine = StorageEngine::open(dir.path()).unwrap();
            let gen = IdGenerator::new(&engine).unwrap();
            assert_eq!(gen.current_id("users").unwrap(), 2);
            assert_eq!(gen.next_id("users").unwrap(), 3);
        }
    }

    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let dir = TempDir::new().unwrap();
        let engine = Arc::new(StorageEngine::open(dir.path()).unwrap());
        let gen = Arc::new(IdGenerator::new(&engine).unwrap());

        let mut handles = vec![];
        for _ in 0..10 {
            let gen = Arc::clone(&gen);
            handles.push(thread::spawn(move || {
                let mut ids = vec![];
                for _ in 0..100 {
                    ids.push(gen.next_id("counter").unwrap());
                }
                ids
            }));
        }

        let mut all_ids: Vec<i64> = vec![];
        for h in handles {
            all_ids.extend(h.join().unwrap());
        }

        all_ids.sort();
        all_ids.dedup();
        assert_eq!(all_ids.len(), 1000); // All unique
        assert_eq!(*all_ids.first().unwrap(), 1);
        assert_eq!(*all_ids.last().unwrap(), 1000);
    }
}

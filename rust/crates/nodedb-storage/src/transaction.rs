use sled::Transactional;

use crate::engine::StorageEngine;
use crate::error::StorageError;

pub struct TransactionContext<'a> {
    engine: &'a StorageEngine,
    operations: Vec<TransactionOp>,
}

enum TransactionOp {
    Insert {
        tree_name: String,
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Remove {
        tree_name: String,
        key: Vec<u8>,
    },
}

fn txn_error_to_string(e: sled::transaction::TransactionError<()>) -> String {
    match e {
        sled::transaction::TransactionError::Abort(()) => "transaction aborted".to_string(),
        sled::transaction::TransactionError::Storage(e) => format!("storage error: {}", e),
    }
}

impl<'a> TransactionContext<'a> {
    pub fn new(engine: &'a StorageEngine) -> Self {
        TransactionContext {
            engine,
            operations: Vec::new(),
        }
    }

    pub fn insert(&mut self, tree_name: &str, key: Vec<u8>, value: Vec<u8>) {
        self.operations.push(TransactionOp::Insert {
            tree_name: tree_name.to_string(),
            key,
            value,
        });
    }

    pub fn remove(&mut self, tree_name: &str, key: Vec<u8>) {
        self.operations.push(TransactionOp::Remove {
            tree_name: tree_name.to_string(),
            key,
        });
    }

    pub fn commit(self) -> Result<(), StorageError> {
        // Collect unique tree names
        let mut tree_names: Vec<String> = Vec::new();
        for op in &self.operations {
            let name = match op {
                TransactionOp::Insert { tree_name, .. } => tree_name,
                TransactionOp::Remove { tree_name, .. } => tree_name,
            };
            if !tree_names.contains(name) {
                tree_names.push(name.clone());
            }
        }

        if tree_names.is_empty() {
            return Ok(());
        }

        let trees: Vec<sled::Tree> = tree_names
            .iter()
            .map(|name| self.engine.inner().open_tree(name))
            .collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)?;

        let ops = &self.operations;

        if trees.len() == 1 {
            trees[0]
                .transaction(|tx_tree| {
                    for op in ops {
                        match op {
                            TransactionOp::Insert { key, value, .. } => {
                                tx_tree.insert(key.as_slice(), value.as_slice())?;
                            }
                            TransactionOp::Remove { key, .. } => {
                                tx_tree.remove(key.as_slice())?;
                            }
                        }
                    }
                    Ok(())
                })
                .map_err(|e| StorageError::Transaction(txn_error_to_string(e)))?;
        } else {
            let tree_slice: Vec<&sled::Tree> = trees.iter().collect();
            tree_slice
                .as_slice()
                .transaction(|tx_trees| {
                    for op in ops {
                        match op {
                            TransactionOp::Insert {
                                tree_name,
                                key,
                                value,
                            } => {
                                let idx = tree_names.iter().position(|n| n == tree_name).unwrap();
                                tx_trees[idx].insert(key.as_slice(), value.as_slice())?;
                            }
                            TransactionOp::Remove { tree_name, key } => {
                                let idx = tree_names.iter().position(|n| n == tree_name).unwrap();
                                tx_trees[idx].remove(key.as_slice())?;
                            }
                        }
                    }
                    Ok(())
                })
                .map_err(|e| StorageError::Transaction(txn_error_to_string(e)))?;
        }

        Ok(())
    }
}

impl StorageEngine {
    pub fn transaction(&self) -> TransactionContext<'_> {
        TransactionContext::new(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::StorageEngine;
    use tempfile::TempDir;

    fn tmp_engine() -> (StorageEngine, TempDir) {
        let dir = TempDir::new().unwrap();
        let engine = StorageEngine::open(dir.path()).unwrap();
        (engine, dir)
    }

    #[test]
    fn test_transaction_commit() {
        let (engine, _dir) = tmp_engine();
        let tree = engine.open_tree("test").unwrap();

        let mut txn = engine.transaction();
        txn.insert("test", b"k1".to_vec(), b"v1".to_vec());
        txn.insert("test", b"k2".to_vec(), b"v2".to_vec());
        txn.commit().unwrap();

        assert_eq!(tree.get(b"k1").unwrap().unwrap(), b"v1");
        assert_eq!(tree.get(b"k2").unwrap().unwrap(), b"v2");
    }

    #[test]
    fn test_transaction_atomicity() {
        let (engine, _dir) = tmp_engine();
        let tree = engine.open_tree("test").unwrap();

        tree.insert(b"existing", b"old").unwrap();

        let mut txn = engine.transaction();
        txn.insert("test", b"new_key".to_vec(), b"new_val".to_vec());
        txn.remove("test", b"existing".to_vec());
        txn.commit().unwrap();

        assert_eq!(tree.get(b"new_key").unwrap().unwrap(), b"new_val");
        assert!(tree.get(b"existing").unwrap().is_none());
    }

    #[test]
    fn test_multi_tree_transaction() {
        let (engine, _dir) = tmp_engine();
        let tree_a = engine.open_tree("a").unwrap();
        let tree_b = engine.open_tree("b").unwrap();

        let mut txn = engine.transaction();
        txn.insert("a", b"k1".to_vec(), b"v1".to_vec());
        txn.insert("b", b"k2".to_vec(), b"v2".to_vec());
        txn.commit().unwrap();

        assert_eq!(tree_a.get(b"k1").unwrap().unwrap(), b"v1");
        assert_eq!(tree_b.get(b"k2").unwrap().unwrap(), b"v2");
    }
}

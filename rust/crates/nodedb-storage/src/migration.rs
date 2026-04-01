use serde::{Deserialize, Serialize};

use crate::engine::StorageEngine;
use crate::error::StorageError;

/// A single migration operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationOp {
    /// Rename a tree (copies all key-value pairs to new tree, drops old tree).
    RenameTree { from: String, to: String },
    /// Drop a tree and all its data.
    DropTree(String),
}

/// Runs schema migrations against a StorageEngine.
pub struct MigrationRunner;

impl MigrationRunner {
    /// Run migrations if the current db_version is less than target_version.
    ///
    /// Reads the current version from the database header (defaults to 0 if no header).
    /// If current >= target, returns Ok without executing any operations.
    /// After executing all operations, updates the db_version in the header.
    pub fn run(
        engine: &StorageEngine,
        target_version: u32,
        ops: Vec<MigrationOp>,
    ) -> Result<(), StorageError> {
        let current_version = match engine.get_db_header()? {
            Some(h) => h.db_version,
            None => 0,
        };

        if current_version >= target_version {
            return Ok(());
        }

        for op in ops {
            match op {
                MigrationOp::RenameTree { from, to } => {
                    Self::rename_tree(engine, &from, &to)?;
                }
                MigrationOp::DropTree(name) => {
                    engine.drop_tree(&name)?;
                }
            }
        }

        // Update version in header
        let mut header = engine.get_db_header()?.unwrap_or_else(|| {
            crate::engine::DbHeader {
                sealed_dek: vec![],
                owner_fingerprint: String::new(),
                db_version: 0,
                database_name: None,
            }
        });
        header.db_version = target_version;
        engine.put_db_header(&header)?;
        engine.flush()?;

        Ok(())
    }

    /// Copy all KVs from source tree to destination tree, then drop source.
    fn rename_tree(engine: &StorageEngine, from: &str, to: &str) -> Result<(), StorageError> {
        let source = engine.open_tree(from)?;
        let dest = engine.open_tree(to)?;

        for result in source.iter() {
            let (key, value) = result?;
            dest.insert(&key, &value)?;
        }

        engine.drop_tree(from)?;
        Ok(())
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
    fn test_rename_tree() {
        let (engine, _dir) = tmp_engine();

        // Create source tree with data
        let tree = engine.open_tree("old_name").unwrap();
        tree.insert(b"k1", b"v1").unwrap();
        tree.insert(b"k2", b"v2").unwrap();
        drop(tree);

        MigrationRunner::run(
            &engine,
            1,
            vec![MigrationOp::RenameTree {
                from: "old_name".to_string(),
                to: "new_name".to_string(),
            }],
        )
        .unwrap();

        // Old tree should be empty/dropped
        let old_tree = engine.open_tree("old_name").unwrap();
        assert!(old_tree.is_empty());

        // New tree should have the data
        let new_tree = engine.open_tree("new_name").unwrap();
        assert_eq!(new_tree.get(b"k1").unwrap().unwrap(), b"v1");
        assert_eq!(new_tree.get(b"k2").unwrap().unwrap(), b"v2");
    }

    #[test]
    fn test_drop_tree() {
        let (engine, _dir) = tmp_engine();

        let tree = engine.open_tree("to_drop").unwrap();
        tree.insert(b"k", b"v").unwrap();
        drop(tree);

        MigrationRunner::run(
            &engine,
            1,
            vec![MigrationOp::DropTree("to_drop".to_string())],
        )
        .unwrap();

        let tree = engine.open_tree("to_drop").unwrap();
        assert!(tree.is_empty());
    }

    #[test]
    fn test_version_skip() {
        let (engine, _dir) = tmp_engine();

        // Run migration to version 3
        MigrationRunner::run(&engine, 3, vec![]).unwrap();

        // Check version is 3
        let header = engine.get_db_header().unwrap().unwrap();
        assert_eq!(header.db_version, 3);

        // Running to version 2 should be a no-op
        let tree = engine.open_tree("should_survive").unwrap();
        tree.insert(b"k", b"v").unwrap();
        drop(tree);

        MigrationRunner::run(
            &engine,
            2,
            vec![MigrationOp::DropTree("should_survive".to_string())],
        )
        .unwrap();

        // Tree should still exist (migration was skipped)
        let tree = engine.open_tree("should_survive").unwrap();
        assert_eq!(tree.get(b"k").unwrap().unwrap(), b"v");
    }

    #[test]
    fn test_sequential_migrations() {
        let (engine, _dir) = tmp_engine();

        // V1: create tree
        let tree = engine.open_tree("users_v1").unwrap();
        tree.insert(b"1", b"alice").unwrap();
        drop(tree);

        // Migrate v0→v1: rename users_v1 to users
        MigrationRunner::run(
            &engine,
            1,
            vec![MigrationOp::RenameTree {
                from: "users_v1".to_string(),
                to: "users".to_string(),
            }],
        )
        .unwrap();

        // Migrate v1→v2: drop a temp tree (no-op if doesn't exist)
        let temp = engine.open_tree("temp_data").unwrap();
        temp.insert(b"t", b"d").unwrap();
        drop(temp);

        MigrationRunner::run(
            &engine,
            2,
            vec![MigrationOp::DropTree("temp_data".to_string())],
        )
        .unwrap();

        // Verify final state
        let header = engine.get_db_header().unwrap().unwrap();
        assert_eq!(header.db_version, 2);

        let users = engine.open_tree("users").unwrap();
        assert_eq!(users.get(b"1").unwrap().unwrap(), b"alice");

        let temp = engine.open_tree("temp_data").unwrap();
        assert!(temp.is_empty());
    }

    #[test]
    fn test_version_persists() {
        let dir = TempDir::new().unwrap();

        {
            let engine = StorageEngine::open(dir.path()).unwrap();
            MigrationRunner::run(&engine, 5, vec![]).unwrap();
        }

        {
            let engine = StorageEngine::open(dir.path()).unwrap();
            let header = engine.get_db_header().unwrap().unwrap();
            assert_eq!(header.db_version, 5);
        }
    }
}

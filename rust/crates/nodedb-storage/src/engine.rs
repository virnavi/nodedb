use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use serde::{Deserialize, Serialize};

use crate::error::StorageError;

/// Database header stored in `__db_header__` tree.
/// Contains sealed DEK, owner fingerprint, schema version, and database name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbHeader {
    pub sealed_dek: Vec<u8>,
    pub owner_fingerprint: String,
    pub db_version: u32,
    #[serde(default)]
    pub database_name: Option<String>,
}

pub struct StorageEngine {
    db: sled::Db,
    dek: Option<[u8; 32]>,
    mismatch: bool,
}

pub struct StorageTree {
    tree: sled::Tree,
    dek: Option<[u8; 32]>,
    mismatch: bool,
}

/// Owner key status for keypair-bound databases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerKeyStatus {
    Verified,
    Mismatch,
    Unbound,
}

/// Validate a database name: lowercase alphanumeric, hyphens, underscores, max 64 chars, not "local".
pub fn validate_database_name(name: &str) -> Result<(), StorageError> {
    if name.is_empty() {
        return Err(StorageError::InvalidDatabaseName("name cannot be empty".into()));
    }
    if name.len() > 64 {
        return Err(StorageError::InvalidDatabaseName("name exceeds 64 characters".into()));
    }
    if name == "local" || name == "mgmt" {
        return Err(StorageError::InvalidDatabaseName(
            format!("'{}' is a reserved name", name),
        ));
    }
    if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_') {
        return Err(StorageError::InvalidDatabaseName(
            "name must contain only lowercase alphanumeric, hyphens, and underscores".into(),
        ));
    }
    Ok(())
}

impl StorageEngine {
    pub fn open(path: &std::path::Path) -> Result<Self, StorageError> {
        let db = sled::open(path)?;
        Ok(StorageEngine { db, dek: None, mismatch: false })
    }

    /// Open a storage engine with a Data Encryption Key for transparent encryption.
    pub fn open_encrypted(path: &std::path::Path, dek: [u8; 32]) -> Result<Self, StorageError> {
        let db = sled::open(path)?;
        Ok(StorageEngine { db, dek: Some(dek), mismatch: false })
    }

    /// Open a storage engine in mismatch mode (all reads return empty, writes are no-ops).
    pub fn open_mismatch(path: &std::path::Path) -> Result<Self, StorageError> {
        let db = sled::open(path)?;
        Ok(StorageEngine { db, dek: None, mismatch: true })
    }

    /// Set the DEK after opening.
    pub fn set_dek(&mut self, dek: [u8; 32]) {
        self.dek = Some(dek);
    }

    /// Get a copy of the DEK (if set).
    pub fn dek(&self) -> Option<[u8; 32]> {
        self.dek
    }

    /// Set mismatch mode.
    pub fn set_mismatch(&mut self, mismatch: bool) {
        self.mismatch = mismatch;
    }

    /// Get the current owner key status.
    pub fn owner_key_status(&self) -> OwnerKeyStatus {
        if self.mismatch {
            OwnerKeyStatus::Mismatch
        } else if self.dek.is_some() {
            OwnerKeyStatus::Verified
        } else {
            OwnerKeyStatus::Unbound
        }
    }

    pub fn open_tree(&self, name: &str) -> Result<StorageTree, StorageError> {
        let tree = self.db.open_tree(name)?;
        Ok(StorageTree { tree, dek: self.dek, mismatch: self.mismatch })
    }

    /// Read raw bytes from the `__db_header__` tree (unencrypted).
    pub fn read_header(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let header_tree = self.db.open_tree("__db_header__")?;
        let val = header_tree.get(key.as_bytes())?;
        Ok(val.map(|v| v.to_vec()))
    }

    /// Write raw bytes to the `__db_header__` tree (unencrypted).
    pub fn write_header(&self, key: &str, value: &[u8]) -> Result<(), StorageError> {
        let header_tree = self.db.open_tree("__db_header__")?;
        header_tree.insert(key.as_bytes(), value)?;
        Ok(())
    }

    /// Read the structured database header.
    pub fn get_db_header(&self) -> Result<Option<DbHeader>, StorageError> {
        match self.read_header("db_header")? {
            Some(bytes) => {
                let header: DbHeader = rmp_serde::from_slice(&bytes)?;
                Ok(Some(header))
            }
            None => Ok(None),
        }
    }

    /// Write the structured database header.
    pub fn put_db_header(&self, header: &DbHeader) -> Result<(), StorageError> {
        let bytes = rmp_serde::to_vec(header)?;
        self.write_header("db_header", &bytes)
    }

    /// Get the database name from the header (if set).
    pub fn database_name(&self) -> Result<Option<String>, StorageError> {
        match self.get_db_header()? {
            Some(h) => Ok(h.database_name),
            None => Ok(None),
        }
    }

    /// Set the database name. Validates the name and writes to header.
    /// Fails if already set to a different name (immutable after first set).
    pub fn set_database_name(&self, name: &str) -> Result<(), StorageError> {
        validate_database_name(name)?;
        let mut header = self.get_db_header()?.unwrap_or(DbHeader {
            sealed_dek: vec![],
            owner_fingerprint: String::new(),
            db_version: 0,
            database_name: None,
        });
        if let Some(ref existing) = header.database_name {
            if existing != name {
                return Err(StorageError::InvalidDatabaseName(format!(
                    "database name already set to '{}', cannot change to '{}'",
                    existing, name
                )));
            }
            return Ok(()); // already set to same name
        }
        header.database_name = Some(name.to_string());
        self.put_db_header(&header)
    }

    /// Drop a named tree from the database.
    pub fn drop_tree(&self, name: &str) -> Result<bool, StorageError> {
        Ok(self.db.drop_tree(name)?)
    }

    /// List all tree names in the database.
    pub fn tree_names(&self) -> Vec<String> {
        self.db.tree_names().iter().filter_map(|name| {
            std::str::from_utf8(name).ok().map(|s| s.to_string())
        }).collect()
    }

    pub fn flush(&self) -> Result<(), StorageError> {
        self.db.flush()?;
        Ok(())
    }

    pub(crate) fn inner(&self) -> &sled::Db {
        &self.db
    }
}

impl StorageTree {
    fn encrypt_value(&self, value: &[u8]) -> Result<Vec<u8>, StorageError> {
        let dek = self.dek.unwrap();
        let cipher = Aes256Gcm::new_from_slice(&dek)
            .map_err(|e| StorageError::Encryption(e.to_string()))?;
        let mut nonce_bytes = [0u8; 12];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, value)
            .map_err(|e| StorageError::Encryption(e.to_string()))?;
        // Pack: [nonce (12) | ciphertext]
        let mut packed = Vec::with_capacity(12 + ciphertext.len());
        packed.extend_from_slice(&nonce_bytes);
        packed.extend_from_slice(&ciphertext);
        Ok(packed)
    }

    fn decrypt_value(&self, packed: &[u8]) -> Result<Vec<u8>, StorageError> {
        if packed.len() < 12 {
            return Err(StorageError::Decryption(
                "encrypted value too short".into(),
            ));
        }
        let dek = self.dek.unwrap();
        let nonce = Nonce::from_slice(&packed[..12]);
        let ciphertext = &packed[12..];
        let cipher = Aes256Gcm::new_from_slice(&dek)
            .map_err(|e| StorageError::Decryption(e.to_string()))?;
        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| StorageError::Decryption(e.to_string()))
    }

    pub fn insert(&self, key: &[u8], value: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        if self.mismatch {
            return Ok(None);
        }
        if self.dek.is_some() {
            let encrypted = self.encrypt_value(value)?;
            let old = self.tree.insert(key, encrypted)?;
            match old {
                Some(v) => Ok(Some(self.decrypt_value(&v)?)),
                None => Ok(None),
            }
        } else {
            let old = self.tree.insert(key, value)?;
            Ok(old.map(|v| v.to_vec()))
        }
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        if self.mismatch {
            return Ok(None);
        }
        let val = self.tree.get(key)?;
        match val {
            Some(v) if self.dek.is_some() => Ok(Some(self.decrypt_value(&v)?)),
            Some(v) => Ok(Some(v.to_vec())),
            None => Ok(None),
        }
    }

    pub fn remove(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        if self.mismatch {
            return Ok(None);
        }
        let old = self.tree.remove(key)?;
        match old {
            Some(v) if self.dek.is_some() => Ok(Some(self.decrypt_value(&v)?)),
            Some(v) => Ok(Some(v.to_vec())),
            None => Ok(None),
        }
    }

    pub fn contains_key(&self, key: &[u8]) -> Result<bool, StorageError> {
        if self.mismatch {
            return Ok(false);
        }
        Ok(self.tree.contains_key(key)?)
    }

    pub fn iter(&self) -> Box<dyn Iterator<Item = Result<(Vec<u8>, Vec<u8>), StorageError>> + '_> {
        if self.mismatch {
            return Box::new(std::iter::empty());
        }
        let has_dek = self.dek.is_some();
        let dek = self.dek;
        Box::new(self.tree.iter().map(move |result| {
            let (k, v) = result.map_err(StorageError::from)?;
            let value = if has_dek {
                decrypt_with_dek(dek.as_ref().unwrap(), &v)?
            } else {
                v.to_vec()
            };
            Ok((k.to_vec(), value))
        }))
    }

    pub fn scan_prefix(
        &self,
        prefix: &[u8],
    ) -> Box<dyn Iterator<Item = Result<(Vec<u8>, Vec<u8>), StorageError>> + '_> {
        if self.mismatch {
            return Box::new(std::iter::empty());
        }
        let has_dek = self.dek.is_some();
        let dek = self.dek;
        Box::new(self.tree.scan_prefix(prefix).map(move |result| {
            let (k, v) = result.map_err(StorageError::from)?;
            let value = if has_dek {
                decrypt_with_dek(dek.as_ref().unwrap(), &v)?
            } else {
                v.to_vec()
            };
            Ok((k.to_vec(), value))
        }))
    }

    pub fn len(&self) -> usize {
        if self.mismatch {
            return 0;
        }
        self.tree.len()
    }

    pub fn is_empty(&self) -> bool {
        if self.mismatch {
            return true;
        }
        self.tree.is_empty()
    }

    /// Remove all entries from the tree atomically. Returns the previous count.
    pub fn clear(&self) -> Result<usize, StorageError> {
        if self.mismatch {
            return Ok(0);
        }
        let count = self.tree.len();
        self.tree.clear()?;
        Ok(count)
    }

    /// Apply a batch of insert/remove operations atomically.
    /// Items: (key, Some(value)) for insert, (key, None) for remove.
    /// When encryption is enabled, values are encrypted before insertion.
    pub fn apply_batch(&self, items: &[(Vec<u8>, Option<Vec<u8>>)]) -> Result<(), StorageError> {
        if self.mismatch {
            return Ok(());
        }
        let mut batch = sled::Batch::default();
        if self.dek.is_some() {
            for (key, value) in items {
                match value {
                    Some(v) => {
                        let encrypted = self.encrypt_value(v)?;
                        batch.insert(key.as_slice(), encrypted);
                    }
                    None => {
                        batch.remove(key.as_slice());
                    }
                }
            }
        } else {
            for (key, value) in items {
                match value {
                    Some(v) => batch.insert(key.as_slice(), v.as_slice()),
                    None => batch.remove(key.as_slice()),
                }
            }
        }
        self.tree.apply_batch(batch)?;
        Ok(())
    }
}

/// Standalone decrypt function for use in closures (avoids borrowing self).
fn decrypt_with_dek(dek: &[u8; 32], packed: &[u8]) -> Result<Vec<u8>, StorageError> {
    if packed.len() < 12 {
        return Err(StorageError::Decryption("encrypted value too short".into()));
    }
    let nonce = Nonce::from_slice(&packed[..12]);
    let ciphertext = &packed[12..];
    let cipher = Aes256Gcm::new_from_slice(dek)
        .map_err(|e| StorageError::Decryption(e.to_string()))?;
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| StorageError::Decryption(e.to_string()))
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

    fn tmp_encrypted_engine() -> (StorageEngine, TempDir) {
        let dir = TempDir::new().unwrap();
        let dek = [42u8; 32];
        let engine = StorageEngine::open_encrypted(dir.path(), dek).unwrap();
        (engine, dir)
    }

    #[test]
    fn test_insert_get_remove() {
        let (engine, _dir) = tmp_engine();
        let tree = engine.open_tree("test").unwrap();

        assert!(tree.get(b"key1").unwrap().is_none());

        tree.insert(b"key1", b"value1").unwrap();
        assert_eq!(tree.get(b"key1").unwrap().unwrap(), b"value1");

        tree.remove(b"key1").unwrap();
        assert!(tree.get(b"key1").unwrap().is_none());
    }

    #[test]
    fn test_contains_key() {
        let (engine, _dir) = tmp_engine();
        let tree = engine.open_tree("test").unwrap();

        assert!(!tree.contains_key(b"key").unwrap());
        tree.insert(b"key", b"val").unwrap();
        assert!(tree.contains_key(b"key").unwrap());
    }

    #[test]
    fn test_len_is_empty() {
        let (engine, _dir) = tmp_engine();
        let tree = engine.open_tree("test").unwrap();

        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);

        tree.insert(b"a", b"1").unwrap();
        tree.insert(b"b", b"2").unwrap();
        assert_eq!(tree.len(), 2);
        assert!(!tree.is_empty());
    }

    #[test]
    fn test_iter() {
        let (engine, _dir) = tmp_engine();
        let tree = engine.open_tree("test").unwrap();

        tree.insert(b"a", b"1").unwrap();
        tree.insert(b"b", b"2").unwrap();
        tree.insert(b"c", b"3").unwrap();

        let items: Vec<_> = tree.iter().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].0, b"a");
        assert_eq!(items[2].0, b"c");
    }

    #[test]
    fn test_scan_prefix() {
        let (engine, _dir) = tmp_engine();
        let tree = engine.open_tree("test").unwrap();

        tree.insert(b"user:1", b"alice").unwrap();
        tree.insert(b"user:2", b"bob").unwrap();
        tree.insert(b"post:1", b"hello").unwrap();

        let users: Vec<_> = tree.scan_prefix(b"user:").collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(users.len(), 2);
    }

    // --- Encryption tests ---

    #[test]
    fn test_encrypted_insert_get_roundtrip() {
        let (engine, _dir) = tmp_encrypted_engine();
        let tree = engine.open_tree("test").unwrap();

        tree.insert(b"key1", b"secret value").unwrap();
        let val = tree.get(b"key1").unwrap().unwrap();
        assert_eq!(val, b"secret value");
    }

    #[test]
    fn test_encrypted_remove_returns_plaintext() {
        let (engine, _dir) = tmp_encrypted_engine();
        let tree = engine.open_tree("test").unwrap();

        tree.insert(b"key1", b"hello").unwrap();
        let old = tree.remove(b"key1").unwrap().unwrap();
        assert_eq!(old, b"hello");
        assert!(tree.get(b"key1").unwrap().is_none());
    }

    #[test]
    fn test_encrypted_iter() {
        let (engine, _dir) = tmp_encrypted_engine();
        let tree = engine.open_tree("test").unwrap();

        tree.insert(b"a", b"alpha").unwrap();
        tree.insert(b"b", b"beta").unwrap();

        let items: Vec<_> = tree.iter().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].1, b"alpha");
        assert_eq!(items[1].1, b"beta");
    }

    #[test]
    fn test_encrypted_scan_prefix() {
        let (engine, _dir) = tmp_encrypted_engine();
        let tree = engine.open_tree("test").unwrap();

        tree.insert(b"ns:1", b"one").unwrap();
        tree.insert(b"ns:2", b"two").unwrap();
        tree.insert(b"other:1", b"three").unwrap();

        let items: Vec<_> = tree.scan_prefix(b"ns:").collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].1, b"one");
        assert_eq!(items[1].1, b"two");
    }

    #[test]
    fn test_encrypted_data_not_readable_without_dek() {
        let dir = TempDir::new().unwrap();
        let dek = [42u8; 32];

        // Write encrypted
        {
            let engine = StorageEngine::open_encrypted(dir.path(), dek).unwrap();
            let tree = engine.open_tree("test").unwrap();
            tree.insert(b"key", b"secret").unwrap();
            engine.flush().unwrap();
        }

        // Read without DEK — raw bytes are not "secret"
        {
            let engine = StorageEngine::open(dir.path()).unwrap();
            let tree = engine.open_tree("test").unwrap();
            let raw = tree.get(b"key").unwrap().unwrap();
            assert_ne!(raw, b"secret");
        }

        // Read with correct DEK — value is "secret"
        {
            let engine = StorageEngine::open_encrypted(dir.path(), dek).unwrap();
            let tree = engine.open_tree("test").unwrap();
            let val = tree.get(b"key").unwrap().unwrap();
            assert_eq!(val, b"secret");
        }
    }

    #[test]
    fn test_wrong_dek_fails_decrypt() {
        let dir = TempDir::new().unwrap();

        // Write with one DEK
        {
            let engine = StorageEngine::open_encrypted(dir.path(), [1u8; 32]).unwrap();
            let tree = engine.open_tree("test").unwrap();
            tree.insert(b"key", b"data").unwrap();
            engine.flush().unwrap();
        }

        // Read with different DEK — decryption should fail
        {
            let engine = StorageEngine::open_encrypted(dir.path(), [2u8; 32]).unwrap();
            let tree = engine.open_tree("test").unwrap();
            let result = tree.get(b"key");
            assert!(result.is_err());
        }
    }

    // --- Mismatch mode tests ---

    #[test]
    fn test_mismatch_reads_empty() {
        let (mut engine, _dir) = tmp_engine();
        // Write some data first
        let tree = engine.open_tree("test").unwrap();
        tree.insert(b"key", b"value").unwrap();
        drop(tree);

        // Switch to mismatch mode
        engine.set_mismatch(true);
        let tree = engine.open_tree("test").unwrap();

        assert!(tree.get(b"key").unwrap().is_none());
        assert!(!tree.contains_key(b"key").unwrap());
        assert_eq!(tree.len(), 0);
        assert!(tree.is_empty());
        assert_eq!(tree.iter().count(), 0);
        assert_eq!(tree.scan_prefix(b"").count(), 0);
    }

    #[test]
    fn test_mismatch_writes_silent_noop() {
        let dir = TempDir::new().unwrap();
        let engine = StorageEngine::open_mismatch(dir.path()).unwrap();
        let tree = engine.open_tree("test").unwrap();

        // Writes succeed but are no-ops
        assert!(tree.insert(b"key", b"val").unwrap().is_none());
        assert!(tree.get(b"key").unwrap().is_none());
    }

    #[test]
    fn test_owner_key_status() {
        let dir = TempDir::new().unwrap();
        let engine = StorageEngine::open(dir.path()).unwrap();
        assert_eq!(engine.owner_key_status(), OwnerKeyStatus::Unbound);

        let dir2 = TempDir::new().unwrap();
        let engine2 = StorageEngine::open_encrypted(dir2.path(), [0u8; 32]).unwrap();
        assert_eq!(engine2.owner_key_status(), OwnerKeyStatus::Verified);

        let dir3 = TempDir::new().unwrap();
        let engine3 = StorageEngine::open_mismatch(dir3.path()).unwrap();
        assert_eq!(engine3.owner_key_status(), OwnerKeyStatus::Mismatch);
    }

    // --- Header tests ---

    #[test]
    fn test_header_read_write() {
        let (engine, _dir) = tmp_engine();

        assert!(engine.read_header("version").unwrap().is_none());

        engine.write_header("version", b"1").unwrap();
        let val = engine.read_header("version").unwrap().unwrap();
        assert_eq!(val, b"1");
    }

    #[test]
    fn test_drop_tree() {
        let (engine, _dir) = tmp_engine();
        let tree = engine.open_tree("to_drop").unwrap();
        tree.insert(b"k", b"v").unwrap();
        drop(tree);

        assert!(engine.drop_tree("to_drop").unwrap());

        // After drop, opening the tree gives an empty tree
        let tree = engine.open_tree("to_drop").unwrap();
        assert!(tree.is_empty());
    }

    // --- DbHeader tests ---

    #[test]
    fn test_db_header_roundtrip() {
        let (engine, _dir) = tmp_engine();

        assert!(engine.get_db_header().unwrap().is_none());

        let header = DbHeader {
            sealed_dek: vec![1, 2, 3, 4],
            owner_fingerprint: "abc123".to_string(),
            db_version: 5,
            database_name: None,
        };
        engine.put_db_header(&header).unwrap();

        let loaded = engine.get_db_header().unwrap().unwrap();
        assert_eq!(loaded.sealed_dek, vec![1, 2, 3, 4]);
        assert_eq!(loaded.owner_fingerprint, "abc123");
        assert_eq!(loaded.db_version, 5);
        assert!(loaded.database_name.is_none());
    }

    #[test]
    fn test_db_header_persists() {
        let dir = TempDir::new().unwrap();

        {
            let engine = StorageEngine::open(dir.path()).unwrap();
            let header = DbHeader {
                sealed_dek: vec![10, 20],
                owner_fingerprint: "fp".to_string(),
                db_version: 3,
                database_name: None,
            };
            engine.put_db_header(&header).unwrap();
            engine.flush().unwrap();
        }

        {
            let engine = StorageEngine::open(dir.path()).unwrap();
            let header = engine.get_db_header().unwrap().unwrap();
            assert_eq!(header.db_version, 3);
            assert_eq!(header.owner_fingerprint, "fp");
        }
    }

    #[test]
    fn test_tree_names() {
        let (engine, _dir) = tmp_engine();
        engine.open_tree("alpha").unwrap();
        engine.open_tree("beta").unwrap();

        let names = engine.tree_names();
        assert!(names.contains(&"alpha".to_string()));
        assert!(names.contains(&"beta".to_string()));
    }

    // --- Database name tests ---

    #[test]
    fn test_validate_database_name_valid() {
        assert!(validate_database_name("warehouse").is_ok());
        assert!(validate_database_name("my-db").is_ok());
        assert!(validate_database_name("db_01").is_ok());
        assert!(validate_database_name("a").is_ok());
        assert!(validate_database_name(&"x".repeat(64)).is_ok());
    }

    #[test]
    fn test_validate_database_name_invalid() {
        assert!(validate_database_name("").is_err());
        assert!(validate_database_name("local").is_err());
        assert!(validate_database_name("mgmt").is_err());
        assert!(validate_database_name("MyDB").is_err());
        assert!(validate_database_name("db name").is_err());
        assert!(validate_database_name("db.name").is_err());
        assert!(validate_database_name(&"x".repeat(65)).is_err());
    }

    #[test]
    fn test_set_and_get_database_name() {
        let (engine, _dir) = tmp_engine();
        assert!(engine.database_name().unwrap().is_none());

        engine.set_database_name("warehouse").unwrap();
        assert_eq!(engine.database_name().unwrap().unwrap(), "warehouse");
    }

    #[test]
    fn test_database_name_immutable() {
        let (engine, _dir) = tmp_engine();
        engine.set_database_name("warehouse").unwrap();

        // Same name is fine (idempotent)
        engine.set_database_name("warehouse").unwrap();

        // Different name fails
        let err = engine.set_database_name("other-db").unwrap_err();
        assert!(err.to_string().contains("already set"));
    }

    #[test]
    fn test_database_name_persists() {
        let dir = TempDir::new().unwrap();
        {
            let engine = StorageEngine::open(dir.path()).unwrap();
            engine.set_database_name("my-db").unwrap();
            engine.flush().unwrap();
        }
        {
            let engine = StorageEngine::open(dir.path()).unwrap();
            assert_eq!(engine.database_name().unwrap().unwrap(), "my-db");
        }
    }

    #[test]
    fn test_dek_accessor_none() {
        let (engine, _dir) = tmp_engine();
        assert!(engine.dek().is_none());
    }

    #[test]
    fn test_dek_accessor_some() {
        let (engine, _dir) = tmp_encrypted_engine();
        let dek = engine.dek().unwrap();
        assert_eq!(dek, [42u8; 32]);
    }

    #[test]
    fn test_db_header_backward_compat_no_database_name() {
        // Simulate old header without database_name field
        #[derive(Serialize)]
        struct OldDbHeader {
            sealed_dek: Vec<u8>,
            owner_fingerprint: String,
            db_version: u32,
        }

        let (engine, _dir) = tmp_engine();
        let old = OldDbHeader {
            sealed_dek: vec![1, 2],
            owner_fingerprint: "fp".to_string(),
            db_version: 1,
        };
        let bytes = rmp_serde::to_vec(&old).unwrap();
        engine.write_header("db_header", &bytes).unwrap();

        let header = engine.get_db_header().unwrap().unwrap();
        assert_eq!(header.db_version, 1);
        assert!(header.database_name.is_none());
    }
}

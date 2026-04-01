use nodedb_storage::*;
use tempfile::TempDir;

#[test]
fn test_open_close_reopen() {
    let dir = TempDir::new().unwrap();

    {
        let engine = StorageEngine::open(dir.path()).unwrap();
        let tree = engine.open_tree("data").unwrap();
        tree.insert(b"key", b"value").unwrap();
        engine.flush().unwrap();
    }

    {
        let engine = StorageEngine::open(dir.path()).unwrap();
        let tree = engine.open_tree("data").unwrap();
        assert_eq!(tree.get(b"key").unwrap().unwrap(), b"value");
    }
}

#[test]
fn test_crud_lifecycle() {
    let dir = TempDir::new().unwrap();
    let engine = StorageEngine::open(dir.path()).unwrap();
    let tree = engine.open_tree("crud").unwrap();

    // Create
    tree.insert(b"user:1", b"Alice").unwrap();
    tree.insert(b"user:2", b"Bob").unwrap();
    assert_eq!(tree.len(), 2);

    // Read
    assert_eq!(tree.get(b"user:1").unwrap().unwrap(), b"Alice");
    assert!(tree.contains_key(b"user:2").unwrap());
    assert!(!tree.contains_key(b"user:3").unwrap());

    // Update
    tree.insert(b"user:1", b"Alice Updated").unwrap();
    assert_eq!(tree.get(b"user:1").unwrap().unwrap(), b"Alice Updated");

    // Delete
    let old = tree.remove(b"user:1").unwrap();
    assert_eq!(old.unwrap(), b"Alice Updated");
    assert!(tree.get(b"user:1").unwrap().is_none());
    assert_eq!(tree.len(), 1);
}

#[test]
fn test_transactions_atomic_commit() {
    let dir = TempDir::new().unwrap();
    let engine = StorageEngine::open(dir.path()).unwrap();
    let tree = engine.open_tree("txn_test").unwrap();

    let mut txn = engine.transaction();
    txn.insert("txn_test", b"a".to_vec(), b"1".to_vec());
    txn.insert("txn_test", b"b".to_vec(), b"2".to_vec());
    txn.insert("txn_test", b"c".to_vec(), b"3".to_vec());
    txn.commit().unwrap();

    assert_eq!(tree.get(b"a").unwrap().unwrap(), b"1");
    assert_eq!(tree.get(b"b").unwrap().unwrap(), b"2");
    assert_eq!(tree.get(b"c").unwrap().unwrap(), b"3");
}

#[test]
fn test_id_generation_across_namespaces() {
    let dir = TempDir::new().unwrap();
    let engine = StorageEngine::open(dir.path()).unwrap();
    let gen = IdGenerator::new(&engine).unwrap();

    // Generate IDs in different namespaces
    assert_eq!(gen.next_id("users").unwrap(), 1);
    assert_eq!(gen.next_id("posts").unwrap(), 1);
    assert_eq!(gen.next_id("users").unwrap(), 2);
    assert_eq!(gen.next_id("users").unwrap(), 3);
    assert_eq!(gen.next_id("posts").unwrap(), 2);

    // Verify current IDs
    assert_eq!(gen.current_id("users").unwrap(), 3);
    assert_eq!(gen.current_id("posts").unwrap(), 2);
    assert_eq!(gen.current_id("nonexistent").unwrap(), 0);
}

#[test]
fn test_prefix_scan() {
    let dir = TempDir::new().unwrap();
    let engine = StorageEngine::open(dir.path()).unwrap();
    let tree = engine.open_tree("prefixed").unwrap();

    tree.insert(b"user:001", b"Alice").unwrap();
    tree.insert(b"user:002", b"Bob").unwrap();
    tree.insert(b"user:003", b"Charlie").unwrap();
    tree.insert(b"post:001", b"Hello").unwrap();
    tree.insert(b"post:002", b"World").unwrap();

    let users: Vec<_> = tree
        .scan_prefix(b"user:")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(users.len(), 3);

    let posts: Vec<_> = tree
        .scan_prefix(b"post:")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(posts.len(), 2);
}

#[test]
fn test_serialization_roundtrips() {
    // Primitives
    let s = "hello".to_string();
    let bytes = to_msgpack(&s).unwrap();
    let decoded: String = from_msgpack(&bytes).unwrap();
    assert_eq!(s, decoded);

    // Numbers
    let n: i64 = 42;
    let bytes = to_msgpack(&n).unwrap();
    let decoded: i64 = from_msgpack(&bytes).unwrap();
    assert_eq!(n, decoded);

    // ID encoding
    for id in [1i64, 100, 1000, i64::MAX] {
        let encoded = encode_id(id);
        let decoded = decode_id(&encoded).unwrap();
        assert_eq!(id, decoded);
    }
}

#[test]
fn test_multiple_trees() {
    let dir = TempDir::new().unwrap();
    let engine = StorageEngine::open(dir.path()).unwrap();

    let tree_a = engine.open_tree("alpha").unwrap();
    let tree_b = engine.open_tree("beta").unwrap();

    tree_a.insert(b"key", b"from_alpha").unwrap();
    tree_b.insert(b"key", b"from_beta").unwrap();

    assert_eq!(tree_a.get(b"key").unwrap().unwrap(), b"from_alpha");
    assert_eq!(tree_b.get(b"key").unwrap().unwrap(), b"from_beta");
}

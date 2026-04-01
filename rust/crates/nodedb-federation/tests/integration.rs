use std::sync::Arc;

use nodedb_federation::{FederationEngine, FederationError, PeerStatus};
use nodedb_storage::StorageEngine;
use rmpv::Value;
use tempfile::TempDir;

fn open_engine() -> (TempDir, FederationEngine) {
    let dir = TempDir::new().unwrap();
    let storage = Arc::new(StorageEngine::open(&dir.path().join("db")).unwrap());
    let engine = FederationEngine::new(storage).unwrap();
    (dir, engine)
}

#[test]
fn full_workflow() {
    let (_dir, eng) = open_engine();

    // Add peers
    let alice = eng.add_peer("alice", Some("ws://localhost:8080".into()), None, PeerStatus::Active, Value::Nil).unwrap();
    let bob = eng.add_peer("bob", None, None, PeerStatus::Inactive, Value::Nil).unwrap();
    let carol = eng.add_peer("carol", None, None, PeerStatus::Active, Value::Nil).unwrap();
    assert_eq!(eng.peer_count(), 3);

    // Get by name
    let fetched = eng.get_peer_by_name("alice").unwrap();
    assert_eq!(fetched.id, alice.id);
    assert_eq!(fetched.endpoint, Some("ws://localhost:8080".to_string()));

    // Update peer
    let updated = eng.update_peer(bob.id, None, None, Some(PeerStatus::Banned), None).unwrap();
    assert_eq!(updated.status, PeerStatus::Banned);

    // Add groups
    let admins = eng.add_group("admins", Value::Nil).unwrap();
    let editors = eng.add_group("editors", Value::Nil).unwrap();
    assert_eq!(eng.group_count(), 2);

    // Add members
    eng.add_member(admins.id, alice.id).unwrap();
    eng.add_member(admins.id, bob.id).unwrap();
    eng.add_member(editors.id, alice.id).unwrap();
    eng.add_member(editors.id, carol.id).unwrap();

    // Check memberships
    let admin_group = eng.get_group(admins.id).unwrap();
    assert_eq!(admin_group.members, vec![alice.id, bob.id]);

    let mut alice_groups = eng.groups_for_peer(alice.id).unwrap();
    alice_groups.sort();
    assert_eq!(alice_groups, vec![admins.id, editors.id]);

    // Remove member
    eng.remove_member(editors.id, alice.id).unwrap();
    let editor_group = eng.get_group(editors.id).unwrap();
    assert_eq!(editor_group.members, vec![carol.id]);

    // All peers/groups
    assert_eq!(eng.all_peers().unwrap().len(), 3);
    assert_eq!(eng.all_groups().unwrap().len(), 2);
}

#[test]
fn persistence_close_reopen() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("db");

    // First session
    {
        let storage = Arc::new(StorageEngine::open(&db_path).unwrap());
        let eng = FederationEngine::new(storage).unwrap();

        eng.add_peer("alice", None, None, PeerStatus::Active, Value::Nil).unwrap();
        eng.add_peer("bob", None, None, PeerStatus::Inactive, Value::Nil).unwrap();

        let group = eng.add_group("team", Value::Nil).unwrap();
        eng.add_member(group.id, 1).unwrap();
        eng.add_member(group.id, 2).unwrap();

        eng.flush().unwrap();
    }

    // Second session — data survives
    {
        let storage = Arc::new(StorageEngine::open(&db_path).unwrap());
        let eng = FederationEngine::new(storage).unwrap();

        assert_eq!(eng.peer_count(), 2);
        assert_eq!(eng.group_count(), 1);

        let alice = eng.get_peer_by_name("alice").unwrap();
        assert_eq!(alice.status, PeerStatus::Active);

        let team = eng.get_group_by_name("team").unwrap();
        assert_eq!(team.members.len(), 2);

        let gids = eng.groups_for_peer(alice.id).unwrap();
        assert_eq!(gids.len(), 1);
    }
}

#[test]
fn delete_peer_cascade_removes_from_groups() {
    let (_dir, eng) = open_engine();

    let alice = eng.add_peer("alice", None, None, PeerStatus::Active, Value::Nil).unwrap();
    let bob = eng.add_peer("bob", None, None, PeerStatus::Active, Value::Nil).unwrap();

    let g1 = eng.add_group("admins", Value::Nil).unwrap();
    let g2 = eng.add_group("editors", Value::Nil).unwrap();

    eng.add_member(g1.id, alice.id).unwrap();
    eng.add_member(g1.id, bob.id).unwrap();
    eng.add_member(g2.id, alice.id).unwrap();

    // Delete alice — should be removed from both groups
    eng.delete_peer(alice.id).unwrap();
    assert_eq!(eng.peer_count(), 1);

    let admins = eng.get_group(g1.id).unwrap();
    assert_eq!(admins.members, vec![bob.id]);

    let editors = eng.get_group(g2.id).unwrap();
    assert!(editors.members.is_empty());

    // Alice's peer_groups index should be empty
    let gids = eng.groups_for_peer(alice.id).unwrap();
    assert!(gids.is_empty());
}

#[test]
fn delete_group_cleans_all_indexes() {
    let (_dir, eng) = open_engine();

    let alice = eng.add_peer("alice", None, None, PeerStatus::Active, Value::Nil).unwrap();
    let group = eng.add_group("team", Value::Nil).unwrap();
    eng.add_member(group.id, alice.id).unwrap();

    eng.delete_group(group.id).unwrap();
    assert_eq!(eng.group_count(), 0);

    // Group name freed
    match eng.get_group_by_name("team") {
        Err(FederationError::GroupNotFoundByName(_)) => {}
        other => panic!("expected GroupNotFoundByName, got {:?}", other),
    }

    // Alice no longer in any group
    let gids = eng.groups_for_peer(alice.id).unwrap();
    assert!(gids.is_empty());
}

#[test]
fn error_cases() {
    let (_dir, eng) = open_engine();

    // Peer not found
    match eng.get_peer(999) {
        Err(FederationError::PeerNotFound(999)) => {}
        other => panic!("expected PeerNotFound, got {:?}", other),
    }

    // Peer not found by name
    match eng.get_peer_by_name("nobody") {
        Err(FederationError::PeerNotFoundByName(_)) => {}
        other => panic!("expected PeerNotFoundByName, got {:?}", other),
    }

    // Duplicate peer name
    eng.add_peer("alice", None, None, PeerStatus::Active, Value::Nil).unwrap();
    match eng.add_peer("alice", None, None, PeerStatus::Active, Value::Nil) {
        Err(FederationError::DuplicatePeerName(_)) => {}
        other => panic!("expected DuplicatePeerName, got {:?}", other),
    }

    // Duplicate group name
    eng.add_group("team", Value::Nil).unwrap();
    match eng.add_group("team", Value::Nil) {
        Err(FederationError::DuplicateGroupName(_)) => {}
        other => panic!("expected DuplicateGroupName, got {:?}", other),
    }

    // Invalid member peer
    let group = eng.get_group_by_name("team").unwrap();
    match eng.add_member(group.id, 999) {
        Err(FederationError::InvalidMemberPeer(999)) => {}
        other => panic!("expected InvalidMemberPeer, got {:?}", other),
    }

    // Group not found
    match eng.get_group(999) {
        Err(FederationError::GroupNotFound(999)) => {}
        other => panic!("expected GroupNotFound, got {:?}", other),
    }
}

#[test]
fn metadata_roundtrip() {
    let (_dir, eng) = open_engine();

    let meta = Value::Map(vec![
        (Value::String("role".into()), Value::String("admin".into())),
        (Value::String("level".into()), Value::Integer(5.into())),
    ]);

    let peer = eng.add_peer("alice", None, None, PeerStatus::Active, meta.clone()).unwrap();
    let fetched = eng.get_peer(peer.id).unwrap();
    assert_eq!(fetched.metadata, meta);

    let group_meta = Value::Map(vec![
        (Value::String("dept".into()), Value::String("engineering".into())),
    ]);
    let group = eng.add_group("eng", group_meta.clone()).unwrap();
    let fetched = eng.get_group(group.id).unwrap();
    assert_eq!(fetched.metadata, group_meta);
}

use std::sync::Arc;

use chrono::{Duration, Utc};
use nodedb_dac::{DacEngine, DacError, AccessSubjectType, AccessPermission, DacSubject};
use nodedb_storage::StorageEngine;
use rmpv::Value;
use tempfile::TempDir;

fn open_engine() -> (TempDir, DacEngine) {
    let dir = TempDir::new().unwrap();
    let storage = Arc::new(StorageEngine::open(&dir.path().join("db")).unwrap());
    let engine = DacEngine::new(storage).unwrap();
    (dir, engine)
}

fn make_doc() -> Value {
    Value::Map(vec![
        (Value::String("name".into()), Value::String("Alice".into())),
        (Value::String("email".into()), Value::String("alice@example.com".into())),
        (Value::String("age".into()), Value::Integer(30.into())),
        (Value::String("phone".into()), Value::String("+1234567890".into())),
    ])
}

fn peer_subject(name: &str) -> DacSubject {
    DacSubject { peer_id: name.to_string(), group_ids: vec![] }
}

fn group_subject(peer: &str, groups: &[&str]) -> DacSubject {
    DacSubject {
        peer_id: peer.to_string(),
        group_ids: groups.iter().map(|g| g.to_string()).collect(),
    }
}

#[test]
fn full_pipeline() {
    let (_dir, eng) = open_engine();

    // Allow collection-level access for "admins" group
    eng.add_rule(
        "users", None, None,
        AccessSubjectType::Group, "admins",
        AccessPermission::Allow, None, None,
    ).unwrap();

    // Redact email field for "admins"
    eng.add_rule(
        "users", Some("email".into()), None,
        AccessSubjectType::Group, "admins",
        AccessPermission::Redact, None, None,
    ).unwrap();

    // Deny phone field for "admins"
    eng.add_rule(
        "users", Some("phone".into()), None,
        AccessSubjectType::Group, "admins",
        AccessPermission::Deny, None, None,
    ).unwrap();

    let subject = group_subject("alice", &["admins"]);
    let doc = make_doc();
    let result = eng.filter_document("users", &doc, &subject, None).unwrap();
    let map = result.as_map().unwrap();

    // name and age should be allowed
    assert!(map.iter().any(|(k, v)| k.as_str() == Some("name") && v.as_str() == Some("Alice")));
    assert!(map.iter().any(|(k, v)| k.as_str() == Some("age") && v.as_i64() == Some(30)));

    // email should be redacted (Nil)
    let email = map.iter().find(|(k, _)| k.as_str() == Some("email")).unwrap();
    assert_eq!(email.1, Value::Nil);

    // phone should be denied (absent)
    assert!(!map.iter().any(|(k, _)| k.as_str() == Some("phone")));
}

#[test]
fn no_rules_denies_everything() {
    let (_dir, eng) = open_engine();
    let subject = peer_subject("alice");
    let doc = make_doc();
    let result = eng.filter_document("users", &doc, &subject, None).unwrap();
    assert!(result.as_map().unwrap().is_empty());
}

#[test]
fn rule_precedence_field_over_collection() {
    let (_dir, eng) = open_engine();

    // Allow everything at collection level
    eng.add_rule("users", None, None, AccessSubjectType::Peer, "bob", AccessPermission::Allow, None, None).unwrap();

    // Deny email specifically
    eng.add_rule("users", Some("email".into()), None, AccessSubjectType::Peer, "bob", AccessPermission::Deny, None, None).unwrap();

    let subject = peer_subject("bob");
    let doc = make_doc();
    let result = eng.filter_document("users", &doc, &subject, None).unwrap();
    let map = result.as_map().unwrap();

    // name, age, phone allowed; email denied
    assert_eq!(map.len(), 3);
    assert!(!map.iter().any(|(k, _)| k.as_str() == Some("email")));
}

#[test]
fn expired_rule_ignored() {
    let (_dir, eng) = open_engine();

    // Allow rule that expired an hour ago
    eng.add_rule(
        "users", None, None,
        AccessSubjectType::Peer, "alice",
        AccessPermission::Allow,
        Some(Utc::now() - Duration::hours(1)),
        None,
    ).unwrap();

    let subject = peer_subject("alice");
    let doc = make_doc();
    let result = eng.filter_document("users", &doc, &subject, None).unwrap();
    // Expired allow → default deny → empty
    assert!(result.as_map().unwrap().is_empty());
}

#[test]
fn persistence_close_reopen() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("db");

    // First session
    {
        let storage = Arc::new(StorageEngine::open(&db_path).unwrap());
        let eng = DacEngine::new(storage).unwrap();
        eng.add_rule("users", None, None, AccessSubjectType::Peer, "alice", AccessPermission::Allow, None, None).unwrap();
        eng.add_rule("users", Some("email".into()), None, AccessSubjectType::Peer, "alice", AccessPermission::Redact, None, None).unwrap();
        eng.flush().unwrap();
    }

    // Second session
    {
        let storage = Arc::new(StorageEngine::open(&db_path).unwrap());
        let eng = DacEngine::new(storage).unwrap();
        assert_eq!(eng.rule_count(), 2);

        let subject = peer_subject("alice");
        let doc = make_doc();
        let result = eng.filter_document("users", &doc, &subject, None).unwrap();
        let map = result.as_map().unwrap();

        // email should still be redacted
        let email = map.iter().find(|(k, _)| k.as_str() == Some("email")).unwrap();
        assert_eq!(email.1, Value::Nil);
    }
}

#[test]
fn rule_crud_operations() {
    let (_dir, eng) = open_engine();

    let r1 = eng.add_rule("users", None, None, AccessSubjectType::Peer, "alice", AccessPermission::Allow, None, None).unwrap();
    let r2 = eng.add_rule("posts", None, None, AccessSubjectType::Group, "editors", AccessPermission::Deny, None, None).unwrap();
    assert_eq!(eng.rule_count(), 2);

    // Get rule
    let fetched = eng.get_rule(r1.id).unwrap();
    assert_eq!(fetched.collection, "users");

    // Update rule
    let updated = eng.update_rule(r2.id, Some(AccessPermission::Allow), None).unwrap();
    assert_eq!(updated.permission, AccessPermission::Allow);

    // Rules for collection
    let user_rules = eng.rules_for_collection("users").unwrap();
    assert_eq!(user_rules.len(), 1);

    // Delete rule
    eng.delete_rule(r1.id).unwrap();
    assert_eq!(eng.rule_count(), 1);

    // Not found
    match eng.get_rule(r1.id) {
        Err(DacError::RuleNotFound(_)) => {}
        other => panic!("expected RuleNotFound, got {:?}", other),
    }
}

#[test]
fn record_level_rules() {
    let (_dir, eng) = open_engine();

    // Deny all at collection level
    eng.add_rule("users", None, None, AccessSubjectType::Peer, "alice", AccessPermission::Deny, None, None).unwrap();

    // Allow specific record
    eng.add_rule("users", None, Some("42".into()), AccessSubjectType::Peer, "alice", AccessPermission::Allow, None, None).unwrap();

    let subject = peer_subject("alice");
    let doc = make_doc();

    // Record 42 should be allowed
    let result = eng.filter_document("users", &doc, &subject, Some("42")).unwrap();
    assert_eq!(result.as_map().unwrap().len(), 4);

    // Record 99 should be denied
    let result = eng.filter_document("users", &doc, &subject, Some("99")).unwrap();
    assert!(result.as_map().unwrap().is_empty());
}

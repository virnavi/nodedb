use nodedb_nosql::*;
use nodedb_nosql::access_history::{AccessEventType, QueryScope};
use rmpv::Value;
use tempfile::TempDir;

fn sample_data(name: &str, age: i64) -> Value {
    Value::Map(vec![
        (Value::String("name".into()), Value::String(name.into())),
        (Value::String("age".into()), Value::Integer(age.into())),
    ])
}

#[test]
fn test_database_crud() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let users = db.collection("users").unwrap();

    // Put
    let doc = users.put(sample_data("Alice", 30)).unwrap();
    assert_eq!(doc.id, 1);
    assert_eq!(doc.get_field("name"), Some(&Value::String("Alice".into())));

    // Get
    let fetched = users.get(1).unwrap();
    assert_eq!(fetched.id, 1);

    // Update
    let updated = users.update(1, sample_data("Alice Updated", 31)).unwrap();
    assert_eq!(
        updated.get_field("name"),
        Some(&Value::String("Alice Updated".into()))
    );

    // Delete
    assert!(users.delete(1).unwrap());
    assert!(users.get(1).is_err());
}

#[test]
fn test_filter_equal_to() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let users = db.collection("users").unwrap();

    users.put(sample_data("Alice", 30)).unwrap();
    users.put(sample_data("Bob", 25)).unwrap();
    users.put(sample_data("Charlie", 35)).unwrap();

    let query = Query::new().with_filter(Filter::Condition(FilterCondition::EqualTo {
        field: "name".to_string(),
        value: Value::String("Bob".into()),
    }));

    let results = users.query(&query).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].get_field("name"),
        Some(&Value::String("Bob".into()))
    );
}

#[test]
fn test_filter_comparison() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let users = db.collection("users").unwrap();

    users.put(sample_data("Alice", 30)).unwrap();
    users.put(sample_data("Bob", 25)).unwrap();
    users.put(sample_data("Charlie", 35)).unwrap();
    users.put(sample_data("Diana", 28)).unwrap();

    let query = Query::new().with_filter(Filter::Condition(FilterCondition::GreaterThan {
        field: "age".to_string(),
        value: Value::Integer(28.into()),
    }));

    let results = users.query(&query).unwrap();
    assert_eq!(results.len(), 2); // Alice(30) and Charlie(35)
}

#[test]
fn test_filter_and_or() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let users = db.collection("users").unwrap();

    users.put(sample_data("Alice", 30)).unwrap();
    users.put(sample_data("Bob", 25)).unwrap();
    users.put(sample_data("Charlie", 35)).unwrap();

    // AND: age > 25 AND age < 35
    let query = Query::new().with_filter(Filter::And(vec![
        Filter::Condition(FilterCondition::GreaterThan {
            field: "age".to_string(),
            value: Value::Integer(25.into()),
        }),
        Filter::Condition(FilterCondition::LessThan {
            field: "age".to_string(),
            value: Value::Integer(35.into()),
        }),
    ]));
    let results = users.query(&query).unwrap();
    assert_eq!(results.len(), 1); // Only Alice(30)

    // OR: name == "Alice" OR name == "Charlie"
    let query = Query::new().with_filter(Filter::Or(vec![
        Filter::Condition(FilterCondition::EqualTo {
            field: "name".to_string(),
            value: Value::String("Alice".into()),
        }),
        Filter::Condition(FilterCondition::EqualTo {
            field: "name".to_string(),
            value: Value::String("Charlie".into()),
        }),
    ]));
    let results = users.query(&query).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_sorting() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let users = db.collection("users").unwrap();

    users.put(sample_data("Charlie", 35)).unwrap();
    users.put(sample_data("Alice", 25)).unwrap();
    users.put(sample_data("Bob", 30)).unwrap();

    let query = Query::new().with_sort("age", query::SortDirection::Asc);
    let results = users.query(&query).unwrap();
    assert_eq!(results[0].get_field("name"), Some(&Value::String("Alice".into())));
    assert_eq!(results[1].get_field("name"), Some(&Value::String("Bob".into())));
    assert_eq!(results[2].get_field("name"), Some(&Value::String("Charlie".into())));

    let query = Query::new().with_sort("age", query::SortDirection::Desc);
    let results = users.query(&query).unwrap();
    assert_eq!(results[0].get_field("name"), Some(&Value::String("Charlie".into())));
}

#[test]
fn test_pagination() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let users = db.collection("users").unwrap();

    for i in 0..20 {
        users.put(sample_data(&format!("User{:02}", i), 20 + i)).unwrap();
    }

    let query = Query::new()
        .with_sort("age", query::SortDirection::Asc)
        .with_offset(5)
        .with_limit(10);
    let results = users.query(&query).unwrap();
    assert_eq!(results.len(), 10);
    assert_eq!(results[0].get_field("age"), Some(&Value::Integer(25.into())));
}

#[test]
fn test_persistence_across_close_reopen() {
    let dir = TempDir::new().unwrap();

    {
        let db = Database::open(dir.path()).unwrap();
        let users = db.collection("users").unwrap();
        users.put(sample_data("Alice", 30)).unwrap();
        users.put(sample_data("Bob", 25)).unwrap();
        let posts = db.collection("posts").unwrap();
        posts
            .put(Value::Map(vec![(
                Value::String("title".into()),
                Value::String("Hello World".into()),
            )]))
            .unwrap();
        db.close().unwrap();
    }

    {
        let db = Database::open(dir.path()).unwrap();
        let users = db.collection("users").unwrap();
        assert_eq!(users.count(), 2);

        let alice = users.get(1).unwrap();
        assert_eq!(alice.get_field("name"), Some(&Value::String("Alice".into())));

        let posts = db.collection("posts").unwrap();
        assert_eq!(posts.count(), 1);

        assert!(db.collection_names().len() >= 2);
    }
}

#[test]
fn test_drop_collection() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let users = db.collection("users").unwrap();
    users.put(sample_data("Alice", 30)).unwrap();
    drop(users);

    assert!(db.drop_collection("users").unwrap());
    assert!(!db.drop_collection("users").unwrap()); // already dropped

    let users = db.collection("users").unwrap();
    assert_eq!(users.count(), 0);
}

#[test]
fn test_write_txn() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();

    db.write_txn(|db| {
        let users = db.collection("users")?;
        users.put(sample_data("Alice", 30))?;
        users.put(sample_data("Bob", 25))?;

        let posts = db.collection("posts")?;
        posts.put(Value::Map(vec![(
            Value::String("title".into()),
            Value::String("First Post".into()),
        )]))?;
        Ok(())
    })
    .unwrap();

    let users = db.collection("users").unwrap();
    assert_eq!(users.count(), 2);

    let posts = db.collection("posts").unwrap();
    assert_eq!(posts.count(), 1);
}

#[test]
fn test_contains_filter() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let users = db.collection("users").unwrap();

    users.put(sample_data("Alice Wonderland", 30)).unwrap();
    users.put(sample_data("Bob Builder", 25)).unwrap();

    let query = Query::new().with_filter(Filter::Condition(FilterCondition::Contains {
        field: "name".to_string(),
        value: "wonder".to_string(),
    }));

    let results = users.query(&query).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_between_filter() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let users = db.collection("users").unwrap();

    users.put(sample_data("Alice", 20)).unwrap();
    users.put(sample_data("Bob", 25)).unwrap();
    users.put(sample_data("Charlie", 30)).unwrap();
    users.put(sample_data("Diana", 35)).unwrap();

    let query = Query::new().with_filter(Filter::Condition(FilterCondition::Between {
        field: "age".to_string(),
        low: Value::Integer(25.into()),
        high: Value::Integer(30.into()),
    }));

    let results = users.query(&query).unwrap();
    assert_eq!(results.len(), 2); // Bob(25) and Charlie(30)
}

// ── Access History Integration Tests ────────────────────────────────

#[test]
fn test_access_history_record_and_query() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();

    // Create some records
    let users = db.collection("users").unwrap();
    users.put(sample_data("Alice", 30)).unwrap();
    users.put(sample_data("Bob", 25)).unwrap();

    // Record access events
    db.access_history().record("users", 1, AccessEventType::Read, "local", QueryScope::Local, None, false).unwrap();
    db.access_history().record("users", 2, AccessEventType::Write, "local", QueryScope::Local, None, false).unwrap();
    db.access_history().record("users", 1, AccessEventType::FederatedRead, "peer:alice", QueryScope::Federated, Some(1), false).unwrap();

    assert_eq!(db.access_history().count().unwrap(), 3);

    // Query by collection
    let user_history = db.access_history().query_history(Some("users"), None, None, None, None).unwrap();
    assert_eq!(user_history.len(), 3);

    // Query by record_id
    let record1_history = db.access_history().query_history(Some("users"), Some(1), None, None, None).unwrap();
    assert_eq!(record1_history.len(), 2);

    // Query by event type
    let reads = db.access_history().query_history(None, None, Some(AccessEventType::Read), None, None).unwrap();
    assert_eq!(reads.len(), 1);
}

#[test]
fn test_access_history_last_access() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();

    assert!(db.access_history().last_access_time("users", 1).unwrap().is_none());

    db.access_history().record("users", 1, AccessEventType::Read, "local", QueryScope::Local, None, false).unwrap();
    let t = db.access_history().last_access_time("users", 1).unwrap();
    assert!(t.is_some());
}

#[test]
fn test_access_history_should_record_excludes_internal() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();

    // __access_history__ is excluded by default
    assert!(!db.should_record_access("__access_history__"));
    assert!(db.should_record_access("users"));
}

// ── Trim Integration Tests ──────────────────────────────────────────

#[test]
fn test_trim_default_never_trim() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let users = db.collection("users").unwrap();
    users.put(sample_data("Alice", 30)).unwrap();
    users.put(sample_data("Bob", 25)).unwrap();

    // Default: all collections are never-trim
    let report = db.trim("users", &TrimPolicy::NotAccessedSince(0), false).unwrap();
    assert_eq!(report.deleted_count, 0);
    assert_eq!(report.candidate_count, 0); // never-trim = empty report
}

#[test]
fn test_trim_with_trimmable_override() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let users = db.collection("users").unwrap();
    users.put(sample_data("Alice", 30)).unwrap();
    users.put(sample_data("Bob", 25)).unwrap();

    // Mark as trimmable via runtime override
    let qn = QualifiedName::parse("users");
    db.trim_config().set_trim_policy(&qn.meta_key(), &TrimPolicy::NotAccessedSince(0)).unwrap();

    // Now trim should work — no access history means all records are eligible
    let report = db.trim("users", &TrimPolicy::NotAccessedSince(0), false).unwrap();
    assert_eq!(report.deleted_count, 2);
    assert_eq!(users.count(), 0);
}

#[test]
fn test_trim_dry_run() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let users = db.collection("users").unwrap();
    users.put(sample_data("Alice", 30)).unwrap();

    let qn = QualifiedName::parse("users");
    db.trim_config().set_trim_policy(&qn.meta_key(), &TrimPolicy::NotAccessedSince(0)).unwrap();

    let report = db.trim("users", &TrimPolicy::NotAccessedSince(0), true).unwrap();
    assert_eq!(report.deleted_count, 1);
    assert!(report.dry_run);
    // Dry run: record should still exist
    assert_eq!(users.count(), 1);
}

#[test]
fn test_trim_record_never_trim_protection() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let users = db.collection("users").unwrap();
    users.put(sample_data("Alice", 30)).unwrap();
    users.put(sample_data("Bob", 25)).unwrap();

    let qn = QualifiedName::parse("users");
    let meta_key = qn.meta_key();
    db.trim_config().set_trim_policy(&meta_key, &TrimPolicy::NotAccessedSince(0)).unwrap();

    // Protect record 1 from trimming
    db.trim_config().set_record_never_trim(&meta_key, 1).unwrap();

    let report = db.trim("users", &TrimPolicy::NotAccessedSince(0), false).unwrap();
    assert_eq!(report.deleted_count, 1); // Only Bob deleted
    assert_eq!(report.never_trim_skipped_count, 1); // Alice protected
    assert_eq!(users.count(), 1);
    assert!(users.get(1).is_ok()); // Alice still there
}

#[test]
fn test_trim_recommend() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let users = db.collection("users").unwrap();
    users.put(sample_data("Alice", 30)).unwrap();
    users.put(sample_data("Bob", 25)).unwrap();

    let qn = QualifiedName::parse("users");
    db.trim_config().set_trim_policy(&qn.meta_key(), &TrimPolicy::NotAccessedSince(0)).unwrap();

    let rec = db.recommend_trim(&TrimPolicy::NotAccessedSince(0), &[]).unwrap();
    assert_eq!(rec.total_candidate_count, 2);
    assert_eq!(rec.by_collection.len(), 1);
    assert_eq!(rec.by_collection[0].collection, "public.users");
}

#[test]
fn test_trim_approved() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let users = db.collection("users").unwrap();
    users.put(sample_data("Alice", 30)).unwrap();
    users.put(sample_data("Bob", 25)).unwrap();

    let qn = QualifiedName::parse("users");
    db.trim_config().set_trim_policy(&qn.meta_key(), &TrimPolicy::NotAccessedSince(0)).unwrap();

    let approval = UserApprovedTrim {
        policy: TrimPolicy::NotAccessedSince(0),
        confirmed_record_ids: vec![("users".to_string(), 1)],
        approval_note: Some("test".to_string()),
        approved_at_utc: chrono::Utc::now(),
    };

    let report = db.trim_approved(&approval).unwrap();
    assert_eq!(report.deleted_count, 1);
    assert_eq!(users.count(), 1);
    assert!(users.get(1).is_err()); // Alice deleted
    assert!(users.get(2).is_ok()); // Bob still there
}

#[test]
fn test_trim_config_persistence() {
    let dir = TempDir::new().unwrap();

    {
        let db = Database::open(dir.path()).unwrap();
        let qn = QualifiedName::parse("users");
        db.trim_config().set_trim_policy(&qn.meta_key(), &TrimPolicy::NotAccessedSince(86400)).unwrap();
        db.flush().unwrap();
    }

    {
        let db = Database::open(dir.path()).unwrap();
        let qn = QualifiedName::parse("users");
        let policy = db.trim_config().get_collection_override(&qn.meta_key()).unwrap();
        assert!(policy.is_some());
        assert_eq!(policy.unwrap().as_str(), "not_accessed_since");
    }
}

#[test]
fn test_trim_all() {
    let dir = TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let users = db.collection("users").unwrap();
    users.put(sample_data("Alice", 30)).unwrap();
    let posts = db.collection("posts").unwrap();
    posts.put(Value::Map(vec![
        (Value::String("title".into()), Value::String("Hello".into())),
    ])).unwrap();

    // Mark both as trimmable
    let qn1 = QualifiedName::parse("users");
    let qn2 = QualifiedName::parse("posts");
    db.trim_config().set_trim_policy(&qn1.meta_key(), &TrimPolicy::NotAccessedSince(0)).unwrap();
    db.trim_config().set_trim_policy(&qn2.meta_key(), &TrimPolicy::NotAccessedSince(0)).unwrap();

    let report = db.trim_all(&TrimPolicy::NotAccessedSince(0), false).unwrap();
    assert_eq!(report.deleted_count, 2);
    assert_eq!(users.count(), 0);
    assert_eq!(posts.count(), 0);
}

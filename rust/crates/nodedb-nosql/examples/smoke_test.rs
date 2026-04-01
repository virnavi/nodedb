use nodedb_nosql::{Database, Filter, FilterCondition, Query, query::SortDirection};
use rmpv::Value;

fn main() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = Database::open(dir.path()).unwrap();

    // Create a collection and insert documents
    let users = db.collection("users").unwrap();
    users.put(Value::Map(vec![
        (Value::String("name".into()), Value::String("Alice".into())),
        (Value::String("age".into()), Value::Integer(30.into())),
    ])).unwrap();
    users.put(Value::Map(vec![
        (Value::String("name".into()), Value::String("Bob".into())),
        (Value::String("age".into()), Value::Integer(25.into())),
    ])).unwrap();
    users.put(Value::Map(vec![
        (Value::String("name".into()), Value::String("Charlie".into())),
        (Value::String("age".into()), Value::Integer(35.into())),
    ])).unwrap();

    println!("Inserted {} documents", users.count());

    // Query: age > 26, sorted by age descending
    let query = Query::new()
        .with_filter(Filter::Condition(FilterCondition::GreaterThan {
            field: "age".to_string(),
            value: Value::Integer(26.into()),
        }))
        .with_sort("age", SortDirection::Desc);

    let results = users.query(&query).unwrap();
    println!("Query results (age > 26, sorted desc):");
    for doc in &results {
        let name = doc.get_field("name").unwrap();
        let age = doc.get_field("age").unwrap();
        println!("  id={} name={} age={}", doc.id, name, age);
    }
    assert_eq!(results.len(), 2);

    // Get by ID
    let alice = users.get(1).unwrap();
    println!("Get by ID 1: {}", alice.get_field("name").unwrap());

    // Delete
    users.delete(2).unwrap();
    println!("After delete: {} documents", users.count());
    assert_eq!(users.count(), 2);

    db.close().unwrap();
    println!("Smoke test passed!");
}

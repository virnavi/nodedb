use chrono::{DateTime, Utc};
use rmpv::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: i64,
    pub collection: String,
    pub data: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Document {
    pub fn new(id: i64, collection: &str, data: Value) -> Self {
        let now = Utc::now();
        Document {
            id,
            collection: collection.to_string(),
            data,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a document with a pre-computed timestamp (for batch operations).
    pub fn new_with_timestamp(id: i64, collection: &str, data: Value, now: DateTime<Utc>) -> Self {
        Document {
            id,
            collection: collection.to_string(),
            data,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn get_field(&self, path: &str) -> Option<&Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = &self.data;
        for part in parts {
            match current {
                Value::Map(map) => {
                    let found = map.iter().find(|(k, _)| {
                        match k {
                            Value::String(s) => s.as_str() == Some(part),
                            _ => false,
                        }
                    });
                    match found {
                        Some((_, v)) => current = v,
                        None => return None,
                    }
                }
                _ => return None,
            }
        }
        Some(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_doc() -> Document {
        let data = Value::Map(vec![
            (Value::String("name".into()), Value::String("Alice".into())),
            (Value::String("age".into()), Value::Integer(30.into())),
            (
                Value::String("address".into()),
                Value::Map(vec![(
                    Value::String("city".into()),
                    Value::String("NYC".into()),
                )]),
            ),
        ]);
        Document::new(1, "users", data)
    }

    #[test]
    fn test_get_field() {
        let doc = sample_doc();
        assert_eq!(
            doc.get_field("name"),
            Some(&Value::String("Alice".into()))
        );
        assert_eq!(doc.get_field("age"), Some(&Value::Integer(30.into())));
        assert!(doc.get_field("missing").is_none());
    }

    #[test]
    fn test_get_nested_field() {
        let doc = sample_doc();
        assert_eq!(
            doc.get_field("address.city"),
            Some(&Value::String("NYC".into()))
        );
    }

    #[test]
    fn test_serialization_roundtrip() {
        let doc = sample_doc();
        let bytes = rmp_serde::to_vec(&doc).unwrap();
        let decoded: Document = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, doc.id);
        assert_eq!(decoded.collection, doc.collection);
    }
}

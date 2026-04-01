use rmpv::Value;
use serde::{Deserialize, Serialize};

use crate::document::Document;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterCondition {
    EqualTo { field: String, value: Value },
    NotEqualTo { field: String, value: Value },
    GreaterThan { field: String, value: Value },
    GreaterThanOrEqual { field: String, value: Value },
    LessThan { field: String, value: Value },
    LessThanOrEqual { field: String, value: Value },
    Contains { field: String, value: String },
    StartsWith { field: String, value: String },
    EndsWith { field: String, value: String },
    IsNull { field: String },
    IsNotNull { field: String },
    Between { field: String, low: Value, high: Value },
    /// Match a value at a JSON path within a field (JSONB path query).
    JsonPathEquals { field: String, path: String, value: Value },
    /// Check if a key exists (and is not null) at a JSON path within a field.
    JsonHasKey { field: String, path: String },
    /// Check if a Map field is a superset of the given map.
    JsonContains { field: String, value: Value },
    /// Check if an array field contains a specific element.
    ArrayContains { field: String, value: Value },
    /// Check if an array field has any overlap with a given list of values.
    ArrayOverlap { field: String, values: Vec<Value> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Filter {
    Condition(FilterCondition),
    And(Vec<Filter>),
    Or(Vec<Filter>),
}

impl Filter {
    pub fn matches(&self, doc: &Document) -> bool {
        match self {
            Filter::Condition(cond) => cond.matches(doc),
            Filter::And(filters) => filters.iter().all(|f| f.matches(doc)),
            Filter::Or(filters) => filters.iter().any(|f| f.matches(doc)),
        }
    }
}

impl FilterCondition {
    fn matches(&self, doc: &Document) -> bool {
        match self {
            FilterCondition::EqualTo { field, value } => {
                doc.get_field(field).map_or(false, |v| values_equal(v, value))
            }
            FilterCondition::NotEqualTo { field, value } => {
                doc.get_field(field).map_or(true, |v| !values_equal(v, value))
            }
            FilterCondition::GreaterThan { field, value } => {
                doc.get_field(field).map_or(false, |v| compare_values(v, value) == Some(std::cmp::Ordering::Greater))
            }
            FilterCondition::GreaterThanOrEqual { field, value } => {
                doc.get_field(field).map_or(false, |v| {
                    matches!(compare_values(v, value), Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal))
                })
            }
            FilterCondition::LessThan { field, value } => {
                doc.get_field(field).map_or(false, |v| compare_values(v, value) == Some(std::cmp::Ordering::Less))
            }
            FilterCondition::LessThanOrEqual { field, value } => {
                doc.get_field(field).map_or(false, |v| {
                    matches!(compare_values(v, value), Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal))
                })
            }
            FilterCondition::Contains { field, value } => {
                doc.get_field(field).map_or(false, |v| {
                    if let Value::String(s) = v {
                        s.as_str().map_or(false, |s| s.to_lowercase().contains(&value.to_lowercase()))
                    } else {
                        false
                    }
                })
            }
            FilterCondition::StartsWith { field, value } => {
                doc.get_field(field).map_or(false, |v| {
                    if let Value::String(s) = v {
                        s.as_str().map_or(false, |s| s.starts_with(value.as_str()))
                    } else {
                        false
                    }
                })
            }
            FilterCondition::EndsWith { field, value } => {
                doc.get_field(field).map_or(false, |v| {
                    if let Value::String(s) = v {
                        s.as_str().map_or(false, |s| s.ends_with(value.as_str()))
                    } else {
                        false
                    }
                })
            }
            FilterCondition::IsNull { field } => {
                doc.get_field(field).map_or(true, |v| v.is_nil())
            }
            FilterCondition::IsNotNull { field } => {
                doc.get_field(field).map_or(false, |v| !v.is_nil())
            }
            FilterCondition::Between { field, low, high } => {
                doc.get_field(field).map_or(false, |v| {
                    matches!(
                        (compare_values(v, low), compare_values(v, high)),
                        (Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal), Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal))
                    )
                })
            }
            FilterCondition::JsonPathEquals { field, path, value } => {
                doc.get_field(field)
                    .and_then(|field_val| navigate_json_path(field_val, path))
                    .map_or(false, |v| values_equal(v, value))
            }
            FilterCondition::JsonHasKey { field, path } => {
                doc.get_field(field)
                    .and_then(|field_val| navigate_json_path(field_val, path))
                    .map_or(false, |v| !v.is_nil())
            }
            FilterCondition::JsonContains { field, value } => {
                doc.get_field(field).map_or(false, |field_val| {
                    json_map_contains(field_val, value)
                })
            }
            FilterCondition::ArrayContains { field, value } => {
                doc.get_field(field).map_or(false, |field_val| {
                    if let Value::Array(arr) = field_val {
                        arr.iter().any(|v| values_equal(v, value))
                    } else {
                        false
                    }
                })
            }
            FilterCondition::ArrayOverlap { field, values } => {
                doc.get_field(field).map_or(false, |field_val| {
                    if let Value::Array(arr) = field_val {
                        values.iter().any(|target| arr.iter().any(|v| values_equal(v, target)))
                    } else {
                        false
                    }
                })
            }
        }
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Nil, Value::Nil) => true,
        (Value::Boolean(a), Value::Boolean(b)) => a == b,
        (Value::Integer(a), Value::Integer(b)) => {
            // Compare as i64 or u64
            match (a.as_i64(), b.as_i64()) {
                (Some(a), Some(b)) => a == b,
                _ => match (a.as_f64(), b.as_f64()) {
                    (Some(a), Some(b)) => a == b,
                    _ => false,
                },
            }
        }
        (Value::F32(a), Value::F32(b)) => a == b,
        (Value::F64(a), Value::F64(b)) => a == b,
        // Allow cross-numeric comparison
        (Value::Integer(a), Value::F64(b)) => a.as_f64().map_or(false, |a| a == *b),
        (Value::F64(a), Value::Integer(b)) => b.as_f64().map_or(false, |b| *a == b),
        (Value::String(a), Value::String(b)) => a == b,
        _ => a == b,
    }
}

fn compare_values(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Integer(a), Value::Integer(b)) => {
            match (a.as_i64(), b.as_i64()) {
                (Some(a), Some(b)) => Some(a.cmp(&b)),
                _ => a.as_f64().and_then(|a| b.as_f64().map(|b| a.partial_cmp(&b))).flatten(),
            }
        }
        (Value::F64(a), Value::F64(b)) => a.partial_cmp(b),
        (Value::F32(a), Value::F32(b)) => a.partial_cmp(b),
        (Value::Integer(a), Value::F64(b)) => a.as_f64().and_then(|a| a.partial_cmp(b)),
        (Value::F64(a), Value::Integer(b)) => b.as_f64().and_then(|b| a.partial_cmp(&b)),
        (Value::String(a), Value::String(b)) => a.as_str().and_then(|a| b.as_str().map(|b| a.cmp(b))),
        _ => None,
    }
}

pub(crate) fn compare_field_values(a: Option<&Value>, b: Option<&Value>) -> std::cmp::Ordering {
    match (a, b) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
        (Some(a), Some(b)) => compare_values(a, b).unwrap_or(std::cmp::Ordering::Equal),
    }
}

/// Navigate a JSON path within a Value (dot-notation with integer array indexing).
fn navigate_json_path<'a>(val: &'a Value, path: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = val;
    for part in parts {
        match current {
            Value::Map(map) => {
                let found = map.iter().find(|(k, _)| {
                    k.as_str().map_or(false, |s| s == part)
                });
                match found {
                    Some((_, v)) => current = v,
                    None => return None,
                }
            }
            Value::Array(arr) => {
                if let Ok(idx) = part.parse::<usize>() {
                    current = arr.get(idx)?;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }
    Some(current)
}

/// Check if `haystack` map contains all entries from `needle` map.
fn json_map_contains(haystack: &Value, needle: &Value) -> bool {
    match (haystack, needle) {
        (Value::Map(h), Value::Map(n)) => {
            n.iter().all(|(nk, nv)| {
                h.iter().any(|(hk, hv)| {
                    values_equal(hk, nk) && values_equal(hv, nv)
                })
            })
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_doc(name: &str, age: i64) -> Document {
        let data = Value::Map(vec![
            (Value::String("name".into()), Value::String(name.into())),
            (Value::String("age".into()), Value::Integer(age.into())),
        ]);
        Document::new(1, "users", data)
    }

    #[test]
    fn test_equal_to() {
        let doc = make_doc("Alice", 30);
        let f = Filter::Condition(FilterCondition::EqualTo {
            field: "name".to_string(),
            value: Value::String("Alice".into()),
        });
        assert!(f.matches(&doc));

        let f2 = Filter::Condition(FilterCondition::EqualTo {
            field: "name".to_string(),
            value: Value::String("Bob".into()),
        });
        assert!(!f2.matches(&doc));
    }

    #[test]
    fn test_greater_than() {
        let doc = make_doc("Alice", 30);
        let f = Filter::Condition(FilterCondition::GreaterThan {
            field: "age".to_string(),
            value: Value::Integer(25.into()),
        });
        assert!(f.matches(&doc));

        let f2 = Filter::Condition(FilterCondition::GreaterThan {
            field: "age".to_string(),
            value: Value::Integer(30.into()),
        });
        assert!(!f2.matches(&doc));
    }

    #[test]
    fn test_contains() {
        let doc = make_doc("Alice Wonderland", 25);
        let f = Filter::Condition(FilterCondition::Contains {
            field: "name".to_string(),
            value: "alice".to_string(),
        });
        assert!(f.matches(&doc)); // case insensitive
    }

    #[test]
    fn test_is_null() {
        let doc = make_doc("Alice", 30);
        let f = Filter::Condition(FilterCondition::IsNull {
            field: "email".to_string(),
        });
        assert!(f.matches(&doc)); // missing field treated as null

        let f2 = Filter::Condition(FilterCondition::IsNull {
            field: "name".to_string(),
        });
        assert!(!f2.matches(&doc));
    }

    #[test]
    fn test_and() {
        let doc = make_doc("Alice", 30);
        let f = Filter::And(vec![
            Filter::Condition(FilterCondition::EqualTo {
                field: "name".to_string(),
                value: Value::String("Alice".into()),
            }),
            Filter::Condition(FilterCondition::GreaterThan {
                field: "age".to_string(),
                value: Value::Integer(25.into()),
            }),
        ]);
        assert!(f.matches(&doc));
    }

    #[test]
    fn test_or() {
        let doc = make_doc("Alice", 20);
        let f = Filter::Or(vec![
            Filter::Condition(FilterCondition::EqualTo {
                field: "name".to_string(),
                value: Value::String("Bob".into()),
            }),
            Filter::Condition(FilterCondition::LessThan {
                field: "age".to_string(),
                value: Value::Integer(25.into()),
            }),
        ]);
        assert!(f.matches(&doc));
    }

    #[test]
    fn test_between() {
        let doc = make_doc("Alice", 30);
        let f = Filter::Condition(FilterCondition::Between {
            field: "age".to_string(),
            low: Value::Integer(25.into()),
            high: Value::Integer(35.into()),
        });
        assert!(f.matches(&doc));

        let f2 = Filter::Condition(FilterCondition::Between {
            field: "age".to_string(),
            low: Value::Integer(31.into()),
            high: Value::Integer(35.into()),
        });
        assert!(!f2.matches(&doc));
    }

    #[test]
    fn test_starts_with() {
        let doc = make_doc("Alice", 30);
        let f = Filter::Condition(FilterCondition::StartsWith {
            field: "name".to_string(),
            value: "Ali".to_string(),
        });
        assert!(f.matches(&doc));
    }

    #[test]
    fn test_ends_with() {
        let doc = make_doc("Alice", 30);
        let f = Filter::Condition(FilterCondition::EndsWith {
            field: "name".to_string(),
            value: "ice".to_string(),
        });
        assert!(f.matches(&doc));
    }

    fn make_jsonb_doc() -> Document {
        let metadata = Value::Map(vec![
            (Value::String("type".into()), Value::String("article".into())),
            (Value::String("published".into()), Value::Boolean(true)),
            (Value::String("nested".into()), Value::Map(vec![
                (Value::String("key".into()), Value::String("val".into())),
            ])),
        ]);
        let tags = Value::Array(vec![
            Value::String("dart".into()),
            Value::String("rust".into()),
            Value::String("flutter".into()),
        ]);
        let data = Value::Map(vec![
            (Value::String("name".into()), Value::String("Alice".into())),
            (Value::String("metadata".into()), metadata),
            (Value::String("tags".into()), tags),
        ]);
        Document::new(1, "items", data)
    }

    #[test]
    fn test_json_path_equals() {
        let doc = make_jsonb_doc();
        let f = Filter::Condition(FilterCondition::JsonPathEquals {
            field: "metadata".to_string(),
            path: "type".to_string(),
            value: Value::String("article".into()),
        });
        assert!(f.matches(&doc));

        let f2 = Filter::Condition(FilterCondition::JsonPathEquals {
            field: "metadata".to_string(),
            path: "type".to_string(),
            value: Value::String("blog".into()),
        });
        assert!(!f2.matches(&doc));
    }

    #[test]
    fn test_json_path_nested() {
        let doc = make_jsonb_doc();
        let f = Filter::Condition(FilterCondition::JsonPathEquals {
            field: "metadata".to_string(),
            path: "nested.key".to_string(),
            value: Value::String("val".into()),
        });
        assert!(f.matches(&doc));
    }

    #[test]
    fn test_json_path_array_index() {
        let doc = make_jsonb_doc();
        let f = Filter::Condition(FilterCondition::JsonPathEquals {
            field: "tags".to_string(),
            path: "0".to_string(),
            value: Value::String("dart".into()),
        });
        assert!(f.matches(&doc));

        let f2 = Filter::Condition(FilterCondition::JsonPathEquals {
            field: "tags".to_string(),
            path: "1".to_string(),
            value: Value::String("rust".into()),
        });
        assert!(f2.matches(&doc));
    }

    #[test]
    fn test_json_has_key() {
        let doc = make_jsonb_doc();
        let f = Filter::Condition(FilterCondition::JsonHasKey {
            field: "metadata".to_string(),
            path: "type".to_string(),
        });
        assert!(f.matches(&doc));

        let f2 = Filter::Condition(FilterCondition::JsonHasKey {
            field: "metadata".to_string(),
            path: "missing".to_string(),
        });
        assert!(!f2.matches(&doc));
    }

    #[test]
    fn test_json_has_key_nested() {
        let doc = make_jsonb_doc();
        let f = Filter::Condition(FilterCondition::JsonHasKey {
            field: "metadata".to_string(),
            path: "nested.key".to_string(),
        });
        assert!(f.matches(&doc));

        let f2 = Filter::Condition(FilterCondition::JsonHasKey {
            field: "metadata".to_string(),
            path: "nested.nope".to_string(),
        });
        assert!(!f2.matches(&doc));
    }

    #[test]
    fn test_json_contains() {
        let doc = make_jsonb_doc();
        let needle = Value::Map(vec![
            (Value::String("type".into()), Value::String("article".into())),
        ]);
        let f = Filter::Condition(FilterCondition::JsonContains {
            field: "metadata".to_string(),
            value: needle,
        });
        assert!(f.matches(&doc));

        let needle2 = Value::Map(vec![
            (Value::String("type".into()), Value::String("article".into())),
            (Value::String("published".into()), Value::Boolean(true)),
        ]);
        let f2 = Filter::Condition(FilterCondition::JsonContains {
            field: "metadata".to_string(),
            value: needle2,
        });
        assert!(f2.matches(&doc));

        let needle3 = Value::Map(vec![
            (Value::String("type".into()), Value::String("blog".into())),
        ]);
        let f3 = Filter::Condition(FilterCondition::JsonContains {
            field: "metadata".to_string(),
            value: needle3,
        });
        assert!(!f3.matches(&doc));
    }

    #[test]
    fn test_array_contains() {
        let doc = make_jsonb_doc();
        let f = Filter::Condition(FilterCondition::ArrayContains {
            field: "tags".to_string(),
            value: Value::String("dart".into()),
        });
        assert!(f.matches(&doc));

        let f2 = Filter::Condition(FilterCondition::ArrayContains {
            field: "tags".to_string(),
            value: Value::String("python".into()),
        });
        assert!(!f2.matches(&doc));
    }

    #[test]
    fn test_array_contains_empty() {
        let data = Value::Map(vec![
            (Value::String("tags".into()), Value::Array(vec![])),
        ]);
        let doc = Document::new(1, "items", data);
        let f = Filter::Condition(FilterCondition::ArrayContains {
            field: "tags".to_string(),
            value: Value::String("dart".into()),
        });
        assert!(!f.matches(&doc));
    }

    #[test]
    fn test_array_overlap() {
        let doc = make_jsonb_doc();
        let f = Filter::Condition(FilterCondition::ArrayOverlap {
            field: "tags".to_string(),
            values: vec![Value::String("dart".into()), Value::String("python".into())],
        });
        assert!(f.matches(&doc));

        let f2 = Filter::Condition(FilterCondition::ArrayOverlap {
            field: "tags".to_string(),
            values: vec![Value::String("python".into()), Value::String("java".into())],
        });
        assert!(!f2.matches(&doc));
    }

    #[test]
    fn test_json_path_missing_field() {
        let doc = make_jsonb_doc();
        let f = Filter::Condition(FilterCondition::JsonPathEquals {
            field: "nonexistent".to_string(),
            path: "key".to_string(),
            value: Value::String("val".into()),
        });
        assert!(!f.matches(&doc));
    }
}

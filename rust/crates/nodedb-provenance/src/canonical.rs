use rmpv::Value;
use crate::error::ProvenanceError;

/// Serialize an rmpv::Value to canonical MessagePack bytes.
/// Maps are sorted by key alphabetically (recursively) to produce deterministic output.
pub fn canonical_msgpack(value: &Value) -> Result<Vec<u8>, ProvenanceError> {
    let sorted = sort_value(value);
    let mut buf = Vec::new();
    rmpv::encode::write_value(&mut buf, &sorted)
        .map_err(|e| ProvenanceError::Canonical(e.to_string()))?;
    Ok(buf)
}

fn sort_value(value: &Value) -> Value {
    match value {
        Value::Map(entries) => {
            let mut sorted: Vec<(Value, Value)> = entries
                .iter()
                .map(|(k, v)| (sort_value(k), sort_value(v)))
                .collect();
            sorted.sort_by(|(a, _), (b, _)| cmp_value(a, b));
            Value::Map(sorted)
        }
        Value::Array(items) => {
            Value::Array(items.iter().map(sort_value).collect())
        }
        other => other.clone(),
    }
}

fn cmp_value(a: &Value, b: &Value) -> std::cmp::Ordering {
    let a_str = value_to_sort_string(a);
    let b_str = value_to_sort_string(b);
    a_str.cmp(&b_str)
}

fn value_to_sort_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.as_str().unwrap_or("").to_string(),
        Value::Integer(i) => format!("{}", i),
        Value::Boolean(b) => format!("{}", b),
        Value::F32(f) => format!("{}", f),
        Value::F64(f) => format!("{}", f),
        Value::Nil => "null".to_string(),
        Value::Binary(b) => format!("{:?}", b),
        _ => format!("{:?}", v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmpv::Value;

    #[test]
    fn sorted_keys() {
        let map = Value::Map(vec![
            (Value::String("zebra".into()), Value::Integer(3.into())),
            (Value::String("alpha".into()), Value::Integer(1.into())),
            (Value::String("middle".into()), Value::Integer(2.into())),
        ]);
        let bytes = canonical_msgpack(&map).unwrap();

        // Decode and verify order
        let decoded: Value = rmpv::decode::read_value(&mut &bytes[..]).unwrap();
        if let Value::Map(entries) = decoded {
            let keys: Vec<&str> = entries.iter()
                .map(|(k, _)| k.as_str().unwrap())
                .collect();
            assert_eq!(keys, vec!["alpha", "middle", "zebra"]);
        } else {
            panic!("expected map");
        }
    }

    #[test]
    fn nested_maps_sorted() {
        let inner = Value::Map(vec![
            (Value::String("z".into()), Value::Integer(2.into())),
            (Value::String("a".into()), Value::Integer(1.into())),
        ]);
        let outer = Value::Map(vec![
            (Value::String("nested".into()), inner),
            (Value::String("alpha".into()), Value::Integer(0.into())),
        ]);
        let bytes = canonical_msgpack(&outer).unwrap();
        let decoded: Value = rmpv::decode::read_value(&mut &bytes[..]).unwrap();
        if let Value::Map(entries) = &decoded {
            assert_eq!(entries[0].0.as_str().unwrap(), "alpha");
            assert_eq!(entries[1].0.as_str().unwrap(), "nested");
            if let Value::Map(inner_entries) = &entries[1].1 {
                assert_eq!(inner_entries[0].0.as_str().unwrap(), "a");
                assert_eq!(inner_entries[1].0.as_str().unwrap(), "z");
            } else {
                panic!("expected nested map");
            }
        } else {
            panic!("expected map");
        }
    }

    #[test]
    fn stable_output() {
        let map = Value::Map(vec![
            (Value::String("b".into()), Value::Integer(2.into())),
            (Value::String("a".into()), Value::Integer(1.into())),
        ]);
        let bytes1 = canonical_msgpack(&map).unwrap();
        let bytes2 = canonical_msgpack(&map).unwrap();
        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn non_map_passthrough() {
        let val = Value::Integer(42.into());
        let bytes = canonical_msgpack(&val).unwrap();
        let decoded: Value = rmpv::decode::read_value(&mut &bytes[..]).unwrap();
        assert_eq!(decoded, Value::Integer(42.into()));
    }

    #[test]
    fn array_with_maps() {
        let arr = Value::Array(vec![
            Value::Map(vec![
                (Value::String("z".into()), Value::Integer(1.into())),
                (Value::String("a".into()), Value::Integer(2.into())),
            ]),
        ]);
        let bytes = canonical_msgpack(&arr).unwrap();
        let decoded: Value = rmpv::decode::read_value(&mut &bytes[..]).unwrap();
        if let Value::Array(items) = decoded {
            if let Value::Map(entries) = &items[0] {
                assert_eq!(entries[0].0.as_str().unwrap(), "a");
            } else {
                panic!("expected map in array");
            }
        } else {
            panic!("expected array");
        }
    }

    #[test]
    fn empty_map() {
        let map = Value::Map(vec![]);
        let bytes = canonical_msgpack(&map).unwrap();
        let decoded: Value = rmpv::decode::read_value(&mut &bytes[..]).unwrap();
        assert_eq!(decoded, Value::Map(vec![]));
    }
}

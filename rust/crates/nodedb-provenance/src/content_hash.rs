use rmpv::Value;
use sha2::{Sha256, Digest};
use crate::canonical::canonical_msgpack;
use crate::error::ProvenanceError;

/// Compute the SHA-256 content hash of data in canonical MessagePack form.
/// Returns a hex-encoded string (64 characters).
pub fn compute_content_hash(data: &Value) -> Result<String, ProvenanceError> {
    let canonical_bytes = canonical_msgpack(data)?;
    let hash = Sha256::digest(&canonical_bytes);
    Ok(hex_encode(&hash))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmpv::Value;

    #[test]
    fn hash_is_64_hex_chars() {
        let data = Value::Map(vec![
            (Value::String("name".into()), Value::String("Alice".into())),
        ]);
        let hash = compute_content_hash(&data).unwrap();
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn deterministic_regardless_of_key_order() {
        let data1 = Value::Map(vec![
            (Value::String("b".into()), Value::Integer(2.into())),
            (Value::String("a".into()), Value::Integer(1.into())),
        ]);
        let data2 = Value::Map(vec![
            (Value::String("a".into()), Value::Integer(1.into())),
            (Value::String("b".into()), Value::Integer(2.into())),
        ]);
        assert_eq!(
            compute_content_hash(&data1).unwrap(),
            compute_content_hash(&data2).unwrap()
        );
    }

    #[test]
    fn different_data_different_hash() {
        let data1 = Value::Map(vec![
            (Value::String("x".into()), Value::Integer(1.into())),
        ]);
        let data2 = Value::Map(vec![
            (Value::String("x".into()), Value::Integer(2.into())),
        ]);
        assert_ne!(
            compute_content_hash(&data1).unwrap(),
            compute_content_hash(&data2).unwrap()
        );
    }

    #[test]
    fn empty_map_hash() {
        let data = Value::Map(vec![]);
        let hash = compute_content_hash(&data).unwrap();
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn nested_data_hash() {
        let data = Value::Map(vec![
            (Value::String("outer".into()), Value::Map(vec![
                (Value::String("inner".into()), Value::Integer(42.into())),
            ])),
        ]);
        let hash = compute_content_hash(&data).unwrap();
        assert_eq!(hash.len(), 64);
    }
}

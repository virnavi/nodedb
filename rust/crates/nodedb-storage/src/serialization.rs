use crate::error::StorageError;
use serde::{Deserialize, Serialize};

pub fn to_msgpack<T: Serialize>(value: &T) -> Result<Vec<u8>, StorageError> {
    rmp_serde::to_vec(value).map_err(StorageError::from)
}

pub fn from_msgpack<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, StorageError> {
    rmp_serde::from_slice(bytes).map_err(StorageError::from)
}

pub fn encode_id(id: i64) -> Vec<u8> {
    id.to_be_bytes().to_vec()
}

pub fn decode_id(bytes: &[u8]) -> Result<i64, StorageError> {
    let arr: [u8; 8] = bytes
        .try_into()
        .map_err(|_| StorageError::Serialization("invalid id bytes: expected 8 bytes".to_string()))?;
    Ok(i64::from_be_bytes(arr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_string() {
        let val = "hello world".to_string();
        let bytes = to_msgpack(&val).unwrap();
        let decoded: String = from_msgpack(&bytes).unwrap();
        assert_eq!(val, decoded);
    }

    #[test]
    fn test_roundtrip_struct() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Point {
            x: f64,
            y: f64,
        }

        let p = Point { x: 1.5, y: 2.5 };
        let bytes = to_msgpack(&p).unwrap();
        let decoded: Point = from_msgpack(&bytes).unwrap();
        assert_eq!(p, decoded);
    }

    #[test]
    fn test_roundtrip_vec() {
        let val = vec![1u32, 2, 3, 4, 5];
        let bytes = to_msgpack(&val).unwrap();
        let decoded: Vec<u32> = from_msgpack(&bytes).unwrap();
        assert_eq!(val, decoded);
    }

    #[test]
    fn test_encode_decode_id() {
        for id in [0i64, 1, -1, i64::MAX, i64::MIN, 42, 100000] {
            let bytes = encode_id(id);
            assert_eq!(bytes.len(), 8);
            let decoded = decode_id(&bytes).unwrap();
            assert_eq!(id, decoded);
        }
    }

    #[test]
    fn test_id_ordering() {
        // Big-endian encoding preserves ordering for positive IDs
        let a = encode_id(1);
        let b = encode_id(2);
        let c = encode_id(100);
        assert!(a < b);
        assert!(b < c);
    }

    #[test]
    fn test_decode_id_invalid() {
        assert!(decode_id(&[1, 2, 3]).is_err());
        assert!(decode_id(&[]).is_err());
    }
}

use chrono::{DateTime, Utc};
use rmpv::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
}

impl DistanceMetric {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "cosine" => Some(DistanceMetric::Cosine),
            "euclidean" | "l2" => Some(DistanceMetric::Euclidean),
            "dotproduct" | "dot_product" | "dot" => Some(DistanceMetric::DotProduct),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    pub dimension: usize,
    pub metric: DistanceMetric,
    pub max_elements: usize,
    pub ef_construction: usize,
    pub max_nb_connection: usize,
    pub max_layer: usize,
}

impl CollectionConfig {
    pub fn new(dimension: usize) -> Self {
        CollectionConfig {
            dimension,
            metric: DistanceMetric::Cosine,
            max_elements: 100_000,
            ef_construction: 200,
            max_nb_connection: 16,
            max_layer: 16,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorRecord {
    pub id: i64,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl VectorRecord {
    pub fn new(id: i64, metadata: Value) -> Self {
        let now = Utc::now();
        VectorRecord {
            id,
            metadata,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: i64,
    pub distance: f32,
    pub metadata: Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_config_defaults() {
        let config = CollectionConfig::new(512);
        assert_eq!(config.dimension, 512);
        assert_eq!(config.metric, DistanceMetric::Cosine);
        assert_eq!(config.max_elements, 100_000);
    }

    #[test]
    fn test_vector_record_roundtrip() {
        let record = VectorRecord::new(1, Value::String("test".into()));
        let bytes = rmp_serde::to_vec(&record).unwrap();
        let decoded: VectorRecord = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, 1);
    }

    #[test]
    fn test_config_roundtrip() {
        let config = CollectionConfig::new(128);
        let bytes = rmp_serde::to_vec(&config).unwrap();
        let decoded: CollectionConfig = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.dimension, 128);
        assert_eq!(decoded.metric, DistanceMetric::Cosine);
    }

    #[test]
    fn test_distance_metric_from_str() {
        assert_eq!(DistanceMetric::from_str("cosine"), Some(DistanceMetric::Cosine));
        assert_eq!(DistanceMetric::from_str("euclidean"), Some(DistanceMetric::Euclidean));
        assert_eq!(DistanceMetric::from_str("l2"), Some(DistanceMetric::Euclidean));
        assert_eq!(DistanceMetric::from_str("dotproduct"), Some(DistanceMetric::DotProduct));
        assert_eq!(DistanceMetric::from_str("unknown"), None);
    }
}

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use rmpv::Value;

/// A single result produced by the AI query adapter.
#[derive(Debug, Clone)]
pub struct AiQueryResult {
    /// The record data as a MessagePack Value (must be a Map).
    pub data: Value,
    /// AI self-reported confidence for this result (0.0–1.0).
    pub confidence: f64,
    /// Human-readable explanation of where/how the AI found this data.
    pub source_explanation: String,
    /// Optional URI or reference to the external source the AI consulted.
    pub external_source_uri: Option<String>,
    /// Optional key-value tags.
    pub tags: Option<HashMap<String, String>>,
}

/// The outcome of processing a single AI query result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiQueryWriteDecision {
    /// Whether the result was persisted to the database.
    pub persisted: bool,
    /// The record ID assigned if persisted, or None.
    pub record_id: Option<i64>,
    /// The confidence value (AI self-reported).
    pub confidence: f64,
    /// The AI origin tag if persisted.
    pub ai_origin_tag: Option<String>,
    /// Reason the result was not persisted, if applicable.
    pub rejection_reason: Option<String>,
}

/// Supported property types for schema validation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SchemaPropertyType {
    String,
    Integer,
    Float,
    Boolean,
    Array,
    Map,
    Any,
}

impl SchemaPropertyType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "string" => Self::String,
            "integer" => Self::Integer,
            "float" => Self::Float,
            "boolean" => Self::Boolean,
            "array" => Self::Array,
            "map" => Self::Map,
            _ => Self::Any,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::String => "string",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::Boolean => "boolean",
            Self::Array => "array",
            Self::Map => "map",
            Self::Any => "any",
        }
    }
}

/// Schema definition for validating AI query results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiQuerySchema {
    /// Fields that must be present in the data.
    pub required_fields: Vec<String>,
    /// Optional type constraints for fields.
    pub field_types: HashMap<String, SchemaPropertyType>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_property_type_roundtrip() {
        let types = vec![
            SchemaPropertyType::String,
            SchemaPropertyType::Integer,
            SchemaPropertyType::Float,
            SchemaPropertyType::Boolean,
            SchemaPropertyType::Array,
            SchemaPropertyType::Map,
            SchemaPropertyType::Any,
        ];
        for t in types {
            assert_eq!(SchemaPropertyType::from_str(t.as_str()), t);
        }
    }

    #[test]
    fn write_decision_serde_roundtrip() {
        let decision = AiQueryWriteDecision {
            persisted: true,
            record_id: Some(42),
            confidence: 0.92,
            ai_origin_tag: Some("ai-query:products:2025-07-01T12:00:00Z".to_string()),
            rejection_reason: None,
        };
        let bytes = rmp_serde::to_vec(&decision).unwrap();
        let decoded: AiQueryWriteDecision = rmp_serde::from_slice(&bytes).unwrap();
        assert!(decoded.persisted);
        assert_eq!(decoded.record_id, Some(42));
        assert!((decoded.confidence - 0.92).abs() < f64::EPSILON);
        assert!(decoded.ai_origin_tag.is_some());
        assert!(decoded.rejection_reason.is_none());
    }

    #[test]
    fn schema_serde_roundtrip() {
        let mut field_types = HashMap::new();
        field_types.insert("name".to_string(), SchemaPropertyType::String);
        field_types.insert("price".to_string(), SchemaPropertyType::Float);
        let schema = AiQuerySchema {
            required_fields: vec!["name".to_string(), "price".to_string()],
            field_types,
        };
        let bytes = rmp_serde::to_vec(&schema).unwrap();
        let decoded: AiQuerySchema = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.required_fields.len(), 2);
        assert_eq!(decoded.field_types.len(), 2);
    }
}

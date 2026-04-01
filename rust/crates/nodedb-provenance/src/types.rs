use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceEnvelope {
    pub id: i64,
    pub collection: String,
    pub record_id: i64,
    pub confidence_factor: f64,
    pub source_id: String,
    pub source_type: ProvenanceSourceType,
    pub content_hash: String,
    pub created_at_utc: String,
    pub updated_at_utc: String,
    pub pki_signature: Option<String>,
    pub pki_id: Option<String>,
    pub user_id: Option<String>,
    pub verification_status: ProvenanceVerificationStatus,
    #[serde(default)]
    pub ai_augmented: bool,
    #[serde(default)]
    pub ai_raw_confidence: Option<f64>,
    #[serde(default)]
    pub ai_blend_weight_used: Option<f64>,
    #[serde(default)]
    pub ai_reasoning: Option<String>,
    #[serde(default)]
    pub ai_tags: Option<HashMap<String, String>>,
    #[serde(default)]
    pub ai_anomaly_flagged: bool,
    #[serde(default)]
    pub ai_anomaly_severity: Option<AnomalySeverity>,
    #[serde(default)]
    pub ai_originated: bool,
    #[serde(default)]
    pub ai_origin_tag: Option<String>,
    #[serde(default)]
    pub ai_source_explanation: Option<String>,
    #[serde(default)]
    pub ai_external_source_uri: Option<String>,
    #[serde(default)]
    pub checked_at_utc: Option<String>,
    #[serde(default)]
    pub data_updated_at_utc: Option<String>,
    #[serde(default)]
    pub local_id: Option<String>,
    #[serde(default)]
    pub global_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl AnomalySeverity {
    pub fn from_str(s: &str) -> Self {
        match s {
            "low" => Self::Low,
            "medium" => Self::Medium,
            "high" => Self::High,
            "critical" => Self::Critical,
            _ => Self::Low,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProvenanceSourceType {
    Peer,
    Import,
    Model,
    User,
    Sensor,
    AiQuery,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProvenanceVerificationStatus {
    Unverified,
    Verified,
    Failed,
    KeyRequested,
    TrustAll,
}

impl ProvenanceSourceType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "peer" => Self::Peer,
            "import" => Self::Import,
            "model" => Self::Model,
            "user" => Self::User,
            "sensor" => Self::Sensor,
            "ai_query" => Self::AiQuery,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Peer => "peer",
            Self::Import => "import",
            Self::Model => "model",
            Self::User => "user",
            Self::Sensor => "sensor",
            Self::AiQuery => "ai_query",
            Self::Unknown => "unknown",
        }
    }
}

impl ProvenanceVerificationStatus {
    pub fn from_str(s: &str) -> Self {
        match s {
            "unverified" => Self::Unverified,
            "verified" => Self::Verified,
            "failed" => Self::Failed,
            "key_requested" => Self::KeyRequested,
            "trust_all" => Self::TrustAll,
            _ => Self::Unverified,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Unverified => "unverified",
            Self::Verified => "verified",
            Self::Failed => "failed",
            Self::KeyRequested => "key_requested",
            Self::TrustAll => "trust_all",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_type_roundtrip() {
        let types = vec![
            ProvenanceSourceType::Peer,
            ProvenanceSourceType::Import,
            ProvenanceSourceType::Model,
            ProvenanceSourceType::User,
            ProvenanceSourceType::Sensor,
            ProvenanceSourceType::AiQuery,
            ProvenanceSourceType::Unknown,
        ];
        for t in types {
            assert_eq!(ProvenanceSourceType::from_str(t.as_str()), t);
        }
    }

    #[test]
    fn verification_status_roundtrip() {
        let statuses = vec![
            ProvenanceVerificationStatus::Unverified,
            ProvenanceVerificationStatus::Verified,
            ProvenanceVerificationStatus::Failed,
            ProvenanceVerificationStatus::KeyRequested,
            ProvenanceVerificationStatus::TrustAll,
        ];
        for s in statuses {
            assert_eq!(ProvenanceVerificationStatus::from_str(s.as_str()), s);
        }
    }

    #[test]
    fn envelope_serde_roundtrip() {
        let mut tags = HashMap::new();
        tags.insert("model".to_string(), "gpt-4".to_string());
        let envelope = ProvenanceEnvelope {
            id: 1,
            collection: "users".to_string(),
            record_id: 42,
            confidence_factor: 0.85,
            source_id: "user:direct-entry".to_string(),
            source_type: ProvenanceSourceType::User,
            content_hash: "a".repeat(64),
            created_at_utc: "2025-01-01T00:00:00Z".to_string(),
            updated_at_utc: "2025-01-01T00:00:00Z".to_string(),
            pki_signature: None,
            pki_id: None,
            user_id: Some("user-1".to_string()),
            verification_status: ProvenanceVerificationStatus::Unverified,
            ai_augmented: true,
            ai_raw_confidence: Some(0.9),
            ai_blend_weight_used: Some(0.3),
            ai_reasoning: Some("test reasoning".to_string()),
            ai_tags: Some(tags),
            ai_anomaly_flagged: false,
            ai_anomaly_severity: None,
            ai_originated: false,
            ai_origin_tag: None,
            ai_source_explanation: None,
            ai_external_source_uri: None,
            checked_at_utc: None,
            data_updated_at_utc: None,
            local_id: None,
            global_id: None,
        };
        let bytes = rmp_serde::to_vec(&envelope).unwrap();
        let decoded: ProvenanceEnvelope = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.collection, "users");
        assert_eq!(decoded.record_id, 42);
        assert!((decoded.confidence_factor - 0.85).abs() < f64::EPSILON);
        assert_eq!(decoded.source_type, ProvenanceSourceType::User);
        assert_eq!(decoded.verification_status, ProvenanceVerificationStatus::Unverified);
        assert_eq!(decoded.user_id, Some("user-1".to_string()));
        assert!(decoded.pki_signature.is_none());
        assert!(decoded.ai_augmented);
        assert!((decoded.ai_raw_confidence.unwrap() - 0.9).abs() < f64::EPSILON);
        assert!((decoded.ai_blend_weight_used.unwrap() - 0.3).abs() < f64::EPSILON);
        assert_eq!(decoded.ai_reasoning, Some("test reasoning".to_string()));
        assert_eq!(decoded.ai_tags.unwrap().get("model").unwrap(), "gpt-4");
        assert!(!decoded.ai_anomaly_flagged);
        assert!(decoded.ai_anomaly_severity.is_none());
        assert!(!decoded.ai_originated);
        assert!(decoded.ai_origin_tag.is_none());
        assert!(decoded.ai_source_explanation.is_none());
        assert!(decoded.ai_external_source_uri.is_none());
        assert!(decoded.checked_at_utc.is_none());
        assert!(decoded.data_updated_at_utc.is_none());
        assert!(decoded.local_id.is_none());
        assert!(decoded.global_id.is_none());
    }

    #[test]
    fn anomaly_severity_roundtrip() {
        let severities = vec![
            AnomalySeverity::Low,
            AnomalySeverity::Medium,
            AnomalySeverity::High,
            AnomalySeverity::Critical,
        ];
        for s in severities {
            assert_eq!(AnomalySeverity::from_str(s.as_str()), s);
        }
    }

    #[test]
    fn backward_compat_old_envelope_deserializes() {
        // Simulate an envelope serialized BEFORE AI fields existed (13 fields only)
        use rmp_serde;
        #[derive(Serialize)]
        struct OldEnvelope {
            id: i64,
            collection: String,
            record_id: i64,
            confidence_factor: f64,
            source_id: String,
            source_type: ProvenanceSourceType,
            content_hash: String,
            created_at_utc: String,
            updated_at_utc: String,
            pki_signature: Option<String>,
            pki_id: Option<String>,
            user_id: Option<String>,
            verification_status: ProvenanceVerificationStatus,
        }
        let old = OldEnvelope {
            id: 1,
            collection: "users".to_string(),
            record_id: 42,
            confidence_factor: 0.85,
            source_id: "user:alice".to_string(),
            source_type: ProvenanceSourceType::User,
            content_hash: "a".repeat(64),
            created_at_utc: "2025-01-01T00:00:00Z".to_string(),
            updated_at_utc: "2025-01-01T00:00:00Z".to_string(),
            pki_signature: None,
            pki_id: None,
            user_id: None,
            verification_status: ProvenanceVerificationStatus::Unverified,
        };
        let bytes = rmp_serde::to_vec(&old).unwrap();
        let decoded: ProvenanceEnvelope = rmp_serde::from_slice(&bytes).unwrap();
        // AI fields should all default
        assert!(!decoded.ai_augmented);
        assert!(decoded.ai_raw_confidence.is_none());
        assert!(decoded.ai_blend_weight_used.is_none());
        assert!(decoded.ai_reasoning.is_none());
        assert!(decoded.ai_tags.is_none());
        assert!(!decoded.ai_anomaly_flagged);
        assert!(decoded.ai_anomaly_severity.is_none());
        assert!(!decoded.ai_originated);
        assert!(decoded.ai_origin_tag.is_none());
        assert!(decoded.ai_source_explanation.is_none());
        assert!(decoded.ai_external_source_uri.is_none());
        assert!(decoded.checked_at_utc.is_none());
        assert!(decoded.data_updated_at_utc.is_none());
        assert!(decoded.local_id.is_none());
        assert!(decoded.global_id.is_none());
    }

    #[test]
    fn envelope_lifecycle_fields_roundtrip() {
        let envelope = ProvenanceEnvelope {
            id: 1,
            collection: "users".to_string(),
            record_id: 42,
            confidence_factor: 0.85,
            source_id: "user:alice".to_string(),
            source_type: ProvenanceSourceType::User,
            content_hash: "a".repeat(64),
            created_at_utc: "2025-01-01T00:00:00Z".to_string(),
            updated_at_utc: "2025-01-01T00:00:00Z".to_string(),
            pki_signature: None,
            pki_id: None,
            user_id: None,
            verification_status: ProvenanceVerificationStatus::Unverified,
            ai_augmented: false,
            ai_raw_confidence: None,
            ai_blend_weight_used: None,
            ai_reasoning: None,
            ai_tags: None,
            ai_anomaly_flagged: false,
            ai_anomaly_severity: None,
            ai_originated: false,
            ai_origin_tag: None,
            ai_source_explanation: None,
            ai_external_source_uri: None,
            checked_at_utc: Some("2025-06-01T12:00:00Z".to_string()),
            data_updated_at_utc: Some("2025-06-01T11:00:00Z".to_string()),
            local_id: Some("local-abc-123".to_string()),
            global_id: Some("global-xyz-789".to_string()),
        };
        let bytes = rmp_serde::to_vec(&envelope).unwrap();
        let decoded: ProvenanceEnvelope = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.checked_at_utc, Some("2025-06-01T12:00:00Z".to_string()));
        assert_eq!(decoded.data_updated_at_utc, Some("2025-06-01T11:00:00Z".to_string()));
        assert_eq!(decoded.local_id, Some("local-abc-123".to_string()));
        assert_eq!(decoded.global_id, Some("global-xyz-789".to_string()));
    }
}

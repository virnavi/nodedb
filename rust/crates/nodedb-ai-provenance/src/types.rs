use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictPreference {
    PreferA,
    PreferB,
    PreferNeither,
}

impl ConflictPreference {
    pub fn from_str(s: &str) -> Self {
        match s {
            "prefer_a" => Self::PreferA,
            "prefer_b" => Self::PreferB,
            "prefer_neither" => Self::PreferNeither,
            _ => Self::PreferNeither,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::PreferA => "prefer_a",
            Self::PreferB => "prefer_b",
            Self::PreferNeither => "prefer_neither",
        }
    }
}

/// AI-suggested confidence assessment for a provenance envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProvenanceAssessment {
    pub envelope_id: i64,
    pub suggested_confidence: f64,
    pub source_type: Option<String>,
    pub reasoning: Option<String>,
    pub tags: Option<HashMap<String, String>>,
}

/// AI-suggested conflict resolution between two provenance envelopes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConflictResolution {
    pub envelope_id_a: i64,
    pub envelope_id_b: i64,
    pub confidence_delta_a: f64,
    pub confidence_delta_b: f64,
    pub preference: ConflictPreference,
    pub reasoning: Option<String>,
}

/// AI anomaly flag for a specific record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAnomalyFlag {
    pub record_id: i64,
    pub confidence_penalty: f64,
    pub reason: Option<String>,
    pub severity: String,
}

/// AI source classification for a provenance envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSourceClassification {
    pub envelope_id: i64,
    pub source_type: String,
    pub credibility_prior: f64,
    pub reasoning: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflict_preference_roundtrip() {
        let prefs = vec![
            ConflictPreference::PreferA,
            ConflictPreference::PreferB,
            ConflictPreference::PreferNeither,
        ];
        for p in prefs {
            assert_eq!(ConflictPreference::from_str(p.as_str()), p);
        }
    }

    #[test]
    fn assessment_serde_roundtrip() {
        let mut tags = HashMap::new();
        tags.insert("model".to_string(), "gpt-4".to_string());
        let assessment = AiProvenanceAssessment {
            envelope_id: 1,
            suggested_confidence: 0.9,
            source_type: Some("user".to_string()),
            reasoning: Some("high quality".to_string()),
            tags: Some(tags),
        };
        let bytes = rmp_serde::to_vec(&assessment).unwrap();
        let decoded: AiProvenanceAssessment = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.envelope_id, 1);
        assert!((decoded.suggested_confidence - 0.9).abs() < f64::EPSILON);
        assert_eq!(decoded.source_type, Some("user".to_string()));
        assert_eq!(decoded.tags.unwrap().get("model").unwrap(), "gpt-4");
    }

    #[test]
    fn conflict_resolution_serde_roundtrip() {
        let resolution = AiConflictResolution {
            envelope_id_a: 1,
            envelope_id_b: 2,
            confidence_delta_a: -0.1,
            confidence_delta_b: 0.05,
            preference: ConflictPreference::PreferA,
            reasoning: Some("newer data".to_string()),
        };
        let bytes = rmp_serde::to_vec(&resolution).unwrap();
        let decoded: AiConflictResolution = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.envelope_id_a, 1);
        assert_eq!(decoded.envelope_id_b, 2);
        assert!((decoded.confidence_delta_a - (-0.1)).abs() < f64::EPSILON);
        assert_eq!(decoded.preference, ConflictPreference::PreferA);
    }

    #[test]
    fn anomaly_flag_serde_roundtrip() {
        let flag = AiAnomalyFlag {
            record_id: 42,
            confidence_penalty: 0.2,
            reason: Some("suspicious pattern".to_string()),
            severity: "high".to_string(),
        };
        let bytes = rmp_serde::to_vec(&flag).unwrap();
        let decoded: AiAnomalyFlag = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.record_id, 42);
        assert!((decoded.confidence_penalty - 0.2).abs() < f64::EPSILON);
        assert_eq!(decoded.severity, "high");
    }

    #[test]
    fn source_classification_serde_roundtrip() {
        let classification = AiSourceClassification {
            envelope_id: 1,
            source_type: "model".to_string(),
            credibility_prior: 0.5,
            reasoning: Some("classified by AI".to_string()),
        };
        let bytes = rmp_serde::to_vec(&classification).unwrap();
        let decoded: AiSourceClassification = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.envelope_id, 1);
        assert_eq!(decoded.source_type, "model");
        assert!((decoded.credibility_prior - 0.5).abs() < f64::EPSILON);
    }
}

use std::sync::Arc;

use chrono::Utc;

use nodedb_provenance::{ProvenanceEngine, ProvenanceSourceType, AnomalySeverity};

use crate::blending::{blend_confidence, apply_verification_boost_post_blend, apply_anomaly_penalty, apply_conflict_deltas};
use crate::config::AiProvenanceConfig;
use crate::error::AiProvenanceError;
use crate::types::{AiProvenanceAssessment, AiConflictResolution, AiAnomalyFlag, AiSourceClassification};

pub struct AiProvenanceEngine {
    provenance: Arc<ProvenanceEngine>,
    config: AiProvenanceConfig,
}

impl AiProvenanceEngine {
    pub fn new(provenance: Arc<ProvenanceEngine>, config: AiProvenanceConfig) -> Self {
        AiProvenanceEngine { provenance, config }
    }

    pub fn config(&self) -> &AiProvenanceConfig {
        &self.config
    }

    /// Apply an AI-suggested confidence assessment to an envelope.
    /// Blends the deterministic confidence with the AI suggestion using the configured weight.
    pub fn apply_assessment(
        &self,
        assessment: &AiProvenanceAssessment,
    ) -> Result<(), AiProvenanceError> {
        let mut envelope = self.provenance.get(assessment.envelope_id)?
            .ok_or(AiProvenanceError::EnvelopeNotFound(assessment.envelope_id))?;

        if !self.config.is_collection_enabled(&envelope.collection) {
            return Err(AiProvenanceError::CollectionNotEnabled(envelope.collection.clone()));
        }

        let verification_failed = envelope.verification_status == nodedb_provenance::ProvenanceVerificationStatus::Failed;
        let blended = blend_confidence(
            envelope.confidence_factor,
            assessment.suggested_confidence,
            self.config.ai_blend_weight,
            verification_failed,
        );
        let final_conf = apply_verification_boost_post_blend(blended, &envelope.verification_status);

        envelope.confidence_factor = final_conf;
        envelope.ai_augmented = true;
        envelope.ai_raw_confidence = Some(assessment.suggested_confidence);
        envelope.ai_blend_weight_used = Some(self.config.ai_blend_weight);
        envelope.ai_reasoning = assessment.reasoning.clone();
        envelope.ai_tags = assessment.tags.clone();
        envelope.updated_at_utc = Utc::now().to_rfc3339();

        if let Some(ref st) = assessment.source_type {
            envelope.source_type = ProvenanceSourceType::from_str(st);
        }

        self.provenance.update_envelope(&envelope)?;
        Ok(())
    }

    /// Apply AI-suggested conflict resolution between two envelopes.
    pub fn apply_conflict_resolution(
        &self,
        resolution: &AiConflictResolution,
    ) -> Result<(), AiProvenanceError> {
        let mut env_a = self.provenance.get(resolution.envelope_id_a)?
            .ok_or(AiProvenanceError::EnvelopeNotFound(resolution.envelope_id_a))?;
        let mut env_b = self.provenance.get(resolution.envelope_id_b)?
            .ok_or(AiProvenanceError::EnvelopeNotFound(resolution.envelope_id_b))?;

        let (new_a, new_b) = apply_conflict_deltas(
            env_a.confidence_factor,
            env_b.confidence_factor,
            resolution.confidence_delta_a,
            resolution.confidence_delta_b,
        );

        env_a.confidence_factor = new_a;
        env_a.ai_augmented = true;
        env_a.ai_reasoning = resolution.reasoning.clone();
        env_a.updated_at_utc = Utc::now().to_rfc3339();

        env_b.confidence_factor = new_b;
        env_b.ai_augmented = true;
        env_b.ai_reasoning = resolution.reasoning.clone();
        env_b.updated_at_utc = Utc::now().to_rfc3339();

        self.provenance.update_envelope(&env_a)?;
        self.provenance.update_envelope(&env_b)?;
        Ok(())
    }

    /// Apply AI anomaly flags to envelopes for records in a collection.
    pub fn apply_anomaly_flags(
        &self,
        collection: &str,
        flags: &[AiAnomalyFlag],
    ) -> Result<u32, AiProvenanceError> {
        if !self.config.is_collection_enabled(collection) {
            return Err(AiProvenanceError::CollectionNotEnabled(collection.to_string()));
        }

        let mut affected = 0u32;

        for flag in flags {
            let envelopes = self.provenance.get_for_record(collection, flag.record_id)?;
            for mut env in envelopes {
                let new_conf = apply_anomaly_penalty(env.confidence_factor, flag.confidence_penalty);
                env.confidence_factor = new_conf;
                env.ai_augmented = true;
                env.ai_anomaly_flagged = true;
                env.ai_anomaly_severity = Some(AnomalySeverity::from_str(&flag.severity));
                env.ai_reasoning = flag.reason.clone();
                env.updated_at_utc = Utc::now().to_rfc3339();
                self.provenance.update_envelope(&env)?;
                affected += 1;
            }
        }

        Ok(affected)
    }

    /// Apply AI source classification to an envelope.
    pub fn apply_source_classification(
        &self,
        classification: &AiSourceClassification,
    ) -> Result<(), AiProvenanceError> {
        let mut envelope = self.provenance.get(classification.envelope_id)?
            .ok_or(AiProvenanceError::EnvelopeNotFound(classification.envelope_id))?;

        envelope.source_type = ProvenanceSourceType::from_str(&classification.source_type);
        envelope.ai_augmented = true;
        envelope.ai_reasoning = classification.reasoning.clone();
        envelope.updated_at_utc = Utc::now().to_rfc3339();

        // Re-blend using credibility_prior as the AI suggested confidence
        let verification_failed = envelope.verification_status == nodedb_provenance::ProvenanceVerificationStatus::Failed;
        let blended = blend_confidence(
            envelope.confidence_factor,
            classification.credibility_prior,
            self.config.ai_blend_weight,
            verification_failed,
        );
        envelope.confidence_factor = apply_verification_boost_post_blend(blended, &envelope.verification_status);
        envelope.ai_raw_confidence = Some(classification.credibility_prior);
        envelope.ai_blend_weight_used = Some(self.config.ai_blend_weight);

        self.provenance.update_envelope(&envelope)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use nodedb_provenance::ProvenanceSourceType;
    use nodedb_storage::StorageEngine;
    use tempfile::TempDir;

    fn setup() -> (TempDir, Arc<ProvenanceEngine>, AiProvenanceEngine) {
        let dir = TempDir::new().unwrap();
        let storage = Arc::new(StorageEngine::open(dir.path()).unwrap());
        let prov = Arc::new(ProvenanceEngine::new(storage).unwrap());
        let config = AiProvenanceConfig {
            ai_blend_weight: 0.3,
            ..Default::default()
        };
        let ai_engine = AiProvenanceEngine::new(Arc::clone(&prov), config);
        (dir, prov, ai_engine)
    }

    #[test]
    fn apply_assessment_blends_confidence() {
        let (_dir, prov, ai_engine) = setup();
        prov.attach(
            "users", 1, "src", ProvenanceSourceType::User,
            "a".repeat(64), None, None, None, false, 0, None, None, None, None,
        ).unwrap();

        // Deterministic confidence for User unsigned 0-hop = 0.85
        let assessment = AiProvenanceAssessment {
            envelope_id: 1,
            suggested_confidence: 0.9,
            source_type: None,
            reasoning: Some("high quality".to_string()),
            tags: None,
        };
        ai_engine.apply_assessment(&assessment).unwrap();

        let env = prov.get(1).unwrap().unwrap();
        // blend(0.85, 0.9, 0.3, false) = 0.85*0.7 + 0.9*0.3 = 0.595+0.27 = 0.865
        assert!((env.confidence_factor - 0.865).abs() < 1e-10);
        assert!(env.ai_augmented);
        assert!((env.ai_raw_confidence.unwrap() - 0.9).abs() < f64::EPSILON);
        assert!((env.ai_blend_weight_used.unwrap() - 0.3).abs() < f64::EPSILON);
        assert_eq!(env.ai_reasoning, Some("high quality".to_string()));
    }

    #[test]
    fn apply_assessment_with_source_type_override() {
        let (_dir, prov, ai_engine) = setup();
        prov.attach(
            "users", 1, "src", ProvenanceSourceType::Unknown,
            "a".repeat(64), None, None, None, false, 0, None, None, None, None,
        ).unwrap();

        let assessment = AiProvenanceAssessment {
            envelope_id: 1,
            suggested_confidence: 0.8,
            source_type: Some("model".to_string()),
            reasoning: None,
            tags: None,
        };
        ai_engine.apply_assessment(&assessment).unwrap();

        let env = prov.get(1).unwrap().unwrap();
        assert_eq!(env.source_type, ProvenanceSourceType::Model);
    }

    #[test]
    fn apply_assessment_with_tags() {
        let (_dir, prov, ai_engine) = setup();
        prov.attach(
            "users", 1, "src", ProvenanceSourceType::User,
            "a".repeat(64), None, None, None, false, 0, None, None, None, None,
        ).unwrap();

        let mut tags = HashMap::new();
        tags.insert("model".to_string(), "gpt-4".to_string());
        let assessment = AiProvenanceAssessment {
            envelope_id: 1,
            suggested_confidence: 0.9,
            source_type: None,
            reasoning: None,
            tags: Some(tags),
        };
        ai_engine.apply_assessment(&assessment).unwrap();

        let env = prov.get(1).unwrap().unwrap();
        assert_eq!(env.ai_tags.unwrap().get("model").unwrap(), "gpt-4");
    }

    #[test]
    fn apply_assessment_envelope_not_found() {
        let (_dir, _prov, ai_engine) = setup();
        let assessment = AiProvenanceAssessment {
            envelope_id: 999,
            suggested_confidence: 0.9,
            source_type: None,
            reasoning: None,
            tags: None,
        };
        assert!(ai_engine.apply_assessment(&assessment).is_err());
    }

    #[test]
    fn apply_conflict_resolution() {
        let (_dir, prov, ai_engine) = setup();
        prov.attach(
            "users", 1, "src1", ProvenanceSourceType::User,
            "a".repeat(64), None, None, None, false, 0, None, None, None, None,
        ).unwrap();
        prov.attach(
            "users", 2, "src2", ProvenanceSourceType::Peer,
            "b".repeat(64), None, None, None, false, 1, None, None, None, None,
        ).unwrap();

        let resolution = AiConflictResolution {
            envelope_id_a: 1,
            envelope_id_b: 2,
            confidence_delta_a: -0.1,
            confidence_delta_b: 0.05,
            preference: crate::types::ConflictPreference::PreferA,
            reasoning: Some("newer data".to_string()),
        };
        ai_engine.apply_conflict_resolution(&resolution).unwrap();

        let env_a = prov.get(1).unwrap().unwrap();
        let env_b = prov.get(2).unwrap().unwrap();
        // User unsigned 0-hop = 0.85, delta -0.1 => 0.75
        assert!((env_a.confidence_factor - 0.75).abs() < 1e-10);
        // Peer unsigned 1-hop = 0.60, delta +0.05 => 0.65
        assert!((env_b.confidence_factor - 0.65).abs() < 1e-10);
        assert!(env_a.ai_augmented);
        assert!(env_b.ai_augmented);
    }

    #[test]
    fn apply_anomaly_flags() {
        let (_dir, prov, ai_engine) = setup();
        prov.attach(
            "users", 42, "src1", ProvenanceSourceType::User,
            "a".repeat(64), None, None, None, false, 0, None, None, None, None,
        ).unwrap();
        prov.attach(
            "users", 42, "src2", ProvenanceSourceType::Peer,
            "b".repeat(64), None, None, None, false, 1, None, None, None, None,
        ).unwrap();

        let flags = vec![AiAnomalyFlag {
            record_id: 42,
            confidence_penalty: 0.2,
            reason: Some("anomaly detected".to_string()),
            severity: "high".to_string(),
        }];
        let affected = ai_engine.apply_anomaly_flags("users", &flags).unwrap();
        assert_eq!(affected, 2);

        let env1 = prov.get(1).unwrap().unwrap();
        // User unsigned 0-hop=0.85, penalty=0.2 => 0.65
        assert!((env1.confidence_factor - 0.65).abs() < 1e-10);
        assert!(env1.ai_anomaly_flagged);
        assert_eq!(env1.ai_anomaly_severity, Some(AnomalySeverity::High));
    }

    #[test]
    fn apply_anomaly_flags_collection_not_enabled() {
        let dir = TempDir::new().unwrap();
        let storage = Arc::new(StorageEngine::open(dir.path()).unwrap());
        let prov = Arc::new(ProvenanceEngine::new(storage).unwrap());
        let config = AiProvenanceConfig {
            enabled_collections: vec!["users".to_string()],
            ..Default::default()
        };
        let ai_engine = AiProvenanceEngine::new(Arc::clone(&prov), config);

        let flags = vec![AiAnomalyFlag {
            record_id: 1,
            confidence_penalty: 0.2,
            reason: None,
            severity: "low".to_string(),
        }];
        assert!(ai_engine.apply_anomaly_flags("posts", &flags).is_err());
    }

    #[test]
    fn apply_source_classification() {
        let (_dir, prov, ai_engine) = setup();
        prov.attach(
            "users", 1, "src", ProvenanceSourceType::Unknown,
            "a".repeat(64), None, None, None, false, 0, None, None, None, None,
        ).unwrap();

        let classification = AiSourceClassification {
            envelope_id: 1,
            source_type: "sensor".to_string(),
            credibility_prior: 0.7,
            reasoning: Some("classified as sensor".to_string()),
        };
        ai_engine.apply_source_classification(&classification).unwrap();

        let env = prov.get(1).unwrap().unwrap();
        assert_eq!(env.source_type, ProvenanceSourceType::Sensor);
        assert!(env.ai_augmented);
        assert!((env.ai_raw_confidence.unwrap() - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn config_accessor() {
        let (_dir, _prov, ai_engine) = setup();
        assert!((ai_engine.config().ai_blend_weight - 0.3).abs() < f64::EPSILON);
    }
}

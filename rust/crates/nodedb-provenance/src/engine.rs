use std::sync::Arc;

use chrono::Utc;
use rmpv::Value;

use nodedb_storage::{StorageEngine, StorageTree, IdGenerator, encode_id, decode_id, to_msgpack, from_msgpack};

use crate::error::ProvenanceError;
use crate::types::{ProvenanceEnvelope, ProvenanceSourceType, ProvenanceVerificationStatus};
use crate::confidence;
use crate::content_hash::compute_content_hash;
use crate::verification;

const ENVELOPES_TREE: &str = "__provenance_envelopes__";
const BY_RECORD_TREE: &str = "__provenance_by_record__";

pub struct ProvenanceEngine {
    #[allow(dead_code)]
    engine: Arc<StorageEngine>,
    envelopes: StorageTree,
    by_record: StorageTree,
    pub(crate) id_gen: Arc<IdGenerator>,
}

impl ProvenanceEngine {
    pub fn new(engine: Arc<StorageEngine>) -> Result<Self, ProvenanceError> {
        let envelopes = engine.open_tree(ENVELOPES_TREE)?;
        let by_record = engine.open_tree(BY_RECORD_TREE)?;
        let id_gen = Arc::new(IdGenerator::new(&engine)?);
        Ok(ProvenanceEngine { engine, envelopes, by_record, id_gen })
    }

    /// Attach a provenance envelope to a record.
    pub fn attach(
        &self,
        collection: &str,
        record_id: i64,
        source_id: &str,
        source_type: ProvenanceSourceType,
        content_hash: String,
        pki_signature: Option<String>,
        pki_id: Option<String>,
        user_id: Option<String>,
        is_signed: bool,
        hops: u8,
        created_at_utc: Option<String>,
        data_updated_at_utc: Option<String>,
        local_id: Option<String>,
        global_id: Option<String>,
    ) -> Result<ProvenanceEnvelope, ProvenanceError> {
        let id = self.id_gen.next_id("provenance")?;
        let now = created_at_utc.unwrap_or_else(|| Utc::now().to_rfc3339());
        let conf = confidence::initial_confidence(&source_type, is_signed, hops);

        let envelope = ProvenanceEnvelope {
            id,
            collection: collection.to_string(),
            record_id,
            confidence_factor: conf,
            source_id: source_id.to_string(),
            source_type,
            content_hash,
            created_at_utc: now.clone(),
            updated_at_utc: now,
            pki_signature,
            pki_id,
            user_id,
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
            checked_at_utc: None,
            data_updated_at_utc,
            local_id,
            global_id,
        };

        let bytes = to_msgpack(&envelope)?;
        self.envelopes.insert(&encode_id(id), &bytes)?;

        // Index: collection + record_id + envelope_id -> empty
        let index_key = self.make_record_index_key(collection, record_id, id);
        self.by_record.insert(&index_key, &[])?;

        Ok(envelope)
    }

    /// Get a provenance envelope by ID.
    pub fn get(&self, id: i64) -> Result<Option<ProvenanceEnvelope>, ProvenanceError> {
        match self.envelopes.get(&encode_id(id))? {
            Some(bytes) => {
                let envelope: ProvenanceEnvelope = from_msgpack(&bytes)?;
                Ok(Some(envelope))
            }
            None => Ok(None),
        }
    }

    /// Get all provenance envelopes for a specific record.
    pub fn get_for_record(
        &self,
        collection: &str,
        record_id: i64,
    ) -> Result<Vec<ProvenanceEnvelope>, ProvenanceError> {
        let prefix = self.make_record_prefix(collection, record_id);
        let mut results = Vec::new();

        for entry in self.by_record.scan_prefix(&prefix) {
            let (key, _) = entry?;
            // Last 8 bytes of key are the envelope ID
            if key.len() >= 8 {
                let env_id_bytes = &key[key.len() - 8..];
                let env_id = decode_id(env_id_bytes)?;
                if let Some(env) = self.get(env_id)? {
                    results.push(env);
                }
            }
        }

        Ok(results)
    }

    /// Update the confidence factor of an envelope.
    pub fn update_confidence(
        &self,
        id: i64,
        new_confidence: f64,
    ) -> Result<ProvenanceEnvelope, ProvenanceError> {
        if !(0.0..=1.0).contains(&new_confidence) {
            return Err(ProvenanceError::InvalidConfidence(
                format!("confidence must be in [0.0, 1.0], got {}", new_confidence),
            ));
        }

        let mut envelope = self.get(id)?
            .ok_or(ProvenanceError::NotFound(id))?;

        envelope.confidence_factor = new_confidence;
        envelope.updated_at_utc = Utc::now().to_rfc3339();

        let bytes = to_msgpack(&envelope)?;
        self.envelopes.insert(&encode_id(id), &bytes)?;

        Ok(envelope)
    }

    /// Persist a full envelope (used by AI provenance to write AI fields).
    pub fn update_envelope(
        &self,
        envelope: &ProvenanceEnvelope,
    ) -> Result<(), ProvenanceError> {
        if !(0.0..=1.0).contains(&envelope.confidence_factor) {
            return Err(ProvenanceError::InvalidConfidence(
                format!("confidence must be in [0.0, 1.0], got {}", envelope.confidence_factor),
            ));
        }
        // Verify envelope exists
        if self.envelopes.get(&encode_id(envelope.id))?.is_none() {
            return Err(ProvenanceError::NotFound(envelope.id));
        }
        let bytes = to_msgpack(envelope)?;
        self.envelopes.insert(&encode_id(envelope.id), &bytes)?;
        Ok(())
    }

    /// Apply corroboration (Noisy-OR) to an envelope's confidence.
    pub fn corroborate(
        &self,
        id: i64,
        new_source_confidence: f64,
    ) -> Result<ProvenanceEnvelope, ProvenanceError> {
        let envelope = self.get(id)?
            .ok_or(ProvenanceError::NotFound(id))?;

        let new_conf = confidence::corroborate(envelope.confidence_factor, new_source_confidence);
        self.update_confidence(id, new_conf)
    }

    /// Verify the signature on an envelope and update its status.
    pub fn verify(
        &self,
        id: i64,
        public_key_bytes: &[u8],
    ) -> Result<ProvenanceEnvelope, ProvenanceError> {
        let mut envelope = self.get(id)?
            .ok_or(ProvenanceError::NotFound(id))?;

        match verification::verify_signature(&envelope, public_key_bytes) {
            Ok(true) => {
                envelope.verification_status = ProvenanceVerificationStatus::Verified;
                envelope.confidence_factor = confidence::verification_boost(envelope.confidence_factor);
            }
            Ok(false) => {
                // No signature present — stays unverified
                return Ok(envelope);
            }
            Err(_) => {
                envelope.verification_status = ProvenanceVerificationStatus::Failed;
                envelope.confidence_factor = confidence::verification_failure();
            }
        }

        let now = Utc::now().to_rfc3339();
        envelope.updated_at_utc = now.clone();
        envelope.checked_at_utc = Some(now);
        let bytes = to_msgpack(&envelope)?;
        self.envelopes.insert(&encode_id(id), &bytes)?;

        Ok(envelope)
    }

    /// Delete a provenance envelope.
    pub fn delete(&self, id: i64) -> Result<bool, ProvenanceError> {
        let envelope = match self.get(id)? {
            Some(e) => e,
            None => return Ok(false),
        };

        self.envelopes.remove(&encode_id(id))?;

        let index_key = self.make_record_index_key(&envelope.collection, envelope.record_id, id);
        self.by_record.remove(&index_key)?;

        Ok(true)
    }

    /// Query envelopes with optional filters.
    pub fn query(
        &self,
        collection: Option<&str>,
        source_type: Option<&ProvenanceSourceType>,
        verification_status: Option<&ProvenanceVerificationStatus>,
        min_confidence: Option<f64>,
    ) -> Result<Vec<ProvenanceEnvelope>, ProvenanceError> {
        let mut results = Vec::new();

        for entry in self.envelopes.iter() {
            let (_, bytes) = entry?;
            let envelope: ProvenanceEnvelope = from_msgpack(&bytes)?;

            if let Some(c) = collection {
                if envelope.collection != c {
                    continue;
                }
            }
            if let Some(st) = source_type {
                if &envelope.source_type != st {
                    continue;
                }
            }
            if let Some(vs) = verification_status {
                if &envelope.verification_status != vs {
                    continue;
                }
            }
            if let Some(mc) = min_confidence {
                if envelope.confidence_factor < mc {
                    continue;
                }
            }

            results.push(envelope);
        }

        Ok(results)
    }

    /// Count total provenance envelopes.
    pub fn envelope_count(&self) -> Result<usize, ProvenanceError> {
        Ok(self.envelopes.len())
    }

    /// Compute content hash for arbitrary data (public API).
    pub fn compute_hash(&self, data: &Value) -> Result<String, ProvenanceError> {
        compute_content_hash(data)
    }

    // --- Internal helpers ---

    fn make_record_prefix(&self, collection: &str, record_id: i64) -> Vec<u8> {
        let mut key = Vec::new();
        key.extend_from_slice(collection.as_bytes());
        key.push(0); // null separator
        key.extend_from_slice(&encode_id(record_id));
        key
    }

    fn make_record_index_key(&self, collection: &str, record_id: i64, envelope_id: i64) -> Vec<u8> {
        let mut key = self.make_record_prefix(collection, record_id);
        key.extend_from_slice(&encode_id(envelope_id));
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, ProvenanceEngine) {
        let dir = TempDir::new().unwrap();
        let storage = Arc::new(StorageEngine::open(dir.path()).unwrap());
        let engine = ProvenanceEngine::new(storage).unwrap();
        (dir, engine)
    }

    #[test]
    fn attach_and_get() {
        let (_dir, engine) = setup();
        let env = engine.attach(
            "users", 42, "user:alice", ProvenanceSourceType::User,
            "a".repeat(64), None, None, Some("alice".to_string()), false, 0, None, None, None, None,
        ).unwrap();

        assert_eq!(env.id, 1);
        assert_eq!(env.collection, "users");
        assert_eq!(env.record_id, 42);
        assert!((env.confidence_factor - 0.85).abs() < f64::EPSILON);
        assert_eq!(env.source_type, ProvenanceSourceType::User);

        let fetched = engine.get(1).unwrap().unwrap();
        assert_eq!(fetched.id, 1);
        assert_eq!(fetched.collection, "users");
    }

    #[test]
    fn get_nonexistent() {
        let (_dir, engine) = setup();
        assert!(engine.get(999).unwrap().is_none());
    }

    #[test]
    fn get_for_record() {
        let (_dir, engine) = setup();
        engine.attach(
            "users", 1, "src1", ProvenanceSourceType::User,
            "a".repeat(64), None, None, None, false, 0, None, None, None, None,
        ).unwrap();
        engine.attach(
            "users", 1, "src2", ProvenanceSourceType::Peer,
            "b".repeat(64), None, None, None, false, 1, None, None, None, None,
        ).unwrap();
        engine.attach(
            "users", 2, "src3", ProvenanceSourceType::Import,
            "c".repeat(64), None, None, None, false, 0, None, None, None, None,
        ).unwrap();

        let results = engine.get_for_record("users", 1).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn update_confidence() {
        let (_dir, engine) = setup();
        engine.attach(
            "test", 1, "src", ProvenanceSourceType::User,
            "a".repeat(64), None, None, None, false, 0, None, None, None, None,
        ).unwrap();

        let updated = engine.update_confidence(1, 0.50).unwrap();
        assert!((updated.confidence_factor - 0.50).abs() < f64::EPSILON);
    }

    #[test]
    fn update_confidence_out_of_range() {
        let (_dir, engine) = setup();
        engine.attach(
            "test", 1, "src", ProvenanceSourceType::User,
            "a".repeat(64), None, None, None, false, 0, None, None, None, None,
        ).unwrap();

        assert!(engine.update_confidence(1, 1.5).is_err());
        assert!(engine.update_confidence(1, -0.1).is_err());
    }

    #[test]
    fn corroborate_envelope() {
        let (_dir, engine) = setup();
        engine.attach(
            "test", 1, "src", ProvenanceSourceType::Peer,
            "a".repeat(64), None, None, None, false, 1, None, None, None, None,
        ).unwrap();

        let original = engine.get(1).unwrap().unwrap();
        let corroborated = engine.corroborate(1, 0.70).unwrap();
        assert!(corroborated.confidence_factor > original.confidence_factor);
    }

    #[test]
    fn verify_updates_status() {
        let (_dir, engine) = setup();
        use nodedb_crypto::NodeIdentity;

        let identity = NodeIdentity::generate();
        let content_hash = "a".repeat(64);
        let created_at = "2025-01-01T00:00:00Z";
        let pki_id = "test-pki";
        let user_id = "test-user";

        let payload = crate::verification::build_signature_payload(
            &content_hash, created_at, pki_id, user_id,
        );
        let sig_bytes = identity.sign(&payload);
        let sig_hex: String = sig_bytes.iter().map(|b| format!("{:02x}", b)).collect();

        // Manually insert with controlled timestamp for signature verification
        let _ = engine.id_gen.next_id("provenance").unwrap(); // consume id 1
        let env = ProvenanceEnvelope {
            id: 1,
            collection: "test".to_string(),
            record_id: 1,
            confidence_factor: 0.85,
            source_id: "user:test".to_string(),
            source_type: ProvenanceSourceType::User,
            content_hash,
            created_at_utc: created_at.to_string(),
            updated_at_utc: created_at.to_string(),
            pki_signature: Some(sig_hex),
            pki_id: Some(pki_id.to_string()),
            user_id: Some(user_id.to_string()),
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
            checked_at_utc: None,
            data_updated_at_utc: None,
            local_id: None,
            global_id: None,
        };
        let bytes = to_msgpack(&env).unwrap();
        engine.envelopes.insert(&encode_id(1), &bytes).unwrap();

        let verified = engine.verify(1, &identity.verifying_key_bytes()).unwrap();
        assert_eq!(verified.verification_status, ProvenanceVerificationStatus::Verified);
        assert!((verified.confidence_factor - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn verify_wrong_key_sets_failed() {
        let (_dir, engine) = setup();
        use nodedb_crypto::NodeIdentity;

        let identity1 = NodeIdentity::generate();
        let identity2 = NodeIdentity::generate();

        let payload = crate::verification::build_signature_payload(
            &"a".repeat(64), "2025-01-01T00:00:00Z", "pki", "user",
        );
        let sig_bytes = identity1.sign(&payload);
        let sig_hex: String = sig_bytes.iter().map(|b| format!("{:02x}", b)).collect();

        let _ = engine.id_gen.next_id("provenance").unwrap();
        let env = ProvenanceEnvelope {
            id: 1,
            collection: "test".to_string(),
            record_id: 1,
            confidence_factor: 0.85,
            source_id: "user:test".to_string(),
            source_type: ProvenanceSourceType::User,
            content_hash: "a".repeat(64),
            created_at_utc: "2025-01-01T00:00:00Z".to_string(),
            updated_at_utc: "2025-01-01T00:00:00Z".to_string(),
            pki_signature: Some(sig_hex),
            pki_id: Some("pki".to_string()),
            user_id: Some("user".to_string()),
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
            checked_at_utc: None,
            data_updated_at_utc: None,
            local_id: None,
            global_id: None,
        };
        let bytes = to_msgpack(&env).unwrap();
        engine.envelopes.insert(&encode_id(1), &bytes).unwrap();

        let result = engine.verify(1, &identity2.verifying_key_bytes()).unwrap();
        assert_eq!(result.verification_status, ProvenanceVerificationStatus::Failed);
        assert!((result.confidence_factor - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn delete_envelope() {
        let (_dir, engine) = setup();
        engine.attach(
            "test", 1, "src", ProvenanceSourceType::User,
            "a".repeat(64), None, None, None, false, 0, None, None, None, None,
        ).unwrap();

        assert!(engine.delete(1).unwrap());
        assert!(engine.get(1).unwrap().is_none());
        assert!(!engine.delete(1).unwrap());
    }

    #[test]
    fn query_filters() {
        let (_dir, engine) = setup();
        engine.attach(
            "users", 1, "src1", ProvenanceSourceType::User,
            "a".repeat(64), None, None, None, true, 0, None, None, None, None,
        ).unwrap();
        engine.attach(
            "posts", 2, "src2", ProvenanceSourceType::Peer,
            "b".repeat(64), None, None, None, false, 1, None, None, None, None,
        ).unwrap();
        engine.attach(
            "users", 3, "src3", ProvenanceSourceType::Model,
            "c".repeat(64), None, None, None, false, 0, None, None, None, None,
        ).unwrap();

        // Filter by collection
        let results = engine.query(Some("users"), None, None, None).unwrap();
        assert_eq!(results.len(), 2);

        // Filter by source type
        let results = engine.query(None, Some(&ProvenanceSourceType::Peer), None, None).unwrap();
        assert_eq!(results.len(), 1);

        // Filter by min confidence
        let results = engine.query(None, None, None, Some(0.80)).unwrap();
        // User signed=1.0, Peer unsigned(1 hop)=0.60, Model unsigned=0.50
        assert_eq!(results.len(), 1); // only User signed
    }

    #[test]
    fn envelope_count() {
        let (_dir, engine) = setup();
        assert_eq!(engine.envelope_count().unwrap(), 0);

        engine.attach(
            "test", 1, "src", ProvenanceSourceType::User,
            "a".repeat(64), None, None, None, false, 0, None, None, None, None,
        ).unwrap();
        assert_eq!(engine.envelope_count().unwrap(), 1);
    }

    #[test]
    fn compute_hash_public_api() {
        let (_dir, engine) = setup();
        let data = Value::Map(vec![
            (Value::String("name".into()), Value::String("Alice".into())),
        ]);
        let hash = engine.compute_hash(&data).unwrap();
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn update_envelope_persists_ai_fields() {
        let (_dir, engine) = setup();
        engine.attach(
            "test", 1, "src", ProvenanceSourceType::User,
            "a".repeat(64), None, None, None, false, 0, None, None, None, None,
        ).unwrap();

        let mut env = engine.get(1).unwrap().unwrap();
        assert!(!env.ai_augmented);

        env.ai_augmented = true;
        env.ai_raw_confidence = Some(0.9);
        env.ai_reasoning = Some("test".to_string());
        env.confidence_factor = 0.76;
        engine.update_envelope(&env).unwrap();

        let fetched = engine.get(1).unwrap().unwrap();
        assert!(fetched.ai_augmented);
        assert!((fetched.ai_raw_confidence.unwrap() - 0.9).abs() < f64::EPSILON);
        assert_eq!(fetched.ai_reasoning, Some("test".to_string()));
        assert!((fetched.confidence_factor - 0.76).abs() < f64::EPSILON);
    }

    #[test]
    fn update_envelope_rejects_invalid_confidence() {
        let (_dir, engine) = setup();
        engine.attach(
            "test", 1, "src", ProvenanceSourceType::User,
            "a".repeat(64), None, None, None, false, 0, None, None, None, None,
        ).unwrap();

        let mut env = engine.get(1).unwrap().unwrap();
        env.confidence_factor = 1.5;
        assert!(engine.update_envelope(&env).is_err());
    }

    #[test]
    fn update_envelope_nonexistent() {
        let (_dir, engine) = setup();
        use crate::types::AnomalySeverity;
        let env = ProvenanceEnvelope {
            id: 999,
            collection: "test".to_string(),
            record_id: 1,
            confidence_factor: 0.5,
            source_id: "src".to_string(),
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
            ai_anomaly_flagged: true,
            ai_anomaly_severity: Some(AnomalySeverity::High),
            ai_originated: false,
            ai_origin_tag: None,
            ai_source_explanation: None,
            ai_external_source_uri: None,
            checked_at_utc: None,
            data_updated_at_utc: None,
            local_id: None,
            global_id: None,
        };
        assert!(engine.update_envelope(&env).is_err());
    }
}

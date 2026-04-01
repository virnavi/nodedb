use nodedb_crypto::PublicIdentity;
use crate::types::ProvenanceEnvelope;
use crate::error::ProvenanceError;

/// Build the signature payload: contentHash|createdAtUtc|pkiId|userId
pub fn build_signature_payload(
    content_hash: &str,
    created_at_utc: &str,
    pki_id: &str,
    user_id: &str,
) -> Vec<u8> {
    format!("{}|{}|{}|{}", content_hash, created_at_utc, pki_id, user_id)
        .into_bytes()
}

/// Verify the Ed25519 signature on a ProvenanceEnvelope.
/// Returns Ok(true) if signature is valid, Ok(false) if no signature present,
/// or Err on verification failure.
pub fn verify_signature(
    envelope: &ProvenanceEnvelope,
    public_key_bytes: &[u8],
) -> Result<bool, ProvenanceError> {
    let sig_hex = match &envelope.pki_signature {
        Some(s) => s,
        None => return Ok(false),
    };

    let pki_id = envelope.pki_id.as_deref().unwrap_or("");
    let user_id = envelope.user_id.as_deref().unwrap_or("");

    let payload = build_signature_payload(
        &envelope.content_hash,
        &envelope.created_at_utc,
        pki_id,
        user_id,
    );

    let sig_bytes = hex_decode(sig_hex)
        .map_err(|e| ProvenanceError::Verification(format!("invalid signature hex: {}", e)))?;

    let identity = PublicIdentity::from_public_key_bytes(public_key_bytes)
        .map_err(|e| ProvenanceError::Verification(format!("invalid public key: {}", e)))?;

    match identity.verify(&payload, &sig_bytes) {
        Ok(()) => Ok(true),
        Err(e) => Err(ProvenanceError::Verification(format!("signature verification failed: {}", e))),
    }
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err("odd length".to_string());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| e.to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodedb_crypto::NodeIdentity;

    fn hex_encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }

    #[test]
    fn build_payload_format() {
        let payload = build_signature_payload("abc123", "2025-01-01T00:00:00Z", "pki-1", "user-1");
        assert_eq!(
            String::from_utf8(payload).unwrap(),
            "abc123|2025-01-01T00:00:00Z|pki-1|user-1"
        );
    }

    #[test]
    fn verify_valid_signature() {
        let identity = NodeIdentity::generate();
        let content_hash = "a".repeat(64);
        let created_at = "2025-01-01T00:00:00Z";
        let pki_id = "test-pki";
        let user_id = "test-user";

        let payload = build_signature_payload(&content_hash, created_at, pki_id, user_id);
        let sig_bytes = identity.sign(&payload);
        let sig_hex = hex_encode(&sig_bytes);

        let envelope = ProvenanceEnvelope {
            id: 1,
            collection: "test".to_string(),
            record_id: 1,
            confidence_factor: 0.85,
            source_id: "user:test".to_string(),
            source_type: crate::types::ProvenanceSourceType::User,
            content_hash,
            created_at_utc: created_at.to_string(),
            updated_at_utc: created_at.to_string(),
            pki_signature: Some(sig_hex),
            pki_id: Some(pki_id.to_string()),
            user_id: Some(user_id.to_string()),
            verification_status: crate::types::ProvenanceVerificationStatus::Unverified,
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

        let result = verify_signature(&envelope, &identity.verifying_key_bytes());
        assert!(result.unwrap());
    }

    #[test]
    fn verify_wrong_key_fails() {
        let identity1 = NodeIdentity::generate();
        let identity2 = NodeIdentity::generate();
        let content_hash = "b".repeat(64);
        let created_at = "2025-01-01T00:00:00Z";

        let payload = build_signature_payload(&content_hash, created_at, "pki", "user");
        let sig_bytes = identity1.sign(&payload);
        let sig_hex = hex_encode(&sig_bytes);

        let envelope = ProvenanceEnvelope {
            id: 1,
            collection: "test".to_string(),
            record_id: 1,
            confidence_factor: 0.85,
            source_id: "user:test".to_string(),
            source_type: crate::types::ProvenanceSourceType::User,
            content_hash,
            created_at_utc: created_at.to_string(),
            updated_at_utc: created_at.to_string(),
            pki_signature: Some(sig_hex),
            pki_id: Some("pki".to_string()),
            user_id: Some("user".to_string()),
            verification_status: crate::types::ProvenanceVerificationStatus::Unverified,
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

        let result = verify_signature(&envelope, &identity2.verifying_key_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn verify_no_signature_returns_false() {
        let envelope = ProvenanceEnvelope {
            id: 1,
            collection: "test".to_string(),
            record_id: 1,
            confidence_factor: 0.85,
            source_id: "user:test".to_string(),
            source_type: crate::types::ProvenanceSourceType::User,
            content_hash: "c".repeat(64),
            created_at_utc: "2025-01-01T00:00:00Z".to_string(),
            updated_at_utc: "2025-01-01T00:00:00Z".to_string(),
            pki_signature: None,
            pki_id: None,
            user_id: None,
            verification_status: crate::types::ProvenanceVerificationStatus::Unverified,
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

        let identity = NodeIdentity::generate();
        let result = verify_signature(&envelope, &identity.verifying_key_bytes());
        assert!(!result.unwrap());
    }
}

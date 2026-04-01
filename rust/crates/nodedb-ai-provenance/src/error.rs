use nodedb_provenance::ProvenanceError;

#[derive(Debug, thiserror::Error)]
pub enum AiProvenanceError {
    #[error("provenance error: {0}")]
    Provenance(#[from] ProvenanceError),

    #[error("envelope not found: {0}")]
    EnvelopeNotFound(i64),

    #[error("invalid confidence: {0}")]
    InvalidConfidence(String),

    #[error("collection not enabled: {0}")]
    CollectionNotEnabled(String),

    #[error("configuration error: {0}")]
    ConfigError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = AiProvenanceError::EnvelopeNotFound(42);
        assert_eq!(err.to_string(), "envelope not found: 42");

        let err = AiProvenanceError::CollectionNotEnabled("users".to_string());
        assert_eq!(err.to_string(), "collection not enabled: users");
    }
}

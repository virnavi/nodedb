pub mod types;
pub mod error;
pub mod canonical;
pub mod content_hash;
pub mod confidence;
pub mod verification;
pub mod engine;

pub use types::{ProvenanceEnvelope, ProvenanceSourceType, ProvenanceVerificationStatus, AnomalySeverity};
pub use error::ProvenanceError;
pub use engine::ProvenanceEngine;
pub use canonical::canonical_msgpack;
pub use content_hash::compute_content_hash;
pub use confidence::{initial_confidence, corroborate, conflict, verification_boost, verification_failure, age_decay};
pub use verification::{build_signature_payload, verify_signature};

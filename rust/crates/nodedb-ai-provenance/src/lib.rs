pub mod types;
pub mod error;
pub mod config;
pub mod blending;
pub mod engine;

pub use types::{
    ConflictPreference, AiProvenanceAssessment, AiConflictResolution,
    AiAnomalyFlag, AiSourceClassification,
};
pub use error::AiProvenanceError;
pub use config::AiProvenanceConfig;
pub use blending::{blend_confidence, apply_verification_boost_post_blend, apply_anomaly_penalty, apply_conflict_deltas};
pub use engine::AiProvenanceEngine;

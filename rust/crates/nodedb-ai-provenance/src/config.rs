use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProvenanceConfig {
    pub ai_blend_weight: f64,
    pub enabled_collections: Vec<String>,
    pub response_timeout_secs: u64,
    pub silent_on_error: bool,
    pub rate_limit_per_minute: u32,
}

impl Default for AiProvenanceConfig {
    fn default() -> Self {
        AiProvenanceConfig {
            ai_blend_weight: 0.3,
            enabled_collections: Vec::new(),
            response_timeout_secs: 5,
            silent_on_error: true,
            rate_limit_per_minute: 60,
        }
    }
}

impl AiProvenanceConfig {
    /// Returns true if the collection is enabled for AI provenance, or if no
    /// collections are specified (meaning all are enabled).
    pub fn is_collection_enabled(&self, collection: &str) -> bool {
        self.enabled_collections.is_empty()
            || self.enabled_collections.iter().any(|c| c == collection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = AiProvenanceConfig::default();
        assert!((config.ai_blend_weight - 0.3).abs() < f64::EPSILON);
        assert!(config.enabled_collections.is_empty());
        assert_eq!(config.response_timeout_secs, 5);
        assert!(config.silent_on_error);
        assert_eq!(config.rate_limit_per_minute, 60);
    }

    #[test]
    fn collection_enabled_empty_means_all() {
        let config = AiProvenanceConfig::default();
        assert!(config.is_collection_enabled("users"));
        assert!(config.is_collection_enabled("anything"));
    }

    #[test]
    fn collection_enabled_specific() {
        let config = AiProvenanceConfig {
            enabled_collections: vec!["users".to_string(), "posts".to_string()],
            ..Default::default()
        };
        assert!(config.is_collection_enabled("users"));
        assert!(config.is_collection_enabled("posts"));
        assert!(!config.is_collection_enabled("comments"));
    }

    #[test]
    fn config_serde_roundtrip() {
        let config = AiProvenanceConfig {
            ai_blend_weight: 0.5,
            enabled_collections: vec!["users".to_string()],
            response_timeout_secs: 10,
            silent_on_error: false,
            rate_limit_per_minute: 120,
        };
        let bytes = rmp_serde::to_vec(&config).unwrap();
        let decoded: AiProvenanceConfig = rmp_serde::from_slice(&bytes).unwrap();
        assert!((decoded.ai_blend_weight - 0.5).abs() < f64::EPSILON);
        assert_eq!(decoded.enabled_collections, vec!["users".to_string()]);
        assert!(!decoded.silent_on_error);
    }
}

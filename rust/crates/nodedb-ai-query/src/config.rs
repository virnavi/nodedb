use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiQueryConfig {
    /// Minimum AI confidence required to write a result to the db.
    pub minimum_write_confidence: f64,
    /// Maximum number of records the AI may insert per query.
    pub max_results_per_query: usize,
    /// Collections that allow AI query fallback.
    pub enabled_collections: Vec<String>,
    /// Whether to report which results were persisted vs returned-only.
    pub report_write_decisions: bool,
    /// Rate limit: maximum AI query fallback calls per minute.
    pub rate_limit_per_minute: u32,
}

impl Default for AiQueryConfig {
    fn default() -> Self {
        AiQueryConfig {
            minimum_write_confidence: 0.80,
            max_results_per_query: 10,
            enabled_collections: Vec::new(),
            report_write_decisions: true,
            rate_limit_per_minute: 20,
        }
    }
}

impl AiQueryConfig {
    pub fn is_collection_enabled(&self, collection: &str) -> bool {
        self.enabled_collections.iter().any(|c| c == collection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = AiQueryConfig::default();
        assert!((config.minimum_write_confidence - 0.80).abs() < f64::EPSILON);
        assert_eq!(config.max_results_per_query, 10);
        assert!(config.enabled_collections.is_empty());
        assert!(config.report_write_decisions);
        assert_eq!(config.rate_limit_per_minute, 20);
    }

    #[test]
    fn collection_enabled() {
        let config = AiQueryConfig {
            enabled_collections: vec!["products".to_string(), "users".to_string()],
            ..Default::default()
        };
        assert!(config.is_collection_enabled("products"));
        assert!(config.is_collection_enabled("users"));
        assert!(!config.is_collection_enabled("logs"));
    }

    #[test]
    fn config_serde_roundtrip() {
        let config = AiQueryConfig {
            minimum_write_confidence: 0.75,
            max_results_per_query: 5,
            enabled_collections: vec!["docs".to_string()],
            report_write_decisions: false,
            rate_limit_per_minute: 30,
        };
        let bytes = rmp_serde::to_vec(&config).unwrap();
        let decoded: AiQueryConfig = rmp_serde::from_slice(&bytes).unwrap();
        assert!((decoded.minimum_write_confidence - 0.75).abs() < f64::EPSILON);
        assert_eq!(decoded.max_results_per_query, 5);
        assert_eq!(decoded.enabled_collections, vec!["docs"]);
        assert!(!decoded.report_write_decisions);
        assert_eq!(decoded.rate_limit_per_minute, 30);
    }
}

use std::sync::Arc;

use chrono::Utc;

use nodedb_nosql::Database;
use nodedb_provenance::{ProvenanceEngine, ProvenanceSourceType, compute_content_hash};

use crate::config::AiQueryConfig;
use crate::error::AiQueryError;
use crate::schema_validator;
use crate::types::{AiQueryResult, AiQueryWriteDecision, AiQuerySchema};

pub struct AiQueryEngine {
    database: Arc<Database>,
    provenance: Arc<ProvenanceEngine>,
    config: AiQueryConfig,
}

impl AiQueryEngine {
    pub fn new(
        database: Arc<Database>,
        provenance: Arc<ProvenanceEngine>,
        config: AiQueryConfig,
    ) -> Self {
        AiQueryEngine { database, provenance, config }
    }

    pub fn config(&self) -> &AiQueryConfig {
        &self.config
    }

    /// Process AI query results: confidence gate, schema validate, write to DB, attach provenance.
    pub fn process_results(
        &self,
        collection: &str,
        results: Vec<AiQueryResult>,
        schema: Option<&AiQuerySchema>,
    ) -> Result<Vec<AiQueryWriteDecision>, AiQueryError> {
        if !self.config.is_collection_enabled(collection) {
            return Err(AiQueryError::CollectionNotEnabled(collection.to_string()));
        }

        // Truncate to max_results_per_query
        let max = self.config.max_results_per_query;
        let results: Vec<AiQueryResult> = if results.len() > max {
            results.into_iter().take(max).collect()
        } else {
            results
        };

        let mut decisions = Vec::with_capacity(results.len());

        for result in results {
            // 1. Confidence gate
            if result.confidence < self.config.minimum_write_confidence {
                decisions.push(AiQueryWriteDecision {
                    persisted: false,
                    record_id: None,
                    confidence: result.confidence,
                    ai_origin_tag: None,
                    rejection_reason: Some(format!(
                        "confidence {} below threshold {}",
                        result.confidence, self.config.minimum_write_confidence
                    )),
                });
                continue;
            }

            // 2. Schema validation (if schema provided)
            if let Some(schema) = schema {
                if let Err(e) = schema_validator::validate(&result.data, schema) {
                    decisions.push(AiQueryWriteDecision {
                        persisted: false,
                        record_id: None,
                        confidence: result.confidence,
                        ai_origin_tag: None,
                        rejection_reason: Some(format!("schema validation failed: {}", e)),
                    });
                    continue;
                }
            }

            // 3. Write to database
            let col = self.database.collection(collection)?;
            let doc = col.put(result.data.clone())?;
            let record_id = doc.id;

            // 4. Compute content hash
            let content_hash = compute_content_hash(&result.data)?;

            // 5. Attach provenance with AiQuery source type
            let source_id = result.external_source_uri
                .as_deref()
                .unwrap_or("ai-query");
            let mut envelope = self.provenance.attach(
                collection,
                record_id,
                source_id,
                ProvenanceSourceType::AiQuery,
                content_hash,
                None,  // pki_signature
                None,  // pki_id
                None,  // user_id
                false, // is_signed
                0,     // hops
                None,  // created_at_utc
                None,  // data_updated_at_utc
                None,  // local_id
                None,  // global_id
            )?;

            // 6. Set AI origin fields
            let now = Utc::now().to_rfc3339();
            let ai_origin_tag = format!("ai-query:{}:{}", collection, now);
            envelope.ai_originated = true;
            envelope.ai_origin_tag = Some(ai_origin_tag.clone());
            envelope.ai_source_explanation = Some(result.source_explanation);
            envelope.ai_external_source_uri = result.external_source_uri;
            if let Some(tags) = result.tags {
                envelope.ai_tags = Some(tags);
            }

            // 7. Persist updated envelope
            self.provenance.update_envelope(&envelope)?;

            decisions.push(AiQueryWriteDecision {
                persisted: true,
                record_id: Some(record_id),
                confidence: result.confidence,
                ai_origin_tag: Some(ai_origin_tag),
                rejection_reason: None,
            });
        }

        Ok(decisions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmpv::Value;
    use std::collections::HashMap;
    use tempfile::TempDir;
    use nodedb_storage::StorageEngine;

    fn setup() -> (AiQueryEngine, TempDir) {
        let dir = TempDir::new().unwrap();
        let nosql_path = dir.path().join("nosql");
        let prov_path = dir.path().join("prov");
        std::fs::create_dir_all(&nosql_path).unwrap();
        std::fs::create_dir_all(&prov_path).unwrap();

        let db = Arc::new(Database::open(&nosql_path).unwrap());
        let prov_engine = Arc::new(StorageEngine::open(&prov_path).unwrap());
        let prov = Arc::new(ProvenanceEngine::new(prov_engine).unwrap());

        let config = AiQueryConfig {
            minimum_write_confidence: 0.80,
            max_results_per_query: 10,
            enabled_collections: vec!["products".to_string()],
            report_write_decisions: true,
            rate_limit_per_minute: 20,
        };

        (AiQueryEngine::new(db, prov, config), dir)
    }

    fn make_result(name: &str, confidence: f64) -> AiQueryResult {
        AiQueryResult {
            data: Value::Map(vec![
                (Value::String("name".into()), Value::String(name.into())),
                (Value::String("price".into()), Value::F64(9.99)),
            ]),
            confidence,
            source_explanation: "test source".to_string(),
            external_source_uri: Some("https://example.com".to_string()),
            tags: None,
        }
    }

    #[test]
    fn process_results_writes_to_db() {
        let (engine, _dir) = setup();
        let results = vec![make_result("Widget", 0.92)];
        let decisions = engine.process_results("products", results, None).unwrap();
        assert_eq!(decisions.len(), 1);
        assert!(decisions[0].persisted);
        assert_eq!(decisions[0].record_id, Some(1));
        assert!(decisions[0].ai_origin_tag.is_some());
        assert!(decisions[0].ai_origin_tag.as_ref().unwrap().starts_with("ai-query:products:"));
        assert!(decisions[0].rejection_reason.is_none());
    }

    #[test]
    fn below_threshold_not_persisted() {
        let (engine, _dir) = setup();
        let results = vec![make_result("Widget", 0.50)];
        let decisions = engine.process_results("products", results, None).unwrap();
        assert_eq!(decisions.len(), 1);
        assert!(!decisions[0].persisted);
        assert!(decisions[0].record_id.is_none());
        assert!(decisions[0].rejection_reason.as_ref().unwrap().contains("below threshold"));
    }

    #[test]
    fn schema_validation_failure() {
        let (engine, _dir) = setup();
        let mut field_types = HashMap::new();
        field_types.insert("name".to_string(), crate::types::SchemaPropertyType::Integer);
        let schema = AiQuerySchema {
            required_fields: vec![],
            field_types,
        };
        let results = vec![make_result("Widget", 0.92)]; // name is String, not Integer
        let decisions = engine.process_results("products", results, Some(&schema)).unwrap();
        assert_eq!(decisions.len(), 1);
        assert!(!decisions[0].persisted);
        assert!(decisions[0].rejection_reason.as_ref().unwrap().contains("schema validation failed"));
    }

    #[test]
    fn collection_not_enabled() {
        let (engine, _dir) = setup();
        let results = vec![make_result("Widget", 0.92)];
        let err = engine.process_results("unknown_col", results, None).unwrap_err();
        assert!(err.to_string().contains("collection not enabled"));
    }

    #[test]
    fn multiple_results_mixed() {
        let (engine, _dir) = setup();
        let results = vec![
            make_result("Widget A", 0.92),
            make_result("Widget B", 0.50), // below threshold
            make_result("Widget C", 0.85),
        ];
        let decisions = engine.process_results("products", results, None).unwrap();
        assert_eq!(decisions.len(), 3);
        assert!(decisions[0].persisted);
        assert!(!decisions[1].persisted);
        assert!(decisions[2].persisted);
    }

    #[test]
    fn truncation_to_max_results() {
        let dir = TempDir::new().unwrap();
        let nosql_path = dir.path().join("nosql");
        let prov_path = dir.path().join("prov");
        std::fs::create_dir_all(&nosql_path).unwrap();
        std::fs::create_dir_all(&prov_path).unwrap();

        let db = Arc::new(Database::open(&nosql_path).unwrap());
        let prov_engine = Arc::new(StorageEngine::open(&prov_path).unwrap());
        let prov = Arc::new(ProvenanceEngine::new(prov_engine).unwrap());

        let config = AiQueryConfig {
            minimum_write_confidence: 0.80,
            max_results_per_query: 2, // limit to 2
            enabled_collections: vec!["products".to_string()],
            ..Default::default()
        };
        let engine = AiQueryEngine::new(db, prov, config);

        let results = vec![
            make_result("A", 0.90),
            make_result("B", 0.91),
            make_result("C", 0.92), // should be truncated
        ];
        let decisions = engine.process_results("products", results, None).unwrap();
        assert_eq!(decisions.len(), 2);
    }

    #[test]
    fn provenance_envelope_has_ai_origin() {
        let (engine, _dir) = setup();
        let results = vec![make_result("Widget", 0.92)];
        let decisions = engine.process_results("products", results, None).unwrap();
        assert!(decisions[0].persisted);

        // Verify provenance envelope
        let record_id = decisions[0].record_id.unwrap();
        let envelopes = engine.provenance.get_for_record("products", record_id).unwrap();
        assert_eq!(envelopes.len(), 1);
        let env = &envelopes[0];
        assert!(env.ai_originated);
        assert!(env.ai_origin_tag.is_some());
        assert_eq!(env.source_type, ProvenanceSourceType::AiQuery);
        assert_eq!(env.ai_source_explanation, Some("test source".to_string()));
        assert_eq!(env.ai_external_source_uri, Some("https://example.com".to_string()));
    }

    #[test]
    fn result_with_tags_persisted() {
        let (engine, _dir) = setup();
        let mut tags = HashMap::new();
        tags.insert("model".to_string(), "gpt-4".to_string());
        let result = AiQueryResult {
            data: Value::Map(vec![
                (Value::String("name".into()), Value::String("Widget".into())),
            ]),
            confidence: 0.95,
            source_explanation: "test".to_string(),
            external_source_uri: None,
            tags: Some(tags),
        };
        let decisions = engine.process_results("products", vec![result], None).unwrap();
        assert!(decisions[0].persisted);

        let record_id = decisions[0].record_id.unwrap();
        let envelopes = engine.provenance.get_for_record("products", record_id).unwrap();
        let env = &envelopes[0];
        assert_eq!(env.ai_tags.as_ref().unwrap().get("model").unwrap(), "gpt-4");
    }
}

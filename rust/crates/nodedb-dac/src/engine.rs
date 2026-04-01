use std::sync::Arc;

use chrono::{DateTime, Utc};
use rmpv::Value;

use nodedb_storage::{StorageEngine, IdGenerator};

use crate::error::DacError;
use crate::filter;
use crate::rule_manager::RuleManager;
use crate::types::{NodeAccessRule, AccessSubjectType, AccessPermission, DacSubject};

pub struct DacEngine {
    engine: Arc<StorageEngine>,
    rule_mgr: RuleManager,
}

impl DacEngine {
    pub fn new(engine: Arc<StorageEngine>) -> Result<Self, DacError> {
        let id_gen = Arc::new(IdGenerator::new(&engine)?);
        let rule_mgr = RuleManager::new(&engine, id_gen)?;
        Ok(DacEngine { engine, rule_mgr })
    }

    // --- Rule CRUD ---

    pub fn add_rule(
        &self,
        collection: &str,
        field: Option<String>,
        record_id: Option<String>,
        subject_type: AccessSubjectType,
        subject_id: &str,
        permission: AccessPermission,
        expires_at: Option<DateTime<Utc>>,
        created_by: Option<String>,
    ) -> Result<NodeAccessRule, DacError> {
        self.rule_mgr.add_rule(collection, field, record_id, subject_type, subject_id, permission, expires_at, created_by)
    }

    pub fn get_rule(&self, id: i64) -> Result<NodeAccessRule, DacError> {
        self.rule_mgr.get_rule(id)
    }

    pub fn update_rule(
        &self,
        id: i64,
        permission: Option<AccessPermission>,
        expires_at: Option<Option<DateTime<Utc>>>,
    ) -> Result<NodeAccessRule, DacError> {
        self.rule_mgr.update_rule(id, permission, expires_at)
    }

    pub fn delete_rule(&self, id: i64) -> Result<NodeAccessRule, DacError> {
        self.rule_mgr.delete_rule(id)
    }

    pub fn all_rules(&self) -> Result<Vec<NodeAccessRule>, DacError> {
        self.rule_mgr.all_rules()
    }

    pub fn rules_for_collection(&self, collection: &str) -> Result<Vec<NodeAccessRule>, DacError> {
        self.rule_mgr.rules_for_collection(collection)
    }

    pub fn rule_count(&self) -> usize {
        self.rule_mgr.rule_count()
    }

    // --- Filter API ---

    /// Filter a document according to DAC rules for a given subject.
    pub fn filter_document(
        &self,
        collection: &str,
        doc: &Value,
        subject: &DacSubject,
        record_id: Option<&str>,
    ) -> Result<Value, DacError> {
        let rules = self.rule_mgr.rules_for_collection(collection)?;
        filter::filter_document(doc, &rules, subject, record_id)
    }

    pub fn flush(&self) -> Result<(), DacError> {
        self.engine.flush()?;
        Ok(())
    }
}

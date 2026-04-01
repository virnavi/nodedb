use std::sync::Arc;

use chrono::{DateTime, Utc};

use nodedb_storage::{StorageEngine, StorageTree, IdGenerator, encode_id, to_msgpack, from_msgpack};

use crate::error::DacError;
use crate::types::{NodeAccessRule, AccessSubjectType, AccessPermission};

pub struct RuleManager {
    rules: StorageTree,
    id_gen: Arc<IdGenerator>,
}

impl RuleManager {
    pub fn new(engine: &Arc<StorageEngine>, id_gen: Arc<IdGenerator>) -> Result<Self, DacError> {
        let rules = engine.open_tree("__access_rules__")?;
        Ok(RuleManager { rules, id_gen })
    }

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
        let id = self.id_gen.next_id("access_rules")?;
        let now = Utc::now();
        let rule = NodeAccessRule {
            id,
            collection: collection.to_string(),
            field,
            record_id,
            subject_type,
            subject_id: subject_id.to_string(),
            permission,
            expires_at,
            created_at: now,
            created_by,
        };

        let bytes = to_msgpack(&rule)?;
        self.rules.insert(&encode_id(id), &bytes)?;

        Ok(rule)
    }

    pub fn get_rule(&self, id: i64) -> Result<NodeAccessRule, DacError> {
        let bytes = self.rules.get(&encode_id(id))?
            .ok_or(DacError::RuleNotFound(id))?;
        let rule: NodeAccessRule = from_msgpack(&bytes)?;
        Ok(rule)
    }

    pub fn update_rule(
        &self,
        id: i64,
        permission: Option<AccessPermission>,
        expires_at: Option<Option<DateTime<Utc>>>,
    ) -> Result<NodeAccessRule, DacError> {
        let mut rule = self.get_rule(id)?;

        if let Some(p) = permission {
            rule.permission = p;
        }
        if let Some(ea) = expires_at {
            rule.expires_at = ea;
        }

        let bytes = to_msgpack(&rule)?;
        self.rules.insert(&encode_id(id), &bytes)?;

        Ok(rule)
    }

    pub fn delete_rule(&self, id: i64) -> Result<NodeAccessRule, DacError> {
        let rule = self.get_rule(id)?;
        self.rules.remove(&encode_id(id))?;
        Ok(rule)
    }

    pub fn all_rules(&self) -> Result<Vec<NodeAccessRule>, DacError> {
        let mut rules = Vec::new();
        for item in self.rules.iter() {
            let (_, v) = item?;
            let rule: NodeAccessRule = from_msgpack(&v)?;
            rules.push(rule);
        }
        Ok(rules)
    }

    pub fn rules_for_collection(&self, collection: &str) -> Result<Vec<NodeAccessRule>, DacError> {
        let all = self.all_rules()?;
        Ok(all.into_iter().filter(|r| r.collection == collection).collect())
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, Arc<StorageEngine>, Arc<IdGenerator>) {
        let dir = TempDir::new().unwrap();
        let engine = Arc::new(StorageEngine::open(&dir.path().join("db")).unwrap());
        let id_gen = Arc::new(IdGenerator::new(&engine).unwrap());
        (dir, engine, id_gen)
    }

    #[test]
    fn add_and_get_rule() {
        let (_dir, engine, id_gen) = setup();
        let mgr = RuleManager::new(&engine, id_gen).unwrap();

        let rule = mgr.add_rule(
            "users", Some("email".into()), None,
            AccessSubjectType::Peer, "alice",
            AccessPermission::Allow, None, None,
        ).unwrap();
        assert_eq!(rule.id, 1);
        assert_eq!(rule.collection, "users");
        assert_eq!(rule.field, Some("email".to_string()));

        let fetched = mgr.get_rule(1).unwrap();
        assert_eq!(fetched.permission, AccessPermission::Allow);
    }

    #[test]
    fn update_rule() {
        let (_dir, engine, id_gen) = setup();
        let mgr = RuleManager::new(&engine, id_gen).unwrap();

        mgr.add_rule(
            "users", None, None,
            AccessSubjectType::Group, "admins",
            AccessPermission::Allow, None, None,
        ).unwrap();

        let updated = mgr.update_rule(1, Some(AccessPermission::Deny), None).unwrap();
        assert_eq!(updated.permission, AccessPermission::Deny);

        let fetched = mgr.get_rule(1).unwrap();
        assert_eq!(fetched.permission, AccessPermission::Deny);
    }

    #[test]
    fn delete_rule() {
        let (_dir, engine, id_gen) = setup();
        let mgr = RuleManager::new(&engine, id_gen).unwrap();

        mgr.add_rule(
            "users", None, None,
            AccessSubjectType::Peer, "alice",
            AccessPermission::Allow, None, None,
        ).unwrap();
        assert_eq!(mgr.rule_count(), 1);

        mgr.delete_rule(1).unwrap();
        assert_eq!(mgr.rule_count(), 0);
    }

    #[test]
    fn all_rules_and_count() {
        let (_dir, engine, id_gen) = setup();
        let mgr = RuleManager::new(&engine, id_gen).unwrap();

        mgr.add_rule("users", None, None, AccessSubjectType::Peer, "a", AccessPermission::Allow, None, None).unwrap();
        mgr.add_rule("posts", None, None, AccessSubjectType::Peer, "b", AccessPermission::Deny, None, None).unwrap();
        mgr.add_rule("users", Some("email".into()), None, AccessSubjectType::Group, "g", AccessPermission::Redact, None, None).unwrap();

        assert_eq!(mgr.rule_count(), 3);
        assert_eq!(mgr.all_rules().unwrap().len(), 3);
    }

    #[test]
    fn rules_for_collection() {
        let (_dir, engine, id_gen) = setup();
        let mgr = RuleManager::new(&engine, id_gen).unwrap();

        mgr.add_rule("users", None, None, AccessSubjectType::Peer, "a", AccessPermission::Allow, None, None).unwrap();
        mgr.add_rule("posts", None, None, AccessSubjectType::Peer, "b", AccessPermission::Deny, None, None).unwrap();
        mgr.add_rule("users", Some("email".into()), None, AccessSubjectType::Group, "g", AccessPermission::Redact, None, None).unwrap();

        let user_rules = mgr.rules_for_collection("users").unwrap();
        assert_eq!(user_rules.len(), 2);

        let post_rules = mgr.rules_for_collection("posts").unwrap();
        assert_eq!(post_rules.len(), 1);

        let empty = mgr.rules_for_collection("nonexistent").unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn rule_not_found() {
        let (_dir, engine, id_gen) = setup();
        let mgr = RuleManager::new(&engine, id_gen).unwrap();

        match mgr.get_rule(999) {
            Err(DacError::RuleNotFound(999)) => {}
            other => panic!("expected RuleNotFound, got {:?}", other),
        }
    }
}

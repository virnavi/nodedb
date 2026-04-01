use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessSubjectType {
    Peer,
    Group,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessPermission {
    Allow,
    Deny,
    Redact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAccessRule {
    pub id: i64,
    pub collection: String,
    pub field: Option<String>,
    pub record_id: Option<String>,
    pub subject_type: AccessSubjectType,
    pub subject_id: String,
    pub permission: AccessPermission,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DacSubject {
    pub peer_id: String,
    pub group_ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_type_serde_roundtrip() {
        for st in [AccessSubjectType::Peer, AccessSubjectType::Group] {
            let bytes = rmp_serde::to_vec(&st).unwrap();
            let decoded: AccessSubjectType = rmp_serde::from_slice(&bytes).unwrap();
            assert_eq!(decoded, st);
        }
    }

    #[test]
    fn permission_serde_roundtrip() {
        for p in [AccessPermission::Allow, AccessPermission::Deny, AccessPermission::Redact] {
            let bytes = rmp_serde::to_vec(&p).unwrap();
            let decoded: AccessPermission = rmp_serde::from_slice(&bytes).unwrap();
            assert_eq!(decoded, p);
        }
    }

    #[test]
    fn access_rule_serde_roundtrip() {
        let rule = NodeAccessRule {
            id: 1,
            collection: "users".to_string(),
            field: Some("email".to_string()),
            record_id: None,
            subject_type: AccessSubjectType::Group,
            subject_id: "editors".to_string(),
            permission: AccessPermission::Redact,
            expires_at: None,
            created_at: Utc::now(),
            created_by: Some("admin".to_string()),
        };
        let bytes = rmp_serde::to_vec(&rule).unwrap();
        let decoded: NodeAccessRule = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, rule.id);
        assert_eq!(decoded.collection, rule.collection);
        assert_eq!(decoded.field, rule.field);
        assert_eq!(decoded.subject_type, rule.subject_type);
        assert_eq!(decoded.permission, rule.permission);
    }

    #[test]
    fn access_rule_all_optionals_none() {
        let rule = NodeAccessRule {
            id: 2,
            collection: "posts".to_string(),
            field: None,
            record_id: None,
            subject_type: AccessSubjectType::Peer,
            subject_id: "alice".to_string(),
            permission: AccessPermission::Allow,
            expires_at: None,
            created_at: Utc::now(),
            created_by: None,
        };
        let bytes = rmp_serde::to_vec(&rule).unwrap();
        let decoded: NodeAccessRule = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.field, None);
        assert_eq!(decoded.record_id, None);
        assert_eq!(decoded.expires_at, None);
        assert_eq!(decoded.created_by, None);
    }
}

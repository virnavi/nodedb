use chrono::Utc;

use crate::types::{NodeAccessRule, AccessSubjectType, AccessPermission, DacSubject};

fn is_expired(rule: &NodeAccessRule) -> bool {
    match rule.expires_at {
        Some(t) => Utc::now() > t,
        None => false,
    }
}

fn matches_subject(rule: &NodeAccessRule, subject: &DacSubject) -> bool {
    match rule.subject_type {
        AccessSubjectType::Peer => rule.subject_id == subject.peer_id,
        AccessSubjectType::Group => subject.group_ids.contains(&rule.subject_id),
    }
}

fn resolve_priority_flat(rules: &[&NodeAccessRule]) -> Option<AccessPermission> {
    if rules.is_empty() {
        return None;
    }
    // deny > allow > redact
    if rules.iter().any(|r| r.permission == AccessPermission::Deny) {
        return Some(AccessPermission::Deny);
    }
    if rules.iter().any(|r| r.permission == AccessPermission::Allow) {
        return Some(AccessPermission::Allow);
    }
    if rules.iter().any(|r| r.permission == AccessPermission::Redact) {
        return Some(AccessPermission::Redact);
    }
    None
}

/// Evaluate the effective permission for a specific field.
///
/// Uses 4-level specificity cascade:
/// 1. Record-level (collection + recordId): record+field first, then record-only
/// 2. Field-level (collection + field, no recordId)
/// 3. Collection-level (collection only)
/// 4. Default: Deny
pub fn evaluate_field(
    rules: &[NodeAccessRule],
    subject: &DacSubject,
    record_id: Option<&str>,
    field: &str,
) -> AccessPermission {
    // Filter to non-expired, subject-matching rules
    let applicable: Vec<&NodeAccessRule> = rules.iter()
        .filter(|r| !is_expired(r) && matches_subject(r, subject))
        .collect();

    // Step 1: Record-level rules
    if let Some(rid) = record_id {
        let record_rules: Vec<&NodeAccessRule> = applicable.iter()
            .filter(|r| r.record_id.as_deref() == Some(rid))
            .copied()
            .collect();

        if !record_rules.is_empty() {
            // 1a: Record + field rules (most specific)
            let record_field: Vec<&NodeAccessRule> = record_rules.iter()
                .filter(|r| r.field.as_deref() == Some(field))
                .copied()
                .collect();
            if let Some(p) = resolve_priority_flat(&record_field) {
                return p;
            }

            // 1b: Record-only rules (field is None)
            let record_only: Vec<&NodeAccessRule> = record_rules.iter()
                .filter(|r| r.field.is_none())
                .copied()
                .collect();
            if let Some(p) = resolve_priority_flat(&record_only) {
                return p;
            }
        }
    }

    // Step 2: Field-level rules (no recordId, specific field)
    let field_rules: Vec<&NodeAccessRule> = applicable.iter()
        .filter(|r| r.record_id.is_none() && r.field.as_deref() == Some(field))
        .copied()
        .collect();
    if let Some(p) = resolve_priority_flat(&field_rules) {
        return p;
    }

    // Step 3: Collection-level rules (no recordId, no field)
    let collection_rules: Vec<&NodeAccessRule> = applicable.iter()
        .filter(|r| r.record_id.is_none() && r.field.is_none())
        .copied()
        .collect();
    if let Some(p) = resolve_priority_flat(&collection_rules) {
        return p;
    }

    // Step 4: Default deny
    AccessPermission::Deny
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn make_rule(
        collection: &str,
        field: Option<&str>,
        record_id: Option<&str>,
        subject_type: AccessSubjectType,
        subject_id: &str,
        permission: AccessPermission,
    ) -> NodeAccessRule {
        NodeAccessRule {
            id: 0,
            collection: collection.to_string(),
            field: field.map(|f| f.to_string()),
            record_id: record_id.map(|r| r.to_string()),
            subject_type,
            subject_id: subject_id.to_string(),
            permission,
            expires_at: None,
            created_at: Utc::now(),
            created_by: None,
        }
    }

    fn peer_subject(name: &str) -> DacSubject {
        DacSubject { peer_id: name.to_string(), group_ids: vec![] }
    }

    fn group_subject(peer: &str, groups: &[&str]) -> DacSubject {
        DacSubject {
            peer_id: peer.to_string(),
            group_ids: groups.iter().map(|g| g.to_string()).collect(),
        }
    }

    #[test]
    fn no_rules_defaults_to_deny() {
        let rules: Vec<NodeAccessRule> = vec![];
        let subject = peer_subject("alice");
        assert_eq!(evaluate_field(&rules, &subject, None, "email"), AccessPermission::Deny);
    }

    #[test]
    fn deny_overrides_allow_at_same_level() {
        let rules = vec![
            make_rule("users", None, None, AccessSubjectType::Peer, "alice", AccessPermission::Allow),
            make_rule("users", None, None, AccessSubjectType::Peer, "alice", AccessPermission::Deny),
        ];
        let subject = peer_subject("alice");
        assert_eq!(evaluate_field(&rules, &subject, None, "name"), AccessPermission::Deny);
    }

    #[test]
    fn allow_overrides_redact_at_same_level() {
        let rules = vec![
            make_rule("users", None, None, AccessSubjectType::Peer, "alice", AccessPermission::Redact),
            make_rule("users", None, None, AccessSubjectType::Peer, "alice", AccessPermission::Allow),
        ];
        let subject = peer_subject("alice");
        assert_eq!(evaluate_field(&rules, &subject, None, "name"), AccessPermission::Allow);
    }

    #[test]
    fn record_level_overrides_collection_level() {
        let rules = vec![
            make_rule("users", None, None, AccessSubjectType::Peer, "alice", AccessPermission::Deny),
            make_rule("users", None, Some("42"), AccessSubjectType::Peer, "alice", AccessPermission::Allow),
        ];
        let subject = peer_subject("alice");
        // Record-level allow overrides collection-level deny
        assert_eq!(evaluate_field(&rules, &subject, Some("42"), "name"), AccessPermission::Allow);
        // Different record falls through to collection-level deny
        assert_eq!(evaluate_field(&rules, &subject, Some("99"), "name"), AccessPermission::Deny);
    }

    #[test]
    fn field_level_overrides_collection_level() {
        let rules = vec![
            make_rule("users", None, None, AccessSubjectType::Peer, "alice", AccessPermission::Allow),
            make_rule("users", Some("email"), None, AccessSubjectType::Peer, "alice", AccessPermission::Redact),
        ];
        let subject = peer_subject("alice");
        assert_eq!(evaluate_field(&rules, &subject, None, "email"), AccessPermission::Redact);
        assert_eq!(evaluate_field(&rules, &subject, None, "name"), AccessPermission::Allow);
    }

    #[test]
    fn record_field_overrides_record_only() {
        let rules = vec![
            make_rule("users", None, Some("42"), AccessSubjectType::Peer, "alice", AccessPermission::Allow),
            make_rule("users", Some("email"), Some("42"), AccessSubjectType::Peer, "alice", AccessPermission::Deny),
        ];
        let subject = peer_subject("alice");
        assert_eq!(evaluate_field(&rules, &subject, Some("42"), "email"), AccessPermission::Deny);
        assert_eq!(evaluate_field(&rules, &subject, Some("42"), "name"), AccessPermission::Allow);
    }

    #[test]
    fn expired_rule_skipped() {
        let mut rule = make_rule("users", None, None, AccessSubjectType::Peer, "alice", AccessPermission::Allow);
        rule.expires_at = Some(Utc::now() - Duration::hours(1));
        let rules = vec![rule];
        let subject = peer_subject("alice");
        // Expired allow → falls through to default deny
        assert_eq!(evaluate_field(&rules, &subject, None, "name"), AccessPermission::Deny);
    }

    #[test]
    fn non_expired_rule_applies() {
        let mut rule = make_rule("users", None, None, AccessSubjectType::Peer, "alice", AccessPermission::Allow);
        rule.expires_at = Some(Utc::now() + Duration::hours(1));
        let rules = vec![rule];
        let subject = peer_subject("alice");
        assert_eq!(evaluate_field(&rules, &subject, None, "name"), AccessPermission::Allow);
    }

    #[test]
    fn peer_subject_match() {
        let rules = vec![
            make_rule("users", None, None, AccessSubjectType::Peer, "alice", AccessPermission::Allow),
        ];
        assert_eq!(evaluate_field(&rules, &peer_subject("alice"), None, "name"), AccessPermission::Allow);
        assert_eq!(evaluate_field(&rules, &peer_subject("bob"), None, "name"), AccessPermission::Deny);
    }

    #[test]
    fn group_subject_match() {
        let rules = vec![
            make_rule("users", None, None, AccessSubjectType::Group, "admins", AccessPermission::Allow),
        ];
        let subject = group_subject("alice", &["admins", "editors"]);
        assert_eq!(evaluate_field(&rules, &subject, None, "name"), AccessPermission::Allow);

        let non_member = group_subject("bob", &["editors"]);
        assert_eq!(evaluate_field(&rules, &non_member, None, "name"), AccessPermission::Deny);
    }

    #[test]
    fn mixed_peer_and_group_rules() {
        let rules = vec![
            make_rule("users", None, None, AccessSubjectType::Peer, "alice", AccessPermission::Deny),
            make_rule("users", None, None, AccessSubjectType::Group, "admins", AccessPermission::Allow),
        ];
        let subject = group_subject("alice", &["admins"]);
        // deny > allow at same level
        assert_eq!(evaluate_field(&rules, &subject, None, "name"), AccessPermission::Deny);
    }
}

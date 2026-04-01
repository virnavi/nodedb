use rmpv::Value;

use crate::error::DacError;
use crate::evaluator::evaluate_field;
use crate::types::{NodeAccessRule, AccessPermission, DacSubject};

/// Filter a document (rmpv::Value map) according to DAC rules.
///
/// - Denied fields are removed entirely
/// - Redacted fields are replaced with Value::Nil
/// - Allowed fields pass through unchanged
///
/// Supports one-level dot-path resolution: a rule on field "parent.child"
/// will apply to key "child" inside a nested map at key "parent".
pub fn filter_document(
    doc: &Value,
    rules: &[NodeAccessRule],
    subject: &DacSubject,
    record_id: Option<&str>,
) -> Result<Value, DacError> {
    let entries = match doc.as_map() {
        Some(m) => m,
        None => return Err(DacError::InvalidDocument),
    };

    // Collect dot-path rules grouped by parent key
    let dot_rules: Vec<(&str, &str, &NodeAccessRule)> = rules.iter()
        .filter_map(|r| {
            r.field.as_deref().and_then(|f| {
                f.split_once('.').map(|(parent, child)| (parent, child, r))
            })
        })
        .collect();

    let mut output = Vec::new();

    for (key, value) in entries {
        let field_name = match key.as_str() {
            Some(s) => s,
            None => {
                // Non-string keys pass through
                output.push((key.clone(), value.clone()));
                continue;
            }
        };

        let perm = evaluate_field(rules, subject, record_id, field_name);

        match perm {
            AccessPermission::Allow => {
                // Check if there are dot-path rules targeting children of this field
                let child_rules: Vec<_> = dot_rules.iter()
                    .filter(|(parent, _, _)| *parent == field_name)
                    .collect();

                if !child_rules.is_empty() {
                    if let Some(nested_entries) = value.as_map() {
                        // Filter the nested map
                        let filtered_nested = filter_nested_map(
                            nested_entries, &child_rules, rules, subject, record_id, field_name,
                        );
                        output.push((key.clone(), Value::Map(filtered_nested)));
                    } else {
                        output.push((key.clone(), value.clone()));
                    }
                } else {
                    output.push((key.clone(), value.clone()));
                }
            }
            AccessPermission::Deny => {
                // Omit field entirely
            }
            AccessPermission::Redact => {
                output.push((key.clone(), Value::Nil));
            }
        }
    }

    Ok(Value::Map(output))
}

fn filter_nested_map(
    entries: &[(Value, Value)],
    child_rules: &[&(&str, &str, &NodeAccessRule)],
    _all_rules: &[NodeAccessRule],
    _subject: &DacSubject,
    _record_id: Option<&str>,
    _parent_field: &str,
) -> Vec<(Value, Value)> {
    let mut output = Vec::new();

    for (key, value) in entries {
        let child_name = match key.as_str() {
            Some(s) => s,
            None => {
                output.push((key.clone(), value.clone()));
                continue;
            }
        };

        // Check if any dot-path rule targets this child
        let matching: Vec<_> = child_rules.iter()
            .filter(|(_, child, _)| *child == child_name)
            .collect();

        if matching.is_empty() {
            // No specific rule for this child — include as-is
            output.push((key.clone(), value.clone()));
        } else {
            // Apply priority: deny > allow > redact
            let has_deny = matching.iter().any(|(_, _, r)| r.permission == AccessPermission::Deny);
            let has_allow = matching.iter().any(|(_, _, r)| r.permission == AccessPermission::Allow);
            let has_redact = matching.iter().any(|(_, _, r)| r.permission == AccessPermission::Redact);

            if has_deny {
                // Omit
            } else if has_allow {
                output.push((key.clone(), value.clone()));
            } else if has_redact {
                output.push((key.clone(), Value::Nil));
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::types::AccessSubjectType;

    fn make_rule(
        field: Option<&str>,
        record_id: Option<&str>,
        subject_type: AccessSubjectType,
        subject_id: &str,
        permission: AccessPermission,
    ) -> NodeAccessRule {
        NodeAccessRule {
            id: 0,
            collection: "test".to_string(),
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

    fn make_doc() -> Value {
        Value::Map(vec![
            (Value::String("name".into()), Value::String("Alice".into())),
            (Value::String("email".into()), Value::String("alice@example.com".into())),
            (Value::String("age".into()), Value::Integer(30.into())),
        ])
    }

    #[test]
    fn filter_allows_all_when_allowed() {
        let rules = vec![
            make_rule(None, None, AccessSubjectType::Peer, "alice", AccessPermission::Allow),
        ];
        let subject = peer_subject("alice");
        let doc = make_doc();
        let result = filter_document(&doc, &rules, &subject, None).unwrap();
        assert_eq!(result.as_map().unwrap().len(), 3);
    }

    #[test]
    fn filter_removes_denied_fields() {
        let rules = vec![
            make_rule(None, None, AccessSubjectType::Peer, "alice", AccessPermission::Allow),
            make_rule(Some("email"), None, AccessSubjectType::Peer, "alice", AccessPermission::Deny),
        ];
        let subject = peer_subject("alice");
        let doc = make_doc();
        let result = filter_document(&doc, &rules, &subject, None).unwrap();
        let map = result.as_map().unwrap();
        assert_eq!(map.len(), 2);
        assert!(map.iter().all(|(k, _)| k.as_str() != Some("email")));
    }

    #[test]
    fn filter_redacts_fields_to_nil() {
        let rules = vec![
            make_rule(None, None, AccessSubjectType::Peer, "alice", AccessPermission::Allow),
            make_rule(Some("email"), None, AccessSubjectType::Peer, "alice", AccessPermission::Redact),
        ];
        let subject = peer_subject("alice");
        let doc = make_doc();
        let result = filter_document(&doc, &rules, &subject, None).unwrap();
        let map = result.as_map().unwrap();
        assert_eq!(map.len(), 3);
        let email = map.iter().find(|(k, _)| k.as_str() == Some("email")).unwrap();
        assert_eq!(email.1, Value::Nil);
    }

    #[test]
    fn filter_denies_all_by_default() {
        let rules: Vec<NodeAccessRule> = vec![];
        let subject = peer_subject("alice");
        let doc = make_doc();
        let result = filter_document(&doc, &rules, &subject, None).unwrap();
        assert!(result.as_map().unwrap().is_empty());
    }

    #[test]
    fn filter_non_map_returns_error() {
        let rules: Vec<NodeAccessRule> = vec![];
        let subject = peer_subject("alice");
        let doc = Value::Integer(42.into());
        match filter_document(&doc, &rules, &subject, None) {
            Err(DacError::InvalidDocument) => {}
            other => panic!("expected InvalidDocument, got {:?}", other),
        }
    }

    #[test]
    fn filter_nested_dot_notation() {
        let doc = Value::Map(vec![
            (Value::String("name".into()), Value::String("Alice".into())),
            (Value::String("provenance".into()), Value::Map(vec![
                (Value::String("pkiId".into()), Value::String("key123".into())),
                (Value::String("source".into()), Value::String("local".into())),
            ])),
        ]);
        let rules = vec![
            make_rule(None, None, AccessSubjectType::Peer, "alice", AccessPermission::Allow),
            make_rule(Some("provenance.pkiId"), None, AccessSubjectType::Peer, "alice", AccessPermission::Redact),
        ];
        let subject = peer_subject("alice");
        let result = filter_document(&doc, &rules, &subject, None).unwrap();
        let map = result.as_map().unwrap();

        // provenance should still be there
        let prov = map.iter().find(|(k, _)| k.as_str() == Some("provenance")).unwrap();
        let prov_map = prov.1.as_map().unwrap();

        // pkiId should be redacted (Nil)
        let pki = prov_map.iter().find(|(k, _)| k.as_str() == Some("pkiId")).unwrap();
        assert_eq!(pki.1, Value::Nil);

        // source should still be present
        let source = prov_map.iter().find(|(k, _)| k.as_str() == Some("source")).unwrap();
        assert_eq!(source.1, Value::String("local".into()));
    }
}

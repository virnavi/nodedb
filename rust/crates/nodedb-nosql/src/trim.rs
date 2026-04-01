use std::collections::HashSet;

use chrono::{DateTime, Utc};
use rmpv::Value;
use serde::{Deserialize, Serialize};

use crate::error::NoSqlError;
use nodedb_storage::StorageTree;

/// Trim policy types.
///
/// All collections are never-trim by default. Trimming must be explicitly opted into.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrimPolicy {
    /// Trim records not accessed (any event) since the given duration (seconds).
    NotAccessedSince(i64),
    /// Trim records not read since the given duration (seconds).
    NotReadSince(i64),
    /// Only trim AI-originated records.
    AiOriginatedOnly,
    /// Trim records with provenance confidence below the threshold.
    ConfidenceBelow(f64),
    /// Trim LRU records until storage is within target bytes.
    ToTargetBytes(u64),
    /// Keep only the N most recently accessed records per collection.
    KeepMostRecentlyAccessed(usize),
    /// All sub-policies must match (AND).
    Compound(Vec<TrimPolicy>),
    /// At least one sub-policy must match (OR).
    Any(Vec<TrimPolicy>),
}

impl TrimPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrimPolicy::NotAccessedSince(_) => "not_accessed_since",
            TrimPolicy::NotReadSince(_) => "not_read_since",
            TrimPolicy::AiOriginatedOnly => "ai_originated_only",
            TrimPolicy::ConfidenceBelow(_) => "confidence_below",
            TrimPolicy::ToTargetBytes(_) => "to_target_bytes",
            TrimPolicy::KeepMostRecentlyAccessed(_) => "keep_most_recently_accessed",
            TrimPolicy::Compound(_) => "compound",
            TrimPolicy::Any(_) => "any",
        }
    }

    /// Parse a TrimPolicy from a MessagePack Value (used in FFI).
    pub fn from_value(val: &Value) -> Result<Self, NoSqlError> {
        let map = val.as_map().ok_or_else(|| {
            NoSqlError::TrimPolicyInvalid("expected map".to_string())
        })?;

        let type_str = map_field_str(map, "type").ok_or_else(|| {
            NoSqlError::TrimPolicyInvalid("missing 'type' field".to_string())
        })?;

        match type_str {
            "not_accessed_since" => {
                let secs = map_field_i64(map, "duration_secs").ok_or_else(|| {
                    NoSqlError::TrimPolicyInvalid("missing 'duration_secs'".to_string())
                })?;
                Ok(TrimPolicy::NotAccessedSince(secs))
            }
            "not_read_since" => {
                let secs = map_field_i64(map, "duration_secs").ok_or_else(|| {
                    NoSqlError::TrimPolicyInvalid("missing 'duration_secs'".to_string())
                })?;
                Ok(TrimPolicy::NotReadSince(secs))
            }
            "ai_originated_only" => Ok(TrimPolicy::AiOriginatedOnly),
            "confidence_below" => {
                let thresh = map_field_f64(map, "threshold").ok_or_else(|| {
                    NoSqlError::TrimPolicyInvalid("missing 'threshold'".to_string())
                })?;
                Ok(TrimPolicy::ConfidenceBelow(thresh))
            }
            "to_target_bytes" => {
                let bytes = map_field_i64(map, "max_bytes").ok_or_else(|| {
                    NoSqlError::TrimPolicyInvalid("missing 'max_bytes'".to_string())
                })? as u64;
                Ok(TrimPolicy::ToTargetBytes(bytes))
            }
            "keep_most_recently_accessed" => {
                let count = map_field_i64(map, "count").ok_or_else(|| {
                    NoSqlError::TrimPolicyInvalid("missing 'count'".to_string())
                })? as usize;
                Ok(TrimPolicy::KeepMostRecentlyAccessed(count))
            }
            "compound" => {
                let policies = map_field_array(map, "policies").ok_or_else(|| {
                    NoSqlError::TrimPolicyInvalid("missing 'policies'".to_string())
                })?;
                let parsed: Result<Vec<TrimPolicy>, _> = policies.iter().map(TrimPolicy::from_value).collect();
                Ok(TrimPolicy::Compound(parsed?))
            }
            "any" => {
                let policies = map_field_array(map, "policies").ok_or_else(|| {
                    NoSqlError::TrimPolicyInvalid("missing 'policies'".to_string())
                })?;
                let parsed: Result<Vec<TrimPolicy>, _> = policies.iter().map(TrimPolicy::from_value).collect();
                Ok(TrimPolicy::Any(parsed?))
            }
            _ => Err(NoSqlError::TrimPolicyInvalid(format!("unknown type: {}", type_str))),
        }
    }

    /// Convert to a MessagePack Value.
    pub fn to_value(&self) -> Value {
        match self {
            TrimPolicy::NotAccessedSince(secs) => Value::Map(vec![
                (Value::String("type".into()), Value::String("not_accessed_since".into())),
                (Value::String("duration_secs".into()), Value::Integer((*secs).into())),
            ]),
            TrimPolicy::NotReadSince(secs) => Value::Map(vec![
                (Value::String("type".into()), Value::String("not_read_since".into())),
                (Value::String("duration_secs".into()), Value::Integer((*secs).into())),
            ]),
            TrimPolicy::AiOriginatedOnly => Value::Map(vec![
                (Value::String("type".into()), Value::String("ai_originated_only".into())),
            ]),
            TrimPolicy::ConfidenceBelow(t) => Value::Map(vec![
                (Value::String("type".into()), Value::String("confidence_below".into())),
                (Value::String("threshold".into()), Value::F64(*t)),
            ]),
            TrimPolicy::ToTargetBytes(b) => Value::Map(vec![
                (Value::String("type".into()), Value::String("to_target_bytes".into())),
                (Value::String("max_bytes".into()), Value::Integer((*b as i64).into())),
            ]),
            TrimPolicy::KeepMostRecentlyAccessed(c) => Value::Map(vec![
                (Value::String("type".into()), Value::String("keep_most_recently_accessed".into())),
                (Value::String("count".into()), Value::Integer((*c as i64).into())),
            ]),
            TrimPolicy::Compound(policies) => Value::Map(vec![
                (Value::String("type".into()), Value::String("compound".into())),
                (Value::String("policies".into()), Value::Array(policies.iter().map(|p| p.to_value()).collect())),
            ]),
            TrimPolicy::Any(policies) => Value::Map(vec![
                (Value::String("type".into()), Value::String("any".into())),
                (Value::String("policies".into()), Value::Array(policies.iter().map(|p| p.to_value()).collect())),
            ]),
        }
    }
}

/// A candidate record for trimming.
#[derive(Debug, Clone)]
pub struct TrimCandidate {
    pub collection: String,
    pub record_id: i64,
    pub last_accessed_at_utc: Option<DateTime<Utc>>,
    pub age_since_last_access_secs: Option<i64>,
    pub ai_originated: bool,
    pub confidence: Option<f64>,
    pub never_trim_protected: bool,
    pub reasons: Vec<String>,
}

impl TrimCandidate {
    pub fn to_value(&self) -> Value {
        let mut fields = vec![
            (Value::String("collection".into()), Value::String(self.collection.clone().into())),
            (Value::String("record_id".into()), Value::Integer(self.record_id.into())),
            (Value::String("ai_originated".into()), Value::Boolean(self.ai_originated)),
            (Value::String("never_trim_protected".into()), Value::Boolean(self.never_trim_protected)),
        ];

        if let Some(ts) = self.last_accessed_at_utc {
            fields.push((Value::String("last_accessed_at_utc".into()), Value::String(ts.to_rfc3339().into())));
        }
        if let Some(age) = self.age_since_last_access_secs {
            fields.push((Value::String("age_since_last_access_secs".into()), Value::Integer(age.into())));
        }
        if let Some(conf) = self.confidence {
            fields.push((Value::String("confidence".into()), Value::F64(conf)));
        }

        let reasons_val: Vec<Value> = self.reasons.iter().map(|r| Value::String(r.clone().into())).collect();
        fields.push((Value::String("reasons".into()), Value::Array(reasons_val)));

        Value::Map(fields)
    }
}

/// Per-collection recommendation within a TrimRecommendation.
#[derive(Debug, Clone)]
pub struct TrimCollectionRecommendation {
    pub collection: String,
    pub candidate_count: usize,
    pub candidates: Vec<TrimCandidate>,
}

impl TrimCollectionRecommendation {
    pub fn to_value(&self) -> Value {
        Value::Map(vec![
            (Value::String("collection".into()), Value::String(self.collection.clone().into())),
            (Value::String("candidate_count".into()), Value::Integer((self.candidate_count as i64).into())),
            (Value::String("candidates".into()), Value::Array(self.candidates.iter().map(|c| c.to_value()).collect())),
        ])
    }
}

/// Result of recommendTrim — a full trim recommendation.
#[derive(Debug, Clone)]
pub struct TrimRecommendation {
    pub total_candidate_count: usize,
    pub by_collection: Vec<TrimCollectionRecommendation>,
    pub generated_at_utc: DateTime<Utc>,
    pub policy: TrimPolicy,
}

impl TrimRecommendation {
    pub fn to_value(&self) -> Value {
        Value::Map(vec![
            (Value::String("total_candidate_count".into()), Value::Integer((self.total_candidate_count as i64).into())),
            (Value::String("by_collection".into()), Value::Array(self.by_collection.iter().map(|c| c.to_value()).collect())),
            (Value::String("generated_at_utc".into()), Value::String(self.generated_at_utc.to_rfc3339().into())),
            (Value::String("policy".into()), self.policy.to_value()),
        ])
    }
}

/// User-approved trim request.
#[derive(Debug, Clone)]
pub struct UserApprovedTrim {
    pub policy: TrimPolicy,
    pub confirmed_record_ids: Vec<(String, i64)>, // (collection, record_id)
    pub approval_note: Option<String>,
    pub approved_at_utc: DateTime<Utc>,
}

impl UserApprovedTrim {
    pub fn from_value(val: &Value) -> Result<Self, NoSqlError> {
        let map = val.as_map().ok_or_else(|| {
            NoSqlError::TrimPolicyInvalid("expected map for UserApprovedTrim".to_string())
        })?;

        let policy_val = map_field_value(map, "policy").ok_or_else(|| {
            NoSqlError::TrimPolicyInvalid("missing 'policy'".to_string())
        })?;
        let policy = TrimPolicy::from_value(policy_val)?;

        let ids_arr = map_field_array(map, "confirmed_record_ids").ok_or_else(|| {
            NoSqlError::TrimPolicyInvalid("missing 'confirmed_record_ids'".to_string())
        })?;

        let mut confirmed = Vec::new();
        for item in ids_arr {
            if let Some(pair) = item.as_map() {
                let pair_slice: &[(Value, Value)] = pair;
                let col = map_field_str(pair_slice, "collection").unwrap_or_default().to_string();
                let rid = map_field_i64(pair_slice, "record_id").unwrap_or(0);
                confirmed.push((col, rid));
            }
        }

        let note = map_field_str(map, "approval_note").map(|s| s.to_string());

        Ok(UserApprovedTrim {
            policy,
            confirmed_record_ids: confirmed,
            approval_note: note,
            approved_at_utc: Utc::now(),
        })
    }
}

/// Report of a completed trim operation.
#[derive(Debug, Clone)]
pub struct TrimReport {
    pub collection: String,
    pub candidate_count: usize,
    pub deleted_count: usize,
    pub skipped_count: usize,
    pub never_trim_skipped_count: usize,
    pub trigger_aborted_count: usize,
    pub dry_run: bool,
    pub executed_at_utc: DateTime<Utc>,
    pub deleted_record_ids: Vec<(String, i64)>, // (collection, record_id)
}

impl TrimReport {
    pub fn empty(collection: &str, dry_run: bool) -> Self {
        TrimReport {
            collection: collection.to_string(),
            candidate_count: 0,
            deleted_count: 0,
            skipped_count: 0,
            never_trim_skipped_count: 0,
            trigger_aborted_count: 0,
            dry_run,
            executed_at_utc: Utc::now(),
            deleted_record_ids: Vec::new(),
        }
    }

    pub fn to_value(&self) -> Value {
        let deleted_ids: Vec<Value> = self.deleted_record_ids.iter().map(|(col, id)| {
            Value::Map(vec![
                (Value::String("collection".into()), Value::String(col.clone().into())),
                (Value::String("record_id".into()), Value::Integer((*id).into())),
            ])
        }).collect();

        Value::Map(vec![
            (Value::String("collection".into()), Value::String(self.collection.clone().into())),
            (Value::String("candidate_count".into()), Value::Integer((self.candidate_count as i64).into())),
            (Value::String("deleted_count".into()), Value::Integer((self.deleted_count as i64).into())),
            (Value::String("skipped_count".into()), Value::Integer((self.skipped_count as i64).into())),
            (Value::String("never_trim_skipped_count".into()), Value::Integer((self.never_trim_skipped_count as i64).into())),
            (Value::String("trigger_aborted_count".into()), Value::Integer((self.trigger_aborted_count as i64).into())),
            (Value::String("dry_run".into()), Value::Boolean(self.dry_run)),
            (Value::String("executed_at_utc".into()), Value::String(self.executed_at_utc.to_rfc3339().into())),
            (Value::String("deleted_record_ids".into()), Value::Array(deleted_ids)),
        ])
    }
}

/// Runtime trim configuration.
///
/// Manages per-collection and per-record trim policy overrides.
/// Persisted across database restarts via sled trees.
pub struct TrimConfig {
    /// Collection-level runtime overrides (meta_key → serialized TrimPolicy).
    collection_overrides: StorageTree,
    /// Record-level overrides (composite key: `"{meta_key}\0{record_id}"` → "never_trim" or serialized TrimPolicy).
    record_overrides: StorageTree,
    /// Collections that have been explicitly marked as trimmable (with their policy).
    /// Loaded from annotation defaults — typically empty in Rust (set by Dart code gen).
    trimmable_collections: std::sync::Mutex<HashSet<String>>,
}

impl TrimConfig {
    pub fn new(collection_overrides: StorageTree, record_overrides: StorageTree) -> Self {
        TrimConfig {
            collection_overrides,
            record_overrides,
            trimmable_collections: std::sync::Mutex::new(HashSet::new()),
        }
    }

    /// Mark a collection as trimmable (annotation-level opt-in).
    pub fn mark_trimmable(&self, meta_key: &str) {
        self.trimmable_collections.lock().unwrap().insert(meta_key.to_string());
    }

    /// Set a runtime trim policy override for a collection.
    pub fn set_trim_policy(&self, meta_key: &str, policy: &TrimPolicy) -> Result<(), NoSqlError> {
        let bytes = rmp_serde::to_vec(&policy)
            .map_err(|e| NoSqlError::Serialization(e.to_string()))?;
        self.collection_overrides.insert(meta_key.as_bytes(), &bytes)?;
        Ok(())
    }

    /// Remove the runtime override, reverting to annotation default.
    pub fn reset_to_annotation_default(&self, meta_key: &str) -> Result<(), NoSqlError> {
        self.collection_overrides.remove(meta_key.as_bytes())?;
        Ok(())
    }

    /// Get the runtime override for a collection (if any).
    pub fn get_collection_override(&self, meta_key: &str) -> Result<Option<TrimPolicy>, NoSqlError> {
        match self.collection_overrides.get(meta_key.as_bytes())? {
            Some(bytes) => {
                let policy: TrimPolicy = rmp_serde::from_slice(&bytes)
                    .map_err(|e| NoSqlError::Serialization(e.to_string()))?;
                Ok(Some(policy))
            }
            None => Ok(None),
        }
    }

    /// Check if a collection is effectively never-trim.
    ///
    /// Precedence: runtime override > annotation > global default (never-trim).
    pub fn is_never_trim(&self, meta_key: &str) -> Result<bool, NoSqlError> {
        // If there's a runtime override, collection is trimmable
        if self.get_collection_override(meta_key)?.is_some() {
            return Ok(false);
        }
        // If annotation-marked as trimmable, not never-trim
        if self.trimmable_collections.lock().unwrap().contains(meta_key) {
            return Ok(false);
        }
        // Default: never-trim
        Ok(true)
    }

    /// Set a record-level override (never-trim or specific policy).
    pub fn set_record_never_trim(&self, meta_key: &str, record_id: i64) -> Result<(), NoSqlError> {
        let key = record_override_key(meta_key, record_id);
        self.record_overrides.insert(key.as_bytes(), b"never_trim")?;
        Ok(())
    }

    /// Set a record-level trim policy override.
    pub fn set_record_trim_policy(&self, meta_key: &str, record_id: i64, policy: &TrimPolicy) -> Result<(), NoSqlError> {
        let key = record_override_key(meta_key, record_id);
        let bytes = rmp_serde::to_vec(&policy)
            .map_err(|e| NoSqlError::Serialization(e.to_string()))?;
        self.record_overrides.insert(key.as_bytes(), &bytes)?;
        Ok(())
    }

    /// Clear a record-level override.
    pub fn clear_record_override(&self, meta_key: &str, record_id: i64) -> Result<(), NoSqlError> {
        let key = record_override_key(meta_key, record_id);
        self.record_overrides.remove(key.as_bytes())?;
        Ok(())
    }

    /// Check if a specific record is never-trim protected.
    pub fn is_record_never_trim(&self, meta_key: &str, record_id: i64) -> Result<bool, NoSqlError> {
        let key = record_override_key(meta_key, record_id);
        match self.record_overrides.get(key.as_bytes())? {
            Some(bytes) => Ok(bytes == b"never_trim"),
            None => Ok(false),
        }
    }

    /// Get record-level trim policy (if set and not never-trim).
    pub fn get_record_trim_policy(&self, meta_key: &str, record_id: i64) -> Result<Option<TrimPolicy>, NoSqlError> {
        let key = record_override_key(meta_key, record_id);
        match self.record_overrides.get(key.as_bytes())? {
            Some(bytes) => {
                if bytes == b"never_trim" {
                    return Ok(None);
                }
                let policy: TrimPolicy = rmp_serde::from_slice(&bytes)
                    .map_err(|e| NoSqlError::Serialization(e.to_string()))?;
                Ok(Some(policy))
            }
            None => Ok(None),
        }
    }
}

fn record_override_key(meta_key: &str, record_id: i64) -> String {
    format!("{}\0{}", meta_key, record_id)
}

// ── Value helpers ───────────────────────────────────────────────────────

fn map_field_str<'a>(map: &'a [(Value, Value)], key: &str) -> Option<&'a str> {
    for (k, v) in map {
        if let Value::String(s) = k {
            if s.as_str() == Some(key) {
                if let Value::String(vs) = v {
                    return vs.as_str();
                }
            }
        }
    }
    None
}

fn map_field_i64(map: &[(Value, Value)], key: &str) -> Option<i64> {
    for (k, v) in map {
        if let Value::String(s) = k {
            if s.as_str() == Some(key) {
                if let Value::Integer(i) = v {
                    return i.as_i64();
                }
            }
        }
    }
    None
}

fn map_field_f64(map: &[(Value, Value)], key: &str) -> Option<f64> {
    for (k, v) in map {
        if let Value::String(s) = k {
            if s.as_str() == Some(key) {
                return match v {
                    Value::F64(f) => Some(*f),
                    Value::F32(f) => Some(*f as f64),
                    Value::Integer(i) => i.as_f64(),
                    _ => None,
                };
            }
        }
    }
    None
}

fn map_field_value<'a>(map: &'a [(Value, Value)], key: &str) -> Option<&'a Value> {
    for (k, v) in map {
        if let Value::String(s) = k {
            if s.as_str() == Some(key) {
                return Some(v);
            }
        }
    }
    None
}

fn map_field_array<'a>(map: &'a [(Value, Value)], key: &str) -> Option<&'a Vec<Value>> {
    for (k, v) in map {
        if let Value::String(s) = k {
            if s.as_str() == Some(key) {
                if let Value::Array(arr) = v {
                    return Some(arr);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use nodedb_storage::StorageEngine;

    fn open_trim_config(dir: &TempDir) -> TrimConfig {
        let engine = StorageEngine::open(dir.path()).unwrap();
        let col_tree = engine.open_tree("__trim_config__").unwrap();
        let rec_tree = engine.open_tree("__trim_record_overrides__").unwrap();
        TrimConfig::new(col_tree, rec_tree)
    }

    #[test]
    fn test_trim_policy_value_roundtrip() {
        let policies = vec![
            TrimPolicy::NotAccessedSince(86400),
            TrimPolicy::NotReadSince(3600),
            TrimPolicy::AiOriginatedOnly,
            TrimPolicy::ConfidenceBelow(0.5),
            TrimPolicy::ToTargetBytes(1_000_000),
            TrimPolicy::KeepMostRecentlyAccessed(100),
        ];
        for p in policies {
            let val = p.to_value();
            let parsed = TrimPolicy::from_value(&val).unwrap();
            assert_eq!(p.as_str(), parsed.as_str());
        }
    }

    #[test]
    fn test_compound_policy_roundtrip() {
        let policy = TrimPolicy::Compound(vec![
            TrimPolicy::NotAccessedSince(86400),
            TrimPolicy::ConfidenceBelow(0.3),
        ]);
        let val = policy.to_value();
        let parsed = TrimPolicy::from_value(&val).unwrap();
        assert_eq!(parsed.as_str(), "compound");
    }

    #[test]
    fn test_any_policy_roundtrip() {
        let policy = TrimPolicy::Any(vec![
            TrimPolicy::AiOriginatedOnly,
            TrimPolicy::NotReadSince(3600),
        ]);
        let val = policy.to_value();
        let parsed = TrimPolicy::from_value(&val).unwrap();
        assert_eq!(parsed.as_str(), "any");
    }

    #[test]
    fn test_trim_config_default_never_trim() {
        let dir = TempDir::new().unwrap();
        let config = open_trim_config(&dir);
        assert!(config.is_never_trim("public::users").unwrap());
    }

    #[test]
    fn test_trim_config_set_override() {
        let dir = TempDir::new().unwrap();
        let config = open_trim_config(&dir);

        config.set_trim_policy("public::users", &TrimPolicy::NotAccessedSince(86400)).unwrap();
        assert!(!config.is_never_trim("public::users").unwrap());

        let policy = config.get_collection_override("public::users").unwrap().unwrap();
        assert_eq!(policy.as_str(), "not_accessed_since");
    }

    #[test]
    fn test_trim_config_reset_override() {
        let dir = TempDir::new().unwrap();
        let config = open_trim_config(&dir);

        config.set_trim_policy("public::users", &TrimPolicy::NotAccessedSince(86400)).unwrap();
        config.reset_to_annotation_default("public::users").unwrap();
        assert!(config.is_never_trim("public::users").unwrap());
    }

    #[test]
    fn test_trim_config_trimmable_annotation() {
        let dir = TempDir::new().unwrap();
        let config = open_trim_config(&dir);

        config.mark_trimmable("public::logs");
        assert!(!config.is_never_trim("public::logs").unwrap());
    }

    #[test]
    fn test_record_never_trim() {
        let dir = TempDir::new().unwrap();
        let config = open_trim_config(&dir);

        assert!(!config.is_record_never_trim("public::users", 1).unwrap());
        config.set_record_never_trim("public::users", 1).unwrap();
        assert!(config.is_record_never_trim("public::users", 1).unwrap());
    }

    #[test]
    fn test_record_override_clear() {
        let dir = TempDir::new().unwrap();
        let config = open_trim_config(&dir);

        config.set_record_never_trim("public::users", 1).unwrap();
        config.clear_record_override("public::users", 1).unwrap();
        assert!(!config.is_record_never_trim("public::users", 1).unwrap());
    }

    #[test]
    fn test_record_trim_policy_override() {
        let dir = TempDir::new().unwrap();
        let config = open_trim_config(&dir);

        config.set_record_trim_policy("public::users", 1, &TrimPolicy::NotAccessedSince(3600)).unwrap();
        let policy = config.get_record_trim_policy("public::users", 1).unwrap().unwrap();
        assert_eq!(policy.as_str(), "not_accessed_since");
    }

    #[test]
    fn test_trim_report_to_value() {
        let report = TrimReport {
            collection: "users".to_string(),
            candidate_count: 10,
            deleted_count: 5,
            skipped_count: 3,
            never_trim_skipped_count: 2,
            trigger_aborted_count: 0,
            dry_run: false,
            executed_at_utc: Utc::now(),
            deleted_record_ids: vec![("users".to_string(), 1), ("users".to_string(), 2)],
        };
        let val = report.to_value();
        assert!(val.is_map());
    }

    #[test]
    fn test_trim_candidate_to_value() {
        let candidate = TrimCandidate {
            collection: "users".to_string(),
            record_id: 42,
            last_accessed_at_utc: Some(Utc::now()),
            age_since_last_access_secs: Some(86400),
            ai_originated: false,
            confidence: Some(0.75),
            never_trim_protected: false,
            reasons: vec!["not_accessed_since".to_string()],
        };
        let val = candidate.to_value();
        assert!(val.is_map());
    }

    #[test]
    fn test_trim_recommendation_to_value() {
        let rec = TrimRecommendation {
            total_candidate_count: 5,
            by_collection: vec![],
            generated_at_utc: Utc::now(),
            policy: TrimPolicy::NotAccessedSince(86400),
        };
        let val = rec.to_value();
        assert!(val.is_map());
    }

    #[test]
    fn test_invalid_policy_type() {
        let val = Value::Map(vec![
            (Value::String("type".into()), Value::String("bogus".into())),
        ]);
        assert!(TrimPolicy::from_value(&val).is_err());
    }

    #[test]
    fn test_policy_missing_type() {
        let val = Value::Map(vec![]);
        assert!(TrimPolicy::from_value(&val).is_err());
    }
}

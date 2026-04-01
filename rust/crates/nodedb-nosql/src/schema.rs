use serde::{Deserialize, Serialize};

/// Default schema name for collections without explicit schema.
pub const DEFAULT_SCHEMA: &str = "public";

/// Separator used in meta tree keys: `{schema}::{collection}`.
pub const META_KEY_SEPARATOR: &str = "::";

/// Reserved schema for security-related data (provenance, federation, DAC).
pub const SECURITY_SCHEMA: &str = "security";

/// All reserved schemas that cannot be modified by user writes.
pub const RESERVED_SCHEMAS: &[&str] = &[SECURITY_SCHEMA];

/// Returns true if the given schema name is a reserved system schema.
pub fn is_reserved_schema(name: &str) -> bool {
    RESERVED_SCHEMAS.contains(&name)
}

/// A fully qualified collection name: `{database}.{schema}.{collection}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedName {
    /// The database name. `None` means the local database.
    pub database: Option<String>,
    /// The schema name (default: "public").
    pub schema: String,
    /// The collection name.
    pub collection: String,
}

impl QualifiedName {
    /// Parse a collection reference into its qualified parts.
    ///
    /// - `"users"` → `(None, "public", "users")`
    /// - `"analytics.page_views"` → `(None, "analytics", "page_views")`
    /// - `"warehouse.public.products"` → `(Some("warehouse"), "public", "products")`
    pub fn parse(input: &str) -> Self {
        let parts: Vec<&str> = input.splitn(3, '.').collect();
        match parts.len() {
            1 => QualifiedName {
                database: None,
                schema: DEFAULT_SCHEMA.to_string(),
                collection: parts[0].to_string(),
            },
            2 => QualifiedName {
                database: None,
                schema: parts[0].to_string(),
                collection: parts[1].to_string(),
            },
            _ => QualifiedName {
                database: Some(parts[0].to_string()),
                schema: parts[1].to_string(),
                collection: parts[2].to_string(),
            },
        }
    }

    /// Returns the meta tree key: `"{schema}::{collection}"`.
    pub fn meta_key(&self) -> String {
        format!("{}{}{}", self.schema, META_KEY_SEPARATOR, self.collection)
    }

    /// Returns true if this refers to the local database.
    pub fn is_local(&self, current_db_name: Option<&str>) -> bool {
        match &self.database {
            None => true,
            Some(db) if db == "local" => true,
            Some(db) => current_db_name.map_or(false, |name| name == db),
        }
    }

    /// Returns the display FQN: `"{schema}.{collection}"` or `"{database}.{schema}.{collection}"`.
    pub fn display_name(&self) -> String {
        match &self.database {
            Some(db) => format!("{}.{}.{}", db, self.schema, self.collection),
            None => format!("{}.{}", self.schema, self.collection),
        }
    }
}

/// Schema entry stored in the collections meta tree.
/// Replaces the old `CollectionSchema` format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEntry {
    /// Schema this collection belongs to.
    pub schema_name: String,
    /// Bare collection name.
    pub collection_name: String,
    /// Actual sled tree name (may differ from logical name for legacy collections).
    pub tree_name: String,
    /// Optional sharing status override for this collection (string form).
    #[serde(default)]
    pub sharing_status: Option<String>,
    /// When this collection was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Whether this collection is a singleton (exactly one record, ID=1).
    #[serde(default)]
    pub singleton: bool,
    /// Collection type tag (e.g. "preferences").
    #[serde(default)]
    pub collection_type: Option<String>,
}

/// Schema-level metadata stored in the schema meta tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMetadata {
    /// Schema name.
    pub name: String,
    /// Optional sharing status override for this schema (string form).
    #[serde(default)]
    pub sharing_status: Option<String>,
    /// When this schema was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Legacy collection schema format (v1.1 and earlier).
/// Kept for deserialization during auto-migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSchema {
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl CollectionSchema {
    pub fn new(name: &str) -> Self {
        CollectionSchema {
            name: name.to_string(),
            created_at: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bare_name() {
        let qn = QualifiedName::parse("users");
        assert_eq!(qn.database, None);
        assert_eq!(qn.schema, "public");
        assert_eq!(qn.collection, "users");
    }

    #[test]
    fn parse_schema_dot_collection() {
        let qn = QualifiedName::parse("analytics.page_views");
        assert_eq!(qn.database, None);
        assert_eq!(qn.schema, "analytics");
        assert_eq!(qn.collection, "page_views");
    }

    #[test]
    fn parse_full_fqn() {
        let qn = QualifiedName::parse("warehouse.public.products");
        assert_eq!(qn.database, Some("warehouse".to_string()));
        assert_eq!(qn.schema, "public");
        assert_eq!(qn.collection, "products");
    }

    #[test]
    fn parse_fqn_with_dots_in_collection() {
        // 3-part split with extra dots goes into collection
        let qn = QualifiedName::parse("db.schema.coll.extra");
        assert_eq!(qn.database, Some("db".to_string()));
        assert_eq!(qn.schema, "schema");
        assert_eq!(qn.collection, "coll.extra");
    }

    #[test]
    fn meta_key_format() {
        let qn = QualifiedName::parse("analytics.page_views");
        assert_eq!(qn.meta_key(), "analytics::page_views");

        let qn2 = QualifiedName::parse("users");
        assert_eq!(qn2.meta_key(), "public::users");
    }

    #[test]
    fn is_local_none_database() {
        let qn = QualifiedName::parse("users");
        assert!(qn.is_local(None));
        assert!(qn.is_local(Some("mydb")));
    }

    #[test]
    fn is_local_explicit_local() {
        let qn = QualifiedName::parse("local.public.users");
        assert!(qn.is_local(None));
        assert!(qn.is_local(Some("mydb")));
    }

    #[test]
    fn is_local_matching_db_name() {
        let qn = QualifiedName::parse("warehouse.public.products");
        assert!(!qn.is_local(None));
        assert!(qn.is_local(Some("warehouse")));
        assert!(!qn.is_local(Some("other")));
    }

    #[test]
    fn display_name() {
        let qn = QualifiedName::parse("users");
        assert_eq!(qn.display_name(), "public.users");

        let qn2 = QualifiedName::parse("warehouse.analytics.events");
        assert_eq!(qn2.display_name(), "warehouse.analytics.events");
    }

    #[test]
    fn schema_entry_serde_roundtrip() {
        let entry = SchemaEntry {
            schema_name: "analytics".to_string(),
            collection_name: "page_views".to_string(),
            tree_name: "analytics::page_views".to_string(),
            sharing_status: Some("read_only".to_string()),
            singleton: false,
            collection_type: None,
            created_at: chrono::Utc::now(),
        };
        let bytes = rmp_serde::to_vec(&entry).unwrap();
        let decoded: SchemaEntry = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.schema_name, "analytics");
        assert_eq!(decoded.collection_name, "page_views");
        assert_eq!(decoded.tree_name, "analytics::page_views");
        assert_eq!(decoded.sharing_status, Some("read_only".to_string()));
        assert!(!decoded.singleton);
        assert_eq!(decoded.collection_type, None);
    }

    #[test]
    fn schema_entry_serde_no_sharing_status() {
        let entry = SchemaEntry {
            schema_name: "public".to_string(),
            collection_name: "users".to_string(),
            tree_name: "users".to_string(),
            sharing_status: None,
            singleton: false,
            collection_type: None,
            created_at: chrono::Utc::now(),
        };
        let bytes = rmp_serde::to_vec(&entry).unwrap();
        let decoded: SchemaEntry = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.sharing_status, None);
    }

    #[test]
    fn schema_entry_singleton_roundtrip() {
        let entry = SchemaEntry {
            schema_name: "public".to_string(),
            collection_name: "settings".to_string(),
            tree_name: "settings".to_string(),
            sharing_status: None,
            singleton: true,
            collection_type: None,
            created_at: chrono::Utc::now(),
        };
        let bytes = rmp_serde::to_vec(&entry).unwrap();
        let decoded: SchemaEntry = rmp_serde::from_slice(&bytes).unwrap();
        assert!(decoded.singleton);
    }

    #[test]
    fn schema_entry_collection_type_roundtrip() {
        let entry = SchemaEntry {
            schema_name: "public".to_string(),
            collection_name: "app_prefs".to_string(),
            tree_name: "app_prefs".to_string(),
            sharing_status: None,
            singleton: false,
            collection_type: Some("preferences".to_string()),
            created_at: chrono::Utc::now(),
        };
        let bytes = rmp_serde::to_vec(&entry).unwrap();
        let decoded: SchemaEntry = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.collection_type, Some("preferences".to_string()));
    }

    #[test]
    fn schema_entry_backward_compat_no_singleton_or_type() {
        // Simulate old entry without singleton/collection_type fields
        #[derive(serde::Serialize)]
        struct OldSchemaEntry {
            schema_name: String,
            collection_name: String,
            tree_name: String,
            #[serde(default)]
            sharing_status: Option<String>,
            created_at: chrono::DateTime<chrono::Utc>,
        }
        let old = OldSchemaEntry {
            schema_name: "public".to_string(),
            collection_name: "users".to_string(),
            tree_name: "users".to_string(),
            sharing_status: None,
            created_at: chrono::Utc::now(),
        };
        let bytes = rmp_serde::to_vec(&old).unwrap();
        let decoded: SchemaEntry = rmp_serde::from_slice(&bytes).unwrap();
        assert!(!decoded.singleton);
        assert_eq!(decoded.collection_type, None);
    }

    #[test]
    fn schema_metadata_serde_roundtrip() {
        let meta = SchemaMetadata {
            name: "analytics".to_string(),
            sharing_status: Some("private".to_string()),
            created_at: chrono::Utc::now(),
        };
        let bytes = rmp_serde::to_vec(&meta).unwrap();
        let decoded: SchemaMetadata = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.name, "analytics");
        assert_eq!(decoded.sharing_status, Some("private".to_string()));
    }

    #[test]
    fn legacy_collection_schema_still_deserializes() {
        let old = CollectionSchema::new("users");
        let bytes = rmp_serde::to_vec(&old).unwrap();
        let decoded: CollectionSchema = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.name, "users");
    }
}

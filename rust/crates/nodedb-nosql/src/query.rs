use serde::{Deserialize, Serialize};

use crate::document::Document;
use crate::filter::{compare_field_values, Filter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortField {
    pub field: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Query {
    pub filter: Option<Filter>,
    pub sort: Vec<SortField>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

impl Query {
    pub fn new() -> Self {
        Query::default()
    }

    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_sort(mut self, field: &str, direction: SortDirection) -> Self {
        self.sort.push(SortField {
            field: field.to_string(),
            direction,
        });
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn apply(&self, docs: Vec<Document>) -> Vec<Document> {
        let mut result: Vec<Document> = if let Some(filter) = &self.filter {
            docs.into_iter().filter(|d| filter.matches(d)).collect()
        } else {
            docs
        };

        // Sort
        if !self.sort.is_empty() {
            result.sort_by(|a, b| {
                for sort_field in &self.sort {
                    let a_val = a.get_field(&sort_field.field);
                    let b_val = b.get_field(&sort_field.field);
                    let cmp = compare_field_values(a_val, b_val);
                    let cmp = match sort_field.direction {
                        SortDirection::Asc => cmp,
                        SortDirection::Desc => cmp.reverse(),
                    };
                    if cmp != std::cmp::Ordering::Equal {
                        return cmp;
                    }
                }
                std::cmp::Ordering::Equal
            });
        }

        // Offset
        if let Some(offset) = self.offset {
            result = result.into_iter().skip(offset).collect();
        }

        // Limit
        if let Some(limit) = self.limit {
            result.truncate(limit);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::FilterCondition;
    use rmpv::Value;

    fn make_docs() -> Vec<Document> {
        vec![
            doc("Charlie", 35),
            doc("Alice", 25),
            doc("Bob", 30),
            doc("Diana", 28),
        ]
    }

    fn doc(name: &str, age: i64) -> Document {
        let data = Value::Map(vec![
            (Value::String("name".into()), Value::String(name.into())),
            (Value::String("age".into()), Value::Integer(age.into())),
        ]);
        Document::new(0, "users", data)
    }

    #[test]
    fn test_filter_and_sort() {
        let docs = make_docs();
        let q = Query::new()
            .with_filter(Filter::Condition(FilterCondition::GreaterThan {
                field: "age".to_string(),
                value: Value::Integer(26.into()),
            }))
            .with_sort("age", SortDirection::Asc);

        let result = q.apply(docs);
        assert_eq!(result.len(), 3);
        assert_eq!(
            result[0].get_field("name").unwrap(),
            &Value::String("Diana".into())
        );
    }

    #[test]
    fn test_offset_limit() {
        let docs = make_docs();
        let q = Query::new()
            .with_sort("name", SortDirection::Asc)
            .with_offset(1)
            .with_limit(2);

        let result = q.apply(docs);
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0].get_field("name").unwrap(),
            &Value::String("Bob".into())
        );
        assert_eq!(
            result[1].get_field("name").unwrap(),
            &Value::String("Charlie".into())
        );
    }

    #[test]
    fn test_sort_desc() {
        let docs = make_docs();
        let q = Query::new().with_sort("age", SortDirection::Desc);

        let result = q.apply(docs);
        assert_eq!(
            result[0].get_field("name").unwrap(),
            &Value::String("Charlie".into())
        );
    }
}

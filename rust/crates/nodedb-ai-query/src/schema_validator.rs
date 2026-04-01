use rmpv::Value;
use crate::types::{AiQuerySchema, SchemaPropertyType};
use crate::error::AiQueryError;

/// Validate a MessagePack Value against an AiQuerySchema.
/// Checks: (1) data is a Map, (2) required fields present, (3) typed fields match.
pub fn validate(data: &Value, schema: &AiQuerySchema) -> Result<(), AiQueryError> {
    let map = match data.as_map() {
        Some(m) => m,
        None => return Err(AiQueryError::SchemaValidation("data must be a map".to_string())),
    };

    // Check required fields
    for field in &schema.required_fields {
        let found = map.iter().any(|(k, _)| k.as_str() == Some(field.as_str()));
        if !found {
            return Err(AiQueryError::SchemaValidation(
                format!("missing required field: {}", field),
            ));
        }
    }

    // Check field types
    for (field_name, expected_type) in &schema.field_types {
        if let Some((_, value)) = map.iter().find(|(k, _)| k.as_str() == Some(field_name.as_str())) {
            if !matches_type(value, expected_type) {
                return Err(AiQueryError::SchemaValidation(
                    format!("field '{}' expected type '{}', got incompatible value", field_name, expected_type.as_str()),
                ));
            }
        }
        // If field is not present but typed, that's ok — required_fields handles presence
    }

    Ok(())
}

fn matches_type(value: &Value, expected: &SchemaPropertyType) -> bool {
    match expected {
        SchemaPropertyType::String => value.is_str(),
        SchemaPropertyType::Integer => value.is_i64() || value.is_u64(),
        SchemaPropertyType::Float => value.is_f64() || value.is_i64() || value.is_u64(),
        SchemaPropertyType::Boolean => value.is_bool(),
        SchemaPropertyType::Array => value.is_array(),
        SchemaPropertyType::Map => value.is_map(),
        SchemaPropertyType::Any => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_data(fields: Vec<(&str, Value)>) -> Value {
        Value::Map(
            fields
                .into_iter()
                .map(|(k, v)| (Value::String(k.into()), v))
                .collect(),
        )
    }

    #[test]
    fn valid_data_passes() {
        let mut field_types = HashMap::new();
        field_types.insert("name".to_string(), SchemaPropertyType::String);
        field_types.insert("price".to_string(), SchemaPropertyType::Float);
        let schema = AiQuerySchema {
            required_fields: vec!["name".to_string(), "price".to_string()],
            field_types,
        };
        let data = make_data(vec![
            ("name", Value::String("Widget".into())),
            ("price", Value::F64(9.99)),
        ]);
        assert!(validate(&data, &schema).is_ok());
    }

    #[test]
    fn missing_required_field() {
        let schema = AiQuerySchema {
            required_fields: vec!["name".to_string(), "price".to_string()],
            field_types: HashMap::new(),
        };
        let data = make_data(vec![("name", Value::String("Widget".into()))]);
        let err = validate(&data, &schema).unwrap_err();
        assert!(err.to_string().contains("missing required field: price"));
    }

    #[test]
    fn wrong_type_rejected() {
        let mut field_types = HashMap::new();
        field_types.insert("name".to_string(), SchemaPropertyType::String);
        let schema = AiQuerySchema {
            required_fields: vec![],
            field_types,
        };
        let data = make_data(vec![("name", Value::Integer(42.into()))]);
        let err = validate(&data, &schema).unwrap_err();
        assert!(err.to_string().contains("expected type 'string'"));
    }

    #[test]
    fn extra_fields_ok() {
        let schema = AiQuerySchema {
            required_fields: vec!["name".to_string()],
            field_types: HashMap::new(),
        };
        let data = make_data(vec![
            ("name", Value::String("Widget".into())),
            ("extra", Value::Boolean(true)),
        ]);
        assert!(validate(&data, &schema).is_ok());
    }

    #[test]
    fn any_type_always_passes() {
        let mut field_types = HashMap::new();
        field_types.insert("flexible".to_string(), SchemaPropertyType::Any);
        let schema = AiQuerySchema {
            required_fields: vec![],
            field_types,
        };
        let data = make_data(vec![("flexible", Value::Integer(42.into()))]);
        assert!(validate(&data, &schema).is_ok());
    }

    #[test]
    fn empty_schema_passes() {
        let schema = AiQuerySchema {
            required_fields: vec![],
            field_types: HashMap::new(),
        };
        let data = make_data(vec![("anything", Value::Boolean(true))]);
        assert!(validate(&data, &schema).is_ok());
    }

    #[test]
    fn non_map_data_rejected() {
        let schema = AiQuerySchema {
            required_fields: vec![],
            field_types: HashMap::new(),
        };
        let data = Value::String("not a map".into());
        let err = validate(&data, &schema).unwrap_err();
        assert!(err.to_string().contains("data must be a map"));
    }

    #[test]
    fn integer_accepted_as_float() {
        let mut field_types = HashMap::new();
        field_types.insert("value".to_string(), SchemaPropertyType::Float);
        let schema = AiQuerySchema {
            required_fields: vec![],
            field_types,
        };
        let data = make_data(vec![("value", Value::Integer(42.into()))]);
        assert!(validate(&data, &schema).is_ok());
    }
}

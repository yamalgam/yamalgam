#![allow(missing_docs)]

use serde_json::json;
use yamalgam_core::{Mapping, Value};
use yamalgam_schema::{SchemaOptions, SchemaValidator};

fn schema(document: serde_json::Value) -> SchemaValidator {
    let value = serde_json::from_value(document).expect("schema fits Value");
    SchemaValidator::compile(&value, SchemaOptions::default()).expect("valid schema")
}

fn mapping(entries: impl IntoIterator<Item = (Value, Value)>) -> Value {
    let mut mapping = Mapping::new();
    for (key, value) in entries {
        mapping.insert(key, value);
    }
    Value::Mapping(mapping)
}

// r[verify validate.keys]
#[test]
fn scalar_keys_participate_in_properties_and_required() {
    let validator = schema(json!({
        "type": "object",
        "required": ["1", "true", "null"],
        "properties": {
            "1": {"type": "string"},
            "true": {"type": "integer"},
            "null": {"type": "number"}
        },
        "additionalProperties": false
    }));
    let instance = mapping([
        (Value::Integer(1), Value::from("meter")),
        (Value::Bool(true), Value::Integer(1)),
        (Value::Null, Value::Float(1.5)),
    ]);

    assert!(validator.validate(&instance).valid);
}

#[test]
fn projected_names_drive_pattern_and_additional_properties() {
    let validator = schema(json!({
        "type": "object",
        "patternProperties": {"^[0-9]+$": {"type": "string"}},
        "additionalProperties": false
    }));

    assert!(
        validator
            .validate(&mapping([(Value::Integer(42), Value::from("answer"))]))
            .valid
    );
    assert!(
        !validator
            .validate(&mapping([(Value::Bool(true), Value::from("answer"))]))
            .valid
    );
}

#[test]
fn property_names_always_validate_projected_strings() {
    let validator = schema(json!({"propertyNames": {"pattern": "^[a-z]+$"}}));

    assert!(
        validator
            .validate(&mapping([(Value::Bool(true), Value::Null)]))
            .valid
    );
    assert!(
        !validator
            .validate(&mapping([(Value::Integer(1), Value::Null)]))
            .valid
    );
}

// r[verify validate.keys]
#[test]
fn invalid_yaml_mapping_shapes_stop_before_json_schema() {
    let validator = schema(json!({"type": "object"}));
    let complex = mapping([(Value::Sequence(vec![Value::from("complex")]), Value::Null)]);
    let collision = mapping([
        (Value::Integer(1), Value::Null),
        (Value::Float(1.0), Value::Null),
        (Value::from("1"), Value::Null),
    ]);
    let non_finite = Value::Sequence(vec![Value::Float(f64::NAN)]);

    assert_eq!(
        validator.validate(&complex).diagnostics[0].code,
        "yaml/complex-key"
    );
    assert!(
        validator
            .validate(&collision)
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code == "yaml/projected-key-collision")
    );
    assert_eq!(
        validator.validate(&non_finite).diagnostics[0].code,
        "yaml/non-finite-number"
    );
}

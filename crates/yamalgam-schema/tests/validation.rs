#![allow(missing_docs)]

use serde_json::{Value as JsonValue, json};
use yamalgam_core::{Mapping, Value};
use yamalgam_schema::{SchemaError, SchemaOptions, SchemaValidator};

fn value(json: JsonValue) -> Value {
    serde_json::from_value(json).expect("test JSON fits Yamalgam Value")
}

fn compile(schema: JsonValue) -> SchemaValidator {
    SchemaValidator::compile(&value(schema), SchemaOptions::default()).expect("valid test schema")
}

fn assert_case(schema: JsonValue, valid: JsonValue, invalid: JsonValue) {
    let validator = compile(schema);
    assert!(validator.validate(&value(valid)).valid);
    assert!(!validator.validate(&value(invalid)).valid);
}

// r[verify validate]
// r[verify validate.engine]
// r[verify phases.validation]
#[test]
fn boolean_and_core_validation_keywords_use_draft_2020_12() {
    assert!(compile(json!(true)).validate(&Value::Null).valid);
    assert!(!compile(json!(false)).validate(&Value::Null).valid);

    let cases = [
        (
            json!({
                "type": "object",
                "required": ["name"],
                "properties": {"name": {"type": "string"}}
            }),
            json!({"name": "api"}),
            json!({"name": 42}),
        ),
        (
            json!({"type": "array", "items": {"type": "integer"}}),
            json!([1, 2]),
            json!([1, "two"]),
        ),
        (
            json!({
                "type": "array",
                "prefixItems": [{"type": "string"}, {"type": "integer"}],
                "items": false
            }),
            json!(["port", 443]),
            json!(["port", "443"]),
        ),
        (
            json!({"enum": ["red", "blue"]}),
            json!("red"),
            json!("green"),
        ),
        (json!({"const": 42}), json!(42.0), json!(41)),
        (
            json!({"type": "number", "minimum": 1, "exclusiveMaximum": 10}),
            json!(5),
            json!(10),
        ),
        (
            json!({"type": "string", "minLength": 2, "maxLength": 4, "pattern": "^[a-z]+$"}),
            json!("api"),
            json!("A"),
        ),
        (
            json!({"allOf": [{"type": "integer"}, {"minimum": 1}]}),
            json!(2),
            json!(0),
        ),
        (
            json!({"anyOf": [{"type": "string"}, {"type": "integer"}]}),
            json!(2),
            json!(false),
        ),
        (
            json!({"oneOf": [{"type": "string"}, {"type": "integer"}]}),
            json!("two"),
            json!(false),
        ),
        (json!({"not": {"type": "null"}}), json!(0), JsonValue::Null),
        (
            json!({
                "if": {"type": "integer"},
                "then": {"minimum": 1},
                "else": {"type": "string"}
            }),
            json!(2),
            json!(0),
        ),
        (
            json!({
                "$defs": {"port": {"type": "integer", "minimum": 1, "maximum": 65535}},
                "$ref": "#/$defs/port"
            }),
            json!(443),
            json!(70000),
        ),
    ];

    for (schema, valid, invalid) in cases {
        assert_case(schema, valid, invalid);
    }
}

#[test]
fn validation_sees_composed_anchors_and_merge_keys() {
    let instance = yamalgam_compose::from_str_single(
        "defaults: &defaults\n  timeout: 30\nservice:\n  <<: *defaults\n  name: api\n",
    )
    .expect("composed instance");
    let validator = compile(json!({
        "type": "object",
        "properties": {
            "service": {
                "type": "object",
                "required": ["timeout", "name"],
                "properties": {
                    "timeout": {"type": "integer"},
                    "name": {"const": "api"}
                }
            }
        }
    }));

    assert!(validator.validate(&instance).valid);
}

#[test]
fn schema_documents_reject_yaml_only_shapes() {
    let mut non_string = Mapping::new();
    non_string.insert(Value::Integer(1), Value::from("string"));
    assert!(matches!(
        SchemaValidator::compile(&Value::Mapping(non_string), SchemaOptions::default()),
        Err(SchemaError::NonStringKey { .. })
    ));

    assert!(matches!(
        SchemaValidator::compile(&Value::Float(f64::INFINITY), SchemaOptions::default()),
        Err(SchemaError::NonFiniteNumber { .. })
    ));
}

#[test]
fn linear_regex_and_meta_schema_failures_are_structured() {
    let pattern_result = SchemaValidator::compile(
        &value(json!({"type": "string", "pattern": "^(a+)\\1$"})),
        SchemaOptions::default(),
    );
    match pattern_result {
        Err(SchemaError::UnsupportedPattern { .. }) => {}
        Err(error) => panic!("unexpected pattern error: {error:?}"),
        Ok(_) => panic!("linear regex engine accepted a backreference"),
    }

    assert!(matches!(
        SchemaValidator::compile(&value(json!({"type": 42})), SchemaOptions::default(),),
        Err(SchemaError::InvalidSchema { .. })
    ));
}

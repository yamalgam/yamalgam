#![allow(missing_docs)]

use serde_json::json;
use yamalgam_core::{Severity, Span, Value};
use yamalgam_schema::{SchemaOptions, SchemaValidator, ValidationDiagnostic};

fn validator(schema: serde_json::Value, max_diagnostics: usize) -> SchemaValidator {
    let schema = serde_json::from_value(schema).expect("schema fits Value");
    SchemaValidator::compile(
        &schema,
        SchemaOptions {
            max_diagnostics,
            ..SchemaOptions::default()
        },
    )
    .expect("valid schema")
}

fn value(value: serde_json::Value) -> Value {
    serde_json::from_value(value).expect("JSON fixture fits Value")
}

fn key(diagnostic: &ValidationDiagnostic) -> (&str, &str, &str) {
    (
        &diagnostic.instance_path,
        &diagnostic.schema_path,
        &diagnostic.code,
    )
}

fn span_text(input: &str, span: Option<Span>) -> &str {
    let span = span.expect("diagnostic span");
    &input[span.start.offset..span.end.offset]
}

#[test]
fn diagnostics_are_sorted_and_summarize_common_keywords() {
    let validator = validator(
        json!({
            "type": "object",
            "required": ["missing"],
            "additionalProperties": false,
            "properties": {
                "a/b~c": {"type": "integer"},
                "color": {"enum": ["red", "blue"]},
                "name": {"type": "string"}
            }
        }),
        100,
    );
    let report = validator.validate(&value(json!({
        "z": true,
        "name": 12,
        "color": "green",
        "a/b~c": "nope"
    })));

    let keys = report.diagnostics.iter().map(key).collect::<Vec<_>>();
    assert_eq!(
        keys,
        [
            ("", "/additionalProperties", "schema/additionalProperties"),
            ("", "/required", "schema/required"),
            ("/a~1b~0c", "/properties/a~1b~0c/type", "schema/type"),
            ("/color", "/properties/color/enum", "schema/enum"),
            ("/name", "/properties/name/type", "schema/type"),
        ]
    );
    let additional = &report.diagnostics[0];
    assert_eq!(
        additional.expected.as_deref(),
        Some("no additional properties")
    );
    assert_eq!(additional.actual.as_deref(), Some("[\"z\"]"));
    let required = &report.diagnostics[1];
    assert_eq!(required.expected.as_deref(), Some("property \"missing\""));
    assert_eq!(required.actual.as_deref(), Some("missing"));
    let escaped_type = &report.diagnostics[2];
    assert_eq!(escaped_type.expected.as_deref(), Some("integer"));
    assert_eq!(escaped_type.actual.as_deref(), Some("string"));
    let enum_error = &report.diagnostics[3];
    assert_eq!(enum_error.expected.as_deref(), Some("[\"red\",\"blue\"]"));
    assert_eq!(enum_error.actual.as_deref(), Some("string"));
}

#[test]
fn diagnostics_route_projected_and_escaped_paths_to_original_value_spans() {
    let input = "1: 42\na/b~c: nope\n";
    let instance = yamalgam_compose::from_str_single_located(input).unwrap();
    let validator = validator(
        json!({
            "properties": {
                "1": {"type": "string"},
                "a/b~c": {"type": "integer"}
            }
        }),
        100,
    );
    let report = validator.validate_located(&instance);

    assert_eq!(report.diagnostics[0].instance_path, "/1");
    assert_eq!(span_text(input, report.diagnostics[0].span), "42");
    assert_eq!(report.diagnostics[1].instance_path, "/a~1b~0c");
    assert_eq!(span_text(input, report.diagnostics[1].span), "nope");
}

#[test]
fn diagnostics_cover_pattern_and_reference_traversal() {
    let pattern = validator(json!({"type": "string", "pattern": "^[a-z]+$"}), 100)
        .validate(&Value::from("123"));
    assert_eq!(pattern.diagnostics[0].code, "schema/pattern");
    assert_eq!(pattern.diagnostics[0].expected.as_deref(), Some("^[a-z]+$"));
    assert_eq!(pattern.diagnostics[0].actual.as_deref(), Some("string"));

    let referenced = validator(
        json!({
            "$defs": {"name": {"type": "string"}},
            "$ref": "#/$defs/name"
        }),
        100,
    )
    .validate(&Value::Integer(42));
    assert_eq!(referenced.diagnostics[0].code, "schema/type");
    assert_eq!(referenced.diagnostics[0].schema_path, "/$defs/name/type");
    assert_eq!(
        referenced.diagnostics[0].expected.as_deref(),
        Some("string")
    );
    assert_eq!(referenced.diagnostics[0].actual.as_deref(), Some("integer"));
}

#[test]
fn diagnostics_preserve_yaml_representation_spans_and_collision_labels() {
    let collision_input = "1: first\n1.0: second\n";
    let collision = yamalgam_compose::from_str_single_located(collision_input).unwrap();
    let report = validator(json!(true), 100).validate_located(&collision);
    assert_eq!(report.diagnostics[0].code, "yaml/projected-key-collision");
    assert_eq!(
        span_text(collision_input, report.diagnostics[0].span),
        "1.0"
    );
    assert_eq!(
        span_text(collision_input, Some(report.diagnostics[0].labels[0].span)),
        "1"
    );

    let complex_input = "[a]: value\n";
    let complex = yamalgam_compose::from_str_single_located(complex_input).unwrap();
    let report = validator(json!(true), 100).validate_located(&complex);
    assert_eq!(report.diagnostics[0].code, "yaml/complex-key");
    assert_eq!(span_text(complex_input, report.diagnostics[0].span), "[a]");

    let non_finite_input = "value: .nan\n";
    let non_finite = yamalgam_compose::from_str_single_located(non_finite_input).unwrap();
    let report = validator(json!(true), 100).validate_located(&non_finite);
    assert_eq!(report.diagnostics[0].code, "yaml/non-finite-number");
    assert_eq!(
        span_text(non_finite_input, report.diagnostics[0].span),
        ".nan"
    );
}

#[test]
fn diagnostics_append_one_warning_when_the_error_limit_is_reached() {
    let report = validator(json!({"items": {"type": "integer"}}), 2)
        .validate(&value(json!(["zero", "one", "two"])));

    assert!(!report.valid);
    assert_eq!(report.diagnostics.len(), 3);
    assert_eq!(report.diagnostics[0].instance_path, "/0");
    assert_eq!(report.diagnostics[1].instance_path, "/1");
    assert_eq!(report.diagnostics[2].code, "schema/diagnostic-limit");
    assert_eq!(report.diagnostics[2].severity, Severity::Warning);
}

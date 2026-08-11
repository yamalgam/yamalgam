//! Schema navigation contracts used by completion, hover, and definition.

use std::sync::Arc;

use serde_json::json;
use yamalgam_core::{LoaderConfig, ResolutionMode, Value};
use yamalgam_resolve::{MemoryRetriever, ResourceId, ResourceProvenance, RetrievedResource};
use yamalgam_schema::{SchemaDocument, SchemaOptions, SchemaSession, SchemaValidator};

fn value(json: serde_json::Value) -> Value {
    serde_json::from_value(json).expect("JSON fixture fits Yamalgam Value")
}

fn validator(schema: serde_json::Value) -> SchemaValidator {
    SchemaValidator::compile(
        &value(schema),
        SchemaOptions {
            base_uri: Some("memory://schemas/root.yaml".to_owned()),
            ..SchemaOptions::default()
        },
    )
    .expect("valid schema")
}

#[test]
fn navigation_traverses_properties_items_patterns_and_additional_properties() {
    let validator = validator(json!({
        "type": "object",
        "properties": {
            "users": {
                "type": "array",
                "prefixItems": [
                    {"type": "string", "enum": ["admin"], "description": "first user"}
                ],
                "items": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "default": "guest"}
                    },
                    "required": ["name"]
                }
            },
            "known": {"type": "number"}
        },
        "patternProperties": {
            "^x-": {"type": "integer", "description": "extension value"}
        },
        "additionalProperties": {"type": "boolean", "description": "fallback value"}
    }));

    let root = validator.context_at("");
    assert_eq!(
        root.properties
            .iter()
            .map(|candidate| candidate.name.as_str())
            .collect::<Vec<_>>(),
        vec!["users", "known"]
    );
    assert_eq!(validator.context_at("/users/0").types, vec!["string"]);
    assert_eq!(
        validator.context_at("/users/0").enum_values,
        vec!["\"admin\""]
    );

    let item = validator.context_at("/users/4");
    assert_eq!(
        item.properties
            .iter()
            .map(|candidate| (
                candidate.name.as_str(),
                candidate.required,
                candidate.default.as_deref(),
            ))
            .collect::<Vec<_>>(),
        vec![("name", true, Some("\"guest\""))]
    );
    assert_eq!(
        validator.context_at("/x-rate").description.as_deref(),
        Some("extension value")
    );
    assert_eq!(validator.context_at("/x-rate").types, vec!["integer"]);
    assert_eq!(
        validator.context_at("/other").description.as_deref(),
        Some("fallback value")
    );
    assert!(validator.context_at("/users/missing/name").types.is_empty());
}

#[test]
fn navigation_combines_applicators_and_deduplicates_in_source_order() {
    let validator = validator(json!({
        "title": "Service",
        "description": "service configuration",
        "type": "object",
        "properties": {
            "base": {"type": "string"},
            "shared": {"type": "string", "description": "first description"}
        },
        "required": ["base"],
        "allOf": [
            {
                "properties": {"all": {"type": "integer"}},
                "required": ["all"]
            },
            {
                "properties": {
                    "shared": {"type": "string", "description": "later description"}
                }
            }
        ],
        "anyOf": [
            {
                "properties": {"choice_a": {"type": "boolean"}},
                "required": ["choice_a"]
            },
            {"properties": {"choice_b": {"type": "boolean"}}}
        ],
        "oneOf": [
            {"properties": {"left": {"type": "null"}}},
            {"properties": {"right": {"type": "null"}}}
        ],
        "if": {"properties": {"flag": {"type": "boolean"}}},
        "then": {"properties": {"then_only": {"type": "string"}}},
        "else": {"properties": {"else_only": {"type": "string"}}}
    }));

    let context = validator.context_at("");
    assert_eq!(context.title.as_deref(), Some("Service"));
    assert_eq!(
        context.description.as_deref(),
        Some("service configuration")
    );
    assert_eq!(context.types, vec!["object"]);
    assert_eq!(
        context
            .properties
            .iter()
            .map(|candidate| (candidate.name.as_str(), candidate.required))
            .collect::<Vec<_>>(),
        vec![
            ("base", true),
            ("all", true),
            ("shared", false),
            ("choice_a", false),
            ("choice_b", false),
            ("left", false),
            ("right", false),
            ("flag", false),
            ("then_only", false),
            ("else_only", false),
        ]
    );
    assert_eq!(
        context.properties[2].description.as_deref(),
        Some("first description")
    );
}

#[test]
fn navigation_follows_local_anchors_and_stops_recursive_refs() {
    let source = concat!(
        "$id: memory://schemas/root.yaml\n",
        "$defs:\n",
        "  node:\n",
        "    $anchor: node\n",
        "    type: object\n",
        "    properties:\n",
        "      value: {type: string}\n",
        "      next: {$ref: '#node'}\n",
        "$ref: '#node'\n",
    );
    let located = yamalgam_compose::from_str_single_located(source).unwrap();
    let expected_span = located.span_at_pointer("/$defs/node");
    let validator = SchemaValidator::compile_located(
        &located,
        SchemaOptions {
            base_uri: Some("memory://schemas/root.yaml".to_owned()),
            ..SchemaOptions::default()
        },
    )
    .unwrap();

    assert_eq!(
        validator.context_at("/next/next/value").types,
        vec!["string"]
    );
    assert!(
        validator
            .context_at("/next/next/missing/deeper")
            .types
            .is_empty()
    );

    let definition = validator
        .definition_for_ref("memory://schemas/root.yaml", "#node")
        .expect("anchor definition is indexed");
    assert_eq!(definition.uri, "memory://schemas/root.yaml");
    assert_eq!(definition.pointer, "/$defs/node");
    assert_eq!(definition.span, expected_span);
}

#[test]
fn navigation_resolves_json_pointers_from_nested_id_resources() {
    let validator = validator(json!({
        "$id": "memory://schemas/root.yaml",
        "$defs": {
            "embedded": {
                "$id": "embedded.yaml",
                "$defs": {
                    "leaf": {"type": "string", "description": "embedded leaf"}
                },
                "type": "object",
                "properties": {
                    "value": {"$ref": "#/$defs/leaf"}
                }
            }
        },
        "type": "object",
        "properties": {
            "embedded": {"$ref": "embedded.yaml"}
        }
    }));

    let context = validator.context_at("/embedded/value");
    assert_eq!(context.types, vec!["string"]);
    assert_eq!(context.description.as_deref(), Some("embedded leaf"));

    let definition = validator
        .definition_for_ref("memory://schemas/embedded.yaml", "#/$defs/leaf")
        .expect("embedded pointer is indexed from its resource root");
    assert_eq!(definition.uri, "memory://schemas/root.yaml");
    assert_eq!(definition.pointer, "/$defs/embedded/$defs/leaf");
}

#[test]
fn navigation_follows_remote_refs_and_indexes_dynamic_anchors() {
    let mut retriever = MemoryRetriever::default();
    retriever.insert(RetrievedResource {
        id: ResourceId::new("memory://schemas/remote.yaml").unwrap(),
        bytes: concat!(
            "$id: memory://schemas/remote.yaml\n",
            "$defs:\n",
            "  count:\n",
            "    $dynamicAnchor: count\n",
            "    type: integer\n",
            "    description: remote count\n",
        )
        .as_bytes()
        .to_vec(),
        provenance: ResourceProvenance::default(),
    });
    let mut config = LoaderConfig::moderate();
    config.resolution.refs.mode = ResolutionMode::Automatic;
    config.resolution.refs.allow_schemes = vec!["memory".to_owned()];
    let session = SchemaSession::new(Arc::new(retriever), config);
    let root = SchemaDocument {
        id: ResourceId::new("memory://schemas/root.yaml").unwrap(),
        document: yamalgam_compose::from_str_single_located(
            "type: object\nproperties:\n  count: {$ref: ./remote.yaml#count}\n",
        )
        .unwrap(),
    };
    let validator = session.compile(root).unwrap();

    let context = validator.context_at("/count");
    assert_eq!(context.types, vec!["integer"]);
    assert_eq!(context.description.as_deref(), Some("remote count"));
    assert_eq!(
        context.schema_locations,
        vec![
            "memory://schemas/root.yaml#/properties/count",
            "memory://schemas/remote.yaml#/$defs/count",
        ]
    );

    let definition = validator
        .definition_for_ref("memory://schemas/root.yaml", "./remote.yaml#count")
        .expect("remote dynamic anchor is indexed");
    assert_eq!(definition.uri, "memory://schemas/remote.yaml");
    assert_eq!(definition.pointer, "/$defs/count");
    assert!(definition.span.is_some());
}

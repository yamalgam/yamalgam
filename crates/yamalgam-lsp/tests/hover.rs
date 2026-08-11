#![allow(missing_docs)]

use std::str::FromStr;
use std::sync::Arc;

use lsp_types::{HoverContents, MarkedString, Position, Uri};
use yamalgam_core::{LoaderConfig, ResolutionMode};
use yamalgam_lsp::{DocumentState, SchemaWorkspace, ServerConfig, hover};
use yamalgam_resolve::{MemoryRetriever, ResourceId, ResourceProvenance, RetrievedResource};
use yamalgam_schema::SchemaAssociation;

fn fixture_with_schema(schema: &str, text: &str) -> (SchemaWorkspace, DocumentState) {
    let schema_uri = "memory://schemas/hover";
    let mut retriever = MemoryRetriever::default();
    retriever.insert(RetrievedResource {
        id: ResourceId::new(schema_uri).unwrap(),
        bytes: schema.as_bytes().to_vec(),
        provenance: ResourceProvenance::default(),
    });
    let mut schema_config = LoaderConfig::trusted();
    schema_config.resolution.refs.mode = ResolutionMode::Explicit;
    schema_config.resolution.refs.allow_schemes = vec!["memory".to_owned()];
    let workspace = SchemaWorkspace::new(Arc::new(retriever), schema_config);
    let server = ServerConfig {
        schema_associations: vec![SchemaAssociation {
            uri: schema_uri.to_owned(),
            file_match: vec!["*.yaml".to_owned()],
        }],
        ..ServerConfig::default()
    };
    let mut document = DocumentState::new(
        Uri::from_str("file:///workspace/hover.yaml").unwrap(),
        1,
        text,
        &LoaderConfig::default(),
    );
    document.select_schemas(&server).unwrap();
    (workspace, document)
}

fn fixture(text: &str) -> (SchemaWorkspace, DocumentState) {
    fixture_with_schema(
        r#"{
          "type": "object",
          "properties": {
            "mode": {
              "title": "Run mode",
              "description": "Chooses <fast> or safe execution.",
              "type": ["string", "null"],
              "enum": ["fast", "safe", null],
              "default": "safe",
              "deprecated": true
            }
          }
        }"#,
        text,
    )
}

#[test]
fn hover_renders_schema_annotations_as_safe_plain_text() {
    let (schemas, document) = fixture("mode: fast\n");
    let hover = hover(&document, Position::new(0, 7), &schemas).unwrap();
    let HoverContents::Scalar(MarkedString::String(text)) = hover.contents else {
        panic!("expected plain-text hover");
    };
    assert!(text.contains("Run mode"));
    assert!(text.contains("Chooses <fast> or safe execution."));
    assert!(text.contains("Type: string | null"));
    assert!(text.contains("Default: \"safe\""));
    assert!(text.contains("Allowed: \"fast\", \"safe\", null"));
    assert!(text.contains("Deprecated"));
    assert!(text.contains("Schema: memory://schemas/hover"));
}

#[test]
fn hover_renders_bounded_deduplicated_examples_for_example_only_schema() {
    let schema = r#"{
      "examples": ["alpha", "alpha", {"mode": "fast"}],
      "allOf": [
        {"examples": [{"mode": "fast"}, 1, 2, 3, 4, 5, 6, 7, 8]}
      ]
    }"#;
    let (schemas, document) = fixture_with_schema(schema, "value\n");

    let hover = hover(&document, Position::new(0, 2), &schemas).unwrap();
    let HoverContents::Scalar(MarkedString::String(text)) = hover.contents else {
        panic!("expected plain-text hover");
    };
    assert!(text.contains("Examples: \"alpha\", {\"mode\":\"fast\"}, 1, 2, 3, 4, 5, 6"));
    assert!(!text.contains(", 7"));
}

#[test]
fn hover_keeps_a_leading_document_marker_in_document_zero() {
    let (schemas, document) = fixture("---\nmode: fast\n");

    let hover = hover(&document, Position::new(1, 7), &schemas).unwrap();
    let HoverContents::Scalar(MarkedString::String(text)) = hover.contents else {
        panic!("expected plain-text hover");
    };
    assert!(text.contains("Run mode"));
}

#[test]
fn hover_reaches_sequence_item_schemas() {
    let schema = r#"{
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "mode": {"title": "Sequence mode", "enum": ["fast", "safe"]}
        }
      }
    }"#;
    let (schemas, document) = fixture_with_schema(schema, "- mode: fast\n");

    let hover = hover(&document, Position::new(0, 10), &schemas).unwrap();
    let HoverContents::Scalar(MarkedString::String(text)) = hover.contents else {
        panic!("expected plain-text hover");
    };
    assert!(text.contains("Sequence mode"));
}

#[test]
fn hover_ignores_whitespace_comments_and_documents_without_schemas() {
    let (schemas, document) = fixture("mode: fast # comment\n");
    assert!(hover(&document, Position::new(0, 5), &schemas).is_none());
    assert!(hover(&document, Position::new(0, 14), &schemas).is_none());

    let plain = DocumentState::new(
        Uri::from_str("file:///workspace/plain.yaml").unwrap(),
        1,
        "mode: fast\n",
        &LoaderConfig::default(),
    );
    assert!(hover(&plain, Position::new(0, 7), &schemas).is_none());
}

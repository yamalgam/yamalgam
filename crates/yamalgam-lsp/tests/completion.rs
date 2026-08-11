#![allow(missing_docs)]

use std::str::FromStr;
use std::sync::Arc;

use lsp_types::{CompletionResponse, Position, Uri};
use yamalgam_core::{LoaderConfig, ResolutionMode};
use yamalgam_lsp::{DocumentState, SchemaWorkspace, ServerConfig, complete};
use yamalgam_resolve::{MemoryRetriever, ResourceId, ResourceProvenance, RetrievedResource};
use yamalgam_schema::SchemaAssociation;

fn fixture(schema: &str, text: &str) -> (SchemaWorkspace, DocumentState) {
    let schema_uri = "memory://schemas/completion";
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
        Uri::from_str("file:///workspace/completion.yaml").unwrap(),
        1,
        text,
        &LoaderConfig::default(),
    );
    document.select_schemas(&server).unwrap();
    (workspace, document)
}

fn items(response: CompletionResponse) -> Vec<lsp_types::CompletionItem> {
    match response {
        CompletionResponse::Array(items) => items,
        CompletionResponse::List(list) => list.items,
    }
}

#[test]
fn completion_orders_required_properties_and_suppresses_existing_keys() {
    let schema = r#"{
      "type": "object",
      "required": ["name"],
      "properties": {
        "name": {"type": "string"},
        "mode": {"enum": ["fast", "safe"], "default": "safe"}
      }
    }"#;
    let (schemas, empty) = fixture(schema, "");
    let empty_items = items(complete(&empty, Position::new(0, 0), &schemas));
    assert_eq!(
        empty_items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        vec!["name", "mode"]
    );
    assert_eq!(empty_items[0].sort_text.as_deref(), Some("0-name"));
    assert_eq!(
        empty_items[1].insert_text.as_deref(),
        Some("mode: \"safe\"")
    );

    let (schemas, existing) = fixture(schema, "name: yamalgam\n");
    let existing_items = items(complete(&existing, Position::new(1, 0), &schemas));
    assert_eq!(
        existing_items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        vec!["mode"]
    );
}

#[test]
fn completion_handles_nested_partial_mappings_and_enum_values() {
    let schema = r#"{
      "type": "object",
      "properties": {
        "service": {
          "type": "object",
          "required": ["name"],
          "properties": {"name": {"type": "string"}, "port": {"type": "integer"}}
        },
        "mode": {"enum": ["fast", "safe"]}
      }
    }"#;
    let (schemas, partial) = fixture(schema, "service:\n  na");
    let nested = items(complete(&partial, Position::new(1, 4), &schemas));
    assert!(nested.iter().any(|item| item.label == "name"));
    assert!(nested.iter().any(|item| item.label == "port"));

    let (schemas, value) = fixture(schema, "mode: \n");
    let values = items(complete(&value, Position::new(0, 6), &schemas));
    assert_eq!(
        values
            .iter()
            .map(|item| item.insert_text.as_deref().unwrap())
            .collect::<Vec<_>>(),
        vec!["\"fast\"", "\"safe\""]
    );
}

#[test]
fn completion_returns_no_candidates_without_an_active_schema() {
    let document = DocumentState::new(
        Uri::from_str("file:///workspace/plain.yaml").unwrap(),
        1,
        "name: value\n",
        &LoaderConfig::default(),
    );
    let response = complete(
        &document,
        Position::new(0, 2),
        &SchemaWorkspace::empty(LoaderConfig::default()),
    );
    assert!(items(response).is_empty());
}

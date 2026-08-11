#![allow(missing_docs)]

use std::str::FromStr;
use std::sync::Arc;

use lsp_types::{GotoDefinitionResponse, Position, Uri};
use yamalgam_core::{LoaderConfig, ResolutionMode};
use yamalgam_lsp::{DocumentState, SchemaWorkspace, ServerConfig, WorkspaceState, definition};
use yamalgam_resolve::{MemoryRetriever, ResourceId, ResourceProvenance, RetrievedResource};
use yamalgam_schema::SchemaAssociation;

fn link(response: GotoDefinitionResponse) -> lsp_types::LocationLink {
    let GotoDefinitionResponse::Link(mut links) = response else {
        panic!("expected location links");
    };
    assert_eq!(links.len(), 1);
    links.remove(0)
}

fn document(uri: &str, text: &str) -> DocumentState {
    DocumentState::new(
        Uri::from_str(uri).unwrap(),
        1,
        text,
        &LoaderConfig::default(),
    )
}

#[test]
fn definition_navigates_scalar_includes_and_data_ref_pointers() {
    let schemas = Arc::new(SchemaWorkspace::empty(LoaderConfig::default()));
    let mut workspace = WorkspaceState::new(Arc::clone(&schemas), Vec::new());
    workspace.insert_resource(
        Uri::from_str("file:///workspace/child.yaml").unwrap(),
        "nested:\n  value: target\n".to_owned(),
    );

    let include = document(
        "file:///workspace/root.yaml",
        "value: !include child.yaml\n",
    );
    let include_link = link(definition(&include, Position::new(0, 18), &workspace).unwrap());
    assert_eq!(
        include_link.target_uri.as_str(),
        "file:///workspace/child.yaml"
    );
    assert_eq!(
        include_link.origin_selection_range.unwrap().start.character,
        16
    );
    assert_eq!(
        include_link.origin_selection_range.unwrap().end.character,
        26
    );
    assert_eq!(
        include_link.target_selection_range.start,
        Position::new(0, 0)
    );

    let data_ref = document(
        "file:///workspace/root.yaml",
        "selected:\n  $ref: child.yaml#/nested/value\n",
    );
    let data_link = link(definition(&data_ref, Position::new(1, 17), &workspace).unwrap());
    assert_eq!(
        data_link.target_uri.as_str(),
        "file:///workspace/child.yaml"
    );
    assert_eq!(data_link.target_selection_range.start, Position::new(1, 9));
}

#[test]
fn definition_handles_same_document_refs_and_missing_or_denied_targets() {
    let schemas = Arc::new(SchemaWorkspace::empty(LoaderConfig::default()));
    let workspace = WorkspaceState::new(schemas, Vec::new());
    let same = document(
        "file:///workspace/root.yaml",
        "target:\n  value: here\nselected:\n  $ref: '#/target/value'\n",
    );
    let same_link = link(definition(&same, Position::new(3, 15), &workspace).unwrap());
    assert_eq!(same_link.target_uri, same.uri);
    assert_eq!(same_link.target_selection_range.start, Position::new(1, 9));

    let missing = document(
        "file:///workspace/root.yaml",
        "selected: {$ref: missing.yaml#/value}\n",
    );
    assert!(definition(&missing, Position::new(0, 25), &workspace).is_none());
}

#[test]
fn definition_uses_the_schema_navigation_index_for_schema_refs() {
    let uri = "memory://schemas/root";
    let schema = r##"{
      "$defs": {"name": {"type": "string"}},
      "$ref": "#/$defs/name"
    }"##;
    let mut retriever = MemoryRetriever::default();
    retriever.insert(RetrievedResource {
        id: ResourceId::new(uri).unwrap(),
        bytes: schema.as_bytes().to_vec(),
        provenance: ResourceProvenance::default(),
    });
    let mut loader = LoaderConfig::trusted();
    loader.resolution.refs.mode = ResolutionMode::Explicit;
    loader.resolution.refs.allow_schemes = vec!["memory".to_owned()];
    let schemas = Arc::new(SchemaWorkspace::new(Arc::new(retriever), loader));
    let mut workspace = WorkspaceState::new(Arc::clone(&schemas), Vec::new());
    workspace.insert_resource(Uri::from_str(uri).unwrap(), schema.to_owned());

    let server = ServerConfig {
        schema_associations: vec![SchemaAssociation {
            uri: uri.to_owned(),
            file_match: vec!["*".to_owned()],
        }],
        ..ServerConfig::default()
    };
    let mut document = document(uri, schema);
    document.select_schemas(&server).unwrap();
    let link = link(definition(&document, Position::new(2, 18), &workspace).unwrap());
    assert_eq!(link.target_uri.as_str(), uri);
    assert_eq!(link.target_selection_range.start.line, 1);
}

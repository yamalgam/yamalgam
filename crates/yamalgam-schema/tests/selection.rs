//! Schema-source precedence and SchemaStore matching contracts.

use std::collections::BTreeMap;

use yamalgam_resolve::ResourceId;
use yamalgam_schema::{
    SchemaAssociation, SchemaCatalog, SchemaCatalogEntry, SchemaSelectionError, SchemaSelector,
    SchemaSource,
};

fn association(uri: &str, patterns: &[&str]) -> SchemaAssociation {
    SchemaAssociation {
        uri: uri.to_owned(),
        file_match: patterns
            .iter()
            .map(|pattern| (*pattern).to_owned())
            .collect(),
    }
}

fn catalog() -> SchemaCatalog {
    serde_json::from_str(include_str!("fixtures/schemastore-catalog.json"))
        .expect("valid SchemaStore fixture")
}

fn selected_uri(
    selector: &SchemaSelector,
    input: &str,
    containing: &ResourceId,
    explicit: Option<&str>,
) -> (String, SchemaSource) {
    let selected = selector
        .select(input, Some(containing), explicit)
        .expect("selection succeeds");
    let selected = selected
        .first()
        .and_then(Option::as_ref)
        .expect("schema selected");
    (selected.uri.clone(), selected.source)
}

#[test]
fn selection_uses_the_documented_precedence() {
    let selector = SchemaSelector {
        associations: vec![association("memory://configured", &["*.yaml"])],
        schema_store: Some(catalog()),
    };
    let containing = ResourceId::from_file("/workspace/.github/workflows/build.yaml");
    let all_sources = "# yg-schema-validate: memory://pragma\n$schema: memory://inline\n";

    assert_eq!(
        selected_uri(
            &selector,
            all_sources,
            &containing,
            Some("memory://explicit")
        ),
        ("memory://explicit".to_owned(), SchemaSource::Explicit)
    );
    assert_eq!(
        selected_uri(&selector, all_sources, &containing, None),
        ("memory://pragma".to_owned(), SchemaSource::Pragma)
    );
    assert_eq!(
        selected_uri(
            &selector,
            "# yaml-language-server: $schema=memory://modeline\n$schema: memory://inline\n",
            &containing,
            None,
        ),
        ("memory://modeline".to_owned(), SchemaSource::Pragma)
    );
    assert_eq!(
        selected_uri(
            &selector,
            "# $schema: memory://modeline\n$schema: memory://inline\n",
            &containing,
            None,
        ),
        ("memory://modeline".to_owned(), SchemaSource::Pragma)
    );
    assert_eq!(
        selected_uri(&selector, "$schema: memory://inline\n", &containing, None,),
        ("memory://inline".to_owned(), SchemaSource::InlineProperty)
    );
    assert_eq!(
        selected_uri(&selector, "name: build\n", &containing, None),
        ("memory://configured".to_owned(), SchemaSource::Association)
    );

    let store_only = SchemaSelector {
        associations: Vec::new(),
        schema_store: Some(catalog()),
    };
    assert_eq!(
        selected_uri(&store_only, "name: build\n", &containing, None),
        (
            "memory://catalog/workflow".to_owned(),
            SchemaSource::SchemaStore
        )
    );
}

#[test]
fn selection_none_disables_every_lower_priority_source() {
    let selector = SchemaSelector {
        associations: vec![association("memory://configured", &["*.yaml"])],
        schema_store: Some(catalog()),
    };
    let containing = ResourceId::from_file("/workspace/example.yaml");

    let explicit = selector
        .select(
            "$schema: memory://inline\n",
            Some(&containing),
            Some("none"),
        )
        .unwrap();
    assert_eq!(explicit, vec![None]);

    let pragma = selector
        .select(
            "# yg-schema-validate: none\n$schema: memory://inline\n",
            Some(&containing),
            None,
        )
        .unwrap();
    assert_eq!(pragma, vec![None]);

    let inline = selector
        .select("$schema: none\n", Some(&containing), None)
        .unwrap();
    assert_eq!(inline, vec![None]);
}

#[test]
fn selection_directives_are_cst_comments_before_each_documents_first_node() {
    let selector = SchemaSelector::default();
    let containing = ResourceId::from_file("/workspace/config/document.yaml");
    let input = concat!(
        "# yg-schema-validate: ../schemas/first.json\n",
        "message: \"# yg-schema-validate: memory://quoted\"\n",
        "# yg-schema-validate: memory://late\n",
        "---\n",
        "# yg-schema-validate: ../schemas/second.json\n",
        "message: second\n",
    );

    let selected = selector.select(input, Some(&containing), None).unwrap();
    assert_eq!(selected.len(), 2);
    assert_eq!(
        selected[0].as_ref().map(|schema| schema.uri.as_str()),
        Some("file:///workspace/schemas/first.json")
    );
    assert_eq!(
        selected[1].as_ref().map(|schema| schema.uri.as_str()),
        Some("file:///workspace/schemas/second.json")
    );
}

#[test]
fn selection_malformed_and_conflicting_directives_are_structured_errors() {
    let selector = SchemaSelector::default();
    let containing = ResourceId::from_file("/workspace/document.yaml");

    let malformed = selector
        .select(
            "# yg-schema-validate: one.json two.json\nvalue: 1\n",
            Some(&containing),
            None,
        )
        .unwrap_err();
    assert!(matches!(
        malformed,
        SchemaSelectionError::MalformedDirective { document: 0, .. }
    ));

    let conflict = selector
        .select(
            "# $schema: one.json\n# yaml-language-server: $schema=two.json\nvalue: 1\n",
            Some(&containing),
            None,
        )
        .unwrap_err();
    assert!(matches!(
        conflict,
        SchemaSelectionError::ConflictingDirectives { document: 0, .. }
    ));
}

#[test]
fn selection_reports_missing_bases_and_invalid_globs() {
    let missing_base = SchemaSelector::default()
        .select("name: app\n", None, Some("schema.json"))
        .unwrap_err();
    assert!(matches!(
        missing_base,
        SchemaSelectionError::MissingBase { document: 0 }
    ));

    let invalid_glob = SchemaSelector {
        associations: vec![association("memory://configured", &["["])],
        schema_store: None,
    }
    .select(
        "name: app\n",
        Some(&ResourceId::from_file("/workspace/app.yaml")),
        None,
    )
    .unwrap_err();
    assert!(matches!(
        invalid_glob,
        SchemaSelectionError::InvalidGlob { .. }
    ));
}

#[test]
fn selection_schemastore_uses_longest_literal_then_catalog_order() {
    let catalog = catalog();
    assert_eq!(catalog.schemas[0].name, "Generic YAML");
    assert_eq!(
        catalog.schemas[0].description.as_deref(),
        Some("Generic files")
    );
    assert_eq!(
        catalog.schemas[0].versions.get("1.0").map(String::as_str),
        Some("memory://catalog/generic-v1")
    );

    let selector = SchemaSelector {
        associations: Vec::new(),
        schema_store: Some(catalog),
    };
    let containing = ResourceId::from_file("/repo/.github/workflows/build.yaml");
    assert_eq!(
        selected_uri(&selector, "name: CI\n", &containing, None),
        (
            "memory://catalog/workflow".to_owned(),
            SchemaSource::SchemaStore
        )
    );
}

#[test]
fn selection_association_patterns_and_paths_are_normalized_to_forward_slashes() {
    let selector = SchemaSelector {
        associations: vec![association(
            "memory://configured/windows",
            &["workspace\\config\\*.yaml"],
        )],
        schema_store: None,
    };
    let containing = ResourceId::from_file("/workspace/config/app.yaml");

    assert_eq!(
        selected_uri(&selector, "name: app\n", &containing, None),
        (
            "memory://configured/windows".to_owned(),
            SchemaSource::Association
        )
    );
}

#[test]
fn selection_public_catalog_entries_can_be_built_without_json() {
    let entry = SchemaCatalogEntry {
        name: "Local".to_owned(),
        description: None,
        file_match: vec!["*.yaml".to_owned()],
        url: "memory://local".to_owned(),
        versions: BTreeMap::new(),
    };
    let selector = SchemaSelector {
        associations: Vec::new(),
        schema_store: Some(SchemaCatalog {
            schemas: vec![entry],
        }),
    };
    let containing = ResourceId::from_file("/workspace/app.yaml");

    assert_eq!(
        selected_uri(&selector, "name: app\n", &containing, None),
        ("memory://local".to_owned(), SchemaSource::SchemaStore)
    );
}

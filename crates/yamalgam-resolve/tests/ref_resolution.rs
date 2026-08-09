//! Sole-key `$ref` replacement and RFC 6901 selection behavior.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use yamalgam_core::{LoaderConfig, ResolutionMode, TagResolution};
use yamalgam_parser::{Parser, ResolutionErrorCategory, ResolveError, ScalarStyle};
use yamalgam_resolve::{
    DiscoveryRequest, OwnedDocument, OwnedKind, OwnedNode, ResolutionSession, ResourceId,
    ResourceProvenance, ResourceRequest, ResourceRetriever, RetrieveError, RetrievedResource,
    SessionError,
};

#[derive(Clone, Default)]
struct CountingRetriever {
    state: Arc<Mutex<RetrieverState>>,
}

#[derive(Default)]
struct RetrieverState {
    resources: HashMap<ResourceId, RetrievedResource>,
    calls: Vec<ResourceId>,
}

impl CountingRetriever {
    fn insert(&self, id: ResourceId, input: impl Into<Vec<u8>>) {
        self.state.lock().unwrap().resources.insert(
            id.clone(),
            RetrievedResource {
                id,
                bytes: input.into(),
                provenance: ResourceProvenance::default(),
            },
        );
    }

    fn calls(&self) -> Vec<ResourceId> {
        self.state.lock().unwrap().calls.clone()
    }
}

impl ResourceRetriever for CountingRetriever {
    fn retrieve(&self, request: &ResourceRequest) -> Result<RetrievedResource, RetrieveError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(request.id.clone());
        state
            .resources
            .get(&request.id)
            .cloned()
            .ok_or_else(|| RetrieveError::NotFound(request.id.clone()))
    }

    fn discover(&self, request: &DiscoveryRequest) -> Result<Vec<ResourceId>, RetrieveError> {
        Err(RetrieveError::DiscoveryUnsupported(request.base.clone()))
    }
}

fn id(value: &str) -> ResourceId {
    ResourceId::new(value).unwrap()
}

fn document(origin: &ResourceId, input: &str) -> OwnedDocument {
    let events = Parser::new(input).collect::<Result<Vec<_>, _>>().unwrap();
    OwnedDocument::from_events(origin.clone(), events).unwrap()
}

fn config() -> LoaderConfig {
    let mut config = LoaderConfig::unchecked();
    config.resolution.refs.mode = ResolutionMode::Automatic;
    config.resolution.refs.allow_schemes = vec![
        "memory".into(),
        "file".into(),
        "http".into(),
        "https".into(),
    ];
    config.resolution.refs.allow_hosts = vec!["example.com".into()];
    config
}

fn resolve(
    retriever: CountingRetriever,
    origin: &ResourceId,
    input: &str,
) -> Result<OwnedDocument, ResolveError> {
    ResolutionSession::new(retriever, config()).resolve_root(document(origin, input))
}

fn scalar(node: &OwnedNode) -> &str {
    let OwnedKind::Scalar(scalar) = &node.kind else {
        panic!("expected scalar, got {:?}", node.kind);
    };
    &scalar.value
}

fn mapping_value<'a>(node: &'a OwnedNode, name: &str) -> &'a OwnedNode {
    let OwnedKind::Mapping { entries, .. } = &node.kind else {
        panic!("expected mapping, got {:?}", node.kind);
    };
    entries
        .iter()
        .find_map(|(key, value)| (scalar(key) == name).then_some(value))
        .unwrap_or_else(|| panic!("missing mapping key {name}"))
}

fn clear_spans(root: &mut OwnedNode) {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        node.span = Default::default();
        match &mut node.kind {
            OwnedKind::Sequence { items, .. } => pending.extend(items),
            OwnedKind::Mapping { entries, .. } => {
                for (key, value) in entries {
                    pending.push(key);
                    pending.push(value);
                }
            }
            OwnedKind::Scalar(_) | OwnedKind::Alias { .. } => {}
        }
    }
}

fn category(error: &ResolveError) -> ResolutionErrorCategory {
    match error {
        ResolveError::Resolution { category, .. } => *category,
        other => panic!("expected structured resolution error, got {other:?}"),
    }
}

#[test]
fn absent_and_empty_fragments_replace_with_the_whole_native_document() {
    let retriever = CountingRetriever::default();
    let root = id("memory://root/document.yaml");
    let shared = id("memory://shared/types.yaml");
    retriever.insert(
        shared.clone(),
        "null: null\nbool: true\ninteger: 42\nfloat: 3.5\nstring: \"42\"\nsequence: [a, b]\nmapping: {k: v}\n",
    );

    let output = resolve(
        retriever.clone(),
        &root,
        "- {$ref: 'memory://shared/types.yaml'}\n- {$ref: 'memory://shared/types.yaml#'}\n",
    )
    .unwrap();

    let OwnedKind::Sequence { items, .. } = &output.root.kind else {
        panic!("expected sequence root");
    };
    let mut first = items[0].clone();
    let mut second = items[1].clone();
    clear_spans(&mut first);
    clear_spans(&mut second);
    assert_eq!(first, second);
    assert_eq!(scalar(mapping_value(&items[0], "null")), "null");
    assert_eq!(scalar(mapping_value(&items[0], "bool")), "true");
    assert_eq!(scalar(mapping_value(&items[0], "integer")), "42");
    assert_eq!(scalar(mapping_value(&items[0], "float")), "3.5");
    let OwnedKind::Scalar(string) = &mapping_value(&items[0], "string").kind else {
        panic!("expected quoted string");
    };
    assert_eq!(string.style, ScalarStyle::DoubleQuoted);
    assert!(matches!(
        mapping_value(&items[0], "sequence").kind,
        OwnedKind::Sequence { .. }
    ));
    assert!(matches!(
        mapping_value(&items[0], "mapping").kind,
        OwnedKind::Mapping { .. }
    ));
    assert_eq!(retriever.calls(), vec![shared]);
}

// r[verify resolution.ref.fragment]
#[test]
fn pointer_selects_escaped_percent_encoded_mapping_keys_and_sequence_indices() {
    let retriever = CountingRetriever::default();
    let root = id("memory://root/document.yaml");
    let shared = id("memory://shared/pointer.yaml");
    retriever.insert(shared, "'a/b~☃': [zero, one]\n");

    let output = resolve(
        retriever,
        &root,
        "$ref: 'memory://shared/pointer.yaml#/a~1b~0%E2%98%83/1'\n",
    )
    .unwrap();

    assert_eq!(scalar(&output.root), "one");
}

#[test]
fn sequence_indices_reject_out_of_range_leading_zero_dash_and_non_numeric_tokens() {
    for token in ["2", "01", "-", "one"] {
        let retriever = CountingRetriever::default();
        let root = id("memory://root/document.yaml");
        let shared = id("memory://shared/sequence.yaml");
        retriever.insert(shared, "[zero, one]\n");

        let error = resolve(
            retriever,
            &root,
            &format!("$ref: 'memory://shared/sequence.yaml#/{token}'\n"),
        )
        .unwrap_err();

        assert_eq!(category(&error), ResolutionErrorCategory::MissingFragment);
    }
}

// r[verify resolution.ref]
#[test]
fn block_and_flow_sole_key_refs_resolve_but_siblings_and_non_strings_fail() {
    let root = id("memory://root/document.yaml");
    let shared = id("memory://shared/value.yaml");
    for input in [
        "$ref: 'memory://shared/value.yaml'\n",
        "{$ref: 'memory://shared/value.yaml'}\n",
    ] {
        let retriever = CountingRetriever::default();
        retriever.insert(shared.clone(), "resolved\n");
        assert_eq!(
            scalar(&resolve(retriever, &root, input).unwrap().root),
            "resolved"
        );
    }

    for input in [
        "$ref: ./shared.yaml#/defaults\nlocal: must-not-be-silently-merged\n",
        "$ref: [not, a, string]\n",
        "$ref: null\n",
        "$ref: true\n",
        "$ref: 42\n",
        "{$ref: one, '$ref': two}\n",
    ] {
        let error = resolve(CountingRetriever::default(), &root, input).unwrap_err();
        assert_eq!(
            category(&error),
            ResolutionErrorCategory::MalformedDirective,
            "input {input:?}"
        );
    }
}

#[test]
fn ref_string_validation_uses_the_configured_tag_resolution() {
    let retriever = CountingRetriever::default();
    let root = id("memory://root/document.yaml");
    let target = id("memory://root/null");
    retriever.insert(target, "resolved\n");

    let explicitly_string = resolve(retriever.clone(), &root, "$ref: !!str null\n").unwrap();
    assert_eq!(scalar(&explicitly_string.root), "resolved");

    let failsafe = config().with_tag_resolution(TagResolution::Failsafe);
    let plain_string = ResolutionSession::new(retriever.clone(), failsafe)
        .resolve_root(document(&root, "$ref: null\n"))
        .unwrap();
    assert_eq!(scalar(&plain_string.root), "resolved");

    let error = ResolutionSession::new(
        retriever,
        config().with_tag_resolution(TagResolution::Failsafe),
    )
    .resolve_root(document(&root, "$ref: !!int 42\n"))
    .unwrap_err();
    assert_eq!(
        category(&error),
        ResolutionErrorCategory::MalformedDirective
    );
}

#[test]
fn non_string_yaml_mapping_keys_cannot_be_selected_by_json_pointer() {
    let retriever = CountingRetriever::default();
    let root = id("memory://root/document.yaml");
    let shared = id("memory://shared/complex-key.yaml");
    retriever.insert(shared, "? [complex, key]\n: value\n");

    let error = resolve(
        retriever,
        &root,
        "$ref: 'memory://shared/complex-key.yaml#/anything'\n",
    )
    .unwrap_err();

    let ResolveError::Resolution {
        category,
        source: Some(source),
        ..
    } = error
    else {
        panic!("expected typed missing-fragment error");
    };
    assert_eq!(category, ResolutionErrorCategory::MissingFragment);
    assert!(matches!(
        source.downcast_ref::<SessionError>(),
        Some(SessionError::NonStringMappingKey)
    ));
}

#[test]
fn selection_happens_before_recursion() {
    let retriever = CountingRetriever::default();
    let root = id("memory://root/document.yaml");
    let shared = id("memory://shared/selection.yaml");
    retriever.insert(
        shared.clone(),
        "selected: retained\nignored: {$ref: 'memory://missing/document.yaml'}\n",
    );

    let output = resolve(
        retriever.clone(),
        &root,
        "$ref: 'memory://shared/selection.yaml#/selected'\n",
    )
    .unwrap();

    assert_eq!(scalar(&output.root), "retained");
    assert_eq!(retriever.calls(), vec![shared]);
}

// r[verify resolution.ref.base]
#[test]
fn relative_targets_use_the_containing_resource_at_every_level() {
    let retriever = CountingRetriever::default();
    let root = id("memory://example/root/app.yaml");
    let shared = id("memory://example/shared.yaml");
    let child = id("memory://example/child.yaml");
    retriever.insert(shared.clone(), "nested: {$ref: './child.yaml#/value'}\n");
    retriever.insert(child.clone(), "value: nested result\n");

    let output = resolve(retriever.clone(), &root, "$ref: '../shared.yaml#/nested'\n").unwrap();

    assert_eq!(scalar(&output.root), "nested result");
    assert_eq!(retriever.calls(), vec![shared, child]);
}

#[test]
fn absolute_file_http_and_https_targets_use_the_same_substitution_path() {
    let targets = [
        ResourceId::from_file("/tmp/yamalgam-ref-absolute.yaml"),
        id("http://example.com/value.yaml"),
        id("https://example.com/value.yaml"),
    ];

    for target in targets {
        let retriever = CountingRetriever::default();
        retriever.insert(target.clone(), "resolved\n");
        let root = id("memory://root/document.yaml");
        let input = format!("$ref: '{}'\n", target);

        let output = resolve(retriever.clone(), &root, &input).unwrap();

        assert_eq!(scalar(&output.root), "resolved");
        assert_eq!(retriever.calls(), vec![target]);
    }
}

#[test]
fn malformed_uri_pointer_fails_before_retrieval() {
    let retriever = CountingRetriever::default();
    let root = id("memory://root/document.yaml");

    let error = resolve(
        retriever.clone(),
        &root,
        "$ref: 'memory://shared/document.yaml#/%ZZ'\n",
    )
    .unwrap_err();

    assert_eq!(category(&error), ResolutionErrorCategory::InvalidResource);
    assert!(retriever.calls().is_empty());
}

#[test]
fn missing_mapping_keys_and_scalar_descent_are_missing_fragments() {
    for fragment in ["missing", "scalar/child"] {
        let retriever = CountingRetriever::default();
        let root = id("memory://root/document.yaml");
        let shared = id("memory://shared/document.yaml");
        retriever.insert(shared, "scalar: value\n");

        let error = resolve(
            retriever,
            &root,
            &format!("$ref: 'memory://shared/document.yaml#/{fragment}'\n"),
        )
        .unwrap_err();

        assert_eq!(category(&error), ResolutionErrorCategory::MissingFragment);
    }
}

#[test]
fn relative_files_and_absolute_file_uris_resolve_through_the_containing_origin() {
    let retriever = CountingRetriever::default();
    let root = ResourceId::from_file("/workspace/config/app.yaml");
    let shared = ResourceId::from_file("/workspace/shared.yaml");
    retriever.insert(shared.clone(), "relative file\n");

    let output = resolve(retriever.clone(), &root, "$ref: '../shared.yaml'\n").unwrap();

    assert_eq!(scalar(&output.root), "relative file");
    assert_eq!(retriever.calls(), vec![shared]);

    let retriever = CountingRetriever::default();
    let target_path = std::env::temp_dir().join("yamalgam-ref-file-uri.yaml");
    let target = ResourceId::from_file(&target_path);
    let target_url = url::Url::from_file_path(&target_path).unwrap();
    retriever.insert(target.clone(), "file URI\n");

    let output = resolve(retriever.clone(), &root, &format!("$ref: '{target_url}'\n")).unwrap();

    assert_eq!(scalar(&output.root), "file URI");
    assert_eq!(retriever.calls(), vec![target]);
}

#[test]
fn nested_http_references_resolve_relative_to_the_remote_document() {
    let retriever = CountingRetriever::default();
    let root = id("http://example.com/config/app.yaml");
    let shared = id("http://example.com/shared.yaml");
    let child = id("http://example.com/child.yaml");
    retriever.insert(shared.clone(), "node: {$ref: './child.yaml'}\n");
    retriever.insert(child.clone(), "remote result\n");

    let output = resolve(retriever.clone(), &root, "$ref: '../shared.yaml#/node'\n").unwrap();

    assert_eq!(scalar(&output.root), "remote result");
    assert_eq!(retriever.calls(), vec![shared, child]);
}

#[test]
fn cross_resource_cycles_return_the_complete_target_chain() {
    let retriever = CountingRetriever::default();
    let root = id("memory://root/document.yaml");
    let left = id("memory://cycle/left.yaml");
    let right = id("memory://cycle/right.yaml");
    retriever.insert(left, "$ref: './right.yaml#/node'\n");
    retriever.insert(right, "node: {$ref: './left.yaml'}\n");

    let error = resolve(retriever, &root, "$ref: 'memory://cycle/left.yaml'\n").unwrap_err();

    let ResolveError::Resolution {
        category,
        reference_chain,
        ..
    } = error
    else {
        panic!("expected cycle error");
    };
    assert_eq!(category, ResolutionErrorCategory::Cycle);
    assert_eq!(reference_chain.len(), 3);
    assert_eq!(reference_chain[0], reference_chain[2]);
    assert!(reference_chain[0].contains("left.yaml"));
    assert!(reference_chain[1].contains("right.yaml#/node"));
}

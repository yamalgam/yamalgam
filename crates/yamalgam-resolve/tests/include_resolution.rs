//! Scalar and structured `!include` resolution behavior.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use yamalgam_core::{LoaderConfig, ResolutionMode, ResourceLimit};
use yamalgam_parser::{Parser, ResolutionErrorCategory, ResolveError};
use yamalgam_resolve::{
    DiscoveryRequest, DiscoverySelector, OwnedDocument, OwnedKind, OwnedNode, ResolutionSession,
    ResourceId, ResourceProvenance, ResourceRequest, ResourceRetriever, RetrieveError,
    RetrievedResource, RetrieverRouter,
};

#[derive(Clone, Default)]
struct IncludeRetriever {
    state: Arc<Mutex<RetrieverState>>,
}

#[derive(Default)]
struct RetrieverState {
    resources: HashMap<ResourceId, RetrievedResource>,
    discoveries: Vec<ResourceId>,
    retrieve_calls: Vec<ResourceId>,
    discovery_calls: Vec<DiscoveryRequest>,
}

impl IncludeRetriever {
    fn insert(&self, id: ResourceId, bytes: impl Into<Vec<u8>>) {
        self.state.lock().unwrap().resources.insert(
            id.clone(),
            RetrievedResource {
                id,
                bytes: bytes.into(),
                provenance: ResourceProvenance::default(),
            },
        );
    }

    fn set_discoveries(&self, discoveries: Vec<ResourceId>) {
        self.state.lock().unwrap().discoveries = discoveries;
    }

    fn retrieve_calls(&self) -> Vec<ResourceId> {
        self.state.lock().unwrap().retrieve_calls.clone()
    }

    fn discovery_calls(&self) -> Vec<DiscoveryRequest> {
        self.state.lock().unwrap().discovery_calls.clone()
    }
}

impl ResourceRetriever for IncludeRetriever {
    fn retrieve(&self, request: &ResourceRequest) -> Result<RetrievedResource, RetrieveError> {
        let mut state = self.state.lock().unwrap();
        state.retrieve_calls.push(request.id.clone());
        state
            .resources
            .get(&request.id)
            .cloned()
            .ok_or_else(|| RetrieveError::NotFound(request.id.clone()))
    }

    fn discover(&self, request: &DiscoveryRequest) -> Result<Vec<ResourceId>, RetrieveError> {
        let mut state = self.state.lock().unwrap();
        state.discovery_calls.push(request.clone());
        Ok(state.discoveries.clone())
    }
}

fn file(path: &str) -> ResourceId {
    ResourceId::from_file(path)
}

fn document(origin: &ResourceId, input: &str) -> OwnedDocument {
    let events = Parser::new(input).collect::<Result<Vec<_>, _>>().unwrap();
    OwnedDocument::from_events(origin.clone(), events).unwrap()
}

fn config() -> LoaderConfig {
    let mut config = LoaderConfig::unchecked();
    config.resolution.include.mode = ResolutionMode::Automatic;
    config.resolution.refs.mode = ResolutionMode::Automatic;
    config.resolution.refs.allow_schemes = vec!["memory".into(), "file".into(), "http".into()];
    config.resolution.refs.allow_hosts = vec!["example.com".into()];
    config
}

fn resolve(
    retriever: IncludeRetriever,
    input: &str,
) -> Result<(OwnedDocument, ResolutionSession<IncludeRetriever>), ResolveError> {
    let origin = file("/workspace/root.yaml");
    let mut session = ResolutionSession::new(retriever, config());
    let output = session.resolve_root(document(&origin, input))?;
    Ok((output, session))
}

fn scalar(node: &OwnedNode) -> &str {
    let OwnedKind::Scalar(scalar) = &node.kind else {
        panic!("expected scalar, got {:?}", node.kind);
    };
    &scalar.value
}

fn mapping_entries(node: &OwnedNode) -> &[(OwnedNode, OwnedNode)] {
    let OwnedKind::Mapping { entries, .. } = &node.kind else {
        panic!("expected mapping, got {:?}", node.kind);
    };
    entries
}

fn mapping_value<'a>(node: &'a OwnedNode, key: &str) -> &'a OwnedNode {
    mapping_entries(node)
        .iter()
        .find_map(|(candidate, value)| (scalar(candidate) == key).then_some(value))
        .unwrap_or_else(|| panic!("missing key {key}"))
}

fn category(error: &ResolveError) -> ResolutionErrorCategory {
    match error {
        ResolveError::Resolution { category, .. } => *category,
        other => panic!("expected structured resolution error, got {other:?}"),
    }
}

// r[verify resolution.include]
#[test]
fn scalar_shorthand_substitutes_one_yaml_node_and_recurses() {
    let retriever = IncludeRetriever::default();
    let child = file("/workspace/child.yaml");
    let grandchild = file("/workspace/grandchild.yaml");
    retriever.insert(
        child.clone(),
        "value: !include grandchild.yaml\nreference: {$ref: './grandchild.yaml'}\n",
    );
    retriever.insert(grandchild.clone(), "nested\n");

    let (output, _) = resolve(retriever.clone(), "root: !include child.yaml\n").unwrap();

    let included = mapping_value(&output.root, "root");
    assert_eq!(scalar(mapping_value(included, "value")), "nested");
    assert_eq!(scalar(mapping_value(included, "reference")), "nested");
    assert_eq!(retriever.retrieve_calls(), vec![child, grandchild]);
}

// r[verify resolution.include.structured]
#[test]
fn structured_shape_rejects_missing_conflicting_unknown_duplicate_and_wrong_options() {
    let cases = [
        "!include\n  mode: node\n",
        "!include\n  path: a.yaml\n  glob: '*.yaml'\n",
        "!include\n  path: a.yaml\n  mystery: value\n",
        "!include\n  path: a.yaml\n  path: b.yaml\n",
        "!include\n  path: [a.yaml]\n",
        "!include\n  path: a.yaml\n  format: binary\n",
        "!include\n  path: a.yaml\n  mode: flatten\n",
        "!include\n  path: a.yaml\n  recursive: true\n",
        "!include\n  glob: '*.yaml'\n  match: '*.yaml'\n",
        "!include\n  path: a.yaml\n  mode: list\n  key: stem\n",
        "!include\n  path: a.yaml\n  mode: list\n  on_conflict: first\n",
        "!include\n  path: a.yaml\n  format: text\n  mode: merge\n",
    ];

    for input in cases {
        let error = resolve(IncludeRetriever::default(), input).err().unwrap();
        assert_eq!(
            category(&error),
            ResolutionErrorCategory::MalformedDirective,
            "input {input:?}"
        );
    }
}

#[test]
fn include_selectors_must_be_relative_paths() {
    let cases = [
        "!include /workspace/a.yaml\n",
        "!include\n  path: /workspace/a.yaml\n",
        "!include\n  glob: '/workspace/*.yaml'\n  mode: list\n",
        "!include\n  dir: /workspace\n  mode: list\n",
    ];

    for input in cases {
        let retriever = IncludeRetriever::default();
        let error = resolve(retriever.clone(), input).err().unwrap();
        assert_eq!(
            category(&error),
            ResolutionErrorCategory::MalformedDirective,
            "input {input:?}"
        );
        assert!(retriever.retrieve_calls().is_empty(), "input {input:?}");
        assert!(retriever.discovery_calls().is_empty(), "input {input:?}");
    }
}

#[test]
fn node_requires_one_match_while_list_normalizes_discovery_order() {
    let retriever = IncludeRetriever::default();
    let a = file("/workspace/parts/a.yaml");
    let b = file("/workspace/parts/b.yaml");
    retriever.insert(a.clone(), "a\n");
    retriever.insert(b.clone(), "b\n");

    retriever.set_discoveries(Vec::new());
    let error = resolve(
        retriever.clone(),
        "!include\n  glob: 'parts/*.yaml'\n  mode: node\n",
    )
    .err()
    .unwrap();
    assert_eq!(category(&error), ResolutionErrorCategory::AggregateType);

    let error = resolve(
        retriever.clone(),
        "!include\n  glob: 'parts/*.yaml'\n  mode: list\n",
    )
    .err()
    .unwrap();
    assert_eq!(category(&error), ResolutionErrorCategory::AggregateType);

    retriever.set_discoveries(vec![a.clone()]);
    let (single, _) = resolve(
        retriever.clone(),
        "!include\n  glob: 'parts/*.yaml'\n  mode: node\n",
    )
    .unwrap();
    assert_eq!(scalar(&single.root), "a");

    retriever.set_discoveries(vec![b.clone(), a, b]);
    let error = resolve(
        retriever.clone(),
        "!include\n  glob: 'parts/*.yaml'\n  mode: node\n",
    )
    .err()
    .unwrap();
    assert_eq!(category(&error), ResolutionErrorCategory::AggregateType);

    let (output, session) = resolve(
        retriever,
        "!include\n  glob: 'parts/*.yaml'\n  mode: list\n",
    )
    .unwrap();
    let OwnedKind::Sequence { items, .. } = &output.root.kind else {
        panic!("expected synthesized list");
    };
    assert_eq!(items.iter().map(scalar).collect::<Vec<_>>(), ["a", "b"]);
    assert_eq!(session.report().edges().len(), 3);
    let collection = &session.report().edges()[0];
    assert_eq!(
        collection.matches,
        [
            file("/workspace/parts/a.yaml"),
            file("/workspace/parts/b.yaml")
        ]
    );
    assert_eq!(
        collection.outcome,
        yamalgam_resolve::ResolutionOutcome::Resolved
    );
    assert!(
        session
            .report()
            .edges()
            .iter()
            .skip(1)
            .all(|edge| edge.materialized_bytes > 0)
    );
    assert_eq!(
        session
            .report()
            .edges()
            .iter()
            .skip(1)
            .map(|edge| edge.fetched_bytes)
            .collect::<Vec<_>>(),
        [2, 2]
    );
}

#[test]
fn collection_report_edges_keep_matches_and_terminal_failures() {
    let retriever = IncludeRetriever::default();
    let a = file("/workspace/parts/a.yaml");
    let b = file("/workspace/parts/b.yaml");
    retriever.insert(a.clone(), "a\n");
    retriever.insert(b.clone(), "b\n");
    retriever.set_discoveries(vec![b.clone(), a.clone(), b]);
    let origin = file("/workspace/root.yaml");
    let mut session = ResolutionSession::new(retriever, config());

    let error = session
        .resolve_root(document(
            &origin,
            "!include\n  glob: 'parts/*.yaml'\n  mode: node\n",
        ))
        .unwrap_err();

    assert_eq!(category(&error), ResolutionErrorCategory::AggregateType);
    let edges = session.report().edges();
    assert_eq!(edges.len(), 3);
    assert_eq!(edges[0].requested, "parts/*.yaml");
    assert_eq!(edges[0].matches, [a, file("/workspace/parts/b.yaml")]);
    assert_eq!(
        edges[0].outcome,
        yamalgam_resolve::ResolutionOutcome::Failed(ResolutionErrorCategory::AggregateType)
    );
    assert!(
        edges[1..]
            .iter()
            .all(|edge| { edge.outcome == yamalgam_resolve::ResolutionOutcome::Resolved })
    );
}

#[test]
fn named_uses_stem_or_extensionless_relative_keys_and_rejects_duplicates() {
    let retriever = IncludeRetriever::default();
    let a = file("/workspace/parts/a.yaml");
    let nested = file("/workspace/parts/nested/b.yml");
    retriever.insert(a.clone(), "one\n");
    retriever.insert(nested.clone(), "two\n");
    retriever.set_discoveries(vec![a.clone(), nested.clone()]);

    let (stemmed, _) = resolve(
        retriever.clone(),
        "!include\n  dir: parts\n  recursive: true\n  mode: named\n",
    )
    .unwrap();
    assert_eq!(scalar(mapping_value(&stemmed.root, "a")), "one");
    assert_eq!(scalar(mapping_value(&stemmed.root, "b")), "two");

    let (relative, _) = resolve(
        retriever.clone(),
        "!include\n  dir: parts\n  recursive: true\n  mode: named\n  key: relative\n",
    )
    .unwrap();
    assert_eq!(scalar(mapping_value(&relative.root, "a")), "one");
    assert_eq!(scalar(mapping_value(&relative.root, "nested/b")), "two");

    retriever.set_discoveries(vec![nested]);
    let (glob_relative, _) = resolve(
        retriever.clone(),
        "!include\n  glob: 'parts/**/*.yml'\n  mode: named\n  key: relative\n",
    )
    .unwrap();
    assert_eq!(
        scalar(mapping_value(&glob_relative.root, "nested/b")),
        "two"
    );

    let (path_named, _) = resolve(
        retriever.clone(),
        "!include\n  path: parts/a.yaml\n  mode: named\n",
    )
    .unwrap();
    assert_eq!(scalar(mapping_value(&path_named.root, "a")), "one");

    let duplicate = file("/workspace/other/a.yml");
    retriever.insert(duplicate.clone(), "duplicate\n");
    retriever.set_discoveries(vec![a, duplicate]);
    let error = resolve(retriever, "!include\n  glob: '**/*.y*ml'\n  mode: named\n")
        .err()
        .unwrap();
    assert_eq!(category(&error), ResolutionErrorCategory::Conflict);
}

// r[verify resolution.include.aggregation]
#[test]
fn merge_combines_homogeneous_mappings_or_sequences_with_explicit_conflict_policy() {
    let retriever = IncludeRetriever::default();
    let a = file("/workspace/a.yaml");
    let b = file("/workspace/b.yaml");
    retriever.insert(a.clone(), "shared: first\na: one\n");
    retriever.insert(b.clone(), "shared: last\nb: two\n");
    retriever.set_discoveries(vec![a.clone(), b.clone()]);

    let base = "!include\n  glob: '*.yaml'\n  mode: merge\n";
    let error = resolve(retriever.clone(), base).err().unwrap();
    assert_eq!(category(&error), ResolutionErrorCategory::Conflict);

    let (first, _) = resolve(retriever.clone(), &format!("{base}  on_conflict: first\n")).unwrap();
    assert_eq!(scalar(mapping_value(&first.root, "shared")), "first");

    let (last, _) = resolve(retriever.clone(), &format!("{base}  on_conflict: last\n")).unwrap();
    assert_eq!(scalar(mapping_value(&last.root, "shared")), "last");

    retriever.insert(a.clone(), "1: numeric\n");
    retriever.insert(b.clone(), "'1': string\n");
    let (typed_keys, _) = resolve(retriever.clone(), base).unwrap();
    assert_eq!(mapping_entries(&typed_keys.root).len(), 2);

    retriever.insert(a.clone(), "[a, b]\n");
    retriever.insert(b, "[c]\n");
    let (sequence, _) = resolve(retriever.clone(), base).unwrap();
    let OwnedKind::Sequence { items, .. } = &sequence.root.kind else {
        panic!("expected merged sequence");
    };
    assert_eq!(
        items.iter().map(scalar).collect::<Vec<_>>(),
        ["a", "b", "c"]
    );

    let error = resolve(retriever.clone(), &format!("{base}  on_conflict: first\n"))
        .err()
        .unwrap();
    assert_eq!(
        category(&error),
        ResolutionErrorCategory::MalformedDirective
    );

    retriever.insert(a, "scalar\n");
    let error = resolve(retriever, base).err().unwrap();
    assert_eq!(category(&error), ResolutionErrorCategory::AggregateType);
}

#[test]
fn splice_is_sequence_item_only_and_requires_sequence_roots() {
    let retriever = IncludeRetriever::default();
    let a = file("/workspace/a.yaml");
    let b = file("/workspace/b.yaml");
    retriever.insert(a.clone(), "[a, b]\n");
    retriever.insert(b.clone(), "[c]\n");
    retriever.set_discoveries(vec![a, b.clone()]);
    let directive = "!include\n    glob: '*.yaml'\n    mode: splice\n";

    let (output, _) = resolve(
        retriever.clone(),
        &format!("- before\n- {directive}- after\n"),
    )
    .unwrap();
    let OwnedKind::Sequence { items, .. } = &output.root.kind else {
        panic!("expected containing sequence");
    };
    assert_eq!(
        items.iter().map(scalar).collect::<Vec<_>>(),
        ["before", "a", "b", "c", "after"]
    );

    let error = resolve(retriever.clone(), directive).err().unwrap();
    assert_eq!(
        category(&error),
        ResolutionErrorCategory::IncompatibleContext
    );

    retriever.insert(b, "not a sequence\n");
    let error = resolve(retriever, &format!("- {directive}")).err().unwrap();
    assert_eq!(category(&error), ResolutionErrorCategory::AggregateType);
}

#[test]
fn merge_key_includes_mapping_and_explicit_parent_entries_win_in_any_order() {
    let retriever = IncludeRetriever::default();
    let defaults = file("/workspace/defaults.yaml");
    retriever.insert(defaults, "host: included\nport: 80\nextra: retained\n");

    for input in [
        "service:\n  host: explicit\n  <<: !include defaults.yaml\n  port: 443\n",
        "service:\n  <<: !include defaults.yaml\n  host: explicit\n  port: 443\n",
    ] {
        let (output, _) = resolve(retriever.clone(), input).unwrap();
        let service = mapping_value(&output.root, "service");
        assert_eq!(scalar(mapping_value(service, "host")), "explicit");
        assert_eq!(scalar(mapping_value(service, "port")), "443");
        assert_eq!(scalar(mapping_value(service, "extra")), "retained");
        assert!(
            !mapping_entries(service)
                .iter()
                .any(|(key, _)| scalar(key) == "<<")
        );
    }
}

// r[verify resolution.include.text]
#[test]
fn text_node_list_and_named_preserve_exact_utf8_and_reject_invalid_bytes() {
    let retriever = IncludeRetriever::default();
    let a = file("/workspace/text/a.txt");
    let b = file("/workspace/text/b.txt");
    retriever.insert(a.clone(), "hello\nworld\n");
    retriever.insert(b.clone(), "second");

    let (node, _) = resolve(
        retriever.clone(),
        "!include\n  path: text/a.txt\n  format: text\n",
    )
    .unwrap();
    assert_eq!(scalar(&node.root), "hello\nworld\n");

    retriever.set_discoveries(vec![a.clone(), b]);
    let (list, _) = resolve(
        retriever.clone(),
        "!include\n  glob: 'text/*.txt'\n  format: text\n  mode: list\n",
    )
    .unwrap();
    let OwnedKind::Sequence { items, .. } = &list.root.kind else {
        panic!("expected text list");
    };
    assert_eq!(
        items.iter().map(scalar).collect::<Vec<_>>(),
        ["hello\nworld\n", "second"]
    );

    let (named, _) = resolve(
        retriever.clone(),
        "!include\n  glob: 'text/*.txt'\n  format: text\n  mode: named\n",
    )
    .unwrap();
    assert_eq!(scalar(mapping_value(&named.root, "a")), "hello\nworld\n");

    retriever.insert(a, vec![0xff]);
    let error = resolve(retriever, "!include\n  path: text/a.txt\n  format: text\n")
        .err()
        .unwrap();
    assert_eq!(category(&error), ResolutionErrorCategory::InvalidUtf8);
}

#[test]
fn directory_options_become_one_typed_discovery_request() {
    let retriever = IncludeRetriever::default();
    let a = file("/workspace/parts/a.yaml");
    retriever.insert(a.clone(), "a\n");
    retriever.set_discoveries(vec![a]);

    resolve(
        retriever.clone(),
        "!include\n  dir: parts\n  recursive: true\n  match: ['*.yaml', '*.yml']\n  mode: list\n",
    )
    .unwrap();

    let calls = retriever.discovery_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].base, file("/workspace/root.yaml"));
    assert_eq!(
        calls[0].selector,
        DiscoverySelector::Directory {
            path: "parts".into(),
            recursive: true,
            matches: vec!["*.yaml".into(), "*.yml".into()],
        }
    );
}

#[test]
fn absolute_http_includes_are_malformed_even_when_http_refs_are_enabled() {
    let retriever = IncludeRetriever::default();
    let remote = ResourceId::new("http://example.com/data.yaml").unwrap();
    retriever.insert(remote, "remote\n");

    let error = resolve(retriever.clone(), "!include http://example.com/data.yaml\n")
        .err()
        .unwrap();

    assert_eq!(
        category(&error),
        ResolutionErrorCategory::MalformedDirective
    );
    assert!(retriever.retrieve_calls().is_empty());
}

#[test]
fn absolute_uri_include_selectors_are_malformed_before_retrieval() {
    let cases = [
        "!include file:///workspace/data.yaml\n",
        "!include\n  glob: 'memory://fixtures/*.yaml'\n  mode: list\n",
        "!include\n  dir: 'custom://fixtures'\n  mode: list\n",
    ];

    for input in cases {
        let retriever = IncludeRetriever::default();
        let error = resolve(retriever.clone(), input).err().unwrap();

        assert_eq!(
            category(&error),
            ResolutionErrorCategory::MalformedDirective
        );
        assert!(retriever.retrieve_calls().is_empty());
        assert!(retriever.discovery_calls().is_empty());
    }
}

// r[verify resolution.include.cycle]
#[test]
fn include_and_ref_recursion_share_one_cycle_stack() {
    let retriever = IncludeRetriever::default();
    let a = file("/workspace/a.yaml");
    let b = file("/workspace/b.yaml");
    retriever.insert(a.clone(), "!include b.yaml\n");
    retriever.insert(b.clone(), "$ref: './a.yaml'\n");

    let error = resolve(retriever.clone(), "!include a.yaml\n")
        .err()
        .unwrap();

    let ResolveError::Resolution {
        category,
        reference_chain,
        ..
    } = error
    else {
        panic!("expected cross-capability cycle");
    };
    assert_eq!(category, ResolutionErrorCategory::Cycle);
    assert_eq!(reference_chain.len(), 3);
    assert_eq!(reference_chain[0], reference_chain[2]);
    assert_eq!(retriever.retrieve_calls(), vec![a, b]);
}

#[test]
fn invalid_yaml_and_multiple_documents_keep_distinct_include_failures() {
    let cases = [
        ("[unterminated", ResolutionErrorCategory::InvalidYaml),
        (
            "---\none\n---\ntwo\n",
            ResolutionErrorCategory::DocumentCount,
        ),
    ];
    for (bytes, expected) in cases {
        let retriever = IncludeRetriever::default();
        retriever.insert(file("/workspace/bad.yaml"), bytes);

        let error = resolve(retriever, "!include bad.yaml\n").err().unwrap();

        assert_eq!(category(&error), expected);
    }
}

#[test]
fn yaml_and_text_views_share_one_raw_fetch() {
    let retriever = IncludeRetriever::default();
    let dual = file("/workspace/dual.yaml");
    retriever.insert(dual.clone(), "value\n");

    let (output, session) = resolve(
        retriever.clone(),
        "- !include\n  path: dual.yaml\n  format: text\n- !include dual.yaml\n",
    )
    .unwrap();

    let OwnedKind::Sequence { items, .. } = &output.root.kind else {
        panic!("expected sequence");
    };
    assert_eq!(
        items.iter().map(scalar).collect::<Vec<_>>(),
        ["value\n", "value"]
    );
    assert_eq!(retriever.retrieve_calls(), vec![dual]);
    assert!(!session.report().edges()[0].cache_hit);
    assert!(session.report().edges()[1].cache_hit);
    assert_eq!(session.report().edges()[1].fetched_bytes, 0);
}

#[test]
fn synthesized_aggregate_storage_cannot_evade_the_materialized_budget() {
    let retriever = IncludeRetriever::default();
    let a = file("/workspace/a.yaml");
    let b = file("/workspace/b.yaml");
    retriever.insert(a.clone(), "a\n");
    retriever.insert(b.clone(), "b\n");
    retriever.set_discoveries(vec![a.clone(), b]);
    let child_weight = document(&a, "a\n").root.materialized_weight().unwrap();
    let child_total = child_weight.checked_mul(2).unwrap();
    let mut limited = config();
    limited.resolution.limits.max_materialized_bytes = Some(child_total);
    let origin = file("/workspace/root.yaml");
    let mut session = ResolutionSession::new(retriever, limited);

    let error = session
        .resolve_root(document(
            &origin,
            "!include\n  glob: '*.yaml'\n  mode: list\n",
        ))
        .unwrap_err();

    let ResolveError::Resolution {
        category,
        source: Some(source),
        ..
    } = error
    else {
        panic!("expected aggregate budget failure");
    };
    let exceeded = source
        .downcast_ref::<yamalgam_core::LimitExceeded>()
        .unwrap();
    assert_eq!(category, ResolutionErrorCategory::Limit);
    assert_eq!(exceeded.limit, ResourceLimit::MaterializedBytes);
    assert!(exceeded.observed > child_total);
    assert_eq!(exceeded.maximum, child_total);
    assert_eq!(
        session.report().edges()[0].outcome,
        yamalgam_resolve::ResolutionOutcome::Failed(ResolutionErrorCategory::Limit)
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_named_keys_fail_before_resource_io() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::path::PathBuf;

    let retriever = IncludeRetriever::default();
    let invalid = ResourceId::from_file(PathBuf::from(OsString::from_vec(
        b"/workspace/parts/name-\xff.yaml".to_vec(),
    )));
    retriever.set_discoveries(vec![invalid]);

    let error = resolve(retriever.clone(), "!include\n  dir: parts\n  mode: named\n")
        .err()
        .unwrap();

    assert_eq!(category(&error), ResolutionErrorCategory::AggregateType);
    assert!(retriever.retrieve_calls().is_empty());
}

#[cfg(unix)]
#[test]
fn non_utf8_paths_still_work_when_the_mode_does_not_derive_names() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::path::PathBuf;

    let retriever = IncludeRetriever::default();
    let invalid = ResourceId::from_file(PathBuf::from(OsString::from_vec(
        b"/workspace/parts/name-\xff.yaml".to_vec(),
    )));
    retriever.insert(invalid.clone(), "retained\n");
    retriever.set_discoveries(vec![invalid.clone()]);

    let (output, _) = resolve(retriever.clone(), "!include\n  dir: parts\n  mode: list\n").unwrap();

    let OwnedKind::Sequence { items, .. } = &output.root.kind else {
        panic!("expected list");
    };
    assert_eq!(scalar(&items[0]), "retained");
    assert_eq!(retriever.retrieve_calls(), vec![invalid]);
}

#[test]
fn file_directory_includes_are_recursive_and_deterministic() {
    use std::fs;

    use yamalgam_core::FilePolicy;
    use yamalgam_resolve::FileRetriever;

    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("parts/nested")).unwrap();
    fs::write(root.path().join("parts/nested/b.yaml"), "b\n").unwrap();
    fs::write(root.path().join("parts/a.yaml"), "a\n").unwrap();
    let policy = FilePolicy {
        root: Some(root.path().to_path_buf()),
        allow: Vec::new(),
        deny: Vec::new(),
        follow_symlinks: false,
    };
    let mut router = RetrieverRouter::default();
    router
        .register("file", FileRetriever::open(&policy).unwrap())
        .unwrap();
    let mut configured = config();
    configured.resolution.files = policy;
    let origin = ResourceId::from_file(root.path().join("root.yaml"));
    let mut session = ResolutionSession::new(router, configured);

    let output = session
        .resolve_root(document(
            &origin,
            "!include\n  dir: parts\n  recursive: true\n  mode: list\n",
        ))
        .unwrap();

    let OwnedKind::Sequence { items, .. } = &output.root.kind else {
        panic!("expected list");
    };
    assert_eq!(items.iter().map(scalar).collect::<Vec<_>>(), ["a", "b"]);
}

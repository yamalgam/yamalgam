//! Resolution-session cache, recursion, budget, and report invariants.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use yamalgam_core::{LoaderConfig, ResolutionCapability, ResolutionMode, ResourceLimit, Span};
use yamalgam_parser::{Parser, ResolutionErrorCategory, ResolveError};
use yamalgam_resolve::{
    DiscoveryRequest, OwnedDocument, OwnedKind, ResolutionOutcome, ResolutionSession, ResourceId,
    ResourceProvenance, ResourceRequest, ResourceRetriever, RetrieveError, RetrievedResource,
};

#[derive(Clone, Default)]
struct SpyRetriever {
    state: Arc<Mutex<SpyState>>,
}

#[derive(Default)]
struct SpyState {
    resources: HashMap<ResourceId, RetrievedResource>,
    retrieve_calls: Vec<ResourceId>,
    discover_calls: usize,
}

impl SpyRetriever {
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

    fn retrieve_calls(&self) -> Vec<ResourceId> {
        self.state.lock().unwrap().retrieve_calls.clone()
    }

    fn discover_calls(&self) -> usize {
        self.state.lock().unwrap().discover_calls
    }
}

impl ResourceRetriever for SpyRetriever {
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
        self.state.lock().unwrap().discover_calls += 1;
        Err(RetrieveError::DiscoveryUnsupported(request.base.clone()))
    }
}

fn id(value: &str) -> ResourceId {
    ResourceId::new(value).unwrap()
}

fn document(origin: &ResourceId, yaml: &str) -> OwnedDocument {
    let events = Parser::new(yaml).collect::<Result<Vec<_>, _>>().unwrap();
    OwnedDocument::from_events(origin.clone(), events).unwrap()
}

fn config(refs: ResolutionMode, include: ResolutionMode) -> LoaderConfig {
    let mut config = LoaderConfig::unchecked();
    config.resolution.refs.mode = refs;
    config.resolution.refs.allow_schemes = vec!["memory".into(), "file".into()];
    config.resolution.include.mode = include;
    config
}

fn resolution_error(error: &ResolveError) -> (ResolutionErrorCategory, Span) {
    match error {
        ResolveError::Resolution { category, span, .. } => (*category, *span),
        other => panic!("expected structured resolution error, got {other:?}"),
    }
}

#[test]
fn disabled_policy_is_a_zero_io_passthrough() {
    let retriever = SpyRetriever::default();
    let root_id = id("memory://root/document.yaml");
    let input = document(
        &root_id,
        "ref: {$ref: 'memory://shared/document.yaml'}\ninclude: !include child.yaml\n",
    );
    let mut session = ResolutionSession::new(retriever.clone(), LoaderConfig::unchecked());

    let output = session.resolve_root(input.clone()).unwrap();

    assert_eq!(output, input);
    assert!(retriever.retrieve_calls().is_empty());
    assert_eq!(retriever.discover_calls(), 0);
    assert!(session.report().edges().is_empty());
}

#[test]
fn one_raw_fetch_and_parse_serves_multiple_materializations() {
    let retriever = SpyRetriever::default();
    let root_id = id("memory://root/document.yaml");
    let child_id = id("memory://shared/document.yaml");
    let child_bytes = b"shared value\n".to_vec();
    retriever.insert(child_id.clone(), child_bytes.clone());
    let input = document(
        &root_id,
        "- {$ref: 'memory://shared/document.yaml'}\n- {$ref: 'memory://shared/document.yaml'}\n",
    );
    let mut session = ResolutionSession::new(
        retriever.clone(),
        config(ResolutionMode::Automatic, ResolutionMode::Disabled),
    );

    let output = session.resolve_root(input).unwrap();

    let OwnedKind::Sequence { items, .. } = &output.root.kind else {
        panic!("expected sequence root");
    };
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|item| {
        matches!(&item.kind, OwnedKind::Scalar(scalar) if scalar.value == "shared value")
    }));
    assert_eq!(retriever.retrieve_calls(), vec![child_id.clone()]);

    let edges = session.report().edges();
    assert_eq!(edges.len(), 2);
    assert!(!edges[0].cache_hit);
    assert!(edges[1].cache_hit);
    assert_eq!(edges[0].fetched_bytes, child_bytes.len());
    assert_eq!(edges[1].fetched_bytes, 0);
    assert_eq!(edges[0].materialized_bytes, edges[1].materialized_bytes);
    assert!(edges[0].materialized_bytes > 0);
    assert_eq!(edges[0].outcome, ResolutionOutcome::Resolved);
    assert_eq!(edges[1].outcome, ResolutionOutcome::Resolved);

    let child_record = session
        .report()
        .resources()
        .iter()
        .find(|record| record.id == child_id)
        .unwrap();
    assert_eq!(child_record.fetched_bytes, child_bytes.len());
    assert_eq!(child_record.cache_hits, 1);
}

#[test]
fn root_is_primed_for_same_document_references() {
    let retriever = SpyRetriever::default();
    let root_id = id("memory://root/document.yaml");
    let input = document(&root_id, "value: retained\ncopy: {$ref: '#/value'}\n");
    let mut session = ResolutionSession::new(
        retriever.clone(),
        config(ResolutionMode::Automatic, ResolutionMode::Disabled),
    );

    let output = session.resolve_root(input).unwrap();

    let OwnedKind::Mapping { entries, .. } = &output.root.kind else {
        panic!("expected mapping root");
    };
    assert!(matches!(
        &entries[1].1.kind,
        OwnedKind::Scalar(scalar) if scalar.value == "retained"
    ));
    assert!(retriever.retrieve_calls().is_empty());
    assert!(session.report().edges()[0].cache_hit);
}

#[test]
fn same_document_refs_bypass_transport_policy_but_external_files_do_not() {
    let retriever = SpyRetriever::default();
    let anonymous = ResourceId::anonymous();
    let input = document(&anonymous, "value: retained\ncopy: {$ref: '#/value'}\n");
    let mut selected = config(ResolutionMode::Automatic, ResolutionMode::Disabled);
    selected.resolution.refs.allow_schemes = vec!["http".into()];
    let output = ResolutionSession::new(retriever.clone(), selected)
        .resolve_root(input)
        .unwrap();
    let OwnedKind::Mapping { entries, .. } = output.root.kind else {
        panic!("expected mapping root");
    };
    assert!(matches!(
        &entries[1].1.kind,
        OwnedKind::Scalar(scalar) if scalar.value == "retained"
    ));

    let root = ResourceId::from_file("/sandbox/root.yaml");
    let input = document(&root, "$ref: ./external.yaml\n");
    let mut selected = config(ResolutionMode::Automatic, ResolutionMode::Disabled);
    selected.resolution.refs.allow_schemes = vec!["http".into()];
    let error = ResolutionSession::new(retriever.clone(), selected)
        .resolve_root(input)
        .unwrap_err();
    assert_eq!(
        resolution_error(&error).0,
        ResolutionErrorCategory::PolicyDenied
    );
    assert!(retriever.retrieve_calls().is_empty());
}

#[test]
fn reports_and_errors_sanitize_credential_bearing_references() {
    let retriever = SpyRetriever::default();
    let root_id = id("memory://root/document.yaml");
    let input = document(
        &root_id,
        "$ref: 'https://alice:password@example.com/data.yaml?token=secret#/value'\n",
    );
    let mut selected = config(ResolutionMode::Automatic, ResolutionMode::Disabled);
    selected.resolution.refs.allow_schemes = vec!["https".into()];
    selected.resolution.refs.allow_hosts = vec!["example.com".into()];
    let mut session = ResolutionSession::new(retriever.clone(), selected);

    let error = session.resolve_root(input).unwrap_err();

    let ResolveError::Resolution { target, .. } = &error else {
        panic!("expected structured resolution error");
    };
    assert_eq!(target, "https://example.com/data.yaml#/value");
    assert_eq!(
        session.report().edges()[0].requested,
        "https://example.com/data.yaml#/value"
    );
    for rendered in [format!("{error:?}"), format!("{:?}", session.report())] {
        for secret in ["alice", "password", "token", "secret"] {
            assert!(
                !rendered.contains(secret),
                "leaked {secret:?} in {rendered}"
            );
        }
    }
    assert!(retriever.retrieve_calls().is_empty());
}

#[test]
fn cached_include_resources_do_not_bypass_ref_transport_policy() {
    let retriever = SpyRetriever::default();
    let root = ResourceId::from_file("/sandbox/root.yaml");
    let shared = ResourceId::from_file("/sandbox/shared.yaml");
    retriever.insert(shared.clone(), "retained\n");
    let input = document(
        &root,
        "included: !include ./shared.yaml\nreferenced: {$ref: ./shared.yaml}\n",
    );
    let mut selected = config(ResolutionMode::Automatic, ResolutionMode::Automatic);
    selected.resolution.refs.allow_schemes = vec!["http".into()];

    let error = ResolutionSession::new(retriever.clone(), selected)
        .resolve_root(input)
        .unwrap_err();

    assert_eq!(
        resolution_error(&error).0,
        ResolutionErrorCategory::PolicyDenied
    );
    assert_eq!(retriever.retrieve_calls(), vec![shared]);
}

#[test]
fn relative_includes_from_non_file_origins_are_denied_before_retrieval() {
    let retriever = SpyRetriever::default();
    let root = id("memory://root/document.yaml");
    let child = id("memory://root/child.yaml");
    retriever.insert(child, "retained\n");
    let input = document(&root, "!include child.yaml\n");
    let selected = config(ResolutionMode::Disabled, ResolutionMode::Automatic);

    let error = ResolutionSession::new(retriever.clone(), selected)
        .resolve_root(input)
        .unwrap_err();

    assert_eq!(
        resolution_error(&error).0,
        ResolutionErrorCategory::PolicyDenied
    );
    assert!(retriever.retrieve_calls().is_empty());
}

// r[verify resolution.ref.document]
#[test]
fn invalid_utf8_yaml_and_document_count_remain_distinct() {
    let cases = [
        (
            "memory://bad/utf8.yaml",
            vec![0xff],
            ResolutionErrorCategory::InvalidUtf8,
        ),
        (
            "memory://bad/yaml.yaml",
            b"[unterminated".to_vec(),
            ResolutionErrorCategory::InvalidYaml,
        ),
        (
            "memory://bad/empty.yaml",
            Vec::new(),
            ResolutionErrorCategory::DocumentCount,
        ),
        (
            "memory://bad/multiple.yaml",
            b"---\none\n---\ntwo\n".to_vec(),
            ResolutionErrorCategory::DocumentCount,
        ),
    ];

    for (target, bytes, expected) in cases {
        let retriever = SpyRetriever::default();
        retriever.insert(id(target), bytes);
        let root_id = id("memory://root/document.yaml");
        let input = document(&root_id, &format!("$ref: '{target}'\n"));
        let mut session = ResolutionSession::new(
            retriever,
            config(ResolutionMode::Automatic, ResolutionMode::Disabled),
        );

        let error = session.resolve_root(input).unwrap_err();

        assert_eq!(resolution_error(&error).0, expected, "target {target}");
        assert_eq!(
            session.report().edges()[0].outcome,
            ResolutionOutcome::Failed(expected)
        );
    }
}

#[test]
fn distinct_fragments_share_a_document_but_repeated_target_is_a_cycle() {
    let retriever = SpyRetriever::default();
    let root_id = id("memory://root/document.yaml");
    let child_id = id("memory://cycle/document.yaml");
    retriever.insert(child_id.clone(), "a: {$ref: '#/b'}\nb: {$ref: '#/a'}\n");
    let input = document(&root_id, "$ref: 'memory://cycle/document.yaml#/a'\n");
    let mut session = ResolutionSession::new(
        retriever.clone(),
        config(ResolutionMode::Automatic, ResolutionMode::Disabled),
    );

    let error = session.resolve_root(input).unwrap_err();

    let ResolveError::Resolution {
        category,
        reference_chain,
        ..
    } = error
    else {
        panic!("expected structured cycle error");
    };
    assert_eq!(category, ResolutionErrorCategory::Cycle);
    assert_eq!(reference_chain.len(), 3);
    assert_ne!(reference_chain[0], reference_chain[1]);
    assert_eq!(reference_chain[0], reference_chain[2]);
    assert_eq!(retriever.retrieve_calls(), vec![child_id]);
    assert_eq!(session.report().edges().len(), 3);
    assert!(session.report().edges()[1].cache_hit);
}

// r[verify resolution.include.budget]
#[test]
fn edge_depth_and_materialized_limits_accept_exact_boundary_and_reject_plus_one() {
    let root_id = id("memory://root/document.yaml");
    let child_id = id("memory://shared/document.yaml");
    let input = document(&root_id, "$ref: 'memory://shared/document.yaml'\n");
    let child = document(&child_id, "value\n");
    let weight = child.root.materialized_weight().unwrap();

    for (limit, maximum, expected_resource) in [
        ("edges", 1, ResourceLimit::ResolutionEdges),
        ("depth", 1, ResourceLimit::ResolutionDepth),
        ("bytes", weight, ResourceLimit::MaterializedBytes),
    ] {
        let retriever = SpyRetriever::default();
        retriever.insert(child_id.clone(), b"value\n".to_vec());
        let mut exact = config(ResolutionMode::Automatic, ResolutionMode::Disabled);
        exact.resolution.limits.max_edges = Some(1);
        exact.resolution.limits.max_depth = Some(1);
        exact.resolution.limits.max_materialized_bytes = Some(weight);
        ResolutionSession::new(retriever, exact)
            .resolve_root(input.clone())
            .unwrap_or_else(|error| panic!("exact {limit} boundary failed: {error}"));

        let retriever = SpyRetriever::default();
        retriever.insert(child_id.clone(), b"value\n".to_vec());
        let mut over = config(ResolutionMode::Automatic, ResolutionMode::Disabled);
        over.resolution.limits.max_edges = Some(if limit == "edges" { maximum - 1 } else { 1 });
        over.resolution.limits.max_depth = Some(if limit == "depth" { maximum - 1 } else { 1 });
        over.resolution.limits.max_materialized_bytes = Some(if limit == "bytes" {
            maximum - 1
        } else {
            weight
        });
        let mut session = ResolutionSession::new(retriever, over);

        let error = session.resolve_root(input.clone()).unwrap_err();
        let ResolveError::Resolution {
            category,
            span,
            source: Some(source),
            ..
        } = error
        else {
            panic!("expected structured limit error for {limit}");
        };
        let exceeded = source
            .downcast_ref::<yamalgam_core::LimitExceeded>()
            .unwrap();
        assert_eq!(category, ResolutionErrorCategory::Limit);
        assert_eq!(exceeded.limit, expected_resource);
        assert_eq!(exceeded.observed, maximum);
        assert_eq!(exceeded.maximum, maximum - 1);
        assert_ne!(span, Span::default());
        assert_eq!(
            session.report().edges()[0].outcome,
            ResolutionOutcome::Failed(ResolutionErrorCategory::Limit)
        );
    }
}

#[test]
fn explicit_resolution_recurses_for_that_capability_only() {
    let retriever = SpyRetriever::default();
    let root_id = id("memory://root/document.yaml");
    let ref_id = ResourceId::from_file("/shared/ref.yaml");
    let include_id = ResourceId::from_file("/shared/include.yaml");
    retriever.insert(
        ref_id.clone(),
        b"nested:\n  value: {$ref: 'file:///shared/ref.yaml#/leaf'}\n  other: !include include.yaml\nleaf: done\n".to_vec(),
    );
    retriever.insert(include_id.clone(), b"included\n".to_vec());
    let node = document(&root_id, "$ref: 'file:///shared/ref.yaml#/nested'\n").root;
    let mut session = ResolutionSession::new(
        retriever.clone(),
        config(ResolutionMode::Explicit, ResolutionMode::Explicit),
    );

    let output = session
        .resolve_explicit(ResolutionCapability::Ref, node)
        .unwrap();

    let OwnedKind::Mapping { entries, .. } = &output.kind else {
        panic!("expected selected mapping");
    };
    assert!(matches!(&entries[0].1.kind, OwnedKind::Scalar(value) if value.value == "done"));
    assert_eq!(retriever.retrieve_calls(), vec![ref_id]);

    let mut automatic = config(ResolutionMode::Explicit, ResolutionMode::Automatic);
    automatic.resolution.refs.mode = ResolutionMode::Explicit;
    let mut session = ResolutionSession::new(retriever.clone(), automatic);
    let node = document(&root_id, "$ref: 'file:///shared/ref.yaml#/nested'\n").root;
    let output = session
        .resolve_explicit(ResolutionCapability::Ref, node)
        .unwrap();
    let OwnedKind::Mapping { entries, .. } = &output.kind else {
        panic!("expected selected mapping");
    };
    assert!(matches!(&entries[1].1.kind, OwnedKind::Scalar(value) if value.value == "included"));
    assert!(retriever.retrieve_calls().contains(&include_id));
}

#[test]
fn reset_returns_finished_report_and_clears_session_state() {
    let retriever = SpyRetriever::default();
    let root_id = id("memory://root/document.yaml");
    let child_id = id("memory://shared/document.yaml");
    retriever.insert(child_id, b"value\n".to_vec());
    let input = document(&root_id, "$ref: 'memory://shared/document.yaml'\n");
    let mut session = ResolutionSession::new(
        retriever,
        config(ResolutionMode::Automatic, ResolutionMode::Disabled),
    );
    session.resolve_root(input).unwrap();

    let completed = session.reset_for_next_root();

    assert_eq!(completed.edges().len(), 1);
    assert!(session.report().edges().is_empty());
    assert!(session.report().resources().is_empty());
}

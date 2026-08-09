#![allow(missing_docs)]

use yamalgam_core::{ResolutionCapability, Span};
use yamalgam_parser::ResolutionErrorCategory;
use yamalgam_resolve::{
    ContentIntegrity, DiscoveryRequest, DiscoverySelector, ResolutionEdge, ResolutionOutcome,
    ResolutionReport, ResourceId, ResourceKind, ResourceProvenance, ResourceRecord,
    ResourceRequest, ResourceRetriever, ResourceValidator, RetrieveError, RetrievedResource,
};

struct FixtureRetriever;

impl ResourceRetriever for FixtureRetriever {
    fn retrieve(&self, request: &ResourceRequest) -> Result<RetrievedResource, RetrieveError> {
        Ok(RetrievedResource {
            id: request.id.clone(),
            bytes: b"answer: 42".to_vec(),
            provenance: ResourceProvenance {
                validators: vec![
                    ResourceValidator::Etag("secret-etag".into()),
                    ResourceValidator::LastModified("yesterday".into()),
                ],
                integrity: Some(ContentIntegrity {
                    algorithm: "sha256".into(),
                    encoded_value: "digest-value".into(),
                }),
            },
        })
    }
}

// r[verify resolution.integrity]
#[test]
fn custom_retrievers_return_bytes_validators_and_integrity() {
    let id = ResourceId::new("memory://fixtures/document.yaml").unwrap();
    let result = FixtureRetriever
        .retrieve(&ResourceRequest {
            id: id.clone(),
            max_bytes: Some(1024),
        })
        .unwrap();

    assert_eq!(result.id, id);
    assert_eq!(result.bytes, b"answer: 42");
    assert_eq!(result.provenance.validators.len(), 2);
    assert_eq!(
        result.provenance.integrity.as_ref().unwrap().algorithm,
        "sha256"
    );
    assert_eq!(
        result.provenance.integrity.as_ref().unwrap().encoded_value,
        "digest-value"
    );
    assert_eq!(
        format!("{result:?}"),
        "RetrievedResource { id: Uri(\"memory://fixtures/document.yaml\"), bytes_len: 10, provenance: ResourceProvenance { validators: 2, integrity: true } }"
    );
}

#[test]
fn discovery_is_unsupported_by_default() {
    let base = ResourceId::from_file("fixtures/root.yaml");
    let error = FixtureRetriever
        .discover(&DiscoveryRequest {
            base: base.clone(),
            selector: DiscoverySelector::Glob {
                pattern: "*.yaml".into(),
            },
            max_matches: Some(10),
            max_visits: None,
        })
        .unwrap_err();
    assert!(matches!(error, RetrieveError::DiscoveryUnsupported(id) if id == base));
}

// r[verify resolution.report]
#[test]
fn reports_preserve_insertion_order_and_complete_edge_state() {
    let parent = ResourceId::from_file("root.yaml");
    let first = ResourceId::from_file("first.yaml");
    let second = ResourceId::from_file("second.yaml");
    let fragment = parent.resolve("#/defaults").unwrap().fragment;
    let span = Span::default();
    let provenance = ResourceProvenance::default();
    let mut report = ResolutionReport::default();

    report.record_resource(ResourceRecord {
        id: first.clone(),
        kind: ResourceKind::File,
        fetched_bytes: 10,
        cache_hits: 0,
        provenance: provenance.clone(),
    });
    report.record_resource(ResourceRecord {
        id: second.clone(),
        kind: ResourceKind::File,
        fetched_bytes: 20,
        cache_hits: 1,
        provenance,
    });
    report.record_edge(ResolutionEdge {
        parent: parent.clone(),
        child: Some(first.clone()),
        capability: ResolutionCapability::Ref,
        source_span: span,
        requested: "first.yaml#/defaults".into(),
        fragment: fragment.clone(),
        matches: vec![first.clone(), second.clone()],
        cache_hit: false,
        fetched_bytes: 10,
        materialized_bytes: 7,
        outcome: ResolutionOutcome::Resolved,
    });
    report.record_edge(ResolutionEdge {
        parent,
        child: None,
        capability: ResolutionCapability::Include,
        source_span: span,
        requested: "missing.yaml".into(),
        fragment,
        matches: Vec::new(),
        cache_hit: true,
        fetched_bytes: 0,
        materialized_bytes: 0,
        outcome: ResolutionOutcome::Failed(ResolutionErrorCategory::Retrieval),
    });

    assert_eq!(report.resources()[0].id, first);
    assert_eq!(report.resources()[1].id, second);
    let success = &report.edges()[0];
    assert_eq!(success.requested, "first.yaml#/defaults");
    assert_eq!(success.matches.len(), 2);
    assert_eq!(success.fetched_bytes, 10);
    assert_eq!(success.materialized_bytes, 7);
    assert!(!success.cache_hit);
    assert_eq!(success.source_span, span);
    assert!(matches!(success.outcome, ResolutionOutcome::Resolved));
    let failure = &report.edges()[1];
    assert!(failure.cache_hit);
    assert!(matches!(
        failure.outcome,
        ResolutionOutcome::Failed(ResolutionErrorCategory::Retrieval)
    ));
}

// r[verify resolution.report.safe]
#[test]
fn report_rendering_redacts_sensitive_resource_data() {
    let id = ResourceId::new("https://alice:secret@example.com/data.yaml?token=value").unwrap();
    let mut report = ResolutionReport::default();
    report.record_resource(ResourceRecord {
        id,
        kind: ResourceKind::Https,
        fetched_bytes: 9,
        cache_hits: 0,
        provenance: ResourceProvenance {
            validators: vec![ResourceValidator::Opaque {
                name: "authorization".into(),
                value: "validator-secret".into(),
            }],
            integrity: None,
        },
    });

    for rendered in [report.to_string(), format!("{report:?}")] {
        assert!(rendered.contains("https://example.com/data.yaml"));
        for secret in [
            "alice",
            "secret",
            "token",
            "value",
            "validator-secret",
            "answer: 42",
        ] {
            assert!(
                !rendered.contains(secret),
                "leaked {secret:?} in {rendered:?}"
            );
        }
    }
}

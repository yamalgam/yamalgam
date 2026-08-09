//! Built-in event middleware integration.

use std::borrow::Cow;
use std::path::PathBuf;

use yamalgam_core::{LoaderConfig, ResolutionMode};
use yamalgam_parser::{Event, Parser, ResolveError, ResolvedEvents, Resolver};
use yamalgam_resolve::{
    BuiltinResolver, MemoryRetriever, ResolutionErrorCategory, ResourceId, ResourceProvenance,
    RetrievedResource,
};

fn id(value: &str) -> ResourceId {
    ResourceId::new(value).unwrap()
}

fn automatic_config() -> LoaderConfig {
    let mut config = LoaderConfig::unchecked();
    config.resolution.include.mode = ResolutionMode::Automatic;
    config.resolution.refs.mode = ResolutionMode::Automatic;
    config.resolution.refs.allow_schemes = vec!["memory".into()];
    config
}

fn retriever(resource: &ResourceId, input: &str) -> MemoryRetriever {
    let mut retriever = MemoryRetriever::default();
    retriever.insert(RetrievedResource {
        id: resource.clone(),
        bytes: input.as_bytes().to_vec(),
        provenance: ResourceProvenance::default(),
    });
    retriever
}

fn drive<'input>(
    resolver: &mut BuiltinResolver<MemoryRetriever>,
    input: &'input str,
    config: &LoaderConfig,
) -> Result<Vec<Event<'input>>, ResolveError> {
    let mut output = Vec::new();
    for event in Parser::with_config(input, config) {
        output.extend(resolver.on_event_with_config(event?, config)?);
    }
    Ok(output)
}

#[test]
fn builtin_resolution_buffers_and_emits_owned_replacements() {
    let root = id("memory://fixtures/root.yaml");
    let shared = id("memory://fixtures/shared.yaml");
    let config = automatic_config();
    let mut resolver =
        BuiltinResolver::new(retriever(&shared, "value: 42\n"), root, config.clone());

    let events = drive(
        &mut resolver,
        "answer:\n  $ref: ./shared.yaml#/value\n",
        &config,
    )
    .unwrap();
    let values = events
        .iter()
        .filter_map(|event| match event {
            Event::Scalar { value, .. } => Some(value.as_ref()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(values, ["answer", "42"]);
    let Event::Scalar { value, .. } = events
        .iter()
        .find(|event| matches!(event, Event::Scalar { value, .. } if value == "42"))
        .unwrap()
    else {
        unreachable!();
    };
    assert!(matches!(value, Cow::Owned(_)));
    assert_eq!(resolver.reports().len(), 1);
    assert_eq!(resolver.reports()[0].edges().len(), 1);
}

#[test]
fn builtin_resolution_resets_session_and_report_for_each_document() {
    let root = id("memory://fixtures/root.yaml");
    let shared = id("memory://fixtures/shared.yaml");
    let config = automatic_config();
    let mut resolver = BuiltinResolver::new(retriever(&shared, "42\n"), root, config.clone());

    let events = drive(
        &mut resolver,
        "---\n{$ref: ./shared.yaml}\n---\n{$ref: ./shared.yaml}\n",
        &config,
    )
    .unwrap();

    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::Scalar { value, .. } if value == "42"))
            .count(),
        2
    );
    assert_eq!(resolver.reports().len(), 2);
    for report in resolver.reports() {
        assert_eq!(report.edges().len(), 1);
        assert!(!report.edges()[0].cache_hit);
        assert_eq!(report.resources().len(), 2);
    }
}

#[test]
fn builtin_resolution_uses_zero_width_directive_spans() {
    let root = id("memory://fixtures/root.yaml");
    let shared = id("memory://fixtures/shared.yaml");
    let config = automatic_config();
    let input = "answer:\n  $ref: ./shared.yaml#/value\n";
    let mut resolver = BuiltinResolver::new(
        retriever(&shared, "value: [forty, two]\n"),
        root,
        config.clone(),
    );

    let events = drive(&mut resolver, input, &config).unwrap();
    let directive_span = resolver.reports()[0].edges()[0].source_span;
    let replacement = events
        .iter()
        .filter(|event| match event {
            Event::SequenceStart { .. } | Event::SequenceEnd { .. } => true,
            Event::Scalar { value, .. } => value == "forty" || value == "two",
            _ => false,
        })
        .collect::<Vec<_>>();

    assert!(!replacement.is_empty());
    for event in replacement {
        let span = event.span().unwrap();
        assert_eq!(span.start, span.end);
        assert_eq!(span.start, directive_span.start);
    }
    assert_ne!(directive_span.start, directive_span.end);
}

#[test]
fn builtin_resolution_stops_after_one_terminal_error() {
    let root = id("memory://fixtures/root.yaml");
    let config = automatic_config();
    let resolver = BuiltinResolver::new(MemoryRetriever::default(), root, config.clone());
    let parser = Parser::with_config("{$ref: ./missing.yaml}\n---\nafter\n", &config)
        .map(|event| event.map_err(ResolveError::Parse));
    let mut events = ResolvedEvents::new_with_config(parser, resolver, &config);

    assert!(matches!(events.next(), Some(Ok(Event::StreamStart))));
    let error = events.next().unwrap().unwrap_err();
    assert!(matches!(
        error,
        ResolveError::Resolution {
            category: ResolutionErrorCategory::Retrieval,
            ..
        }
    ));
    assert!(events.next().is_none());
    assert!(events.next().is_none());
}

#[test]
fn builtin_resolution_disabled_policy_is_borrowed_passthrough() {
    let config = LoaderConfig::default();
    let root = id("memory://fixtures/root.yaml");
    let mut resolver = BuiltinResolver::new(MemoryRetriever::default(), root, config.clone());
    let event = Event::Scalar {
        anchor: None,
        tag: None,
        value: Cow::Borrowed("borrowed"),
        style: yamalgam_parser::ScalarStyle::Plain,
        chomp: yamalgam_parser::Chomp::Clip,
        flags: yamalgam_parser::AtomFlags::empty(),
        span: Default::default(),
    };

    let output = resolver.on_event_with_config(event, &config).unwrap();

    assert!(matches!(
        output.as_slice(),
        [Event::Scalar {
            value: Cow::Borrowed("borrowed"),
            ..
        }]
    ));
    assert!(resolver.reports().is_empty());
}

#[test]
fn builtin_resolution_rejects_every_caller_session_config_mismatch() {
    let session_config = automatic_config();
    let mut mismatches = Vec::new();

    let mut mode = session_config.clone();
    mode.resolution.include.mode = ResolutionMode::Disabled;
    mismatches.push(mode);

    let mut root = session_config.clone();
    root.resolution.files.root = Some(PathBuf::from("/different-root"));
    mismatches.push(root);

    let mut host = session_config.clone();
    host.resolution.refs.allow_hosts = vec!["example.com".into()];
    mismatches.push(host);

    let mut limit = session_config.clone();
    limit.resolution.limits.max_depth = Some(1);
    mismatches.push(limit);

    for caller_config in mismatches {
        let mut resolver = BuiltinResolver::new(
            MemoryRetriever::default(),
            id("memory://fixtures/root.yaml"),
            session_config.clone(),
        );

        let error = resolver
            .on_event_with_config(Event::StreamStart, &caller_config)
            .unwrap_err();

        assert!(matches!(error, ResolveError::Custom(_)));
        assert!(resolver.reports().is_empty());
    }
}

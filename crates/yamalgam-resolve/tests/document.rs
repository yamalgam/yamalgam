#![allow(missing_docs)]

use std::borrow::Cow;
use std::mem;

use yamalgam_core::Span;
use yamalgam_parser::{AtomFlags, Chomp, CollectionStyle, Event, Parser, ScalarStyle};
use yamalgam_resolve::{
    DocumentError, OwnedDocument, OwnedKind, OwnedNode, OwnedScalar, ResourceId,
};

fn origin() -> ResourceId {
    ResourceId::new("memory://documents/root.yaml").unwrap()
}

fn parse(input: &str) -> Vec<Event<'_>> {
    Parser::new(input).collect::<Result<Vec<_>, _>>().unwrap()
}

fn scalar(value: &'static str) -> Event<'static> {
    Event::Scalar {
        anchor: None,
        tag: None,
        value: Cow::Borrowed(value),
        style: ScalarStyle::Plain,
        chomp: Chomp::Clip,
        flags: AtomFlags::DIRECT_OUTPUT,
        span: Span::default(),
    }
}

fn semantic(events: &[Event<'_>]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::SequenceStart {
                anchor, tag, style, ..
            } => Some(format!("sequence-start:{anchor:?}:{tag:?}:{style:?}")),
            Event::SequenceEnd { .. } => Some("sequence-end".into()),
            Event::MappingStart {
                anchor, tag, style, ..
            } => Some(format!("mapping-start:{anchor:?}:{tag:?}:{style:?}")),
            Event::MappingEnd { .. } => Some("mapping-end".into()),
            Event::Scalar {
                anchor,
                tag,
                value,
                style,
                chomp,
                flags,
                ..
            } => Some(format!(
                "scalar:{anchor:?}:{tag:?}:{value:?}:{style:?}:{chomp:?}:{}",
                flags.bits()
            )),
            Event::Alias { name, .. } => Some(format!("alias:{name}")),
            _ => None,
        })
        .collect()
}

#[test]
fn round_trip_preserves_semantic_nodes_and_presentation_metadata() {
    let input = r#"# structural comment
? &complex [plain, "escaped\n"]
: &payload !example
  literal: |+
    line

  alias: *complex
"#;
    let events = parse(input);
    assert!(events.iter().any(Event::is_structural));
    let document = OwnedDocument::from_events(origin(), events.clone()).unwrap();

    let OwnedKind::Mapping { style, entries } = &document.root.kind else {
        panic!("expected root mapping");
    };
    assert_eq!(*style, CollectionStyle::Block);
    assert_eq!(entries.len(), 1);
    let (key, value) = &entries[0];
    assert_eq!(key.anchor.as_deref(), Some("complex"));
    let OwnedKind::Sequence { style, items } = &key.kind else {
        panic!("expected arbitrary sequence key");
    };
    assert_eq!(*style, CollectionStyle::Flow);
    assert_eq!(items.len(), 2);
    let OwnedKind::Scalar(escaped) = &items[1].kind else {
        panic!("expected escaped scalar");
    };
    assert_eq!(escaped.style, ScalarStyle::DoubleQuoted);
    assert!(escaped.flags.contains(AtomFlags::HAS_ESC));
    assert_eq!(value.anchor.as_deref(), Some("payload"));
    assert_eq!(value.tag.as_deref(), Some("!example"));
    let OwnedKind::Mapping { entries, .. } = &value.kind else {
        panic!("expected tagged mapping value");
    };
    let OwnedKind::Scalar(literal) = &entries[0].1.kind else {
        panic!("expected literal block");
    };
    assert_eq!(literal.style, ScalarStyle::Literal);
    assert_eq!(literal.chomp, Chomp::Keep);
    assert!(literal.flags.contains(AtomFlags::CHOMP_EXPLICIT));
    assert!(matches!(
        &entries[1].1.kind,
        OwnedKind::Alias { name } if name == "complex"
    ));
    assert!(all_origins_are(&document.root, &origin()));

    let replay: Vec<Event<'static>> = document.to_events();
    assert_eq!(semantic(&events), semantic(&replay));
}

#[test]
fn event_into_owned_detaches_every_borrowed_string() {
    let input = "---\n&root !tag [*root, value]\n".to_owned();
    let events = parse(&input);
    let owned: Vec<Event<'static>> = events.into_iter().map(Event::into_owned).collect();
    drop(input);

    assert!(owned.iter().any(|event| matches!(
        event,
        Event::SequenceStart {
            anchor: Some(Cow::Owned(anchor)),
            tag: Some(Cow::Owned(tag)),
            ..
        } if anchor == "root" && tag == "!tag"
    )));
    assert!(owned.iter().any(|event| matches!(
        event,
        Event::Scalar {
            value: Cow::Owned(value),
            ..
        } if value == "value"
    )));
}

#[test]
fn empty_document_is_a_real_empty_scalar_but_empty_stream_is_an_error() {
    let document = OwnedDocument::from_events(origin(), parse("---\n...\n")).unwrap();
    assert!(matches!(
        &document.root.kind,
        OwnedKind::Scalar(OwnedScalar { value, .. }) if value.is_empty()
    ));
    assert!(matches!(
        OwnedDocument::from_events(origin(), parse("")),
        Err(DocumentError::EmptyStream)
    ));
}

#[test]
fn multi_document_streams_are_rejected_before_selection() {
    assert!(matches!(
        OwnedDocument::from_events(origin(), parse("---\none\n---\ntwo\n")),
        Err(DocumentError::MultipleDocuments)
    ));
}

#[test]
fn malformed_event_streams_fail_with_specific_structure_errors() {
    let span = Span::default();
    let document_start = Event::DocumentStart {
        implicit: true,
        span,
    };
    let sequence_start = Event::SequenceStart {
        anchor: None,
        tag: None,
        style: CollectionStyle::Block,
        span,
    };
    let mapping_start = Event::MappingStart {
        anchor: None,
        tag: None,
        style: CollectionStyle::Block,
        span,
    };

    assert!(matches!(
        OwnedDocument::from_events(
            origin(),
            vec![
                document_start.clone(),
                sequence_start.clone(),
                scalar("item")
            ]
        ),
        Err(DocumentError::TruncatedCollection)
    ));
    assert!(matches!(
        OwnedDocument::from_events(
            origin(),
            vec![
                document_start.clone(),
                mapping_start,
                scalar("key"),
                Event::MappingEnd { span },
            ]
        ),
        Err(DocumentError::MappingParity)
    ));
    assert!(matches!(
        OwnedDocument::from_events(
            origin(),
            vec![document_start.clone(), scalar("one"), scalar("two")]
        ),
        Err(DocumentError::MultipleRoots)
    ));
    assert!(matches!(
        OwnedDocument::from_events(
            origin(),
            vec![document_start, sequence_start, Event::StreamEnd]
        ),
        Err(DocumentError::UnexpectedEvent("stream end"))
    ));
}

#[test]
fn materialized_weight_is_exact_for_strings_and_stack_safe_for_deep_nodes() {
    let scalar = OwnedNode {
        origin: origin(),
        span: Span::default(),
        anchor: Some("a".into()),
        tag: Some("t".into()),
        kind: OwnedKind::Scalar(OwnedScalar {
            value: "abc".into(),
            style: ScalarStyle::Plain,
            chomp: Chomp::Clip,
            flags: AtomFlags::DIRECT_OUTPUT,
        }),
    };
    let expected = mem::size_of::<OwnedNode>() + "a".len() + "t".len() + "abc".len();
    assert_eq!(scalar.materialized_weight().unwrap(), expected);

    let mut deep = scalar;
    for _ in 0..20_000 {
        deep = OwnedNode {
            origin: origin(),
            span: Span::default(),
            anchor: None,
            tag: None,
            kind: OwnedKind::Sequence {
                style: CollectionStyle::Block,
                items: vec![deep],
            },
        };
    }
    assert!(deep.materialized_weight().unwrap() > expected);
    mem::forget(deep);
}

fn all_origins_are(root: &OwnedNode, expected: &ResourceId) -> bool {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if &node.origin != expected {
            return false;
        }
        match &node.kind {
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
    true
}

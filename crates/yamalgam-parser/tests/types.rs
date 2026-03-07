#![allow(missing_docs)]

use std::borrow::Cow;

use pretty_assertions::assert_eq;
use yamalgam_core::Span;
use yamalgam_parser::{CollectionStyle, Event, ParseError, ScalarStyle};
use yamalgam_scanner::scanner::ScanError;

#[test]
fn event_stream_start_constructable() {
    let event: Event<'_> = Event::StreamStart;
    assert_eq!(format!("{event:?}"), "StreamStart");
}

#[test]
fn event_stream_end_constructable() {
    let event: Event<'_> = Event::StreamEnd;
    assert_eq!(format!("{event:?}"), "StreamEnd");
}

#[test]
fn event_version_directive_constructable() {
    let event: Event<'_> = Event::VersionDirective {
        major: 1,
        minor: 2,
        span: Span::default(),
    };
    assert!(matches!(
        event,
        Event::VersionDirective {
            major: 1,
            minor: 2,
            ..
        }
    ));
}

#[test]
fn event_tag_directive_constructable() {
    let event: Event<'_> = Event::TagDirective {
        handle: Cow::Borrowed("!!"),
        prefix: Cow::Borrowed("tag:yaml.org,2002:"),
        span: Span::default(),
    };
    assert!(matches!(event, Event::TagDirective { .. }));
}

#[test]
fn event_document_start_constructable() {
    let event: Event<'_> = Event::DocumentStart {
        implicit: true,
        span: Span::default(),
    };
    assert!(matches!(
        event,
        Event::DocumentStart {
            implicit: true,
            ..
        }
    ));
}

#[test]
fn event_document_end_constructable() {
    let event: Event<'_> = Event::DocumentEnd {
        implicit: true,
        span: Span::default(),
    };
    assert!(matches!(
        event,
        Event::DocumentEnd {
            implicit: true,
            ..
        }
    ));
}

#[test]
fn event_sequence_start_constructable() {
    let event: Event<'_> = Event::SequenceStart {
        anchor: None,
        tag: None,
        style: CollectionStyle::Block,
        span: Span::default(),
    };
    assert!(matches!(
        event,
        Event::SequenceStart {
            style: CollectionStyle::Block,
            ..
        }
    ));
}

#[test]
fn event_sequence_end_constructable() {
    let event: Event<'_> = Event::SequenceEnd {
        span: Span::default(),
    };
    assert!(matches!(event, Event::SequenceEnd { .. }));
}

#[test]
fn event_mapping_start_constructable() {
    let event: Event<'_> = Event::MappingStart {
        anchor: Some(Cow::Borrowed("foo")),
        tag: Some(Cow::Borrowed("!!map")),
        style: CollectionStyle::Flow,
        span: Span::default(),
    };
    assert!(matches!(
        event,
        Event::MappingStart {
            style: CollectionStyle::Flow,
            ..
        }
    ));
}

#[test]
fn event_mapping_end_constructable() {
    let event: Event<'_> = Event::MappingEnd {
        span: Span::default(),
    };
    assert!(matches!(event, Event::MappingEnd { .. }));
}

#[test]
fn event_scalar_constructable() {
    let event: Event<'_> = Event::Scalar {
        anchor: None,
        tag: None,
        value: Cow::Borrowed("hello"),
        style: ScalarStyle::Plain,
        span: Span::default(),
    };
    assert!(matches!(
        event,
        Event::Scalar {
            style: ScalarStyle::Plain,
            ..
        }
    ));
}

#[test]
fn event_alias_constructable() {
    let event: Event<'_> = Event::Alias {
        name: Cow::Borrowed("anchor1"),
        span: Span::default(),
    };
    assert!(matches!(event, Event::Alias { .. }));
}

#[test]
fn collection_style_variants() {
    assert_ne!(CollectionStyle::Block, CollectionStyle::Flow);
    assert_eq!(CollectionStyle::Block, CollectionStyle::Block);
}

#[test]
fn parse_error_from_scan_error() {
    let scan_err = ScanError {
        message: "test scan error".to_string(),
    };
    let parse_err: ParseError = scan_err.into();
    assert!(matches!(parse_err, ParseError::Scan(_)));
    assert!(parse_err.to_string().contains("test scan error"));
}

#![allow(missing_docs)]

use pretty_assertions::assert_eq;
use yamalgam_parser::{Event, Parser};

#[test]
fn empty_stream() {
    let events: Vec<_> = Parser::new("").collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0], Event::StreamStart));
    assert!(matches!(events[1], Event::StreamEnd));
}

#[test]
fn whitespace_only_stream() {
    let events: Vec<_> = Parser::new("   \n\n  ")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0], Event::StreamStart));
    assert!(matches!(events[1], Event::StreamEnd));
}

#[test]
fn iterator_returns_none_after_stream_end() {
    let mut parser = Parser::new("");
    let e1 = parser.next();
    assert!(e1.is_some());
    let e2 = parser.next();
    assert!(e2.is_some());
    let e3 = parser.next();
    assert!(e3.is_none());
    // Subsequent calls also return None (fused behavior).
    assert!(parser.next().is_none());
}

#[test]
fn iterator_stops_on_error() {
    // Feed a broken token stream: scanner error should propagate.
    use yamalgam_scanner::scanner::ScanError;
    use yamalgam_scanner::Token;

    let tokens: Vec<Result<Token<'_>, ScanError>> = vec![Err(ScanError {
        message: "boom".to_string(),
    })];
    let mut parser = Parser::from_tokens(tokens.into_iter());
    let result = parser.next();
    assert!(result.is_some());
    assert!(result.unwrap().is_err());
    // After error, iterator is done.
    assert!(parser.next().is_none());
}

// -- Document handling tests --

#[test]
fn implicit_document_with_scalar() {
    let events: Vec<_> = Parser::new("hello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(events.len(), 5);
    assert!(matches!(events[0], Event::StreamStart));
    assert!(matches!(
        events[1],
        Event::DocumentStart {
            implicit: true,
            ..
        }
    ));
    assert!(matches!(events[2], Event::Scalar { .. }));
    assert!(matches!(
        events[3],
        Event::DocumentEnd {
            implicit: true,
            ..
        }
    ));
    assert!(matches!(events[4], Event::StreamEnd));
}

#[test]
fn explicit_document_start() {
    let events: Vec<_> = Parser::new("---\nhello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(
        events[1],
        Event::DocumentStart {
            implicit: false,
            ..
        }
    ));
}

#[test]
fn explicit_document_end() {
    let events: Vec<_> = Parser::new("hello\n...")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(
        events[3],
        Event::DocumentEnd {
            implicit: false,
            ..
        }
    ));
}

#[test]
fn version_directive_as_event() {
    let events: Vec<_> = Parser::new("%YAML 1.2\n---\nhello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(
        events[1],
        Event::VersionDirective {
            major: 1,
            minor: 2,
            ..
        }
    ));
    assert!(matches!(
        events[2],
        Event::DocumentStart {
            implicit: false,
            ..
        }
    ));
}

#[test]
fn tag_directive_as_event() {
    let events: Vec<_> = Parser::new("%TAG !e! tag:example.com,2000:\n---\nhello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(events[1], Event::TagDirective { .. }));
    if let Event::TagDirective {
        ref handle,
        ref prefix,
        ..
    } = events[1]
    {
        assert_eq!(handle.as_ref(), "!e!");
        assert_eq!(prefix.as_ref(), "tag:example.com,2000:");
    }
}

#[test]
fn multi_document() {
    let events: Vec<_> = Parser::new("hello\n---\nworld")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let doc_start_count = events
        .iter()
        .filter(|e| matches!(e, Event::DocumentStart { .. }))
        .count();
    assert_eq!(doc_start_count, 2);
}

#[test]
fn empty_document() {
    let events: Vec<_> = Parser::new("---\n...")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // StreamStart, DocStart(explicit), Scalar(""), DocEnd(explicit), StreamEnd
    assert_eq!(events.len(), 5);
    assert!(matches!(events[2], Event::Scalar { .. }));
    if let Event::Scalar { ref value, .. } = events[2] {
        assert_eq!(value.as_ref(), "");
    }
}

#[test]
fn multiple_directives() {
    let events: Vec<_> =
        Parser::new("%YAML 1.2\n%TAG !! tag:yaml.org,2002:\n---\nhello")
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
    // StreamStart, VersionDirective, TagDirective, DocStart, Scalar, DocEnd, StreamEnd
    assert_eq!(events.len(), 7);
    assert!(matches!(events[1], Event::VersionDirective { .. }));
    assert!(matches!(events[2], Event::TagDirective { .. }));
    assert!(matches!(
        events[3],
        Event::DocumentStart {
            implicit: false,
            ..
        }
    ));
}

#[test]
fn document_end_then_new_document() {
    let events: Vec<_> = Parser::new("hello\n...\nworld")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // StreamStart, DocStart(i), Scalar(hello), DocEnd(explicit), DocStart(i), Scalar(world), DocEnd(i), StreamEnd
    assert_eq!(events.len(), 8);
    assert!(matches!(
        events[3],
        Event::DocumentEnd {
            implicit: false,
            ..
        }
    ));
    assert!(matches!(
        events[4],
        Event::DocumentStart {
            implicit: true,
            ..
        }
    ));
}

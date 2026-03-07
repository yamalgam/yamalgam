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

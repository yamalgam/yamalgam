#![allow(missing_docs)]

use yamalgam_scanner::TokenKind;
use yamalgam_scanner::scanner::Scanner;

#[test]
fn empty_input_produces_stream_start_and_end() {
    let scanner = Scanner::new("");
    let tokens: Vec<_> = scanner.collect::<Result<Vec<_>, _>>().unwrap();

    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::StreamStart);
    assert_eq!(tokens[1].kind, TokenKind::StreamEnd);
}

#[test]
fn stream_start_has_zero_span() {
    let scanner = Scanner::new("");
    let tokens = scanner.collect::<Result<Vec<_>, _>>().unwrap();
    let start_token = &tokens[0];

    assert_eq!(start_token.atom.span.start.offset, 0);
    assert_eq!(start_token.atom.span.start.line, 0);
    assert_eq!(start_token.atom.span.start.column, 0);
}

#[test]
fn stream_end_span_reflects_input_length() {
    let input = "hello";
    let scanner = Scanner::new(input);
    let tokens = scanner.collect::<Result<Vec<_>, _>>().unwrap();
    let end_token = &tokens[1];

    assert_eq!(end_token.atom.span.start.offset, input.len());
}

#[test]
fn nonempty_input_still_produces_only_stream_markers() {
    // Until content scanning is implemented, all content is skipped.
    let scanner = Scanner::new("key: value\n");
    let tokens = scanner.collect::<Result<Vec<_>, _>>().unwrap();

    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::StreamStart);
    assert_eq!(tokens[1].kind, TokenKind::StreamEnd);
}

#[test]
fn scanner_is_fused_after_stream_end() {
    let mut scanner = Scanner::new("");
    let _ = scanner.next(); // StreamStart
    let _ = scanner.next(); // StreamEnd
    assert!(scanner.next().is_none());
    assert!(scanner.next().is_none()); // stays None
}

#[test]
fn stream_start_atom_data_is_empty() {
    let scanner = Scanner::new("anything");
    let tokens = scanner.collect::<Result<Vec<_>, _>>().unwrap();

    assert_eq!(tokens[0].atom.data.as_ref(), "");
    assert_eq!(tokens[1].atom.data.as_ref(), "");
}

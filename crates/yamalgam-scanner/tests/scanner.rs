#![allow(missing_docs)]

use yamalgam_scanner::TokenKind;
use yamalgam_scanner::scanner::Scanner;

/// Collect all tokens from a scanner, unwrapping each Result.
fn scan(input: &str) -> Vec<(TokenKind, String)> {
    Scanner::new(input)
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
        .into_iter()
        .map(|t| (t.kind, t.atom.data.to_string()))
        .collect()
}

/// Extract just the token kinds from a scan.
fn kinds(input: &str) -> Vec<TokenKind> {
    scan(input).into_iter().map(|(k, _)| k).collect()
}

// === Stream markers (from PR #9) ===

#[test]
fn empty_input_produces_stream_start_and_end() {
    assert_eq!(
        kinds(""),
        vec![TokenKind::StreamStart, TokenKind::StreamEnd]
    );
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
    // Content is skipped since content scanning is not yet implemented.
    assert_eq!(
        kinds("key: value\n"),
        vec![TokenKind::StreamStart, TokenKind::StreamEnd]
    );
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
    let tokens = scan("anything");
    assert_eq!(tokens[0].1, "");
    assert_eq!(tokens.last().unwrap().1, "");
}

// === Document markers ===

#[test]
fn explicit_document_start() {
    assert_eq!(
        kinds("---\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::DocumentStart,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn explicit_document_end() {
    assert_eq!(
        kinds("...\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::DocumentEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn document_start_and_end_with_content_skipped() {
    // Content between markers is skipped (not yet scanned).
    assert_eq!(
        kinds("---\nkey: value\n...\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::DocumentStart,
            TokenKind::DocumentEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn document_start_followed_by_eof() {
    // --- at EOF (no trailing newline) is still a valid document start.
    assert_eq!(
        kinds("---"),
        vec![
            TokenKind::StreamStart,
            TokenKind::DocumentStart,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn document_start_followed_by_space() {
    // --- followed by space is a valid document start.
    assert_eq!(
        kinds("--- \n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::DocumentStart,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn triple_dash_not_at_column_zero_is_not_document_start() {
    // --- must be at column 0.
    assert_eq!(
        kinds(" ---\n"),
        vec![TokenKind::StreamStart, TokenKind::StreamEnd]
    );
}

#[test]
fn triple_dash_without_trailing_blank_is_not_document_start() {
    // ---x is a scalar, not a document start. Content is skipped.
    assert_eq!(
        kinds("---x\n"),
        vec![TokenKind::StreamStart, TokenKind::StreamEnd]
    );
}

#[test]
fn multiple_documents() {
    assert_eq!(
        kinds("---\n...\n---\n...\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::DocumentStart,
            TokenKind::DocumentEnd,
            TokenKind::DocumentStart,
            TokenKind::DocumentEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn document_start_span_covers_three_chars() {
    let scanner = Scanner::new("---\n");
    let tokens = scanner.collect::<Result<Vec<_>, _>>().unwrap();
    let doc_start = &tokens[1];

    assert_eq!(doc_start.atom.span.start.offset, 0);
    assert_eq!(doc_start.atom.span.end.offset, 3);
}

// === Whitespace and comment skipping ===

#[test]
fn blank_lines_are_skipped() {
    assert_eq!(
        kinds("\n\n---\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::DocumentStart,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn comments_are_skipped() {
    assert_eq!(
        kinds("# this is a comment\n---\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::DocumentStart,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn comment_after_document_start() {
    assert_eq!(
        kinds("--- # start of doc\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::DocumentStart,
            TokenKind::StreamEnd,
        ]
    );
}

// === Directives ===

#[test]
fn version_directive() {
    let tokens = scan("%YAML 1.2\n---\n");
    assert_eq!(tokens[0].0, TokenKind::StreamStart);
    assert_eq!(tokens[1].0, TokenKind::VersionDirective);
    assert_eq!(tokens[1].1, "1.2");
    assert_eq!(tokens[2].0, TokenKind::DocumentStart);
    assert_eq!(tokens[3].0, TokenKind::StreamEnd);
}

#[test]
fn tag_directive() {
    let tokens = scan("%TAG !! tag:yaml.org,2002:\n---\n");
    assert_eq!(tokens[0].0, TokenKind::StreamStart);
    assert_eq!(tokens[1].0, TokenKind::TagDirective);
    assert_eq!(tokens[1].1, "!! tag:yaml.org,2002:");
    assert_eq!(tokens[2].0, TokenKind::DocumentStart);
    assert_eq!(tokens[3].0, TokenKind::StreamEnd);
}

#[test]
fn tag_directive_primary_handle() {
    let tokens = scan("%TAG ! tag:example.com:\n---\n");
    assert_eq!(tokens[1].0, TokenKind::TagDirective);
    assert_eq!(tokens[1].1, "! tag:example.com:");
}

#[test]
fn tag_directive_named_handle() {
    let tokens = scan("%TAG !e! tag:example.com,2000:app/\n---\n");
    assert_eq!(tokens[1].0, TokenKind::TagDirective);
    assert_eq!(tokens[1].1, "!e! tag:example.com,2000:app/");
}

// === Flow indicators ===

#[test]
fn flow_sequence_empty() {
    assert_eq!(
        kinds("[]"),
        vec![
            TokenKind::StreamStart,
            TokenKind::FlowSequenceStart,
            TokenKind::FlowSequenceEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn flow_mapping_empty() {
    assert_eq!(
        kinds("{}"),
        vec![
            TokenKind::StreamStart,
            TokenKind::FlowMappingStart,
            TokenKind::FlowMappingEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn flow_entry_comma() {
    // Scalars between commas are skipped; commas produce FlowEntry.
    assert_eq!(
        kinds("[a, b, c]"),
        vec![
            TokenKind::StreamStart,
            TokenKind::FlowSequenceStart,
            TokenKind::FlowEntry,
            TokenKind::FlowEntry,
            TokenKind::FlowSequenceEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn nested_flow_collections() {
    assert_eq!(
        kinds("[[]]"),
        vec![
            TokenKind::StreamStart,
            TokenKind::FlowSequenceStart,
            TokenKind::FlowSequenceStart,
            TokenKind::FlowSequenceEnd,
            TokenKind::FlowSequenceEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn flow_mapping_in_sequence() {
    assert_eq!(
        kinds("[{}]"),
        vec![
            TokenKind::StreamStart,
            TokenKind::FlowSequenceStart,
            TokenKind::FlowMappingStart,
            TokenKind::FlowMappingEnd,
            TokenKind::FlowSequenceEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn flow_sequence_span_single_char() {
    let scanner = Scanner::new("[]\n");
    let tokens = scanner.collect::<Result<Vec<_>, _>>().unwrap();
    let open = &tokens[1];
    let close = &tokens[2];

    assert_eq!(open.atom.span.start.offset, 0);
    assert_eq!(open.atom.span.end.offset, 1);
    assert_eq!(close.atom.span.start.offset, 1);
    assert_eq!(close.atom.span.end.offset, 2);
}

#[test]
fn flow_indicators_after_document_start() {
    assert_eq!(
        kinds("---\n[]\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::DocumentStart,
            TokenKind::FlowSequenceStart,
            TokenKind::FlowSequenceEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn version_directive_yaml_11() {
    let tokens = scan("%YAML 1.1\n---\n");
    assert_eq!(tokens[1].0, TokenKind::VersionDirective);
    assert_eq!(tokens[1].1, "1.1");
}

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
    let end_token = tokens.last().unwrap();

    assert_eq!(end_token.kind, TokenKind::StreamEnd);
    assert_eq!(end_token.atom.span.start.offset, input.len());
}

#[test]
fn nonempty_input_produces_full_mapping() {
    // Plain scalar + simple key resolution produce full mapping tokens.
    assert_eq!(
        kinds("key: value\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::BlockMappingStart,
            TokenKind::Key,
            TokenKind::Scalar,
            TokenKind::Value,
            TokenKind::Scalar,
            TokenKind::BlockEnd,
            TokenKind::StreamEnd,
        ]
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
fn document_with_content_between_markers() {
    assert_eq!(
        kinds("---\nkey: value\n...\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::DocumentStart,
            TokenKind::BlockMappingStart,
            TokenKind::Key,
            TokenKind::Scalar,
            TokenKind::Value,
            TokenKind::Scalar,
            TokenKind::BlockEnd,
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
    // --- must be at column 0 to be a document start.
    // Without scalar parsing, the third `-` (followed by newline) is
    // detected as a block entry. This will self-correct with plain scalars.
    let k = kinds(" ---\n");
    assert!(!k.contains(&TokenKind::DocumentStart));
}

#[test]
fn triple_dash_without_trailing_blank_is_not_document_start() {
    // ---x is a scalar, not a document start.
    let k = kinds("---x\n");
    assert!(!k.contains(&TokenKind::DocumentStart));
    assert!(k.contains(&TokenKind::Scalar));
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
    assert_eq!(
        kinds("[a, b, c]"),
        vec![
            TokenKind::StreamStart,
            TokenKind::FlowSequenceStart,
            TokenKind::Scalar,
            TokenKind::FlowEntry,
            TokenKind::Scalar,
            TokenKind::FlowEntry,
            TokenKind::Scalar,
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

// === Block indicators ===

#[test]
fn block_sequence_entry() {
    assert_eq!(
        kinds("- a\n- b\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::BlockSequenceStart,
            TokenKind::BlockEntry,
            TokenKind::Scalar,
            TokenKind::BlockEntry,
            TokenKind::Scalar,
            TokenKind::BlockEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn nested_block_sequence() {
    assert_eq!(
        kinds("-\n  - a\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::BlockSequenceStart,
            TokenKind::BlockEntry,
            TokenKind::BlockSequenceStart,
            TokenKind::BlockEntry,
            TokenKind::Scalar,
            TokenKind::BlockEnd,
            TokenKind::BlockEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn block_entry_after_document_start() {
    assert_eq!(
        kinds("---\n- a\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::DocumentStart,
            TokenKind::BlockSequenceStart,
            TokenKind::BlockEntry,
            TokenKind::Scalar,
            TokenKind::BlockEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn explicit_key() {
    assert_eq!(
        kinds("? a\n: b\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::BlockMappingStart,
            TokenKind::Key,
            TokenKind::Scalar,
            TokenKind::Value,
            TokenKind::Scalar,
            TokenKind::BlockEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn block_sequence_dedent_produces_block_end() {
    assert_eq!(
        kinds("- a\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::BlockSequenceStart,
            TokenKind::BlockEntry,
            TokenKind::Scalar,
            TokenKind::BlockEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn multiple_dedents_produce_multiple_block_ends() {
    assert_eq!(
        kinds("-\n  -\n    - a\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::BlockSequenceStart,
            TokenKind::BlockEntry,
            TokenKind::BlockSequenceStart,
            TokenKind::BlockEntry,
            TokenKind::BlockSequenceStart,
            TokenKind::BlockEntry,
            TokenKind::Scalar,
            TokenKind::BlockEnd,
            TokenKind::BlockEnd,
            TokenKind::BlockEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn document_end_unrolls_block_indent() {
    assert_eq!(
        kinds("---\n- a\n...\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::DocumentStart,
            TokenKind::BlockSequenceStart,
            TokenKind::BlockEntry,
            TokenKind::Scalar,
            TokenKind::BlockEnd,
            TokenKind::DocumentEnd,
            TokenKind::StreamEnd,
        ]
    );
}

// === Plain scalars ===

#[test]
fn plain_scalar_key_value() {
    let tokens = scan("key: value\n");
    assert_eq!(tokens[0].0, TokenKind::StreamStart);
    assert_eq!(tokens[1].0, TokenKind::BlockMappingStart);
    assert_eq!(tokens[2].0, TokenKind::Key);
    assert_eq!(
        (tokens[3].0, tokens[3].1.as_str()),
        (TokenKind::Scalar, "key")
    );
    assert_eq!(tokens[4].0, TokenKind::Value);
    assert_eq!(
        (tokens[5].0, tokens[5].1.as_str()),
        (TokenKind::Scalar, "value")
    );
    assert_eq!(tokens[6].0, TokenKind::BlockEnd);
    assert_eq!(tokens[7].0, TokenKind::StreamEnd);
}

#[test]
fn multiple_key_value_pairs() {
    assert_eq!(
        kinds("a: 1\nb: 2\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::BlockMappingStart,
            TokenKind::Key,
            TokenKind::Scalar,
            TokenKind::Value,
            TokenKind::Scalar,
            TokenKind::Key,
            TokenKind::Scalar,
            TokenKind::Value,
            TokenKind::Scalar,
            TokenKind::BlockEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn standalone_scalar() {
    let tokens = scan("hello\n");
    assert_eq!(tokens[0].0, TokenKind::StreamStart);
    assert_eq!(
        (tokens[1].0, tokens[1].1.as_str()),
        (TokenKind::Scalar, "hello")
    );
    assert_eq!(tokens[2].0, TokenKind::StreamEnd);
}

#[test]
fn scalar_in_sequence() {
    let tokens = scan("- hello\n- world\n");
    let k: Vec<_> = tokens.iter().map(|t| t.0).collect();
    assert_eq!(
        k,
        vec![
            TokenKind::StreamStart,
            TokenKind::BlockSequenceStart,
            TokenKind::BlockEntry,
            TokenKind::Scalar,
            TokenKind::BlockEntry,
            TokenKind::Scalar,
            TokenKind::BlockEnd,
            TokenKind::StreamEnd,
        ]
    );
    assert_eq!(tokens[3].1, "hello");
    assert_eq!(tokens[5].1, "world");
}

#[test]
fn scalar_trailing_whitespace_trimmed() {
    let tokens = scan("key : value\n");
    assert_eq!(tokens[3].1, "key");
    assert_eq!(tokens[5].1, "value");
}

#[test]
fn scalar_with_comment() {
    let tokens = scan("hello # comment\n");
    assert_eq!(
        (tokens[1].0, tokens[1].1.as_str()),
        (TokenKind::Scalar, "hello")
    );
}

#[test]
fn flow_sequence_with_scalars() {
    let tokens = scan("[a, b, c]");
    let k: Vec<_> = tokens.iter().map(|t| t.0).collect();
    assert_eq!(
        k,
        vec![
            TokenKind::StreamStart,
            TokenKind::FlowSequenceStart,
            TokenKind::Scalar,
            TokenKind::FlowEntry,
            TokenKind::Scalar,
            TokenKind::FlowEntry,
            TokenKind::Scalar,
            TokenKind::FlowSequenceEnd,
            TokenKind::StreamEnd,
        ]
    );
    assert_eq!(tokens[2].1, "a");
    assert_eq!(tokens[4].1, "b");
    assert_eq!(tokens[6].1, "c");
}

#[test]
fn nested_mapping() {
    assert_eq!(
        kinds("a:\n  b: c\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::BlockMappingStart,
            TokenKind::Key,
            TokenKind::Scalar,
            TokenKind::Value,
            TokenKind::BlockMappingStart,
            TokenKind::Key,
            TokenKind::Scalar,
            TokenKind::Value,
            TokenKind::Scalar,
            TokenKind::BlockEnd,
            TokenKind::BlockEnd,
            TokenKind::StreamEnd,
        ]
    );
}

// === Quoted scalars ===

#[test]
fn double_quoted_scalar() {
    let tokens = scan("\"hello world\"\n");
    assert_eq!(
        (tokens[1].0, tokens[1].1.as_str()),
        (TokenKind::Scalar, "hello world")
    );
}

#[test]
fn single_quoted_scalar() {
    let tokens = scan("'hello world'\n");
    assert_eq!(
        (tokens[1].0, tokens[1].1.as_str()),
        (TokenKind::Scalar, "hello world")
    );
}

#[test]
fn double_quoted_escape_sequences() {
    let tokens = scan("\"a\\nb\\tc\"\n");
    assert_eq!(tokens[1].1, "a\nb\tc");
}

#[test]
fn double_quoted_backslash_escape() {
    let tokens = scan("\"a\\\\b\"\n");
    assert_eq!(tokens[1].1, "a\\b");
}

#[test]
fn double_quoted_quote_escape() {
    let tokens = scan("\"a\\\"b\"\n");
    assert_eq!(tokens[1].1, "a\"b");
}

#[test]
fn single_quoted_apostrophe_escape() {
    let tokens = scan("'it''s'\n");
    assert_eq!(tokens[1].1, "it's");
}

#[test]
fn quoted_scalar_as_key() {
    let tokens = scan("\"key\": value\n");
    assert_eq!(tokens[1].0, TokenKind::BlockMappingStart);
    assert_eq!(tokens[2].0, TokenKind::Key);
    assert_eq!(
        (tokens[3].0, tokens[3].1.as_str()),
        (TokenKind::Scalar, "key")
    );
    assert_eq!(tokens[4].0, TokenKind::Value);
}

#[test]
fn single_quoted_as_key() {
    let tokens = scan("'key': value\n");
    assert_eq!(tokens[2].0, TokenKind::Key);
    assert_eq!(tokens[3].1, "key");
}

#[test]
fn quoted_scalar_as_value() {
    let tokens = scan("key: \"value\"\n");
    assert_eq!(tokens[5].1, "value");
}

#[test]
fn empty_double_quoted() {
    let tokens = scan("\"\"\n");
    assert_eq!((tokens[1].0, tokens[1].1.as_str()), (TokenKind::Scalar, ""));
}

#[test]
fn empty_single_quoted() {
    let tokens = scan("''\n");
    assert_eq!((tokens[1].0, tokens[1].1.as_str()), (TokenKind::Scalar, ""));
}

#[test]
fn double_quoted_unicode_escape() {
    let tokens = scan("\"\\u0041\"\n"); // \u0041 = 'A'
    assert_eq!(tokens[1].1, "A");
}

#[test]
fn quoted_in_flow_sequence() {
    let tokens = scan("[\"a\", 'b']\n");
    assert_eq!(tokens[2].1, "a");
    assert_eq!(tokens[4].1, "b");
}

// === Block scalars ===

#[test]
fn literal_block_scalar() {
    let tokens = scan("text: |\n  line1\n  line2\n");
    // Find the second scalar (the block scalar value).
    let scalars: Vec<_> = tokens.iter().filter(|t| t.0 == TokenKind::Scalar).collect();
    assert_eq!(scalars[0].1, "text");
    assert_eq!(scalars[1].1, "line1\nline2\n");
}

#[test]
fn folded_block_scalar() {
    let tokens = scan("text: >\n  line1\n  line2\n");
    let scalars: Vec<_> = tokens.iter().filter(|t| t.0 == TokenKind::Scalar).collect();
    assert_eq!(scalars[1].1, "line1 line2\n");
}

#[test]
fn literal_strip_chomp() {
    let tokens = scan("text: |-\n  line1\n");
    let scalars: Vec<_> = tokens.iter().filter(|t| t.0 == TokenKind::Scalar).collect();
    assert_eq!(scalars[1].1, "line1");
}

#[test]
fn literal_keep_chomp() {
    let tokens = scan("text: |+\n  line1\n\n");
    let scalars: Vec<_> = tokens.iter().filter(|t| t.0 == TokenKind::Scalar).collect();
    assert_eq!(scalars[1].1, "line1\n\n");
}

#[test]
fn block_scalar_produces_correct_token_sequence() {
    assert_eq!(
        kinds("key: |\n  hello\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::BlockMappingStart,
            TokenKind::Key,
            TokenKind::Scalar,
            TokenKind::Value,
            TokenKind::Scalar,
            TokenKind::BlockEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn literal_with_explicit_indent() {
    let tokens = scan("text: |2\n  ab\n");
    let scalars: Vec<_> = tokens.iter().filter(|t| t.0 == TokenKind::Scalar).collect();
    assert_eq!(scalars[1].1, "ab\n");
}

#[test]
fn block_scalar_empty_lines_preserved() {
    let tokens = scan("text: |\n  a\n\n  b\n");
    let scalars: Vec<_> = tokens.iter().filter(|t| t.0 == TokenKind::Scalar).collect();
    assert_eq!(scalars[1].1, "a\n\nb\n");
}

#[test]
fn folded_more_indented_preserves_newlines() {
    // In folded scalars, more-indented lines preserve newlines.
    let tokens = scan("text: >\n  a\n    b\n  c\n");
    let scalars: Vec<_> = tokens.iter().filter(|t| t.0 == TokenKind::Scalar).collect();
    assert_eq!(scalars[1].1, "a\n  b\nc\n");
}

#[test]
fn block_scalar_terminates_at_document_end() {
    // Block scalar content must stop at `...` at column 0 (YAML §9.1.2).
    let tokens = scan("--- |\n%!PS-Adobe-2.0\n...\n---\n...\n");
    let scalar = tokens.iter().find(|t| t.0 == TokenKind::Scalar).unwrap();
    assert_eq!(scalar.1, "%!PS-Adobe-2.0\n");
    assert!(tokens.iter().any(|t| t.0 == TokenKind::DocumentEnd));
}

#[test]
fn block_scalar_terminates_at_document_start() {
    // Block scalar content must stop at `---` at column 0 (YAML §9.1.2).
    let tokens = scan("--- |\ncontent\n---\nother\n");
    let scalars: Vec<_> = tokens.iter().filter(|t| t.0 == TokenKind::Scalar).collect();
    assert_eq!(scalars[0].1, "content\n");
    assert_eq!(scalars[1].1, "other");
}

#[test]
fn flow_json_key_colon_on_next_line() {
    // YAML §7.4.2 [153]: after a JSON-like key (quoted scalar), `:` is a
    // value indicator even on the next line, even without trailing whitespace.
    let tokens = scan("{ \"foo\"\n  :bar }\n");
    let k: Vec<_> = tokens.iter().map(|t| t.0).collect();
    assert_eq!(
        k,
        vec![
            TokenKind::StreamStart,
            TokenKind::FlowMappingStart,
            TokenKind::Key,
            TokenKind::Scalar,
            TokenKind::Value,
            TokenKind::Scalar,
            TokenKind::FlowMappingEnd,
            TokenKind::StreamEnd,
        ]
    );
    assert_eq!(tokens[3].1, "foo");
    assert_eq!(tokens[5].1, "bar");
}

#[test]
fn flow_json_key_colon_after_comment_on_next_line() {
    // Same as above but with a comment between key and value (K3WX).
    let tokens = scan("{ \"foo\" # comment\n  :bar }\n");
    let k: Vec<_> = tokens.iter().map(|t| t.0).collect();
    assert_eq!(
        k,
        vec![
            TokenKind::StreamStart,
            TokenKind::FlowMappingStart,
            TokenKind::Key,
            TokenKind::Scalar,
            TokenKind::Value,
            TokenKind::Scalar,
            TokenKind::FlowMappingEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn double_quoted_escape_tab_survives_line_fold() {
    // YAML §6.1: escape sequences produce content characters, not whitespace.
    // `\t` before a line fold must not be stripped (DE56).
    let tokens = scan("\"trailing\\t\n    tab\"\n");
    let scalar = tokens.iter().find(|t| t.0 == TokenKind::Scalar).unwrap();
    assert_eq!(scalar.1, "trailing\t tab");
}

#[test]
fn double_quoted_literal_tab_stripped_on_fold() {
    // Literal tab (not from escape) IS trailing whitespace and should be stripped.
    let tokens = scan("\"trailing\t\n    tab\"\n");
    let scalar = tokens.iter().find(|t| t.0 == TokenKind::Scalar).unwrap();
    assert_eq!(scalar.1, "trailing tab");
}

#[test]
fn tag_uri_percent_decoding() {
    // YAML §6.9.1: percent-encoded characters in tag suffixes are decoded.
    // `%21` → `!` (6CK3).
    let tokens = scan("%TAG !e! tag:example.com,\n---\n!e!tag%21 value\n");
    let tag = tokens.iter().find(|t| t.0 == TokenKind::Tag).unwrap();
    assert_eq!(tag.1, "!e!tag!");
}

#[test]
fn unterminated_double_quoted_scalar_errors() {
    let scanner = Scanner::new("key: \"missing close\n");
    let results: Vec<_> = scanner.collect();
    assert!(results.iter().any(|r| r.is_err()), "expected scan error");
}

#[test]
fn unterminated_single_quoted_scalar_errors() {
    let scanner = Scanner::new("key: 'missing close\n");
    let results: Vec<_> = scanner.collect();
    assert!(results.iter().any(|r| r.is_err()), "expected scan error");
}

#[test]
fn invalid_escape_sequence_errors() {
    // `\.` is not a valid YAML escape (55WF).
    let scanner = Scanner::new("\"\\.\"\n");
    let results: Vec<_> = scanner.collect();
    assert!(results.iter().any(|r| r.is_err()), "expected scan error");
}

#[test]
fn invalid_single_quote_escape_in_double_quoted_errors() {
    // `\'` is not valid in double-quoted scalars (HRE5).
    let scanner = Scanner::new("\"quoted \\' scalar\"\n");
    let results: Vec<_> = scanner.collect();
    assert!(results.iter().any(|r| r.is_err()), "expected scan error");
}

#[test]
fn block_scalar_invalid_header_char_errors() {
    // Indent indicator `0` is invalid (must be 1-9, 2G84).
    let scanner = Scanner::new("--- |0\n");
    let results: Vec<_> = scanner.collect();
    assert!(
        results.iter().any(|r| r.is_err()),
        "expected scan error for |0"
    );
}

#[test]
fn block_scalar_content_on_header_line_errors() {
    // Content on the block scalar header line is invalid (S4GJ).
    let scanner = Scanner::new("folded: > first line\n  second\n");
    let results: Vec<_> = scanner.collect();
    assert!(
        results.iter().any(|r| r.is_err()),
        "expected scan error for > with inline content"
    );
}

#[test]
fn extra_flow_end_bracket_errors() {
    // Extra `]` outside flow context is invalid (4H7K).
    let scanner = Scanner::new("[ a ] ]\n");
    let results: Vec<_> = scanner.collect();
    assert!(
        results.iter().any(|r| r.is_err()),
        "expected error for unmatched ]"
    );
}

#[test]
fn content_after_document_end_errors() {
    // Content after `...` on the same line is invalid (3HFZ).
    let scanner = Scanner::new("key: value\n... invalid\n");
    let results: Vec<_> = scanner.collect();
    assert!(
        results.iter().any(|r| r.is_err()),
        "expected error for content after ..."
    );
}

#[test]
fn comment_without_whitespace_not_consumed() {
    // `#` without preceding whitespace is NOT a comment (YAML §6.4).
    // After `]`, `#invalid` should cause an error, not be silently consumed.
    let scanner = Scanner::new("[ a ]#invalid\n");
    let results: Vec<_> = scanner.collect();
    assert!(
        results.iter().any(|r| r.is_err()),
        "expected error for # without preceding whitespace"
    );
}

// === Anchors and aliases ===

#[test]
fn anchor_on_value() {
    let tokens = scan("a: &anchor value\n");
    let anchor = tokens.iter().find(|t| t.0 == TokenKind::Anchor).unwrap();
    assert_eq!(anchor.1, "anchor");
}

#[test]
fn alias_reference() {
    let tokens = scan("a: *anchor\n");
    let alias = tokens.iter().find(|t| t.0 == TokenKind::Alias).unwrap();
    assert_eq!(alias.1, "anchor");
}

#[test]
fn anchor_and_alias_token_sequence() {
    assert_eq!(
        kinds("a: &x val\nb: *x\n"),
        vec![
            TokenKind::StreamStart,
            TokenKind::BlockMappingStart,
            TokenKind::Key,
            TokenKind::Scalar,
            TokenKind::Value,
            TokenKind::Anchor,
            TokenKind::Scalar,
            TokenKind::Key,
            TokenKind::Scalar,
            TokenKind::Value,
            TokenKind::Alias,
            TokenKind::BlockEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn anchor_in_sequence() {
    let tokens = scan("- &a hello\n- *a\n");
    assert!(
        tokens
            .iter()
            .any(|t| t.0 == TokenKind::Anchor && t.1 == "a")
    );
    assert!(tokens.iter().any(|t| t.0 == TokenKind::Alias && t.1 == "a"));
}

#[test]
fn anchor_as_key() {
    // &anchor on a key node — anchor is emitted, then the scalar
    // triggers simple key resolution. BlockMappingStart + Key appear
    // before the scalar (not before the anchor) with eager resolution.
    let tokens = scan("&a key: value\n");
    assert!(
        tokens
            .iter()
            .any(|t| t.0 == TokenKind::Anchor && t.1 == "a")
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.0 == TokenKind::Scalar && t.1 == "key")
    );
    assert!(tokens.iter().any(|t| t.0 == TokenKind::Key));
}

#[test]
fn anchor_in_flow() {
    let tokens = scan("[&a 1, *a]\n");
    assert!(
        tokens
            .iter()
            .any(|t| t.0 == TokenKind::Anchor && t.1 == "a")
    );
    assert!(tokens.iter().any(|t| t.0 == TokenKind::Alias && t.1 == "a"));
}

// === Tags ===

#[test]
fn secondary_tag() {
    let tokens = scan("!!str hello\n");
    let tag = tokens.iter().find(|t| t.0 == TokenKind::Tag).unwrap();
    assert_eq!(tag.1, "!!str");
}

#[test]
fn primary_tag() {
    let tokens = scan("!local hello\n");
    let tag = tokens.iter().find(|t| t.0 == TokenKind::Tag).unwrap();
    assert_eq!(tag.1, "!local");
}

#[test]
fn non_specific_tag() {
    let tokens = scan("! hello\n");
    let tag = tokens.iter().find(|t| t.0 == TokenKind::Tag).unwrap();
    assert_eq!(tag.1, "!");
}

#[test]
fn verbatim_tag() {
    let tokens = scan("!<tag:yaml.org,2002:str> hello\n");
    let tag = tokens.iter().find(|t| t.0 == TokenKind::Tag).unwrap();
    assert_eq!(tag.1, "!<tag:yaml.org,2002:str>");
}

#[test]
fn named_tag_handle() {
    let tokens = scan("%TAG !e! tag:example.com,\n---\n!e!suffix hello\n");
    let tag = tokens.iter().find(|t| t.0 == TokenKind::Tag).unwrap();
    assert_eq!(tag.1, "!e!suffix");
}

#[test]
fn tag_on_mapping_value() {
    let tokens = scan("key: !!str value\n");
    let k: Vec<_> = tokens.iter().map(|t| t.0).collect();
    assert!(k.contains(&TokenKind::Tag));
    assert!(k.contains(&TokenKind::Scalar));
}

#[test]
fn tag_in_sequence() {
    let tokens = scan("- !!int 42\n");
    assert!(
        tokens
            .iter()
            .any(|t| t.0 == TokenKind::Tag && t.1 == "!!int")
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.0 == TokenKind::Scalar && t.1 == "42")
    );
}

// === Edge cases ===

#[test]
fn flow_mapping_simple_key_resolution() {
    // In flow context, scalars followed by `: ` should produce Key.
    let tokens = scan("{a: 1, b: 2}\n");
    let k: Vec<_> = tokens.iter().map(|t| t.0).collect();
    assert_eq!(
        k,
        vec![
            TokenKind::StreamStart,
            TokenKind::FlowMappingStart,
            TokenKind::Key,
            TokenKind::Scalar,
            TokenKind::Value,
            TokenKind::Scalar,
            TokenKind::FlowEntry,
            TokenKind::Key,
            TokenKind::Scalar,
            TokenKind::Value,
            TokenKind::Scalar,
            TokenKind::FlowMappingEnd,
            TokenKind::StreamEnd,
        ]
    );
}

#[test]
fn flow_mapping_quoted_key() {
    let tokens = scan("{\"key\": val}\n");
    assert!(tokens.iter().any(|t| t.0 == TokenKind::Key));
    assert!(
        tokens
            .iter()
            .any(|t| t.0 == TokenKind::Scalar && t.1 == "key")
    );
}

#[test]
fn multi_line_double_quoted_scalar() {
    // Newline in double-quoted scalar is folded to space.
    let tokens = scan("\"hello\n  world\"\n");
    assert_eq!(tokens[1].1, "hello world");
}

#[test]
fn multi_line_single_quoted_scalar() {
    let tokens = scan("'hello\n  world'\n");
    assert_eq!(tokens[1].1, "hello world");
}

// -- fail:true rejection tests (YAML Test Suite compliance) --

/// Helper: assert that scanning produces an error containing `msg`.
fn assert_scan_error(input: &str, msg: &str) {
    let results: Vec<_> = Scanner::new(input).collect();
    assert!(
        results
            .iter()
            .any(|r| { r.as_ref().is_err_and(|e| e.message.contains(msg)) }),
        "expected scan error containing '{msg}' for input: {input:?}\ngot: {results:?}",
    );
}

#[test]
fn duplicate_yaml_directive_errors() {
    // SF5V: duplicate %YAML in the same prologue.
    assert_scan_error("%YAML 1.2\n%YAML 1.2\n---\n", "duplicate %YAML");
}

#[test]
fn directive_only_stream_errors() {
    // 9MMA: directive without a following document.
    assert_scan_error("%YAML 1.2\n", "directives without a document");
}

#[test]
fn directive_without_document_end_errors() {
    // 9HCY: directive in open implicit document.
    assert_scan_error(
        "!foo \"bar\"\n%TAG ! tag:example.com,2000:app/\n---\n!foo \"bar\"\n",
        "directive without document end marker",
    );
    // EB22: %YAML without preceding `...`. The comment terminates the plain
    // scalar so %YAML reaches the directive check.
    assert_scan_error(
        "---\nscalar1 # comment\n%YAML 1.2\n---\nscalar2\n",
        "directive without document end marker",
    );
    // RHX7: same pattern.
    assert_scan_error(
        "---\nkey: value\n%YAML 1.2\n---\n",
        "directive without document end marker",
    );
}

#[test]
fn directive_after_document_end_is_valid() {
    // Directives are allowed after `...` (back in prologue).
    let tokens = scan("---\nvalue\n...\n%YAML 1.2\n---\nother\n");
    assert!(tokens.iter().any(|t| t.0 == TokenKind::VersionDirective));
}

#[test]
fn block_collection_on_document_start_line_errors() {
    // 9KBC: mapping starting on --- line.
    assert_scan_error("--- key1: value1\n    key2: value2\n", "block collection");
    // CXX2: anchor + mapping on --- line.
    assert_scan_error("--- &anchor a: b\n", "block collection");
}

#[test]
fn scalar_on_document_start_line_is_valid() {
    // `--- value` is valid (scalar on --- line, not a block collection).
    let tokens = scan("--- value\n");
    assert!(tokens.iter().any(|t| t.0 == TokenKind::Scalar));
}

#[test]
fn flow_on_document_start_line_is_valid() {
    // `--- [a, b]` is valid (flow collection on --- line).
    let tokens = scan("--- [a, b]\n");
    assert!(tokens.iter().any(|t| t.0 == TokenKind::FlowSequenceStart));
}

#[test]
fn multiline_flow_key_in_block_errors() {
    // C2SP: flow sequence as implicit key spanning lines.
    assert_scan_error("[23\n]: 42\n", "multiline simple key");
}

#[test]
fn single_line_flow_key_in_block_is_valid() {
    // [a]: value — single-line flow collection as implicit key is fine.
    let tokens = scan("[a]: value\n");
    let kinds: Vec<_> = tokens.iter().map(|t| t.0).collect();
    assert!(kinds.contains(&TokenKind::FlowSequenceStart));
    assert!(kinds.contains(&TokenKind::Value));
}

#[test]
fn undeclared_tag_handle_errors() {
    // QLJ7: tag handle used but not declared in this document.
    assert_scan_error(
        "%TAG !prefix! tag:example.com,2011:\n--- !prefix!A\na: b\n--- !prefix!B\nc: d\n",
        "undeclared tag handle",
    );
}

#[test]
fn declared_tag_handle_is_valid() {
    // Handle declared in the same prologue works.
    let tokens = scan("%TAG !p! tag:example.com,\n---\n!p!foo bar\n");
    let tag = tokens.iter().find(|t| t.0 == TokenKind::Tag).unwrap();
    assert_eq!(tag.1, "!p!foo");
}

#[test]
fn primary_and_secondary_handles_always_valid() {
    // `!suffix` and `!!suffix` don't need %TAG.
    let tokens = scan("!foo bar\n");
    assert!(tokens.iter().any(|t| t.0 == TokenKind::Tag));
    let tokens = scan("!!str bar\n");
    assert!(tokens.iter().any(|t| t.0 == TokenKind::Tag));
}

#[test]
fn extra_content_after_root_scalar_errors() {
    // BS4K: two scalars at root level.
    assert_scan_error(
        "word1  # comment\nword2\n",
        "extra content after document root node",
    );
}

#[test]
fn extra_content_after_flow_collection_errors() {
    // KS4U: content after flow sequence at root.
    assert_scan_error(
        "---\n[\nsequence item\n]\ninvalid item\n",
        "extra content after document root node",
    );
}

#[test]
fn single_root_scalar_is_valid() {
    // One scalar at root is fine.
    let tokens = scan("hello\n");
    assert!(tokens.iter().any(|t| t.0 == TokenKind::Scalar));
}

#[test]
fn root_mapping_is_valid() {
    // Mapping at root (enters block context at indent 0).
    let tokens = scan("key: value\nkey2: value2\n");
    assert!(tokens.iter().any(|t| t.0 == TokenKind::BlockMappingStart));
}

#[test]
fn multi_document_root_nodes_valid() {
    // Each document gets its own root node — no false positive.
    let tokens = scan("scalar1\n---\nscalar2\n---\nscalar3\n");
    let scalar_count = tokens.iter().filter(|t| t.0 == TokenKind::Scalar).count();
    assert_eq!(scalar_count, 3);
}

// -- max_depth enforcement --

#[test]
fn max_depth_rejects_deep_flow_nesting() {
    use yamalgam_core::LoaderConfig;

    let mut config = LoaderConfig::strict();
    config.limits.max_depth = Some(3);

    let input = "[[[[nested]]]]";
    let result: Result<Vec<_>, _> = Scanner::with_config(input, &config).collect();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.message.contains("depth"),
        "expected depth error, got: {err}"
    );
}

#[test]
fn max_depth_allows_within_limit() {
    use yamalgam_core::LoaderConfig;

    let mut config = LoaderConfig::strict();
    config.limits.max_depth = Some(3);

    let input = "[[[ok]]]";
    let result: Result<Vec<_>, _> = Scanner::with_config(input, &config).collect();
    assert!(result.is_ok());
}

#[test]
fn max_depth_rejects_deep_block_nesting() {
    use yamalgam_core::LoaderConfig;

    let mut config = LoaderConfig::strict();
    config.limits.max_depth = Some(2);

    let input = "a:\n  b:\n    c:\n      d: too deep";
    let result: Result<Vec<_>, _> = Scanner::with_config(input, &config).collect();
    assert!(result.is_err());
}

#[test]
fn unchecked_config_allows_deep_nesting() {
    let input = "[[[[[[[[[[deep]]]]]]]]]]";
    // Scanner::new() uses ResourceLimits::none() — no limit
    let result: Result<Vec<_>, _> = Scanner::new(input).collect();
    assert!(result.is_ok());
}

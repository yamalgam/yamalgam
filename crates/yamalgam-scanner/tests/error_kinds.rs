#![allow(missing_docs)]

//! Every [`ScanError`] carries a [`ScanErrorKind`] so callers can match on
//! error families without string-matching the message.

use yamalgam_core::LoaderConfig;
use yamalgam_scanner::scanner::{ScanError, ScanErrorKind, Scanner};

/// Scan `input` until the first error and return it.
fn scan_err(input: &str) -> ScanError {
    Scanner::new(input)
        .find_map(Result::err)
        .unwrap_or_else(|| panic!("input should produce a scan error: {input:?}"))
}

#[track_caller]
fn assert_kind(input: &str, kind: ScanErrorKind) {
    let err = scan_err(input);
    assert_eq!(
        err.kind, kind,
        "input {input:?} produced {:?} ({})",
        err.kind, err.message
    );
}

#[test]
fn tab_indentation_is_invalid_tab() {
    assert_kind("a:\n\tb: c\n", ScanErrorKind::InvalidTab);
}

#[test]
fn quoted_continuation_below_indent_is_invalid_indentation() {
    assert_kind("k: \"a\nb\"\n", ScanErrorKind::InvalidIndentation);
}

#[test]
fn extra_root_content_is_unexpected_content() {
    // The comment ends the root scalar; `word2` cannot continue it (BS4K).
    assert_kind(
        "word1\n# comment\nword2\n",
        ScanErrorKind::UnexpectedContent,
    );
}

#[test]
fn duplicate_yaml_directive_is_invalid_directive() {
    assert_kind(
        "%YAML 1.2\n%YAML 1.2\n---\na\n",
        ScanErrorKind::InvalidDirective,
    );
}

#[test]
fn undeclared_tag_handle_is_invalid_tag() {
    assert_kind("- !e!x a\n", ScanErrorKind::InvalidTag);
}

#[test]
fn close_bracket_outside_flow_is_unexpected_character() {
    assert_kind("a: b\n]\n", ScanErrorKind::UnexpectedCharacter);
}

#[test]
fn multiline_plain_key_is_invalid_simple_key() {
    assert_kind("a\nb: c\n", ScanErrorKind::InvalidSimpleKey);
}

#[test]
fn bad_block_scalar_header_is_invalid_header() {
    assert_kind("|x\na\n", ScanErrorKind::InvalidBlockScalarHeader);
}

#[test]
fn unterminated_single_quote_is_unterminated_scalar() {
    assert_kind("'abc", ScanErrorKind::UnterminatedScalar);
}

#[test]
fn bad_escape_is_invalid_escape() {
    assert_kind("\"\\q\"\n", ScanErrorKind::InvalidEscape);
}

#[test]
fn document_marker_in_quoted_scalar() {
    assert_kind("\"a\n--- b\"\n", ScanErrorKind::DocumentMarkerInScalar);
}

#[test]
fn depth_limit_is_limit_exceeded() {
    let mut config = LoaderConfig::trusted();
    config.limits.max_depth = Some(2);
    let err = Scanner::with_config("[[[[1]]]]\n", &config)
        .find_map(Result::err)
        .expect("depth limit should trip");
    assert_eq!(err.kind, ScanErrorKind::LimitExceeded, "{}", err.message);
}

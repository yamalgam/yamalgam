#![allow(missing_docs)]

//! Every [`ScanError`] carries a [`ScanErrorKind`] so callers can match on
//! error families without string-matching the message.

use std::error::Error as _;

use yamalgam_core::{LimitExceeded, LoaderConfig, Mark, ResourceLimit, Span};
use yamalgam_scanner::scanner::{ScanError, ScanErrorKind, Scanner};

/// Scan `input` until the first error and return it.
fn scan_err(input: &str) -> ScanError {
    Scanner::new(input)
        .find_map(Result::err)
        .unwrap_or_else(|| panic!("input should produce a scan error: {input:?}"))
}

#[track_caller]
fn assert_span(input: &str, kind: ScanErrorKind, span: Span) {
    let err = scan_err(input);
    assert_eq!(err.kind, kind, "unexpected error for {input:?}: {err}");
    assert_eq!(err.span(), Some(span), "unexpected span for {input:?}");
}

const fn span(
    start_offset: usize,
    start_line: u32,
    start_column: u32,
    end_offset: usize,
    end_line: u32,
    end_column: u32,
) -> Span {
    Span {
        start: Mark {
            offset: start_offset,
            line: start_line,
            column: start_column,
        },
        end: Mark {
            offset: end_offset,
            line: end_line,
            column: end_column,
        },
    }
}

#[test]
fn tab_indentation_is_invalid_tab() {
    assert_span(
        "a:\n\tb: c\n",
        ScanErrorKind::InvalidTab,
        span(3, 1, 0, 4, 1, 1),
    );
}

#[test]
fn quoted_continuation_below_indent_is_invalid_indentation() {
    assert_span(
        "k: \"a\nb\"\n",
        ScanErrorKind::InvalidIndentation,
        span(6, 1, 0, 7, 1, 1),
    );
}

#[test]
fn extra_root_content_is_unexpected_content() {
    // The comment ends the root scalar; `word2` cannot continue it (BS4K).
    assert_span(
        "word1\n# comment\nword2\n",
        ScanErrorKind::UnexpectedContent,
        span(16, 2, 0, 17, 2, 1),
    );
}

#[test]
fn duplicate_yaml_directive_is_invalid_directive() {
    assert_span(
        "%YAML 1.2\n%YAML 1.2\n---\na\n",
        ScanErrorKind::InvalidDirective,
        span(10, 1, 0, 11, 1, 1),
    );
}

#[test]
fn undeclared_tag_handle_is_invalid_tag() {
    assert_span(
        "- !e!x a\n",
        ScanErrorKind::InvalidTag,
        span(2, 0, 2, 6, 0, 6),
    );
}

#[test]
fn close_bracket_outside_flow_is_unexpected_character() {
    assert_span(
        "a: b\n]\n",
        ScanErrorKind::UnexpectedCharacter,
        span(5, 1, 0, 6, 1, 1),
    );
}

#[test]
fn multiline_plain_key_is_invalid_simple_key() {
    assert_span(
        "a\nb: c\n",
        ScanErrorKind::InvalidSimpleKey,
        span(0, 0, 0, 3, 1, 1),
    );
}

#[test]
fn bad_block_scalar_header_is_invalid_header() {
    assert_span(
        "|x\na\n",
        ScanErrorKind::InvalidBlockScalarHeader,
        span(1, 0, 1, 2, 0, 2),
    );
}

#[test]
fn unterminated_single_quote_is_unterminated_scalar() {
    assert_span(
        "'abc",
        ScanErrorKind::UnterminatedScalar,
        span(0, 0, 0, 4, 0, 4),
    );
}

#[test]
fn bad_escape_is_invalid_escape() {
    assert_span(
        "\"\\q\"\n",
        ScanErrorKind::InvalidEscape,
        span(1, 0, 1, 3, 0, 3),
    );
}

#[test]
fn document_marker_in_quoted_scalar() {
    assert_span(
        "\"a\n--- b\"\n",
        ScanErrorKind::DocumentMarkerInScalar,
        span(3, 1, 0, 6, 1, 3),
    );
}

#[test]
fn depth_limit_is_limit_exceeded() {
    let mut config = LoaderConfig::trusted();
    config.limits.max_depth = Some(2);
    let err = Scanner::with_config("[[[[1]]]]\n", &config)
        .find_map(Result::err)
        .expect("depth limit should trip");
    let ScanErrorKind::LimitExceeded(limit) = &err.kind else {
        panic!("expected a structured limit error, got {err:?}");
    };
    assert_eq!(limit.limit, ResourceLimit::Depth);
    assert_eq!(limit.observed, 3);
    assert_eq!(limit.maximum, 2);
    assert_eq!(err.span(), Some(span(2, 0, 2, 3, 0, 3)));
    assert_eq!(
        err.source()
            .and_then(|source| source.downcast_ref::<LimitExceeded>()),
        Some(limit)
    );
}

#[test]
fn source_free_accounting_overflow_has_no_span() {
    let err = ScanError::new(
        ScanErrorKind::ResourceOverflow("test accounting"),
        "test accounting overflow",
    );
    assert_eq!(err.span(), None);
}

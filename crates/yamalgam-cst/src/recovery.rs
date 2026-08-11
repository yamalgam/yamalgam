//! Multi-error CST recovery for editor and language-server workloads.

use std::borrow::Cow;

use yamalgam_core::{LoaderConfig, Mark, Span};
use yamalgam_parser::ParseError;

use crate::{CstElement, CstNode, CstToken, NodeKind, TokenKind, parse_to_cst_report_with_config};

/// Bounds for structured CST recovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecoveryOptions {
    /// Maximum number of parser diagnostics retained before the remainder is
    /// wrapped in one error node.
    pub max_diagnostics: usize,
}

impl Default for RecoveryOptions {
    fn default() -> Self {
        Self {
            max_diagnostics: 64,
        }
    }
}

/// A lossless CST together with all diagnostics found during bounded recovery.
#[derive(Clone, Debug)]
pub struct RecoveryReport<'input> {
    /// Recovered concrete syntax tree.
    pub cst: CstNode<'input>,
    /// Structured parser errors in source order.
    pub diagnostics: Vec<ParseError>,
}

/// Parse YAML into a lossless CST while recovering after multiple errors.
pub fn parse_to_cst_recovering_with_config<'input>(
    input: &'input str,
    config: &LoaderConfig,
    options: RecoveryOptions,
) -> RecoveryReport<'input> {
    let mut offset = 0;
    let mut children = Vec::new();
    let mut diagnostics = Vec::new();

    while offset < input.len() {
        if diagnostics.len() >= options.max_diagnostics {
            children.push(error_element(input, offset));
            break;
        }

        let remaining = &input[offset..];
        let probe = parse_to_cst_report_with_config(remaining, config);
        let Some(first_error) = probe.diagnostics.into_iter().next() else {
            append_shifted_children(&mut children, probe.cst, offset);
            offset = input.len();
            break;
        };

        let boundary = recovery_boundary(remaining, first_error.span())
            .max(next_scalar_boundary(remaining))
            .min(remaining.len());
        let chunk = &remaining[..boundary];
        let mut chunk_report = parse_to_cst_report_with_config(chunk, config);
        let diagnostic = chunk_report
            .diagnostics
            .drain(..)
            .next()
            .unwrap_or(first_error);
        diagnostics.push(shift_error(diagnostic, mark_at_offset(input, offset)));
        append_shifted_children(&mut children, chunk_report.cst, offset);
        offset += boundary;
    }

    if input.is_empty() {
        offset = 0;
    }

    let mut cst = CstNode {
        kind: NodeKind::Stream,
        span: Span {
            start: Mark::default(),
            end: mark_at_offset(input, offset),
        },
        children,
    };
    normalize_spans(&mut cst, input);
    RecoveryReport { cst, diagnostics }
}

fn append_shifted_children<'input>(
    target: &mut Vec<CstElement<'input>>,
    mut root: CstNode<'input>,
    base_offset: usize,
) {
    let mut children = std::mem::take(&mut root.children);
    for child in &mut children {
        shift_element_offsets(child, base_offset);
    }
    target.extend(children);
}

fn shift_element_offsets(element: &mut CstElement<'_>, base_offset: usize) {
    match element {
        CstElement::Node(node) => {
            node.span.start.offset += base_offset;
            node.span.end.offset += base_offset;
            for child in &mut node.children {
                shift_element_offsets(child, base_offset);
            }
        }
        CstElement::Token(token) => {
            token.span.start.offset += base_offset;
            token.span.end.offset += base_offset;
        }
    }
}

fn normalize_spans(node: &mut CstNode<'_>, input: &str) {
    node.span.start = mark_at_offset(input, node.span.start.offset);
    node.span.end = mark_at_offset(input, node.span.end.offset);
    for child in &mut node.children {
        match child {
            CstElement::Node(child) => normalize_spans(child, input),
            CstElement::Token(token) => {
                token.span.start = mark_at_offset(input, token.span.start.offset);
                token.span.end = mark_at_offset(input, token.span.end.offset);
            }
        }
    }
}

fn error_element(input: &str, offset: usize) -> CstElement<'_> {
    let span = Span {
        start: mark_at_offset(input, offset),
        end: mark_at_offset(input, input.len()),
    };
    CstElement::Node(CstNode {
        kind: NodeKind::Error,
        span,
        children: vec![CstElement::Token(CstToken {
            kind: TokenKind::Whitespace,
            text: Cow::Borrowed(&input[offset..]),
            span,
        })],
    })
}

fn recovery_boundary(input: &str, span: Option<Span>) -> usize {
    let failure = span.unwrap_or_default();
    let start = failure.start.offset.min(input.len());
    let mut candidate = next_line_start(input, line_start(input, start));

    while candidate < input.len() {
        let end = line_content_end(input, candidate);
        let line = &input[candidate..end];
        if line.starts_with("---") || line.starts_with("...") || line.starts_with('%') {
            return candidate;
        }

        let indent = line
            .chars()
            .take_while(|character| *character == ' ')
            .count() as u32;
        if !line.trim().is_empty() && indent <= failure.start.column {
            return candidate;
        }
        candidate = next_line_start(input, candidate);
    }

    input.len()
}

fn next_scalar_boundary(input: &str) -> usize {
    input.chars().next().map_or(0, char::len_utf8)
}

fn line_start(input: &str, offset: usize) -> usize {
    input[..offset]
        .rfind(['\n', '\r'])
        .map_or(0, |position| position + 1)
}

fn line_content_end(input: &str, start: usize) -> usize {
    input[start..]
        .find(['\n', '\r'])
        .map_or(input.len(), |relative| start + relative)
}

fn next_line_start(input: &str, start: usize) -> usize {
    let end = line_content_end(input, start);
    match input.as_bytes().get(end) {
        Some(b'\r') if input.as_bytes().get(end + 1) == Some(&b'\n') => end + 2,
        Some(b'\r' | b'\n') => end + 1,
        _ => input.len(),
    }
}

fn mark_at_offset(input: &str, offset: usize) -> Mark {
    let mut mark = Mark::default();
    let mut characters = input[..offset].chars().peekable();
    while let Some(character) = characters.next() {
        mark.offset += character.len_utf8();
        if character == '\r' {
            if characters.peek() == Some(&'\n') {
                mark.offset += 1;
                characters.next();
            }
            mark.line += 1;
            mark.column = 0;
        } else if character == '\n' {
            mark.line += 1;
            mark.column = 0;
        } else {
            mark.column += 1;
        }
    }
    mark
}

const fn shift_mark(mark: Mark, base: Mark) -> Mark {
    Mark {
        offset: base.offset + mark.offset,
        line: base.line + mark.line,
        column: if mark.line == 0 {
            base.column + mark.column
        } else {
            mark.column
        },
    }
}

const fn shift_span(span: Span, base: Mark) -> Span {
    Span {
        start: shift_mark(span.start, base),
        end: shift_mark(span.end, base),
    }
}

fn shift_error(error: ParseError, base: Mark) -> ParseError {
    match error {
        ParseError::Scan(error) => ParseError::Scan(match error.span() {
            Some(span) => error.with_span(shift_span(span, base)),
            None => error,
        }),
        ParseError::UnexpectedToken {
            expected,
            got,
            span,
        } => ParseError::UnexpectedToken {
            expected,
            got,
            span: shift_span(span, base),
        },
        ParseError::UnexpectedEof { expected, span } => ParseError::UnexpectedEof {
            expected,
            span: shift_span(span, base),
        },
        ParseError::DuplicateVersionDirective { span } => ParseError::DuplicateVersionDirective {
            span: shift_span(span, base),
        },
        ParseError::DuplicateTagDirective { handle, span } => ParseError::DuplicateTagDirective {
            handle,
            span: shift_span(span, base),
        },
        ParseError::UndefinedTagPrefix { prefix, span } => ParseError::UndefinedTagPrefix {
            prefix,
            span: shift_span(span, base),
        },
        ParseError::LimitExceeded(error) => ParseError::LimitExceeded(error),
        ParseError::ResourceOverflow(context) => ParseError::ResourceOverflow(context),
        ParseError::UnsupportedResolutionPolicy { include, refs } => {
            ParseError::UnsupportedResolutionPolicy { include, refs }
        }
        other => other,
    }
}

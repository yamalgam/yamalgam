//! Harness for running yamalgam's scanner and parser, converting output to
//! implementation-neutral snapshots for comparison.

use crate::event_snapshot::EventSnapshot;
use crate::snapshot::{SpanSnapshot, TokenSnapshot};

/// Run the Rust scanner on input bytes.
///
/// Decodes the input, runs the scanner, and converts each
/// [`yamalgam_scanner::Token`] to a [`TokenSnapshot`].
///
/// # Errors
///
/// Returns an error string on encoding failures or scan errors.
pub fn run_rust_scanner(input: &[u8]) -> Result<Vec<TokenSnapshot>, String> {
    let decoded = yamalgam_scanner::input::Input::from_bytes(input)
        .map_err(|diag| format!("input decode error: {}", diag.message))?;

    let scanner = yamalgam_scanner::scanner::Scanner::new(decoded.as_str());
    let mut tokens = Vec::new();

    for result in scanner {
        let token = result.map_err(|e| e.to_string())?;
        tokens.push(token_to_snapshot(&token));
    }

    Ok(tokens)
}

/// Convert a Rust scanner token to an implementation-neutral snapshot.
fn token_to_snapshot(token: &yamalgam_scanner::Token<'_>) -> TokenSnapshot {
    let kind = format!("{:?}", token.kind);
    let value = if token.kind == yamalgam_scanner::TokenKind::Scalar || !token.atom.data.is_empty()
    {
        Some(token.atom.data.to_string())
    } else {
        None
    };
    TokenSnapshot {
        kind,
        value,
        style: None,
        span: SpanSnapshot {
            line: token.atom.span.start.line,
            column: token.atom.span.start.column,
            offset: token.atom.span.start.offset,
            end_line: token.atom.span.end.line,
            end_column: token.atom.span.end.column,
            end_offset: token.atom.span.end.offset,
        },
    }
}

/// Run the Rust parser on input bytes.
///
/// Creates a [`yamalgam_parser::Parser`], collects events, and converts each
/// to an [`EventSnapshot`]. `VersionDirective` and `TagDirective` events are
/// filtered out since the YAML Test Suite tree format does not include them.
///
/// # Errors
///
/// Returns an error string on parse errors.
pub fn run_rust_parser(input: &[u8]) -> Result<Vec<EventSnapshot>, String> {
    let text = std::str::from_utf8(input).map_err(|e| format!("input is not valid UTF-8: {e}"))?;

    let parser = yamalgam_parser::Parser::new(text);
    let mut events = Vec::new();

    for result in parser {
        let event = result.map_err(|e| e.to_string())?;
        if let Some(snapshot) = event_to_snapshot(&event) {
            events.push(snapshot);
        }
    }

    Ok(events)
}

/// Convert a Rust parser event to an implementation-neutral snapshot.
///
/// Returns `None` for `VersionDirective` and `TagDirective` events, which
/// have no counterpart in the YAML Test Suite tree format.
fn event_to_snapshot(event: &yamalgam_parser::Event<'_>) -> Option<EventSnapshot> {
    match event {
        yamalgam_parser::Event::StreamStart => Some(EventSnapshot {
            kind: "StreamStart".to_string(),
            anchor: None,
            tag: None,
            value: None,
            implicit: None,
        }),
        yamalgam_parser::Event::StreamEnd => Some(EventSnapshot {
            kind: "StreamEnd".to_string(),
            anchor: None,
            tag: None,
            value: None,
            implicit: None,
        }),
        yamalgam_parser::Event::VersionDirective { .. } => None,
        yamalgam_parser::Event::TagDirective { .. } => None,
        yamalgam_parser::Event::DocumentStart { implicit, .. } => Some(EventSnapshot {
            kind: "DocumentStart".to_string(),
            anchor: None,
            tag: None,
            value: None,
            implicit: Some(*implicit),
        }),
        yamalgam_parser::Event::DocumentEnd { implicit, .. } => Some(EventSnapshot {
            kind: "DocumentEnd".to_string(),
            anchor: None,
            tag: None,
            value: None,
            implicit: Some(*implicit),
        }),
        yamalgam_parser::Event::SequenceStart { anchor, tag, .. } => Some(EventSnapshot {
            kind: "SequenceStart".to_string(),
            anchor: anchor.as_ref().map(|a| a.to_string()),
            tag: tag.as_ref().map(|t| t.to_string()),
            value: None,
            implicit: None,
        }),
        yamalgam_parser::Event::SequenceEnd { .. } => Some(EventSnapshot {
            kind: "SequenceEnd".to_string(),
            anchor: None,
            tag: None,
            value: None,
            implicit: None,
        }),
        yamalgam_parser::Event::MappingStart { anchor, tag, .. } => Some(EventSnapshot {
            kind: "MappingStart".to_string(),
            anchor: anchor.as_ref().map(|a| a.to_string()),
            tag: tag.as_ref().map(|t| t.to_string()),
            value: None,
            implicit: None,
        }),
        yamalgam_parser::Event::MappingEnd { .. } => Some(EventSnapshot {
            kind: "MappingEnd".to_string(),
            anchor: None,
            tag: None,
            value: None,
            implicit: None,
        }),
        yamalgam_parser::Event::Scalar {
            anchor, tag, value, ..
        } => Some(EventSnapshot {
            kind: "Scalar".to_string(),
            anchor: anchor.as_ref().map(|a| a.to_string()),
            tag: tag.as_ref().map(|t| t.to_string()),
            value: Some(value.to_string()),
            implicit: None,
        }),
        yamalgam_parser::Event::Alias { name, .. } => Some(EventSnapshot {
            kind: "Alias".to_string(),
            anchor: None,
            tag: None,
            value: Some(name.to_string()),
            implicit: None,
        }),
        // yamalgam-specific structural events — not in YAML Test Suite tree format.
        yamalgam_parser::Event::Comment { .. }
        | yamalgam_parser::Event::BlockEntry { .. }
        | yamalgam_parser::Event::KeyIndicator { .. }
        | yamalgam_parser::Event::ValueIndicator { .. } => None,
    }
}

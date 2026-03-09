//! Token and event stream comparison logic.

use serde::{Deserialize, Serialize};

use crate::event_snapshot::EventSnapshot;
use crate::snapshot::TokenSnapshot;

/// Number of preceding tokens to include as context in a mismatch report.
const CONTEXT_WINDOW: usize = 5;

/// Result of comparing token streams from two implementations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompareResult {
    /// Token streams are identical.
    Match {
        /// Number of tokens that matched.
        token_count: usize,
    },
    /// Both produced errors with matching messages.
    BothErrorMatch,
    /// Both errored but with different messages/locations.
    BothErrorMismatch {
        /// Error from the C harness.
        c_error: String,
        /// Error from the Rust scanner.
        rust_error: String,
    },
    /// C succeeded, Rust errored.
    CSuccessRustError {
        /// Error from the Rust scanner.
        rust_error: String,
        /// Number of tokens the C harness produced.
        c_token_count: usize,
    },
    /// Rust succeeded, C errored.
    RustSuccessCError {
        /// Error from the C harness.
        c_error: String,
        /// Number of tokens the Rust scanner produced.
        rust_token_count: usize,
    },
    /// Token streams diverged at a specific point.
    TokenMismatch {
        /// Zero-based index where the streams first diverge.
        index: usize,
        /// The C harness token at the divergence point.
        c_token: TokenSnapshot,
        /// The Rust scanner token at the divergence point.
        rust_token: TokenSnapshot,
        /// Preceding tokens (up to [`CONTEXT_WINDOW`]) for debugging context.
        context: Vec<TokenSnapshot>,
    },
}

/// Compare two token streams element by element.
///
/// Walks both streams in lockstep. On the first token that differs (by kind,
/// value, or style — spans are intentionally excluded from the comparison),
/// returns [`CompareResult::TokenMismatch`] with up to 5 preceding tokens for
/// context. If the streams have different lengths, the divergence is reported
/// at the index where one stream ends.
pub fn compare_token_streams(
    c_tokens: &[TokenSnapshot],
    rust_tokens: &[TokenSnapshot],
) -> CompareResult {
    let common_len = c_tokens.len().min(rust_tokens.len());

    for i in 0..common_len {
        if !tokens_match(&c_tokens[i], &rust_tokens[i]) {
            return CompareResult::TokenMismatch {
                index: i,
                c_token: c_tokens[i].clone(),
                rust_token: rust_tokens[i].clone(),
                context: context_slice(c_tokens, i),
            };
        }
    }

    // If lengths differ, report mismatch at the end of the shorter stream.
    if c_tokens.len() != rust_tokens.len() {
        let i = common_len;
        // Build synthetic tokens for the "missing" side.
        let (c_tok, rust_tok) = if i < c_tokens.len() {
            (c_tokens[i].clone(), eof_sentinel())
        } else {
            (eof_sentinel(), rust_tokens[i].clone())
        };
        return CompareResult::TokenMismatch {
            index: i,
            c_token: c_tok,
            rust_token: rust_tok,
            context: context_slice(c_tokens, i),
        };
    }

    CompareResult::Match {
        token_count: c_tokens.len(),
    }
}

/// Compare two tokens, ignoring span information.
///
/// Two tokens match when their kind, value, and style all agree.
fn tokens_match(a: &TokenSnapshot, b: &TokenSnapshot) -> bool {
    a.kind == b.kind && a.value == b.value && a.style == b.style
}

/// Extract up to [`CONTEXT_WINDOW`] tokens preceding `index`.
fn context_slice(tokens: &[TokenSnapshot], index: usize) -> Vec<TokenSnapshot> {
    let start = index.saturating_sub(CONTEXT_WINDOW);
    tokens[start..index].to_vec()
}

/// Synthetic token representing the end of a shorter stream.
fn eof_sentinel() -> TokenSnapshot {
    TokenSnapshot {
        kind: "<END_OF_STREAM>".to_string(),
        value: None,
        style: None,
        span: crate::snapshot::SpanSnapshot::default(),
    }
}

// ---------------------------------------------------------------------------
// Event stream comparison
// ---------------------------------------------------------------------------

/// Number of preceding events to include as context in a mismatch report.
const EVENT_CONTEXT_WINDOW: usize = 5;

/// Result of comparing event streams from two implementations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompareEventResult {
    /// Event streams are identical.
    Match {
        /// Number of events that matched.
        event_count: usize,
    },
    /// Both produced errors with matching messages.
    BothErrorMatch,
    /// Both errored but with different messages/locations.
    BothErrorMismatch {
        /// Error from the C harness.
        c_error: String,
        /// Error from the Rust parser.
        rust_error: String,
    },
    /// C succeeded, Rust errored.
    CSuccessRustError {
        /// Error from the Rust parser.
        rust_error: String,
        /// Number of events the C harness produced.
        c_event_count: usize,
    },
    /// Rust succeeded, C errored.
    RustSuccessCError {
        /// Error from the C harness.
        c_error: String,
        /// Number of events the Rust parser produced.
        rust_event_count: usize,
    },
    /// Event streams diverged at a specific point.
    EventMismatch {
        /// Zero-based index where the streams first diverge.
        index: usize,
        /// The C harness event at the divergence point.
        c_event: EventSnapshot,
        /// The Rust parser event at the divergence point.
        rust_event: EventSnapshot,
        /// Preceding events (up to [`EVENT_CONTEXT_WINDOW`]) for debugging context.
        context: Vec<EventSnapshot>,
    },
}

/// Compare two event streams element by element.
///
/// Walks both streams in lockstep. On the first event that differs,
/// returns [`CompareEventResult::EventMismatch`] with context.
///
/// Comparison rules:
/// - `kind` must match exactly
/// - `value` must match exactly
/// - `anchor` must match exactly
/// - `implicit` must match exactly
/// - `tag` is **skipped** — implementations may resolve tags to full URIs or keep shorthand forms
pub fn compare_event_streams(
    c_events: &[EventSnapshot],
    rust_events: &[EventSnapshot],
) -> CompareEventResult {
    let common_len = c_events.len().min(rust_events.len());

    for i in 0..common_len {
        if !events_match(&c_events[i], &rust_events[i]) {
            return CompareEventResult::EventMismatch {
                index: i,
                c_event: c_events[i].clone(),
                rust_event: rust_events[i].clone(),
                context: event_context_slice(c_events, i),
            };
        }
    }

    // If lengths differ, report mismatch at the end of the shorter stream.
    if c_events.len() != rust_events.len() {
        let i = common_len;
        let (c_evt, rust_evt) = if i < c_events.len() {
            (c_events[i].clone(), eof_event_sentinel())
        } else {
            (eof_event_sentinel(), rust_events[i].clone())
        };
        return CompareEventResult::EventMismatch {
            index: i,
            c_event: c_evt,
            rust_event: rust_evt,
            context: event_context_slice(c_events, i),
        };
    }

    CompareEventResult::Match {
        event_count: c_events.len(),
    }
}

/// Compare two events, skipping tag comparison.
///
/// Two events match when their kind, value, anchor, and implicit all agree.
/// Tags are intentionally skipped because implementations may resolve tags to
/// full URIs or keep the shorthand form.
fn events_match(a: &EventSnapshot, b: &EventSnapshot) -> bool {
    a.kind == b.kind && a.value == b.value && a.anchor == b.anchor && a.implicit == b.implicit
}

/// Compare two events including tags (strict mode).
///
/// Use when both sides produce tags in the same format (e.g., both shorthand
/// or both resolved URIs).
fn events_match_strict(a: &EventSnapshot, b: &EventSnapshot) -> bool {
    a.kind == b.kind
        && a.value == b.value
        && a.anchor == b.anchor
        && a.implicit == b.implicit
        && a.tag == b.tag
}

/// Compare event streams with tag comparison enabled.
///
/// Identical to [`compare_event_streams`] but includes tag fields in the
/// equality check. Use when both sides emit tags in the same format.
pub fn compare_event_streams_with_tags(
    c_events: &[EventSnapshot],
    rust_events: &[EventSnapshot],
) -> CompareEventResult {
    let common_len = c_events.len().min(rust_events.len());

    for i in 0..common_len {
        if !events_match_strict(&c_events[i], &rust_events[i]) {
            return CompareEventResult::EventMismatch {
                index: i,
                c_event: c_events[i].clone(),
                rust_event: rust_events[i].clone(),
                context: event_context_slice(c_events, i),
            };
        }
    }

    if c_events.len() != rust_events.len() {
        let i = common_len;
        let (c_evt, rust_evt) = if i < c_events.len() {
            (c_events[i].clone(), eof_event_sentinel())
        } else {
            (eof_event_sentinel(), rust_events[i].clone())
        };
        return CompareEventResult::EventMismatch {
            index: i,
            c_event: c_evt,
            rust_event: rust_evt,
            context: event_context_slice(c_events, i),
        };
    }

    CompareEventResult::Match {
        event_count: c_events.len(),
    }
}

/// Extract up to [`EVENT_CONTEXT_WINDOW`] events preceding `index`.
fn event_context_slice(events: &[EventSnapshot], index: usize) -> Vec<EventSnapshot> {
    let start = index.saturating_sub(EVENT_CONTEXT_WINDOW);
    events[start..index].to_vec()
}

/// Synthetic event representing the end of a shorter stream.
fn eof_event_sentinel() -> EventSnapshot {
    EventSnapshot {
        kind: "<END_OF_STREAM>".to_string(),
        anchor: None,
        tag: None,
        value: None,
        implicit: None,
    }
}

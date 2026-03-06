//! Token stream comparison logic.

use serde::{Deserialize, Serialize};

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

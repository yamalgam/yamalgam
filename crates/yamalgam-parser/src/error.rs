//! Parser error types.

use yamalgam_core::Span;
use yamalgam_scanner::TokenKind;
use yamalgam_scanner::scanner::ScanError;

/// Errors produced by the YAML parser.
#[derive(Clone, Debug, thiserror::Error)]
pub enum ParseError {
    /// A scanner-level error propagated upward.
    #[error(transparent)]
    Scan(#[from] ScanError),

    /// Received a token that doesn't match the current parser state.
    #[error("unexpected {got:?}, expected {expected}")]
    UnexpectedToken {
        /// What the parser expected at this point.
        expected: &'static str,
        /// The token kind that was actually found.
        got: TokenKind,
        /// Source span of the unexpected token.
        span: Span,
    },

    /// Hit the end of input when more tokens were expected.
    #[error("unexpected end of input, expected {expected}")]
    UnexpectedEof {
        /// What the parser expected at this point.
        expected: &'static str,
        /// Span at the position where EOF was encountered.
        span: Span,
    },

    /// A second `%YAML` directive in the same document.
    #[error("duplicate %YAML directive")]
    DuplicateVersionDirective {
        /// Source span of the duplicate directive.
        span: Span,
    },

    /// A second `%TAG` directive with the same handle in one document.
    #[error("duplicate %TAG directive for handle {handle:?}")]
    DuplicateTagDirective {
        /// The duplicated tag handle.
        handle: String,
        /// Source span of the duplicate directive.
        span: Span,
    },

    /// A tag references a prefix not declared by any `%TAG` directive.
    #[error("tag prefix {prefix:?} is not defined")]
    UndefinedTagPrefix {
        /// The undefined prefix.
        prefix: String,
        /// Source span of the tag reference.
        span: Span,
    },

    /// A resource limit was exceeded (depth, size, etc.).
    #[error("limit exceeded: {0}")]
    LimitExceeded(String),
}

//! Diagnostic types shared across all yamalgam crates.
//!
//! Designed for compatibility with `miette` for terminal rendering.

use serde::{Deserialize, Serialize};

/// Severity level for a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    /// Informational hint.
    Hint,
    /// Informational message.
    Info,
    /// Non-fatal warning.
    Warning,
    /// Fatal error.
    Error,
}

/// A source position in the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Mark {
    /// Zero-indexed line number.
    pub line: u32,
    /// Zero-indexed column number (in bytes).
    pub column: u32,
    /// Byte offset from start of input.
    pub offset: usize,
}

/// A source range (start..end) in the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Span {
    /// Start position (inclusive).
    pub start: Mark,
    /// End position (exclusive).
    pub end: Mark,
}

/// A labeled region of source code within a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    /// The source range this label covers.
    pub span: Span,
    /// Human-readable message for this label.
    pub message: String,
}

/// A diagnostic message with source location and severity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Severity of this diagnostic.
    pub severity: Severity,
    /// Machine-readable error code (e.g., "E001").
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Primary source location (if applicable).
    pub span: Option<Span>,
    /// Additional labeled spans for context.
    pub labels: Vec<Label>,
}

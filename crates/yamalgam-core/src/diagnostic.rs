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
    /// Zero-indexed column number (in characters).
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

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let severity = match self.severity {
            Severity::Hint => "hint",
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        write!(f, "{severity}[{}]: {}", self.code, self.message)?;
        if let Some(span) = &self.span {
            // Marks are zero-indexed; render one-indexed for humans.
            write!(
                f,
                " (line {}, column {})",
                span.start.line + 1,
                span.start.column + 1
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for Diagnostic {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_with_span_uses_one_indexed_position() {
        let d = Diagnostic {
            severity: Severity::Error,
            code: "E001".to_string(),
            message: "unexpected character".to_string(),
            span: Some(Span {
                start: Mark {
                    line: 3,
                    column: 6,
                    offset: 42,
                },
                end: Mark::default(),
            }),
            labels: Vec::new(),
        };
        assert_eq!(
            d.to_string(),
            "error[E001]: unexpected character (line 4, column 7)"
        );
    }

    #[test]
    fn display_without_span_omits_position() {
        let d = Diagnostic {
            severity: Severity::Warning,
            code: "W010".to_string(),
            message: "tab used for indentation".to_string(),
            span: None,
            labels: Vec::new(),
        };
        assert_eq!(d.to_string(), "warning[W010]: tab used for indentation");
    }

    #[test]
    fn diagnostic_implements_std_error() {
        fn assert_impl<T: std::error::Error>() {}
        assert_impl::<Diagnostic>();
    }
}

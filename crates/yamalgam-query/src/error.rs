//! Error types for the query engine.

use yamalgam_core::Span;

/// Error building a YAML value tree from parser events.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// Underlying YAML parse error.
    #[error(transparent)]
    Parse(#[from] yamalgam_parser::ParseError),
    /// Alias references an anchor that has not been defined.
    #[error(
        "unknown alias *{name} (line {}, column {})",
        .span.start.line + 1,
        .span.start.column + 1
    )]
    UnknownAlias {
        /// The alias name.
        name: String,
        /// Location of the alias.
        span: Span,
    },
    /// A loader resource limit was exceeded.
    #[error("{0}")]
    Limit(String),
}

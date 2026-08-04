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

/// Query-engine errors partitioned by their CLI exit-code class.
#[derive(Debug, thiserror::Error)]
pub enum YgError {
    /// Filter lexing or parsing failed.
    #[error("{0}")]
    FilterSyntax(String),
    /// Filter compilation failed, such as for an undefined name.
    #[error("{0}")]
    FilterCompile(String),
    /// An input document could not be read or parsed.
    #[error("{0}")]
    InputParse(String),
    /// An input file was unavailable.
    #[error("{0}")]
    InputRead(String),
    /// The filter failed while running.
    #[error("{0}")]
    Runtime(String),
    /// The filter explicitly requested `halt(code)`.
    #[error("halt")]
    Halt(i32),
    /// The output consumer closed its pipe normally.
    #[error("broken pipe")]
    BrokenPipe,
}

impl YgError {
    pub(crate) fn runtime(exception: jaq_core::Exn<'_, crate::YamlVal>) -> Self {
        match exception.get_err() {
            Ok(error) => Self::Runtime(error.to_string()),
            Err(exception) => exception.get_halt().map_or_else(
                |_| Self::Runtime("unknown query exception".into()),
                Self::Halt,
            ),
        }
    }
}

impl From<crate::emit::EmitError> for YgError {
    fn from(error: crate::emit::EmitError) -> Self {
        Self::Runtime(error.to_string())
    }
}

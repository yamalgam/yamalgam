//! Error types for the query engine.

use std::sync::Arc;

use yamalgam_core::{LimitExceeded, Span};
use yamalgam_resolve::ResolutionReport;

/// Error building a YAML value tree from parser events.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BuildError {
    /// Underlying YAML parse error.
    #[error(transparent)]
    Parse(#[from] yamalgam_parser::ParseError),
    /// Event resolver middleware failed.
    #[error(transparent)]
    Resolve(yamalgam_parser::ResolveError),
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
    /// The parser event stream violated a builder invariant.
    #[error("parser/builder invariant violated: {0}")]
    Invariant(String),
    /// A loader resource limit was exceeded.
    #[error("{0}")]
    Limit(#[source] LimitExceeded),
    /// Resource-accounting arithmetic overflowed before a configured limit
    /// could be checked.
    #[error("{0} overflow")]
    ResourceOverflow(&'static str),
}

impl From<yamalgam_parser::ResolveError> for BuildError {
    fn from(error: yamalgam_parser::ResolveError) -> Self {
        match error {
            yamalgam_parser::ResolveError::Parse(error) => Self::Parse(error),
            other => Self::Resolve(other),
        }
    }
}

/// Query-engine errors partitioned by their CLI exit-code class.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum YgError {
    /// Command-line arguments were invalid.
    #[error("{0}")]
    Usage(String),
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
    /// An input document exceeded a configured resource limit.
    #[error("{message}")]
    InputLimit {
        /// Structured resource-limit failure.
        #[source]
        error: LimitExceeded,
        /// Original rendered diagnostic, including any parser or scanner
        /// prefix.
        message: String,
    },
    /// Automatic external resolution failed while loading an input document.
    #[error("{message}")]
    InputResolution {
        /// Structured resolver failure.
        #[source]
        error: Box<yamalgam_parser::ResolveError>,
        /// Reports completed before and during the failed document.
        reports: Vec<ResolutionReport>,
        /// Rendered diagnostic, including input context when available.
        message: String,
    },
    /// Converting a successfully resolved document into a query value failed.
    #[error("{message}")]
    InputConversion {
        /// Structured conversion failure.
        #[source]
        error: Box<Self>,
        /// Reports completed before and during the failed conversion.
        reports: Vec<ResolutionReport>,
        /// Rendered diagnostic, including input context when available.
        message: String,
    },
    /// The filter failed while running.
    #[error("{0}")]
    Runtime(String),
    /// Query execution or emission exceeded a configured resource limit.
    #[error("{message}")]
    RuntimeLimit {
        /// Structured resource-limit failure.
        #[source]
        error: LimitExceeded,
        /// Rendered diagnostic, including input context when available.
        message: String,
    },
    /// An explicit `include` or `ref` filter failed.
    #[error("{message}")]
    RuntimeResolution {
        /// Structured resolver failure.
        #[source]
        error: Arc<yamalgam_parser::ResolveError>,
        /// Rendered diagnostic, including input context when available.
        message: String,
    },
    /// Resource-accounting arithmetic overflowed before a configured limit
    /// could be checked.
    #[error("{message}")]
    ResourceOverflow {
        /// Resource whose accounting overflowed.
        resource: &'static str,
        /// Rendered diagnostic, including wrapper context when available.
        message: String,
    },
    /// The filter explicitly requested `halt(code)`.
    #[error("halt")]
    Halt(i32),
    /// The output consumer closed its pipe normally.
    #[error("broken pipe")]
    BrokenPipe,
}

impl YgError {
    /// Prefix an input-side error with filename and document context while
    /// retaining any structured source payload.
    #[must_use]
    pub fn with_input_context(self, source: &str, document_index: usize) -> Self {
        self.with_document_context(document_index)
            .with_input_source(source)
    }

    /// Prefix an input-side error with a filename while retaining structured
    /// source payloads and any document context already present.
    #[must_use]
    pub fn with_input_source(self, source: &str) -> Self {
        let contextualize = |message: String| {
            if message.starts_with("document ") {
                format!("{source} {message}")
            } else {
                format!("{source}: {message}")
            }
        };
        match self {
            Self::InputResolution {
                error,
                reports,
                message,
            } => Self::InputResolution {
                error,
                reports,
                message: contextualize(message),
            },
            Self::InputConversion {
                error,
                reports,
                message,
            } => Self::InputConversion {
                error,
                reports,
                message: contextualize(message),
            },
            Self::InputLimit { error, message } => Self::InputLimit {
                error,
                message: contextualize(message),
            },
            Self::ResourceOverflow { resource, message } => Self::ResourceOverflow {
                resource,
                message: contextualize(message),
            },
            Self::InputParse(message) => Self::InputParse(contextualize(message)),
            Self::InputRead(message) => Self::InputRead(contextualize(message)),
            error => error,
        }
    }

    pub(crate) fn with_document_context(self, document_index: usize) -> Self {
        let contextualize = |message: String| format!("document {}: {message}", document_index + 1);
        match self {
            Self::InputResolution {
                error,
                reports,
                message,
            } => Self::InputResolution {
                error,
                reports,
                message: contextualize(message),
            },
            Self::InputConversion {
                error,
                reports,
                message,
            } => Self::InputConversion {
                error,
                reports,
                message: contextualize(message),
            },
            Self::InputLimit { error, message } => Self::InputLimit {
                error,
                message: contextualize(message),
            },
            Self::ResourceOverflow { resource, message } => Self::ResourceOverflow {
                resource,
                message: contextualize(message),
            },
            Self::InputParse(message) => Self::InputParse(contextualize(message)),
            Self::InputRead(message) => Self::InputRead(contextualize(message)),
            error => error,
        }
    }

    pub(crate) fn runtime(exception: jaq_core::Exn<'_, crate::YamlVal>) -> Self {
        match exception.get_err() {
            Ok(error) => Self::Runtime(error.to_string()),
            Err(exception) => exception.get_halt().map_or_else(
                |_| Self::Runtime("unknown query exception".into()),
                Self::Halt,
            ),
        }
    }

    pub(crate) fn input_limit(error: LimitExceeded, message: impl Into<String>) -> Self {
        Self::InputLimit {
            error,
            message: message.into(),
        }
    }

    pub(crate) fn runtime_limit(error: LimitExceeded, message: impl Into<String>) -> Self {
        Self::RuntimeLimit {
            error,
            message: message.into(),
        }
    }

    pub(crate) fn input_resolution(
        error: yamalgam_parser::ResolveError,
        message: impl Into<String>,
        reports: Vec<ResolutionReport>,
    ) -> Self {
        Self::InputResolution {
            error: Box::new(error),
            reports,
            message: message.into(),
        }
    }

    pub(crate) fn runtime_resolution(
        error: Arc<yamalgam_parser::ResolveError>,
        message: impl Into<String>,
    ) -> Self {
        Self::RuntimeResolution {
            error,
            message: message.into(),
        }
    }

    pub(crate) fn input_conversion(error: Self, reports: Vec<ResolutionReport>) -> Self {
        let message = error.to_string();
        Self::InputConversion {
            error: Box::new(error),
            reports,
            message,
        }
    }

    pub(crate) fn resource_overflow(resource: &'static str, message: impl Into<String>) -> Self {
        Self::ResourceOverflow {
            resource,
            message: message.into(),
        }
    }
}

impl From<crate::emit::EmitError> for YgError {
    fn from(error: crate::emit::EmitError) -> Self {
        match error {
            crate::emit::EmitError::Limit(error) => Self::runtime_limit(error, error.to_string()),
            crate::emit::EmitError::ResourceOverflow(resource) => {
                Self::resource_overflow(resource, format!("{resource} overflow"))
            }
            other => Self::Runtime(other.to_string()),
        }
    }
}

impl From<BuildError> for YgError {
    fn from(error: BuildError) -> Self {
        match error {
            BuildError::Limit(error) => Self::input_limit(error, error.to_string()),
            BuildError::ResourceOverflow(resource) => {
                Self::resource_overflow(resource, format!("{resource} overflow"))
            }
            BuildError::Parse(error) => parse_error(error),
            BuildError::Resolve(error) => resolve_error(error),
            other => Self::InputParse(other.to_string()),
        }
    }
}

fn parse_error(error: yamalgam_parser::ParseError) -> YgError {
    let message = error.to_string();
    match error {
        yamalgam_parser::ParseError::LimitExceeded(error) => YgError::input_limit(error, message),
        yamalgam_parser::ParseError::ResourceOverflow(resource) => {
            YgError::resource_overflow(resource, message)
        }
        yamalgam_parser::ParseError::Scan(error) => match error.kind {
            yamalgam_scanner::scanner::ScanErrorKind::LimitExceeded(limit) => {
                YgError::input_limit(limit, message)
            }
            yamalgam_scanner::scanner::ScanErrorKind::ResourceOverflow(resource) => {
                YgError::resource_overflow(resource, message)
            }
            _ => YgError::InputParse(message),
        },
        _ => YgError::InputParse(message),
    }
}

fn resolve_error(error: yamalgam_parser::ResolveError) -> YgError {
    let message = error.to_string();
    match error {
        yamalgam_parser::ResolveError::LimitExceeded(error) => YgError::input_limit(error, message),
        yamalgam_parser::ResolveError::Parse(error) => parse_error(error),
        error => YgError::input_resolution(error, message, Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use super::{BuildError, YgError};
    use yamalgam_core::{LimitExceeded, ResourceLimit};
    use yamalgam_parser::ParseError;
    use yamalgam_scanner::scanner::{ScanError, ScanErrorKind};

    #[test]
    fn invariant_errors_are_distinct_from_resource_limits() {
        let error = BuildError::Invariant("truncated event stream".into());

        assert_eq!(
            error.to_string(),
            "parser/builder invariant violated: truncated event stream"
        );
        assert!(!matches!(error, BuildError::Limit(_)));
    }

    #[test]
    fn typed_input_limits_preserve_existing_wrapper_messages_and_sources() {
        let limit = LimitExceeded::new(ResourceLimit::Depth, 2, 1);
        let cases = [
            (
                YgError::from(BuildError::Limit(limit)),
                "depth 2 exceeds maximum of 1",
            ),
            (
                YgError::from(BuildError::Parse(ParseError::LimitExceeded(limit))),
                "limit exceeded: depth 2 exceeds maximum of 1",
            ),
            (
                YgError::from(BuildError::Parse(ParseError::Scan(ScanError::new(
                    ScanErrorKind::LimitExceeded(limit),
                    limit.to_string(),
                )))),
                "scan error: depth 2 exceeds maximum of 1",
            ),
        ];

        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
            assert!(matches!(
                &error,
                YgError::InputLimit { error, .. } if *error == limit
            ));
            assert_eq!(
                error
                    .source()
                    .and_then(|source| source.downcast_ref::<LimitExceeded>()),
                Some(&limit)
            );
        }
    }
}

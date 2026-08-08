//! Deserialization error type.

use std::fmt;

use yamalgam_core::Span;
use yamalgam_parser::ParseError;
use yamalgam_parser::ResolveError;

/// Errors that can occur during YAML deserialization.
#[derive(Debug)]
pub enum Error {
    /// Reading YAML input failed before parsing began.
    Io(std::io::Error),
    /// YAML parsing failed (scanner/parser level).
    Parse(ParseError),
    /// Resolver middleware error (!include, $ref).
    Resolve(ResolveError),
    /// Unexpected event during deserialization.
    Unexpected {
        /// What the deserializer expected at this point.
        expected: &'static str,
        /// Description of what was actually found.
        found: String,
        /// Source span of the unexpected event.
        span: Option<Span>,
    },
    /// Undefined alias reference.
    UndefinedAlias {
        /// The alias name (without `*`).
        name: String,
        /// Source span of the alias event.
        span: Option<Span>,
    },
    /// Resource limit exceeded.
    LimitExceeded(String),
    /// Multiple documents found in single-document API.
    MoreThanOneDocument,
    /// Custom error from serde Deserialize impls.
    Custom(String),
}

impl Error {
    /// Clone the structured form of this error.
    ///
    /// `std::io::Error` is reconstructed from its kind and rendered message.
    /// `Resolve` is the only variant that degrades to `Custom`, because a
    /// `ResolveError` can contain arbitrary non-cloneable sources.
    pub(crate) fn clone_structured(&self) -> Self {
        match self {
            Self::Io(e) => Self::Io(std::io::Error::new(e.kind(), e.to_string())),
            Self::Parse(e) => Self::Parse(e.clone()),
            Self::Resolve(e) => Self::Custom(e.to_string()),
            Self::Unexpected {
                expected,
                found,
                span,
            } => Self::Unexpected {
                expected,
                found: found.clone(),
                span: *span,
            },
            Self::UndefinedAlias { name, span } => Self::UndefinedAlias {
                name: name.clone(),
                span: *span,
            },
            Self::LimitExceeded(msg) => Self::LimitExceeded(msg.clone()),
            Self::MoreThanOneDocument => Self::MoreThanOneDocument,
            Self::Custom(msg) => Self::Custom(msg.clone()),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Parse(e) => write!(f, "{e}"),
            Self::Resolve(e) => write!(f, "{e}"),
            Self::Unexpected {
                expected,
                found,
                span,
            } => {
                write!(f, "expected {expected}, found {found}")?;
                write_position(f, *span)
            }
            Self::UndefinedAlias { name, span } => {
                write!(f, "undefined alias: *{name}")?;
                write_position(f, *span)
            }
            Self::LimitExceeded(msg) => write!(f, "limit exceeded: {msg}"),
            Self::MoreThanOneDocument => write!(f, "more than one document in input"),
            Self::Custom(msg) => write!(f, "{msg}"),
        }
    }
}

/// Append ` (line L, column C)` when a span is available (one-indexed).
fn write_position(f: &mut fmt::Formatter<'_>, span: Option<Span>) -> fmt::Result {
    span.map_or(Ok(()), |span| {
        write!(
            f,
            " (line {}, column {})",
            span.start.line + 1,
            span.start.column + 1
        )
    })
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Parse(e) => Some(e),
            Self::Resolve(e) => Some(e),
            _ => None,
        }
    }
}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Self::Parse(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<ResolveError> for Error {
    fn from(e: ResolveError) -> Self {
        Self::Resolve(e)
    }
}

impl From<erased_serde::Error> for Error {
    fn from(e: erased_serde::Error) -> Self {
        Self::Custom(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_structured_preserves_io_error_kind_and_message() {
        let original = Error::Io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "clone fixture failed",
        ));
        let original_message = original.to_string();

        let cloned = original.clone_structured();

        let Error::Io(error) = &cloned else {
            panic!("structured clone should retain Error::Io");
        };
        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        assert_eq!(cloned.to_string(), original_message);
    }
}

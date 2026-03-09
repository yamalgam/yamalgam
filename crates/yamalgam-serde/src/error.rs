//! Deserialization error type.

use std::fmt;

use yamalgam_core::Span;
use yamalgam_parser::ParseError;
use yamalgam_parser::ResolveError;

/// Errors that can occur during YAML deserialization.
#[derive(Debug)]
pub enum Error {
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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "{e}"),
            Self::Resolve(e) => write!(f, "{e}"),
            Self::Unexpected {
                expected, found, ..
            } => {
                write!(f, "expected {expected}, found {found}")
            }
            Self::UndefinedAlias { name, .. } => write!(f, "undefined alias: *{name}"),
            Self::LimitExceeded(msg) => write!(f, "limit exceeded: {msg}"),
            Self::MoreThanOneDocument => write!(f, "more than one document in input"),
            Self::Custom(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
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

impl From<ResolveError> for Error {
    fn from(e: ResolveError) -> Self {
        Self::Resolve(e)
    }
}

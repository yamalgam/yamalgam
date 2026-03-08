//! YAML 1.2 pull parser — events from tokens.
//!
//! This crate implements a StAX-style pull parser that consumes tokens from
//! [`yamalgam_scanner::Scanner`] and emits [`Event`]s representing the YAML
//! data model. The parser is an `Iterator<Item = Result<Event, ParseError>>`.
//!
//! # Quick Start
//!
//! ```
//! use yamalgam_parser::{Event, Parser};
//!
//! let events: Vec<_> = Parser::new("")
//!     .collect::<Result<Vec<_>, _>>()
//!     .unwrap();
//! assert!(matches!(events[0], Event::StreamStart));
//! assert!(matches!(events[1], Event::StreamEnd));
//! ```
#![deny(unsafe_code)]

pub mod compose;
pub mod error;
pub mod event;
pub mod parser;
pub mod resolve;

pub use compose::{ComposeError, Composer};
pub use error::ParseError;
pub use event::{CollectionStyle, Event};
pub use parser::Parser;
pub use resolve::{NoopResolver, ResolveError, ResolvedEvents, Resolver};
pub use yamalgam_scanner::ScalarStyle;

use yamalgam_core::Value;

/// Parse a YAML string into a list of [`Value`] documents.
pub fn from_str(input: &str) -> Result<Vec<Value>, ComposeError> {
    Composer::from_str(input)
}

/// Parse a YAML string into a single [`Value`].
/// Returns Null for empty input, error for multiple documents.
pub fn from_str_single(input: &str) -> Result<Value, ComposeError> {
    let mut docs = Composer::from_str(input)?;
    match docs.len() {
        0 => Ok(Value::Null),
        1 => Ok(docs.remove(0)),
        n => Err(ComposeError::UnexpectedEvent(format!(
            "expected 1 document, got {n}",
        ))),
    }
}

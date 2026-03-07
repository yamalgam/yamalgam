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

pub mod error;
pub mod event;
pub mod parser;

pub use error::ParseError;
pub use event::{CollectionStyle, Event};
pub use parser::Parser;
pub use yamalgam_scanner::ScalarStyle;

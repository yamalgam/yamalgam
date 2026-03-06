//! YAML scanner/tokenizer ported from libfyaml 0.9.5.
//!
//! This crate provides the lowest layer of the yamalgam YAML pipeline:
//! byte input -> token stream. The scanner is version-agnostic; YAML version
//! directives are emitted as [`TokenKind::VersionDirective`] tokens for the
//! parser layer to handle.
#![deny(unsafe_code)]

pub mod input;
pub mod reader;
pub mod scanner;

mod atom;
mod style;
mod token;

pub use atom::Atom;
pub use style::{AtomFlags, Chomp, ScalarStyle};
pub use token::{Token, TokenKind};

// Re-export position types from core for convenience.
pub use yamalgam_core::{Mark, Span};

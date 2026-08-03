//! YAML-native jq-style query engine.
//!
//! Implements the yg query language (`docs/spec/yg-query-language-spec-draft-01.md`)
//! on top of jaq 3. The value type carries YAML metadata—tag, anchor, style,
//! and raw scalar text—through filter pipelines.

pub mod build;
pub mod emit;
pub mod engine;
mod error;
mod funs;
mod ops;
mod valt;
pub mod value;

pub use build::{parse_str, parse_str_with_config};
pub use emit::{EmitFormat, EmitOpts};
pub use error::{BuildError, YgError};
pub use value::{Meta, NodeStyle, YamlVal};

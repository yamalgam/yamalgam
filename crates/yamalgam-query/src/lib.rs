//! YAML-native jq-style query engine.
//!
//! Implements the yg query language (`docs/spec/yg-query-language-spec-draft-01.md`)
//! on top of jaq 3. The value type carries YAML metadata—tag, anchor, style,
//! and raw scalar text—through filter pipelines.

pub mod build;
mod emit;
pub mod engine;
mod error;
mod funs;
mod ops;
mod valt;
pub mod value;

pub use build::{parse_str, parse_str_with_config};
pub use error::BuildError;
pub use value::{Meta, NodeStyle, YamlVal};

//! YAML-native jq-style query engine.
//!
//! Implements the yg query language (`docs/spec/yg-query-language-spec-draft-01.md`)
//! on top of jaq 3. The value type carries YAML metadata—tag, anchor, style,
//! and raw scalar text—through filter pipelines.
//!
//! Value storage is intentionally sealed behind [`YamlVal`]:
//! ```compile_fail
//! use yamalgam_query::value::Data;
//! ```

// r[impl overview]
// r[impl overview.positioning]
// r[impl overview.compat-promise]
// r[impl filters]
// r[impl diverge]
// r[impl ext]
// r[impl phases]

pub mod build;
pub mod emit;
pub mod engine;
mod error;
mod funs;
pub(crate) mod json;
mod ops;
mod runtime_limits;
mod valt;
pub(crate) mod value;

pub use build::{Documents, parse_documents_with_config, parse_str, parse_str_with_config};
pub use emit::{EmitError, EmitFormat, EmitOpts};
pub use error::{BuildError, YgError};
pub use value::{Meta, NodeStyle, YamlVal};

/// Parse one strict JSON value under the supplied resource limits.
pub fn parse_json_with_config(
    source: &str,
    config: &yamalgam_core::LoaderConfig,
) -> Result<YamlVal, YgError> {
    json::parse(source, config).map_err(YgError::Runtime)
}

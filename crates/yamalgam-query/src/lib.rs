//! YAML-native jq-style query engine.
//!
//! Implements the yg query language (`docs/spec/yg-query-language-spec-draft-01.md`)
//! on top of jaq 3. The value type carries YAML metadata—tag, anchor, style,
//! chomp, scalar flags, and raw scalar text—through filter pipelines.
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

#[cfg(test)]
thread_local! {
    static DEPTH_WALK_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn record_depth_walk() {
    DEPTH_WALK_CALLS.set(DEPTH_WALK_CALLS.get().saturating_add(1));
}

#[cfg(test)]
fn reset_depth_walk_calls() {
    DEPTH_WALK_CALLS.set(0);
}

#[cfg(test)]
fn depth_walk_calls() -> usize {
    DEPTH_WALK_CALLS.get()
}

pub mod build;
pub mod emit;
pub mod engine;
mod error;
mod funs;
pub(crate) mod json;
mod ops;
pub mod resolution;
mod runtime_limits;
mod runtime_resolution;
mod validation;
mod valt;
pub(crate) mod value;

pub use build::{
    Documents, parse_documents_with_config, parse_documents_with_resolver_and_config, parse_str,
    parse_str_with_config, parse_str_with_resolver, parse_str_with_resolver_and_config,
};
pub use emit::{EmitError, EmitFormat, EmitOpts};
pub use error::{BuildError, YgError};
pub use resolution::{
    QueryResolver, ResolutionHandle, ResolutionSessionKey, ResolvedInput, ResourceOrigin,
    resolution_handle, resolve_document, resolve_documents,
};
pub use validation::{QuerySchemaResolver, ValidationFailure};
pub use value::{Meta, NodeStyle, ScalarPresentation, ValueLimitError, YamlVal};

/// Parse one strict JSON value under the supplied resource limits.
pub fn parse_json_with_config(
    source: &str,
    config: &yamalgam_core::LoaderConfig,
) -> Result<YamlVal, YgError> {
    json::parse(source, config).map_err(|error| match error {
        json::JsonError::Limit(error) => YgError::input_limit(error, error.to_string()),
        json::JsonError::Build(error) => YgError::from(error),
        json::JsonError::ResourceOverflow(resource) => {
            YgError::resource_overflow(resource, format!("{resource} overflow"))
        }
        other => YgError::Runtime(other.to_string()),
    })
}

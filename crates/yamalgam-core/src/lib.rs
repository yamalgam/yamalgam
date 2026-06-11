//! Core library for yamalgam.
//!
//! Foundational types shared across the yamalgam pipeline: the [`Value`]
//! DOM, tag resolution, loader configuration and resource limits, source
//! positions, and diagnostics.
//!
//! # Quick Start
//!
//! ```
//! use yamalgam_core::Value;
//!
//! let v = Value::from("hello");
//! assert_eq!(v.as_str(), Some("hello"));
//! ```
#![deny(unsafe_code)]

// y[impl intro.goals.human-readable]
// y[impl intro.goals.native-match]
// y[impl intro.goals.portable]
// y[impl intro.goals.expressive]
// y[impl intro.goals.easy-impl]
// y[impl intro.goals.consistent-model]
// y[impl intro.goals.one-pass]
// y[impl intro.terminology.rfc2119]
// y[impl model.repr.graph-definition]
// y[impl model.repr.node-definition]
// y[impl model.repr.tag-definition]
// y[impl model.repr.canonical-form]
// y[impl model.repr.equality]
// y[impl model.repr.collection-vs-scalar]
// y[impl model.loading.well-formed]
// y[impl model.loading.reject-ill-formed]
// y[impl model.loading.failure-points]
// y[impl model.process.load.parse]
// y[impl model.process.load.compose]
// y[impl model.process.load.construct]
// y[impl overview.tags.global-tags-uri+3]
// y[impl overview.tags.local-tags+3]

pub mod diagnostic;

pub mod loader;

pub mod tag;

pub mod tag_resolution;

pub mod value;

pub use diagnostic::{Diagnostic, Label, Mark, Severity, Span};

pub use loader::{IncludePolicy, LoaderConfig, RefPolicy, ResolutionPolicy, ResourceLimits};

pub use tag::{Yaml12TagResolver, resolve_plain_scalar};

pub use tag_resolution::{
    FailsafeTagResolver, JsonTagResolver, TagResolution, TagResolver, Yaml11TagResolver,
};

pub use value::{Mapping, Value};

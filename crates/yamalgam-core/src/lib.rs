//! Core library for yamalgam.
//!
//! This crate provides the foundational types and functionality used by the
//! `yamalgam` CLI and any downstream consumers.
//!
//! # Modules
//!
//! - [`config`] - Configuration loading and management
//! - [`error`] - Error types and result aliases
//!
//! # Quick Start
//!
//! ```no_run
//! use yamalgam_core::{Config, ConfigLoader};
//!
//! let (config, _sources) = ConfigLoader::new()
//!     .with_user_config(true)
//!     .load()
//!     .expect("Failed to load configuration");
//!
//! println!("Log level: {:?}", config.log_level);
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

pub mod config;

pub mod diagnostic;

pub mod error;

pub mod loader;

pub mod observability;

pub mod tag;

pub mod tag_resolution;

pub mod value;

pub use config::{Config, ConfigLoader, LogLevel};

pub use diagnostic::{Diagnostic, Label, Mark, Severity, Span};

pub use error::{ConfigError, ConfigResult};

pub use loader::{IncludePolicy, LoaderConfig, RefPolicy, ResolutionPolicy, ResourceLimits};

pub use tag::{resolve_plain_scalar, Yaml12TagResolver};

pub use tag_resolution::{FailsafeTagResolver, JsonTagResolver, TagResolution, TagResolver};

pub use value::{Mapping, Value};

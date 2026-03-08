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

pub mod config;

pub mod diagnostic;

pub mod error;

pub mod loader;

pub mod observability;

pub mod tag;

pub mod value;

pub use config::{Config, ConfigLoader, LogLevel};

pub use diagnostic::{Diagnostic, Label, Mark, Severity, Span};

pub use error::{ConfigError, ConfigResult};

pub use loader::{IncludePolicy, LoaderConfig, RefPolicy, ResolutionPolicy, ResourceLimits};

pub use tag::resolve_plain_scalar;

pub use value::{Mapping, Value};

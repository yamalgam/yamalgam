//! Error types for yamalgam-core

use thiserror::Error;

/// Errors that can occur when working with configuration.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Failed to deserialize configuration.
    #[error("invalid configuration: {0}")]
    Deserialize(#[from] Box<figment::Error>),

    /// Configuration file not found after searching all locations.
    #[error("no configuration file found")]
    NotFound,

    /// Configuration file has an unsupported format.
    ///
    /// YAML config support returns once yamalgam reads config through its
    /// own parser instead of figment's serde_yaml-backed provider.
    #[error("unsupported config file format `{path}`: use TOML or JSON")]
    UnsupportedFormat {
        /// Path of the rejected config file.
        path: String,
    },
}

/// Result type alias using [`ConfigError`].
pub type ConfigResult<T> = Result<T, ConfigError>;

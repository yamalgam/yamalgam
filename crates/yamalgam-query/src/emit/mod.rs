//! YAML and JSON output serialization.

mod json;
mod yaml;

pub use json::to_json;
pub use yaml::to_yaml;

/// Output format selector for the CLI.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EmitFormat {
    /// YAML output.
    #[default]
    Yaml,
    /// JSON output with aliases materialized.
    Json,
}

/// Options shared by the YAML and JSON emitters.
#[derive(Clone, Debug)]
pub struct EmitOpts {
    /// Number of spaces in each indentation level.
    pub indent: usize,
    /// Emit a single-line representation.
    pub compact: bool,
    /// Sort mapping entries by resolved key value.
    pub sort_keys: bool,
    /// Enable ANSI color in supported emitters.
    pub color: bool,
}

impl Default for EmitOpts {
    fn default() -> Self {
        Self {
            indent: 2,
            compact: false,
            sort_keys: false,
            color: false,
        }
    }
}

impl EmitOpts {
    /// Construct compact single-line output options.
    #[must_use]
    pub fn compact() -> Self {
        Self {
            compact: true,
            ..Self::default()
        }
    }
}

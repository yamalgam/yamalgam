//! CLI configuration, loaded through librebar's layered discovery.
//!
//! Discovery walks up from the working directory looking for
//! `.yamalgam.{toml,yaml,yml,json}` / `yamalgam.{toml,yaml,yml,json}` and
//! merges the user-level config beneath it. An explicit `--config FILE`
//! layers on top.

use camino::Utf8PathBuf;
use librebar::config::LogLevel;
use serde::{Deserialize, Serialize};
use yamalgam_schema::SchemaAssociation;

const DEFAULT_SCHEMA_STORE_URL: &str = "https://www.schemastore.org/api/json/catalog.json";

/// Settings the CLI reads from configuration files.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Baseline log level when no `-q`/`-v` flag is passed.
    pub log_level: LogLevel,
    /// Log directory override. Defaults to the platform log directory;
    /// the `YAMALGAM_LOG_DIR` / `YAMALGAM_LOG_PATH` env vars take
    /// precedence over both.
    pub log_dir: Option<Utf8PathBuf>,
    /// Ordered file-pattern associations used after document-local schema hints.
    pub schemas: Vec<SchemaAssociation>,
    /// Optional SchemaStore catalog integration.
    pub schema_store: SchemaStoreConfig,
}

/// SchemaStore catalog configuration.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SchemaStoreConfig {
    /// Whether catalog selection is enabled without `--schema-store`.
    pub enabled: bool,
    /// Absolute URI of the SchemaStore-compatible catalog.
    pub url: String,
}

impl Default for SchemaStoreConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: DEFAULT_SCHEMA_STORE_URL.to_owned(),
        }
    }
}

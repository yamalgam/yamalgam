//! CLI configuration, loaded through librebar's layered discovery.
//!
//! Discovery walks up from the working directory looking for
//! `.yamalgam.{toml,yaml,yml,json}` / `yamalgam.{toml,yaml,yml,json}` and
//! merges the user-level config beneath it. An explicit `--config FILE`
//! layers on top.

use camino::Utf8PathBuf;
use librebar::config::LogLevel;
use serde::{Deserialize, Serialize};

/// Settings the CLI reads from configuration files.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// Baseline log level when no `-q`/`-v` flag is passed.
    pub log_level: LogLevel,
    /// Log directory override. Defaults to the platform log directory;
    /// the `YAMALGAM_LOG_DIR` / `YAMALGAM_LOG_PATH` env vars take
    /// precedence over both.
    pub log_dir: Option<Utf8PathBuf>,
}

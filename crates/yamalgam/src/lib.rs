//! Library interface for the `yamalgam` CLI.
//!
//! This crate exposes the CLI's argument parser and command structure as a library,
//! primarily for documentation generation and testing. The actual entry point is
//! in `main.rs`.
//!
//! # Structure
//!
//! - [`Cli`] - The root argument parser (clap derive, librebar [`CommonArgs`] flattened)
//! - [`Commands`] - Available subcommands
//! - [`commands`] - Command implementations
//! - [`config`] - CLI configuration loaded via librebar discovery
//!
//! # Documentation Generation
//!
//! The [`command()`] function returns the clap `Command` for generating man pages
//! and shell completions via `xtask`.
//!
//! [`CommonArgs`]: librebar::cli::CommonArgs
pub mod commands;
pub mod config;

// Re-export the YAML API so `yamalgam` works as a single dependency.
pub use yamalgam_core::{LoaderConfig, Mapping, Value};
pub use yamalgam_serde::{
    Deserializer, Error as DeserializeError, from_reader, from_str, from_str_with_config,
};

use clap::{CommandFactory, Parser, Subcommand};

const ENV_HELP: &str = "\
ENVIRONMENT VARIABLES:
    RUST_LOG             Log filter (e.g., debug, yamalgam=trace)
    YAMALGAM_LOG_PATH    Log file path (rotated daily)
    YAMALGAM_LOG_DIR     Log directory
";

/// Command-line interface definition for yamalgam.
#[derive(Parser)]
#[command(name = "yamalgam")]
#[command(about = "YAML toolkit", long_about = None)]
#[command(version, arg_required_else_help = true)]
#[command(after_help = ENV_HELP)]
#[command(disable_help_flag = true)]
pub struct Cli {
    /// Common flags (`-q`, `-v`, `--json`, `--color`, `-C`, `--version-only`).
    #[command(flatten)]
    pub common: librebar::cli::CommonArgs,

    /// Path to configuration file (overrides discovery)
    #[arg(short, long, global = true, value_name = "FILE")]
    pub config: Option<camino::Utf8PathBuf>,

    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Available subcommands for the CLI.
#[derive(Subcommand)]
pub enum Commands {
    /// Diagnose configuration and environment
    Doctor(commands::doctor::DoctorArgs),
    /// Show package information
    Info(commands::info::InfoArgs),
}

/// Returns the clap command for documentation generation.
///
/// Wraps the derive output with librebar's compact `-h`/`--help` flag.
pub fn command() -> clap::Command {
    librebar::cli::with_help_short(Cli::command())
}

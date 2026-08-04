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

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};

const AFTER_HELP: &str = "\
COLOR:
    Use --color auto|always|never. -C is --chdir, not a color flag.

ENVIRONMENT VARIABLES:
    RUST_LOG             Log filter (e.g., debug, yamalgam=trace)
    YAMALGAM_LOG_PATH    Log file path (rotated daily)
    YAMALGAM_LOG_DIR     Log directory
";

/// Command-line interface definition for yg.
// r[impl cli]
#[derive(Parser)]
#[command(name = "yg")]
#[command(about = "YAML-native jq-style query tool", long_about = None)]
#[command(version)]
#[command(after_help = AFTER_HELP)]
#[command(disable_help_flag = true)]
pub struct Cli {
    /// Common flags (`-q`, `-v`, `--format`, `--color`, `-C`, `--version-only`).
    #[command(flatten)]
    pub common: librebar::cli::CommonArgs,

    /// Path to configuration file (overrides discovery)
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<camino::Utf8PathBuf>,

    /// Query-mode arguments used when no subcommand is selected.
    #[command(flatten)]
    pub query: QueryArgs,

    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Arguments for yg's default query mode.
// r[impl cli.usage]
#[derive(Args, Clone, Debug, Default)]
pub struct QueryArgs {
    /// Filter expression (defaults to `.`).
    #[arg(value_name = "FILTER")]
    pub filter: Option<String>,

    /// Input files (standard input when omitted).
    #[arg(value_name = "FILE")]
    pub files: Vec<camino::Utf8PathBuf>,

    /// Run the filter once with null input.
    // r[impl cli.null-input]
    #[arg(short = 'n', long)]
    pub null_input: bool,

    /// Read all input documents into one sequence.
    // r[impl cli.slurp]
    #[arg(short = 's', long)]
    pub slurp: bool,

    /// Read each input line as a string.
    // r[impl cli.raw-input]
    #[arg(short = 'R', long)]
    pub raw_input: bool,

    /// Read the filter from a file.
    // r[impl cli.from-file]
    #[arg(short = 'f', long, value_name = "FILE")]
    pub from_file: Option<camino::Utf8PathBuf>,

    /// Disable the jq-compatible `env` filter.
    #[arg(long)]
    pub no_env: bool,

    /// Bind VALUE to $NAME as a string without filter interpolation.
    #[arg(long = "arg", value_names = ["NAME", "VALUE"], num_args = 2)]
    pub string_args: Vec<String>,

    /// Bind JSON to $NAME as a parsed JSON value without filter interpolation.
    #[arg(long = "argjson", value_names = ["NAME", "JSON"], num_args = 2)]
    pub json_args: Vec<String>,

    /// Output JSON, expanding YAML aliases.
    // r[impl cli.json-output]
    #[arg(short = 'j', long)]
    pub json_output: bool,

    /// Output strings without YAML quoting.
    // r[impl cli.raw-output]
    #[arg(short = 'r', long)]
    pub raw_output: bool,

    /// Emit each result on one line.
    // r[impl cli.compact]
    #[arg(short = 'c', long)]
    pub compact_output: bool,

    /// Edit files atomically; reserialization loses comments until CST editing lands.
    // r[impl cli.inplace]
    #[arg(
        short = 'i',
        long,
        conflicts_with_all = ["null_input", "slurp", "json_output"]
    )]
    pub in_place: bool,

    /// Sort mapping keys in output.
    // r[impl cli.sort-keys]
    #[arg(short = 'S', long)]
    pub sort_keys: bool,

    /// Suppress YAML document separators between results.
    // r[impl output.multidoc.suppress]
    #[arg(short = 'N', long)]
    pub no_doc: bool,

    /// Schema used to resolve plain scalars.
    // r[impl cli.schema-flag]
    #[arg(long, value_enum, default_value_t = SchemaArg::Core)]
    pub schema: SchemaArg,

    /// Use generous resource limits for trusted local input.
    // r[impl cli.trusted]
    #[arg(long, conflicts_with = "strict")]
    pub trusted: bool,

    /// Use tight resource limits for untrusted input.
    // r[impl cli.strict]
    #[arg(long)]
    pub strict: bool,

    /// Exit 1 for false/null and 4 when no output is produced.
    #[arg(short = 'e', long)]
    pub exit_status: bool,
}

/// Schema names accepted by `--schema`.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SchemaArg {
    /// YAML 1.2 core schema.
    #[default]
    Core,
    /// Resolve all plain scalars as strings.
    Failsafe,
    /// JSON scalar resolution rules.
    Json,
    /// Legacy YAML 1.1 scalar resolution rules.
    #[value(name = "1.1")]
    Yaml11,
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

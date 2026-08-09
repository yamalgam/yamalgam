//! yamalgam CLI
#![deny(unsafe_code)]

use std::io::IsTerminal as _;
use std::process::ExitCode;

use anyhow::Context;
use clap::FromArgMatches;
use librebar::cli::{ColorChoice, ResolvedOutputFormat};
use librebar::config::ConfigSources;
use tracing::debug;
use yamalgam::config::Config;
use yamalgam::{Cli, Commands, commands};

struct MainError {
    code: u8,
    source: anyhow::Error,
}

impl MainError {
    const fn usage(source: anyhow::Error) -> Self {
        Self { code: 2, source }
    }

    const fn runtime(source: anyhow::Error) -> Self {
        Self { code: 5, source }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("yg: {:#}", error.source);
            ExitCode::from(error.code)
        }
    }
}

fn run() -> Result<ExitCode, MainError> {
    let cli = Cli::from_arg_matches(&yamalgam::command().get_matches())
        .expect("clap mismatch between Cli derive and command()");

    if cli
        .common
        .apply(env!("CARGO_PKG_VERSION"))
        .context("failed to apply global CLI options")
        .map_err(MainError::usage)?
        .is_exit()
    {
        return Ok(ExitCode::SUCCESS);
    }

    let (config, config_sources) =
        load_config(cli.common.config.as_deref()).map_err(MainError::usage)?;

    let log_cfg = librebar::logging::LoggingConfig::from_app_name(env!("CARGO_PKG_NAME"))
        .with_log_dir(
            config
                .log_dir
                .as_ref()
                .map(|dir| dir.as_std_path().to_path_buf()),
        );
    let filter = librebar::logging::env_filter(
        cli.common.quiet,
        cli.common.verbose,
        config.log_level.as_str(),
    );
    let _guard = librebar::logging::init(&log_cfg, filter)
        .context("failed to initialize logging/tracing")
        .map_err(MainError::runtime)?;
    let subcommand_json = matches!(cli.common.output_format(), ResolvedOutputFormat::Json);

    debug!(
        verbose = cli.common.verbose,
        quiet = cli.common.quiet,
        json = subcommand_json,
        color = ?cli.common.color,
        chdir = ?cli.common.chdir,
        "CLI initialized"
    );

    match cli.command {
        Some(Commands::Doctor(args)) => {
            commands::doctor::cmd_doctor(args, subcommand_json, &config_sources)
                .map_err(MainError::runtime)?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Commands::Info(args)) => {
            commands::info::cmd_info(args, subcommand_json, &config, &config_sources)
                .map_err(MainError::runtime)?;
            Ok(ExitCode::SUCCESS)
        }
        None => {
            // Query output is YAML-native in auto mode, including in pipelines.
            // r[impl cli.json-output]
            let output = if cli.query.json_output
                || matches!(
                    cli.common.output_format_for(true),
                    ResolvedOutputFormat::Json
                ) {
                commands::query::OutputMode::Json
            } else {
                // Librebar 0.4 exposes color as --color auto|always|never.
                // r[impl cli.color]
                let color = match cli.common.color {
                    ColorChoice::Always => true,
                    ColorChoice::Never => false,
                    ColorChoice::Auto => std::io::stdout().is_terminal(),
                };
                commands::query::OutputMode::Yaml { color }
            };
            commands::query::cmd_query(
                &cli.query,
                commands::query::RunOptions {
                    output,
                    quiet: cli.common.quiet,
                },
            )
            .map_err(MainError::runtime)
        }
    }
}

/// Load configuration: project/user discovery, with `--config` layered on top.
fn load_config(explicit: Option<&std::path::Path>) -> anyhow::Result<(Config, ConfigSources)> {
    let cwd = std::env::current_dir().context("failed to determine current directory")?;
    let cwd = camino::Utf8PathBuf::try_from(cwd).map_err(|e| {
        anyhow::anyhow!(
            "current directory is not valid UTF-8: {}",
            e.into_path_buf().display()
        )
    })?;
    let mut loader =
        librebar::config::ConfigLoader::new(env!("CARGO_PKG_NAME")).with_project_search(&cwd);
    if let Some(path) = explicit {
        let path = camino::Utf8Path::from_path(path).ok_or_else(|| {
            anyhow::anyhow!("configuration path is not valid UTF-8: {}", path.display())
        })?;
        loader = loader.with_file(path);
    }
    loader
        .load::<Config>()
        .context("failed to load configuration")
}

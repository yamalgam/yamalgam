//! yamalgam CLI
#![deny(unsafe_code)]

use anyhow::Context;
use clap::FromArgMatches;
use librebar::config::ConfigSources;
use tracing::debug;
use yamalgam::config::Config;
use yamalgam::{Cli, Commands, commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::from_arg_matches(&yamalgam::command().get_matches())
        .expect("clap mismatch between Cli derive and command()");

    cli.common.apply_color();

    if cli.common.version_only {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // arg_required_else_help ensures we have --version-only or a subcommand
    let Some(command) = cli.command else {
        return Ok(());
    };

    cli.common.apply_chdir().with_context(|| {
        format!(
            "failed to change directory to {}",
            cli.common
                .chdir
                .as_deref()
                .unwrap_or_else(|| std::path::Path::new("?"))
                .display()
        )
    })?;

    let (config, config_sources) = load_config(cli.config.as_deref())?;

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
        .context("failed to initialize logging/tracing")?;

    debug!(
        verbose = cli.common.verbose,
        quiet = cli.common.quiet,
        json = cli.common.json,
        color = ?cli.common.color,
        chdir = ?cli.common.chdir,
        "CLI initialized"
    );

    // Execute command
    let result = match command {
        Commands::Doctor(args) => {
            commands::doctor::cmd_doctor(args, cli.common.json, &config_sources)
        }
        Commands::Info(args) => {
            commands::info::cmd_info(args, cli.common.json, &config, &config_sources)
        }
    };
    if let Err(ref err) = result {
        tracing::error!(error = %err, "fatal error");
    }
    result
}

/// Load configuration: project/user discovery, with `--config` layered on top.
fn load_config(explicit: Option<&camino::Utf8Path>) -> anyhow::Result<(Config, ConfigSources)> {
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
        loader = loader.with_file(path);
    }
    loader
        .load::<Config>()
        .context("failed to load configuration")
}

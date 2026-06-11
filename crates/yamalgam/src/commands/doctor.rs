//! Doctor command — diagnose configuration and environment.
//!
//! Health checks run through librebar's [`DoctorRunner`]; directory and
//! environment reporting round out the picture.

use clap::Args;
use librebar::config::{self, ConfigSources};
use librebar::diagnostics::{CheckResult, CheckStatus, DoctorCheck, DoctorRunner};
use owo_colors::OwoColorize;
use serde::Serialize;
use tracing::{debug, instrument};

const APP_NAME: &str = env!("CARGO_PKG_NAME");

/// Arguments for the `doctor` subcommand.
#[derive(Args, Debug, Default)]
pub struct DoctorArgs {
    // No subcommand-specific arguments; uses global --json flag
}

#[derive(Serialize)]
struct DoctorReport {
    checks: Vec<CheckEntry>,
    directories: DirectoryPaths,
    environment: EnvironmentInfo,
}

#[derive(Serialize)]
struct CheckEntry {
    name: String,
    category: String,
    status: &'static str,
    message: String,
}

#[derive(Serialize)]
struct DirectoryPaths {
    config: Option<String>,
    cache: Option<String>,
    data: Option<String>,
    log: Option<String>,
}

#[derive(Serialize)]
struct EnvironmentInfo {
    cwd: Option<String>,
    env_vars: Vec<EnvVar>,
}

#[derive(Serialize)]
struct EnvVar {
    name: &'static str,
    value: Option<String>,
    description: &'static str,
}

const fn status_str(status: CheckStatus) -> &'static str {
    match status {
        CheckStatus::Ok => "ok",
        CheckStatus::Warn => "warn",
        CheckStatus::Error => "error",
    }
}

// ─── Checks ─────────────────────────────────────────────────────────

struct ConfigCheck {
    sources: ConfigSources,
}

impl DoctorCheck for ConfigCheck {
    fn name(&self) -> &str {
        "config-discovered"
    }

    fn category(&self) -> &str {
        "configuration"
    }

    fn run(&self) -> CheckResult {
        self.sources.primary_file().map_or_else(
            || CheckResult {
                status: CheckStatus::Warn,
                message: "no config file found; running with defaults".into(),
            },
            |file| CheckResult {
                status: CheckStatus::Ok,
                message: format!("config loaded from {file}"),
            },
        )
    }
}

struct LogDirCheck;

impl DoctorCheck for LogDirCheck {
    fn name(&self) -> &str {
        "log-dir-resolvable"
    }

    fn category(&self) -> &str {
        "observability"
    }

    fn run(&self) -> CheckResult {
        match librebar::logging::platform_log_dir(APP_NAME) {
            Some(dir) if dir.exists() => CheckResult {
                status: CheckStatus::Ok,
                message: format!("log dir exists at {}", dir.display()),
            },
            Some(dir) => CheckResult {
                status: CheckStatus::Ok,
                message: format!(
                    "log dir resolves to {} (created on first write)",
                    dir.display()
                ),
            },
            None => CheckResult {
                status: CheckStatus::Error,
                message: "platform log directory could not be resolved".into(),
            },
        }
    }
}

// ─── Report assembly ────────────────────────────────────────────────

impl DoctorReport {
    fn gather(sources: &ConfigSources) -> Self {
        let mut runner = DoctorRunner::new();
        runner.add(Box::new(ConfigCheck {
            sources: sources.clone(),
        }));
        runner.add(Box::new(LogDirCheck));

        let checks = runner
            .run_all()
            .into_iter()
            .map(|named| CheckEntry {
                name: named.name,
                category: named.category,
                status: status_str(named.result.status),
                message: named.result.message,
            })
            .collect();

        Self {
            checks,
            directories: DirectoryPaths {
                config: config::user_config_dir(APP_NAME).map(|p| p.to_string()),
                cache: config::user_cache_dir(APP_NAME).map(|p| p.to_string()),
                data: config::user_data_dir(APP_NAME).map(|p| p.to_string()),
                log: librebar::logging::platform_log_dir(APP_NAME).map(|p| p.display().to_string()),
            },
            environment: EnvironmentInfo {
                cwd: std::env::current_dir()
                    .ok()
                    .map(|p| p.display().to_string()),
                env_vars: vec![
                    EnvVar {
                        name: "XDG_CONFIG_HOME",
                        value: std::env::var("XDG_CONFIG_HOME").ok(),
                        description: "Override config directory",
                    },
                    EnvVar {
                        name: "XDG_CACHE_HOME",
                        value: std::env::var("XDG_CACHE_HOME").ok(),
                        description: "Override cache directory",
                    },
                    EnvVar {
                        name: "XDG_DATA_HOME",
                        value: std::env::var("XDG_DATA_HOME").ok(),
                        description: "Override data directory",
                    },
                    EnvVar {
                        name: "RUST_LOG",
                        value: std::env::var("RUST_LOG").ok(),
                        description: "Log filter directive",
                    },
                    EnvVar {
                        name: "YAMALGAM_LOG_DIR",
                        value: std::env::var("YAMALGAM_LOG_DIR").ok(),
                        description: "Override log directory",
                    },
                    EnvVar {
                        name: "YAMALGAM_LOG_PATH",
                        value: std::env::var("YAMALGAM_LOG_PATH").ok(),
                        description: "Override log file path",
                    },
                ],
            },
        }
    }
}

/// Run diagnostics and report configuration status.
#[instrument(name = "cmd_doctor", skip_all, fields(json_output))]
pub fn cmd_doctor(
    _args: DoctorArgs,
    global_json: bool,
    sources: &ConfigSources,
) -> anyhow::Result<()> {
    debug!(json_output = global_json, "executing doctor command");
    let report = DoctorReport::gather(sources);

    if global_json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!("{}", "Checks".bold().underline());
    for check in &report.checks {
        let marker = match check.status {
            "ok" => "✓".green().to_string(),
            "warn" => "○".yellow().to_string(),
            _ => "✗".red().to_string(),
        };
        println!(
            "  {} {} ({}): {}",
            marker,
            check.name,
            check.category.dimmed(),
            check.message.cyan()
        );
    }
    println!();

    println!("{}", "Directories".bold().underline());
    print_dir("  Config", &report.directories.config);
    print_dir("  Cache", &report.directories.cache);
    print_dir("  Data", &report.directories.data);
    print_dir("  Logs", &report.directories.log);
    println!();

    println!("{}", "Environment".bold().underline());
    if let Some(ref cwd) = report.environment.cwd {
        println!("  {}: {}", "Working directory".dimmed(), cwd.cyan());
    }

    let set_vars: Vec<_> = report
        .environment
        .env_vars
        .iter()
        .filter(|v| v.value.is_some())
        .collect();

    if set_vars.is_empty() {
        println!("  {} No XDG/logging overrides set", "○".dimmed());
    } else {
        for var in set_vars {
            println!(
                "  {}: {}",
                var.name.dimmed(),
                var.value.as_deref().unwrap_or("").cyan()
            );
        }
    }

    Ok(())
}

fn print_dir(label: &str, path: &Option<String>) {
    print!("{}: ", label.dimmed());
    match path {
        Some(p) => println!("{}", p.cyan()),
        None => println!("{}", "(unavailable)".yellow()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_doctor_text_succeeds() {
        assert!(cmd_doctor(DoctorArgs::default(), false, &ConfigSources::default()).is_ok());
    }

    #[test]
    fn cmd_doctor_json_succeeds() {
        assert!(cmd_doctor(DoctorArgs::default(), true, &ConfigSources::default()).is_ok());
    }

    #[test]
    fn doctor_report_gathers() {
        let report = DoctorReport::gather(&ConfigSources::default());
        assert_eq!(report.checks.len(), 2);
        // On most systems, at least one user directory should resolve.
        assert!(report.directories.config.is_some() || report.directories.cache.is_some());
    }
}

//! Benchmark runner command for xtask.

use clap::Args;
use std::process::Command;

use crate::workspace_root;

#[derive(Args, Debug)]
pub struct BenchArgs {
    /// Skip scanner/parser self-benchmarks
    #[arg(long)]
    pub skip_self: bool,

    /// Skip comparative (vs peers) benchmarks
    #[arg(long)]
    pub skip_comparative: bool,

    /// Skip CLI (hyperfine) benchmarks
    #[arg(long)]
    pub skip_cli: bool,

    /// Run in quick mode (fewer iterations)
    #[arg(long)]
    pub quick: bool,

    /// Filter benchmarks by name pattern
    #[arg(long)]
    pub filter: Option<String>,
}

pub fn cmd_bench(args: BenchArgs) -> Result<(), String> {
    let root = workspace_root();
    let reports_dir = root.join("bench-reports");

    std::fs::create_dir_all(&reports_dir)
        .map_err(|e| format!("Failed to create bench-reports directory: {e}"))?;

    let mut any_run = false;

    // Scanner/parser self-benchmarks
    if !args.skip_self {
        println!("\n=== Running Self-Benchmarks (scanner + parser) ===\n");

        for pkg in ["yamalgam-scanner", "yamalgam-parser"] {
            let mut cmd = Command::new("cargo");
            cmd.current_dir(&root).args(["bench", "-p", pkg]);

            if let Some(ref filter) = args.filter {
                cmd.args(["--", filter]);
            }

            let status = cmd
                .status()
                .map_err(|e| format!("Failed to run {pkg} benchmarks: {e}"))?;

            if !status.success() {
                return Err(format!("{pkg} benchmarks failed"));
            }
        }
        any_run = true;
    }

    // Comparative benchmarks against peer YAML crates
    if !args.skip_comparative {
        println!("\n=== Running Comparative Benchmarks (vs peers) ===\n");

        let mut cmd = Command::new("cargo");
        cmd.current_dir(&root)
            .args(["bench", "-p", "yamalgam-bench"]);

        if let Some(ref filter) = args.filter {
            cmd.args(["--", filter]);
        }

        let status = cmd
            .status()
            .map_err(|e| format!("Failed to run comparative benchmarks: {e}"))?;

        if !status.success() {
            return Err("Comparative benchmarks failed".to_string());
        }
        any_run = true;
    }

    // CLI benchmarks with hyperfine
    if !args.skip_cli {
        println!("\n=== Running CLI Benchmarks (hyperfine) ===\n");

        // Check if hyperfine is available
        let hyperfine_check = Command::new("which")
            .arg("hyperfine")
            .output()
            .map_err(|e| format!("Failed to check for hyperfine: {e}"))?;

        if !hyperfine_check.status.success() {
            println!("Warning: hyperfine not found. Skipping CLI benchmarks.");
            println!("Install with: brew install hyperfine (or cargo install hyperfine)\n");
        } else {
            let script = root.join("scripts").join("bench-cli.sh");

            if !script.exists() {
                println!("Warning: scripts/bench-cli.sh not found. Skipping CLI benchmarks.\n");
            } else {
                let mut cmd = Command::new("bash");
                cmd.current_dir(&root).arg(&script);

                if args.quick {
                    cmd.arg("--quick");
                }

                let status = cmd
                    .status()
                    .map_err(|e| format!("Failed to run CLI benchmarks: {e}"))?;

                if !status.success() {
                    return Err("CLI benchmarks failed".to_string());
                }
                any_run = true;
            }
        }
    }

    if !any_run {
        println!("All benchmark suites were skipped.");
    }

    Ok(())
}

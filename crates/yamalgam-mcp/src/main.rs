//! MCP server for scanner comparison during development.

use std::process::ExitCode;

const fn run() -> anyhow::Result<()> {
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e:?}");
            ExitCode::FAILURE
        }
    }
}

//! MCP server for scanner comparison during development.
//!
//! Exposes three tools over stdio transport:
//! - `list_test_cases` — discover YAML Test Suite cases
//! - `compare_tokens` — compare libfyaml and yamalgam token streams
//! - `debug_scanner` — run the scanner with trace output

mod paths;
mod tools;

use anyhow::Context;
use rmcp::ServiceExt;
use rmcp::transport::stdio;
use yamalgam_core::observability;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let obs_config =
        observability::ObservabilityConfig::from_env_with_overrides(env!("CARGO_PKG_NAME"), None);
    let env_filter = observability::env_filter(false, 0, "info");
    let _guard = observability::init_observability(&obs_config, env_filter)
        .context("failed to initialize logging")?;

    let server = tools::YamalgamServer::new()?;
    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}

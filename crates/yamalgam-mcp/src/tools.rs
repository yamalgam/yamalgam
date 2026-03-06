//! MCP tool definitions for scanner comparison and debugging.

use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars::{self, JsonSchema},
    tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};

use crate::paths::YamalgamPaths;

// ---------------------------------------------------------------------------
// Parameter structs
// ---------------------------------------------------------------------------

/// Parameters for listing YAML Test Suite cases.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListTestCasesParams {
    /// Optional filter string — only return test cases whose ID contains this substring.
    pub filter: Option<String>,
}

/// Parameters for comparing tokenizer output.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct CompareTokensParams {
    /// A YAML Test Suite case ID (e.g. "229Q"). Loads input from the test suite.
    pub test_case: Option<String>,
    /// Raw YAML input to compare. Used when `test_case` is not provided.
    pub input: Option<String>,
}

/// Parameters for running the scanner with debug output.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DebugScannerParams {
    /// YAML input to scan.
    pub input: String,
    /// Optional grep pattern to filter trace output lines.
    pub filter: Option<String>,
}

// ---------------------------------------------------------------------------
// Server struct
// ---------------------------------------------------------------------------

/// yamalgam MCP server for scanner comparison and debugging.
#[derive(Clone)]
pub struct YamalgamServer {
    paths: Arc<YamalgamPaths>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl YamalgamServer {
    /// Create a new server, discovering paths automatically.
    pub fn new() -> anyhow::Result<Self> {
        let paths = YamalgamPaths::discover()?;

        tracing::info!(
            root = %paths.project_root.display(),
            fyaml = %paths.fyaml_tokenize.display(),
            "yamalgam MCP server initialized"
        );

        Ok(Self {
            paths: Arc::new(paths),
            tool_router: Self::tool_router(),
        })
    }

    /// List available YAML Test Suite cases.
    #[tool(
        description = "List YAML Test Suite cases. Returns an array of test case IDs with metadata. Use 'filter' to search by substring."
    )]
    async fn list_test_cases(
        &self,
        Parameters(params): Parameters<ListTestCasesParams>,
    ) -> Result<CallToolResult, McpError> {
        let cases = list_test_suite_cases(&self.paths.test_suite_dir, params.filter.as_deref());

        let json = serde_json::to_string_pretty(&cases)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Compare tokenizer output between libfyaml and yamalgam.
    #[tool(
        description = "Compare token streams from libfyaml (C) and yamalgam (Rust) on the same YAML input. Provide either 'test_case' (a YAML Test Suite ID) or 'input' (raw YAML)."
    )]
    async fn compare_tokens(
        &self,
        Parameters(params): Parameters<CompareTokensParams>,
    ) -> Result<CallToolResult, McpError> {
        let yaml_input = match (&params.test_case, &params.input) {
            (Some(case_id), _) => {
                let path = self.paths.test_suite_dir.join(format!("{case_id}.yaml"));
                if !path.exists() {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "Test case '{case_id}' not found at {}",
                        path.display()
                    ))]));
                }
                std::fs::read_to_string(&path)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            (None, Some(raw)) => raw.clone(),
            (None, None) => {
                return Ok(CallToolResult::error(vec![Content::text(
                    "Either 'test_case' or 'input' must be provided",
                )]));
            }
        };

        let result = yamalgam_compare::compare_input(yaml_input.as_bytes());

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Run the yamalgam scanner with debug tracing.
    #[tool(
        description = "Run the yamalgam scanner on YAML input with debug logging. Returns trace output. Use 'filter' to grep for specific patterns in the trace."
    )]
    async fn debug_scanner(
        &self,
        Parameters(params): Parameters<DebugScannerParams>,
    ) -> Result<CallToolResult, McpError> {
        let input = params.input.as_bytes();

        // Run the Rust scanner to capture its result (or error).
        let scanner_result = yamalgam_compare::run_rust_scanner(input);

        let mut output = String::new();

        match &scanner_result {
            Ok(tokens) => {
                output.push_str(&format!("Scanner produced {} tokens:\n", tokens.len()));
                let json = serde_json::to_string_pretty(&tokens)
                    .unwrap_or_else(|e| format!("(serialization error: {e})"));
                output.push_str(&json);
            }
            Err(err) => {
                output.push_str(&format!("Scanner error: {err}\n"));
            }
        }

        // Also run the C tokenizer for reference.
        output.push_str("\n\n--- C tokenizer (libfyaml) reference ---\n");
        match yamalgam_compare::run_c_tokenizer(input) {
            Ok(tokens) => {
                output.push_str(&format!("C tokenizer produced {} tokens:\n", tokens.len()));
                let json = serde_json::to_string_pretty(&tokens)
                    .unwrap_or_else(|e| format!("(serialization error: {e})"));
                output.push_str(&json);
            }
            Err(err) => {
                output.push_str(&format!("C tokenizer error: {err}\n"));
            }
        }

        // Apply filter if provided.
        let final_output = if let Some(ref pattern) = params.filter {
            let pattern_lower = pattern.to_lowercase();
            let matching: Vec<&str> = output
                .lines()
                .filter(|line| line.to_lowercase().contains(&pattern_lower))
                .collect();
            format!(
                "Filtered output ({} lines matching '{}'):\n{}",
                matching.len(),
                pattern,
                matching.join("\n")
            )
        } else {
            output
        };

        Ok(CallToolResult::success(vec![Content::text(final_output)]))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for YamalgamServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Compare YAML scanner output between libfyaml (C) and yamalgam (Rust). \
                 Tools: list_test_cases, compare_tokens, debug_scanner.",
        )
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Metadata for a single test suite case.
#[derive(Debug, Serialize)]
struct TestCaseInfo {
    id: String,
}

/// Read the test suite directory and return matching case IDs.
fn list_test_suite_cases(
    test_suite_dir: &std::path::Path,
    filter: Option<&str>,
) -> Vec<TestCaseInfo> {
    let mut cases = Vec::new();

    let Ok(entries) = std::fs::read_dir(test_suite_dir) else {
        return cases;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "yaml")
            && let Some(stem) = path.file_stem()
        {
            let id = stem.to_string_lossy().to_string();

            // Apply filter if provided.
            if let Some(f) = filter
                && !id.to_lowercase().contains(&f.to_lowercase())
            {
                continue;
            }

            cases.push(TestCaseInfo { id });
        }
    }

    cases.sort_by(|a, b| a.id.cmp(&b.id));
    cases
}

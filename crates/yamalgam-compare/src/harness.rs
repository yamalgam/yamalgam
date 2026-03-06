//! Harness for running both tokenizer implementations and comparing results.
//!
//! Invokes the C harness (`fyaml-tokenize`) as a subprocess and the Rust
//! scanner in-process, then feeds both outputs to [`compare_token_streams`].

use std::path::PathBuf;
use std::process::Command;

use crate::compare::{CompareResult, compare_token_streams};
use crate::snapshot::{SpanSnapshot, TokenSnapshot};

/// Locate the `fyaml-tokenize` binary.
///
/// Search order:
/// 1. `FYAML_TOKENIZE_PATH` environment variable
/// 2. `tools/fyaml-tokenize/fyaml-tokenize` relative to the workspace root
///    (detected via `CARGO_MANIFEST_DIR` walking up to the workspace)
fn find_fyaml_binary() -> Option<PathBuf> {
    // 1. Explicit env var.
    if let Ok(path) = std::env::var("FYAML_TOKENIZE_PATH") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    // 2. Walk up from CARGO_MANIFEST_DIR looking for tools/fyaml-tokenize/.
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut dir = PathBuf::from(manifest_dir);
        loop {
            let candidate = dir.join("tools/fyaml-tokenize/fyaml-tokenize");
            if candidate.exists() {
                return Some(candidate);
            }
            if !dir.pop() {
                break;
            }
        }
    }

    // 3. Walk up from current directory.
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir = cwd;
        loop {
            let candidate = dir.join("tools/fyaml-tokenize/fyaml-tokenize");
            if candidate.exists() {
                return Some(candidate);
            }
            if !dir.pop() {
                break;
            }
        }
    }

    None
}

/// Run the C tokenizer (`fyaml-tokenize`) as a subprocess.
///
/// Pipes `input` via stdin, parses JSON lines from stdout into
/// [`TokenSnapshot`]s. Returns `Err` with stderr content on non-zero exit.
///
/// # Errors
///
/// Returns an error string if the binary is not found, the subprocess fails
/// to spawn, or the exit code is non-zero.
pub fn run_c_tokenizer(input: &[u8]) -> Result<Vec<TokenSnapshot>, String> {
    let binary = find_fyaml_binary()
        .ok_or_else(|| "fyaml-tokenize binary not found (set FYAML_TOKENIZE_PATH)".to_string())?;

    let mut child = Command::new(&binary)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn {}: {e}", binary.display()))?;

    // Write input to stdin.
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(input)
            .map_err(|e| format!("failed to write to fyaml-tokenize stdin: {e}"))?;
        // Drop to close stdin so the child can proceed.
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("failed to wait for fyaml-tokenize: {e}"))?;

    // Parse stderr for JSON error objects (ignore non-JSON lines which are
    // libfyaml's native diagnostics).
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        // Try to extract a JSON error from stderr.
        for line in stderr.lines() {
            if let Ok(obj) = serde_json::from_str::<serde_json::Value>(line)
                && let Some(err_msg) = obj.get("error").and_then(|v| v.as_str())
            {
                return Err(err_msg.to_string());
            }
        }
        return Err(format!(
            "fyaml-tokenize exited with {}: {}",
            output.status,
            stderr.trim()
        ));
    }

    // Parse JSON lines from stdout.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut tokens = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let token = parse_c_token_line(line)
            .map_err(|e| format!("failed to parse C token JSON: {e}\nline: {line}"))?;
        tokens.push(token);
    }

    Ok(tokens)
}

/// Parse a single JSON line from the C harness into a [`TokenSnapshot`].
fn parse_c_token_line(line: &str) -> Result<TokenSnapshot, String> {
    let obj: serde_json::Value =
        serde_json::from_str(line).map_err(|e| format!("invalid JSON: {e}"))?;

    let kind = obj
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or("missing 'type' field")?
        .to_string();

    let value = obj
        .get("value")
        .and_then(|v| if v.is_null() { None } else { v.as_str() })
        .map(String::from);

    // The C harness doesn't emit style information.
    let style = None;

    let line_num = obj.get("line").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let column = obj.get("column").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let offset = obj.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let end_line = obj.get("end_line").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let end_column = obj.get("end_column").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let end_offset = obj.get("end_offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

    Ok(TokenSnapshot {
        kind,
        value,
        style,
        span: SpanSnapshot {
            line: line_num,
            column,
            offset,
            end_line,
            end_column,
            end_offset,
        },
    })
}

/// Run the Rust scanner on the same input.
///
/// Converts each [`yamalgam_scanner::Token`] to a [`TokenSnapshot`].
///
/// Currently returns `Err` because the scanner has no state machine yet —
/// that's expected. This stub will be filled in as the scanner matures.
///
/// # Errors
///
/// Returns an error string describing the scanner failure.
pub fn run_rust_scanner(input: &[u8]) -> Result<Vec<TokenSnapshot>, String> {
    // Validate that we can at least decode the input.
    let _input = yamalgam_scanner::input::Input::from_bytes(input)
        .map_err(|diag| format!("input decode error: {}", diag.message))?;

    // The scanner has no state machine yet — return an error indicating that.
    // This will be replaced with real scanning once the state machine is built.
    Err("scanner not yet implemented: no state machine".to_string())
}

/// Compare both implementations on the same input.
///
/// Runs the C harness and Rust scanner, then matches on their results to
/// produce a [`CompareResult`].
pub fn compare_input(input: &[u8]) -> CompareResult {
    let c_result = run_c_tokenizer(input);
    let rust_result = run_rust_scanner(input);

    match (c_result, rust_result) {
        (Ok(c_tokens), Ok(rust_tokens)) => compare_token_streams(&c_tokens, &rust_tokens),
        (Err(c_err), Err(rust_err)) => {
            if c_err == rust_err {
                CompareResult::BothErrorMatch
            } else {
                CompareResult::BothErrorMismatch {
                    c_error: c_err,
                    rust_error: rust_err,
                }
            }
        }
        (Ok(c_tokens), Err(rust_err)) => CompareResult::CSuccessRustError {
            rust_error: rust_err,
            c_token_count: c_tokens.len(),
        },
        (Err(c_err), Ok(rust_tokens)) => CompareResult::RustSuccessCError {
            c_error: c_err,
            rust_token_count: rust_tokens.len(),
        },
    }
}

//! Harness for running both implementations and comparing results.
//!
//! Invokes the C harness (`fyaml-tokenize`) as a subprocess and the Rust
//! scanner/parser in-process, then feeds both outputs to comparison functions.
//!
//! Token-level: [`compare_input`] uses [`run_c_tokenizer`] + [`run_rust_scanner`]
//! Event-level: [`compare_events`] uses [`run_c_events`] + [`run_rust_parser`]

use std::path::PathBuf;
use std::process::Command;

use crate::compare::{CompareEventResult, CompareResult, compare_event_streams, compare_token_streams};
use crate::event_snapshot::EventSnapshot;
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

/// Convert a C harness token type name (`SCREAMING_SNAKE_CASE`) to
/// the PascalCase used by Rust's `TokenKind` debug representation.
fn normalize_c_token_kind(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                let mut s = first.to_uppercase().to_string();
                s.extend(chars.map(|c| c.to_ascii_lowercase()));
                s
            })
        })
        .collect()
}

/// Parse a single JSON line from the C harness into a [`TokenSnapshot`].
fn parse_c_token_line(line: &str) -> Result<TokenSnapshot, String> {
    let obj: serde_json::Value =
        serde_json::from_str(line).map_err(|e| format!("invalid JSON: {e}"))?;

    let kind = obj
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or("missing 'type' field")?;
    let kind = normalize_c_token_kind(kind);

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
/// Decodes the input, runs the scanner, and converts each
/// [`yamalgam_scanner::Token`] to a [`TokenSnapshot`].
///
/// # Errors
///
/// Returns an error string on encoding failures or scan errors.
pub fn run_rust_scanner(input: &[u8]) -> Result<Vec<TokenSnapshot>, String> {
    let decoded = yamalgam_scanner::input::Input::from_bytes(input)
        .map_err(|diag| format!("input decode error: {}", diag.message))?;

    let scanner = yamalgam_scanner::scanner::Scanner::new(decoded.as_str());
    let mut tokens = Vec::new();

    for result in scanner {
        let token = result.map_err(|e| e.to_string())?;
        tokens.push(token_to_snapshot(&token));
    }

    Ok(tokens)
}

/// Convert a Rust scanner token to an implementation-neutral snapshot.
fn token_to_snapshot(token: &yamalgam_scanner::Token<'_>) -> TokenSnapshot {
    let kind = format!("{:?}", token.kind);
    let value = if token.kind == yamalgam_scanner::TokenKind::Scalar || !token.atom.data.is_empty()
    {
        // Scalars always have a value (even empty string "").
        // Other tokens (anchors, tags, etc.) use None for empty data.
        Some(token.atom.data.to_string())
    } else {
        None
    };
    TokenSnapshot {
        kind,
        value,
        style: None,
        span: SpanSnapshot {
            line: token.atom.span.start.line,
            column: token.atom.span.start.column,
            offset: token.atom.span.start.offset,
            end_line: token.atom.span.end.line,
            end_column: token.atom.span.end.column,
            end_offset: token.atom.span.end.offset,
        },
    }
}

/// Compare both implementations on the same input (token-level).
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

// ---------------------------------------------------------------------------
// Event-level comparison
// ---------------------------------------------------------------------------

/// Run the C harness in `--events` mode as a subprocess.
///
/// Pipes `input` via stdin, parses JSON lines from stdout into
/// [`EventSnapshot`]s. Returns `Err` with stderr content on non-zero exit.
///
/// # Errors
///
/// Returns an error string if the binary is not found, the subprocess fails
/// to spawn, or the exit code is non-zero.
pub fn run_c_events(input: &[u8]) -> Result<Vec<EventSnapshot>, String> {
    let binary = find_fyaml_binary()
        .ok_or_else(|| "fyaml-tokenize binary not found (set FYAML_TOKENIZE_PATH)".to_string())?;

    let mut child = Command::new(&binary)
        .arg("--events")
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
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("failed to wait for fyaml-tokenize: {e}"))?;

    // Parse stderr for JSON error objects.
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        for line in stderr.lines() {
            if let Ok(obj) = serde_json::from_str::<serde_json::Value>(line)
                && let Some(err_msg) = obj.get("error").and_then(|v| v.as_str())
            {
                return Err(err_msg.to_string());
            }
        }
        return Err(format!(
            "fyaml-tokenize --events exited with {}: {}",
            output.status,
            stderr.trim()
        ));
    }

    // Parse JSON lines from stdout.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut events = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let event = parse_c_event_line(line)
            .map_err(|e| format!("failed to parse C event JSON: {e}\nline: {line}"))?;
        events.push(event);
    }

    Ok(events)
}

/// Parse a single JSON line from the C harness `--events` output into an [`EventSnapshot`].
fn parse_c_event_line(line: &str) -> Result<EventSnapshot, String> {
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

    // Alias events use "name" field.
    let value = value.or_else(|| {
        obj.get("name")
            .and_then(|v| if v.is_null() { None } else { v.as_str() })
            .map(String::from)
    });

    let anchor = obj
        .get("anchor")
        .and_then(|v| if v.is_null() { None } else { v.as_str() })
        .map(String::from);

    let tag = obj
        .get("tag")
        .and_then(|v| if v.is_null() { None } else { v.as_str() })
        .map(String::from);

    let implicit = obj
        .get("implicit")
        .and_then(|v| v.as_bool());

    Ok(EventSnapshot {
        kind,
        anchor,
        tag,
        value,
        implicit,
    })
}

/// Run the Rust parser on the same input.
///
/// Creates a [`yamalgam_parser::Parser`], collects events, and converts each
/// to an [`EventSnapshot`]. `VersionDirective` and `TagDirective` events are
/// filtered out since libfyaml does not emit them as separate events.
///
/// # Errors
///
/// Returns an error string on parse errors.
pub fn run_rust_parser(input: &[u8]) -> Result<Vec<EventSnapshot>, String> {
    let text = std::str::from_utf8(input)
        .map_err(|e| format!("input is not valid UTF-8: {e}"))?;

    let parser = yamalgam_parser::Parser::new(text);
    let mut events = Vec::new();

    for result in parser {
        let event = result.map_err(|e| e.to_string())?;
        // Filter out VersionDirective and TagDirective — yamalgam-specific,
        // libfyaml doesn't emit these as separate events.
        if let Some(snapshot) = event_to_snapshot(&event) {
            events.push(snapshot);
        }
    }

    Ok(events)
}

/// Convert a Rust parser event to an implementation-neutral snapshot.
///
/// Returns `None` for `VersionDirective` and `TagDirective` events, which
/// are yamalgam-specific and have no counterpart in libfyaml's event stream.
fn event_to_snapshot(event: &yamalgam_parser::Event<'_>) -> Option<EventSnapshot> {
    match event {
        yamalgam_parser::Event::StreamStart => Some(EventSnapshot {
            kind: "StreamStart".to_string(),
            anchor: None,
            tag: None,
            value: None,
            implicit: None,
        }),
        yamalgam_parser::Event::StreamEnd => Some(EventSnapshot {
            kind: "StreamEnd".to_string(),
            anchor: None,
            tag: None,
            value: None,
            implicit: None,
        }),
        yamalgam_parser::Event::VersionDirective { .. } => None,
        yamalgam_parser::Event::TagDirective { .. } => None,
        yamalgam_parser::Event::DocumentStart { implicit, .. } => Some(EventSnapshot {
            kind: "DocumentStart".to_string(),
            anchor: None,
            tag: None,
            value: None,
            implicit: Some(*implicit),
        }),
        yamalgam_parser::Event::DocumentEnd { implicit, .. } => Some(EventSnapshot {
            kind: "DocumentEnd".to_string(),
            anchor: None,
            tag: None,
            value: None,
            implicit: Some(*implicit),
        }),
        yamalgam_parser::Event::SequenceStart { anchor, tag, .. } => Some(EventSnapshot {
            kind: "SequenceStart".to_string(),
            anchor: anchor.as_ref().map(|a| a.to_string()),
            tag: tag.as_ref().map(|t| t.to_string()),
            value: None,
            implicit: None,
        }),
        yamalgam_parser::Event::SequenceEnd { .. } => Some(EventSnapshot {
            kind: "SequenceEnd".to_string(),
            anchor: None,
            tag: None,
            value: None,
            implicit: None,
        }),
        yamalgam_parser::Event::MappingStart { anchor, tag, .. } => Some(EventSnapshot {
            kind: "MappingStart".to_string(),
            anchor: anchor.as_ref().map(|a| a.to_string()),
            tag: tag.as_ref().map(|t| t.to_string()),
            value: None,
            implicit: None,
        }),
        yamalgam_parser::Event::MappingEnd { .. } => Some(EventSnapshot {
            kind: "MappingEnd".to_string(),
            anchor: None,
            tag: None,
            value: None,
            implicit: None,
        }),
        yamalgam_parser::Event::Scalar { anchor, tag, value, .. } => Some(EventSnapshot {
            kind: "Scalar".to_string(),
            anchor: anchor.as_ref().map(|a| a.to_string()),
            tag: tag.as_ref().map(|t| t.to_string()),
            value: Some(value.to_string()),
            implicit: None,
        }),
        yamalgam_parser::Event::Alias { name, .. } => Some(EventSnapshot {
            kind: "Alias".to_string(),
            anchor: None,
            tag: None,
            value: Some(name.to_string()),
            implicit: None,
        }),
    }
}

/// Compare both implementations on the same input (event-level).
///
/// Runs the C harness with `--events` and the Rust parser, then matches on
/// their results to produce a [`CompareEventResult`].
pub fn compare_events(input: &[u8]) -> CompareEventResult {
    let c_result = run_c_events(input);
    let rust_result = run_rust_parser(input);

    match (c_result, rust_result) {
        (Ok(c_events), Ok(rust_events)) => compare_event_streams(&c_events, &rust_events),
        (Err(c_err), Err(rust_err)) => {
            if c_err == rust_err {
                CompareEventResult::BothErrorMatch
            } else {
                CompareEventResult::BothErrorMismatch {
                    c_error: c_err,
                    rust_error: rust_err,
                }
            }
        }
        (Ok(c_events), Err(rust_err)) => CompareEventResult::CSuccessRustError {
            rust_error: rust_err,
            c_event_count: c_events.len(),
        },
        (Err(c_err), Ok(rust_events)) => CompareEventResult::RustSuccessCError {
            c_error: c_err,
            rust_event_count: rust_events.len(),
        },
    }
}

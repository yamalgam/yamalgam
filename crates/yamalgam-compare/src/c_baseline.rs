//! C baseline cache — pre-computed `fyaml-tokenize` results for all test cases.
//!
//! Instead of spawning 700+ subprocesses (one per test × 2 modes), the
//! `generate` function drives a single `fyaml-tokenize --batch` process
//! and caches raw stdout per test ID. The compliance test loads the cache
//! via [`load`] and skips subprocess spawning entirely.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use crate::harness::find_fyaml_binary;

/// A cached C harness result for a single test case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Raw JSON lines from stdout (empty on error).
    pub stdout: String,
    /// Error message if the C harness failed.
    pub error: Option<String>,
}

/// Convert YAML Test Suite visual markers to actual characters.
///
/// Handles: `—` (em-dash), `␣` (open-box), `»` (guillemet), `↵` (return),
/// `∎` (end-of-proof).
fn replace_markers(mut result: String) -> String {
    if result.contains('\u{2014}') {
        result = result.replace('\u{2014}', "");
    }
    if result.contains('\u{2423}') {
        result = result.replace('\u{2423}', " ");
    }
    if result.contains('\u{00BB}') {
        result = result.replace('\u{00BB}', "\t");
    }
    if result.contains('\u{21B5}') {
        result = result.replace('\u{21B5}', "");
    }
    if result.contains('\u{220E}') {
        result = result.replace('\u{220E}', "");
        if result.ends_with('\n') {
            result.pop();
        }
    }
    result
}

/// Try to extract the value after a `yaml:` key in a test-suite line.
///
/// Handles both `  yaml: ...` (indented key) and `- yaml: ...` (first key
/// in an array element). Returns the text after `yaml:` if found.
fn yaml_key_value(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    // Direct key: "yaml: ..."
    if let Some(rest) = trimmed.strip_prefix("yaml:") {
        return Some(rest);
    }
    // Array-element first key: "- yaml: ..."
    if let Some(after_dash) = trimmed.strip_prefix("- ")
        && let Some(rest) = after_dash.trim_start().strip_prefix("yaml:")
    {
        return Some(rest);
    }
    None
}

/// Extract the `yaml:` block from a chunk of lines (a single array element).
///
/// The lines should be the raw content of one array element (possibly
/// including the leading `- ` prefix on the first line).
fn extract_yaml_from_lines(lines: &[&str]) -> Option<String> {
    let mut in_yaml_block = false;
    let mut indent: Option<usize> = None;
    let mut yaml_lines = Vec::new();

    for line in lines {
        if in_yaml_block {
            if let Some(min_indent) = indent {
                let stripped = line.trim_start();
                let current_indent = line.len() - stripped.len();

                if !line.trim().is_empty() && current_indent < min_indent {
                    break;
                }

                if line.len() >= min_indent {
                    yaml_lines.push(&line[min_indent..]);
                } else if line.trim().is_empty() {
                    yaml_lines.push("");
                } else {
                    break;
                }
            } else if !line.trim().is_empty() {
                let stripped = line.trim_start();
                let current_indent = line.len() - stripped.len();
                indent = Some(current_indent);
                yaml_lines.push(&line[current_indent..]);
            }
        } else if let Some(after_key) = yaml_key_value(line) {
            let after_key = after_key.trim();
            if after_key.is_empty() || after_key == "|" || after_key == "|2" || after_key == "|-" {
                in_yaml_block = true;
            } else {
                return Some(replace_markers(after_key.to_string()));
            }
        }
    }

    if yaml_lines.is_empty() {
        return None;
    }

    Some(replace_markers(yaml_lines.join("\n")))
}

/// Extract the raw YAML input from a YAML Test Suite file.
///
/// Each test suite file is itself YAML: a sequence of test case maps.
/// We extract the `yaml` field from the first test case using simple
/// line-based parsing (no YAML parser dependency).
pub fn extract_yaml_input(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    extract_yaml_from_lines(&lines)
}

/// Extract ALL `yaml:` blocks from a YAML Test Suite file.
///
/// Single-case files return one entry `(0, yaml_input)`.
/// Multi-case files (YAML arrays with multiple `- ` elements at column 0)
/// return `(0, ...), (1, ...), ...` for each element that has a `yaml:` field.
pub fn extract_all_yaml_inputs(content: &str) -> Vec<(usize, String)> {
    let lines: Vec<&str> = content.lines().collect();

    // Split into array elements. Each element starts with `- ` at column 0.
    // Skip a leading `---` line if present.
    let mut elements: Vec<Vec<&str>> = Vec::new();
    let mut current: Vec<&str> = Vec::new();

    for &line in &lines {
        // Skip document start marker at the very beginning.
        if elements.is_empty() && current.is_empty() && line.trim() == "---" {
            continue;
        }

        // A new array element starts with `- ` at column 0.
        if line.starts_with("- ") && !current.is_empty() {
            elements.push(std::mem::take(&mut current));
        }

        current.push(line);
    }
    if !current.is_empty() {
        elements.push(current);
    }

    // If there's only one element, it's a single-case file.
    // Extract yaml from each element.
    let mut results = Vec::new();
    for (idx, elem_lines) in elements.iter().enumerate() {
        if let Some(yaml) = extract_yaml_from_lines(elem_lines) {
            results.push((idx, yaml));
        }
    }

    results
}

/// Run all test cases through a single `fyaml-tokenize --batch` process
/// and return cached results keyed by test ID.
///
/// `test_cases` is a vec of `(id, yaml_bytes)` pairs.
fn run_batch(
    test_cases: &[(String, Vec<u8>)],
    events: bool,
) -> Result<HashMap<String, CacheEntry>, String> {
    let binary =
        find_fyaml_binary().ok_or_else(|| "fyaml-tokenize binary not found".to_string())?;

    let mut cmd = Command::new(&binary);
    cmd.arg("--batch");
    if events {
        cmd.arg("--events");
    }
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn {}: {e}", binary.display()))?;

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    let mut results = HashMap::new();

    for (id, yaml) in test_cases {
        // Write length-prefixed input.
        writeln!(stdin, "{}", yaml.len()).map_err(|e| format!("write length for {id}: {e}"))?;
        stdin
            .write_all(yaml)
            .map_err(|e| format!("write input for {id}: {e}"))?;
        stdin.flush().map_err(|e| format!("flush for {id}: {e}"))?;

        // Read JSON lines until ---END sentinel.
        let mut stdout_lines = String::new();
        let mut error = None;
        loop {
            let mut line = String::new();
            let n = reader
                .read_line(&mut line)
                .map_err(|e| format!("read for {id}: {e}"))?;
            if n == 0 {
                return Err(format!("unexpected EOF reading result for {id}"));
            }
            if line.trim() == "---END" {
                break;
            }
            // Check if this line is an error JSON object.
            if line.contains("\"error\"")
                && let Ok(obj) = serde_json::from_str::<serde_json::Value>(line.trim())
                && let Some(msg) = obj.get("error").and_then(|v| v.as_str())
            {
                error = Some(msg.to_string());
                continue;
            }
            stdout_lines.push_str(&line);
        }

        results.insert(
            id.clone(),
            CacheEntry {
                stdout: stdout_lines,
                error,
            },
        );
    }

    drop(stdin);
    let _ = child.wait();
    Ok(results)
}

/// Generate baseline cache files for both token and event modes.
///
/// Reads all `.yaml` files from `test_dir`, extracts YAML inputs, runs
/// them through batch C processes, and writes JSON cache files.
pub fn generate(test_dir: &Path, output_dir: &Path) -> Result<(), String> {
    // Collect test cases.
    let mut test_cases: Vec<(String, Vec<u8>)> = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(test_dir)
        .map_err(|e| format!("read test dir: {e}"))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "yaml"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let path = entry.path();
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let content =
            std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let cases = extract_all_yaml_inputs(&content);
        let multi = cases.len() > 1;
        for (idx, yaml) in cases {
            let id = if multi {
                format!("{stem}#{idx}")
            } else {
                stem.clone()
            };
            test_cases.push((id, yaml.into_bytes()));
        }
    }

    eprintln!(
        "Generating token baseline for {} test cases...",
        test_cases.len()
    );
    let token_results = run_batch(&test_cases, false)?;
    let token_path = output_dir.join("c-baseline-tokens.json");
    let token_json =
        serde_json::to_string(&token_results).map_err(|e| format!("serialize tokens: {e}"))?;
    std::fs::write(&token_path, &token_json)
        .map_err(|e| format!("write {}: {e}", token_path.display()))?;
    eprintln!("Wrote {}", token_path.display());

    eprintln!(
        "Generating event baseline for {} test cases...",
        test_cases.len()
    );
    let event_results = run_batch(&test_cases, true)?;
    let event_path = output_dir.join("c-baseline-events.json");
    let event_json =
        serde_json::to_string(&event_results).map_err(|e| format!("serialize events: {e}"))?;
    std::fs::write(&event_path, &event_json)
        .map_err(|e| format!("write {}: {e}", event_path.display()))?;
    eprintln!("Wrote {}", event_path.display());

    Ok(())
}

/// Load a baseline cache from a JSON file.
///
/// Returns `None` if the file doesn't exist (fallback to subprocess mode).
pub fn load(path: &Path) -> Option<HashMap<String, CacheEntry>> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

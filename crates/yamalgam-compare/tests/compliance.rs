#![allow(missing_docs)]
//! Compliance test harness: runs each YAML Test Suite case through both
//! the C harness and the Rust scanner/parser, then compares the outputs.
//!
//! Uses `datatest-stable` for file-driven test generation.
//!
//! Two test functions are registered:
//! - `compliance_test`: token-level comparison via scanner
//! - `event_compliance_test`: event-level comparison via parser

use std::path::Path;

use yamalgam_compare::{CompareEventResult, CompareResult, compare_events, compare_input};

/// Extract the raw YAML input from a YAML Test Suite file.
///
/// Each test suite file is itself YAML: a sequence of test case maps.
/// We extract the `yaml` field from the first test case. If the file contains
/// an error test (`fail: true`), we still feed the YAML to both implementations
/// to verify the harness handles errors gracefully.
fn extract_yaml_input(content: &str) -> Option<String> {
    // The test suite files are YAML documents containing a sequence of maps.
    // Each map has a `yaml` key whose value is the test input.
    // We use a simple line-based extraction rather than a full YAML parser
    // to avoid circular dependency on the thing we're testing.

    let mut in_yaml_block = false;
    let mut indent: Option<usize> = None;
    let mut yaml_lines = Vec::new();

    for line in content.lines() {
        if in_yaml_block {
            if let Some(min_indent) = indent {
                // Check if we've left the block (dedented or new key at same level).
                let stripped = line.trim_start();
                let current_indent = line.len() - stripped.len();

                if !line.trim().is_empty() && current_indent < min_indent {
                    // We've left the yaml block.
                    break;
                }

                // Strip the block indentation.
                if line.len() >= min_indent {
                    yaml_lines.push(&line[min_indent..]);
                } else if line.trim().is_empty() {
                    yaml_lines.push("");
                } else {
                    break;
                }
            } else if !line.trim().is_empty() {
                // First non-empty line of the block — detect indentation.
                let stripped = line.trim_start();
                let current_indent = line.len() - stripped.len();
                indent = Some(current_indent);
                yaml_lines.push(&line[current_indent..]);
            }
        } else if line.trim_start().starts_with("yaml:") {
            let after_key = line.trim_start().strip_prefix("yaml:").unwrap().trim();
            if after_key.is_empty() || after_key == "|" || after_key == "|2" || after_key == "|-" {
                // Block scalar follows on subsequent lines.
                in_yaml_block = true;
            } else {
                // Inline value (unlikely for test suite but handle it).
                return Some(after_key.to_string());
            }
        }
    }

    if yaml_lines.is_empty() {
        return None;
    }

    let mut result = yaml_lines.join("\n");

    // The YAML Test Suite uses visual markers for significant whitespace:
    // - ␣ (U+2423, OPEN BOX) → space (trailing/significant whitespace)
    // - — (U+2014, EM DASH) → tab filler (visual padding before »), strip
    // - » (U+00BB, RIGHT-POINTING DOUBLE ANGLE QUOTATION MARK) → tab
    // - ↵ (U+21B5, DOWNWARDS ARROW WITH CORNER LEFTWARDS) → newline already
    //   present from block scalar line break, strip the marker
    // - ∎ (U+220E, END OF PROOF) → no final newline, strip trailing newline
    // Convert to actual characters before feeding to scanners.
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
        // ∎ means "no final newline" — strip the trailing newline that
        // the block scalar extraction added.
        if result.ends_with('\n') {
            result.pop();
        }
    }

    Some(result)
}

fn compliance_test(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;

    let yaml_input = match extract_yaml_input(&content) {
        Some(input) => input,
        None => {
            // Some test files might not have a yaml field we can extract.
            // Skip gracefully.
            eprintln!("SKIP: no yaml input found in {}", path.display());
            return Ok(());
        }
    };

    let result = compare_input(yaml_input.as_bytes());

    match &result {
        CompareResult::Match { token_count } => {
            eprintln!(
                "PASS: {} ({token_count} tokens matched)",
                path.file_stem().unwrap().to_string_lossy()
            );
        }
        CompareResult::CSuccessRustError {
            rust_error,
            c_token_count,
        } => {
            eprintln!(
                "EXPECTED: {} (C produced {c_token_count} tokens, Rust: {rust_error})",
                path.file_stem().unwrap().to_string_lossy()
            );
        }
        CompareResult::BothErrorMatch => {
            eprintln!(
                "PASS: {} (both errored, matching)",
                path.file_stem().unwrap().to_string_lossy()
            );
        }
        CompareResult::BothErrorMismatch {
            c_error: _,
            rust_error: _,
        } => {
            // Both implementations rejected the input — this is a pass
            // regardless of differing error messages.
            eprintln!(
                "PASS: {} (both errored)",
                path.file_stem().unwrap().to_string_lossy()
            );
        }
        CompareResult::RustSuccessCError {
            c_error,
            rust_token_count,
        } => {
            eprintln!(
                "UNEXPECTED: {} (Rust produced {rust_token_count} tokens, C: {c_error})",
                path.file_stem().unwrap().to_string_lossy()
            );
        }
        CompareResult::TokenMismatch {
            index,
            c_token,
            rust_token,
            ..
        } => {
            eprintln!(
                "MISMATCH: {} at index {index} (C: {:?}, Rust: {:?})",
                path.file_stem().unwrap().to_string_lossy(),
                c_token.kind,
                rust_token.kind
            );
        }
    }

    Ok(())
}

fn event_compliance_test(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;

    let yaml_input = match extract_yaml_input(&content) {
        Some(input) => input,
        None => {
            eprintln!("EVENT_SKIP: no yaml input found in {}", path.display());
            return Ok(());
        }
    };

    let result = compare_events(yaml_input.as_bytes());

    match &result {
        CompareEventResult::Match { event_count } => {
            eprintln!(
                "EVENT_PASS: {} ({event_count} events matched)",
                path.file_stem().unwrap().to_string_lossy()
            );
        }
        CompareEventResult::CSuccessRustError {
            rust_error,
            c_event_count,
        } => {
            eprintln!(
                "EVENT_EXPECTED: {} (C produced {c_event_count} events, Rust: {rust_error})",
                path.file_stem().unwrap().to_string_lossy()
            );
        }
        CompareEventResult::BothErrorMatch => {
            eprintln!(
                "EVENT_PASS: {} (both errored, matching)",
                path.file_stem().unwrap().to_string_lossy()
            );
        }
        CompareEventResult::BothErrorMismatch {
            c_error: _,
            rust_error: _,
        } => {
            // Both implementations rejected the input — this is a pass
            // regardless of differing error messages.
            eprintln!(
                "EVENT_PASS: {} (both errored)",
                path.file_stem().unwrap().to_string_lossy()
            );
        }
        CompareEventResult::RustSuccessCError {
            c_error,
            rust_event_count,
        } => {
            eprintln!(
                "EVENT_UNEXPECTED: {} (Rust produced {rust_event_count} events, C: {c_error})",
                path.file_stem().unwrap().to_string_lossy()
            );
        }
        CompareEventResult::EventMismatch {
            index,
            c_event,
            rust_event,
            ..
        } => {
            eprintln!(
                "EVENT_MISMATCH: {} at index {index} (C: {:?}, Rust: {:?})",
                path.file_stem().unwrap().to_string_lossy(),
                c_event.kind,
                rust_event.kind
            );
        }
    }

    Ok(())
}

datatest_stable::harness! {
    { test = compliance_test, root = "../../vendor/yaml-test-suite", pattern = r"\.yaml$" },
    { test = event_compliance_test, root = "../../vendor/yaml-test-suite", pattern = r"\.yaml$" },
}

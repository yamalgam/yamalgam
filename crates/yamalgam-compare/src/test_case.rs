//! Test case extractor for YAML Test Suite files.
//!
//! Parses the `.yaml` files in `vendor/yaml-test-suite/` using line-based
//! extraction (no YAML parser — avoids bootstrapping). Extracts `yaml:`,
//! `tree:`, and `fail:` fields from each test case element.

/// A single test case extracted from a YAML Test Suite file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCase {
    /// 0-based index within the file.
    pub index: usize,
    /// Raw YAML input (markers replaced).
    pub yaml: String,
    /// Expected event tree, if present (markers replaced).
    pub tree: Option<String>,
    /// Whether this case is expected to fail.
    pub fail: bool,
}

/// Convert YAML Test Suite visual markers to actual characters.
///
/// Handles: `—` (em-dash → removed), `␣` (open-box → space),
/// `»` (guillemet → tab), `↵` (return → removed),
/// `∎` (end-of-proof → removed + strip trailing newline).
fn replace_markers(mut s: String) -> String {
    // Em-dash: U+2014 → removed
    if s.contains('\u{2014}') {
        s = s.replace('\u{2014}', "");
    }
    // Open-box: U+2423 → space
    if s.contains('\u{2423}') {
        s = s.replace('\u{2423}', " ");
    }
    // Guillemet: U+00BB → tab
    if s.contains('\u{00BB}') {
        s = s.replace('\u{00BB}', "\t");
    }
    // Return arrow: U+21B5 → removed
    if s.contains('\u{21B5}') {
        s = s.replace('\u{21B5}', "");
    }
    // End-of-proof: U+220E → removed, strip trailing newline
    if s.contains('\u{220E}') {
        s = s.replace('\u{220E}', "");
        if s.ends_with('\n') {
            s.pop();
        }
    }
    s
}

/// Try to extract the value portion after a known key in a line.
///
/// Handles both `  key: ...` (indented) and `- key: ...` (array element first key).
/// Returns the text after `key:` if matched.
fn key_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let trimmed = line.trim_start();
    // Direct key: "key: ..." or "key:" at end of line
    if let Some(rest) = trimmed.strip_prefix(key).and_then(|r| r.strip_prefix(':')) {
        return Some(rest);
    }
    // Array-element first key: "- key: ..."
    if let Some(rest) = trimmed
        .strip_prefix("- ")
        .map(str::trim_start)
        .and_then(|s| s.strip_prefix(key))
        .and_then(|r| r.strip_prefix(':'))
    {
        return Some(rest);
    }
    None
}

/// Extract a block scalar field from the lines of an element.
///
/// Starts looking at `start_idx`, returns the extracted text and the
/// line index where the block ended (so the caller can skip past it).
///
/// When `block_header` contains an explicit indent indicator (e.g., `|2`),
/// the content indent is computed as `key_line_indent + indicator_value`.
/// Otherwise, indent is auto-detected from the first non-empty content line.
fn extract_block_field(
    lines: &[&str],
    start_idx: usize,
    block_header: &str,
    key_line: &str,
) -> (String, usize) {
    let mut content_lines = Vec::new();
    let mut indent: Option<usize> = None;
    let mut idx = start_idx;

    // Try to extract explicit indent indicator from block header.
    // Headers like "|2", "|2-", "|-", ">+", ">2+" etc.
    // The digit, if present, is the indent indicator.
    let explicit_indent = extract_indent_indicator(block_header, key_line);
    if let Some(ind) = explicit_indent {
        indent = Some(ind);
    }

    while idx < lines.len() {
        let line = lines[idx];

        if let Some(min_indent) = indent {
            let stripped = line.trim_start();
            let current_indent = line.len() - stripped.len();

            if !line.trim().is_empty() && current_indent < min_indent {
                break;
            }

            if line.len() >= min_indent {
                content_lines.push(&line[min_indent..]);
            } else if line.trim().is_empty() {
                content_lines.push("");
            } else {
                break;
            }
        } else if !line.trim().is_empty() {
            // First non-empty content line — determine indent.
            let stripped = line.trim_start();
            let current_indent = line.len() - stripped.len();
            indent = Some(current_indent);
            content_lines.push(&line[current_indent..]);
        }

        idx += 1;
    }

    (content_lines.join("\n"), idx)
}

/// Extract explicit indent level from a block scalar header.
///
/// Given a header like `|2` and the key line, returns the absolute column
/// where content starts: `key_column + indicator_value`.
///
/// The key column accounts for array-element syntax (`- key: |2`) where
/// the key is at column 2 despite the line starting at column 0.
fn extract_indent_indicator(header: &str, key_line: &str) -> Option<usize> {
    // Find the digit in the header (e.g., "2" in "|2" or "|2-").
    let digit = header.chars().find(|c| c.is_ascii_digit())?;
    let indicator: usize = (digit as u8 - b'0') as usize;
    if indicator == 0 {
        return None;
    }

    // Find the key's effective column: strip leading whitespace and "- " prefix.
    let leading_spaces = key_line.len() - key_line.trim_start().len();
    let trimmed = key_line.trim_start();
    let key_column = if trimmed.starts_with("- ") {
        // Array element: key starts after "- " prefix.
        leading_spaces + 2
    } else {
        leading_spaces
    };

    Some(key_column + indicator)
}

/// Check whether a line indicates `fail: true` for the element.
fn is_fail_line(line: &str) -> bool {
    key_value(line, "fail").is_some_and(|rest| rest.trim() == "true")
}

/// Extract all test cases from a YAML Test Suite file's content.
///
/// The file is a YAML sequence of maps. We split on `- ` at column 0 to
/// find element boundaries, then extract `yaml:`, `tree:`, and `fail:`
/// fields from each element.
pub fn extract_test_cases(content: &str) -> Vec<TestCase> {
    let lines: Vec<&str> = content.lines().collect();

    // Split into array elements at `- ` at column 0.
    let mut elements: Vec<Vec<&str>> = Vec::new();
    let mut current: Vec<&str> = Vec::new();

    for &line in &lines {
        // Skip document start marker at the beginning.
        if elements.is_empty() && current.is_empty() && line.trim() == "---" {
            continue;
        }

        // New array element starts with `- ` at column 0.
        if line.starts_with("- ") && !current.is_empty() {
            elements.push(std::mem::take(&mut current));
        }

        current.push(line);
    }
    if !current.is_empty() {
        elements.push(current);
    }

    let mut results = Vec::new();

    for (idx, elem_lines) in elements.iter().enumerate() {
        let mut yaml: Option<String> = None;
        let mut tree: Option<String> = None;
        let mut fail = false;
        let mut i = 0;

        while i < elem_lines.len() {
            let line = elem_lines[i];

            // Check for fail: true
            if is_fail_line(line) {
                fail = true;
                i += 1;
                continue;
            }

            // Check for yaml: field
            if let Some(after_key) = key_value(line, "yaml") {
                let after_key = after_key.trim();
                if after_key.is_empty() || after_key.starts_with('|') || after_key.starts_with("|-")
                {
                    // Block scalar — extract from subsequent lines.
                    let (text, end_idx) = extract_block_field(elem_lines, i + 1, after_key, line);
                    yaml = Some(replace_markers(text));
                    i = end_idx;
                } else {
                    // Inline value.
                    yaml = Some(replace_markers(after_key.to_string()));
                    i += 1;
                }
                continue;
            }

            // Check for tree: field
            if let Some(after_key) = key_value(line, "tree") {
                let after_key = after_key.trim();
                if after_key.is_empty() || after_key.starts_with('|') || after_key.starts_with("|-")
                {
                    let (text, end_idx) = extract_block_field(elem_lines, i + 1, after_key, line);
                    tree = Some(replace_markers(text));
                    i = end_idx;
                } else {
                    tree = Some(replace_markers(after_key.to_string()));
                    i += 1;
                }
                continue;
            }

            i += 1;
        }

        // Only emit a TestCase if we found a yaml: field.
        if let Some(yaml) = yaml {
            results.push(TestCase {
                index: idx,
                yaml,
                tree,
                fail,
            });
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_markers_em_dash() {
        assert_eq!(replace_markers("a\u{2014}b".to_string()), "ab");
    }

    #[test]
    fn replace_markers_open_box() {
        assert_eq!(replace_markers("a\u{2423}b".to_string()), "a b");
    }

    #[test]
    fn replace_markers_guillemet() {
        assert_eq!(replace_markers("a\u{00BB}b".to_string()), "a\tb");
    }

    #[test]
    fn replace_markers_end_of_proof_strips_newline() {
        assert_eq!(replace_markers("hello\u{220E}\n".to_string()), "hello");
    }

    #[test]
    fn replace_markers_no_markers_passthrough() {
        assert_eq!(replace_markers("plain text".to_string()), "plain text");
    }
}

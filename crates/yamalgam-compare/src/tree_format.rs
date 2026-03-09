//! Parser for the YAML Test Suite tree event format.
//!
//! The tree format is a line-oriented text representation used by the
//! [YAML Test Suite](https://github.com/yaml/yaml-test-suite) to describe
//! expected event streams. Each line (after stripping leading whitespace)
//! encodes one event.
//!
//! # Format summary
//!
//! ```text
//! +STR              → StreamStart
//! +DOC              → DocumentStart (implicit)
//! +DOC ---          → DocumentStart (explicit)
//! -DOC              → DocumentEnd (implicit)
//! -DOC ...          → DocumentEnd (explicit)
//! +SEQ              → SequenceStart
//! +SEQ &anchor      → SequenceStart with anchor
//! +SEQ <tag>        → SequenceStart with tag
//! -SEQ              → SequenceEnd
//! +MAP              → MappingStart
//! -MAP              → MappingEnd
//! =VAL :text        → Scalar (plain)
//! =VAL 'text        → Scalar (single-quoted)
//! =VAL "text        → Scalar (double-quoted)
//! =VAL |text        → Scalar (literal block)
//! =VAL >text        → Scalar (folded block)
//! =ALI *name        → Alias
//! -STR              → StreamEnd
//! ```
//!
//! Anchors appear as `&name` after the event keyword. Tags as `<tag:uri>`.
//! Both can coexist (anchor before tag).
//!
//! Value escapes: `\\` → `\`, `\n` → newline, `\t` → tab, `\r` → CR,
//! `\b` → backspace, `\0` → null, `\x00` → hex byte.

use crate::EventSnapshot;

/// Parse a YAML Test Suite tree-format string into a sequence of [`EventSnapshot`]s.
///
/// Blank lines and leading/trailing whitespace on each line are ignored.
/// The indentation in tree format is purely cosmetic and does not affect parsing.
pub fn parse_tree(tree: &str) -> Vec<EventSnapshot> {
    let mut events = Vec::new();

    for line in tree.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(event) = parse_tree_line(trimmed) {
            events.push(event);
        }
    }

    events
}

/// Parse a single trimmed tree-format line into an [`EventSnapshot`].
#[allow(clippy::option_if_let_else)]
fn parse_tree_line(line: &str) -> Option<EventSnapshot> {
    if let Some(rest) = line.strip_prefix("+STR") {
        debug_assert!(
            rest.trim().is_empty(),
            "unexpected content after +STR: {rest}"
        );
        Some(EventSnapshot {
            kind: "StreamStart".to_string(),
            anchor: None,
            tag: None,
            value: None,
            implicit: None,
        })
    } else if let Some(rest) = line.strip_prefix("-STR") {
        debug_assert!(
            rest.trim().is_empty(),
            "unexpected content after -STR: {rest}"
        );
        Some(EventSnapshot {
            kind: "StreamEnd".to_string(),
            anchor: None,
            tag: None,
            value: None,
            implicit: None,
        })
    } else if let Some(rest) = line.strip_prefix("+DOC") {
        let rest = rest.trim();
        let implicit = !rest.contains("---");
        Some(EventSnapshot {
            kind: "DocumentStart".to_string(),
            anchor: None,
            tag: None,
            value: None,
            implicit: Some(implicit),
        })
    } else if let Some(rest) = line.strip_prefix("-DOC") {
        let rest = rest.trim();
        let implicit = !rest.contains("...");
        Some(EventSnapshot {
            kind: "DocumentEnd".to_string(),
            anchor: None,
            tag: None,
            value: None,
            implicit: Some(implicit),
        })
    } else if let Some(rest) = line.strip_prefix("+SEQ") {
        let (anchor, tag) = parse_anchor_tag(rest.trim());
        Some(EventSnapshot {
            kind: "SequenceStart".to_string(),
            anchor,
            tag,
            value: None,
            implicit: None,
        })
    } else if let Some(rest) = line.strip_prefix("-SEQ") {
        debug_assert!(
            rest.trim().is_empty(),
            "unexpected content after -SEQ: {rest}"
        );
        Some(EventSnapshot {
            kind: "SequenceEnd".to_string(),
            anchor: None,
            tag: None,
            value: None,
            implicit: None,
        })
    } else if let Some(rest) = line.strip_prefix("+MAP") {
        let (anchor, tag) = parse_anchor_tag(rest.trim());
        Some(EventSnapshot {
            kind: "MappingStart".to_string(),
            anchor,
            tag,
            value: None,
            implicit: None,
        })
    } else if let Some(rest) = line.strip_prefix("-MAP") {
        debug_assert!(
            rest.trim().is_empty(),
            "unexpected content after -MAP: {rest}"
        );
        Some(EventSnapshot {
            kind: "MappingEnd".to_string(),
            anchor: None,
            tag: None,
            value: None,
            implicit: None,
        })
    } else if let Some(rest) = line.strip_prefix("=VAL") {
        parse_val(rest.trim_start_matches(' '))
    } else if let Some(rest) = line.strip_prefix("=ALI") {
        parse_alias(rest.trim())
    } else {
        None
    }
}

/// Parse anchor (`&name`) and tag (`<uri>`) from the property portion of a line.
///
/// Expected input is the trimmed remainder after the event keyword (e.g., after `+SEQ`).
/// Returns `(Option<anchor>, Option<tag>)`.
///
/// The tree format may include flow indicators (`[]` for sequences, `{}` for mappings)
/// before the anchor/tag properties. These are skipped.
fn parse_anchor_tag(props: &str) -> (Option<String>, Option<String>) {
    if props.is_empty() {
        return (None, None);
    }

    let mut anchor = None;
    let mut tag = None;
    let mut remaining = props;

    // Skip flow indicators ([] or {}) that appear before anchor/tag.
    if remaining.starts_with("[]") || remaining.starts_with("{}") {
        remaining = remaining[2..].trim_start();
    }

    if remaining.is_empty() {
        return (None, None);
    }

    // Anchor comes first if present.
    if remaining.starts_with('&') {
        remaining = &remaining[1..];
        // Anchor name ends at space or '<' (tag start) or end of string.
        let end = remaining.find([' ', '<']).unwrap_or(remaining.len());
        anchor = Some(remaining[..end].to_string());
        remaining = remaining[end..].trim_start();
    }

    // Tag comes next if present.
    if remaining.starts_with('<')
        && let Some(close) = remaining.find('>')
    {
        tag = Some(remaining[1..close].to_string());
    }

    (anchor, tag)
}

/// Parse a `=VAL` line remainder into a Scalar [`EventSnapshot`].
///
/// The remainder has the form: `[&anchor] [<tag>] STYLE_CHAR value`
/// where `STYLE_CHAR` is one of `:`, `'`, `"`, `|`, `>`.
fn parse_val(rest: &str) -> Option<EventSnapshot> {
    let (anchor, tag, after_props) = parse_val_properties(rest);

    // The next character is the style indicator, followed by the raw value.
    if after_props.is_empty() {
        return None;
    }

    let value = &after_props[1..]; // everything after the style char
    let decoded = unescape_tree_value(value);

    Some(EventSnapshot {
        kind: "Scalar".to_string(),
        anchor,
        tag,
        value: Some(decoded),
        implicit: None,
    })
}

/// Extract anchor, tag, and the remaining string from a `=VAL` line's content.
///
/// Returns `(anchor, tag, rest_after_properties)`.
fn parse_val_properties(input: &str) -> (Option<String>, Option<String>, &str) {
    let mut anchor = None;
    let mut tag = None;
    let mut remaining = input;

    // Anchor comes first if present.
    if remaining.starts_with('&') {
        remaining = &remaining[1..];
        // Anchor name ends at space.
        let end = remaining.find(' ').unwrap_or(remaining.len());
        anchor = Some(remaining[..end].to_string());
        if end < remaining.len() {
            remaining = &remaining[end + 1..];
        } else {
            remaining = "";
        }
    }

    // Tag comes next if present.
    if remaining.starts_with('<')
        && let Some(close) = remaining.find('>')
    {
        tag = Some(remaining[1..close].to_string());
        remaining = &remaining[close + 1..];
        if remaining.starts_with(' ') {
            remaining = &remaining[1..];
        }
    }

    (anchor, tag, remaining)
}

/// Parse a `=ALI` line remainder into an Alias [`EventSnapshot`].
///
/// The remainder has the form: `*name`.
fn parse_alias(rest: &str) -> Option<EventSnapshot> {
    let name = rest.strip_prefix('*')?;

    Some(EventSnapshot {
        kind: "Alias".to_string(),
        anchor: None,
        tag: None,
        value: Some(name.to_string()),
        implicit: None,
    })
}

/// Unescape a tree-format value string.
///
/// Tree format uses these escape sequences:
/// - `\\` → `\`
/// - `\n` → newline (U+000A)
/// - `\t` → tab (U+0009)
/// - `\r` → carriage return (U+000D)
/// - `\b` → backspace (U+0008)
/// - `\0` → null (U+0000)
/// - `\xHH` → byte value from two hex digits
fn unescape_tree_value(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('\\') => result.push('\\'),
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('b') => result.push('\u{0008}'),
                Some('0') => result.push('\0'),
                Some('x') => {
                    let hi = chars.next().unwrap_or('0');
                    let lo = chars.next().unwrap_or('0');
                    let mut hex = String::with_capacity(2);
                    hex.push(hi);
                    hex.push(lo);
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                        result.push(char::from(byte));
                    } else {
                        // Malformed hex — pass through literally.
                        result.push('\\');
                        result.push('x');
                        result.push(hi);
                        result.push(lo);
                    }
                }
                Some(other) => {
                    // Unknown escape — pass through literally.
                    result.push('\\');
                    result.push(other);
                }
                None => {
                    // Trailing backslash.
                    result.push('\\');
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unescape_basic() {
        assert_eq!(unescape_tree_value("hello"), "hello");
        assert_eq!(unescape_tree_value("a\\nb"), "a\nb");
        assert_eq!(unescape_tree_value("a\\tb"), "a\tb");
        assert_eq!(unescape_tree_value("a\\\\b"), "a\\b");
        assert_eq!(unescape_tree_value("a\\rb"), "a\rb");
        assert_eq!(unescape_tree_value("a\\bb"), "a\u{0008}b");
        assert_eq!(unescape_tree_value("a\\0b"), "a\0b");
    }

    #[test]
    fn unescape_hex() {
        assert_eq!(unescape_tree_value("\\x41"), "A");
        assert_eq!(unescape_tree_value("\\x00"), "\0");
        assert_eq!(unescape_tree_value("\\x0a"), "\n");
    }

    #[test]
    fn parse_empty_stream() {
        let tree = "+STR\n-STR\n";
        let events = parse_tree(tree);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, "StreamStart");
        assert_eq!(events[1].kind, "StreamEnd");
    }

    #[test]
    fn parse_implicit_doc() {
        let tree = "+STR\n +DOC\n  =VAL :hello\n -DOC\n-STR\n";
        let events = parse_tree(tree);
        assert_eq!(events.len(), 5);
        assert_eq!(events[1].kind, "DocumentStart");
        assert_eq!(events[1].implicit, Some(true));
        assert_eq!(events[3].kind, "DocumentEnd");
        assert_eq!(events[3].implicit, Some(true));
    }

    #[test]
    fn parse_explicit_doc() {
        let tree = "+STR\n +DOC ---\n  =VAL :hello\n -DOC ...\n-STR\n";
        let events = parse_tree(tree);
        assert_eq!(events[1].kind, "DocumentStart");
        assert_eq!(events[1].implicit, Some(false));
        assert_eq!(events[3].kind, "DocumentEnd");
        assert_eq!(events[3].implicit, Some(false));
    }

    #[test]
    fn parse_anchor_and_tag() {
        let tree = "+STR\n +DOC\n  +MAP &a1 <tag:yaml.org,2002:map>\n  -MAP\n -DOC\n-STR\n";
        let events = parse_tree(tree);
        let map_start = &events[2];
        assert_eq!(map_start.kind, "MappingStart");
        assert_eq!(map_start.anchor.as_deref(), Some("a1"));
        assert_eq!(map_start.tag.as_deref(), Some("tag:yaml.org,2002:map"));
    }

    #[test]
    fn parse_alias() {
        let tree = "+STR\n +DOC\n  =ALI *anchor1\n -DOC\n-STR\n";
        let events = parse_tree(tree);
        let alias = &events[2];
        assert_eq!(alias.kind, "Alias");
        assert_eq!(alias.value.as_deref(), Some("anchor1"));
        assert!(alias.anchor.is_none());
    }

    #[test]
    fn parse_scalar_with_anchor_and_tag() {
        let tree = "=VAL &a1 <tag:yaml.org,2002:str> :scalar1\n";
        let events = parse_tree(tree);
        assert_eq!(events.len(), 1);
        let s = &events[0];
        assert_eq!(s.kind, "Scalar");
        assert_eq!(s.anchor.as_deref(), Some("a1"));
        assert_eq!(s.tag.as_deref(), Some("tag:yaml.org,2002:str"));
        assert_eq!(s.value.as_deref(), Some("scalar1"));
    }
}

use arbitrary::{Arbitrary, Result as ArbResult, Unstructured};
use std::fmt::Write;

/// A fuzzable YAML document (possibly multi-document).
#[derive(Debug)]
pub struct YamlDoc {
    pub documents: Vec<YamlNode>,
}

/// Scalar rendering style.
#[derive(Debug, Arbitrary)]
pub enum ScalarStyle {
    Plain,
    SingleQuoted,
    DoubleQuoted,
    Literal,
    Folded,
}

/// A YAML node — scalars, mappings, sequences, anchors, and aliases.
#[derive(Debug)]
pub enum YamlNode {
    Scalar { value: String, style: ScalarStyle },
    Mapping { entries: Vec<(YamlNode, YamlNode)> },
    Sequence { items: Vec<YamlNode>, flow: bool },
    Alias { anchor_ref: u8 },
    Anchored { anchor_id: u8, node: Box<YamlNode> },
}

// Manual Arbitrary impl so we can control recursion depth and keep documents
// small enough to be useful for fuzzing.
impl<'a> Arbitrary<'a> for YamlDoc {
    fn arbitrary(u: &mut Unstructured<'a>) -> ArbResult<Self> {
        let num_docs = u.int_in_range(1..=3)?;
        let mut documents = Vec::with_capacity(num_docs);
        for _ in 0..num_docs {
            documents.push(arbitrary_node(u, 0)?);
        }
        Ok(YamlDoc { documents })
    }
}

impl<'a> Arbitrary<'a> for YamlNode {
    fn arbitrary(u: &mut Unstructured<'a>) -> ArbResult<Self> {
        arbitrary_node(u, 0)
    }
}

const MAX_DEPTH: usize = 6;
const MAX_COLLECTION_LEN: usize = 4;

fn arbitrary_node(u: &mut Unstructured, depth: usize) -> ArbResult<YamlNode> {
    if depth >= MAX_DEPTH {
        // At max depth, only produce scalars or aliases
        return if u.ratio(1, 5)? {
            Ok(YamlNode::Alias {
                anchor_ref: u.int_in_range(0..=3)?,
            })
        } else {
            Ok(YamlNode::Scalar {
                value: arbitrary_scalar_value(u)?,
                style: u.arbitrary()?,
            })
        };
    }

    // Choose node kind. Weight scalars heavily to keep documents manageable.
    let kind = u.int_in_range(0..=9)?;
    match kind {
        0..=4 => {
            // Scalar (50%)
            Ok(YamlNode::Scalar {
                value: arbitrary_scalar_value(u)?,
                style: u.arbitrary()?,
            })
        }
        5..=6 => {
            // Mapping (20%)
            let len = u.int_in_range(0..=MAX_COLLECTION_LEN)?;
            let mut entries = Vec::with_capacity(len);
            for _ in 0..len {
                let key = arbitrary_node(u, depth + 1)?;
                let val = arbitrary_node(u, depth + 1)?;
                entries.push((key, val));
            }
            Ok(YamlNode::Mapping { entries })
        }
        7..=8 => {
            // Sequence (20%)
            let len = u.int_in_range(0..=MAX_COLLECTION_LEN)?;
            let flow: bool = u.arbitrary()?;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                items.push(arbitrary_node(u, depth + 1)?);
            }
            Ok(YamlNode::Sequence { items, flow })
        }
        _ => {
            // Alias (5%) or Anchored (5%)
            if u.arbitrary()? {
                Ok(YamlNode::Alias {
                    anchor_ref: u.int_in_range(0..=3)?,
                })
            } else {
                Ok(YamlNode::Anchored {
                    anchor_id: u.int_in_range(0..=3)?,
                    node: Box::new(arbitrary_node(u, depth + 1)?),
                })
            }
        }
    }
}

/// Generate a scalar value. Keeps it short for fuzzing efficiency.
fn arbitrary_scalar_value(u: &mut Unstructured) -> ArbResult<String> {
    let len = u.int_in_range(0..=32)?;
    let mut s = String::with_capacity(len);
    for _ in 0..len {
        let ch = match u.int_in_range(0..=9)? {
            0 => ' ',
            1 => '\n',
            2 => '\t',
            3 => ':',
            4 => '-',
            5 => '#',
            6 => '\'',
            7 => '"',
            8 => '\\',
            _ => {
                // Alphanumeric
                let idx = u.int_in_range(0..=35)?;
                if idx < 10 {
                    (b'0' + idx as u8) as char
                } else {
                    (b'a' + (idx - 10) as u8) as char
                }
            }
        };
        s.push(ch);
    }
    Ok(s)
}

impl YamlDoc {
    /// Render this document structure as YAML text.
    ///
    /// The output is best-effort — it won't always be valid YAML, which is
    /// exactly what we want for fuzzing.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for (i, doc) in self.documents.iter().enumerate() {
            if i > 0 {
                out.push_str("---\n");
            }
            render_node(doc, &mut out, 0);
            out.push('\n');
        }
        out
    }
}

fn render_node(node: &YamlNode, out: &mut String, indent: usize) {
    match node {
        YamlNode::Scalar { value, style } => render_scalar(value, style, out, indent),
        YamlNode::Mapping { entries } => {
            if entries.is_empty() {
                out.push_str("{}");
                return;
            }
            for (i, (key, val)) in entries.iter().enumerate() {
                if i > 0 {
                    write_indent(out, indent);
                }
                // Keys are always rendered as plain or quoted scalars
                render_mapping_key(key, out, indent);
                out.push_str(": ");
                if needs_newline(val) {
                    out.push('\n');
                    write_indent(out, indent + 2);
                }
                render_node(val, out, indent + 2);
                out.push('\n');
            }
        }
        YamlNode::Sequence { items, flow } => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            if *flow {
                render_flow_sequence(items, out, indent);
            } else {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write_indent(out, indent);
                    }
                    out.push_str("- ");
                    if needs_newline(item) {
                        out.push('\n');
                        write_indent(out, indent + 2);
                    }
                    render_node(item, out, indent + 2);
                    out.push('\n');
                }
            }
        }
        YamlNode::Alias { anchor_ref } => {
            let _ = write!(out, "*a{anchor_ref}");
        }
        YamlNode::Anchored { anchor_id, node } => {
            let _ = write!(out, "&a{anchor_id} ");
            render_node(node, out, indent);
        }
    }
}

/// Render a mapping key — flatten to a simple scalar to keep output parseable.
fn render_mapping_key(node: &YamlNode, out: &mut String, _indent: usize) {
    match node {
        YamlNode::Scalar { value, style } => render_scalar(value, style, out, 0),
        YamlNode::Alias { anchor_ref } => {
            let _ = write!(out, "*a{anchor_ref}");
        }
        YamlNode::Anchored { anchor_id, node } => {
            let _ = write!(out, "&a{anchor_id} ");
            render_mapping_key(node, out, 0);
        }
        // Complex keys: just emit a quoted string representation
        _ => {
            out.push('"');
            out.push_str("complex_key");
            out.push('"');
        }
    }
}

fn render_scalar(value: &str, style: &ScalarStyle, out: &mut String, indent: usize) {
    match style {
        ScalarStyle::Plain => {
            // Strip chars that would break plain scalars
            let safe: String = value
                .chars()
                .filter(|c| !matches!(c, '\n' | '\r' | '#' | ':' | '[' | ']' | '{' | '}' | ','))
                .collect();
            if safe.is_empty() {
                out.push_str("\"\"");
            } else {
                out.push_str(&safe);
            }
        }
        ScalarStyle::SingleQuoted => {
            out.push('\'');
            for ch in value.chars() {
                if ch == '\'' {
                    out.push_str("''");
                } else if ch == '\n' {
                    // Single-quoted scalars can have newlines but they fold
                    out.push('\n');
                } else {
                    out.push(ch);
                }
            }
            out.push('\'');
        }
        ScalarStyle::DoubleQuoted => {
            out.push('"');
            for ch in value.chars() {
                match ch {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\t' => out.push_str("\\t"),
                    _ => out.push(ch),
                }
            }
            out.push('"');
        }
        ScalarStyle::Literal => {
            out.push_str("|\n");
            for line in value.split('\n') {
                write_indent(out, indent + 2);
                out.push_str(line);
                out.push('\n');
            }
        }
        ScalarStyle::Folded => {
            out.push_str(">\n");
            for line in value.split('\n') {
                write_indent(out, indent + 2);
                out.push_str(line);
                out.push('\n');
            }
        }
    }
}

fn render_flow_sequence(items: &[YamlNode], out: &mut String, indent: usize) {
    out.push('[');
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        render_node(item, out, indent);
    }
    out.push(']');
}

/// Returns true if the node should be preceded by a newline when used as a
/// mapping value or sequence item (i.e., it's a block collection).
fn needs_newline(node: &YamlNode) -> bool {
    matches!(
        node,
        YamlNode::Mapping { entries } if !entries.is_empty()
    ) || matches!(
        node,
        YamlNode::Sequence { items, flow } if !items.is_empty() && !flow
    )
}

fn write_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push(' ');
    }
}

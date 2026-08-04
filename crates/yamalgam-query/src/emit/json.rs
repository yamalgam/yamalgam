//! JSON output serialization.

use std::collections::HashSet;

use crate::emit::{EmitError, EmitFormat, EmitOpts, validate_value};
use crate::value::{Data, YamlVal};

/// Render a value as JSON, discarding YAML presentation metadata.
// r[impl output.json]
pub fn to_json(value: &YamlVal, options: &EmitOpts) -> Result<String, EmitError> {
    validate_value(value, options, EmitFormat::Json)?;
    let mut output = String::new();
    write_json(&mut output, value, options, 0)?;
    options
        .limits
        .check_output_size(output.len())
        .map_err(EmitError::Limit)?;
    Ok(output)
}

fn write_json(
    output: &mut String,
    value: &YamlVal,
    options: &EmitOpts,
    depth: usize,
) -> Result<(), EmitError> {
    match &value.data {
        Data::Null => output.push_str("null"),
        Data::Bool(boolean) => output.push_str(if *boolean { "true" } else { "false" }),
        Data::Int(integer) => output.push_str(&integer.to_string()),
        Data::Float(float) if !float.is_finite() => output.push_str("null"),
        Data::Float(float) => output.push_str(&format!("{float:?}")),
        Data::Str(string) => output.push_str(&json_quote(string)),
        Data::Seq(items) if items.is_empty() => output.push_str("[]"),
        Data::Seq(items) => {
            output.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_indent(output, options, depth + 1);
                write_json(output, item, options, depth + 1)?;
            }
            write_closing_indent(output, options, depth);
            output.push(']');
        }
        Data::Map(mapping) if mapping.0.is_empty() => output.push_str("{}"),
        Data::Map(mapping) => {
            let mut entries = mapping.0.iter().collect::<Vec<_>>();
            if options.sort_keys {
                entries.sort_by(|left, right| left.0.cmp(right.0));
            }

            let mut keys = HashSet::with_capacity(entries.len());
            output.push('{');
            for (index, (key, value)) in entries.into_iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_indent(output, options, depth + 1);
                let key = key_string(key);
                if !keys.insert(key.clone()) {
                    return Err(EmitError::DuplicateJsonKey(key));
                }
                output.push_str(&json_quote(&key));
                output.push(':');
                if !options.compact {
                    output.push(' ');
                }
                write_json(output, value, options, depth + 1)?;
            }
            write_closing_indent(output, options, depth);
            output.push('}');
        }
    }
    Ok(())
}

fn write_indent(output: &mut String, options: &EmitOpts, depth: usize) {
    if !options.compact {
        output.push('\n');
        output.push_str(&" ".repeat(options.indent.saturating_mul(depth)));
    }
}

fn write_closing_indent(output: &mut String, options: &EmitOpts, depth: usize) {
    if !options.compact {
        output.push('\n');
        output.push_str(&" ".repeat(options.indent.saturating_mul(depth)));
    }
}

fn key_string(key: &YamlVal) -> String {
    match &key.data {
        Data::Str(string) => string.to_string(),
        _ => key.to_string(),
    }
}

fn json_quote(string: &str) -> String {
    let mut output = String::with_capacity(string.len() + 2);
    output.push('"');
    for character in string.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\t' => output.push_str("\\t"),
            '\r' => output.push_str("\\r"),
            character if (character as u32) < 0x20 => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

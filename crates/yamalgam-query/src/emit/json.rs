//! JSON output serialization.

use core::fmt::Write as _;
use std::borrow::Cow;
use std::collections::HashSet;

use crate::emit::{EmitError, EmitFormat, EmitOpts, ordered_entries, validate_value};
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
        Data::Int(integer) => {
            write!(output, "{integer}").expect("writing to a String cannot fail");
        }
        Data::Float(float) if !float.is_finite() => output.push_str("null"),
        Data::Float(float) => {
            write!(output, "{float:?}").expect("writing to a String cannot fail");
        }
        Data::Str(string) => write_json_string(output, string),
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
            let mut keys = HashSet::with_capacity(mapping.0.len());
            output.push('{');
            for (index, (key, value)) in ordered_entries(mapping, options.sort_keys).enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_indent(output, options, depth + 1);
                let key = match &key.data {
                    Data::Str(string) => Cow::Borrowed(string.as_ref()),
                    _ => Cow::Owned(key.to_string()),
                };
                if keys.contains(key.as_ref()) {
                    return Err(EmitError::DuplicateJsonKey(key.into_owned()));
                }
                write_json_string(output, key.as_ref());
                keys.insert(key);
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
        output.extend(core::iter::repeat_n(
            ' ',
            options.indent.saturating_mul(depth),
        ));
    }
}

fn write_closing_indent(output: &mut String, options: &EmitOpts, depth: usize) {
    if !options.compact {
        output.push('\n');
        output.extend(core::iter::repeat_n(
            ' ',
            options.indent.saturating_mul(depth),
        ));
    }
}

fn write_json_string(output: &mut String, string: &str) {
    output.push('"');
    for character in string.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\t' => output.push_str("\\t"),
            '\r' => output.push_str("\\r"),
            character
                if (character as u32) < 0x20 || (0x7f..=0x9f).contains(&(character as u32)) =>
            {
                write!(output, "\\u{:04x}", character as u32)
                    .expect("writing to a String cannot fail");
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

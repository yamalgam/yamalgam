//! YAML output serialization.

use yamalgam_scanner::ScalarStyle;

use crate::emit::EmitOpts;
use crate::value::{Data, NodeStyle, YamlVal, fmt_float, needs_quotes, quote_double};

/// Render one value as a YAML document body.
///
/// The result does not include a `---` document separator and always ends in
/// a newline.
#[must_use]
pub fn to_yaml(value: &YamlVal, options: &EmitOpts) -> String {
    let mut output = String::new();
    Context { options }.node(&mut output, value, 0, Position::Root);
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Position {
    Root,
    SequenceItem,
    MappingValue,
    MappingKey,
}

enum Rendered {
    Inline(String),
    Literal(String),
}

struct Context<'options> {
    options: &'options EmitOpts,
}

impl Context<'_> {
    fn node(&self, output: &mut String, value: &YamlVal, depth: usize, position: Position) {
        match &value.data {
            Data::Seq(items) if items.is_empty() => output.push_str("[]"),
            Data::Seq(items) => self.sequence(output, items, depth),
            Data::Map(mapping) if mapping.0.is_empty() => output.push_str("{}"),
            Data::Map(mapping) => self.mapping(output, mapping.0.iter(), depth),
            _ => self.write_scalar(output, value, depth, position),
        }
    }

    fn sequence(&self, output: &mut String, items: &[YamlVal], depth: usize) {
        for item in items {
            self.indent(output, depth);
            output.push_str("- ");
            match &item.data {
                Data::Seq(items) if !items.is_empty() => {
                    output.pop();
                    output.push('\n');
                    self.node(output, item, depth + 1, Position::SequenceItem);
                }
                Data::Map(mapping) if !mapping.0.is_empty() => {
                    output.pop();
                    output.push('\n');
                    self.node(output, item, depth + 1, Position::SequenceItem);
                }
                _ => {
                    self.write_value_after_indicator(output, item, depth, Position::SequenceItem);
                }
            }
        }
    }

    fn mapping<'a>(
        &self,
        output: &mut String,
        entries: impl Iterator<Item = (&'a YamlVal, &'a YamlVal)>,
        depth: usize,
    ) {
        for (key, value) in entries {
            self.indent(output, depth);
            output.push_str(&self.key_text(key));
            output.push(':');
            match &value.data {
                Data::Seq(items) if !items.is_empty() => {
                    output.push('\n');
                    self.node(output, value, depth + 1, Position::MappingValue);
                }
                Data::Map(mapping) if !mapping.0.is_empty() => {
                    output.push('\n');
                    self.node(output, value, depth + 1, Position::MappingValue);
                }
                _ => {
                    output.push(' ');
                    self.write_value_after_indicator(output, value, depth, Position::MappingValue);
                }
            }
        }
    }

    fn write_value_after_indicator(
        &self,
        output: &mut String,
        value: &YamlVal,
        depth: usize,
        position: Position,
    ) {
        match &value.data {
            Data::Seq(items) if items.is_empty() => output.push_str("[]\n"),
            Data::Map(mapping) if mapping.0.is_empty() => output.push_str("{}\n"),
            _ => match self.scalar(value, position) {
                Rendered::Inline(text) => {
                    output.push_str(&text);
                    output.push('\n');
                }
                Rendered::Literal(text) => {
                    output.push_str("|\n");
                    self.literal_content(output, &text, depth + 1);
                }
            },
        }
    }

    fn write_scalar(&self, output: &mut String, value: &YamlVal, depth: usize, position: Position) {
        match self.scalar(value, position) {
            Rendered::Inline(text) => output.push_str(&text),
            Rendered::Literal(text) => {
                output.push_str("|\n");
                self.literal_content(output, &text, depth + 1);
            }
        }
    }

    fn scalar(&self, value: &YamlVal, position: Position) -> Rendered {
        match &value.data {
            Data::Null => Rendered::Inline("null".into()),
            Data::Bool(boolean) => Rendered::Inline(boolean.to_string()),
            Data::Int(integer) => Rendered::Inline(integer.to_string()),
            Data::Float(float) => Rendered::Inline(fmt_float(*float)),
            Data::Str(string) => self.string(string, value, position),
            Data::Seq(_) | Data::Map(_) => unreachable!("collections are not scalars"),
        }
    }

    fn string(&self, string: &str, value: &YamlVal, position: Position) -> Rendered {
        let block_allowed = !self.options.compact && position != Position::MappingKey;
        let one_trailing_newline = string.ends_with('\n') && !string.ends_with("\n\n");
        if string.contains('\n') && one_trailing_newline && block_allowed {
            return Rendered::Literal(string.to_owned());
        }
        if string.contains('\n') || needs_quotes(string) {
            return Rendered::Inline(quote_double(string));
        }

        match value.meta().and_then(|meta| meta.style) {
            Some(NodeStyle::Scalar(ScalarStyle::SingleQuoted)) if !string.contains('\'') => {
                Rendered::Inline(format!("'{string}'"))
            }
            Some(NodeStyle::Scalar(ScalarStyle::DoubleQuoted)) => {
                Rendered::Inline(quote_double(string))
            }
            _ => Rendered::Inline(string.to_owned()),
        }
    }

    fn key_text(&self, key: &YamlVal) -> String {
        match &key.data {
            Data::Seq(_) | Data::Map(_) => key.to_string(),
            _ => match self.scalar(key, Position::MappingKey) {
                Rendered::Inline(text) => text,
                Rendered::Literal(_) => unreachable!("mapping keys cannot use block scalars"),
            },
        }
    }

    fn literal_content(&self, output: &mut String, string: &str, depth: usize) {
        let content = string.strip_suffix('\n').unwrap_or(string);
        for line in content.split('\n') {
            self.indent(output, depth);
            output.push_str(line);
            output.push('\n');
        }
    }

    fn indent(&self, output: &mut String, depth: usize) {
        output.push_str(&" ".repeat(self.options.indent.saturating_mul(depth)));
    }
}

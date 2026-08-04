//! YAML output serialization.

use std::borrow::Cow;
use std::collections::HashSet;
use std::rc::Rc;

use yamalgam_parser::CollectionStyle;
use yamalgam_scanner::ScalarStyle;

use crate::emit::{EmitError, EmitFormat, EmitOpts, ordered_entries, validate_value};
use crate::value::{
    Data, NodeStyle, YamlVal, YgMap, fmt_float, is_valid_tag, needs_quotes, quote_double,
};

/// Render one value as a YAML document body.
///
/// The result does not include a `---` document separator and always ends in
/// a newline.
// r[impl output.yaml]
pub fn to_yaml(value: &YamlVal, options: &EmitOpts) -> Result<String, EmitError> {
    validate_value(value, options, EmitFormat::Yaml)?;
    let mut output = String::new();
    let mut context = Context {
        options,
        anchors_emitted: HashSet::new(),
        indentation: String::new(),
    };
    context.node(&mut output, value, 0, Position::Root);
    if !output.ends_with('\n') {
        output.push('\n');
    }
    options
        .limits
        .check_output_size(output.len())
        .map_err(EmitError::Limit)?;
    Ok(output)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Position {
    Root,
    SequenceItem,
    MappingValue,
    Flow,
}

enum Rendered<'value> {
    Inline(Cow<'value, str>),
    Literal(Cow<'value, str>),
}

struct Context<'options> {
    options: &'options EmitOpts,
    anchors_emitted: HashSet<Rc<str>>,
    indentation: String,
}

impl Context<'_> {
    fn node(&mut self, output: &mut String, value: &YamlVal, depth: usize, position: Position) {
        if self.alias_is_emittable(value) || !self.is_block_collection(value, position) {
            match &value.data {
                Data::Seq(_) | Data::Map(_) => self.inline(output, value),
                _ => self.scalar_node(output, value, depth, position),
            }
            return;
        }

        let prefix = self.properties(value);
        self.record_anchor(value);
        if !prefix.is_empty() {
            output.push_str(prefix.trim_end());
            output.push('\n');
        }
        self.block_collection(output, value, depth);
    }

    fn block_collection(&mut self, output: &mut String, value: &YamlVal, depth: usize) {
        // `node` calls this only after `is_block_collection` accepts the value.
        match &value.data {
            Data::Seq(items) => self.sequence(output, items, depth),
            Data::Map(mapping) => self.mapping(output, mapping, depth, false),
            _ => unreachable!("block collections must be sequences or mappings"),
        }
    }

    fn sequence(&mut self, output: &mut String, items: &[YamlVal], depth: usize) {
        for item in items {
            if self.is_block_collection(item, Position::SequenceItem) {
                let prefix = self.properties(item);
                self.record_anchor(item);
                if let Data::Map(mapping) = &item.data
                    && prefix.is_empty()
                    && self.first_key_is_simple(mapping)
                {
                    self.mapping(output, mapping, depth + 1, true);
                    continue;
                }

                self.indent(output, depth);
                output.push('-');
                if !prefix.is_empty() {
                    output.push(' ');
                    output.push_str(prefix.trim_end());
                }
                output.push('\n');
                self.block_collection(output, item, depth + 1);
            } else {
                self.indent(output, depth);
                output.push_str("- ");
                self.value_after_indicator(output, item, depth, Position::SequenceItem);
            }
        }
    }

    fn mapping(&mut self, output: &mut String, mapping: &YgMap, depth: usize, inline_first: bool) {
        for (index, (key, value)) in ordered_entries(mapping, self.options.sort_keys).enumerate() {
            if inline_first && index == 0 {
                self.indent(output, depth.saturating_sub(1));
                output.push_str("- ");
            } else {
                self.indent(output, depth);
            }
            self.mapping_entry(output, key, value, depth);
        }
    }

    fn mapping_entry(&mut self, output: &mut String, key: &YamlVal, value: &YamlVal, depth: usize) {
        self.inline(output, key);
        if self.key_needs_colon_space(key) {
            output.push_str(" :");
        } else {
            output.push(':');
        }

        if self.is_block_collection(value, Position::MappingValue) {
            let prefix = self.properties(value);
            self.record_anchor(value);
            if !prefix.is_empty() {
                output.push(' ');
                output.push_str(prefix.trim_end());
            }
            output.push('\n');
            self.block_collection(output, value, depth + 1);
        } else {
            output.push(' ');
            self.value_after_indicator(output, value, depth, Position::MappingValue);
        }
    }

    fn value_after_indicator(
        &mut self,
        output: &mut String,
        value: &YamlVal,
        depth: usize,
        position: Position,
    ) {
        if self.alias_is_emittable(value) || matches!(&value.data, Data::Seq(_) | Data::Map(_)) {
            self.inline(output, value);
            output.push('\n');
            return;
        }

        let prefix = self.properties(value);
        self.record_anchor(value);
        output.push_str(&prefix);
        match self.scalar(value, position) {
            Rendered::Inline(text) => {
                output.push_str(&text);
                output.push('\n');
            }
            Rendered::Literal(text) => {
                output.push_str(&self.literal_header(&text));
                output.push('\n');
                self.literal_content(output, &text, depth + 1);
            }
        }
    }

    fn scalar_node(
        &mut self,
        output: &mut String,
        value: &YamlVal,
        depth: usize,
        position: Position,
    ) {
        if self.alias_is_emittable(value) {
            self.inline(output, value);
            return;
        }

        let prefix = self.properties(value);
        self.record_anchor(value);
        output.push_str(&prefix);
        match self.scalar(value, position) {
            Rendered::Inline(text) => output.push_str(&text),
            Rendered::Literal(text) => {
                output.push_str(&self.literal_header(&text));
                output.push('\n');
                self.literal_content(output, &text, depth + 1);
            }
        }
    }

    fn inline(&mut self, output: &mut String, value: &YamlVal) {
        if let Some(alias) = self.alias_text(value) {
            output.push_str(&alias);
            return;
        }

        output.push_str(&self.properties(value));
        self.record_anchor(value);
        match &value.data {
            Data::Seq(items) => {
                output.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        output.push_str(", ");
                    }
                    self.inline(output, item);
                }
                output.push(']');
            }
            Data::Map(mapping) => {
                output.push('{');
                for (index, (key, value)) in
                    ordered_entries(mapping, self.options.sort_keys).enumerate()
                {
                    if index > 0 {
                        output.push_str(", ");
                    }
                    self.inline(output, key);
                    if self.key_needs_colon_space(key) {
                        output.push_str(" : ");
                    } else {
                        output.push_str(": ");
                    }
                    self.inline(output, value);
                }
                output.push('}');
            }
            _ => match self.scalar(value, Position::Flow) {
                Rendered::Inline(text) => output.push_str(&text),
                // `Position::Flow` prevents `string` from selecting block-literal style.
                Rendered::Literal(_) => unreachable!("flow scalars cannot be literal blocks"),
            },
        }
    }

    fn scalar<'value>(&self, value: &'value YamlVal, position: Position) -> Rendered<'value> {
        match &value.data {
            Data::Null => Rendered::Inline(self.paint(Cow::Borrowed("null"), "2")),
            Data::Bool(boolean) => {
                Rendered::Inline(self.paint(Cow::Owned(boolean.to_string()), "33"))
            }
            Data::Int(integer) => {
                Rendered::Inline(self.paint(Cow::Owned(integer.to_string()), "33"))
            }
            Data::Float(float) => Rendered::Inline(self.paint(Cow::Owned(fmt_float(*float)), "33")),
            Data::Str(string) => match self.string(string, value, position) {
                Rendered::Inline(text) => Rendered::Inline(self.paint(text, "32")),
                literal => literal,
            },
            // Callers route collections to `inline` or `block_collection` first.
            Data::Seq(_) | Data::Map(_) => unreachable!("collections are not scalars"),
        }
    }

    // r[impl output.yaml.style-preservation]
    // r[impl output.yaml.style-default]
    fn string<'value>(
        &self,
        string: &'value str,
        value: &YamlVal,
        position: Position,
    ) -> Rendered<'value> {
        let block_allowed = !self.options.compact
            && matches!(
                position,
                Position::Root | Position::SequenceItem | Position::MappingValue
            );
        let literal_safe = string
            .strip_suffix('\n')
            .is_some_and(|content| !content.is_empty())
            && !string.ends_with("\n\n")
            && string
                .chars()
                .all(|character| character == '\n' || !character.is_control());
        if string.contains('\n') && literal_safe && block_allowed {
            return Rendered::Literal(Cow::Borrowed(string));
        }
        if string.contains('\n')
            || needs_quotes(
                string,
                self.options.schema,
                matches!(position, Position::Flow),
            )
        {
            return Rendered::Inline(Cow::Owned(quote_double(string)));
        }

        match value.meta().and_then(|meta| meta.style) {
            Some(NodeStyle::Scalar(ScalarStyle::SingleQuoted)) => {
                Rendered::Inline(Cow::Owned(format!("'{}'", string.replace('\'', "''"))))
            }
            Some(NodeStyle::Scalar(ScalarStyle::DoubleQuoted)) => {
                Rendered::Inline(Cow::Owned(quote_double(string)))
            }
            _ => Rendered::Inline(Cow::Borrowed(string)),
        }
    }

    fn is_block_collection(&self, value: &YamlVal, position: Position) -> bool {
        if self.alias_is_emittable(value) {
            return false;
        }
        let non_empty = match &value.data {
            Data::Seq(items) => !items.is_empty(),
            Data::Map(mapping) => !mapping.0.is_empty(),
            _ => false,
        };
        non_empty && !self.is_flow(value, position)
    }

    fn is_flow(&self, value: &YamlVal, position: Position) -> bool {
        self.options.compact
            || position == Position::Flow
            || matches!(
                value.meta().and_then(|meta| meta.style),
                Some(NodeStyle::Collection(CollectionStyle::Flow))
            )
    }

    fn first_key_is_simple(&self, mapping: &YgMap) -> bool {
        mapping.0.first().is_some_and(|(key, _)| {
            !matches!(&key.data, Data::Seq(_) | Data::Map(_)) && !self.key_needs_colon_space(key)
        })
    }

    fn key_needs_colon_space(&self, key: &YamlVal) -> bool {
        key.meta().is_some_and(|metadata| {
            metadata.anchor.is_some() || metadata.alias_of.is_some() || metadata.tag.is_some()
        })
    }

    fn alias_is_emittable(&self, value: &YamlVal) -> bool {
        value
            .meta()
            .and_then(|meta| meta.alias_of.as_deref())
            .is_some_and(|alias| self.anchors_emitted.contains(alias))
    }

    fn alias_text(&self, value: &YamlVal) -> Option<String> {
        let alias = value.meta()?.alias_of.as_deref()?;
        self.anchors_emitted.contains(alias).then(|| {
            self.paint(Cow::Owned(format!("*{alias}")), "35")
                .into_owned()
        })
    }

    fn properties(&self, value: &YamlVal) -> String {
        let mut prefix = String::new();
        if let Some(metadata) = value.meta()
            && metadata.alias_of.is_none()
        {
            if let Some(anchor) = metadata.anchor.as_deref() {
                prefix.push('&');
                prefix.push_str(anchor);
                prefix.push(' ');
            }
            if let Some(tag) = value.tag_shorthand()
                && tag != "!!merge"
                && let Some(tag) = safe_tag(&tag)
            {
                prefix.push_str(&tag);
                prefix.push(' ');
            }
        }
        if prefix.is_empty() {
            prefix
        } else {
            self.paint(Cow::Owned(prefix), "35").into_owned()
        }
    }

    fn record_anchor(&mut self, value: &YamlVal) {
        if let Some(anchor) = value.meta().and_then(|metadata| metadata.anchor.as_ref()) {
            self.anchors_emitted.insert(Rc::clone(anchor));
        }
    }

    fn literal_content(&mut self, output: &mut String, string: &str, depth: usize) {
        let content = string.strip_suffix('\n').unwrap_or(string);
        for line in content.split('\n') {
            self.indent(output, depth);
            output.push_str(&self.paint(Cow::Borrowed(line), "32"));
            output.push('\n');
        }
    }

    fn literal_header(&self, string: &str) -> String {
        let content = string.strip_suffix('\n').unwrap_or(string);
        let needs_indicator = content
            .split('\n')
            .any(|line| line.starts_with([' ', '\t']));
        if needs_indicator && (1..=9).contains(&self.options.indent) {
            format!("|{}", self.options.indent)
        } else {
            "|".to_owned()
        }
    }

    fn indent(&mut self, output: &mut String, depth: usize) {
        let width = self.options.indent.saturating_mul(depth);
        if self.indentation.len() < width {
            self.indentation.extend(core::iter::repeat_n(
                ' ',
                width.saturating_sub(self.indentation.len()),
            ));
        }
        output.push_str(&self.indentation[..width]);
    }

    fn paint<'text>(&self, text: Cow<'text, str>, ansi_code: &str) -> Cow<'text, str> {
        if self.options.color {
            Cow::Owned(format!("\x1b[{ansi_code}m{text}\x1b[0m"))
        } else {
            text
        }
    }
}

fn safe_tag(tag: &str) -> Option<String> {
    if !is_valid_tag(tag) {
        return None;
    }
    if tag.starts_with("!!") || tag.starts_with("!<") {
        return Some(tag.to_owned());
    }
    if let Some(local) = tag.strip_prefix('!') {
        return (!local.contains('!')).then(|| tag.to_owned());
    }
    Some(format!("!<{tag}>"))
}

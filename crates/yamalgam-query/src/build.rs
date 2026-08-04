//! Build [`YamlVal`] trees directly from parser events.
//!
//! Unlike the lossy composer, this builder retains tags, anchors, aliases,
//! collection/scalar styles, and raw scalar text.

use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;

use yamalgam_core::{LoaderConfig, TagResolution, TagResolver as _, Value};
use yamalgam_parser::{Event, Parser};
use yamalgam_scanner::ScalarStyle;

use crate::error::BuildError;
use crate::value::{Data, Meta, NodeStyle, YamlVal, YgMap, YgSeq};

/// Parse every document using the moderate loader configuration.
// r[impl cli.usage]
pub fn parse_str(input: &str) -> Result<Vec<YamlVal>, BuildError> {
    parse_str_with_config(input, &LoaderConfig::moderate())
}

/// Parse every document using an explicit loader configuration.
pub fn parse_str_with_config(
    input: &str,
    config: &LoaderConfig,
) -> Result<Vec<YamlVal>, BuildError> {
    config
        .limits
        .check_input_size(input.len())
        .map_err(BuildError::Limit)?;

    let mut events = Parser::with_config(input, config)
        .filter(|event| event.as_ref().map_or(true, |event| !event.is_structural()));
    let mut builder = Builder::new(config);
    let mut documents = Vec::new();

    while let Some(event) = next_optional_event(&mut events)? {
        match event {
            Event::StreamStart => {}
            Event::StreamEnd => break,
            Event::VersionDirective { .. } | Event::TagDirective { .. } => {}
            Event::DocumentStart { .. } => {
                builder.start_document();
                match next_optional_event(&mut events)? {
                    Some(Event::DocumentEnd { .. }) | None => {
                        documents.push(YamlVal::null());
                    }
                    Some(Event::StreamEnd) => {
                        documents.push(YamlVal::null());
                        break;
                    }
                    Some(first) => documents.push(builder.node(first, &mut events, 0)?),
                }
            }
            Event::DocumentEnd { .. } => {}
            first => {
                builder.start_document();
                documents.push(builder.node(first, &mut events, 0)?);
            }
        }
    }

    Ok(documents)
}

struct Builder<'config> {
    config: &'config LoaderConfig,
    anchors: HashMap<String, YamlVal>,
    alias_expansions: usize,
}

impl<'config> Builder<'config> {
    fn new(config: &'config LoaderConfig) -> Self {
        Self {
            config,
            anchors: HashMap::new(),
            alias_expansions: 0,
        }
    }

    fn start_document(&mut self) {
        self.anchors.clear();
        self.alias_expansions = 0;
    }

    fn node<'input>(
        &mut self,
        first: Event<'input>,
        events: &mut impl Iterator<Item = Result<Event<'input>, yamalgam_parser::ParseError>>,
        depth: usize,
    ) -> Result<YamlVal, BuildError> {
        self.config
            .limits
            .check_depth(depth)
            .map_err(BuildError::Limit)?;

        match first {
            // r[impl datamodel.metadata.tag]
            // r[impl datamodel.metadata.anchor]
            // r[impl datamodel.metadata.style]
            Event::Scalar {
                anchor,
                tag,
                value,
                style,
                ..
            } => {
                let raw: Rc<str> = Rc::from(value.as_ref());
                let data = self.resolve_scalar(Rc::clone(&raw), style, tag.as_deref());
                self.finish(data, anchor, tag, NodeStyle::Scalar(style), Some(raw))
            }
            Event::SequenceStart {
                anchor, tag, style, ..
            } => {
                let mut items = Vec::new();
                loop {
                    let event = next_event(events)?;
                    if matches!(event, Event::SequenceEnd { .. }) {
                        break;
                    }
                    let size = items
                        .len()
                        .checked_add(1)
                        .ok_or_else(|| BuildError::Limit("sequence entry count overflow".into()))?;
                    self.config
                        .limits
                        .check_collection_size(size)
                        .map_err(BuildError::Limit)?;
                    items.push(self.node(event, events, depth + 1)?);
                }
                self.finish(
                    Data::Seq(Rc::new(YgSeq(items))),
                    anchor,
                    tag,
                    NodeStyle::Collection(style),
                    None,
                )
            }
            Event::MappingStart {
                anchor, tag, style, ..
            } => {
                let mut map = YgMap::default();
                let mut entries = 0usize;
                loop {
                    let event = next_event(events)?;
                    if matches!(event, Event::MappingEnd { .. }) {
                        break;
                    }
                    entries = entries
                        .checked_add(1)
                        .ok_or_else(|| BuildError::Limit("mapping entry count overflow".into()))?;
                    self.config
                        .limits
                        .check_collection_size(entries)
                        .map_err(BuildError::Limit)?;
                    let mut key = self.node(event, events, depth + 1)?;
                    self.tag_merge_key(&mut key);
                    let value_event = next_event(events)?;
                    let value = self.node(value_event, events, depth + 1)?;
                    map.0.insert(key, value);
                }
                self.finish(
                    Data::Map(Rc::new(map)),
                    anchor,
                    tag,
                    NodeStyle::Collection(style),
                    None,
                )
            }
            Event::Alias { name, span } => {
                self.alias_expansions += 1;
                self.config
                    .limits
                    .check_alias_expansions(self.alias_expansions)
                    .map_err(BuildError::Limit)?;

                let mut value = self.anchors.get(name.as_ref()).cloned().ok_or_else(|| {
                    BuildError::UnknownAlias {
                        name: name.to_string(),
                        span,
                    }
                })?;
                let meta = value.meta_mut();
                meta.anchor = None;
                meta.alias_of = Some(Rc::from(name.as_ref()));
                Ok(value)
            }
            unexpected => Err(BuildError::Limit(format!(
                "unexpected parser event while building a node: {unexpected:?}"
            ))),
        }
    }

    // r[impl datamodel.schema]
    fn resolve_scalar(&self, text: Rc<str>, style: ScalarStyle, tag: Option<&str>) -> Data {
        if let Some(tag) = tag {
            return resolve_tagged(text, tag);
        }
        if style != ScalarStyle::Plain {
            return Data::Str(text);
        }
        if self.config.tag_resolution.resolves_as_string(text.as_ref()) {
            return Data::Str(text);
        }
        core_value_to_data(self.config.tag_resolution.resolve_scalar(text.as_ref()))
    }

    fn finish(
        &mut self,
        data: Data,
        anchor: Option<Cow<'_, str>>,
        tag: Option<Cow<'_, str>>,
        style: NodeStyle,
        raw: Option<Rc<str>>,
    ) -> Result<YamlVal, BuildError> {
        let meta = Meta {
            tag: tag.as_deref().map(Rc::from),
            anchor: anchor.as_deref().map(Rc::from),
            alias_of: None,
            style: Some(style),
            raw,
        };
        let node = YamlVal {
            data,
            meta: Some(Rc::new(meta)),
        };
        node.check_depth(&self.config.limits)
            .map_err(BuildError::Limit)?;

        if let Some(name) = anchor {
            self.anchors.insert(name.into_owned(), node.clone());
            self.config
                .limits
                .check_anchor_count(self.anchors.len())
                .map_err(BuildError::Limit)?;
        }

        Ok(node)
    }

    fn tag_merge_key(&self, key: &mut YamlVal) {
        // r[impl ext.merge.preserve]
        let is_merge = matches!(&key.data, Data::Str(value) if &**value == "<<")
            && key.meta().is_none_or(|meta| {
                meta.tag.is_none()
                    && matches!(meta.style, Some(NodeStyle::Scalar(ScalarStyle::Plain)))
            });
        if is_merge {
            key.meta_mut().tag = Some(Rc::from("tag:yaml.org,2002:merge"));
        }
    }
}

fn next_optional_event<'input>(
    events: &mut impl Iterator<Item = Result<Event<'input>, yamalgam_parser::ParseError>>,
) -> Result<Option<Event<'input>>, BuildError> {
    events.next().transpose().map_err(BuildError::from)
}

fn next_event<'input>(
    events: &mut impl Iterator<Item = Result<Event<'input>, yamalgam_parser::ParseError>>,
) -> Result<Event<'input>, BuildError> {
    next_optional_event(events)?
        .ok_or_else(|| BuildError::Limit("unexpected end of parser event stream".into()))
}

pub(crate) fn core_value_to_data(value: Value) -> Data {
    match value {
        Value::Null => Data::Null,
        Value::Bool(value) => Data::Bool(value),
        Value::Integer(value) => Data::Int(value),
        Value::Float(value) => Data::Float(value),
        Value::String(value) => Data::Str(Rc::from(value)),
        Value::Sequence(_) | Value::Mapping(_) => {
            unreachable!("scalar resolver returned a collection")
        }
    }
}

fn resolve_tagged(text: Rc<str>, tag: &str) -> Data {
    let suffix = tag
        .strip_prefix("tag:yaml.org,2002:")
        .or_else(|| tag.strip_prefix("!!"));

    match suffix {
        Some("str") => Data::Str(text),
        Some("null") => match resolve_yaml12_non_string(&text) {
            Some(data @ Data::Null) => data,
            _ => Data::Str(text),
        },
        Some("bool") => match resolve_yaml12_non_string(&text) {
            Some(data @ Data::Bool(_)) => data,
            _ => Data::Str(text),
        },
        Some("int") => match resolve_yaml12_non_string(&text) {
            Some(data @ Data::Int(_)) => data,
            _ => Data::Str(text),
        },
        Some("float") => match resolve_yaml12_non_string(&text) {
            Some(data @ Data::Float(_)) => data,
            Some(Data::Int(value)) => Data::Float(value as f64),
            _ => Data::Str(text),
        },
        Some(_) | None => Data::Str(text),
    }
}

fn resolve_yaml12_non_string(text: &str) -> Option<Data> {
    if TagResolution::Yaml12.resolves_as_string(text) {
        None
    } else {
        Some(core_value_to_data(
            TagResolution::Yaml12.resolve_scalar(text),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_scalars_share_resolved_and_raw_allocation() {
        for source in ["plain\n", "'quoted'\n", "!!str tagged\n", "!!int text\n"] {
            let mut documents = parse_str(source).expect("scalar should parse");
            let value = documents.pop().expect("document should contain a scalar");
            let Data::Str(data) = &value.data else {
                panic!("expected string scalar for {source:?}");
            };
            let raw = value
                .meta()
                .and_then(|meta| meta.raw.as_ref())
                .expect("scalar should retain raw text");

            assert!(
                Rc::ptr_eq(data, raw),
                "string data and raw metadata do not share storage for {source:?}"
            );
        }
    }
}

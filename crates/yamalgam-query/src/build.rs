//! Build [`YamlVal`] trees directly from parser events.
//!
//! Unlike the lossy composer, this builder retains tags, anchors, aliases,
//! collection/scalar styles, and raw scalar text.

use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;

use yamalgam_core::{LoaderConfig, TagResolution, TagResolver as _, Value};
use yamalgam_parser::{Event, NoopResolver, Parser, ResolveError, ResolvedEvents, Resolver};
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
    parse_documents_with_config(input, config)?.collect()
}

/// Parse every document through custom resolver middleware using the moderate
/// loader configuration.
pub fn parse_str_with_resolver<'input, R>(
    input: &'input str,
    resolver: R,
) -> Result<Vec<YamlVal>, BuildError>
where
    R: Resolver<'input> + 'input,
{
    parse_str_with_resolver_and_config(input, resolver, &LoaderConfig::moderate())
}

/// Parse every document through custom resolver middleware using a selected
/// loader configuration.
pub fn parse_str_with_resolver_and_config<'input, R>(
    input: &'input str,
    resolver: R,
    config: &LoaderConfig,
) -> Result<Vec<YamlVal>, BuildError>
where
    R: Resolver<'input> + 'input,
{
    parse_documents_with_resolver_and_config(input, resolver, config)?.collect()
}

/// Parse documents incrementally using an explicit loader configuration.
pub fn parse_documents_with_config<'input, 'config>(
    input: &'input str,
    config: &'config LoaderConfig,
) -> Result<Documents<'input, 'config>, BuildError> {
    parse_documents_with_resolver_and_config(input, NoopResolver, config)
}

/// Parse documents incrementally through custom resolver middleware using a
/// selected loader configuration.
pub fn parse_documents_with_resolver_and_config<'input, 'config, R>(
    input: &'input str,
    resolver: R,
    config: &'config LoaderConfig,
) -> Result<Documents<'input, 'config>, BuildError>
where
    R: Resolver<'input> + 'input,
{
    config
        .limits
        .check_input_size(input.len())
        .map_err(BuildError::Limit)?;

    let parser = Parser::with_config(input, config);
    let events = ResolvedEvents::new_with_config(
        Box::new(parser.map(|result| result.map_err(ResolveError::Parse))),
        resolver,
        config,
    );

    Ok(Documents {
        events: SemanticEvents {
            inner: Box::new(events),
        },
        builder: Builder::new(config),
        done: false,
    })
}

/// Iterator of owned YAML document values parsed from one borrowed input.
pub struct Documents<'input, 'config> {
    events: SemanticEvents<'input>,
    builder: Builder<'config>,
    done: bool,
}

impl Iterator for Documents<'_, '_> {
    type Item = Result<YamlVal, BuildError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        match self.next_document() {
            Ok(Some(document)) => Some(Ok(document)),
            Ok(None) => {
                self.done = true;
                None
            }
            Err(error) => {
                self.done = true;
                Some(Err(error))
            }
        }
    }
}

impl Documents<'_, '_> {
    fn next_document(&mut self) -> Result<Option<YamlVal>, BuildError> {
        while let Some(event) = next_optional_event(&mut self.events)? {
            match event {
                Event::StreamStart => {}
                Event::StreamEnd => return Ok(None),
                Event::VersionDirective { .. } | Event::TagDirective { .. } => {}
                Event::DocumentStart { .. } => {
                    self.builder.start_document();
                    return match next_optional_event(&mut self.events)? {
                        Some(Event::DocumentEnd { .. }) => Ok(Some(YamlVal::null())),
                        None => {
                            self.done = true;
                            Ok(Some(YamlVal::null()))
                        }
                        Some(Event::StreamEnd) => {
                            self.done = true;
                            Ok(Some(YamlVal::null()))
                        }
                        Some(first) => {
                            Ok(Some(self.builder.node(first, &mut self.events, 0)?.value))
                        }
                    };
                }
                Event::DocumentEnd { .. } => {}
                first => {
                    self.builder.start_document();
                    return Ok(Some(self.builder.node(first, &mut self.events, 0)?.value));
                }
            }
        }

        Ok(None)
    }
}

struct SemanticEvents<'input> {
    inner: Box<dyn Iterator<Item = Result<Event<'input>, ResolveError>> + 'input>,
}

impl<'input> Iterator for SemanticEvents<'input> {
    type Item = Result<Event<'input>, ResolveError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let event = self.inner.next()?;
            if event.as_ref().map_or(true, |event| !event.is_structural()) {
                return Some(event);
            }
        }
    }
}

#[derive(Clone)]
struct BuiltNode {
    value: YamlVal,
    subtree_height: usize,
}

struct Builder<'config> {
    config: &'config LoaderConfig,
    anchors: HashMap<String, BuiltNode>,
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
        events: &mut impl Iterator<Item = Result<Event<'input>, ResolveError>>,
        depth: usize,
    ) -> Result<BuiltNode, BuildError> {
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
                self.finish(data, anchor, tag, NodeStyle::Scalar(style), Some(raw), 0)
            }
            Event::SequenceStart {
                anchor, tag, style, ..
            } => {
                let mut items = Vec::new();
                let mut max_child_height = None;
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
                    let child = self.node(event, events, child_depth(depth)?)?;
                    max_child_height = Some(
                        max_child_height.map_or(child.subtree_height, |height: usize| {
                            height.max(child.subtree_height)
                        }),
                    );
                    items.push(child.value);
                }
                self.finish(
                    Data::Seq(Rc::new(YgSeq(items))),
                    anchor,
                    tag,
                    NodeStyle::Collection(style),
                    None,
                    collection_height(max_child_height)?,
                )
            }
            Event::MappingStart {
                anchor, tag, style, ..
            } => {
                let mut map = YgMap::default();
                let mut entry_heights: Vec<(usize, usize)> = Vec::new();
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
                    let entry_depth = child_depth(depth)?;
                    let mut key = self.node(event, events, entry_depth)?;
                    self.tag_merge_key(&mut key.value);
                    let value_event = next_event(events)?;
                    let value = self.node(value_event, events, entry_depth)?;

                    if let Some(index) = map.0.get_index_of(&key.value) {
                        map.0.insert(key.value, value.value);
                        entry_heights[index].1 = value.subtree_height;
                    } else {
                        map.0.insert(key.value, value.value);
                        entry_heights.push((key.subtree_height, value.subtree_height));
                    }
                }
                let max_child_height = entry_heights.into_iter().flat_map(<[_; 2]>::from).max();
                self.finish(
                    Data::Map(Rc::new(map)),
                    anchor,
                    tag,
                    NodeStyle::Collection(style),
                    None,
                    collection_height(max_child_height)?,
                )
            }
            Event::Alias { name, span } => {
                self.alias_expansions += 1;
                self.config
                    .limits
                    .check_alias_expansions(self.alias_expansions)
                    .map_err(BuildError::Limit)?;

                let mut node = self.anchors.get(name.as_ref()).cloned().ok_or_else(|| {
                    BuildError::UnknownAlias {
                        name: name.to_string(),
                        span,
                    }
                })?;
                let materialized_depth = depth
                    .checked_add(node.subtree_height)
                    .ok_or_else(|| BuildError::Limit("value depth overflow".into()))?;
                self.config
                    .limits
                    .check_depth(materialized_depth)
                    .map_err(BuildError::Limit)?;

                let meta = node.value.meta_mut();
                meta.anchor = None;
                meta.alias_of = Some(Rc::from(name.as_ref()));
                Ok(node)
            }
            unexpected => Err(BuildError::Invariant(format!(
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
        subtree_height: usize,
    ) -> Result<BuiltNode, BuildError> {
        let meta = Meta {
            tag: tag.as_deref().map(Rc::from),
            anchor: anchor.as_deref().map(Rc::from),
            alias_of: None,
            style: Some(style),
            raw,
        };
        let node = BuiltNode {
            value: YamlVal {
                data,
                meta: Some(Rc::new(meta)),
            },
            subtree_height,
        };

        if let Some(name) = anchor {
            let additional = usize::from(!self.anchors.contains_key(name.as_ref()));
            let anchor_count = self
                .anchors
                .len()
                .checked_add(additional)
                .ok_or_else(|| BuildError::Limit("anchor count overflow".into()))?;
            self.config
                .limits
                .check_anchor_count(anchor_count)
                .map_err(BuildError::Limit)?;
            self.anchors.insert(name.into_owned(), node.clone());
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

fn child_depth(depth: usize) -> Result<usize, BuildError> {
    depth
        .checked_add(1)
        .ok_or_else(|| BuildError::Limit("value depth overflow".into()))
}

fn collection_height(max_child_height: Option<usize>) -> Result<usize, BuildError> {
    max_child_height.map_or(Ok(0), |height| {
        height
            .checked_add(1)
            .ok_or_else(|| BuildError::Limit("value depth overflow".into()))
    })
}

fn next_optional_event<'input>(
    events: &mut impl Iterator<Item = Result<Event<'input>, ResolveError>>,
) -> Result<Option<Event<'input>>, BuildError> {
    events.next().transpose().map_err(BuildError::from)
}

fn next_event<'input>(
    events: &mut impl Iterator<Item = Result<Event<'input>, ResolveError>>,
) -> Result<Event<'input>, BuildError> {
    next_optional_event(events)?
        .ok_or_else(|| BuildError::Invariant("unexpected end of parser event stream".into()))
}

pub(crate) fn core_value_to_data(value: Value) -> Data {
    match value {
        Value::Null => Data::Null,
        Value::Bool(value) => Data::Bool(value),
        Value::Integer(value) => Data::Int(value),
        Value::Float(value) => Data::Float(value),
        Value::String(value) => Data::Str(Rc::from(value)),
        // This receives only `TagResolver::resolve_scalar` results; crate-owned
        // resolvers return scalar `Value` variants from that method.
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
    use crate::{depth_walk_calls, reset_depth_walk_calls};

    fn parse_with_builder_config(
        input: &str,
        config: &LoaderConfig,
    ) -> Result<Vec<YamlVal>, BuildError> {
        let parser = Parser::with_config(input, &LoaderConfig::unchecked());
        Documents {
            events: SemanticEvents {
                inner: Box::new(parser.map(|result| result.map_err(ResolveError::Parse))),
            },
            builder: Builder::new(config),
            done: false,
        }
        .collect()
    }

    #[test]
    fn builder_does_not_rewalk_completed_values() {
        reset_depth_walk_calls();

        let documents = parse_str(concat!(
            "a: &a0 [0]\n",
            "b: &a1 [*a0]\n",
            "c: {nested: [*a1]}\n",
        ))
        .expect("nested aliases should parse");

        assert_eq!(documents.len(), 1);
        assert_eq!(depth_walk_calls(), 0, "builder rewalked a completed value");
    }

    #[test]
    fn builder_treats_anchored_empty_collections_as_height_zero() {
        let document = concat!(
            "empty_seq: &seq []\n",
            "empty_map: &map {}\n",
            "seq_use: [[*seq]]\n",
            "map_use: [[*map]]\n",
        );
        let mut config = LoaderConfig::unchecked();
        config.limits.max_depth = Some(3);

        assert!(parse_with_builder_config(document, &config).is_ok());
    }

    #[test]
    fn builder_counts_mapping_key_and_value_heights() {
        for document in [
            "base: &base {[0]: 1}\nuse: [*base]\n",
            "base: &base {key: [0]}\nuse: [*base]\n",
        ] {
            let mut rejecting = LoaderConfig::unchecked();
            rejecting.limits.max_depth = Some(3);
            assert!(matches!(
                parse_with_builder_config(document, &rejecting),
                Err(BuildError::Limit(message)) if message.contains("depth")
            ));

            let mut accepting = LoaderConfig::unchecked();
            accepting.limits.max_depth = Some(4);
            assert!(parse_with_builder_config(document, &accepting).is_ok());
        }
    }

    #[test]
    fn builder_height_ignores_overwritten_mapping_values() {
        let document = "base: &base {key: [[0]], key: 1}\nuse: [[*base]]\n";
        let mut config = LoaderConfig::unchecked();
        config.limits.max_depth = Some(4);

        let documents = parse_with_builder_config(document, &config).unwrap();
        assert_eq!(
            documents[0].get_str("base").unwrap().get_str("key"),
            Some(&YamlVal::int(1))
        );
    }

    #[test]
    fn builder_redefines_anchor_value_and_height_without_growing_count() {
        let document = concat!("old: &same [[0]]\n", "new: &same 1\n", "use: [[*same]]\n",);
        let mut config = LoaderConfig::unchecked();
        config.limits.max_depth = Some(3);
        config.limits.max_anchor_count = Some(1);

        let documents = parse_with_builder_config(document, &config).unwrap();
        let alias = &documents[0].get_str("use").unwrap().as_sequence().unwrap()[0]
            .as_sequence()
            .unwrap()[0];
        assert_eq!(alias, &YamlVal::int(1));
        assert_eq!(alias.meta().unwrap().alias_of.as_deref(), Some("same"));

        let error = parse_with_builder_config(&format!("{document}other: &other 2\n"), &config)
            .unwrap_err();
        assert!(matches!(error, BuildError::Limit(message) if message.contains("anchor count")));
    }

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

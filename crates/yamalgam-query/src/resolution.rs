//! Query-facing resource resolution and origin metadata.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use yamalgam_core::{LimitExceeded, LoaderConfig, ResolutionCapability, ResourceLimits};
use yamalgam_parser::{
    AtomFlags, Chomp, CollectionStyle, Event, Parser, ResolveError, ScalarStyle,
};
use yamalgam_resolve::{
    DocumentError, OwnedDocument, OwnedKind, OwnedNode, OwnedScalar, ResolutionReport, ResourceId,
    ResourceRetriever,
};

use crate::error::{BuildError, YgError};
use crate::runtime_resolution::SessionQueryResolver;
use crate::value::{Data, Meta, NodeStyle, ScalarPresentation, YamlVal, YgMap, YgSeq};

/// Source resource and opaque query-session identity for one YAML node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceOrigin {
    /// Resource containing this node.
    pub resource: ResourceId,
    /// Process-unique resolver session. This is intentionally opaque to callers.
    #[doc(hidden)]
    pub session: ResolutionSessionKey,
}

/// Opaque process-unique key for one query resolution session.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ResolutionSessionKey {
    context: u64,
    local: u64,
}

impl ResolutionSessionKey {
    pub(crate) const UNREGISTERED: Self = Self {
        context: 0,
        local: 0,
    };

    pub(crate) const fn registered(context: u64, local: u64) -> Self {
        Self { context, local }
    }
}

/// Shared query resolver retained by the run context.
pub type ResolutionHandle = Rc<RefCell<dyn QueryResolver>>;

/// External resolution available to the `include` and `ref` query filters.
pub trait QueryResolver {
    /// Resolve one query value through the selected capability.
    ///
    /// # Errors
    ///
    /// Returns the structured resolver failure without reducing it to text.
    #[allow(clippy::result_large_err)] // The accepted public contract returns ResolveError directly.
    fn resolve_value(
        &mut self,
        capability: ResolutionCapability,
        value: YamlVal,
    ) -> Result<YamlVal, ResolveError>;

    /// Inspect the report accumulated by this session.
    fn report(&self) -> &ResolutionReport;
}

/// One query value paired with the session that loaded it.
pub struct ResolvedInput {
    /// Parsed and automatically resolved YAML value.
    pub value: YamlVal,
    /// Session retained for later explicit `include` or `ref` filters.
    pub resolution: ResolutionHandle,
}

/// Create a retained query resolver for constructed or separately built input.
///
/// `base` supplies the resource identity used by relative directives on values
/// that do not carry [`ResourceOrigin`] metadata.
#[must_use]
pub fn resolution_handle<R>(
    retriever: R,
    base: Option<ResourceId>,
    config: LoaderConfig,
) -> ResolutionHandle
where
    R: ResourceRetriever + 'static,
{
    Rc::new(RefCell::new(SessionQueryResolver::new(
        retriever, config, base,
    )))
}

/// Parse exactly one YAML document and retain its resolution session.
///
/// Automatic capabilities run before the value reaches a query. Explicit
/// capabilities remain available through [`ResolvedInput::resolution`].
///
/// # Errors
///
/// Returns a typed parse, document-count, or input-resolution failure.
pub fn resolve_document<R>(
    input: &str,
    root: ResourceId,
    retriever: R,
    config: &LoaderConfig,
) -> Result<ResolvedInput, YgError>
where
    R: ResourceRetriever + 'static,
{
    config
        .limits
        .check_input_size(input.len())
        .map_err(|error| YgError::input_limit(error, error.to_string()))?;
    let events = Parser::with_config(input, config)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| YgError::from(BuildError::Parse(error)))?;
    let document = OwnedDocument::from_events(root.clone(), events)
        .map_err(|error| YgError::InputParse(error.to_string()))?;
    let mut resolver = SessionQueryResolver::new(retriever, config.clone(), Some(root));
    let document = match resolver.resolve_input(document) {
        Ok(document) => document,
        Err(error) => {
            let message = error.to_string();
            return Err(YgError::input_resolution(
                error,
                message,
                vec![resolver.into_report()],
            ));
        }
    };
    let value =
        match value_from_owned_node(document.root, ResolutionSessionKey::UNREGISTERED, config) {
            Ok(value) => value,
            Err(error) => {
                return Err(YgError::input_conversion(
                    error,
                    vec![resolver.into_report()],
                ));
            }
        };
    Ok(ResolvedInput {
        value,
        resolution: Rc::new(RefCell::new(resolver)),
    })
}

/// Parse every YAML document and give each one an independent resolution session.
///
/// The retriever is cloned per document so callers can share one thread-safe
/// router while keeping caches, budgets, and reports scoped to one root
/// document.
///
/// # Errors
///
/// Returns a typed parse, document-conversion, or input-resolution failure.
pub fn resolve_documents<R>(
    input: &str,
    root: ResourceId,
    retriever: R,
    config: &LoaderConfig,
) -> Result<Vec<ResolvedInput>, YgError>
where
    R: ResourceRetriever + Clone + 'static,
{
    config
        .limits
        .check_input_size(input.len())
        .map_err(|error| YgError::input_limit(error, error.to_string()))?;
    let events = Parser::with_config(input, config)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| YgError::from(BuildError::Parse(error)))?;
    let documents = split_documents(root.clone(), events)?;
    let mut inputs: Vec<ResolvedInput> = Vec::with_capacity(documents.len());
    for (document_index, document) in documents.into_iter().enumerate() {
        let mut resolver =
            SessionQueryResolver::new(retriever.clone(), config.clone(), Some(root.clone()));
        let document = match resolver.resolve_input(document) {
            Ok(document) => document,
            Err(error) => {
                let message = error.to_string();
                let mut reports = inputs
                    .iter()
                    .map(|input| input.resolution.borrow().report().clone())
                    .collect::<Vec<_>>();
                reports.push(resolver.into_report());
                return Err(YgError::input_resolution(error, message, reports)
                    .with_document_context(document_index));
            }
        };
        let value = match value_from_owned_node(
            document.root,
            ResolutionSessionKey::UNREGISTERED,
            config,
        ) {
            Ok(value) => value,
            Err(error) => {
                let mut reports = inputs
                    .iter()
                    .map(|input| input.resolution.borrow().report().clone())
                    .collect::<Vec<_>>();
                reports.push(resolver.into_report());
                return Err(
                    YgError::input_conversion(error, reports).with_document_context(document_index)
                );
            }
        };
        inputs.push(ResolvedInput {
            value,
            resolution: Rc::new(RefCell::new(resolver)),
        });
    }
    Ok(inputs)
}

fn split_documents(
    origin: ResourceId,
    events: Vec<Event<'_>>,
) -> Result<Vec<OwnedDocument>, YgError> {
    let mut documents = Vec::new();
    let mut current = None::<Vec<Event<'_>>>;
    for event in events {
        match event {
            Event::DocumentStart { .. } => {
                if current.replace(vec![event]).is_some() {
                    return Err(YgError::InputParse(
                        "unexpected document start in YAML document".into(),
                    ));
                }
            }
            Event::DocumentEnd { .. } => {
                let mut document = current.take().ok_or_else(|| {
                    YgError::InputParse("unexpected document end in YAML stream".into())
                })?;
                document.push(event);
                let document_index = documents.len();
                documents.push(
                    OwnedDocument::from_events(origin.clone(), document)
                        .map_err(|error| YgError::InputParse(error.to_string()))
                        .map_err(|error| error.with_document_context(document_index))?,
                );
            }
            Event::StreamStart
            | Event::StreamEnd
            | Event::VersionDirective { .. }
            | Event::TagDirective { .. } => {}
            event => current
                .as_mut()
                .ok_or_else(|| YgError::InputParse("YAML node appeared outside a document".into()))?
                .push(event),
        }
    }
    if current.is_some() {
        return Err(YgError::InputParse(
            "YAML stream contains a truncated document".into(),
        ));
    }
    Ok(documents)
}

pub(crate) fn value_from_owned_node(
    node: OwnedNode,
    session: ResolutionSessionKey,
    config: &LoaderConfig,
) -> Result<YamlVal, YgError> {
    let mut anchors = HashMap::new();
    let value = owned_to_value(node, session, config, &mut anchors)?;
    value
        .check_depth(&config.limits)
        .map_err(|error| match error {
            crate::value::ValueLimitError::Limit(error) => {
                YgError::input_limit(error, error.to_string())
            }
            crate::value::ValueLimitError::ResourceOverflow(resource) => {
                YgError::resource_overflow(resource, format!("{resource} overflow"))
            }
        })?;
    Ok(value)
}

pub(crate) fn assign_session(value: &mut YamlVal, session: ResolutionSessionKey) {
    let mut pending = vec![AssignSessionWork::Visit(std::mem::take(value))];
    let mut resolved = Vec::new();

    while let Some(work) = pending.pop() {
        match work {
            AssignSessionWork::Visit(value) => {
                let YamlVal {
                    data,
                    mut meta,
                    runtime_resolution_error,
                } = value;
                if let Some(metadata) = &mut meta
                    && let Some(origin) = &mut Rc::make_mut(metadata).origin
                {
                    origin.session = session;
                }
                match data {
                    Data::Seq(mut items) => {
                        let values = std::mem::take(&mut Rc::make_mut(&mut items).0);
                        pending.push(AssignSessionWork::FinishSequence {
                            meta,
                            runtime_resolution_error,
                            resolved_start: resolved.len(),
                        });
                        pending.extend(values.into_iter().rev().map(AssignSessionWork::Visit));
                    }
                    Data::Map(mut mapping) => {
                        let entries = std::mem::take(&mut Rc::make_mut(&mut mapping).0);
                        pending.push(AssignSessionWork::FinishMapping {
                            meta,
                            runtime_resolution_error,
                            resolved_start: resolved.len(),
                        });
                        for (key, value) in entries.into_iter().rev() {
                            pending.push(AssignSessionWork::Visit(value));
                            pending.push(AssignSessionWork::Visit(key));
                        }
                    }
                    data => resolved.push(YamlVal {
                        data,
                        meta,
                        runtime_resolution_error,
                    }),
                }
            }
            AssignSessionWork::FinishSequence {
                meta,
                runtime_resolution_error,
                resolved_start,
            } => {
                let items = resolved.split_off(resolved_start);
                resolved.push(YamlVal {
                    data: Data::Seq(Rc::new(YgSeq(items))),
                    meta,
                    runtime_resolution_error,
                });
            }
            AssignSessionWork::FinishMapping {
                meta,
                runtime_resolution_error,
                resolved_start,
            } => {
                let mut nodes = resolved.split_off(resolved_start).into_iter();
                let mut mapping = YgMap::default();
                while let Some(key) = nodes.next() {
                    let value = nodes
                        .next()
                        .expect("session assignment mapping has key/value parity");
                    mapping.0.insert(key, value);
                }
                resolved.push(YamlVal {
                    data: Data::Map(Rc::new(mapping)),
                    meta,
                    runtime_resolution_error,
                });
            }
        }
    }

    *value = resolved
        .pop()
        .expect("session assignment always reconstructs its root");
}

fn owned_to_value(
    node: OwnedNode,
    session: ResolutionSessionKey,
    config: &LoaderConfig,
    anchors: &mut HashMap<String, YamlVal>,
) -> Result<YamlVal, YgError> {
    let mut pending = vec![OwnedToValueWork::Visit { node, depth: 0 }];
    let mut resolved = Vec::new();

    while let Some(work) = pending.pop() {
        match work {
            OwnedToValueWork::Visit { node, depth } => {
                if let Err(error) = config.limits.check_depth(depth) {
                    pending.push(OwnedToValueWork::Visit { node, depth });
                    discard_owned_work(pending);
                    return Err(YgError::input_limit(error, error.to_string()));
                }
                let OwnedNode {
                    origin,
                    anchor,
                    tag,
                    kind,
                    ..
                } = node;
                if let OwnedKind::Alias { name } = kind {
                    let Some(mut value) = anchors.get(&name).cloned() else {
                        discard_owned_work(pending);
                        return Err(YgError::InputParse(format!(
                            "unknown alias *{name} in resolved resource"
                        )));
                    };
                    let metadata = value.meta_mut();
                    metadata.anchor = None;
                    metadata.alias_of = Some(Rc::from(name));
                    metadata.origin = Some(ResourceOrigin {
                        resource: origin,
                        session,
                    });
                    resolved.push(value);
                    continue;
                }

                let metadata = OwnedValueMetadata {
                    origin,
                    anchor,
                    tag,
                };
                match kind {
                    OwnedKind::Scalar(scalar) => {
                        let raw: Rc<str> = Rc::from(scalar.value.as_str());
                        let data = crate::build::resolve_scalar_data(
                            Rc::clone(&raw),
                            scalar.style,
                            metadata.tag.as_deref(),
                            config.tag_resolution,
                        );
                        resolved.push(finish_owned_value(
                            metadata,
                            data,
                            NodeStyle::Scalar(scalar.style),
                            Some(raw),
                            Some(ScalarPresentation {
                                chomp: scalar.chomp,
                                flags: scalar.flags,
                            }),
                            session,
                            anchors,
                        ));
                    }
                    OwnedKind::Sequence { style, items } => {
                        let Some(child_depth) = depth.checked_add(1) else {
                            pending.extend(
                                items
                                    .into_iter()
                                    .map(|node| OwnedToValueWork::Visit { node, depth }),
                            );
                            discard_owned_work(pending);
                            return Err(YgError::resource_overflow(
                                "query value depth",
                                "query value depth overflow",
                            ));
                        };
                        pending.push(OwnedToValueWork::FinishSequence {
                            metadata,
                            style,
                            resolved_start: resolved.len(),
                        });
                        pending.extend(items.into_iter().rev().map(|node| {
                            OwnedToValueWork::Visit {
                                node,
                                depth: child_depth,
                            }
                        }));
                    }
                    OwnedKind::Mapping { style, entries } => {
                        let Some(child_depth) = depth.checked_add(1) else {
                            for (key, value) in entries {
                                pending.push(OwnedToValueWork::Visit { node: key, depth });
                                pending.push(OwnedToValueWork::Visit { node: value, depth });
                            }
                            discard_owned_work(pending);
                            return Err(YgError::resource_overflow(
                                "query value depth",
                                "query value depth overflow",
                            ));
                        };
                        pending.push(OwnedToValueWork::FinishMapping {
                            metadata,
                            style,
                            resolved_start: resolved.len(),
                        });
                        for (key, value) in entries.into_iter().rev() {
                            pending.push(OwnedToValueWork::Visit {
                                node: value,
                                depth: child_depth,
                            });
                            pending.push(OwnedToValueWork::Visit {
                                node: key,
                                depth: child_depth,
                            });
                        }
                    }
                    OwnedKind::Alias { .. } => unreachable!("aliases returned above"),
                }
            }
            OwnedToValueWork::FinishSequence {
                metadata,
                style,
                resolved_start,
            } => {
                let items = resolved.split_off(resolved_start);
                resolved.push(finish_owned_value(
                    metadata,
                    Data::Seq(Rc::new(YgSeq(items))),
                    NodeStyle::Collection(style),
                    None,
                    None,
                    session,
                    anchors,
                ));
            }
            OwnedToValueWork::FinishMapping {
                metadata,
                style,
                resolved_start,
            } => {
                let mut nodes = resolved.split_off(resolved_start).into_iter();
                let mut mapping = YgMap::default();
                while let Some(key) = nodes.next() {
                    let value = nodes
                        .next()
                        .expect("owned mapping conversion has key/value parity");
                    mapping.0.insert(key, value);
                }
                resolved.push(finish_owned_value(
                    metadata,
                    Data::Map(Rc::new(mapping)),
                    NodeStyle::Collection(style),
                    None,
                    None,
                    session,
                    anchors,
                ));
            }
        }
    }

    if resolved.len() != 1 {
        return Err(YgError::resource_overflow(
            "query value conversion",
            "query value conversion work imbalance",
        ));
    }
    Ok(resolved
        .pop()
        .expect("owned conversion reconstructs exactly one root"))
}

enum AssignSessionWork {
    Visit(YamlVal),
    FinishSequence {
        meta: Option<Rc<Meta>>,
        runtime_resolution_error: Option<Arc<ResolveError>>,
        resolved_start: usize,
    },
    FinishMapping {
        meta: Option<Rc<Meta>>,
        runtime_resolution_error: Option<Arc<ResolveError>>,
        resolved_start: usize,
    },
}

struct OwnedValueMetadata {
    origin: ResourceId,
    anchor: Option<String>,
    tag: Option<String>,
}

enum OwnedToValueWork {
    Visit {
        node: OwnedNode,
        depth: usize,
    },
    FinishSequence {
        metadata: OwnedValueMetadata,
        style: CollectionStyle,
        resolved_start: usize,
    },
    FinishMapping {
        metadata: OwnedValueMetadata,
        style: CollectionStyle,
        resolved_start: usize,
    },
}

fn finish_owned_value(
    metadata: OwnedValueMetadata,
    data: Data,
    style: NodeStyle,
    raw: Option<Rc<str>>,
    scalar_presentation: Option<ScalarPresentation>,
    session: ResolutionSessionKey,
    anchors: &mut HashMap<String, YamlVal>,
) -> YamlVal {
    let anchor = metadata.anchor.as_deref().map(Rc::from);
    let value = YamlVal {
        data,
        meta: Some(Rc::new(Meta {
            tag: metadata.tag.map(Rc::from),
            anchor,
            alias_of: None,
            style: Some(style),
            raw,
            scalar_presentation,
            origin: Some(ResourceOrigin {
                resource: metadata.origin,
                session,
            }),
        })),
        runtime_resolution_error: None,
    };
    if let Some(anchor) = metadata.anchor {
        anchors.insert(anchor, value.clone());
    }
    value
}

fn discard_owned_work(mut pending: Vec<OwnedToValueWork>) {
    while let Some(work) = pending.pop() {
        let OwnedToValueWork::Visit { mut node, .. } = work else {
            continue;
        };
        match std::mem::replace(
            &mut node.kind,
            OwnedKind::Sequence {
                style: CollectionStyle::Block,
                items: Vec::new(),
            },
        ) {
            OwnedKind::Sequence { items, .. } => {
                pending.extend(
                    items
                        .into_iter()
                        .map(|node| OwnedToValueWork::Visit { node, depth: 0 }),
                );
            }
            OwnedKind::Mapping { entries, .. } => {
                pending.extend(
                    entries
                        .into_iter()
                        .flat_map(<[_; 2]>::from)
                        .map(|node| OwnedToValueWork::Visit { node, depth: 0 }),
                );
            }
            OwnedKind::Scalar(_) | OwnedKind::Alias { .. } => {}
        }
    }
}

#[allow(clippy::result_large_err)] // QueryResolver returns the shared typed error directly.
pub(crate) fn value_to_owned_node(
    value: YamlVal,
    fallback: &ResourceId,
    capability: ResolutionCapability,
    limits: &ResourceLimits,
) -> Result<OwnedNode, ResolveError> {
    let mut pending = vec![ValueToOwnedWork::Visit { value, depth: 0 }];
    let mut resolved = Vec::new();

    while let Some(work) = pending.pop() {
        match work {
            ValueToOwnedWork::Visit { value, depth } => {
                limits
                    .check_depth(depth)
                    .map_err(|error| value_depth_limit(error, capability))?;
                let origin = value
                    .meta()
                    .and_then(|metadata| metadata.origin.as_ref())
                    .map_or_else(|| fallback.clone(), |origin| origin.resource.clone());
                let metadata = value.meta().cloned().unwrap_or_default();
                if let Some(name) = metadata.alias_of {
                    resolved.push(OwnedNode {
                        origin,
                        span: Default::default(),
                        anchor: None,
                        tag: None,
                        kind: OwnedKind::Alias {
                            name: name.to_string(),
                        },
                    });
                    continue;
                }
                let node = ValueNodeMetadata {
                    origin,
                    anchor: metadata.anchor.as_ref().map(ToString::to_string),
                    tag: metadata.tag.as_ref().map(ToString::to_string),
                };
                match value.data {
                    Data::Seq(values) => {
                        let child_depth = checked_resolution_depth(depth, capability)?;
                        let len = values.0.len();
                        pending.push(ValueToOwnedWork::FinishSequence {
                            node,
                            style: collection_style(metadata.style),
                            len,
                        });
                        pending.extend(values.0.iter().rev().cloned().map(|value| {
                            ValueToOwnedWork::Visit {
                                value,
                                depth: child_depth,
                            }
                        }));
                    }
                    Data::Map(mapping) => {
                        let child_depth = checked_resolution_depth(depth, capability)?;
                        let len = mapping.0.len();
                        pending.push(ValueToOwnedWork::FinishMapping {
                            node,
                            style: collection_style(metadata.style),
                            len,
                        });
                        for (key, value) in mapping.0.iter().rev() {
                            pending.push(ValueToOwnedWork::Visit {
                                value: value.clone(),
                                depth: child_depth,
                            });
                            pending.push(ValueToOwnedWork::Visit {
                                value: key.clone(),
                                depth: child_depth,
                            });
                        }
                    }
                    data => resolved
                        .push(node.with_kind(OwnedKind::Scalar(canonical_scalar(data, &metadata)))),
                }
            }
            ValueToOwnedWork::FinishSequence { node, style, len } => {
                let start = resolved
                    .len()
                    .checked_sub(len)
                    .ok_or_else(|| value_depth_overflow(capability, "query sequence conversion"))?;
                let items = resolved.split_off(start);
                resolved.push(node.with_kind(OwnedKind::Sequence { style, items }));
            }
            ValueToOwnedWork::FinishMapping { node, style, len } => {
                let node_count = len
                    .checked_mul(2)
                    .ok_or_else(|| value_depth_overflow(capability, "query mapping conversion"))?;
                let start = resolved
                    .len()
                    .checked_sub(node_count)
                    .ok_or_else(|| value_depth_overflow(capability, "query mapping conversion"))?;
                let nodes = resolved.split_off(start);
                let mut nodes = nodes.into_iter();
                let mut entries = Vec::with_capacity(len);
                while let Some(key) = nodes.next() {
                    let value = nodes.next().ok_or_else(|| {
                        value_depth_overflow(capability, "query mapping conversion")
                    })?;
                    entries.push((key, value));
                }
                resolved.push(node.with_kind(OwnedKind::Mapping { style, entries }));
            }
        }
    }

    if resolved.len() != 1 {
        return Err(value_depth_overflow(capability, "query value conversion"));
    }
    resolved
        .pop()
        .ok_or_else(|| value_depth_overflow(capability, "query value conversion"))
}

struct ValueNodeMetadata {
    origin: ResourceId,
    anchor: Option<String>,
    tag: Option<String>,
}

impl ValueNodeMetadata {
    fn with_kind(self, kind: OwnedKind) -> OwnedNode {
        OwnedNode {
            origin: self.origin,
            span: Default::default(),
            anchor: self.anchor,
            tag: self.tag,
            kind,
        }
    }
}

enum ValueToOwnedWork {
    Visit {
        value: YamlVal,
        depth: usize,
    },
    FinishSequence {
        node: ValueNodeMetadata,
        style: CollectionStyle,
        len: usize,
    },
    FinishMapping {
        node: ValueNodeMetadata,
        style: CollectionStyle,
        len: usize,
    },
}

fn canonical_scalar(data: Data, metadata: &Meta) -> OwnedScalar {
    let (value, default_style) = match data {
        Data::Null => ("null".to_owned(), ScalarStyle::Plain),
        Data::Bool(value) => (value.to_string(), ScalarStyle::Plain),
        Data::Int(value) => (value.to_string(), ScalarStyle::Plain),
        Data::Float(value) if value.is_nan() => (".nan".to_owned(), ScalarStyle::Plain),
        Data::Float(value) if value == f64::INFINITY => (".inf".to_owned(), ScalarStyle::Plain),
        Data::Float(value) if value == f64::NEG_INFINITY => {
            ("-.inf".to_owned(), ScalarStyle::Plain)
        }
        Data::Float(value) => (value.to_string(), ScalarStyle::Plain),
        Data::Str(value) => (value.to_string(), ScalarStyle::DoubleQuoted),
        Data::Seq(_) | Data::Map(_) => unreachable!("collections handled above"),
    };
    let style = match metadata.style {
        Some(NodeStyle::Scalar(style)) => style,
        Some(NodeStyle::Collection(_)) | None => default_style,
    };
    let presentation = metadata.scalar_presentation.unwrap_or(ScalarPresentation {
        chomp: Chomp::Clip,
        flags: AtomFlags::empty(),
    });
    OwnedScalar {
        value: metadata.raw.as_deref().map_or(value, str::to_owned),
        style,
        chomp: presentation.chomp,
        flags: presentation.flags,
    }
}

const fn collection_style(style: Option<NodeStyle>) -> CollectionStyle {
    match style {
        Some(NodeStyle::Collection(style)) => style,
        Some(NodeStyle::Scalar(_)) | None => CollectionStyle::Block,
    }
}

#[allow(clippy::result_large_err)] // QueryResolver returns the shared typed error directly.
fn checked_resolution_depth(
    depth: usize,
    capability: ResolutionCapability,
) -> Result<usize, ResolveError> {
    depth
        .checked_add(1)
        .ok_or_else(|| ResolveError::Resolution {
            category: yamalgam_parser::ResolutionErrorCategory::ResourceOverflow,
            capability,
            span: Default::default(),
            target: "<query value>".into(),
            reference_chain: Vec::new(),
            source: None,
        })
}

fn value_depth_limit(error: LimitExceeded, capability: ResolutionCapability) -> ResolveError {
    ResolveError::Resolution {
        category: yamalgam_parser::ResolutionErrorCategory::Limit,
        capability,
        span: Default::default(),
        target: "<query value>".into(),
        reference_chain: Vec::new(),
        source: Some(Box::new(error)),
    }
}

fn value_depth_overflow(capability: ResolutionCapability, resource: &'static str) -> ResolveError {
    ResolveError::Resolution {
        category: yamalgam_parser::ResolutionErrorCategory::ResourceOverflow,
        capability,
        span: Default::default(),
        target: "<query value>".into(),
        reference_chain: Vec::new(),
        source: Some(Box::new(crate::value::ValueLimitError::ResourceOverflow(
            resource,
        ))),
    }
}

impl From<DocumentError> for YgError {
    fn from(error: DocumentError) -> Self {
        Self::InputParse(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEEP_NESTING: usize = 20_000;
    const SMALL_STACK: usize = 128 * 1024;

    #[test]
    fn owned_tree_conversion_is_stack_safe_when_depth_is_unchecked() {
        std::thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn(|| {
                let origin = ResourceId::anonymous();
                let node = deep_owned_sequence(&origin);

                let value = value_from_owned_node(
                    node,
                    ResolutionSessionKey::UNREGISTERED,
                    &LoaderConfig::unchecked(),
                )
                .unwrap();
                assert_eq!(nested_sequence_depth(&value), DEEP_NESTING);
            })
            .unwrap()
            .join()
            .expect("iterative owned conversion must not overflow a small thread stack");
    }

    #[test]
    fn owned_tree_depth_failure_discards_remaining_work_without_recursion() {
        std::thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn(|| {
                let origin = ResourceId::anonymous();
                let node = deep_owned_sequence(&origin);
                let mut config = LoaderConfig::unchecked();
                config.limits.max_depth = Some(8);

                let error =
                    value_from_owned_node(node, ResolutionSessionKey::UNREGISTERED, &config)
                        .unwrap_err();

                assert!(matches!(error, YgError::InputLimit { .. }));
            })
            .unwrap()
            .join()
            .expect("depth failure cleanup must not overflow a small thread stack");
    }

    #[test]
    fn session_assignment_is_stack_safe_and_updates_every_origin() {
        std::thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn(|| {
                let resource = ResourceId::anonymous();
                let unregistered = ResourceOrigin {
                    resource,
                    session: ResolutionSessionKey::UNREGISTERED,
                };
                let mut value = YamlVal::str("leaf").with_meta(Meta {
                    origin: Some(unregistered.clone()),
                    ..Meta::default()
                });
                for _ in 0..DEEP_NESTING {
                    value = YamlVal::seq(vec![value]).with_meta(Meta {
                        origin: Some(unregistered.clone()),
                        ..Meta::default()
                    });
                }

                let registered = ResolutionSessionKey::registered(7, 11);
                assign_session(&mut value, registered);

                let mut current = &value;
                let mut depth = 0;
                loop {
                    assert_eq!(
                        current
                            .meta()
                            .and_then(|metadata| metadata.origin.as_ref())
                            .map(|origin| origin.session),
                        Some(registered)
                    );
                    let Some(sequence) = current.as_sequence() else {
                        break;
                    };
                    let [child] = sequence else {
                        panic!("deep assignment fixture must remain unary");
                    };
                    current = child;
                    depth += 1;
                }
                assert_eq!(depth, DEEP_NESTING);
                assert_eq!(current.as_str_content(), Some("leaf"));
            })
            .unwrap()
            .join()
            .expect("iterative session assignment must not overflow a small thread stack");
    }

    #[test]
    fn session_assignment_preserves_mapping_order_and_shared_values() {
        let resource = ResourceId::anonymous();
        let origin = ResourceOrigin {
            resource,
            session: ResolutionSessionKey::UNREGISTERED,
        };
        let metadata = || Meta {
            origin: Some(origin.clone()),
            ..Meta::default()
        };
        let original = YamlVal::map(vec![
            (
                YamlVal::str("first").with_meta(metadata()),
                YamlVal::int(1).with_meta(metadata()),
            ),
            (
                YamlVal::str("second").with_meta(metadata()),
                YamlVal::int(2).with_meta(metadata()),
            ),
        ])
        .with_meta(metadata());
        let mut assigned = original.clone();
        let registered = ResolutionSessionKey::registered(13, 17);

        assign_session(&mut assigned, registered);

        let assigned_entries = assigned.entries().unwrap().collect::<Vec<_>>();
        assert_eq!(
            assigned_entries
                .iter()
                .map(|(key, _)| key.as_str_content().unwrap())
                .collect::<Vec<_>>(),
            ["first", "second"]
        );
        assert!(assigned_entries.iter().all(|(key, value)| {
            has_session(key, registered) && has_session(value, registered)
        }));
        assert!(original.entries().unwrap().all(|(key, value)| {
            has_session(key, ResolutionSessionKey::UNREGISTERED)
                && has_session(value, ResolutionSessionKey::UNREGISTERED)
        }));
    }

    fn nested_sequence_depth(value: &YamlVal) -> usize {
        let mut current = value;
        let mut depth = 0;
        while let Some(sequence) = current.as_sequence() {
            let [child] = sequence else {
                panic!("deep conversion fixture must remain unary");
            };
            current = child;
            depth += 1;
        }
        assert_eq!(current.as_str_content(), Some("leaf"));
        depth
    }

    fn deep_owned_sequence(origin: &ResourceId) -> OwnedNode {
        let mut node = OwnedNode {
            origin: origin.clone(),
            span: Default::default(),
            anchor: None,
            tag: None,
            kind: OwnedKind::Scalar(OwnedScalar {
                value: "leaf".into(),
                style: ScalarStyle::Plain,
                chomp: Chomp::Clip,
                flags: AtomFlags::DIRECT_OUTPUT,
            }),
        };
        for _ in 0..DEEP_NESTING {
            node = OwnedNode {
                origin: origin.clone(),
                span: Default::default(),
                anchor: None,
                tag: None,
                kind: OwnedKind::Sequence {
                    style: CollectionStyle::Block,
                    items: vec![node],
                },
            };
        }
        node
    }

    fn has_session(value: &YamlVal, session: ResolutionSessionKey) -> bool {
        value
            .meta()
            .and_then(|metadata| metadata.origin.as_ref())
            .is_some_and(|origin| origin.session == session)
    }
}

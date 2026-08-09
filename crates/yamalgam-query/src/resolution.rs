//! Query-facing resource resolution and origin metadata.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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
    let value = owned_to_value(node, session, config, &mut anchors, 0)?;
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
    if let Some(metadata) = &mut value.meta
        && let Some(origin) = &mut Rc::make_mut(metadata).origin
    {
        origin.session = session;
    }
    match &mut value.data {
        Data::Seq(items) => {
            for item in &mut Rc::make_mut(items).0 {
                assign_session(item, session);
            }
        }
        Data::Map(mapping) => {
            let entries = std::mem::take(&mut Rc::make_mut(mapping).0);
            for (mut key, mut value) in entries {
                assign_session(&mut key, session);
                assign_session(&mut value, session);
                Rc::make_mut(mapping).0.insert(key, value);
            }
        }
        Data::Null | Data::Bool(_) | Data::Int(_) | Data::Float(_) | Data::Str(_) => {}
    }
}

fn owned_to_value(
    node: OwnedNode,
    session: ResolutionSessionKey,
    config: &LoaderConfig,
    anchors: &mut HashMap<String, YamlVal>,
    depth: usize,
) -> Result<YamlVal, YgError> {
    config
        .limits
        .check_depth(depth)
        .map_err(|error| YgError::input_limit(error, error.to_string()))?;
    let OwnedNode {
        origin,
        anchor,
        tag,
        kind,
        ..
    } = node;
    if let OwnedKind::Alias { name } = kind {
        let mut value = anchors.get(&name).cloned().ok_or_else(|| {
            YgError::InputParse(format!("unknown alias *{name} in resolved resource"))
        })?;
        let metadata = value.meta_mut();
        metadata.anchor = None;
        metadata.alias_of = Some(Rc::from(name));
        metadata.origin = Some(ResourceOrigin {
            resource: origin,
            session,
        });
        return Ok(value);
    }

    let (data, style, raw, scalar_presentation) = match kind {
        OwnedKind::Scalar(scalar) => {
            let raw: Rc<str> = Rc::from(scalar.value.as_str());
            let data = crate::build::resolve_scalar_data(
                Rc::clone(&raw),
                scalar.style,
                tag.as_deref(),
                config.tag_resolution,
            );
            (
                data,
                NodeStyle::Scalar(scalar.style),
                Some(raw),
                Some(ScalarPresentation {
                    chomp: scalar.chomp,
                    flags: scalar.flags,
                }),
            )
        }
        OwnedKind::Sequence { style, items } => {
            let child_depth = checked_child_depth(depth)?;
            let mut values = Vec::with_capacity(items.len());
            for item in items {
                values.push(owned_to_value(item, session, config, anchors, child_depth)?);
            }
            (
                Data::Seq(Rc::new(YgSeq(values))),
                NodeStyle::Collection(style),
                None,
                None,
            )
        }
        OwnedKind::Mapping { style, entries } => {
            let child_depth = checked_child_depth(depth)?;
            let mut mapping = YgMap::default();
            for (key, value) in entries {
                let key = owned_to_value(key, session, config, anchors, child_depth)?;
                let value = owned_to_value(value, session, config, anchors, child_depth)?;
                mapping.0.insert(key, value);
            }
            (
                Data::Map(Rc::new(mapping)),
                NodeStyle::Collection(style),
                None,
                None,
            )
        }
        OwnedKind::Alias { .. } => unreachable!("aliases returned above"),
    };
    let value = YamlVal {
        data,
        meta: Some(Rc::new(Meta {
            tag: tag.map(Rc::from),
            anchor: anchor.as_deref().map(Rc::from),
            alias_of: None,
            style: Some(style),
            raw,
            scalar_presentation,
            origin: Some(ResourceOrigin {
                resource: origin,
                session,
            }),
        })),
        runtime_resolution_error: None,
    };
    if let Some(anchor) = anchor {
        anchors.insert(anchor, value.clone());
    }
    Ok(value)
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

fn checked_child_depth(depth: usize) -> Result<usize, YgError> {
    depth.checked_add(1).ok_or_else(|| {
        YgError::resource_overflow("query value depth", "query value depth overflow")
    })
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

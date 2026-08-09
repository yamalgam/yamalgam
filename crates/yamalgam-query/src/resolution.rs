//! Query-facing resource resolution and origin metadata.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use yamalgam_core::{LoaderConfig, ResolutionCapability};
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
    /// Run-local resolver session. This is intentionally opaque to callers.
    #[doc(hidden)]
    pub session: ResolutionSessionKey,
}

/// Opaque run-local key for one query resolution session.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ResolutionSessionKey(pub(crate) u64);

impl ResolutionSessionKey {
    pub(crate) const UNREGISTERED: Self = Self(0);
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
    let document = resolver.resolve_input(document).map_err(|error| {
        let message = error.to_string();
        YgError::input_resolution(error, message)
    })?;
    let value = value_from_owned_node(document.root, ResolutionSessionKey::UNREGISTERED, config)?;
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
    let mut inputs = Vec::with_capacity(documents.len());
    for (document_index, document) in documents.into_iter().enumerate() {
        let result: Result<ResolvedInput, YgError> = (|| {
            let mut resolver =
                SessionQueryResolver::new(retriever.clone(), config.clone(), Some(root.clone()));
            let document = resolver.resolve_input(document).map_err(|error| {
                let message = error.to_string();
                YgError::input_resolution(error, message)
            })?;
            let value =
                value_from_owned_node(document.root, ResolutionSessionKey::UNREGISTERED, config)?;
            Ok(ResolvedInput {
                value,
                resolution: Rc::new(RefCell::new(resolver)),
            })
        })();
        match result {
            Ok(input) => inputs.push(input),
            Err(error) => return Err(error.with_document_context(document_index)),
        }
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
    owned_to_value(node, session, config, &mut anchors, 0)
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
) -> Result<OwnedNode, ResolveError> {
    value_to_owned(value, fallback, capability, 0)
}

#[allow(clippy::result_large_err)] // Recursive helper preserves the typed resolver error.
fn value_to_owned(
    value: YamlVal,
    fallback: &ResourceId,
    capability: ResolutionCapability,
    depth: usize,
) -> Result<OwnedNode, ResolveError> {
    let origin = value
        .meta()
        .and_then(|metadata| metadata.origin.as_ref())
        .map_or_else(|| fallback.clone(), |origin| origin.resource.clone());
    let metadata = value.meta().cloned().unwrap_or_default();
    if let Some(name) = metadata.alias_of {
        return Ok(OwnedNode {
            origin,
            span: Default::default(),
            anchor: None,
            tag: None,
            kind: OwnedKind::Alias {
                name: name.to_string(),
            },
        });
    }
    let anchor = metadata.anchor.as_ref().map(ToString::to_string);
    let tag = metadata.tag.as_ref().map(ToString::to_string);
    let kind = match value.data {
        Data::Seq(values) => {
            let child_depth = checked_resolution_depth(depth, capability)?;
            let items = values
                .iter()
                .cloned()
                .map(|value| value_to_owned(value, fallback, capability, child_depth))
                .collect::<Result<Vec<_>, _>>()?;
            OwnedKind::Sequence {
                style: collection_style(metadata.style),
                items,
            }
        }
        Data::Map(mapping) => {
            let child_depth = checked_resolution_depth(depth, capability)?;
            let entries = mapping
                .0
                .iter()
                .map(|(key, value)| {
                    Ok((
                        value_to_owned(key.clone(), fallback, capability, child_depth)?,
                        value_to_owned(value.clone(), fallback, capability, child_depth)?,
                    ))
                })
                .collect::<Result<Vec<_>, ResolveError>>()?;
            OwnedKind::Mapping {
                style: collection_style(metadata.style),
                entries,
            }
        }
        data => OwnedKind::Scalar(canonical_scalar(data, &metadata)),
    };
    Ok(OwnedNode {
        origin,
        span: Default::default(),
        anchor,
        tag,
        kind,
    })
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

impl From<DocumentError> for YgError {
    fn from(error: DocumentError) -> Self {
        Self::InputParse(error.to_string())
    }
}

//! Shared external-resolution session state.

use std::collections::HashMap;
use std::mem;
use std::sync::Arc;

use yamalgam_core::{LimitExceeded, LoaderConfig, ResolutionCapability, ResourceLimit, Span};
use yamalgam_parser::{
    AtomFlags, Chomp, Parser, ResolutionErrorCategory, ResolveError, ScalarStyle,
};

use crate::document::{DocumentError, OwnedDocument, OwnedKind, OwnedNode};
use crate::error::{RetrieveError, SessionError};
use crate::include::{
    self, IncludeContext, IncludeExpansion, IncludeFormat, IncludeSelector, IncludeSpec,
};
use crate::ref_resolver;
use crate::report::{
    ResolutionEdge, ResolutionOutcome, ResolutionReport, ResourceKind, ResourceRecord,
};
use crate::resource::{Fragment, ResolvedReference, ResourceError, ResourceId};
use crate::retriever::{
    DiscoveryRequest, DiscoverySelector, ResourceProvenance, ResourceRequest, ResourceRetriever,
};

/// Whether traversal follows automatically enabled capabilities or one
/// explicitly requested capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolutionTrigger {
    /// Resolve capabilities configured as automatic.
    Automatic,
    /// Resolve this capability explicitly, plus any automatic capability.
    Explicit(ResolutionCapability),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ResolutionTarget {
    resource: ResourceId,
    fragment: Fragment,
}

type LoadedBytes = (Arc<[u8]>, ResourceId, bool, usize);

/// Cache, budgets, cycle state, and report for one top-level YAML document.
pub struct ResolutionSession<R> {
    retriever: R,
    config: LoaderConfig,
    cache: HashMap<ResourceId, Arc<OwnedDocument>>,
    raw_cache: HashMap<ResourceId, Arc<[u8]>>,
    text_cache: HashMap<ResourceId, Arc<str>>,
    aliases: HashMap<ResourceId, ResourceId>,
    active: Vec<ResolutionTarget>,
    edges: usize,
    materialized_bytes: usize,
    report: ResolutionReport,
}

impl<R: ResourceRetriever> ResolutionSession<R> {
    /// Create an empty session using `retriever` and one loader policy.
    #[must_use]
    pub fn new(retriever: R, config: LoaderConfig) -> Self {
        Self {
            retriever,
            config,
            cache: HashMap::new(),
            raw_cache: HashMap::new(),
            text_cache: HashMap::new(),
            aliases: HashMap::new(),
            active: Vec::new(),
            edges: 0,
            materialized_bytes: 0,
            report: ResolutionReport::default(),
        }
    }

    /// Seed the raw-document cache with the containing root document.
    pub fn prime_root(&mut self, id: ResourceId, document: OwnedDocument) {
        if self.cache.insert(id.clone(), Arc::new(document)).is_none() {
            self.report.record_resource(ResourceRecord {
                kind: resource_kind(&id),
                id,
                fetched_bytes: 0,
                cache_hits: 0,
                provenance: ResourceProvenance::default(),
            });
        }
    }

    /// Resolve automatic directives in one root document.
    ///
    /// # Errors
    ///
    /// Returns a structured [`ResolveError`] on the first failed directive.
    pub fn resolve_root(&mut self, document: OwnedDocument) -> Result<OwnedDocument, ResolveError> {
        let origin = document.root.origin.clone();
        self.prime_root(origin, document.clone());
        self.resolve_tree(document.root, ResolutionTrigger::Automatic)
            .map(|root| OwnedDocument { root })
    }

    /// Resolve one explicitly selected capability in `node`.
    ///
    /// Capabilities configured as automatic continue to resolve during the
    /// explicit traversal.
    ///
    /// # Errors
    ///
    /// Returns [`ResolutionErrorCategory::DisabledCapability`] when the
    /// selected capability is disabled, or another structured resolution
    /// failure from the directive traversal.
    pub fn resolve_explicit(
        &mut self,
        capability: ResolutionCapability,
        node: OwnedNode,
    ) -> Result<OwnedNode, ResolveError> {
        if !self.config.resolution.allows_explicit(capability) {
            return Err(self.error(
                ResolutionErrorCategory::DisabledCapability,
                capability,
                node.span,
                node.origin.to_string(),
                None,
            ));
        }
        self.resolve_tree(node, ResolutionTrigger::Explicit(capability))
    }

    /// Inspect the deterministic report accumulated so far.
    #[must_use]
    pub const fn report(&self) -> &ResolutionReport {
        &self.report
    }

    /// Consume the session and return its report.
    #[must_use]
    pub fn into_report(self) -> ResolutionReport {
        self.report
    }

    /// Finish one root report and clear every root-scoped cache and counter.
    pub fn reset_for_next_root(&mut self) -> ResolutionReport {
        self.cache.clear();
        self.raw_cache.clear();
        self.text_cache.clear();
        self.aliases.clear();
        self.active.clear();
        self.edges = 0;
        self.materialized_bytes = 0;
        mem::take(&mut self.report)
    }

    fn resolve_tree(
        &mut self,
        root: OwnedNode,
        trigger: ResolutionTrigger,
    ) -> Result<OwnedNode, ResolveError> {
        let mut pending = vec![TreeWork::Visit {
            node: root,
            context: TreeContext::Node,
        }];
        let mut resolved = Vec::new();

        while let Some(work) = pending.pop() {
            match work {
                TreeWork::Visit { node, context } => {
                    if self.should_resolve(ResolutionCapability::Ref, trigger) {
                        let directive = ref_resolver::recognize(&node, self.config.tag_resolution)
                            .map_err(|source| {
                                self.error(
                                    ResolutionErrorCategory::MalformedDirective,
                                    ResolutionCapability::Ref,
                                    node.span,
                                    node.origin.to_string(),
                                    Some(Box::new(source)),
                                )
                            })?;
                        if let Some(directive) = directive {
                            let replacement = self.resolve_requested(
                                ResolutionCapability::Ref,
                                node.origin,
                                directive.span,
                                directive.requested,
                                trigger,
                            )?;
                            resolved.push(TreeResult::Node(replacement));
                            continue;
                        }
                    }
                    if self.should_resolve(ResolutionCapability::Include, trigger) {
                        let spec = include::recognize(&node, self.config.tag_resolution).map_err(
                            |source| {
                                self.include_error(&source, node.span, node.origin.to_string())
                            },
                        )?;
                        if let Some(spec) = spec {
                            let expansion = self.resolve_include(
                                &node,
                                spec,
                                context.include_context(),
                                trigger,
                            )?;
                            resolved.push(TreeResult::from(expansion));
                            continue;
                        }
                    }

                    let OwnedNode {
                        origin,
                        span,
                        anchor,
                        tag,
                        kind,
                    } = node;
                    let metadata = NodeMetadata {
                        origin,
                        span,
                        anchor,
                        tag,
                    };
                    match kind {
                        OwnedKind::Sequence { style, items } => {
                            let len = items.len();
                            pending.push(TreeWork::FinishSequence {
                                node: metadata,
                                style,
                                len,
                            });
                            pending.extend(items.into_iter().rev().map(|node| TreeWork::Visit {
                                node,
                                context: TreeContext::SequenceItem,
                            }));
                        }
                        OwnedKind::Mapping { style, entries } => {
                            let len = entries.len();
                            pending.push(TreeWork::FinishMapping {
                                node: metadata,
                                style,
                                len,
                            });
                            for (key, value) in entries.into_iter().rev() {
                                let merge_value = scalar_value(&key) == Some("<<");
                                pending.push(TreeWork::Visit {
                                    node: value,
                                    context: if merge_value {
                                        TreeContext::MergeValue
                                    } else {
                                        TreeContext::MappingValue
                                    },
                                });
                                pending.push(TreeWork::Visit {
                                    node: key,
                                    context: TreeContext::MappingKey,
                                });
                            }
                        }
                        kind => resolved.push(TreeResult::Node(metadata.with_kind(kind))),
                    }
                }
                TreeWork::FinishSequence { node, style, len } => {
                    let start = resolved.len().checked_sub(len).ok_or_else(|| {
                        self.overflow_error(
                            ResolutionCapability::Ref,
                            node.span,
                            node.origin.to_string(),
                            "sequence reconstruction",
                        )
                    })?;
                    let children = resolved.split_off(start);
                    let mut items = Vec::new();
                    for child in children {
                        match child {
                            TreeResult::Node(node) => items.push(node),
                            TreeResult::Splice(spliced) => items.extend(spliced),
                            TreeResult::MappingMerge(_) => {
                                return Err(self.error(
                                    ResolutionErrorCategory::IncompatibleContext,
                                    ResolutionCapability::Include,
                                    node.span,
                                    node.origin.to_string(),
                                    Some(Box::new(SessionError::IncludeContext)),
                                ));
                            }
                        }
                    }
                    resolved.push(TreeResult::Node(
                        node.with_kind(OwnedKind::Sequence { style, items }),
                    ));
                }
                TreeWork::FinishMapping { node, style, len } => {
                    let node_count = len.checked_mul(2).ok_or_else(|| {
                        self.overflow_error(
                            ResolutionCapability::Ref,
                            node.span,
                            node.origin.to_string(),
                            "mapping reconstruction",
                        )
                    })?;
                    let start = resolved.len().checked_sub(node_count).ok_or_else(|| {
                        self.overflow_error(
                            ResolutionCapability::Ref,
                            node.span,
                            node.origin.to_string(),
                            "mapping reconstruction",
                        )
                    })?;
                    let nodes = resolved.split_off(start);
                    let mut explicit = Vec::with_capacity(len);
                    let mut merged = Vec::new();
                    let mut nodes = nodes.into_iter();
                    while let Some(key) = nodes.next() {
                        let value = nodes.next().ok_or_else(|| {
                            self.overflow_error(
                                ResolutionCapability::Ref,
                                node.span,
                                node.origin.to_string(),
                                "mapping reconstruction",
                            )
                        })?;
                        let TreeResult::Node(key) = key else {
                            return Err(self.error(
                                ResolutionErrorCategory::IncompatibleContext,
                                ResolutionCapability::Include,
                                node.span,
                                node.origin.to_string(),
                                Some(Box::new(SessionError::IncludeContext)),
                            ));
                        };
                        match value {
                            TreeResult::Node(value) => explicit.push((key, value)),
                            TreeResult::MappingMerge(entries) => merged.extend(entries),
                            TreeResult::Splice(_) => {
                                return Err(self.error(
                                    ResolutionErrorCategory::IncompatibleContext,
                                    ResolutionCapability::Include,
                                    node.span,
                                    node.origin.to_string(),
                                    Some(Box::new(SessionError::IncludeContext)),
                                ));
                            }
                        }
                    }
                    for entry in explicit {
                        merged.retain(|(key, _)| {
                            !include::semantic_key_eq(key, &entry.0, self.config.tag_resolution)
                        });
                        merged.push(entry);
                    }
                    resolved.push(TreeResult::Node(node.with_kind(OwnedKind::Mapping {
                        style,
                        entries: merged,
                    })));
                }
            }
        }

        let result = resolved.pop().ok_or_else(|| {
            self.overflow_error(
                ResolutionCapability::Ref,
                Span::default(),
                "<empty tree>".into(),
                "tree reconstruction",
            )
        })?;
        match result {
            TreeResult::Node(node) => Ok(node),
            TreeResult::Splice(_) | TreeResult::MappingMerge(_) => Err(self.error(
                ResolutionErrorCategory::IncompatibleContext,
                ResolutionCapability::Include,
                Span::default(),
                "<root>".into(),
                Some(Box::new(SessionError::IncludeContext)),
            )),
        }
    }

    fn resolve_include(
        &mut self,
        directive: &OwnedNode,
        spec: IncludeSpec,
        context: IncludeContext,
        trigger: ResolutionTrigger,
    ) -> Result<IncludeExpansion, ResolveError> {
        let first_edge = self.report.edges().len();
        let result = self.resolve_include_inner(directive, spec, context, trigger);
        if let Err(error) = &result
            && self.report.edges().len() > first_edge
        {
            let last_edge = self.report.edges().len() - 1;
            self.report.edge_mut(last_edge).outcome =
                ResolutionOutcome::Failed(error_category(error));
        }
        result
    }

    fn resolve_include_inner(
        &mut self,
        directive: &OwnedNode,
        spec: IncludeSpec,
        context: IncludeContext,
        trigger: ResolutionTrigger,
    ) -> Result<IncludeExpansion, ResolveError> {
        let targets = match &spec.selector {
            IncludeSelector::Path(path) => vec![IncludeTarget::Requested(path.clone())],
            IncludeSelector::Glob(pattern) => self
                .discover_include(
                    directive,
                    DiscoverySelector::Glob {
                        pattern: pattern.clone(),
                    },
                )?
                .into_iter()
                .map(IncludeTarget::Discovered)
                .collect(),
            IncludeSelector::Dir(path) => self
                .discover_include(
                    directive,
                    DiscoverySelector::Directory {
                        path: path.clone(),
                        recursive: spec.recursive,
                        matches: spec.matches.clone(),
                    },
                )?
                .into_iter()
                .map(IncludeTarget::Discovered)
                .collect(),
        };

        let target_resources = targets
            .iter()
            .map(|target| match target {
                IncludeTarget::Requested(requested) => directive
                    .origin
                    .resolve(requested)
                    .map(|resolved| resolved.resource)
                    .map_err(|source| {
                        self.error(
                            ResolutionErrorCategory::InvalidResource,
                            ResolutionCapability::Include,
                            directive.span,
                            directive.origin.to_string(),
                            Some(Box::new(source)),
                        )
                    }),
                IncludeTarget::Discovered(resource) => Ok(resource.clone()),
            })
            .collect::<Result<Vec<_>, _>>()?;
        include::validate_names(&spec, directive, &target_resources).map_err(|source| {
            self.include_error(&source, directive.span, directive.origin.to_string())
        })?;

        let mut resources = Vec::with_capacity(targets.len());
        for target in targets {
            let (resource, node) = match (spec.format, target) {
                (IncludeFormat::Yaml, IncludeTarget::Requested(requested)) => {
                    let resolved = directive.origin.resolve(&requested).map_err(|source| {
                        self.error(
                            ResolutionErrorCategory::InvalidResource,
                            ResolutionCapability::Include,
                            directive.span,
                            directive.origin.to_string(),
                            Some(Box::new(source)),
                        )
                    })?;
                    let resource = resolved.resource.clone();
                    let node = self.resolve_requested(
                        ResolutionCapability::Include,
                        directive.origin.clone(),
                        directive.span,
                        requested,
                        trigger,
                    )?;
                    (resource, node)
                }
                (IncludeFormat::Yaml, IncludeTarget::Discovered(resource)) => {
                    let node =
                        self.resolve_discovered_yaml(directive, resource.clone(), trigger)?;
                    (resource, node)
                }
                (IncludeFormat::Text, IncludeTarget::Requested(requested)) => {
                    let resolved = directive.origin.resolve(&requested).map_err(|source| {
                        self.error(
                            ResolutionErrorCategory::InvalidResource,
                            ResolutionCapability::Include,
                            directive.span,
                            directive.origin.to_string(),
                            Some(Box::new(source)),
                        )
                    })?;
                    let resource = resolved.resource.clone();
                    let node = self.resolve_text(directive, resolved, requested)?;
                    (resource, node)
                }
                (IncludeFormat::Text, IncludeTarget::Discovered(resource)) => {
                    let node = self.resolve_text(
                        directive,
                        ResolvedReference {
                            resource: resource.clone(),
                            fragment: Fragment::Root,
                        },
                        resource.to_string(),
                    )?;
                    (resource, node)
                }
            };
            resources.push((resource, node));
        }

        let expansion = include::aggregate(
            &spec,
            context,
            directive,
            resources,
            self.config.tag_resolution,
        )
        .map_err(|source| {
            self.include_error(&source, directive.span, directive.origin.to_string())
        })?;
        let additional_weight = match &expansion {
            IncludeExpansion::Node {
                additional_weight, ..
            }
            | IncludeExpansion::Splice {
                additional_weight, ..
            }
            | IncludeExpansion::MappingMerge {
                additional_weight, ..
            } => *additional_weight,
        };
        self.charge_materialized(
            ResolutionCapability::Include,
            directive.span,
            directive.origin.to_string(),
            additional_weight,
        )?;
        Ok(expansion)
    }

    fn discover_include(
        &self,
        directive: &OwnedNode,
        selector: DiscoverySelector,
    ) -> Result<Vec<ResourceId>, ResolveError> {
        if matches!(directive.origin.scheme(), "http" | "https") {
            return Err(self.error(
                ResolutionErrorCategory::PolicyDenied,
                ResolutionCapability::Include,
                directive.span,
                directive.origin.to_string(),
                Some(Box::new(RetrieveError::PolicyDenied {
                    id: directive.origin.clone(),
                })),
            ));
        }
        let max_matches = self
            .config
            .resolution
            .limits
            .max_edges
            .map(|maximum| maximum.saturating_sub(self.edges));
        self.retriever
            .discover(&DiscoveryRequest {
                base: directive.origin.clone(),
                selector,
                max_matches,
            })
            .map_err(|source| {
                self.error(
                    retrieve_category(&source),
                    ResolutionCapability::Include,
                    directive.span,
                    directive.origin.to_string(),
                    Some(Box::new(source)),
                )
            })
    }

    fn resolve_discovered_yaml(
        &mut self,
        directive: &OwnedNode,
        resource: ResourceId,
        trigger: ResolutionTrigger,
    ) -> Result<OwnedNode, ResolveError> {
        let requested = resource.to_string();
        let edge_index = self.report.record_edge(ResolutionEdge {
            parent: directive.origin.clone(),
            child: Some(resource.clone()),
            capability: ResolutionCapability::Include,
            source_span: directive.span,
            requested: requested.clone(),
            fragment: Fragment::Root,
            matches: Vec::new(),
            cache_hit: false,
            fetched_bytes: 0,
            materialized_bytes: 0,
            outcome: ResolutionOutcome::Resolved,
        });
        let result = self
            .charge_edge(ResolutionCapability::Include, directive.span, &requested)
            .and_then(|()| {
                self.resolve_yaml_reference_inner(
                    edge_index,
                    ResolutionCapability::Include,
                    directive.span,
                    &requested,
                    ResolvedReference {
                        resource,
                        fragment: Fragment::Root,
                    },
                    trigger,
                )
            });
        if let Err(error) = &result {
            self.report.edge_mut(edge_index).outcome =
                ResolutionOutcome::Failed(error_category(error));
        }
        result
    }

    fn resolve_text(
        &mut self,
        directive: &OwnedNode,
        reference: ResolvedReference,
        requested: String,
    ) -> Result<OwnedNode, ResolveError> {
        let edge_index = self.report.record_edge(ResolutionEdge {
            parent: directive.origin.clone(),
            child: Some(reference.resource.clone()),
            capability: ResolutionCapability::Include,
            source_span: directive.span,
            requested: requested.clone(),
            fragment: Fragment::Root,
            matches: Vec::new(),
            cache_hit: false,
            fetched_bytes: 0,
            materialized_bytes: 0,
            outcome: ResolutionOutcome::Resolved,
        });
        let result = self.resolve_text_inner(edge_index, directive, reference, &requested);
        if let Err(error) = &result {
            self.report.edge_mut(edge_index).outcome =
                ResolutionOutcome::Failed(error_category(error));
        }
        result
    }

    fn resolve_text_inner(
        &mut self,
        edge_index: usize,
        directive: &OwnedNode,
        reference: ResolvedReference,
        requested: &str,
    ) -> Result<OwnedNode, ResolveError> {
        let capability = ResolutionCapability::Include;
        let next_edges = self.edges.checked_add(1).ok_or_else(|| {
            self.overflow_error(
                capability,
                directive.span,
                directive.origin.to_string(),
                "resolution edge count",
            )
        })?;
        self.edges = next_edges;
        self.check_limit(
            capability,
            directive.span,
            requested,
            ResourceLimit::ResolutionEdges,
            next_edges,
            self.config.resolution.limits.max_edges,
        )?;
        self.enforce_policy(capability, directive.span, &reference)?;
        let depth = self.active.len().checked_add(1).ok_or_else(|| {
            self.overflow_error(
                capability,
                directive.span,
                target_name(&reference),
                "resolution depth",
            )
        })?;
        self.check_limit(
            capability,
            directive.span,
            &target_name(&reference),
            ResourceLimit::ResolutionDepth,
            depth,
            self.config.resolution.limits.max_depth,
        )?;

        let (text, canonical, cache_hit, fetched_bytes) =
            self.load_text(capability, directive.span, &reference.resource)?;
        let target = ResolutionTarget {
            resource: canonical.clone(),
            fragment: Fragment::Root,
        };
        if let Some(cycle_start) = self.active.iter().position(|active| active == &target) {
            let mut chain = self.active[cycle_start..]
                .iter()
                .map(target_label)
                .collect::<Vec<_>>();
            chain.push(target_label(&target));
            return Err(ResolveError::Resolution {
                category: ResolutionErrorCategory::Cycle,
                capability,
                span: directive.span,
                target: target_label(&target),
                reference_chain: chain,
                source: None,
            });
        }
        {
            let edge = self.report.edge_mut(edge_index);
            edge.child = Some(canonical.clone());
            edge.cache_hit = cache_hit;
            edge.fetched_bytes = fetched_bytes;
        }
        let node = OwnedNode {
            origin: canonical,
            span: directive.span,
            anchor: None,
            tag: None,
            kind: OwnedKind::Scalar(crate::OwnedScalar {
                value: text.to_string(),
                style: ScalarStyle::DoubleQuoted,
                chomp: Chomp::Clip,
                flags: AtomFlags::empty(),
            }),
        };
        let weight = node.materialized_weight().map_err(|source| {
            self.error(
                ResolutionErrorCategory::ResourceOverflow,
                capability,
                directive.span,
                requested.into(),
                Some(Box::new(source)),
            )
        })?;
        self.report.edge_mut(edge_index).materialized_bytes = weight;
        self.charge_materialized(capability, directive.span, requested.into(), weight)?;
        Ok(node)
    }

    fn load_text(
        &mut self,
        capability: ResolutionCapability,
        span: Span,
        requested: &ResourceId,
    ) -> Result<(Arc<str>, ResourceId, bool, usize), ResolveError> {
        let canonical = self
            .aliases
            .get(requested)
            .cloned()
            .unwrap_or_else(|| requested.clone());
        if let Some(text) = self.text_cache.get(&canonical).cloned() {
            self.bump_cache_hit(capability, span, &canonical)?;
            return Ok((text, canonical, true, 0));
        }
        let (bytes, canonical, cache_hit, fetched_bytes) =
            self.load_bytes(capability, span, requested)?;
        let text = std::str::from_utf8(&bytes).map_err(|source| {
            self.error(
                ResolutionErrorCategory::InvalidUtf8,
                capability,
                span,
                canonical.to_string(),
                Some(Box::new(SessionError::InvalidUtf8(source))),
            )
        })?;
        let text: Arc<str> = Arc::from(text);
        self.text_cache.insert(canonical.clone(), Arc::clone(&text));
        Ok((text, canonical, cache_hit, fetched_bytes))
    }

    fn load_bytes(
        &mut self,
        capability: ResolutionCapability,
        span: Span,
        requested: &ResourceId,
    ) -> Result<LoadedBytes, ResolveError> {
        let canonical = self
            .aliases
            .get(requested)
            .cloned()
            .unwrap_or_else(|| requested.clone());
        if let Some(bytes) = self.raw_cache.get(&canonical).cloned() {
            self.bump_cache_hit(capability, span, &canonical)?;
            return Ok((bytes, canonical, true, 0));
        }
        let retrieved = self
            .retriever
            .retrieve(&ResourceRequest {
                id: requested.clone(),
                max_bytes: self.config.limits.max_input_bytes,
            })
            .map_err(|source| {
                self.error(
                    retrieve_category(&source),
                    capability,
                    span,
                    requested.to_string(),
                    Some(Box::new(source)),
                )
            })?;
        let fetched_bytes = retrieved.bytes.len();
        let canonical = retrieved.id;
        if &canonical != requested {
            self.aliases.insert(requested.clone(), canonical.clone());
        }
        self.report.record_resource(ResourceRecord {
            id: canonical.clone(),
            kind: resource_kind(&canonical),
            fetched_bytes,
            cache_hits: 0,
            provenance: retrieved.provenance,
        });
        let bytes: Arc<[u8]> = Arc::from(retrieved.bytes);
        self.raw_cache.insert(canonical.clone(), Arc::clone(&bytes));
        Ok((bytes, canonical, false, fetched_bytes))
    }

    fn bump_cache_hit(
        &mut self,
        capability: ResolutionCapability,
        span: Span,
        canonical: &ResourceId,
    ) -> Result<(), ResolveError> {
        let overflow = self.overflow_error(
            capability,
            span,
            canonical.to_string(),
            "resource cache-hit count",
        );
        if let Some(record) = self.report.resource_mut(canonical) {
            record.cache_hits = record.cache_hits.checked_add(1).ok_or(overflow)?;
        }
        Ok(())
    }

    fn charge_materialized(
        &mut self,
        capability: ResolutionCapability,
        span: Span,
        target: String,
        weight: usize,
    ) -> Result<(), ResolveError> {
        let next = self.materialized_bytes.checked_add(weight).ok_or_else(|| {
            self.overflow_error(capability, span, target.clone(), "materialized byte count")
        })?;
        self.check_limit(
            capability,
            span,
            &target,
            ResourceLimit::MaterializedBytes,
            next,
            self.config.resolution.limits.max_materialized_bytes,
        )?;
        self.materialized_bytes = next;
        Ok(())
    }

    fn include_error(&self, source: &SessionError, span: Span, target: String) -> ResolveError {
        let category = match source {
            SessionError::MalformedInclude(_) => ResolutionErrorCategory::MalformedDirective,
            SessionError::IncludeConflict => ResolutionErrorCategory::Conflict,
            SessionError::IncludeContext => ResolutionErrorCategory::IncompatibleContext,
            SessionError::ResourceOverflow(_) => ResolutionErrorCategory::ResourceOverflow,
            SessionError::IncludeMatchCount { .. }
            | SessionError::IncludeAggregateType
            | SessionError::IncludeName => ResolutionErrorCategory::AggregateType,
            _ => ResolutionErrorCategory::MalformedDirective,
        };
        self.error(
            category,
            ResolutionCapability::Include,
            span,
            target,
            Some(Box::new(source.clone())),
        )
    }

    fn resolve_requested(
        &mut self,
        capability: ResolutionCapability,
        parent: ResourceId,
        span: Span,
        requested: String,
        trigger: ResolutionTrigger,
    ) -> Result<OwnedNode, ResolveError> {
        let edge_index = self.report.record_edge(ResolutionEdge {
            parent: parent.clone(),
            child: None,
            capability,
            source_span: span,
            requested: requested.clone(),
            fragment: Fragment::Root,
            matches: Vec::new(),
            cache_hit: false,
            fetched_bytes: 0,
            materialized_bytes: 0,
            outcome: ResolutionOutcome::Resolved,
        });

        let result = self
            .resolve_requested_inner(edge_index, capability, &parent, span, &requested, trigger);
        if let Err(error) = &result {
            self.report.edge_mut(edge_index).outcome =
                ResolutionOutcome::Failed(error_category(error));
        }
        result
    }

    fn resolve_requested_inner(
        &mut self,
        edge_index: usize,
        capability: ResolutionCapability,
        parent: &ResourceId,
        span: Span,
        requested: &str,
        trigger: ResolutionTrigger,
    ) -> Result<OwnedNode, ResolveError> {
        self.charge_edge(capability, span, requested)?;
        let reference = parent.resolve(requested).map_err(|source| {
            let category = match source {
                ResourceError::MissingBase => ResolutionErrorCategory::MissingBase,
                _ => ResolutionErrorCategory::InvalidResource,
            };
            self.error(
                category,
                capability,
                span,
                parent.to_string(),
                Some(Box::new(source)),
            )
        })?;
        self.resolve_yaml_reference_inner(
            edge_index, capability, span, requested, reference, trigger,
        )
    }

    fn resolve_yaml_reference_inner(
        &mut self,
        edge_index: usize,
        capability: ResolutionCapability,
        span: Span,
        requested: &str,
        reference: ResolvedReference,
        trigger: ResolutionTrigger,
    ) -> Result<OwnedNode, ResolveError> {
        self.enforce_policy(capability, span, &reference)?;
        {
            let edge = self.report.edge_mut(edge_index);
            edge.child = Some(reference.resource.clone());
            edge.fragment = reference.fragment.clone();
        }

        let depth = self.active.len().checked_add(1).ok_or_else(|| {
            self.overflow_error(
                capability,
                span,
                target_name(&reference),
                "resolution depth",
            )
        })?;
        self.check_limit(
            capability,
            span,
            &target_name(&reference),
            ResourceLimit::ResolutionDepth,
            depth,
            self.config.resolution.limits.max_depth,
        )?;

        let (document, canonical, cache_hit, fetched_bytes) =
            self.load_document(capability, span, &reference.resource)?;
        {
            let edge = self.report.edge_mut(edge_index);
            edge.child = Some(canonical.clone());
            edge.cache_hit = cache_hit;
            edge.fetched_bytes = fetched_bytes;
        }

        let target = ResolutionTarget {
            resource: canonical,
            fragment: reference.fragment.clone(),
        };
        if let Some(cycle_start) = self.active.iter().position(|active| active == &target) {
            let mut chain = self.active[cycle_start..]
                .iter()
                .map(target_label)
                .collect::<Vec<_>>();
            chain.push(target_label(&target));
            return Err(ResolveError::Resolution {
                category: ResolutionErrorCategory::Cycle,
                capability,
                span,
                target: target_label(&target),
                reference_chain: chain,
                source: None,
            });
        }

        let selected =
            ref_resolver::select(&document.root, &target.fragment, self.config.tag_resolution)
                .map_err(|source| {
                    self.error(
                        ResolutionErrorCategory::MissingFragment,
                        capability,
                        span,
                        target_label(&target),
                        Some(Box::new(source)),
                    )
                })?;

        self.active.push(target);
        let resolved = self.resolve_tree(selected, trigger);
        self.active.pop();
        let resolved = resolved?;

        let weight = resolved.materialized_weight().map_err(|source| {
            self.error(
                ResolutionErrorCategory::ResourceOverflow,
                capability,
                span,
                requested.into(),
                Some(Box::new(source)),
            )
        })?;
        self.report.edge_mut(edge_index).materialized_bytes = weight;
        let next_materialized = self.materialized_bytes.checked_add(weight).ok_or_else(|| {
            self.overflow_error(
                capability,
                span,
                requested.into(),
                "materialized byte count",
            )
        })?;
        self.check_limit(
            capability,
            span,
            requested,
            ResourceLimit::MaterializedBytes,
            next_materialized,
            self.config.resolution.limits.max_materialized_bytes,
        )?;
        self.materialized_bytes = next_materialized;
        Ok(resolved)
    }

    fn charge_edge(
        &mut self,
        capability: ResolutionCapability,
        span: Span,
        target: &str,
    ) -> Result<(), ResolveError> {
        let next_edges = self.edges.checked_add(1).ok_or_else(|| {
            self.overflow_error(capability, span, target.into(), "resolution edge count")
        })?;
        self.edges = next_edges;
        self.check_limit(
            capability,
            span,
            target,
            ResourceLimit::ResolutionEdges,
            next_edges,
            self.config.resolution.limits.max_edges,
        )
    }

    fn load_document(
        &mut self,
        capability: ResolutionCapability,
        span: Span,
        requested: &ResourceId,
    ) -> Result<(Arc<OwnedDocument>, ResourceId, bool, usize), ResolveError> {
        let canonical = self
            .aliases
            .get(requested)
            .cloned()
            .unwrap_or_else(|| requested.clone());
        if let Some(document) = self.cache.get(&canonical).cloned() {
            self.bump_cache_hit(capability, span, &canonical)?;
            return Ok((document, canonical, true, 0));
        }

        let (bytes, canonical, cache_hit, fetched_bytes) =
            self.load_bytes(capability, span, requested)?;
        let input = std::str::from_utf8(&bytes).map_err(|source| {
            self.error(
                ResolutionErrorCategory::InvalidUtf8,
                capability,
                span,
                canonical.to_string(),
                Some(Box::new(SessionError::InvalidUtf8(source))),
            )
        })?;
        let events = Parser::with_config(input, &self.config)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| {
                self.error(
                    ResolutionErrorCategory::InvalidYaml,
                    capability,
                    span,
                    canonical.to_string(),
                    Some(Box::new(source)),
                )
            })?;
        let document = OwnedDocument::from_events(canonical.clone(), events).map_err(|source| {
            let category = match source {
                DocumentError::EmptyStream
                | DocumentError::EmptyDocument
                | DocumentError::MultipleDocuments => ResolutionErrorCategory::DocumentCount,
                _ => ResolutionErrorCategory::InvalidYaml,
            };
            self.error(
                category,
                capability,
                span,
                canonical.to_string(),
                Some(Box::new(source)),
            )
        })?;
        let document = Arc::new(document);
        self.cache.insert(canonical.clone(), Arc::clone(&document));
        Ok((document, canonical, cache_hit, fetched_bytes))
    }

    fn should_resolve(&self, capability: ResolutionCapability, trigger: ResolutionTrigger) -> bool {
        match trigger {
            ResolutionTrigger::Automatic => {
                self.config.resolution.resolves_automatically(capability)
            }
            ResolutionTrigger::Explicit(selected) => {
                capability == selected || self.config.resolution.resolves_automatically(capability)
            }
        }
    }

    fn enforce_policy(
        &self,
        capability: ResolutionCapability,
        span: Span,
        reference: &ResolvedReference,
    ) -> Result<(), ResolveError> {
        if capability == ResolutionCapability::Include
            && matches!(reference.resource.scheme(), "http" | "https")
        {
            return Err(self.error(
                ResolutionErrorCategory::PolicyDenied,
                capability,
                span,
                target_name(reference),
                Some(Box::new(RetrieveError::PolicyDenied {
                    id: reference.resource.clone(),
                })),
            ));
        }
        if capability != ResolutionCapability::Ref || reference.resource.as_path().is_some() {
            return Ok(());
        }
        let policy = &self.config.resolution.refs;
        let scheme_allowed = policy
            .allow_schemes
            .iter()
            .any(|scheme| scheme == reference.resource.scheme());
        let host_allowed = !matches!(reference.resource.scheme(), "http" | "https")
            || reference.resource.host_str().is_some_and(|host| {
                policy
                    .allow_hosts
                    .iter()
                    .any(|allowed| allowed.eq_ignore_ascii_case(host))
            });
        if !scheme_allowed || !host_allowed || reference.resource.has_userinfo() {
            return Err(self.error(
                ResolutionErrorCategory::PolicyDenied,
                capability,
                span,
                target_name(reference),
                Some(Box::new(RetrieveError::PolicyDenied {
                    id: reference.resource.clone(),
                })),
            ));
        }
        Ok(())
    }

    fn check_limit(
        &self,
        capability: ResolutionCapability,
        span: Span,
        target: &str,
        limit: ResourceLimit,
        observed: usize,
        maximum: Option<usize>,
    ) -> Result<(), ResolveError> {
        if let Some(maximum) = maximum
            && observed > maximum
        {
            return Err(self.error(
                ResolutionErrorCategory::Limit,
                capability,
                span,
                target.into(),
                Some(Box::new(LimitExceeded::new(limit, observed, maximum))),
            ));
        }
        Ok(())
    }

    fn overflow_error(
        &self,
        capability: ResolutionCapability,
        span: Span,
        target: String,
        counter: &'static str,
    ) -> ResolveError {
        self.error(
            ResolutionErrorCategory::ResourceOverflow,
            capability,
            span,
            target,
            Some(Box::new(SessionError::ResourceOverflow(counter))),
        )
    }

    fn error(
        &self,
        category: ResolutionErrorCategory,
        capability: ResolutionCapability,
        span: Span,
        target: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    ) -> ResolveError {
        ResolveError::Resolution {
            category,
            capability,
            span,
            target,
            reference_chain: self.active.iter().map(target_label).collect(),
            source,
        }
    }
}

#[derive(Debug)]
struct NodeMetadata {
    origin: ResourceId,
    span: Span,
    anchor: Option<String>,
    tag: Option<String>,
}

impl NodeMetadata {
    fn with_kind(self, kind: OwnedKind) -> OwnedNode {
        OwnedNode {
            origin: self.origin,
            span: self.span,
            anchor: self.anchor,
            tag: self.tag,
            kind,
        }
    }
}

enum TreeWork {
    Visit {
        node: OwnedNode,
        context: TreeContext,
    },
    FinishSequence {
        node: NodeMetadata,
        style: yamalgam_parser::CollectionStyle,
        len: usize,
    },
    FinishMapping {
        node: NodeMetadata,
        style: yamalgam_parser::CollectionStyle,
        len: usize,
    },
}

#[derive(Clone, Copy)]
enum TreeContext {
    Node,
    SequenceItem,
    MappingKey,
    MappingValue,
    MergeValue,
}

impl TreeContext {
    const fn include_context(self) -> IncludeContext {
        match self {
            Self::SequenceItem => IncludeContext::SequenceItem,
            Self::MergeValue => IncludeContext::MergeValue,
            Self::Node | Self::MappingKey | Self::MappingValue => IncludeContext::Node,
        }
    }
}

enum TreeResult {
    Node(OwnedNode),
    Splice(Vec<OwnedNode>),
    MappingMerge(Vec<(OwnedNode, OwnedNode)>),
}

impl From<IncludeExpansion> for TreeResult {
    fn from(expansion: IncludeExpansion) -> Self {
        match expansion {
            IncludeExpansion::Node { node, .. } => Self::Node(node),
            IncludeExpansion::Splice { items, .. } => Self::Splice(items),
            IncludeExpansion::MappingMerge { entries, .. } => Self::MappingMerge(entries),
        }
    }
}

enum IncludeTarget {
    Requested(String),
    Discovered(ResourceId),
}

fn scalar_value(node: &OwnedNode) -> Option<&str> {
    match &node.kind {
        OwnedKind::Scalar(scalar) => Some(&scalar.value),
        _ => None,
    }
}

const fn retrieve_category(error: &RetrieveError) -> ResolutionErrorCategory {
    match error {
        RetrieveError::PolicyDenied { .. }
        | RetrieveError::OutsideRoot(_)
        | RetrieveError::SymlinkDenied(_) => ResolutionErrorCategory::PolicyDenied,
        _ => ResolutionErrorCategory::Retrieval,
    }
}

const fn error_category(error: &ResolveError) -> ResolutionErrorCategory {
    match error {
        ResolveError::Resolution { category, .. } => *category,
        ResolveError::LimitExceeded(_) => ResolutionErrorCategory::Limit,
        ResolveError::Cycle { .. } => ResolutionErrorCategory::Cycle,
        _ => ResolutionErrorCategory::Retrieval,
    }
}

fn resource_kind(id: &ResourceId) -> ResourceKind {
    match id.scheme() {
        "memory" => ResourceKind::Memory,
        "file" => ResourceKind::File,
        "http" => ResourceKind::Http,
        "https" => ResourceKind::Https,
        other => ResourceKind::Other(other.into()),
    }
}

fn target_name(reference: &ResolvedReference) -> String {
    target_label(&ResolutionTarget {
        resource: reference.resource.clone(),
        fragment: reference.fragment.clone(),
    })
}

fn target_label(target: &ResolutionTarget) -> String {
    match &target.fragment {
        Fragment::Root => target.resource.to_string(),
        Fragment::Pointer(pointer) => {
            let encoded = pointer
                .tokens()
                .iter()
                .map(|token| token.replace('~', "~0").replace('/', "~1"))
                .collect::<Vec<_>>()
                .join("/");
            format!("{}#/{encoded}", target.resource)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DiscoveryRequest, RetrievedResource};
    use yamalgam_core::ResolutionMode;

    struct NeverRetrieve;

    impl ResourceRetriever for NeverRetrieve {
        fn retrieve(&self, request: &ResourceRequest) -> Result<RetrievedResource, RetrieveError> {
            Err(RetrieveError::NotFound(request.id.clone()))
        }

        fn discover(&self, request: &DiscoveryRequest) -> Result<Vec<ResourceId>, RetrieveError> {
            Err(RetrieveError::DiscoveryUnsupported(request.base.clone()))
        }
    }

    fn automatic_config() -> LoaderConfig {
        let mut config = LoaderConfig::unchecked();
        config.resolution.refs.mode = ResolutionMode::Automatic;
        config.resolution.refs.allow_schemes = vec!["memory".into()];
        config
    }

    fn parse(origin: &ResourceId, input: &str) -> OwnedDocument {
        let events = Parser::new(input).collect::<Result<Vec<_>, _>>().unwrap();
        OwnedDocument::from_events(origin.clone(), events).unwrap()
    }

    #[test]
    fn edge_counter_overflow_is_structured_and_records_failure() {
        let origin = ResourceId::new("memory://root/document.yaml").unwrap();
        let document = parse(&origin, "$ref: '#'\n");
        let mut session = ResolutionSession::new(NeverRetrieve, automatic_config());
        session.edges = usize::MAX;

        let error = session.resolve_root(document).unwrap_err();

        let ResolveError::Resolution {
            category,
            source: Some(source),
            ..
        } = error
        else {
            panic!("expected structured accounting overflow");
        };
        assert_eq!(category, ResolutionErrorCategory::ResourceOverflow);
        assert!(matches!(
            source.downcast_ref::<SessionError>(),
            Some(SessionError::ResourceOverflow("resolution edge count"))
        ));
        assert_eq!(
            session.report.edges()[0].outcome,
            ResolutionOutcome::Failed(ResolutionErrorCategory::ResourceOverflow)
        );
    }

    #[test]
    fn materialized_counter_overflow_is_structured_and_pops_cycle_state() {
        let origin = ResourceId::new("memory://root/document.yaml").unwrap();
        let document = parse(&origin, "value: retained\ncopy: {$ref: '#/value'}\n");
        let mut session = ResolutionSession::new(NeverRetrieve, automatic_config());
        session.materialized_bytes = usize::MAX;

        let error = session.resolve_root(document).unwrap_err();

        let ResolveError::Resolution {
            category,
            source: Some(source),
            ..
        } = error
        else {
            panic!("expected structured accounting overflow");
        };
        assert_eq!(category, ResolutionErrorCategory::ResourceOverflow);
        assert!(matches!(
            source.downcast_ref::<SessionError>(),
            Some(SessionError::ResourceOverflow("materialized byte count"))
        ));
        assert!(session.active.is_empty());
        assert_eq!(
            session.report.edges()[0].outcome,
            ResolutionOutcome::Failed(ResolutionErrorCategory::ResourceOverflow)
        );
    }
}

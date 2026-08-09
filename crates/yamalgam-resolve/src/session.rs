//! Shared external-resolution session state.

use std::collections::HashMap;
use std::mem;
use std::sync::Arc;

use yamalgam_core::{LimitExceeded, LoaderConfig, ResolutionCapability, ResourceLimit, Span};
use yamalgam_parser::{Parser, ResolutionErrorCategory, ResolveError};

use crate::document::{DocumentError, OwnedDocument, OwnedKind, OwnedNode};
use crate::error::{RetrieveError, SessionError};
use crate::report::{
    ResolutionEdge, ResolutionOutcome, ResolutionReport, ResourceKind, ResourceRecord,
};
use crate::resource::{Fragment, ResolvedReference, ResourceError, ResourceId};
use crate::retriever::{ResourceProvenance, ResourceRequest, ResourceRetriever};

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

/// Cache, budgets, cycle state, and report for one top-level YAML document.
pub struct ResolutionSession<R> {
    retriever: R,
    config: LoaderConfig,
    cache: HashMap<ResourceId, Arc<OwnedDocument>>,
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
        let mut pending = vec![TreeWork::Visit(root)];
        let mut resolved = Vec::new();

        while let Some(work) = pending.pop() {
            match work {
                TreeWork::Visit(node) => {
                    if let Some((requested, span)) = ref_directive(&node)
                        && self.should_resolve(ResolutionCapability::Ref, trigger)
                    {
                        let replacement = self.resolve_requested(
                            ResolutionCapability::Ref,
                            node.origin,
                            span,
                            requested,
                            trigger,
                        )?;
                        resolved.push(replacement);
                        continue;
                    }
                    if let Some(requested) = include_directive(&node)
                        && self.should_resolve(ResolutionCapability::Include, trigger)
                    {
                        let replacement = self.resolve_requested(
                            ResolutionCapability::Include,
                            node.origin,
                            node.span,
                            requested,
                            trigger,
                        )?;
                        resolved.push(replacement);
                        continue;
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
                            pending.extend(items.into_iter().rev().map(TreeWork::Visit));
                        }
                        OwnedKind::Mapping { style, entries } => {
                            let len = entries.len();
                            pending.push(TreeWork::FinishMapping {
                                node: metadata,
                                style,
                                len,
                            });
                            for (key, value) in entries.into_iter().rev() {
                                pending.push(TreeWork::Visit(value));
                                pending.push(TreeWork::Visit(key));
                            }
                        }
                        kind => resolved.push(metadata.with_kind(kind)),
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
                    let items = resolved.split_off(start);
                    resolved.push(node.with_kind(OwnedKind::Sequence { style, items }));
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
                    let mut entries = Vec::with_capacity(len);
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
                        entries.push((key, value));
                    }
                    resolved.push(node.with_kind(OwnedKind::Mapping { style, entries }));
                }
            }
        }

        resolved.pop().ok_or_else(|| {
            self.overflow_error(
                ResolutionCapability::Ref,
                Span::default(),
                "<empty tree>".into(),
                "tree reconstruction",
            )
        })
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
        let next_edges = self.edges.checked_add(1).ok_or_else(|| {
            self.overflow_error(
                capability,
                span,
                parent.to_string(),
                "resolution edge count",
            )
        })?;
        self.edges = next_edges;
        self.check_limit(
            capability,
            span,
            requested,
            ResourceLimit::ResolutionEdges,
            next_edges,
            self.config.resolution.limits.max_edges,
        )?;

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

        let selected = select_fragment(&document.root, &target.fragment).map_err(|source| {
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
            let overflow = self.overflow_error(
                capability,
                span,
                canonical.to_string(),
                "resource cache-hit count",
            );
            if let Some(record) = self.report.resource_mut(&canonical) {
                record.cache_hits = record.cache_hits.checked_add(1).ok_or(overflow)?;
            }
            return Ok((document, canonical, true, 0));
        }

        let retrieved = self
            .retriever
            .retrieve(&ResourceRequest {
                id: requested.clone(),
                max_bytes: self.config.limits.max_input_bytes,
            })
            .map_err(|source| {
                let category = retrieve_category(&source);
                self.error(
                    category,
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

        let input = std::str::from_utf8(&retrieved.bytes).map_err(|source| {
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
        Ok((document, canonical, false, fetched_bytes))
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
    Visit(OwnedNode),
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

fn ref_directive(node: &OwnedNode) -> Option<(String, Span)> {
    let OwnedKind::Mapping { entries, .. } = &node.kind else {
        return None;
    };
    let ref_entries = entries
        .iter()
        .filter(|(key, _)| scalar_value(key) == Some("$ref"))
        .collect::<Vec<_>>();
    if ref_entries.is_empty() {
        return None;
    }
    if entries.len() != 1 || ref_entries.len() != 1 {
        return Some((String::new(), ref_entries[0].1.span));
    }
    let (_, value) = ref_entries[0];
    scalar_value(value).map(|scalar| (scalar.into(), value.span))
}

fn include_directive(node: &OwnedNode) -> Option<String> {
    if node.tag.as_deref() != Some("!include") {
        return None;
    }
    scalar_value(node).map(str::to_owned)
}

fn scalar_value(node: &OwnedNode) -> Option<&str> {
    match &node.kind {
        OwnedKind::Scalar(scalar) => Some(&scalar.value),
        _ => None,
    }
}

fn select_fragment(root: &OwnedNode, fragment: &Fragment) -> Result<OwnedNode, SessionError> {
    let mut selected = root;
    for token in fragment.pointer_tokens() {
        selected = match &selected.kind {
            OwnedKind::Mapping { entries, .. } => entries
                .iter()
                .find_map(|(key, value)| (scalar_value(key) == Some(token)).then_some(value))
                .ok_or(SessionError::MissingFragment)?,
            OwnedKind::Sequence { items, .. } => {
                if token == "-" || (token.len() > 1 && token.starts_with('0')) {
                    return Err(SessionError::InvalidSequenceIndex);
                }
                let index = token
                    .parse::<usize>()
                    .map_err(|_| SessionError::InvalidSequenceIndex)?;
                items.get(index).ok_or(SessionError::MissingFragment)?
            }
            _ => return Err(SessionError::MissingFragment),
        };
    }
    Ok(selected.clone())
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

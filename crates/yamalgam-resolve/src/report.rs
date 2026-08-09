//! Deterministic external-resolution reports.

use std::fmt;

use yamalgam_core::{ResolutionCapability, Span};
pub use yamalgam_parser::ResolutionErrorCategory;

use crate::{Fragment, ResourceId, ResourceProvenance};

/// Structured resource and edge records for one resolution session.
// r[impl resolution.report]
// r[impl resolution.integrity]
#[derive(Default)]
pub struct ResolutionReport {
    resources: Vec<ResourceRecord>,
    edges: Vec<ResolutionEdge>,
}

impl ResolutionReport {
    /// Append a resource in deterministic discovery/fetch order.
    pub fn record_resource(&mut self, record: ResourceRecord) {
        self.resources.push(record);
    }

    /// Append an attempted edge in traversal order.
    pub fn record_edge(&mut self, edge: ResolutionEdge) -> usize {
        self.edges.push(edge);
        self.edges.len() - 1
    }

    /// Return resource records in insertion order.
    #[must_use]
    pub fn resources(&self) -> &[ResourceRecord] {
        &self.resources
    }

    /// Return resolution edges in traversal order.
    #[must_use]
    pub fn edges(&self) -> &[ResolutionEdge] {
        &self.edges
    }

    pub(crate) fn edge_mut(&mut self, index: usize) -> &mut ResolutionEdge {
        &mut self.edges[index]
    }

    pub(crate) fn resource_mut(&mut self, id: &ResourceId) -> Option<&mut ResourceRecord> {
        self.resources.iter_mut().find(|record| &record.id == id)
    }

    pub(crate) fn resource_ordinal(&self, id: &ResourceId) -> Option<usize> {
        self.resources
            .iter()
            .position(|record| &record.id == id)
            .and_then(|position| position.checked_add(1))
    }
}

impl fmt::Display for ResolutionReport {
    // r[impl resolution.report.safe]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "resolution report:")?;
        for resource in &self.resources {
            writeln!(
                formatter,
                "  resource {} ({:?}, {} bytes, {} cache hits)",
                resource.id, resource.kind, resource.fetched_bytes, resource.cache_hits
            )?;
        }
        for edge in &self.edges {
            write!(formatter, "  edge {} -> ", edge.parent)?;
            match &edge.child {
                Some(child) => write!(formatter, "{child}")?,
                None => formatter.write_str("<unresolved>")?,
            }
            writeln!(formatter, " ({:?})", edge.outcome)?;
        }
        Ok(())
    }
}

impl fmt::Debug for ResolutionReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

/// One fetched or cached resource.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceRecord {
    /// Canonical fragmentless identity.
    pub id: ResourceId,
    /// Backend/resource kind.
    pub kind: ResourceKind,
    /// Transport bytes fetched for this resource.
    pub fetched_bytes: usize,
    /// Number of raw-document cache hits.
    pub cache_hits: usize,
    /// Optional validators and integrity metadata.
    pub provenance: ResourceProvenance,
}

/// Stable resource/backend classification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceKind {
    /// In-memory resource.
    Memory,
    /// Capability-rooted file.
    File,
    /// HTTP resource.
    Http,
    /// HTTPS resource.
    Https,
    /// Application-defined scheme.
    Other(String),
}

/// One attempted parent-to-child resolution edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionEdge {
    /// Containing resource.
    pub parent: ResourceId,
    /// Resolved child identity, absent for failures before identity creation.
    pub child: Option<ResourceId>,
    /// Include or reference capability.
    pub capability: ResolutionCapability,
    /// Directive location in the containing resource.
    pub source_span: Span,
    /// Original directive reference retained as structured data.
    pub requested: String,
    /// Selected document fragment.
    pub fragment: Fragment,
    /// Deterministically ordered collection matches.
    pub matches: Vec<ResourceId>,
    /// Whether retrieval/parsing reused the session cache.
    pub cache_hit: bool,
    /// Transport bytes fetched by this edge.
    pub fetched_bytes: usize,
    /// Bytes inserted into the resolved document.
    pub materialized_bytes: usize,
    /// Terminal typed outcome.
    pub outcome: ResolutionOutcome,
}

/// Terminal outcome for a resolution edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolutionOutcome {
    /// Resolution completed successfully.
    Resolved,
    /// Resolution failed with a stable category.
    Failed(ResolutionErrorCategory),
}

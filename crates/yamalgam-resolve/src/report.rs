//! Deterministic external-resolution reports.

use std::fmt;

use yamalgam_core::{ResolutionCapability, Span};
pub use yamalgam_parser::ResolutionErrorCategory;

use crate::{Fragment, ResourceId, ResourceProvenance};

/// Structured resource and edge records for one resolution session.
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
    pub fn record_edge(&mut self, edge: ResolutionEdge) {
        self.edges.push(edge);
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
}

impl fmt::Display for ResolutionReport {
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

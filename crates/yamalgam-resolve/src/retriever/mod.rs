//! Pluggable synchronous resource retrieval.

use std::fmt;
use std::sync::Arc;

use crate::{ResourceId, RetrieveError};

mod file;
mod http;
mod memory;
mod router;

pub use file::FileRetriever;
pub use http::LibrebarRetriever;
pub use memory::MemoryRetriever;
pub use router::RetrieverRouter;

/// Synchronous transport boundary for external resources.
pub trait ResourceRetriever: Send + Sync {
    /// Retrieve one approved resource.
    fn retrieve(&self, request: &ResourceRequest) -> Result<RetrievedResource, RetrieveError>;

    /// Discover resources below a capability-specific base.
    fn discover(&self, request: &DiscoveryRequest) -> Result<Vec<ResourceId>, RetrieveError> {
        Err(RetrieveError::DiscoveryUnsupported(request.base.clone()))
    }
}

impl<T: ResourceRetriever + ?Sized> ResourceRetriever for Box<T> {
    fn retrieve(&self, request: &ResourceRequest) -> Result<RetrievedResource, RetrieveError> {
        (**self).retrieve(request)
    }

    fn discover(&self, request: &DiscoveryRequest) -> Result<Vec<ResourceId>, RetrieveError> {
        (**self).discover(request)
    }
}

impl<T: ResourceRetriever + ?Sized> ResourceRetriever for Arc<T> {
    fn retrieve(&self, request: &ResourceRequest) -> Result<RetrievedResource, RetrieveError> {
        (**self).retrieve(request)
    }

    fn discover(&self, request: &DiscoveryRequest) -> Result<Vec<ResourceId>, RetrieveError> {
        (**self).discover(request)
    }
}

/// Request to retrieve one resource with an optional byte ceiling.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceRequest {
    /// Fragmentless resource identity.
    pub id: ResourceId,
    /// Maximum response bytes accepted by the backend.
    pub max_bytes: Option<usize>,
}

/// Request for deterministic filesystem-style discovery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveryRequest {
    /// Resource used as the discovery base.
    pub base: ResourceId,
    /// Selector to apply below the base.
    pub selector: DiscoverySelector,
    /// Maximum approved matches.
    pub max_matches: Option<usize>,
}

/// Selector used for resource discovery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiscoverySelector {
    /// Match one root-relative glob.
    Glob {
        /// Glob pattern.
        pattern: String,
    },
    /// Walk one directory and apply filename patterns.
    Directory {
        /// Root-relative directory.
        path: String,
        /// Whether to descend recursively.
        recursive: bool,
        /// Filename glob patterns.
        matches: Vec<String>,
    },
}

/// Bytes and source metadata returned by a retriever.
#[derive(Clone)]
pub struct RetrievedResource {
    /// Canonical identity returned by the backend.
    pub id: ResourceId,
    /// Complete resource bytes.
    pub bytes: Vec<u8>,
    /// Optional validators and integrity metadata.
    pub provenance: ResourceProvenance,
}

impl fmt::Debug for RetrievedResource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RetrievedResource")
            .field("id", &self.id)
            .field("bytes_len", &self.bytes.len())
            .field("provenance", &self.provenance)
            .finish()
    }
}

/// Optional source validators and future integrity data.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct ResourceProvenance {
    /// Source-specific cache validators.
    pub validators: Vec<ResourceValidator>,
    /// Optional strong content identity supplied by the backend.
    pub integrity: Option<ContentIntegrity>,
}

impl fmt::Debug for ResourceProvenance {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResourceProvenance")
            .field("validators", &self.validators.len())
            .field("integrity", &self.integrity.is_some())
            .finish()
    }
}

/// Opaque source validator retained as structured data.
#[derive(Clone, Eq, PartialEq)]
pub enum ResourceValidator {
    /// HTTP ETag value.
    Etag(String),
    /// HTTP Last-Modified value.
    LastModified(String),
    /// Application-defined validator.
    Opaque {
        /// Validator name.
        name: String,
        /// Validator value.
        value: String,
    },
}

impl fmt::Debug for ResourceValidator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Etag(_) => "Etag(<redacted>)",
            Self::LastModified(_) => "LastModified(<redacted>)",
            Self::Opaque { .. } => "Opaque(<redacted>)",
        })
    }
}

/// Optional content-integrity seam for later lockfile enforcement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentIntegrity {
    /// Algorithm identifier, such as `sha256`.
    pub algorithm: String,
    /// Algorithm-specific encoded digest.
    pub encoded_value: String,
}

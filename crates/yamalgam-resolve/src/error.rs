//! Errors returned by resource retrievers.

use crate::ResourceId;

/// Failure while retrieving or discovering an external resource.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RetrieveError {
    /// Filesystem retrieval was configured without a capability root.
    #[error("filesystem retrieval requires a configured root")]
    MissingRoot,
    /// A configured discovery or policy glob was invalid.
    #[error("invalid glob pattern: {0}")]
    InvalidPattern(String),
    /// A router already has a backend for the scheme.
    #[error("a retriever is already registered for scheme {0}")]
    DuplicateScheme(String),
    /// No backend handles the requested scheme.
    #[error("no retriever is registered for scheme {0}")]
    UnsupportedScheme(String),
    /// The selected backend does not support discovery.
    #[error("resource discovery is unsupported from {0}")]
    DiscoveryUnsupported(ResourceId),
    /// Active policy denied the resource.
    #[error("resource denied by policy: {id}")]
    PolicyDenied {
        /// Denied resource.
        id: ResourceId,
    },
    /// The resource does not exist.
    #[error("resource not found: {0}")]
    NotFound(ResourceId),
    /// A file target is not a regular file.
    #[error("resource is not a regular file: {0}")]
    NotRegularFile(ResourceId),
    /// A symbolic link was denied.
    #[error("symbolic link denied: {0}")]
    SymlinkDenied(ResourceId),
    /// A file target escapes the configured capability root.
    #[error("resource escapes the configured root: {0}")]
    OutsideRoot(ResourceId),
    /// A resource exceeded its byte ceiling.
    #[error("resource size {observed} exceeds maximum of {maximum}: {id}")]
    TooLarge {
        /// Resource being retrieved.
        id: ResourceId,
        /// Observed byte count.
        observed: usize,
        /// Configured maximum.
        maximum: usize,
    },
    /// Discovery exceeded its match ceiling.
    #[error("matched resource count {observed} exceeds maximum of {maximum}")]
    TooManyMatches {
        /// Observed match count.
        observed: usize,
        /// Configured maximum.
        maximum: usize,
    },
    /// HTTP returned a non-success status.
    #[error("HTTP status {status} for {id}")]
    HttpStatus {
        /// Requested resource.
        id: ResourceId,
        /// HTTP status code.
        status: u16,
    },
    /// The request timed out.
    #[error("HTTP request timed out for {0}")]
    Timeout(ResourceId),
    /// Filesystem I/O failed.
    #[error("I/O failed for {id}: {source}")]
    Io {
        /// Requested resource.
        id: ResourceId,
        /// Underlying I/O failure.
        #[source]
        source: std::io::Error,
    },
    /// HTTP transport failed.
    #[error("HTTP transport failed for {id}: {source}")]
    Transport {
        /// Requested resource.
        id: ResourceId,
        /// Underlying transport failure.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// The private HTTP worker stopped unexpectedly.
    #[error("HTTP worker stopped before completing {0}")]
    WorkerStopped(ResourceId),
}

//! Errors returned by resource retrievers.

use crate::ResourceId;

/// Failure produced by resolution-session bookkeeping or selection.
#[derive(Clone, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SessionError {
    /// Resolver construction and downstream consumer policies differ.
    #[error("resolver and consumer loader configurations must match")]
    LoaderConfigMismatch,
    /// Retrieved bytes were not UTF-8 text.
    #[error("retrieved YAML is not valid UTF-8")]
    InvalidUtf8(#[source] std::str::Utf8Error),
    /// A decoded JSON Pointer did not select a node.
    #[error("JSON Pointer does not select a YAML node")]
    MissingFragment,
    /// A sequence pointer token was not a canonical array index.
    #[error("JSON Pointer token is not a canonical sequence index")]
    InvalidSequenceIndex,
    /// A mapping containing `$ref` also contained sibling entries.
    #[error("$ref mappings must contain exactly one entry")]
    RefSiblings,
    /// A `$ref` value did not resolve to a YAML string.
    #[error("$ref value must be a YAML string")]
    RefValueNotString,
    /// JSON Pointer cannot address a non-string YAML mapping key.
    #[error("JSON Pointer cannot address a non-string YAML mapping key")]
    NonStringMappingKey,
    /// An `!include` directive has an invalid shape or option combination.
    #[error("malformed !include directive: {0}")]
    MalformedInclude(&'static str),
    /// An include selector did not produce the number of resources required by its mode.
    #[error("include mode requires {expected} resource(s), observed {observed}")]
    IncludeMatchCount {
        /// Required number of resources.
        expected: usize,
        /// Observed number of resources.
        observed: usize,
    },
    /// An include mode received incompatible YAML root types.
    #[error("include aggregation requires homogeneous compatible root types")]
    IncludeAggregateType,
    /// An include mode is invalid at its insertion site.
    #[error("include mode is incompatible with its YAML insertion context")]
    IncludeContext,
    /// An include-generated mapping key conflicts with an earlier key.
    #[error("include aggregation produced a duplicate mapping key")]
    IncludeConflict,
    /// A filename-derived include key was not valid portable UTF-8.
    #[error("include resource cannot produce a portable UTF-8 mapping key")]
    IncludeName,
    /// A selected subtree uses an anchor defined outside that subtree.
    #[error("selected subtree depends on external anchor {0}")]
    NonSelfContainedAlias(String),
    /// Checked session accounting exceeded `usize`.
    #[error("resource accounting overflow for {0}")]
    ResourceOverflow(&'static str),
}

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
    /// Discovery exceeded its directory-and-entry visit ceiling.
    #[error("discovery visit count {observed} exceeds maximum of {maximum}")]
    TooManyVisits {
        /// Observed visit count.
        observed: usize,
        /// Configured maximum.
        maximum: usize,
    },
    /// Checked retriever accounting exceeded `usize`.
    #[error("resource accounting overflow for {0}")]
    ResourceOverflow(&'static str),
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
    /// The private HTTP worker could not be started.
    #[error("HTTP worker failed to start: {0}")]
    WorkerStart(#[source] Box<dyn std::error::Error + Send + Sync>),
}

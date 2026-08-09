//! External resource resolution for Yamalgam.
//!
//! This crate sits above `yamalgam-parser` and below query and CLI consumers.
//! It owns retrieval, resource identity, resolution sessions, and the built-in
//! event middleware without coupling filesystem or network I/O to the parser.
#![deny(unsafe_code)]

pub mod document;
pub mod error;
pub mod pointer;
mod ref_resolver;
pub mod report;
pub mod resource;
pub mod retriever;
pub mod session;

pub use document::{DocumentError, OwnedDocument, OwnedKind, OwnedNode, OwnedScalar};
pub use error::{RetrieveError, SessionError};
pub use pointer::{JsonPointer, PointerError};
pub use report::{
    ResolutionEdge, ResolutionErrorCategory, ResolutionOutcome, ResolutionReport, ResourceKind,
    ResourceRecord,
};
pub use resource::{Fragment, ResolvedReference, ResourceError, ResourceId, resolve_reference};
pub use retriever::{
    ContentIntegrity, DiscoveryRequest, DiscoverySelector, FileRetriever, LibrebarRetriever,
    MemoryRetriever, ResourceProvenance, ResourceRequest, ResourceRetriever, ResourceValidator,
    RetrievedResource, RetrieverRouter,
};
pub use session::{ResolutionSession, ResolutionTrigger};

//! Resource retriever routing by normalized scheme.

use std::collections::BTreeMap;

use super::{DiscoveryRequest, ResourceRequest, ResourceRetriever, RetrievedResource};
use crate::{ResourceId, RetrieveError};

/// Deterministic scheme-to-retriever router.
#[derive(Default)]
pub struct RetrieverRouter {
    by_scheme: BTreeMap<String, Box<dyn ResourceRetriever>>,
}

impl RetrieverRouter {
    /// Register one backend for a case-insensitive URI scheme.
    ///
    /// # Errors
    ///
    /// Returns [`RetrieveError::DuplicateScheme`] when the normalized scheme
    /// already has a backend.
    pub fn register(
        &mut self,
        scheme: impl Into<String>,
        retriever: impl ResourceRetriever + 'static,
    ) -> Result<(), RetrieveError> {
        let scheme = scheme.into().to_ascii_lowercase();
        if self.by_scheme.contains_key(&scheme) {
            return Err(RetrieveError::DuplicateScheme(scheme));
        }
        self.by_scheme.insert(scheme, Box::new(retriever));
        Ok(())
    }

    fn backend(&self, id: &ResourceId) -> Result<&dyn ResourceRetriever, RetrieveError> {
        self.by_scheme
            .get(id.scheme())
            .map(Box::as_ref)
            .ok_or_else(|| RetrieveError::UnsupportedScheme(id.scheme().to_owned()))
    }
}

impl ResourceRetriever for RetrieverRouter {
    fn retrieve(&self, request: &ResourceRequest) -> Result<RetrievedResource, RetrieveError> {
        self.backend(&request.id)?.retrieve(request)
    }

    fn discover(&self, request: &DiscoveryRequest) -> Result<Vec<ResourceId>, RetrieveError> {
        self.backend(&request.base)?.discover(request)
    }
}

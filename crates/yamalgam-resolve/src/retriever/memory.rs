//! In-memory resource retrieval.

use std::collections::BTreeMap;

use super::{ResourceRequest, ResourceRetriever, RetrievedResource};
use crate::{ResourceId, RetrieveError};

/// Exact-identity in-memory resource backend.
///
/// Direct use can serve any [`ResourceId`] identity. A [`RetrieverRouter`](super::RetrieverRouter)
/// registration controls which URI scheme reaches it when routing is required.
#[derive(Default)]
pub struct MemoryRetriever {
    resources: BTreeMap<ResourceId, RetrievedResource>,
}

impl MemoryRetriever {
    /// Insert or replace one resource, returning the previous value.
    pub fn insert(&mut self, resource: RetrievedResource) -> Option<RetrievedResource> {
        self.resources.insert(resource.id.clone(), resource)
    }
}

impl ResourceRetriever for MemoryRetriever {
    fn retrieve(&self, request: &ResourceRequest) -> Result<RetrievedResource, RetrieveError> {
        let resource = self
            .resources
            .get(&request.id)
            .ok_or_else(|| RetrieveError::NotFound(request.id.clone()))?;
        if let Some(maximum) = request.max_bytes
            && resource.bytes.len() > maximum
        {
            return Err(RetrieveError::TooLarge {
                id: request.id.clone(),
                observed: resource.bytes.len(),
                maximum,
            });
        }
        Ok(resource.clone())
    }
}

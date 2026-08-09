#![allow(missing_docs)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use yamalgam_resolve::{
    MemoryRetriever, ResourceId, ResourceProvenance, ResourceRequest, ResourceRetriever,
    RetrieveError, RetrievedResource, RetrieverRouter,
};

fn fixture(id: ResourceId, bytes: &[u8]) -> RetrievedResource {
    RetrievedResource {
        id,
        bytes: bytes.to_vec(),
        provenance: ResourceProvenance::default(),
    }
}

#[test]
fn memory_retrieval_is_exact_and_enforces_byte_limits() {
    let id = ResourceId::new("memory://fixtures/root/document.yaml").unwrap();
    let mut retriever = MemoryRetriever::default();
    assert!(
        retriever
            .insert(fixture(id.clone(), b"answer: 42"))
            .is_none()
    );

    let result = retriever
        .retrieve(&ResourceRequest {
            id: id.clone(),
            max_bytes: Some(10),
        })
        .unwrap();
    assert_eq!(result.id, id);
    assert_eq!(result.bytes, b"answer: 42");

    let error = retriever
        .retrieve(&ResourceRequest {
            id: result.id,
            max_bytes: Some(9),
        })
        .unwrap_err();
    assert!(matches!(
        error,
        RetrieveError::TooLarge {
            observed: 10,
            maximum: 9,
            ..
        }
    ));
}

#[test]
fn memory_retrieval_reports_missing_resources() {
    let id = ResourceId::new("memory://fixtures/missing.yaml").unwrap();
    let error = MemoryRetriever::default()
        .retrieve(&ResourceRequest {
            id: id.clone(),
            max_bytes: None,
        })
        .unwrap_err();
    assert!(matches!(error, RetrieveError::NotFound(missing) if missing == id));
}

#[test]
fn memory_retrieval_can_back_file_identities_for_embedded_callers() {
    let id = ResourceId::from_file("/fixtures/document.yaml");
    let mut retriever = MemoryRetriever::default();
    retriever.insert(fixture(id.clone(), b"embedded: true"));

    let result = retriever
        .retrieve(&ResourceRequest {
            id: id.clone(),
            max_bytes: None,
        })
        .unwrap();

    assert_eq!(result.id, id);
    assert_eq!(result.bytes, b"embedded: true");
}

#[test]
fn router_rejects_duplicate_and_unsupported_schemes() {
    let mut router = RetrieverRouter::default();
    router
        .register("memory", MemoryRetriever::default())
        .unwrap();
    assert!(matches!(
        router.register("MEMORY", MemoryRetriever::default()),
        Err(RetrieveError::DuplicateScheme(scheme)) if scheme == "memory"
    ));

    let id = ResourceId::new("custom://fixtures/document.yaml").unwrap();
    assert!(matches!(
        router.retrieve(&ResourceRequest { id, max_bytes: None }),
        Err(RetrieveError::UnsupportedScheme(scheme)) if scheme == "custom"
    ));
}

#[test]
fn memory_resources_resolve_relative_to_hierarchical_memory_bases() {
    let base = ResourceId::new("memory://fixtures/root/document.yaml").unwrap();
    let child = base.resolve("../shared.yaml").unwrap().resource;
    let mut memory = MemoryRetriever::default();
    memory.insert(fixture(child.clone(), b"shared: true"));
    let mut router = RetrieverRouter::default();
    router.register("memory", memory).unwrap();

    let result = router
        .retrieve(&ResourceRequest {
            id: child,
            max_bytes: None,
        })
        .unwrap();
    assert_eq!(result.bytes, b"shared: true");
}

struct SpyRetriever(Arc<AtomicUsize>);

impl ResourceRetriever for SpyRetriever {
    fn retrieve(&self, request: &ResourceRequest) -> Result<RetrievedResource, RetrieveError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(fixture(request.id.clone(), b"unexpected"))
    }
}

#[test]
fn unsupported_routes_do_not_call_registered_retrievers() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut router = RetrieverRouter::default();
    router
        .register("memory", SpyRetriever(Arc::clone(&calls)))
        .unwrap();

    let id = ResourceId::new("https://example.com/data.yaml").unwrap();
    assert!(matches!(
        router.retrieve(&ResourceRequest {
            id,
            max_bytes: None
        }),
        Err(RetrieveError::UnsupportedScheme(_))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn boxed_and_shared_retrievers_preserve_the_sync_contract() {
    let id = ResourceId::new("memory://fixtures/document.yaml").unwrap();
    let mut memory = MemoryRetriever::default();
    memory.insert(fixture(id.clone(), b"value"));
    let boxed: Box<dyn ResourceRetriever> = Box::new(memory);
    let shared: Arc<dyn ResourceRetriever> = Arc::from(boxed);

    let result = shared
        .retrieve(&ResourceRequest {
            id,
            max_bytes: None,
        })
        .unwrap();
    assert_eq!(result.bytes, b"value");
}

//! Synchronous HTTP retrieval through a private Librebar runtime.

use std::collections::BTreeSet;
use std::fmt;
use std::io;
use std::sync::Mutex;
use std::sync::mpsc::{self, SyncSender};
use std::thread::{self, JoinHandle};

use librebar::error::HttpError;
use librebar::http::{HttpClient, RetryPolicy, header};
use yamalgam_core::RefPolicy;

use super::{
    ResourceProvenance, ResourceRequest, ResourceRetriever, ResourceValidator, RetrievedResource,
};
use crate::{ResourceId, RetrieveError};

type BoxError = Box<dyn std::error::Error + Send + Sync>;
type ClientFactory =
    Box<dyn FnOnce() -> Result<(tokio::runtime::Runtime, HttpClient), BoxError> + Send + 'static>;

struct HttpWork {
    id: ResourceId,
    url: String,
    response: SyncSender<Result<RetrievedResource, RetrieveError>>,
}

/// HTTP and HTTPS backend using Librebar without redirects, retries, cookies,
/// cache state, or ambient credentials.
pub struct LibrebarRetriever {
    allowed_schemes: BTreeSet<String>,
    allowed_hosts: BTreeSet<String>,
    work: Option<SyncSender<HttpWork>>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl LibrebarRetriever {
    /// Start a private current-thread runtime using the supplied network policy.
    ///
    /// `max_response_size` is the decoded body ceiling. A zero value uses
    /// Librebar's documented unbounded mode for unchecked loaders.
    ///
    /// # Errors
    ///
    /// Returns [`RetrieveError::WorkerStart`] when the worker thread, Tokio
    /// runtime, TLS client, or readiness handshake cannot be initialized.
    pub fn new(policy: &RefPolicy, max_response_size: usize) -> Result<Self, RetrieveError> {
        let timeout = policy.timeout;
        let factory: ClientFactory = Box::new(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|source| Box::new(source) as BoxError)?;
            let client = HttpClient::builder("yamalgam", env!("CARGO_PKG_VERSION"))
                .max_redirects(0)
                .retry_policy(RetryPolicy::none())
                .max_response_size(max_response_size)
                .timeout(timeout)
                .build()
                .map_err(|source| Box::new(source) as BoxError)?;
            Ok((runtime, client))
        });
        Self::with_factory(policy, factory)
    }

    fn with_factory(policy: &RefPolicy, factory: ClientFactory) -> Result<Self, RetrieveError> {
        let allowed_schemes = normalized_set(&policy.allow_schemes);
        let allowed_hosts = normalized_set(&policy.allow_hosts);
        let (work, worker) = start_worker(factory)?;
        Ok(Self {
            allowed_schemes,
            allowed_hosts,
            work: Some(work),
            worker: Mutex::new(Some(worker)),
        })
    }

    fn approved_url(&self, id: &ResourceId) -> Result<String, RetrieveError> {
        let Some(url) = id.as_url() else {
            return Err(RetrieveError::UnsupportedScheme(id.scheme().to_owned()));
        };
        let scheme = url.scheme().to_ascii_lowercase();
        if !self.allowed_schemes.contains(&scheme) {
            return Err(RetrieveError::PolicyDenied { id: id.clone() });
        }
        if id.has_userinfo() {
            return Err(RetrieveError::PolicyDenied { id: id.clone() });
        }
        let Some(host) = url.host_str() else {
            return Err(RetrieveError::PolicyDenied { id: id.clone() });
        };
        if !matches!(scheme.as_str(), "http" | "https")
            || !self.allowed_hosts.contains(&host.to_ascii_lowercase())
        {
            return Err(RetrieveError::PolicyDenied { id: id.clone() });
        }
        Ok(url.as_str().to_owned())
    }
}

impl fmt::Debug for LibrebarRetriever {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LibrebarRetriever")
            .field("allowed_schemes", &self.allowed_schemes.len())
            .field("allowed_hosts", &self.allowed_hosts.len())
            .field("worker_active", &self.work.is_some())
            .finish_non_exhaustive()
    }
}

impl ResourceRetriever for LibrebarRetriever {
    fn retrieve(&self, request: &ResourceRequest) -> Result<RetrievedResource, RetrieveError> {
        let url = self.approved_url(&request.id)?;
        let (response_tx, response_rx) = mpsc::sync_channel(1);
        self.work
            .as_ref()
            .ok_or_else(|| RetrieveError::WorkerStopped(request.id.clone()))?
            .send(HttpWork {
                id: request.id.clone(),
                url,
                response: response_tx,
            })
            .map_err(|_| RetrieveError::WorkerStopped(request.id.clone()))?;
        let resource = response_rx
            .recv()
            .map_err(|_| RetrieveError::WorkerStopped(request.id.clone()))??;
        if let Some(maximum) = request.max_bytes
            && resource.bytes.len() > maximum
        {
            return Err(RetrieveError::TooLarge {
                id: resource.id,
                observed: resource.bytes.len(),
                maximum,
            });
        }
        Ok(resource)
    }
}

impl Drop for LibrebarRetriever {
    fn drop(&mut self) {
        self.work.take();
        let worker = self
            .worker
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(worker) = worker {
            let _ = worker.join();
        }
    }
}

fn start_worker(
    factory: ClientFactory,
) -> Result<(SyncSender<HttpWork>, JoinHandle<()>), RetrieveError> {
    let (work, receiver) = mpsc::sync_channel(1);
    let (ready_tx, ready_rx) = mpsc::sync_channel(0);
    let worker = thread::Builder::new()
        .name("yamalgam-librebar-http".to_owned())
        .spawn(move || {
            let (runtime, client) = match factory() {
                Ok(parts) => parts,
                Err(source) => {
                    let _ = ready_tx.send(Err(source));
                    return;
                }
            };
            if ready_tx.send(Ok(())).is_err() {
                return;
            }
            while let Ok(HttpWork { id, url, response }) = receiver.recv() {
                let result = runtime.block_on(fetch(&client, id, &url));
                let _ = response.send(result);
            }
        })
        .map_err(|source| RetrieveError::WorkerStart(Box::new(source)))?;

    match ready_rx.recv() {
        Ok(Ok(())) => Ok((work, worker)),
        Ok(Err(source)) => {
            let _ = worker.join();
            Err(RetrieveError::WorkerStart(source))
        }
        Err(_) => {
            let _ = worker.join();
            Err(RetrieveError::WorkerStart(Box::new(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "HTTP runtime stopped during initialization",
            ))))
        }
    }
}

async fn fetch(
    client: &HttpClient,
    id: ResourceId,
    url: &str,
) -> Result<RetrievedResource, RetrieveError> {
    let response = client
        .get(url)
        .await
        .map_err(|source| map_http_error(id.clone(), source))?;
    let status = response.status();
    if !status.is_success() {
        return Err(RetrieveError::HttpStatus {
            id,
            status: status.as_u16(),
        });
    }
    let mut validators = Vec::with_capacity(2);
    if let Some(value) = response.header(header::ETAG)
        && let Ok(value) = value.to_str()
    {
        validators.push(ResourceValidator::Etag(value.to_owned()));
    }
    if let Some(value) = response.header(header::LAST_MODIFIED)
        && let Ok(value) = value.to_str()
    {
        validators.push(ResourceValidator::LastModified(value.to_owned()));
    }
    let (_, bytes) = response.into_parts();
    Ok(RetrievedResource {
        id,
        bytes,
        provenance: ResourceProvenance {
            validators,
            integrity: None,
        },
    })
}

fn map_http_error(id: ResourceId, source: librebar::Error) -> RetrieveError {
    match source {
        librebar::Error::Http(HttpError::ResponseTooLarge { maximum }) => RetrieveError::TooLarge {
            id,
            observed: maximum.saturating_add(1),
            maximum,
        },
        librebar::Error::Http(HttpError::Io(source))
            if source.kind() == io::ErrorKind::TimedOut =>
        {
            RetrieveError::Timeout(id)
        }
        source => RetrieveError::Transport {
            id,
            source: Box::new(source),
        },
    }
}

fn normalized_set(values: &[String]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use yamalgam_core::ResolutionMode;

    use super::*;

    fn policy() -> RefPolicy {
        RefPolicy {
            mode: ResolutionMode::Automatic,
            allow_schemes: vec!["http".into()],
            allow_hosts: vec!["example.com".into()],
            timeout: Duration::from_secs(1),
        }
    }

    #[test]
    fn runtime_factory_failure_is_reported_from_readiness_handshake() {
        let factory: ClientFactory =
            Box::new(|| Err(Box::new(io::Error::other("injected runtime failure")) as BoxError));
        assert!(matches!(
            LibrebarRetriever::with_factory(&policy(), factory),
            Err(RetrieveError::WorkerStart(_))
        ));
    }

    #[test]
    fn closed_work_channel_is_a_typed_worker_failure() {
        let (work, receiver) = mpsc::sync_channel(1);
        drop(receiver);
        let retriever = LibrebarRetriever {
            allowed_schemes: BTreeSet::from(["http".into()]),
            allowed_hosts: BTreeSet::from(["example.com".into()]),
            work: Some(work),
            worker: Mutex::new(None),
        };
        let request = ResourceRequest {
            id: ResourceId::new("http://example.com/data.yaml").unwrap(),
            max_bytes: None,
        };
        assert!(matches!(
            retriever.retrieve(&request),
            Err(RetrieveError::WorkerStopped(_))
        ));
    }

    #[test]
    fn worker_panic_closes_the_response_channel_without_panicking_the_caller() {
        let (work, receiver) = mpsc::sync_channel::<HttpWork>(1);
        let worker = thread::spawn(move || {
            let _accepted = receiver.recv().unwrap();
            panic!("injected worker panic");
        });
        let retriever = LibrebarRetriever {
            allowed_schemes: BTreeSet::from(["http".into()]),
            allowed_hosts: BTreeSet::from(["example.com".into()]),
            work: Some(work),
            worker: Mutex::new(Some(worker)),
        };
        let request = ResourceRequest {
            id: ResourceId::new("http://example.com/data.yaml").unwrap(),
            max_bytes: None,
        };
        assert!(matches!(
            retriever.retrieve(&request),
            Err(RetrieveError::WorkerStopped(_))
        ));
    }
}

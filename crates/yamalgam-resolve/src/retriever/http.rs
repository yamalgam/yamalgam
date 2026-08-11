//! Synchronous HTTP retrieval through a private Librebar runtime.

use std::collections::BTreeSet;
use std::fmt;
use std::io::{self, Write as _};
use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex};
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
type WorkerTermination = Arc<Mutex<Option<String>>>;

struct HttpWork {
    id: ResourceId,
    url: String,
    response: SyncSender<Result<RetrievedResource, RetrieveError>>,
}

/// HTTP and HTTPS backend using Librebar without redirects, retries, cookies,
/// cache state, or ambient credentials.
// r[impl resolution.ref.schemes]
// r[impl resolution.ref.network]
pub struct LibrebarRetriever {
    allowed_schemes: BTreeSet<String>,
    allowed_hosts: BTreeSet<String>,
    work: Option<SyncSender<HttpWork>>,
    worker: Mutex<Option<JoinHandle<()>>>,
    termination: WorkerTermination,
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
        let (work, worker, termination) = start_worker(factory)?;
        Ok(Self {
            allowed_schemes,
            allowed_hosts,
            work: Some(work),
            worker: Mutex::new(Some(worker)),
            termination,
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

    /// Stop the private worker and report an abnormal termination.
    ///
    /// Calling this method more than once is harmless.
    ///
    /// # Errors
    ///
    /// Returns [`RetrieveError::WorkerPanicked`] when the worker panicked.
    pub fn shutdown(&mut self) -> Result<(), RetrieveError> {
        self.work.take();
        self.join_worker();
        self.termination_error().map_or(Ok(()), Err)
    }

    fn worker_stopped(&self, id: ResourceId) -> RetrieveError {
        self.join_worker();
        self.termination_error()
            .unwrap_or(RetrieveError::WorkerStopped(id))
    }

    fn join_worker(&self) {
        let worker = self
            .worker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(worker) = worker
            && let Err(payload) = worker.join()
        {
            record_worker_panic(&self.termination, payload);
        }
    }

    fn termination_error(&self) -> Option<RetrieveError> {
        self.termination
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
            .map(RetrieveError::WorkerPanicked)
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
        if self
            .work
            .as_ref()
            .ok_or_else(|| RetrieveError::WorkerStopped(request.id.clone()))?
            .send(HttpWork {
                id: request.id.clone(),
                url,
                response: response_tx,
            })
            .is_err()
        {
            return Err(self.worker_stopped(request.id.clone()));
        }
        let resource = response_rx
            .recv()
            .map_err(|_| self.worker_stopped(request.id.clone()))??;
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
        if let Err(error) = self.shutdown() {
            let _ = writeln!(
                io::stderr().lock(),
                "yamalgam_event=http_worker_shutdown status=error error={error}"
            );
        }
    }
}

fn start_worker(
    factory: ClientFactory,
) -> Result<(SyncSender<HttpWork>, JoinHandle<()>, WorkerTermination), RetrieveError> {
    let (work, receiver) = mpsc::sync_channel(1);
    let (ready_tx, ready_rx) = mpsc::sync_channel(0);
    let termination = WorkerTermination::default();
    let worker = spawn_recorded_worker(
        "yamalgam-librebar-http",
        Arc::clone(&termination),
        move || {
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
        },
    )
    .map_err(|source| RetrieveError::WorkerStart(Box::new(source)))?;

    match ready_rx.recv() {
        Ok(Ok(())) => Ok((work, worker, termination)),
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

fn spawn_recorded_worker(
    name: &str,
    termination: WorkerTermination,
    worker: impl FnOnce() + Send + 'static,
) -> io::Result<JoinHandle<()>> {
    thread::Builder::new().name(name.to_owned()).spawn(move || {
        if let Err(payload) = panic::catch_unwind(AssertUnwindSafe(worker)) {
            record_worker_panic(&termination, payload);
        }
    })
}

fn record_worker_panic(termination: &WorkerTermination, payload: Box<dyn std::any::Any + Send>) {
    let message = payload
        .downcast_ref::<&str>()
        .map(|message| (*message).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "non-string panic payload".to_owned());
    *termination
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(message);
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
            termination: WorkerTermination::default(),
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
    fn worker_panic_is_reported_without_panicking_the_caller() {
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
            termination: WorkerTermination::default(),
        };
        let request = ResourceRequest {
            id: ResourceId::new("http://example.com/data.yaml").unwrap(),
            max_bytes: None,
        };
        assert!(matches!(
            retriever.retrieve(&request),
            Err(RetrieveError::WorkerPanicked(message)) if message == "injected worker panic"
        ));
    }

    #[test]
    fn explicit_shutdown_reports_an_unobserved_worker_panic_once() {
        let (work, _receiver) = mpsc::sync_channel(1);
        let worker = thread::spawn(|| panic!("shutdown worker panic"));
        let mut retriever = LibrebarRetriever {
            allowed_schemes: BTreeSet::new(),
            allowed_hosts: BTreeSet::new(),
            work: Some(work),
            worker: Mutex::new(Some(worker)),
            termination: WorkerTermination::default(),
        };

        assert!(matches!(
            retriever.shutdown(),
            Err(RetrieveError::WorkerPanicked(message)) if message == "shutdown worker panic"
        ));
        assert!(retriever.shutdown().is_ok());
    }
}

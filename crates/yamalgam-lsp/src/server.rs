//! LSP lifecycle, request dispatch, and document-state loop.

use std::collections::HashMap;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;

use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    CancelParams, CompletionOptions, CompletionParams, CompletionResponse,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    GotoDefinitionParams, HoverParams, HoverProviderCapability, InitializeResult, NumberOrString,
    OneOf, PositionEncodingKind, PublishDiagnosticsParams, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions, Uri,
};
use thiserror::Error;
use tracing::warn;
use yamalgam_core::{LoaderConfig, ResolutionMode};
use yamalgam_resolve::{
    FileRetriever, ResourceRequest, ResourceRetriever, RetrieveError, RetrievedResource,
};
use yamalgam_schema::SchemaAssociation;

use crate::{
    DocumentState, SchemaWorkspace, ServerConfig, WorkspaceState, complete, definition,
    document_diagnostics, hover,
};

/// Run the language server over standard input and standard output.
pub fn run_stdio() -> Result<(), ServerError> {
    let (connection, threads) = Connection::stdio();
    finish_transport(run(connection), || threads.join())
}

fn finish_transport(
    result: Result<(), ServerError>,
    join: impl FnOnce() -> io::Result<()>,
) -> Result<(), ServerError> {
    result?;
    join()?;
    Ok(())
}

/// Run one synchronous language-server connection to completion.
pub fn run(connection: Connection) -> Result<(), ServerError> {
    let (initialize_id, initialize_params) = connection.initialize_start()?;
    let config = server_config(&initialize_params)?;
    connection.initialize_finish(
        initialize_id,
        serde_json::to_value(InitializeResult {
            capabilities: capabilities(),
            server_info: Some(ServerInfo {
                name: "yamalgam-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })?,
    )?;

    let mut server = Server::new(config)?;
    let response_sender = connection.sender.clone();
    let dispatcher = RequestDispatcher::new(move |response| {
        response_sender
            .send(Message::Response(response))
            .map_err(|error| ServerError::Transport(error.to_string()))
    });
    let result = run_loop(&connection, &mut server, &dispatcher);
    let shutdown = dispatcher.shutdown();
    result.and(shutdown)
}

fn run_loop(
    connection: &Connection,
    server: &mut Server,
    dispatcher: &RequestDispatcher,
) -> Result<(), ServerError> {
    while let Ok(message) = connection.receiver.recv() {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    return Ok(());
                }
                let id = request.id.clone();
                dispatcher.dispatch(id, server.request_work(request))?;
            }
            Message::Notification(notification) => {
                if notification.method == "$/cancelRequest" {
                    match serde_json::from_value::<CancelParams>(notification.params) {
                        Ok(params) => {
                            dispatcher.cancel(&request_id(params.id))?;
                        }
                        Err(error) => warn!(%error, "ignored malformed cancellation"),
                    }
                } else {
                    server.handle_notification(connection, notification)?;
                }
            }
            Message::Response(_) => {}
        }
    }
    Ok(())
}

fn capabilities() -> ServerCapabilities {
    ServerCapabilities {
        position_encoding: Some(PositionEncodingKind::UTF16),
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::INCREMENTAL),
                ..TextDocumentSyncOptions::default()
            },
        )),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![":".to_string(), "-".to_string()]),
            ..CompletionOptions::default()
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        ..ServerCapabilities::default()
    }
}

#[derive(Clone)]
struct Server {
    documents: HashMap<Uri, Arc<DocumentState>>,
    loader: LoaderConfig,
    config: ServerConfig,
    schemas: Arc<SchemaWorkspace>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new(ServerConfig::default()).expect("default server configuration should be valid")
    }
}

impl Server {
    fn new(config: ServerConfig) -> Result<Self, ServerError> {
        let loader = LoaderConfig::default();
        let schemas = schema_workspace(&config)?;
        Ok(Self {
            documents: HashMap::new(),
            schemas: Arc::new(schemas),
            loader,
            config,
        })
    }

    fn request_work(&self, request: Request) -> RequestWork {
        let snapshot = self.clone();
        Box::new(move |cancelled| snapshot.request_response(request, cancelled))
    }

    fn request_response(&self, request: Request, cancelled: &AtomicBool) -> Response {
        if cancelled.load(Ordering::Acquire) {
            return cancelled_response(request.id);
        }
        match request.method.as_str() {
            "textDocument/completion" => self.completion_response(request, cancelled),
            "textDocument/hover" => self.hover_response(request, cancelled),
            "textDocument/definition" => self.definition_response(request, cancelled),
            _ => Response::new_err(
                request.id,
                ErrorCode::MethodNotFound as i32,
                format!("unknown request method {}", request.method),
            ),
        }
    }

    fn completion_response(&self, request: Request, cancelled: &AtomicBool) -> Response {
        let id = request.id;
        let params = match serde_json::from_value::<CompletionParams>(request.params) {
            Ok(params) => params,
            Err(error) => return invalid_params(id, error),
        };
        if cancelled.load(Ordering::Acquire) {
            return cancelled_response(id);
        }
        let position = params.text_document_position.position;
        let uri = params.text_document_position.text_document.uri;
        let response = self.documents.get(&uri).map_or_else(
            || CompletionResponse::Array(Vec::new()),
            |document| complete(document, position, &self.schemas),
        );
        serialized_response(id, response)
    }

    fn hover_response(&self, request: Request, cancelled: &AtomicBool) -> Response {
        let id = request.id;
        let params = match serde_json::from_value::<HoverParams>(request.params) {
            Ok(params) => params,
            Err(error) => return invalid_params(id, error),
        };
        if cancelled.load(Ordering::Acquire) {
            return cancelled_response(id);
        }
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let response = self
            .documents
            .get(&uri)
            .and_then(|document| hover(document, position, &self.schemas));
        serialized_response(id, response)
    }

    fn definition_response(&self, request: Request, cancelled: &AtomicBool) -> Response {
        let id = request.id;
        let params = match serde_json::from_value::<GotoDefinitionParams>(request.params) {
            Ok(params) => params,
            Err(error) => return invalid_params(id, error),
        };
        if cancelled.load(Ordering::Acquire) {
            return cancelled_response(id);
        }
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let mut workspace = WorkspaceState::new(
            Arc::clone(&self.schemas),
            self.config.workspace_roots.clone(),
        );
        for document in self.documents.values() {
            if cancelled.load(Ordering::Acquire) {
                return cancelled_response(id);
            }
            workspace.insert_document(document);
        }
        let Some(document) = self.documents.get(&uri) else {
            return serialized_response(id, serde_json::Value::Null);
        };
        match definition(document, position, &workspace) {
            Ok(response) => serialized_response(id, response),
            Err(error) => {
                warn!(%error, "definition target lookup failed");
                Response::new_err(
                    id,
                    ErrorCode::RequestFailed as i32,
                    "definition target could not be read".to_string(),
                )
            }
        }
    }

    fn handle_notification(
        &mut self,
        connection: &Connection,
        notification: Notification,
    ) -> Result<(), ServerError> {
        match notification.method.as_str() {
            "textDocument/didOpen" => {
                match serde_json::from_value::<DidOpenTextDocumentParams>(notification.params) {
                    Ok(params) => {
                        let document = params.text_document;
                        let mut state = DocumentState::new(
                            document.uri.clone(),
                            document.version,
                            &document.text,
                            &self.loader,
                        );
                        let _ = state.select_schemas(&self.config);
                        self.publish_diagnostics(connection, &state)?;
                        self.documents.insert(document.uri, Arc::new(state));
                    }
                    Err(error) => warn!(%error, "ignored malformed didOpen"),
                }
            }
            "textDocument/didChange" => {
                match serde_json::from_value::<DidChangeTextDocumentParams>(notification.params) {
                    Ok(params) => {
                        if let Some(mut document) = self.documents.remove(&params.text_document.uri)
                        {
                            let state = Arc::make_mut(&mut document);
                            let changed = match state.apply_changes(
                                params.text_document.version,
                                &params.content_changes,
                                &self.loader,
                            ) {
                                Ok(changed) => changed,
                                Err(error) => {
                                    warn!(%error, "ignored malformed didChange");
                                    false
                                }
                            };
                            if changed {
                                let _ = state.select_schemas(&self.config);
                                self.publish_diagnostics(connection, state)?;
                            }
                            self.documents.insert(params.text_document.uri, document);
                        }
                    }
                    Err(error) => warn!(%error, "ignored malformed didChange"),
                }
            }
            "textDocument/didClose" => {
                match serde_json::from_value::<DidCloseTextDocumentParams>(notification.params) {
                    Ok(params) => {
                        self.documents.remove(&params.text_document.uri);
                        self.publish_empty_diagnostics(connection, params.text_document.uri)?;
                    }
                    Err(error) => warn!(%error, "ignored malformed didClose"),
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn publish_diagnostics(
        &self,
        connection: &Connection,
        document: &DocumentState,
    ) -> Result<(), ServerError> {
        send_notification(
            connection,
            "textDocument/publishDiagnostics",
            PublishDiagnosticsParams {
                uri: document.uri.clone(),
                diagnostics: document_diagnostics(document, &self.schemas, &self.config),
                version: Some(document.version),
            },
        )
    }

    fn publish_empty_diagnostics(
        &self,
        connection: &Connection,
        uri: Uri,
    ) -> Result<(), ServerError> {
        send_notification(
            connection,
            "textDocument/publishDiagnostics",
            PublishDiagnosticsParams {
                uri,
                diagnostics: Vec::new(),
                version: None,
            },
        )
    }
}

type RequestWork = Box<dyn FnOnce(&AtomicBool) -> Response + Send + 'static>;
type ResponseSink = dyn Fn(Response) -> Result<(), ServerError> + Send + Sync;
const REQUEST_WORKER_COUNT: usize = 4;

struct RequestJob {
    id: RequestId,
    cancelled: Arc<AtomicBool>,
    work: RequestWork,
}

#[derive(Default)]
struct ActiveRequests {
    requests: Mutex<HashMap<RequestId, Arc<AtomicBool>>>,
}

impl ActiveRequests {
    fn start(&self, id: RequestId) -> Arc<AtomicBool> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let previous = self
            .requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(id, Arc::clone(&cancelled));
        if let Some(previous) = previous {
            previous.store(true, Ordering::Release);
        }
        cancelled
    }

    fn cancel(&self, id: &RequestId) -> bool {
        let cancelled = self
            .requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(id);
        cancelled.is_some_and(|cancelled| {
            cancelled.store(true, Ordering::Release);
            true
        })
    }

    fn complete(&self, id: &RequestId, cancelled: &Arc<AtomicBool>) -> bool {
        let mut requests = self
            .requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let is_current = requests
            .get(id)
            .is_some_and(|current| Arc::ptr_eq(current, cancelled));
        if is_current {
            requests.remove(id);
        }
        drop(requests);
        is_current && !cancelled.load(Ordering::Acquire)
    }

    fn discard(&self, id: &RequestId, cancelled: &Arc<AtomicBool>) {
        cancelled.store(true, Ordering::Release);
        let mut requests = self
            .requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if requests
            .get(id)
            .is_some_and(|current| Arc::ptr_eq(current, cancelled))
        {
            requests.remove(id);
        }
    }

    fn cancel_all(&self) {
        let requests = std::mem::take(
            &mut *self
                .requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
        for cancelled in requests.into_values() {
            cancelled.store(true, Ordering::Release);
        }
    }
}

struct RequestDispatcher {
    active: Arc<ActiveRequests>,
    responder: Arc<ResponseSink>,
    sender: Option<mpsc::Sender<RequestJob>>,
    workers: Vec<JoinHandle<Result<(), ServerError>>>,
}

impl RequestDispatcher {
    fn new<F>(respond: F) -> Self
    where
        F: Fn(Response) -> Result<(), ServerError> + Send + Sync + 'static,
    {
        let active = Arc::new(ActiveRequests::default());
        let responder: Arc<ResponseSink> = Arc::new(respond);
        let (sender, receiver) = mpsc::channel::<RequestJob>();
        let receiver = Arc::new(Mutex::new(receiver));
        let workers = (0..REQUEST_WORKER_COUNT)
            .map(|_| {
                let worker_active = Arc::clone(&active);
                let worker_responder = Arc::clone(&responder);
                let worker_receiver = Arc::clone(&receiver);
                std::thread::spawn(move || -> Result<(), ServerError> {
                    loop {
                        let job = worker_receiver
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .recv();
                        let Ok(job) = job else {
                            return Ok(());
                        };
                        let RequestJob {
                            id,
                            cancelled,
                            work,
                        } = job;
                        if cancelled.load(Ordering::Acquire) {
                            continue;
                        }
                        let response = work(&cancelled);
                        if worker_active.complete(&id, &cancelled) {
                            worker_responder(response)?;
                        }
                    }
                })
            })
            .collect();
        Self {
            active,
            responder,
            sender: Some(sender),
            workers,
        }
    }

    fn dispatch(&self, id: RequestId, work: RequestWork) -> Result<(), ServerError> {
        let cancelled = self.active.start(id.clone());
        let job = RequestJob {
            id: id.clone(),
            cancelled: Arc::clone(&cancelled),
            work,
        };
        if self
            .sender
            .as_ref()
            .is_none_or(|sender| sender.send(job).is_err())
        {
            self.active.discard(&id, &cancelled);
            return Err(ServerError::RequestWorkerStopped);
        }
        Ok(())
    }

    fn cancel(&self, id: &RequestId) -> Result<bool, ServerError> {
        if !self.active.cancel(id) {
            return Ok(false);
        }
        (self.responder)(cancelled_response(id.clone()))?;
        Ok(true)
    }

    fn shutdown(mut self) -> Result<(), ServerError> {
        self.active.cancel_all();
        drop(self.sender.take());
        let mut failure = None;
        for worker in self.workers.drain(..) {
            let result = worker
                .join()
                .map_err(|_| ServerError::RequestWorkerPanicked)
                .and_then(std::convert::identity);
            if failure.is_none()
                && let Err(error) = result
            {
                failure = Some(error);
            }
        }
        failure.map_or(Ok(()), Err)
    }
}

fn request_id(id: NumberOrString) -> RequestId {
    match id {
        NumberOrString::Number(id) => RequestId::from(id),
        NumberOrString::String(id) => RequestId::from(id),
    }
}

fn cancelled_response(id: RequestId) -> Response {
    Response::new_err(
        id,
        ErrorCode::RequestCanceled as i32,
        "request cancelled".to_string(),
    )
}

fn send_notification(
    connection: &Connection,
    method: &str,
    params: impl serde::Serialize,
) -> Result<(), ServerError> {
    let notification = Notification::new(method.to_owned(), params);
    connection
        .sender
        .send(Message::Notification(notification))
        .map_err(|error| ServerError::Transport(error.to_string()))
}

fn serialized_response(id: RequestId, value: impl serde::Serialize) -> Response {
    match serde_json::to_value(value) {
        Ok(value) => Response::new_ok(id, value),
        Err(error) => Response::new_err(
            id,
            ErrorCode::InternalError as i32,
            format!("response serialization failed: {error}"),
        ),
    }
}

fn invalid_params(id: RequestId, error: serde_json::Error) -> Response {
    Response::new_err(
        id,
        ErrorCode::InvalidParams as i32,
        format!("invalid request parameters: {error}"),
    )
}

fn server_config(initialize_params: &serde_json::Value) -> Result<ServerConfig, ServerError> {
    let mut config = ServerConfig::default();
    if let Some(folders) = initialize_params
        .get("workspaceFolders")
        .and_then(serde_json::Value::as_array)
    {
        for uri in folders.iter().filter_map(|folder| folder.get("uri")) {
            push_workspace_root(&mut config, uri);
        }
    }
    if config.workspace_roots.is_empty()
        && let Some(uri) = initialize_params.get("rootUri")
    {
        push_workspace_root(&mut config, uri);
    }

    if let Some(options) = initialize_params.get("initializationOptions") {
        let options = options.get("yamalgam").unwrap_or(options);
        if !options.is_null() {
            let options: InitializationOptions = serde_json::from_value(options.clone())?;
            config.schema_associations = options
                .schemas
                .into_iter()
                .map(InitializationSchemaAssociation::into_schema_association)
                .collect();
            if let Some(maximum) = options.max_diagnostics {
                config.max_diagnostics = maximum;
            }
        }
    }
    Ok(config)
}

fn push_workspace_root(config: &mut ServerConfig, uri: &serde_json::Value) {
    let Some(uri) = uri.as_str() else {
        return;
    };
    let Ok(id) = yamalgam_resolve::ResourceId::new(uri) else {
        return;
    };
    if let Some(path) = id.as_path() {
        config.workspace_roots.push(path.to_path_buf());
    }
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct InitializationOptions {
    schemas: Vec<InitializationSchemaAssociation>,
    max_diagnostics: Option<usize>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitializationSchemaAssociation {
    uri: String,
    file_match: Vec<String>,
}

impl InitializationSchemaAssociation {
    fn into_schema_association(self) -> SchemaAssociation {
        SchemaAssociation {
            uri: self.uri,
            file_match: self.file_match,
        }
    }
}

fn schema_workspace(config: &ServerConfig) -> Result<SchemaWorkspace, ServerError> {
    if config.workspace_roots.is_empty() {
        return Ok(SchemaWorkspace::empty(LoaderConfig::default()));
    }

    let mut loader = LoaderConfig::default();
    loader.resolution.refs.mode = ResolutionMode::Explicit;
    loader.resolution.refs.allow_schemes = vec!["file".to_owned()];
    loader.resolution.files.root = config.workspace_roots.first().cloned();
    let retriever = WorkspaceFileRetriever::open(&config.workspace_roots, &loader)?;
    Ok(SchemaWorkspace::new(Arc::new(retriever), loader))
}

struct WorkspaceFileRetriever {
    roots: Vec<(std::path::PathBuf, FileRetriever)>,
}

impl WorkspaceFileRetriever {
    fn open(roots: &[std::path::PathBuf], loader: &LoaderConfig) -> Result<Self, RetrieveError> {
        let mut retrievers = Vec::with_capacity(roots.len());
        for root in roots {
            let root = std::path::absolute(root).map_err(|source| RetrieveError::Io {
                id: yamalgam_resolve::ResourceId::from_file(root),
                source,
            })?;
            let mut policy = loader.resolution.files.clone();
            policy.root = Some(root.clone());
            retrievers.push((root, FileRetriever::open(&policy)?));
        }
        Ok(Self { roots: retrievers })
    }
}

impl ResourceRetriever for WorkspaceFileRetriever {
    fn retrieve(&self, request: &ResourceRequest) -> Result<RetrievedResource, RetrieveError> {
        let Some(path) = request.id.as_path() else {
            return Err(RetrieveError::UnsupportedScheme(
                request.id.scheme().to_owned(),
            ));
        };
        let Some((_, retriever)) = self.roots.iter().find(|(root, _)| path.starts_with(root))
        else {
            return Err(RetrieveError::OutsideRoot(request.id.clone()));
        };
        retriever.retrieve(request)
    }
}

/// Language-server startup or transport failure.
#[derive(Debug, Error)]
pub enum ServerError {
    /// LSP lifecycle violation or disconnection.
    #[error(transparent)]
    Protocol(#[from] lsp_server::ProtocolError),
    /// Standard-I/O transport failure.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// JSON serialization failure.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// Workspace schema retriever configuration failed.
    #[error(transparent)]
    Retrieval(#[from] RetrieveError),
    /// In-memory or standard-I/O channel disconnected.
    #[error("language-server transport failed: {0}")]
    Transport(String),
    /// The request worker stopped before accepting queued work.
    #[error("language-server request worker stopped")]
    RequestWorkerStopped,
    /// The request worker panicked during shutdown.
    #[error("language-server request worker panicked")]
    RequestWorkerPanicked,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn transport_join_is_skipped_after_server_failure() {
        let joined = AtomicBool::new(false);
        let result = finish_transport(
            Err(ServerError::Transport("boom".to_owned())),
            || -> io::Result<()> {
                joined.store(true, Ordering::Release);
                Ok(())
            },
        );

        assert!(matches!(
            result,
            Err(ServerError::Transport(message)) if message == "boom"
        ));
        assert!(!joined.load(Ordering::Acquire));
    }

    #[test]
    fn transport_join_runs_after_clean_server_exit() {
        let joined = AtomicBool::new(false);
        let result = finish_transport(Ok(()), || -> io::Result<()> {
            joined.store(true, Ordering::Release);
            Ok(())
        });

        assert!(result.is_ok());
        assert!(joined.load(Ordering::Acquire));
    }

    #[test]
    fn request_dispatcher_cancels_active_work_without_poisoning_reused_ids() {
        let (response_sender, response_receiver) = mpsc::channel();
        let dispatcher = Arc::new(RequestDispatcher::new(move |response| {
            response_sender
                .send(response)
                .map_err(|error| ServerError::Transport(error.to_string()))
        }));
        let id = RequestId::from(7);
        let work_id = id.clone();
        let (started_sender, started_receiver) = mpsc::channel();
        let (release_sender, release_receiver) = mpsc::channel();
        let (dispatched_sender, dispatched_receiver) = mpsc::channel();
        let dispatching = Arc::clone(&dispatcher);
        let dispatch_id = id.clone();
        let dispatch_thread = std::thread::spawn(move || {
            dispatching
                .dispatch(
                    dispatch_id,
                    Box::new(move |_| {
                        started_sender.send(()).unwrap();
                        release_receiver.recv().unwrap();
                        Response::new_ok(work_id, serde_json::Value::Null)
                    }),
                )
                .unwrap();
            dispatched_sender.send(()).unwrap();
        });

        started_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("worker should start request work");
        dispatched_receiver
            .recv_timeout(Duration::from_millis(100))
            .expect("dispatch must not block the receiver loop");
        assert!(dispatcher.cancel(&id).unwrap());
        let cancelled = response_receiver
            .recv_timeout(Duration::from_millis(100))
            .expect("active cancellation should respond before work returns");
        assert_eq!(
            cancelled
                .response_result
                .expect_err("active request should be cancelled")
                .code,
            ErrorCode::RequestCanceled as i32
        );
        assert!(!dispatcher.cancel(&id).unwrap());

        let reused_id = id.clone();
        dispatcher
            .dispatch(
                id,
                Box::new(move |_| Response::new_ok(reused_id, serde_json::json!({"fresh": true}))),
            )
            .unwrap();
        let reused = response_receiver
            .recv_timeout(Duration::from_millis(100))
            .expect("cancelled work must not block a reused request ID");
        assert_eq!(
            reused.response_result.unwrap(),
            serde_json::json!({"fresh": true})
        );
        release_sender.send(()).unwrap();
        assert!(
            response_receiver
                .recv_timeout(Duration::from_millis(50))
                .is_err()
        );

        dispatch_thread.join().unwrap();
        Arc::try_unwrap(dispatcher)
            .unwrap_or_else(|_| panic!("dispatcher should have one owner"))
            .shutdown()
            .unwrap();
    }
}

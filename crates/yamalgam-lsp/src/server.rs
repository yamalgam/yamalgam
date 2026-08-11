//! Synchronous LSP lifecycle and document-state loop.

use std::collections::{HashMap, HashSet};
use std::io;

use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    CancelParams, CompletionOptions, CompletionParams, CompletionResponse,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    HoverParams, HoverProviderCapability, InitializeResult, NumberOrString, OneOf,
    PositionEncodingKind, PublishDiagnosticsParams, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions, Uri,
};
use thiserror::Error;
use tracing::warn;
use yamalgam_core::LoaderConfig;

use crate::{DocumentState, SchemaWorkspace, ServerConfig, complete, document_diagnostics, hover};

/// Run the language server over standard input and standard output.
pub fn run_stdio() -> Result<(), ServerError> {
    let (connection, threads) = Connection::stdio();
    let result = run(connection);
    threads.join()?;
    result
}

/// Run one synchronous language-server connection to completion.
pub fn run(connection: Connection) -> Result<(), ServerError> {
    let (initialize_id, _initialize_params) = connection.initialize_start()?;
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

    let mut server = Server::default();
    while let Ok(message) = connection.receiver.recv() {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    return Ok(());
                }
                server.handle_request(&connection, request)?;
            }
            Message::Notification(notification) => {
                server.handle_notification(&connection, notification)?;
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

struct Server {
    documents: HashMap<Uri, DocumentState>,
    cancelled: HashSet<RequestId>,
    loader: LoaderConfig,
    config: ServerConfig,
    schemas: SchemaWorkspace,
}

impl Default for Server {
    fn default() -> Self {
        let loader = LoaderConfig::default();
        Self {
            documents: HashMap::new(),
            cancelled: HashSet::new(),
            schemas: SchemaWorkspace::empty(loader.clone()),
            loader,
            config: ServerConfig::default(),
        }
    }
}

impl Server {
    fn handle_request(
        &mut self,
        connection: &Connection,
        request: Request,
    ) -> Result<(), ServerError> {
        if self.cancelled.remove(&request.id) {
            return send_response(
                connection,
                Response::new_err(
                    request.id,
                    ErrorCode::RequestCanceled as i32,
                    "request cancelled".to_string(),
                ),
            );
        }

        let response = match request.method.as_str() {
            "textDocument/completion" => self.completion_response(request),
            "textDocument/hover" => self.hover_response(request),
            "textDocument/definition" => Response::new_ok(request.id, serde_json::Value::Null),
            _ => Response::new_err(
                request.id,
                ErrorCode::MethodNotFound as i32,
                format!("unknown request method {}", request.method),
            ),
        };
        send_response(connection, response)
    }

    fn completion_response(&self, request: Request) -> Response {
        let id = request.id;
        let params = match serde_json::from_value::<CompletionParams>(request.params) {
            Ok(params) => params,
            Err(error) => return invalid_params(id, error),
        };
        let position = params.text_document_position.position;
        let uri = params.text_document_position.text_document.uri;
        let response = self.documents.get(&uri).map_or_else(
            || CompletionResponse::Array(Vec::new()),
            |document| complete(document, position, &self.schemas),
        );
        serialized_response(id, response)
    }

    fn hover_response(&self, request: Request) -> Response {
        let id = request.id;
        let params = match serde_json::from_value::<HoverParams>(request.params) {
            Ok(params) => params,
            Err(error) => return invalid_params(id, error),
        };
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let response = self
            .documents
            .get(&uri)
            .and_then(|document| hover(document, position, &self.schemas));
        serialized_response(id, response)
    }

    fn handle_notification(
        &mut self,
        connection: &Connection,
        notification: Notification,
    ) -> Result<(), ServerError> {
        match notification.method.as_str() {
            "$/cancelRequest" => {
                match serde_json::from_value::<CancelParams>(notification.params) {
                    Ok(params) => {
                        self.cancelled.insert(request_id(params.id));
                    }
                    Err(error) => warn!(%error, "ignored malformed cancellation"),
                }
            }
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
                        self.documents.insert(document.uri, state);
                    }
                    Err(error) => warn!(%error, "ignored malformed didOpen"),
                }
            }
            "textDocument/didChange" => {
                match serde_json::from_value::<DidChangeTextDocumentParams>(notification.params) {
                    Ok(params) => {
                        if let Some(mut document) = self.documents.remove(&params.text_document.uri)
                        {
                            let changed = match document.apply_changes(
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
                                let _ = document.select_schemas(&self.config);
                                self.publish_diagnostics(connection, &document)?;
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

fn request_id(id: NumberOrString) -> RequestId {
    match id {
        NumberOrString::Number(id) => RequestId::from(id),
        NumberOrString::String(id) => RequestId::from(id),
    }
}

fn send_response(connection: &Connection, response: Response) -> Result<(), ServerError> {
    connection
        .sender
        .send(Message::Response(response))
        .map_err(|error| ServerError::Transport(error.to_string()))
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
    /// In-memory or standard-I/O channel disconnected.
    #[error("language-server transport failed: {0}")]
    Transport(String),
}

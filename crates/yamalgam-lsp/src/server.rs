//! Synchronous LSP lifecycle and document-state loop.

use std::collections::{HashMap, HashSet};
use std::io;

use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    CancelParams, CompletionOptions, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, HoverProviderCapability, InitializeResult, NumberOrString, OneOf,
    PositionEncodingKind, ServerCapabilities, ServerInfo, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, Uri,
};
use thiserror::Error;
use tracing::warn;
use yamalgam_core::LoaderConfig;

use crate::DocumentState;

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
                server.handle_notification(notification);
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

#[derive(Default)]
struct Server {
    documents: HashMap<Uri, DocumentState>,
    cancelled: HashSet<RequestId>,
    loader: LoaderConfig,
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
            "textDocument/completion" | "textDocument/hover" | "textDocument/definition" => {
                Response::new_ok(request.id, serde_json::Value::Null)
            }
            _ => Response::new_err(
                request.id,
                ErrorCode::MethodNotFound as i32,
                format!("unknown request method {}", request.method),
            ),
        };
        send_response(connection, response)
    }

    fn handle_notification(&mut self, notification: Notification) {
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
                        self.documents.insert(
                            document.uri.clone(),
                            DocumentState::new(
                                document.uri,
                                document.version,
                                &document.text,
                                &self.loader,
                            ),
                        );
                    }
                    Err(error) => warn!(%error, "ignored malformed didOpen"),
                }
            }
            "textDocument/didChange" => {
                match serde_json::from_value::<DidChangeTextDocumentParams>(notification.params) {
                    Ok(params) => {
                        if let Some(document) = self.documents.get_mut(&params.text_document.uri)
                            && let Err(error) = document.apply_changes(
                                params.text_document.version,
                                &params.content_changes,
                                &self.loader,
                            )
                        {
                            warn!(%error, "ignored malformed didChange");
                        }
                    }
                    Err(error) => warn!(%error, "ignored malformed didChange"),
                }
            }
            "textDocument/didClose" => {
                match serde_json::from_value::<DidCloseTextDocumentParams>(notification.params) {
                    Ok(params) => {
                        self.documents.remove(&params.text_document.uri);
                    }
                    Err(error) => warn!(%error, "ignored malformed didClose"),
                }
            }
            _ => {}
        }
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

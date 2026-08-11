#![allow(missing_docs)]

use std::str::FromStr;
use std::time::Duration;

use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    CompletionOptions, InitializeResult, OneOf, Position, PositionEncodingKind, Range,
    TextDocumentContentChangeEvent, TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
};
use serde_json::json;
use yamalgam_core::LoaderConfig;
use yamalgam_lsp::{DocumentState, run};

fn response(connection: &Connection, expected: i32) -> Response {
    loop {
        let message = connection
            .receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("server should respond");
        if let Message::Response(response) = message {
            assert_eq!(response.id, RequestId::from(expected));
            return response;
        }
    }
}

#[test]
fn protocol_lifecycle_and_capabilities_work_over_an_in_memory_connection() {
    let (server, client) = Connection::memory();
    let worker = std::thread::spawn(move || run(server));

    client
        .sender
        .send(Message::Request(Request {
            id: RequestId::from(1),
            method: "initialize".to_string(),
            params: json!({
                "processId": null,
                "capabilities": {
                    "general": { "positionEncodings": ["utf-16"] }
                }
            }),
        }))
        .unwrap();
    let initialized = response(&client, 1);
    let result: InitializeResult = serde_json::from_value(
        initialized
            .response_result
            .expect("initialize result should succeed"),
    )
    .unwrap();
    assert_eq!(
        result.capabilities.position_encoding,
        Some(PositionEncodingKind::UTF16)
    );
    let TextDocumentSyncCapability::Options(sync) =
        result.capabilities.text_document_sync.expect("text sync")
    else {
        panic!("expected detailed text sync options");
    };
    assert_eq!(sync.open_close, Some(true));
    assert_eq!(sync.change, Some(TextDocumentSyncKind::INCREMENTAL));
    assert_eq!(
        result.capabilities.completion_provider,
        Some(CompletionOptions {
            trigger_characters: Some(vec![":".to_string(), "-".to_string()]),
            ..CompletionOptions::default()
        })
    );
    assert_eq!(
        result.capabilities.hover_provider,
        Some(lsp_types::HoverProviderCapability::Simple(true))
    );
    assert_eq!(
        result.capabilities.definition_provider,
        Some(OneOf::Left(true))
    );

    for notification in [
        Notification {
            method: "initialized".to_string(),
            params: json!({}),
        },
        Notification {
            method: "textDocument/didOpen".to_string(),
            params: json!({
                "textDocument": {
                    "uri": "file:///tmp/example.yaml",
                    "languageId": "yaml",
                    "version": 1,
                    "text": "name: first\n"
                }
            }),
        },
        Notification {
            method: "textDocument/didChange".to_string(),
            params: json!({
                "textDocument": {
                    "uri": "file:///tmp/example.yaml",
                    "version": 2
                },
                "contentChanges": [{
                    "range": {
                        "start": { "line": 0, "character": 6 },
                        "end": { "line": 0, "character": 11 }
                    },
                    "text": "second"
                }]
            }),
        },
        Notification {
            method: "textDocument/didClose".to_string(),
            params: json!({
                "textDocument": { "uri": "file:///tmp/example.yaml" }
            }),
        },
    ] {
        client
            .sender
            .send(Message::Notification(notification))
            .unwrap();
    }

    client
        .sender
        .send(Message::Notification(Notification {
            method: "$/cancelRequest".to_string(),
            params: json!({ "id": 2 }),
        }))
        .unwrap();
    client
        .sender
        .send(Message::Request(Request {
            id: RequestId::from(2),
            method: "textDocument/completion".to_string(),
            params: json!({}),
        }))
        .unwrap();
    assert_eq!(
        response(&client, 2)
            .response_result
            .expect_err("unknown cancellation must not poison a future request")
            .code,
        ErrorCode::InvalidParams as i32
    );

    client
        .sender
        .send(Message::Request(Request {
            id: RequestId::from(3),
            method: "yamalgam/unknown".to_string(),
            params: json!({}),
        }))
        .unwrap();
    let unknown = response(&client, 3);
    assert_eq!(
        unknown.response_result.expect_err("method error").code,
        ErrorCode::MethodNotFound as i32
    );

    client
        .sender
        .send(Message::Request(Request {
            id: RequestId::from(4),
            method: "shutdown".to_string(),
            params: serde_json::Value::Null,
        }))
        .unwrap();
    assert!(response(&client, 4).response_result.is_ok());
    client
        .sender
        .send(Message::Notification(Notification {
            method: "exit".to_string(),
            params: serde_json::Value::Null,
        }))
        .unwrap();

    worker
        .join()
        .expect("server thread should not panic")
        .unwrap();
}

#[test]
fn protocol_definition_surfaces_resource_failures_without_leaking_paths() {
    let (server, client) = Connection::memory();
    let worker = std::thread::spawn(move || run(server));
    let root = std::env::temp_dir();
    let root_uri = format!("file://{}", root.display())
        .trim_end_matches('/')
        .to_owned();
    let missing = format!("yamalgam-missing-{}.yaml", std::process::id());
    let document_uri = format!("{root_uri}/definition-source.yaml");
    let text = format!("selected: {{$ref: {missing}#/value}}\n");

    client
        .sender
        .send(Message::Request(Request {
            id: RequestId::from(1),
            method: "initialize".to_string(),
            params: json!({
                "processId": null,
                "rootUri": root_uri,
                "capabilities": {}
            }),
        }))
        .unwrap();
    assert!(response(&client, 1).response_result.is_ok());
    client
        .sender
        .send(Message::Notification(Notification {
            method: "initialized".to_string(),
            params: json!({}),
        }))
        .unwrap();
    client
        .sender
        .send(Message::Notification(Notification {
            method: "textDocument/didOpen".to_string(),
            params: json!({
                "textDocument": {
                    "uri": document_uri,
                    "languageId": "yaml",
                    "version": 1,
                    "text": text
                }
            }),
        }))
        .unwrap();
    client
        .sender
        .send(Message::Request(Request {
            id: RequestId::from(2),
            method: "textDocument/definition".to_string(),
            params: json!({
                "textDocument": { "uri": document_uri },
                "position": { "line": 0, "character": 20 }
            }),
        }))
        .unwrap();

    let error = response(&client, 2)
        .response_result
        .expect_err("missing workspace target should be a request failure");
    assert_eq!(error.code, ErrorCode::RequestFailed as i32);
    assert_eq!(error.message, "definition target could not be read");
    assert!(!error.message.contains(&missing));

    client
        .sender
        .send(Message::Request(Request {
            id: RequestId::from(3),
            method: "shutdown".to_string(),
            params: serde_json::Value::Null,
        }))
        .unwrap();
    assert!(response(&client, 3).response_result.is_ok());
    client
        .sender
        .send(Message::Notification(Notification {
            method: "exit".to_string(),
            params: serde_json::Value::Null,
        }))
        .unwrap();
    worker
        .join()
        .expect("server thread should not panic")
        .unwrap();
}

#[test]
fn protocol_document_state_applies_changes_and_discards_stale_versions() {
    let config = LoaderConfig::default();
    let uri = Uri::from_str("file:///tmp/example.yaml").unwrap();
    let mut document = DocumentState::new(uri, 1, "name: first\n", &config);
    assert_eq!(document.located.len(), 1);

    let applied = document
        .apply_changes(
            2,
            &[TextDocumentContentChangeEvent {
                range: Some(Range {
                    start: Position::new(0, 6),
                    end: Position::new(0, 11),
                }),
                range_length: None,
                text: "second".to_string(),
            }],
            &config,
        )
        .unwrap();
    assert!(applied);
    assert_eq!(document.version, 2);
    assert_eq!(document.syntax.text(), "name: second\n");

    let stale = document
        .apply_changes(
            2,
            &[TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "stale: true\n".to_string(),
            }],
            &config,
        )
        .unwrap();
    assert!(!stale);
    assert_eq!(document.syntax.text(), "name: second\n");

    assert!(
        document
            .apply_changes(
                3,
                &[TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: "bad: ]\n".to_string(),
                }],
                &config,
            )
            .unwrap()
    );
    assert_eq!(document.syntax.text(), "bad: ]\n");
    assert!(!document.syntax.syntax().diagnostics().is_empty());
    assert!(document.located.is_empty());

    assert!(
        document
            .apply_changes(
                4,
                &[TextDocumentContentChangeEvent {
                    range: Some(Range {
                        start: Position::new(99, 0),
                        end: Position::new(99, 1),
                    }),
                    range_length: None,
                    text: String::new(),
                }],
                &config,
            )
            .is_err()
    );
    assert_eq!(document.version, 3);
}

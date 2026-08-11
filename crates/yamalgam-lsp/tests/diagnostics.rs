#![allow(missing_docs)]

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use lsp_server::{Connection, Message, Notification, Request, RequestId};
use lsp_types::{DiagnosticSeverity, PublishDiagnosticsParams, Uri};
use serde_json::json;
use yamalgam_core::{LoaderConfig, ResolutionMode};
use yamalgam_lsp::{DocumentState, SchemaWorkspace, ServerConfig, document_diagnostics, run};
use yamalgam_resolve::{MemoryRetriever, ResourceId, ResourceProvenance, RetrievedResource};

fn schema_workspace(uri: &str, schema: &str) -> (SchemaWorkspace, LoaderConfig) {
    let mut schema_config = LoaderConfig::trusted();
    schema_config.resolution.refs.mode = ResolutionMode::Explicit;
    schema_config.resolution.refs.allow_schemes = vec!["memory".to_owned()];
    let mut retriever = MemoryRetriever::default();
    retriever.insert(RetrievedResource {
        id: ResourceId::new(uri).unwrap(),
        bytes: schema.as_bytes().to_vec(),
        provenance: ResourceProvenance::default(),
    });
    (
        SchemaWorkspace::new(Arc::new(retriever), schema_config),
        LoaderConfig::default(),
    )
}

fn selected_document(text: &str, loader: &LoaderConfig, server: &ServerConfig) -> DocumentState {
    let mut document = DocumentState::new(
        Uri::from_str("file:///workspace/example.yaml").unwrap(),
        1,
        text,
        loader,
    );
    document.select_schemas(server).unwrap();
    document
}

#[test]
fn diagnostics_report_schema_failures_and_clear_after_a_valid_fix() {
    let schema_uri = "memory://schemas/service";
    let (schemas, loader) = schema_workspace(
        schema_uri,
        r#"{
          "type": "object",
          "required": ["name"],
          "properties": {"name": {"type": "string"}}
        }"#,
    );
    let server = ServerConfig {
        schema_associations: vec![yamalgam_schema::SchemaAssociation {
            uri: schema_uri.to_owned(),
            file_match: vec!["*.yaml".to_owned()],
        }],
        ..ServerConfig::default()
    };

    let missing = selected_document("enabled: true\n", &loader, &server);
    let diagnostics = document_diagnostics(&missing, &schemas, &server);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::ERROR));
    assert_eq!(
        diagnostics[0].code,
        Some(lsp_types::NumberOrString::String(
            "schema/required".to_owned()
        )),
        "{diagnostics:#?}"
    );
    assert_eq!(diagnostics[0].source.as_deref(), Some("yamalgam"));
    assert_eq!(diagnostics[0].data.as_ref().unwrap()["documentIndex"], 0);
    assert_eq!(
        diagnostics[0].data.as_ref().unwrap()["schemaUri"],
        schema_uri
    );

    let valid = selected_document("name: yamalgam\n", &loader, &server);
    assert!(document_diagnostics(&valid, &schemas, &server).is_empty());
}

#[test]
fn diagnostics_preserve_unicode_ranges_and_document_indexes() {
    let schemas = SchemaWorkspace::empty(LoaderConfig::default());
    let server = ServerConfig::default();
    let document = DocumentState::new(
        Uri::from_str("file:///workspace/unicode.yaml").unwrap(),
        7,
        "emoji: 😀\nbad: ]\n---\nalso: ]\n",
        &LoaderConfig::default(),
    );

    let diagnostics = document_diagnostics(&document, &schemas, &server);
    assert!(diagnostics.len() >= 2);
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.source.as_deref() == Some("yamalgam")
            && diagnostic.code == Some(lsp_types::NumberOrString::String("yaml/syntax".to_owned()))
    }));
    assert_eq!(diagnostics[0].range.start.line, 1);
    assert_eq!(diagnostics[0].data.as_ref().unwrap()["documentIndex"], 0);
    assert_eq!(diagnostics[1].data.as_ref().unwrap()["documentIndex"], 1);
}

#[test]
fn diagnostics_publish_current_versions_and_skip_stale_changes() {
    let (server, client) = Connection::memory();
    let worker = std::thread::spawn(move || run(server));

    client
        .sender
        .send(Message::Request(Request {
            id: RequestId::from(1),
            method: "initialize".to_owned(),
            params: json!({"processId": null, "capabilities": {}}),
        }))
        .unwrap();
    let _ = client
        .receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    client
        .sender
        .send(Message::Notification(Notification {
            method: "initialized".to_owned(),
            params: json!({}),
        }))
        .unwrap();

    client
        .sender
        .send(Message::Notification(Notification {
            method: "textDocument/didOpen".to_owned(),
            params: json!({
                "textDocument": {
                    "uri": "file:///tmp/diagnostics.yaml",
                    "languageId": "yaml",
                    "version": 3,
                    "text": "bad: ]\n"
                }
            }),
        }))
        .unwrap();
    let published = client.receiver.recv_timeout(Duration::from_secs(5));
    if let Err(error) = &published {
        panic!(
            "diagnostics should be published ({error}); server returned {:?}",
            worker.join()
        );
    }
    let Message::Notification(published) = published.unwrap() else {
        panic!("expected publishDiagnostics notification");
    };
    let params: PublishDiagnosticsParams = serde_json::from_value(published.params).unwrap();
    assert_eq!(params.version, Some(3));
    assert!(!params.diagnostics.is_empty());

    client
        .sender
        .send(Message::Notification(Notification {
            method: "textDocument/didChange".to_owned(),
            params: json!({
                "textDocument": {"uri": "file:///tmp/diagnostics.yaml", "version": 3},
                "contentChanges": [{"text": "valid: true\n"}]
            }),
        }))
        .unwrap();
    assert!(
        client
            .receiver
            .recv_timeout(Duration::from_millis(100))
            .is_err(),
        "stale work must not publish diagnostics"
    );

    client
        .sender
        .send(Message::Request(Request {
            id: RequestId::from(2),
            method: "shutdown".to_owned(),
            params: serde_json::Value::Null,
        }))
        .unwrap();
    let _ = client
        .receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    client
        .sender
        .send(Message::Notification(Notification {
            method: "exit".to_owned(),
            params: serde_json::Value::Null,
        }))
        .unwrap();
    worker.join().unwrap().unwrap();
}

#[test]
fn initialization_options_load_workspace_local_schema_associations() {
    let workspace = std::env::temp_dir().join(format!(
        "yamalgam-lsp-schema-settings-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::write(
        workspace.join("schema.yaml"),
        "type: object\nrequired: [name]\n",
    )
    .unwrap();
    let workspace_uri = yamalgam_resolve::ResourceId::from_file(&workspace).to_string();
    let document_uri =
        yamalgam_resolve::ResourceId::from_file(workspace.join("service.yaml")).to_string();

    let (server, client) = Connection::memory();
    let worker = std::thread::spawn(move || run(server));

    client
        .sender
        .send(Message::Request(Request {
            id: RequestId::from(1),
            method: "initialize".to_owned(),
            params: json!({
                "processId": null,
                "rootUri": workspace_uri,
                "capabilities": {},
                "initializationOptions": {
                    "yamalgam": {
                        "schemas": [{
                            "uri": "./schema.yaml",
                            "fileMatch": ["*.yaml"]
                        }],
                        "maxDiagnostics": 8
                    }
                }
            }),
        }))
        .unwrap();
    let _ = client
        .receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    client
        .sender
        .send(Message::Notification(Notification {
            method: "initialized".to_owned(),
            params: json!({}),
        }))
        .unwrap();
    client
        .sender
        .send(Message::Notification(Notification {
            method: "textDocument/didOpen".to_owned(),
            params: json!({
                "textDocument": {
                    "uri": document_uri,
                    "languageId": "yaml",
                    "version": 1,
                    "text": "enabled: true\n"
                }
            }),
        }))
        .unwrap();

    let Message::Notification(published) = client
        .receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("schema diagnostics should be published")
    else {
        panic!("expected publishDiagnostics notification");
    };
    let params: PublishDiagnosticsParams = serde_json::from_value(published.params).unwrap();
    assert_eq!(params.diagnostics.len(), 1, "{:#?}", params.diagnostics);
    assert_eq!(
        params.diagnostics[0].code,
        Some(lsp_types::NumberOrString::String(
            "schema/required".to_owned()
        ))
    );

    client
        .sender
        .send(Message::Request(Request {
            id: RequestId::from(2),
            method: "shutdown".to_owned(),
            params: serde_json::Value::Null,
        }))
        .unwrap();
    let _ = client
        .receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    client
        .sender
        .send(Message::Notification(Notification {
            method: "exit".to_owned(),
            params: serde_json::Value::Null,
        }))
        .unwrap();
    worker.join().unwrap().unwrap();
    std::fs::remove_dir_all(workspace).unwrap();
}

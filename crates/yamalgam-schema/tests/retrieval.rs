#![allow(missing_docs)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak, mpsc};
use std::time::Duration;

use serde_json::json;
use yamalgam_core::{LoaderConfig, ResolutionMode, ResourceLimit, Value};
use yamalgam_resolve::{
    FileRetriever, LibrebarRetriever, MemoryRetriever, ResourceId, ResourceProvenance,
    ResourceRequest, ResourceRetriever, RetrieveError, RetrievedResource,
};
use yamalgam_schema::{
    SchemaDocument, SchemaError, SchemaOptions, SchemaRetrieveError, SchemaSession,
};

fn resource(id: &str, yaml: &str) -> RetrievedResource {
    RetrievedResource {
        id: ResourceId::new(id).expect("valid fixture URI"),
        bytes: yaml.as_bytes().to_vec(),
        provenance: ResourceProvenance::default(),
    }
}

fn memory(resources: &[(&str, &str)]) -> MemoryRetriever {
    let mut retriever = MemoryRetriever::default();
    for (id, yaml) in resources {
        retriever.insert(resource(id, yaml));
    }
    retriever
}

fn config(schemes: &[&str]) -> LoaderConfig {
    let mut config = LoaderConfig::moderate();
    config.resolution.refs.mode = ResolutionMode::Automatic;
    config.resolution.refs.allow_schemes = schemes.iter().map(|value| (*value).into()).collect();
    config
}

fn document(id: &str, yaml: &str) -> SchemaDocument {
    SchemaDocument::new(
        ResourceId::new(id).expect("valid fixture URI"),
        yamalgam_compose::from_str_single_located(yaml).expect("valid fixture schema"),
    )
}

fn value(value: serde_json::Value) -> Value {
    serde_json::from_value(value).expect("JSON fixture fits Value")
}

#[test]
fn retrieval_resolves_relative_absolute_and_cyclic_schema_graphs() {
    let retriever = memory(&[
        (
            "memory://schemas/root.yaml",
            "type: object\nrequired: [name, count, node]\nproperties:\n  name: {$ref: ./string.yaml}\n  count: {$ref: memory://shared/count.yaml}\n  node: {$ref: ./node.yaml}\n",
        ),
        ("memory://schemas/string.yaml", "type: string\n"),
        ("memory://shared/count.yaml", "type: integer\nminimum: 1\n"),
        (
            "memory://schemas/node.yaml",
            "$id: memory://schemas/node.yaml\ntype: object\nproperties:\n  next: {$ref: memory://schemas/node.yaml}\n",
        ),
    ]);
    let session = SchemaSession::new(Arc::new(retriever), config(&["memory"]));
    let root = session
        .load(&ResourceId::new("memory://schemas/root.yaml").unwrap())
        .expect("root loads");
    let validator = session.compile(root).expect("schema graph compiles");

    assert!(
        validator
            .validate(&value(json!({
                "name": "api",
                "count": 2,
                "node": {"next": {"next": {}}}
            })))
            .valid
    );
    assert!(
        !validator
            .validate(&value(json!({"name": 1, "count": 0, "node": {}})))
            .valid
    );
}

#[test]
fn retrieval_cache_returns_shared_document_handles() {
    let id = ResourceId::new("memory://schemas/shared.yaml").unwrap();
    let session = SchemaSession::new(
        Arc::new(memory(&[(
            "memory://schemas/shared.yaml",
            "type: object\nproperties:\n  name: {type: string}\n",
        )])),
        config(&["memory"]),
    );

    let first = session.load(&id).unwrap();
    let second = session.load(&id).unwrap();

    assert!(std::ptr::eq(&first.document, &second.document));
}

#[test]
fn retrieval_compile_options_control_diagnostic_limits() {
    let session = SchemaSession::new(Arc::new(MemoryRetriever::default()), config(&["memory"]));
    let validator = session
        .compile_with_options(
            document(
                "memory://schemas/root.yaml",
                "type: object\nrequired: [first, second]\n",
            ),
            SchemaOptions {
                max_diagnostics: 1,
                ..SchemaOptions::default()
            },
        )
        .unwrap();

    let report = validator.validate(&value(json!({})));
    assert_eq!(report.diagnostics.len(), 2);
    assert_eq!(report.diagnostics[1].code, "schema/diagnostic-limit");
}

#[test]
fn retrieval_charges_cache_hits_as_edges_and_enforces_depth() {
    let duplicate = document(
        "memory://schemas/root.yaml",
        "allOf:\n  - {$ref: ./shared.yaml}\n  - {$ref: ./shared.yaml}\n",
    );
    let mut edge_config = config(&["memory"]);
    edge_config.resolution.limits.max_edges = Some(1);
    let edge_session = SchemaSession::new(
        Arc::new(memory(&[(
            "memory://schemas/shared.yaml",
            "type: string\n",
        )])),
        edge_config,
    );
    assert!(matches!(
        edge_session.compile(duplicate),
        Err(SchemaError::Retrieval(SchemaRetrieveError::LimitExceeded(error)))
            if error.limit == ResourceLimit::ResolutionEdges
                && error.observed == 2
                && error.maximum == 1
    ));

    let deep = document(
        "memory://schemas/root.yaml",
        "$ref: memory://schemas/one.yaml\n",
    );
    let mut depth_config = config(&["memory"]);
    depth_config.resolution.limits.max_depth = Some(1);
    let depth_session = SchemaSession::new(
        Arc::new(memory(&[
            ("memory://schemas/one.yaml", "$ref: ./two.yaml\n"),
            ("memory://schemas/two.yaml", "type: string\n"),
        ])),
        depth_config,
    );
    assert!(matches!(
        depth_session.compile(deep),
        Err(SchemaError::Retrieval(SchemaRetrieveError::LimitExceeded(error)))
            if error.limit == ResourceLimit::ResolutionDepth
                && error.observed == 2
                && error.maximum == 1
    ));
}

#[test]
fn retrieval_enforces_per_document_and_session_byte_ceilings() {
    let root_id = ResourceId::new("memory://schemas/root.yaml").unwrap();
    let mut input_config = config(&["memory"]);
    input_config.limits.max_input_bytes = Some(4);
    let input_session = SchemaSession::new(
        Arc::new(memory(&[("memory://schemas/root.yaml", "type: string\n")])),
        input_config,
    );
    assert!(matches!(
        input_session.load(&root_id),
        Err(SchemaRetrieveError::LimitExceeded(error))
            if error.limit == ResourceLimit::InputBytes
    ));

    let root = document(
        "memory://schemas/root.yaml",
        "allOf:\n  - {$ref: ./one.yaml}\n  - {$ref: ./two.yaml}\n",
    );
    let one = "type: string\n";
    let two = "minLength: 2\n";
    let mut fetched_config = config(&["memory"]);
    fetched_config.resolution.limits.max_fetched_bytes = Some(one.len() + two.len() - 1);
    let fetched_session = SchemaSession::new(
        Arc::new(memory(&[
            ("memory://schemas/one.yaml", one),
            ("memory://schemas/two.yaml", two),
        ])),
        fetched_config,
    );
    assert!(matches!(
        fetched_session.compile(root),
        Err(SchemaError::Retrieval(SchemaRetrieveError::LimitExceeded(error)))
            if error.limit == ResourceLimit::FetchedBytes
                && error.observed == one.len() + two.len()
    ));

    let oversized = document(
        "memory://schemas/root.yaml",
        "$ref: memory://schemas/large.yaml\n",
    );
    let mut oversized_config = config(&["memory"]);
    oversized_config.limits.max_input_bytes = Some(4);
    let oversized_session = SchemaSession::new(
        Arc::new(memory(&[("memory://schemas/large.yaml", "type: string\n")])),
        oversized_config,
    );
    assert!(matches!(
        oversized_session.compile(oversized),
        Err(SchemaError::Retrieval(SchemaRetrieveError::LimitExceeded(error)))
            if error.limit == ResourceLimit::InputBytes
    ));
}

struct SpyRetriever {
    calls: Arc<AtomicUsize>,
}

impl ResourceRetriever for SpyRetriever {
    fn retrieve(&self, request: &ResourceRequest) -> Result<RetrievedResource, RetrieveError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(RetrievedResource {
            id: request.id.clone(),
            bytes: b"type: string\n".to_vec(),
            provenance: ResourceProvenance::default(),
        })
    }
}

struct BlockingRetriever {
    entered: mpsc::Sender<ResourceRequest>,
    release: Mutex<mpsc::Receiver<()>>,
}

impl ResourceRetriever for BlockingRetriever {
    fn retrieve(&self, request: &ResourceRequest) -> Result<RetrievedResource, RetrieveError> {
        self.entered.send(request.clone()).unwrap();
        self.release.lock().unwrap().recv().unwrap();
        Ok(RetrievedResource {
            id: request.id.clone(),
            bytes: b"true".to_vec(),
            provenance: ResourceProvenance::default(),
        })
    }
}

#[test]
fn unrelated_schema_loads_can_retrieve_concurrently() {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let mut loader = config(&["memory"]);
    loader.limits.max_input_bytes = Some(6);
    loader.resolution.limits.max_fetched_bytes = Some(10);
    let session = Arc::new(SchemaSession::new(
        Arc::new(BlockingRetriever {
            entered: entered_tx,
            release: Mutex::new(release_rx),
        }),
        loader,
    ));
    let first_id = ResourceId::new("memory://schemas/first.yaml").unwrap();
    let second_id = ResourceId::new("memory://schemas/second.yaml").unwrap();

    let first_session = Arc::clone(&session);
    let first_load_id = first_id.clone();
    let first = std::thread::spawn(move || first_session.load(&first_load_id).is_ok());
    let first_request = entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(first_request.id, first_id);
    assert_eq!(first_request.max_bytes, Some(6));

    let second_session = Arc::clone(&session);
    let second_load_id = second_id.clone();
    let second = std::thread::spawn(move || second_session.load(&second_load_id).is_ok());
    let second_entered = entered_rx.recv_timeout(Duration::from_millis(500));

    release_tx.send(()).unwrap();
    release_tx.send(()).unwrap();
    assert!(first.join().unwrap());
    assert!(second.join().unwrap());
    let second_request = second_entered.unwrap();
    assert_eq!(second_request.id, second_id);
    assert_eq!(second_request.max_bytes, Some(4));
}

#[test]
fn same_schema_loads_share_one_in_flight_retrieval() {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let session = Arc::new(SchemaSession::new(
        Arc::new(BlockingRetriever {
            entered: entered_tx,
            release: Mutex::new(release_rx),
        }),
        config(&["memory"]),
    ));
    let id = ResourceId::new("memory://schemas/shared.yaml").unwrap();

    let first_session = Arc::clone(&session);
    let first_id = id.clone();
    let first = std::thread::spawn(move || first_session.load(&first_id).is_ok());
    assert_eq!(
        entered_rx.recv_timeout(Duration::from_secs(1)).unwrap().id,
        id
    );

    let second_session = Arc::clone(&session);
    let second = std::thread::spawn(move || second_session.load(&id).is_ok());
    let duplicate = entered_rx.recv_timeout(Duration::from_millis(100));

    release_tx.send(()).unwrap();
    if duplicate.is_ok() {
        release_tx.send(()).unwrap();
    }
    assert!(first.join().unwrap());
    assert!(second.join().unwrap());
    assert!(
        duplicate.is_err(),
        "same-key load reached the backend twice"
    );
}

struct ReentrantRetriever {
    session: OnceLock<Weak<SchemaSession>>,
}

impl ResourceRetriever for ReentrantRetriever {
    fn retrieve(&self, request: &ResourceRequest) -> Result<RetrievedResource, RetrieveError> {
        let session = self.session.get().unwrap().upgrade().unwrap();
        assert!(matches!(
            session.load(&request.id),
            Err(SchemaRetrieveError::ReentrantLoad { id }) if id == request.id
        ));
        Ok(resource(
            request.id.as_url().unwrap().as_str(),
            "type: string\n",
        ))
    }
}

#[test]
fn same_thread_same_key_reentrancy_is_rejected() {
    let retriever = Arc::new(ReentrantRetriever {
        session: OnceLock::new(),
    });
    let session = Arc::new(SchemaSession::new(retriever.clone(), config(&["memory"])));
    retriever.session.set(Arc::downgrade(&session)).unwrap();
    let (result_tx, result_rx) = mpsc::channel();

    std::thread::spawn(move || {
        let id = ResourceId::new("memory://schemas/reentrant.yaml").unwrap();
        result_tx.send(session.load(&id).is_ok()).unwrap();
    });

    assert!(
        result_rx
            .recv_timeout(Duration::from_millis(500))
            .expect("same-key reentrancy must return instead of deadlocking")
    );
}

// r[verify validate.policy]
#[test]
fn retrieval_denies_policy_violations_before_backend_calls_and_redacts_errors() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut policy = config(&["https"]);
    policy.resolution.refs.allow_hosts = vec!["example.com".into()];
    let session = SchemaSession::new(
        Arc::new(SpyRetriever {
            calls: Arc::clone(&calls),
        }),
        policy,
    );
    let mut rendered = String::new();
    for reference in [
        "ftp://example.com/schema.yaml",
        "https://blocked.example/schema.yaml",
        "https://user:password@example.com/schema.yaml?token=secret",
    ] {
        let root = document(
            "memory://schemas/root.yaml",
            &format!("$ref: {reference}\n"),
        );
        let error = match session.compile(root) {
            Ok(_) => panic!("policy violation is denied"),
            Err(error) => error,
        };
        rendered.push_str(&error.to_string());
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(!rendered.contains("user"));
    assert!(!rendered.contains("password"));
    assert!(!rendered.contains("token"));
    assert!(!rendered.contains("secret"));
}

#[test]
fn retrieval_reports_missing_external_resources() {
    let session = SchemaSession::new(Arc::new(MemoryRetriever::default()), config(&["memory"]));
    let root = document(
        "memory://schemas/root.yaml",
        "$ref: memory://schemas/missing.yaml\n",
    );

    assert!(matches!(
        session.compile(root),
        Err(SchemaError::Retrieval(SchemaRetrieveError::Backend {
            source: RetrieveError::NotFound(_),
            ..
        }))
    ));
}

#[test]
fn retrieval_bridges_the_existing_librebar_loopback_path() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let worker = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request);
        let body = b"type: string\n";
        let headers = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(headers.as_bytes()).unwrap();
        stream.write_all(body).unwrap();
    });
    let mut policy = config(&["http"]);
    policy.resolution.refs.allow_hosts = vec!["127.0.0.1".into()];
    policy.resolution.refs.timeout = Duration::from_secs(2);
    let retriever = LibrebarRetriever::new(&policy.resolution.refs, 1024).unwrap();
    let session = SchemaSession::new(Arc::new(retriever), policy);
    let root = document(
        "memory://schemas/root.yaml",
        &format!("$ref: http://{address}/string.yaml\n"),
    );
    let validator = session.compile(root).expect("loopback schema compiles");

    assert!(validator.validate(&Value::from("api")).valid);
    assert!(!validator.validate(&Value::Integer(42)).valid);
    worker.join().unwrap();
}

#[test]
fn retrieval_uses_file_capability_roots_for_relative_refs() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("root.yaml"),
        "properties:\n  port: {$ref: ./port.yaml}\n",
    )
    .unwrap();
    std::fs::write(
        directory.path().join("port.yaml"),
        "type: integer\nminimum: 1\n",
    )
    .unwrap();
    let mut config = config(&["file"]);
    config.resolution.files.root = Some(directory.path().to_path_buf());
    let retriever = FileRetriever::open(&config.resolution.files).unwrap();
    let session = SchemaSession::new(Arc::new(retriever), config);
    let root = session
        .load(&ResourceId::from_file(directory.path().join("root.yaml")))
        .expect("root file loads");
    let validator = session.compile(root).expect("relative file ref compiles");

    assert!(validator.validate(&value(json!({"port": 443}))).valid);
    assert!(!validator.validate(&value(json!({"port": 0}))).valid);
}

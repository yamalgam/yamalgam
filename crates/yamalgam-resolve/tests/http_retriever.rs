#![allow(missing_docs)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use yamalgam_core::{RefPolicy, ResolutionMode};
use yamalgam_resolve::{
    LibrebarRetriever, ResourceId, ResourceRequest, ResourceRetriever, ResourceValidator,
    RetrieveError,
};

#[derive(Clone)]
struct Reply {
    status: &'static str,
    headers: Vec<(&'static str, String)>,
    body: Vec<u8>,
    delay: Duration,
}

impl Reply {
    fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: "200 OK",
            headers: Vec::new(),
            body: body.into(),
            delay: Duration::ZERO,
        }
    }
}

struct SpyServer {
    address: SocketAddr,
    accepted: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl SpyServer {
    fn spawn(replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_accepted = Arc::clone(&accepted);
        let worker_stop = Arc::clone(&stop);
        let worker = thread::spawn(move || {
            while !worker_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        if worker_stop.load(Ordering::Acquire) {
                            break;
                        }
                        let index = worker_accepted.fetch_add(1, Ordering::AcqRel);
                        let Some(reply) = replies.get(index) else {
                            continue;
                        };
                        read_request(&mut stream);
                        thread::sleep(reply.delay);
                        let _ = stream.write_all(&render_response(reply));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            address,
            accepted,
            stop,
            worker: Some(worker),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.address)
    }

    fn accepted(&self) -> usize {
        self.accepted.load(Ordering::Acquire)
    }
}

impl Drop for SpyServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.address);
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
    }
}

fn read_request(stream: &mut TcpStream) {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(count) => request.extend_from_slice(&buffer[..count]),
        }
    }
}

fn render_response(reply: &Reply) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        reply.status,
        reply.body.len()
    )
    .into_bytes();
    for (name, value) in &reply.headers {
        response.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    response.extend_from_slice(b"\r\n");
    response.extend_from_slice(&reply.body);
    response
}

fn policy(schemes: &[&str], hosts: &[&str], timeout: Duration) -> RefPolicy {
    RefPolicy {
        mode: ResolutionMode::Automatic,
        allow_schemes: schemes.iter().map(|value| (*value).to_owned()).collect(),
        allow_hosts: hosts.iter().map(|value| (*value).to_owned()).collect(),
        timeout,
    }
}

fn request(url: &str, max_bytes: Option<usize>) -> ResourceRequest {
    ResourceRequest {
        id: ResourceId::new(url).unwrap(),
        max_bytes,
    }
}

fn local_retriever(timeout: Duration, maximum: usize) -> LibrebarRetriever {
    LibrebarRetriever::new(
        &policy(&["HTTP"], &["LOCALHOST", "127.0.0.1"], timeout),
        maximum,
    )
    .unwrap()
}

#[test]
fn retrieves_bytes_and_opaque_validators_across_repeated_requests() {
    let server = SpyServer::spawn(vec![
        Reply {
            status: "200 OK",
            headers: vec![
                ("ETag", "\"opaque-secret\"".into()),
                ("Last-Modified", "Wed, 21 Oct 2015 07:28:00 GMT".into()),
            ],
            body: b"answer: 42".to_vec(),
            delay: Duration::ZERO,
        },
        Reply::ok([0xff, 0xfe]),
    ]);
    let retriever = local_retriever(Duration::from_secs(2), 1024);
    let uppercase_url = format!("HTTP://LOCALHOST:{}/first.yaml", server.address.port());

    let first = retriever.retrieve(&request(&uppercase_url, None)).unwrap();
    assert_eq!(first.bytes, b"answer: 42");
    assert_eq!(first.provenance.validators.len(), 2);
    assert!(matches!(
        &first.provenance.validators[0],
        ResourceValidator::Etag(value) if value == "\"opaque-secret\""
    ));
    assert!(matches!(
        &first.provenance.validators[1],
        ResourceValidator::LastModified(value) if value == "Wed, 21 Oct 2015 07:28:00 GMT"
    ));

    let binary = retriever
        .retrieve(&request(&server.url("/binary.yaml"), None))
        .unwrap();
    assert_eq!(binary.bytes, [0xff, 0xfe]);
    assert_eq!(server.accepted(), 2);
    let rendered = format!("{retriever:?} {first:?}");
    assert!(!rendered.contains("opaque-secret"));
    assert!(!rendered.contains("answer: 42"));
}

#[test]
fn policy_rejections_happen_before_any_connection() {
    let server = SpyServer::spawn(Vec::new());
    let retriever = LibrebarRetriever::new(
        &policy(&["HTTP", "URN"], &["LOCALHOST"], Duration::from_secs(1)),
        1024,
    )
    .unwrap();
    let port = server.address.port();
    let cases = [
        format!("http://user:password@localhost:{port}/secret.yaml"),
        format!("http://127.0.0.1:{port}/host.yaml"),
        format!("http://notlocalhost:{port}/suffix.yaml"),
        format!("https://localhost:{port}/scheme.yaml"),
        "urn:yamalgam:no-host".to_owned(),
    ];

    for url in cases {
        assert!(matches!(
            retriever.retrieve(&request(&url, None)),
            Err(RetrieveError::PolicyDenied { .. })
        ));
    }
    thread::sleep(Duration::from_millis(25));
    assert_eq!(server.accepted(), 0);
}

#[test]
fn redirects_are_returned_without_following_location() {
    let destination = SpyServer::spawn(vec![Reply::ok("followed")]);
    let origin = SpyServer::spawn(vec![Reply {
        status: "302 Found",
        headers: vec![("Location", destination.url("/escaped.yaml"))],
        body: Vec::new(),
        delay: Duration::ZERO,
    }]);
    let retriever = local_retriever(Duration::from_secs(2), 1024);

    assert!(matches!(
        retriever.retrieve(&request(&origin.url("/redirect.yaml"), None)),
        Err(RetrieveError::HttpStatus { status: 302, .. })
    ));
    thread::sleep(Duration::from_millis(25));
    assert_eq!(origin.accepted(), 1);
    assert_eq!(destination.accepted(), 0);
}

#[test]
fn non_success_statuses_remain_typed() {
    let server = SpyServer::spawn(vec![
        Reply {
            status: "404 Not Found",
            headers: Vec::new(),
            body: b"missing".to_vec(),
            delay: Duration::ZERO,
        },
        Reply {
            status: "503 Service Unavailable",
            headers: Vec::new(),
            body: b"retry me".to_vec(),
            delay: Duration::ZERO,
        },
    ]);
    let retriever = local_retriever(Duration::from_secs(2), 1024);

    for (path, expected) in [("/missing", 404), ("/broken", 503)] {
        assert!(matches!(
            retriever.retrieve(&request(&server.url(path), None)),
            Err(RetrieveError::HttpStatus { status, .. }) if status == expected
        ));
    }
    assert_eq!(server.accepted(), 2, "HTTP retries must remain disabled");
}

#[test]
fn whole_operation_timeout_and_decoded_body_limits_are_typed() {
    let slow = SpyServer::spawn(vec![Reply {
        status: "200 OK",
        headers: Vec::new(),
        body: b"eventually".to_vec(),
        delay: Duration::from_millis(200),
    }]);
    let timeout = local_retriever(Duration::from_millis(25), 1024);
    assert!(matches!(
        timeout.retrieve(&request(&slow.url("/slow"), None)),
        Err(RetrieveError::Timeout(_))
    ));

    let large = SpyServer::spawn(vec![Reply::ok("12345"), Reply::ok("12345")]);
    let globally_bounded = local_retriever(Duration::from_secs(2), 4);
    assert!(matches!(
        globally_bounded.retrieve(&request(&large.url("/global"), None)),
        Err(RetrieveError::TooLarge {
            observed: 5,
            maximum: 4,
            ..
        })
    ));
    let request_bounded = local_retriever(Duration::from_secs(2), 1024);
    assert!(matches!(
        request_bounded.retrieve(&request(&large.url("/request"), Some(4))),
        Err(RetrieveError::TooLarge {
            observed: 5,
            maximum: 4,
            ..
        })
    ));

    let compressed = SpyServer::spawn(vec![Reply {
        status: "200 OK",
        headers: vec![("Content-Encoding", "gzip".into())],
        body: vec![
            0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x4b, 0x4c, 0xa4, 0x3d,
            0x00, 0x00, 0x64, 0x7a, 0x70, 0xaf, 0x64, 0x00, 0x00, 0x00,
        ],
        delay: Duration::ZERO,
    }]);
    let decoded_bound = local_retriever(Duration::from_secs(2), 30);
    assert!(matches!(
        decoded_bound.retrieve(&request(&compressed.url("/compressed"), None)),
        Err(RetrieveError::TooLarge {
            observed: 31,
            maximum: 30,
            ..
        })
    ));
}

#[test]
fn dropping_an_idle_worker_is_clean() {
    for _ in 0..4 {
        drop(local_retriever(Duration::from_secs(1), 1024));
    }
}

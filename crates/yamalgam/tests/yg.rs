//! End-to-end tests for yg's default query mode.

use assert_cmd::Command;
use predicates::prelude::*;
use std::io::{Read as _, Write as _};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[allow(deprecated)]
fn yg() -> Command {
    let mut command = Command::cargo_bin("yg").unwrap();
    command.env("YAMALGAM_LOG_DIR", std::env::temp_dir());
    command
}

#[allow(deprecated)]
fn yg_process() -> std::process::Command {
    let mut command = std::process::Command::new(assert_cmd::cargo::cargo_bin("yg"));
    command.env("YAMALGAM_LOG_DIR", std::env::temp_dir());
    command
}

#[derive(Clone)]
struct HttpReply {
    status: &'static str,
    headers: Vec<(&'static str, String)>,
    body: Vec<u8>,
}

impl HttpReply {
    fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: "200 OK",
            headers: Vec::new(),
            body: body.into(),
        }
    }
}

struct HttpFixture {
    address: SocketAddr,
    accepted: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl HttpFixture {
    fn spawn(replies: Vec<HttpReply>) -> Self {
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
                        stream.set_nonblocking(false).unwrap();
                        let index = worker_accepted.fetch_add(1, Ordering::AcqRel);
                        let Some(reply) = replies.get(index) else {
                            continue;
                        };
                        read_http_request(&mut stream);
                        let _ = stream.write_all(&render_http_response(reply));
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

impl Drop for HttpFixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.address);
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
    }
}

fn read_http_request(stream: &mut TcpStream) {
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

fn render_http_response(reply: &HttpReply) -> Vec<u8> {
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

#[test]
fn stdin_identity() {
    yg().arg(".")
        .write_stdin("a: 1\n")
        .assert()
        .success()
        .stdout("a: 1\n");
}

#[test]
fn default_filter_is_identity() {
    yg().write_stdin("a: 1\n")
        .assert()
        .success()
        .stdout("a: 1\n");
}

#[test]
fn files_process_as_unified_stream() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("1.yml");
    let second = dir.path().join("2.yml");
    std::fs::write(&first, "1\n").unwrap();
    std::fs::write(&second, "2\n").unwrap();

    yg().arg(".")
        .arg(&first)
        .arg(&second)
        .assert()
        .success()
        .stdout("1\n---\n2\n");
}

#[test]
fn null_input() {
    yg().args(["-n", "1 + 1"]).assert().success().stdout("2\n");
}

#[test]
fn null_input_does_not_wait_for_open_stdin() {
    let mut child = yg_process()
        .args(["-n", "1 + 1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            panic!("yg -n blocked while standard input remained open");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert_eq!(output.stdout, b"2\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn null_input_keeps_real_inputs_available() {
    yg().args(["-n", "inputs"])
        .write_stdin("1\n")
        .assert()
        .success()
        .stdout("1\n");
}

#[test]
fn slurp_collects_all_documents() {
    yg().args(["-s", "length"])
        .write_stdin("---\n1\n---\n2\n")
        .assert()
        .success()
        .stdout("2\n");
}

#[test]
fn raw_input_reads_lines() {
    yg().args(["-R", "."])
        .write_stdin("plain text\n")
        .assert()
        .success()
        .stdout("plain text\n");
}

#[test]
fn raw_slurp_is_one_string() {
    yg().args(["-R", "-s", "length"])
        .write_stdin("a\nb\n")
        .assert()
        .success()
        .stdout("4\n");
}

#[test]
fn input_context_tracks_files_and_documents() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.yml");
    std::fs::write(&input, "---\n1\n---\n2\n").unwrap();
    let expected = format!(
        "[{}, 0, 0]\n---\n[{}, 1, 0]\n",
        input.display(),
        input.display()
    );

    yg().arg("-c")
        .arg("[filename, document_index, file_index]")
        .arg(input)
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn input_lookahead_does_not_replace_current_document_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first.yml");
    let second = dir.path().join("second.yml");
    std::fs::write(&first, "1\n").unwrap();
    std::fs::write(&second, "2\n").unwrap();
    let expected = format!("[{}, 2, {}]\n", first.display(), first.display());

    yg().arg("-c")
        .arg("[filename, input, filename]")
        .arg(first)
        .arg(second)
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn filter_from_file() {
    let dir = tempfile::tempdir().unwrap();
    let program = dir.path().join("prog.jq");
    std::fs::write(&program, ".a\n").unwrap();

    yg().arg("-f")
        .arg(&program)
        .write_stdin("a: 5\n")
        .assert()
        .success()
        .stdout("5\n");
}

#[test]
fn arguments_bind_strings_and_json_without_filter_interpolation() {
    yg().args(["-n", "-r", "--arg", "name", "Ada Lovelace", "$name"])
        .assert()
        .success()
        .stdout("Ada Lovelace\n");

    yg().args([
        "-n",
        "--argjson",
        "payload",
        r#"{"count": 2}"#,
        "$payload.count + 1",
    ])
    .assert()
    .success()
    .stdout("3\n");
}

#[test]
fn no_env_disables_environment_filter_access() {
    const SECRET: &str = "m10-no-env-sentinel";

    yg().env("YG_AUDIT_SECRET", SECRET)
        .args(["-n", "-r", "env.YG_AUDIT_SECRET"])
        .assert()
        .success()
        .stdout(format!("{SECRET}\n"));

    yg().env("YG_AUDIT_SECRET", SECRET)
        .args(["--no-env", "-n", "-r", "env.YG_AUDIT_SECRET"])
        .assert()
        .code(3)
        .stdout("")
        .stderr(predicate::str::contains("env").and(predicate::str::contains(SECRET).not()));
}

#[test]
fn no_env_is_documented_as_a_capability_control() {
    yg().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--no-env"))
        .stdout(predicate::str::contains(
            "independently of input resource limits",
        ));
}

#[test]
fn schema_flag_changes_resolution() {
    yg().args(["--schema", "1.1", ".a | type"])
        .write_stdin("a: yes\n")
        .assert()
        .success()
        .stdout("boolean\n")
        .stderr(predicate::str::contains("YAML 1.1"));
    yg().args(["--schema", "failsafe", ".a | type"])
        .write_stdin("a: 5\n")
        .assert()
        .success()
        .stdout("string\n");
}

#[test]
fn json_output_explodes_aliases_and_is_json() {
    yg().args(["-j", "."])
        .write_stdin("a: &x [1]\nb: *x\n")
        .assert()
        .success()
        .stdout("{\n  \"a\": [\n    1\n  ],\n  \"b\": [\n    1\n  ]\n}\n");
}

#[test]
fn explicit_global_json_format_selects_json() {
    yg().args(["--format", "json", "."])
        .write_stdin("a: 1\n")
        .assert()
        .success()
        .stdout("{\n  \"a\": 1\n}\n");
}

#[test]
fn automatic_format_remains_yaml_when_piped() {
    yg().arg(".")
        .write_stdin("a: 1\n")
        .assert()
        .success()
        .stdout("a: 1\n");
}

#[test]
fn raw_output_unquotes_strings() {
    yg().args(["-r", ".a"])
        .write_stdin("a: 'hello: world'\n")
        .assert()
        .success()
        .stdout("hello: world\n");
}

#[test]
fn compact_output_is_single_line() {
    yg().args(["-c", "."])
        .write_stdin("a:\n  - 1\n")
        .assert()
        .success()
        .stdout("{a: [1]}\n");
}

#[test]
fn multidoc_separators_and_suppression() {
    yg().arg(".[]")
        .write_stdin("[1, 2]\n")
        .assert()
        .success()
        .stdout("1\n---\n2\n");
    yg().args(["-N", ".[]"])
        .write_stdin("[1, 2]\n")
        .assert()
        .success()
        .stdout("1\n2\n");
}

#[test]
fn exit_codes() {
    yg().arg(".").write_stdin("1\n").assert().code(0);
    yg().args(["-e", ".a"])
        .write_stdin("a: false\n")
        .assert()
        .code(1);
    yg().args(["-e", ".missing"])
        .write_stdin("a: 1\n")
        .assert()
        .code(1);
    yg().arg(".a |").write_stdin("1\n").assert().code(2);
    yg().arg("nosuchfilter").write_stdin("1\n").assert().code(3);
    yg().args(["-e", "empty"])
        .write_stdin("1\n")
        .assert()
        .code(4);
    yg().arg(".a + true").write_stdin("a: 1\n").assert().code(5);
    yg().arg(".").write_stdin("a: [unclosed\n").assert().code(5);
}

#[test]
fn halt_uses_posix_exit_codes_without_diagnostics() {
    yg().args(["-n", "halt"])
        .assert()
        .code(0)
        .stdout("")
        .stderr("");
    yg().args(["-n", "halt(-1)"])
        .assert()
        .code(255)
        .stdout("")
        .stderr("");
    yg().args(["-n", "halt(256)"])
        .assert()
        .code(255)
        .stdout("")
        .stderr("");
}

#[test]
fn missing_input_uses_usage_exit_code() {
    yg().arg(".")
        .arg("this-file-does-not-exist.yml")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to read"));
}

#[test]
fn runtime_error_names_its_input_file() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("bad-value.yml");
    std::fs::write(&input, "a: text\n").unwrap();

    yg().arg(".a.b")
        .arg(&input)
        .assert()
        .code(5)
        .stderr(predicate::str::contains(format!(
            "{} document 1:",
            input.display()
        )));
}

#[test]
fn closed_stdout_is_a_clean_success() {
    let mut child = yg_process()
        .args(["-n", "range(200000)"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut prefix = [0; 2];
    stdout.read_exact(&mut prefix).unwrap();
    drop(stdout);
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
}

#[test]
fn strict_mode_bounds_filter_allocations() {
    yg().args(["--strict", "-n", ".[100000] = 1 | length"])
        .assert()
        .code(5)
        .stderr(predicate::str::contains("collection size"));
    yg().args(["--strict", "-n", r#""a" * 2000000 | length"#])
        .assert()
        .code(5)
        .stderr(predicate::str::contains("scalar size"));
}

#[test]
fn strict_mode_bounds_constructed_output_depth() {
    yg().args(["--strict", "-n", "reduce range(100) as $i (null; [.])"])
        .assert()
        .code(5)
        .stderr(predicate::str::contains("depth"));
}

#[test]
fn strict_mode_rejects_discarded_over_depth_values() {
    yg().timeout(Duration::from_secs(10))
        .args(["--strict", "-n", "reduce range(100) as $i (null; [.]) | 1"])
        .assert()
        .code(5)
        .stdout("")
        .stderr(predicate::str::contains("depth"));
}

#[test]
fn fromjson_uses_active_limits_and_rejects_yaml_extensions() {
    let nested_json = format!("{}null{}", "[".repeat(70), "]".repeat(70));
    yg().args(["--strict", "fromjson"])
        .write_stdin(format!("{nested_json:?}\n"))
        .assert()
        .code(5)
        .stderr(predicate::str::contains("depth"));

    yg().args(["--strict", "-n", r#""a: &x [1]\nb: *x\n" | fromjson"#])
        .assert()
        .code(5)
        .stderr(predicate::str::contains("fromjson"));
}

#[test]
fn librebar_color_flags_are_canonical() {
    let output = yg()
        .args(["--color", "always", "."])
        .write_stdin("a: 1\n")
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("\x1b["),
        "expected ANSI escapes with --color always"
    );

    yg().args(["--color", "never", "."])
        .write_stdin("a: 1\n")
        .assert()
        .success()
        .stdout("a: 1\n");
}

#[test]
fn help_explains_color_and_chdir_collision() {
    yg().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Use --color auto|always|never. -C is --chdir, not a color flag.",
        ));
}

#[test]
fn help_explains_subcommand_filter_shadowing() {
    yg().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Leading doctor and info select subcommands.",
        ))
        .stdout(predicate::str::contains("yg -- FILTER"));
}

#[test]
fn argument_terminator_forces_subcommand_name_to_filter() {
    yg().args(["--", "doctor"])
        .write_stdin("null\n")
        .assert()
        .code(3)
        .stdout("")
        .stderr(predicate::str::contains("undefined filter doctor"));
}

#[test]
fn in_place_edits_file_and_preserves_sibling_style() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("config.yml");
    std::fs::write(&input, "a: 1\nb: \"keep\"\n").unwrap();

    yg().args(["-i", ".a = 2"])
        .arg(&input)
        .assert()
        .success()
        .stdout("");
    assert_eq!(
        std::fs::read_to_string(input).unwrap(),
        "a: 2\nb: \"keep\"\n"
    );
}

#[test]
fn in_place_refuses_empty_output_without_touching_file() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("config.yml");
    let original = "enabled: false\nname: production\n";
    std::fs::write(&input, original).unwrap();

    yg().args(["-i", "select(.enabled)"])
        .arg(&input)
        .assert()
        .code(5)
        .stderr(predicate::str::contains("produced no output"));
    assert_eq!(std::fs::read_to_string(input).unwrap(), original);
}

#[test]
fn in_place_stages_all_files_before_replacing_any() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first.yml");
    let second = dir.path().join("second.yml");
    std::fs::write(&first, "n: 1\n").unwrap();
    std::fs::write(&second, "n: [bad\n").unwrap();

    yg().args(["-i", ".n += 1"])
        .arg(&first)
        .arg(&second)
        .assert()
        .code(5);
    assert_eq!(std::fs::read_to_string(first).unwrap(), "n: 1\n");
}

#[test]
fn in_place_help_states_conflicts() {
    yg().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "incompatible with -n, -s, and JSON output",
        ));
}

#[test]
fn in_place_rejects_slurp_null_input_and_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("config.yml");
    std::fs::write(&input, "a: 1\n").unwrap();

    for arguments in [
        vec!["-i", "-s", "."],
        vec!["-i", "-n", "."],
        vec!["-i", "--json", "."],
    ] {
        yg().args(arguments).arg(&input).assert().code(2);
    }
}

#[test]
fn in_place_accepts_filter_from_file() {
    let dir = tempfile::tempdir().unwrap();
    let program = dir.path().join("program.jq");
    let input = dir.path().join("config.yml");
    std::fs::write(&program, ".a = 2\n").unwrap();
    std::fs::write(&input, "a: 1\n").unwrap();

    yg().arg("-i")
        .arg("-f")
        .arg(&program)
        .arg(&input)
        .assert()
        .success();
    assert_eq!(std::fs::read_to_string(input).unwrap(), "a: 2\n");
}

#[cfg(unix)]
#[test]
fn in_place_refuses_to_replace_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target.yml");
    let link = dir.path().join("link.yml");
    std::fs::write(&target, "a: 1\n").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();

    yg().args(["-i", ".a = 2"])
        .arg(&link)
        .assert()
        .code(5)
        .stderr(predicate::str::contains("symbolic link"));
    assert!(
        std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(std::fs::read_to_string(target).unwrap(), "a: 1\n");
}

#[test]
fn in_place_edits_multi_document_file() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("multi.yml");
    std::fs::write(&input, "a: 1\n---\na: 2\n").unwrap();

    yg().args(["-i", ".a |= . + 10"])
        .arg(&input)
        .assert()
        .success()
        .stdout("");
    assert_eq!(
        std::fs::read_to_string(input).unwrap(),
        "a: 11\n---\na: 12\n"
    );
}

#[test]
fn in_place_requires_a_file() {
    yg().args(["-i", "."])
        .write_stdin("a: 1\n")
        .assert()
        .code(2);
}

#[cfg(unix)]
#[test]
fn in_place_preserves_permissions() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("private.yml");
    std::fs::write(&input, "a: 1\n").unwrap();
    std::fs::set_permissions(&input, std::fs::Permissions::from_mode(0o600)).unwrap();

    yg().args(["-i", "."]).arg(&input).assert().success();
    assert_eq!(
        std::fs::metadata(input).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn in_place_failure_leaves_current_file_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.yml");
    std::fs::write(&input, "a: 1\n").unwrap();

    yg().args(["-i", ".a + true"]).arg(&input).assert().code(5);
    assert_eq!(std::fs::read_to_string(input).unwrap(), "a: 1\n");
}

#[test]
fn resolution_flags_are_documented() {
    yg().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--allow-includes"))
        .stdout(predicate::str::contains("--resolve-includes"))
        .stdout(predicate::str::contains("--allow-refs <ALLOW_REFS>"))
        .stdout(predicate::str::contains("--resolve-refs <RESOLVE_REFS>"))
        .stdout(predicate::str::contains(
            "--allow-ref-host <ALLOW_REF_HOSTS>",
        ))
        .stdout(predicate::str::contains("--resolution-root <DIR>"))
        .stdout(predicate::str::contains("--base-uri <URI>"));
}

#[test]
fn resolution_modes_conflict_and_ref_schemes_are_validated() {
    yg().args(["--allow-includes", "--resolve-includes", "."])
        .assert()
        .code(2);
    yg().args(["--allow-refs=file", "--resolve-refs=file", "."])
        .assert()
        .code(2);
    yg().args(["--allow-refs=file,file", "."])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("duplicate reference scheme file"));
    yg().args(["--allow-refs=ftp", "."])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid value 'ftp'"));
}

#[test]
fn network_resolution_requires_matching_schemes_and_hosts() {
    yg().args(["--allow-refs=http", "."])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("requires --allow-ref-host"));
    yg().args(["--allow-refs=file", "--allow-ref-host=example.test", "."])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "--allow-ref-host requires http or https",
        ));

    for host in ["https://example.test", "example.test:443", "bad host"] {
        yg().arg("--allow-refs=https")
            .arg(format!("--allow-ref-host={host}"))
            .arg(".")
            .assert()
            .code(2)
            .stderr(predicate::str::contains("invalid --allow-ref-host"));
    }
}

#[test]
fn base_uri_and_resolution_root_are_validated_before_input() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let outside_base = outside.path().join("stdin.yaml");

    yg().arg("--allow-refs=file")
        .arg(format!("--resolution-root={}", root.path().display()))
        .arg(format!("--base-uri={}", outside_base.display()))
        .arg(".")
        .write_stdin("null\n")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("outside --resolution-root"));
    yg().args(["--allow-refs=file", "--base-uri=relative.yaml", "."])
        .write_stdin("null\n")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("absolute URI"));
    yg().args([
        "--allow-refs=https",
        "--allow-ref-host=example.test",
        "--base-uri=https://example.test/root.yaml#node",
        ".",
    ])
    .write_stdin("null\n")
    .assert()
    .code(2)
    .stderr(predicate::str::contains("must not contain a fragment"));
}

#[test]
fn resolution_is_disabled_by_default_and_file_refs_are_opt_in() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.yaml");
    let shared = dir.path().join("shared.yaml");
    std::fs::write(&root, "$ref: ./shared.yaml\n").unwrap();
    std::fs::write(&shared, "answer: 42\n").unwrap();

    yg().arg(".")
        .arg(&root)
        .assert()
        .success()
        .stdout("$ref: ./shared.yaml\n");
    yg().args(["--resolve-refs=file", "."])
        .arg(&root)
        .assert()
        .success()
        .stdout("answer: 42\n");
    yg().args(["--allow-refs=file", "ref"])
        .arg(&root)
        .assert()
        .success()
        .stdout("answer: 42\n");
}

#[test]
fn file_includes_use_the_input_directory_or_explicit_root() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("nested");
    std::fs::create_dir(&nested).unwrap();
    let root = nested.join("root.yaml");
    std::fs::write(&root, "value: !include shared.yaml\n").unwrap();
    std::fs::write(nested.join("shared.yaml"), "included: true\n").unwrap();

    yg().args(["--resolve-includes", ".value"])
        .arg(&root)
        .assert()
        .success()
        .stdout("included: true\n");
    yg().arg("--allow-includes")
        .arg(".value | include")
        .arg(&root)
        .assert()
        .success()
        .stdout("included: true\n");

    std::fs::write(&root, "$ref: ../outside.yaml\n").unwrap();
    std::fs::write(dir.path().join("outside.yaml"), "escaped: true\n").unwrap();
    yg().arg("--resolve-refs=file")
        .arg(format!("--resolution-root={}", nested.display()))
        .arg(".")
        .arg(&root)
        .assert()
        .code(5)
        .stderr(predicate::str::contains("escapes the configured root"));
}

#[test]
fn resolution_tracks_per_file_bases_across_multiple_roots_and_slurp() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    for (directory, value) in [(&first, "first"), (&second, "second")] {
        std::fs::write(directory.path().join("root.yaml"), "$ref: ./shared.yaml\n").unwrap();
        std::fs::write(
            directory.path().join("shared.yaml"),
            format!("value: {value}\n"),
        )
        .unwrap();
    }
    let first_root = first.path().join("root.yaml");
    let second_root = second.path().join("root.yaml");

    yg().args(["--resolve-refs=file", ".value"])
        .arg(&first_root)
        .arg(&second_root)
        .assert()
        .success()
        .stdout("first\n---\nsecond\n");
    yg().args(["--allow-refs=file", "-s", "map(ref | .value)"])
        .arg(&first_root)
        .arg(&second_root)
        .assert()
        .success()
        .stdout("- first\n- second\n");
}

#[test]
fn stdin_resolution_uses_explicit_file_bases_and_roots() {
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path().join("stdin.yaml");
    std::fs::write(directory.path().join("shared.yaml"), "answer: 42\n").unwrap();

    yg().arg("--resolve-refs=file")
        .arg(format!("--resolution-root={}", directory.path().display()))
        .arg(format!("--base-uri={}", base.display()))
        .arg(".")
        .write_stdin("$ref: ./shared.yaml\n")
        .assert()
        .success()
        .stdout("answer: 42\n");
    yg().arg("--resolve-includes")
        .arg(format!("--resolution-root={}", directory.path().display()))
        .arg(".value")
        .write_stdin("value: !include shared.yaml\n")
        .assert()
        .success()
        .stdout("answer: 42\n");

    yg().args(["--allow-refs=file", "."])
        .write_stdin("$ref: ./shared.yaml\n")
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "relative filesystem access from standard input",
        ));

    yg().args(["--resolve-refs=http", "--allow-ref-host=example.test", "."])
        .write_stdin("$ref: ./relative.yaml\n")
        .assert()
        .code(5)
        .stderr(predicate::str::contains("requires a base"));

    yg().args([
        "--resolve-refs=http",
        "--allow-ref-host=example.test",
        ".copy.answer",
    ])
    .write_stdin("root: {answer: 42}\ncopy: {$ref: '#/root'}\n")
    .assert()
    .success()
    .stdout("42\n");
}

#[test]
fn raw_and_null_inputs_can_resolve_constructed_refs_from_the_active_base() {
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path().join("stdin.yaml");
    std::fs::write(directory.path().join("shared.yaml"), "answer: 42\n").unwrap();
    let root_arg = format!("--resolution-root={}", directory.path().display());
    let base_arg = format!("--base-uri={}", base.display());

    yg().args([
        "--allow-refs=file",
        &root_arg,
        &base_arg,
        "-R",
        r#"{"$ref": .} | ref"#,
    ])
    .write_stdin("./shared.yaml\n")
    .assert()
    .success()
    .stdout("answer: 42\n");
    yg().args([
        "--allow-refs=file",
        &root_arg,
        &base_arg,
        "-n",
        r#"{"$ref": "./shared.yaml"} | ref"#,
    ])
    .assert()
    .success()
    .stdout("answer: 42\n");
}

#[test]
fn resolution_works_with_json_output_and_atomic_in_place_edits() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("root.yaml");
    std::fs::write(&root, "value: !include shared.yaml\n").unwrap();
    std::fs::write(directory.path().join("shared.yaml"), "answer: 42\n").unwrap();

    yg().args(["--resolve-includes", "-j", ".value"])
        .arg(&root)
        .assert()
        .success()
        .stdout("{\n  \"answer\": 42\n}\n");
    yg().args(["--resolve-includes", "-i", "."])
        .arg(&root)
        .assert()
        .success();
    assert_eq!(
        std::fs::read_to_string(root).unwrap(),
        "value:\n  answer: 42\n"
    );
}

#[test]
fn resolution_failures_keep_input_context_and_exit_five() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("root.yaml");
    std::fs::write(&root, "$ref: ./missing.yaml\n").unwrap();

    yg().args(["--resolve-refs=file", "."])
        .arg(&root)
        .assert()
        .code(5)
        .stderr(predicate::str::contains(format!(
            "{} document 1:",
            root.display()
        )))
        .stderr(predicate::str::contains("resource not found"));
    yg().args(["--allow-refs=file", "ref"])
        .arg(&root)
        .assert()
        .code(5)
        .stderr(predicate::str::contains(format!(
            "{} document 1:",
            root.display()
        )))
        .stderr(predicate::str::contains("resource not found"));

    std::fs::write(&root, "---\nvalue: ok\n---\n$ref: ./missing.yaml\n").unwrap();
    yg().args(["--resolve-refs=file", "."])
        .arg(&root)
        .assert()
        .code(5)
        .stderr(predicate::str::contains(format!(
            "{} document 2:",
            root.display()
        )));

    std::fs::write(&root, "value: [unterminated\n").unwrap();
    yg().args(["--resolve-refs=file", "."])
        .arg(&root)
        .assert()
        .code(5)
        .stderr(predicate::str::contains(root.display().to_string()));
}

#[test]
fn explicit_resolution_root_must_contain_every_initial_file() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let input = outside.path().join("input.yaml");
    std::fs::write(&input, "$ref: ./shared.yaml\n").unwrap();

    yg().arg("--resolve-refs=file")
        .arg(format!("--resolution-root={}", root.path().display()))
        .arg(".")
        .arg(&input)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("outside --resolution-root"));
}

#[test]
fn url_shaped_top_level_inputs_remain_ordinary_file_paths() {
    yg().args([
        "--resolve-refs=https",
        "--allow-ref-host=example.test",
        ".",
        "https://example.test/root.yaml",
    ])
    .assert()
    .code(2)
    .stderr(predicate::str::contains("failed to read"));
}

#[test]
fn http_refs_resolve_whole_documents_and_nested_relative_urls() {
    let server = HttpFixture::spawn(vec![
        HttpReply::ok("$ref: ./child.yaml\n"),
        HttpReply::ok("answer: 42\n"),
    ]);
    let input = format!("$ref: {}\n", server.url("/root.yaml"));

    yg().args(["--resolve-refs=http", "--allow-ref-host=127.0.0.1", "."])
        .write_stdin(input)
        .assert()
        .success()
        .stdout("answer: 42\n");
    assert_eq!(server.accepted(), 2);
}

#[test]
fn explicit_http_ref_filter_uses_the_same_allowlisted_transport() {
    let server = HttpFixture::spawn(vec![HttpReply::ok("answer: 42\n")]);
    let input = format!("$ref: {}\n", server.url("/shared.yaml"));

    yg().args(["--allow-refs=http", "--allow-ref-host=127.0.0.1", "ref"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout("answer: 42\n");
    assert_eq!(server.accepted(), 1);
}

#[test]
fn http_host_denial_happens_before_a_connection() {
    let server = HttpFixture::spawn(Vec::new());
    let input = format!("$ref: {}\n", server.url("/denied.yaml"));

    yg().args(["--resolve-refs=http", "--allow-ref-host=localhost", "."])
        .write_stdin(input)
        .assert()
        .code(5)
        .stderr(predicate::str::contains("denied by policy"));
    thread::sleep(Duration::from_millis(20));
    assert_eq!(server.accepted(), 0);
}

#[test]
fn http_redirects_and_non_success_statuses_are_hard_failures() {
    let destination = HttpFixture::spawn(vec![HttpReply::ok("escaped: true\n")]);
    let origin = HttpFixture::spawn(vec![HttpReply {
        status: "302 Found",
        headers: vec![("Location", destination.url("/escaped.yaml"))],
        body: Vec::new(),
    }]);
    let input = format!("$ref: {}\n", origin.url("/redirect.yaml"));
    yg().args(["--resolve-refs=http", "--allow-ref-host=127.0.0.1", "."])
        .write_stdin(input)
        .assert()
        .code(5)
        .stderr(predicate::str::contains("HTTP status 302"));
    thread::sleep(Duration::from_millis(20));
    assert_eq!(origin.accepted(), 1);
    assert_eq!(destination.accepted(), 0);

    let missing = HttpFixture::spawn(vec![HttpReply {
        status: "404 Not Found",
        headers: Vec::new(),
        body: b"not YAML and not disclosed".to_vec(),
    }]);
    let input = format!("$ref: {}\n", missing.url("/missing.yaml"));
    yg().args(["--resolve-refs=http", "--allow-ref-host=127.0.0.1", "."])
        .write_stdin(input)
        .assert()
        .code(5)
        .stderr(predicate::str::contains("HTTP status 404"))
        .stderr(predicate::str::contains("not disclosed").not());
}

#[test]
fn http_payloads_must_be_bounded_utf8_single_document_yaml() {
    let invalid = HttpFixture::spawn(vec![
        HttpReply::ok([0xff, 0xfe]),
        HttpReply::ok("value: [unterminated\n"),
        HttpReply::ok("---\none\n---\ntwo\n"),
    ]);
    for (path, diagnostic) in [
        ("/binary.yaml", "not valid UTF-8"),
        ("/invalid.yaml", "invalid YAML"),
        ("/multiple.yaml", "multiple documents"),
    ] {
        let input = format!("$ref: {}\n", invalid.url(path));
        yg().args(["--resolve-refs=http", "--allow-ref-host=127.0.0.1", "."])
            .write_stdin(input)
            .assert()
            .code(5)
            .stderr(predicate::str::contains(diagnostic));
    }

    let oversized = HttpFixture::spawn(vec![HttpReply::ok(vec![b'a'; 10 * 1024 * 1024 + 1])]);
    let input = format!("$ref: {}\n", oversized.url("/oversized.yaml"));
    yg().args([
        "--strict",
        "--resolve-refs=http",
        "--allow-ref-host=127.0.0.1",
        ".",
    ])
    .write_stdin(input)
    .assert()
    .code(5)
    .stderr(predicate::str::contains("maximum of 10485760"));
}

#[test]
fn includes_never_fetch_http_even_when_http_refs_are_enabled() {
    let server = HttpFixture::spawn(Vec::new());
    let root = tempfile::tempdir().unwrap();
    let input = format!("value: !include {}\n", server.url("/forbidden.yaml"));

    yg().arg("--resolve-includes")
        .arg("--resolve-refs=http")
        .arg("--allow-ref-host=127.0.0.1")
        .arg(format!("--resolution-root={}", root.path().display()))
        .arg(".")
        .write_stdin(input)
        .assert()
        .code(5)
        .stderr(predicate::str::contains("include"));
    thread::sleep(Duration::from_millis(20));
    assert_eq!(server.accepted(), 0);
}

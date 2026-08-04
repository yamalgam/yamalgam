//! End-to-end tests for yg's default query mode.

use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Read as _;
use std::process::Stdio;
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

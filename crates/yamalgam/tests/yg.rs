//! End-to-end tests for yg's default query mode.

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn yg() -> Command {
    Command::cargo_bin("yg").unwrap()
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

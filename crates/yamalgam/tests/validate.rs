//! End-to-end contracts for `yg validate` and its `lint` alias.

use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

#[allow(deprecated)]
fn yg(directory: &TempDir) -> Command {
    let mut command = Command::cargo_bin("yg").unwrap();
    command.env("YAMALGAM_LOG_DIR", std::env::temp_dir());
    command.arg("-C").arg(directory.path());
    command
}

fn write(directory: &TempDir, path: &str, contents: &str) {
    let path = directory.path().join(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

fn json_output(assertion: &assert_cmd::assert::Assert) -> Value {
    serde_json::from_slice(&assertion.get_output().stdout).expect("validation report is JSON")
}

#[test]
fn explicit_schema_returns_zero_for_valid_and_one_for_findings() {
    let directory = TempDir::new().unwrap();
    write(
        &directory,
        "schema.yaml",
        "type: object\nrequired: [name]\nproperties:\n  name: {type: string}\n",
    );
    write(&directory, "valid.yaml", "name: api\n");
    write(&directory, "invalid.yaml", "name: 7\n");

    let valid = yg(&directory)
        .args([
            "--format",
            "json",
            "validate",
            "--schema",
            "schema.yaml",
            "valid.yaml",
        ])
        .assert()
        .success();
    let report = json_output(&valid);
    assert_eq!(report[0]["filename"], "valid.yaml");
    assert_eq!(report[0]["document"], 0);
    assert_eq!(report[0]["valid"], true);
    assert_eq!(report[0]["diagnostics"], serde_json::json!([]));

    let invalid = yg(&directory)
        .args([
            "--format",
            "json",
            "validate",
            "--schema",
            "schema.yaml",
            "invalid.yaml",
        ])
        .assert()
        .code(1);
    let report = json_output(&invalid);
    assert_eq!(report[0]["valid"], false);
    assert_eq!(report[0]["diagnostics"][0]["code"], "schema/type");
    assert_eq!(report[0]["diagnostics"][0]["instance_path"], "/name");
}

#[test]
fn pragma_selection_preserves_file_and_document_order() {
    let directory = TempDir::new().unwrap();
    write(
        &directory,
        "schema.yaml",
        "type: object\nrequired: [name]\n",
    );
    write(
        &directory,
        "one.yaml",
        "# yg-schema-validate: ./schema.yaml\nname: first\n---\n# yg-schema-validate: ./schema.yaml\nport: 80\n",
    );
    write(
        &directory,
        "two.yaml",
        "# yg-schema-validate: ./schema.yaml\nname: second\n",
    );

    let assertion = yg(&directory)
        .args(["--format", "json", "validate", "one.yaml", "two.yaml"])
        .assert()
        .code(1);
    let report = json_output(&assertion);
    assert_eq!(report.as_array().unwrap().len(), 3);
    assert_eq!(report[0]["filename"], "one.yaml");
    assert_eq!(report[0]["document"], 0);
    assert_eq!(report[0]["valid"], true);
    assert_eq!(report[1]["filename"], "one.yaml");
    assert_eq!(report[1]["document"], 1);
    assert_eq!(report[1]["valid"], false);
    assert_eq!(report[2]["filename"], "two.yaml");
    assert_eq!(report[2]["document"], 0);
    assert_eq!(report[2]["valid"], true);
}

#[test]
fn missing_malformed_and_relative_stdin_schemas_are_runtime_failures() {
    let directory = TempDir::new().unwrap();
    write(&directory, "input.yaml", "name: api\n");
    write(&directory, "malformed.yaml", "type: 7\n");

    yg(&directory)
        .args(["validate", "--schema", "missing.yaml", "input.yaml"])
        .assert()
        .code(5)
        .stderr(predicate::str::contains("schema resource retrieval failed"));

    yg(&directory)
        .args(["validate", "--schema", "malformed.yaml", "input.yaml"])
        .assert()
        .code(5)
        .stderr(predicate::str::contains("invalid schema"));

    yg(&directory)
        .args(["validate", "--schema", "schema.yaml"])
        .write_stdin("name: api\n")
        .assert()
        .code(5)
        .stderr(predicate::str::contains("requires a containing resource"));
}

#[test]
fn yaml_output_and_lint_alias_share_the_validation_command() {
    let directory = TempDir::new().unwrap();
    write(&directory, "schema.yaml", "type: object\n");
    write(&directory, "input.yaml", "name: api\n");

    yg(&directory)
        .args([
            "--format",
            "text",
            "lint",
            "--schema",
            "schema.yaml",
            "input.yaml",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("filename: input.yaml"))
        .stdout(predicate::str::contains("document: 0"))
        .stdout(predicate::str::contains("valid: true"))
        .stdout(predicate::str::contains("diagnostics: []"));
}

#[test]
fn configured_associations_select_local_schemas() {
    let directory = TempDir::new().unwrap();
    write(
        &directory,
        ".yamalgam.toml",
        "[[schemas]]\nuri = './schema.yaml'\nfile_match = ['service.yaml']\n",
    );
    write(&directory, "schema.yaml", "required: [name]\n");
    write(&directory, "service.yaml", "name: api\n");

    yg(&directory)
        .args(["validate", "service.yaml"])
        .assert()
        .success();
}

#[test]
fn schemastore_is_opt_in_and_can_use_a_local_catalog() {
    let directory = TempDir::new().unwrap();
    let schema_url = format!(
        "file://{}",
        directory.path().join("schema.yaml").to_string_lossy()
    );
    let catalog_url = format!(
        "file://{}",
        directory.path().join("catalog.json").to_string_lossy()
    );
    write(
        &directory,
        ".yamalgam.toml",
        &format!("[schema_store]\nenabled = false\nurl = {catalog_url:?}\n"),
    );
    write(
        &directory,
        "catalog.json",
        &serde_json::json!({
            "schemas": [{
                "name": "local service",
                "fileMatch": ["store.yaml"],
                "url": schema_url
            }]
        })
        .to_string(),
    );
    write(&directory, "schema.yaml", "required: [name]\n");
    write(&directory, "store.yaml", "name: api\n");

    yg(&directory)
        .args(["validate", "store.yaml"])
        .assert()
        .code(5)
        .stderr(predicate::str::contains("no schema selected"));
    yg(&directory)
        .args(["validate", "--schema-store", "store.yaml"])
        .assert()
        .success();
}

#[test]
fn network_schema_retrieval_is_denied_by_default() {
    let directory = TempDir::new().unwrap();
    write(&directory, "input.yaml", "name: api\n");

    yg(&directory)
        .args([
            "validate",
            "--schema",
            "http://127.0.0.1:9/schema.json",
            "input.yaml",
        ])
        .assert()
        .code(5)
        .stderr(predicate::str::contains("schema resource denied by policy"));
}

#[test]
fn parse_failures_are_runtime_errors_and_bad_flags_are_usage_errors() {
    let directory = TempDir::new().unwrap();
    write(&directory, "schema.yaml", "type: object\n");
    write(&directory, "broken.yaml", "name: [unterminated\n");

    yg(&directory)
        .args(["validate", "--schema", "schema.yaml", "broken.yaml"])
        .assert()
        .code(5)
        .stderr(predicate::str::contains("invalid YAML"));
    yg(&directory)
        .args(["validate", "--max-diagnostics", "many"])
        .assert()
        .code(2);
    yg(&directory)
        .args(["validate", "--allow-refs", "http"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("requires --allow-ref-host"));
}

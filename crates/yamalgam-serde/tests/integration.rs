#![allow(missing_docs)]
//! Integration tests over real-world YAML fixtures.
//!
//! Every `.yaml`/`.yml` file under `tests/fixtures/` must deserialize
//! through the streaming serde path. See `tests/fixtures/FIXTURES.md` for
//! fixture provenance.

use std::fs;
use std::path::{Path, PathBuf};

use yamalgam_core::Value;
use yamalgam_serde::Deserializer;

/// Fixtures that do not yet deserialize, with the parser/scanner limitation
/// that blocks each.
///
/// STALENESS: if a file in this list deserializes, the test panics — remove
/// the entry.
const KNOWN_FAILING: &[&str] = &[
    // Explicit `? key` entries without values in block mappings.
    "prettier/root/example.yml",
    // Scanner tab-indentation strictness — same family as the event
    // compliance allowlist entries (6BCT, 6CA3, 6HB6, 7A4E, DC7X, J3BT).
    "prettier/spec/spec-example-5-12-tabs-and-spaces.yml",
    "prettier/spec/spec-example-6-1-indentation-spaces.yml",
    "prettier/spec/spec-example-6-3-separation-spaces.yml",
    "prettier/spec/spec-example-7-6-double-quoted-lines.yml",
    "prettier/spec/various-trailing-tabs.yml",
];

/// Recursively collect `.yaml`/`.yml` files under `dir`.
fn collect_yaml_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read fixture dir {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("readable dir entry").path();
        if path.is_dir() {
            collect_yaml_files(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext == "yaml" || ext == "yml")
        {
            out.push(path);
        }
    }
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixture_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_yaml_files(&fixtures_dir(), &mut files);
    files.sort();
    assert!(
        files.len() > 100,
        "fixture corpus looks wrong: only {} files found",
        files.len()
    );
    files
}

/// Every fixture must deserialize into `Value` documents, except the
/// documented `KNOWN_FAILING` entries.
#[test]
fn fixtures_deserialize_into_value() {
    let mut failures = Vec::new();
    let mut stale = Vec::new();

    for path in fixture_files() {
        let content = fs::read_to_string(&path).expect("fixture is valid UTF-8");
        let rel = path
            .strip_prefix(fixtures_dir())
            .unwrap_or(&path)
            .display()
            .to_string();
        let known_failing = KNOWN_FAILING.contains(&rel.as_str());

        let result: Result<Vec<Value>, _> = Deserializer::from_str(&content)
            .documents::<Value>()
            .collect();

        match result {
            Ok(_) if known_failing => stale.push(rel),
            Err(e) if !known_failing => failures.push(format!("{rel}: {e}")),
            _ => {}
        }
    }

    assert!(
        stale.is_empty(),
        "STALE KNOWN_FAILING entries (now deserialize — remove them):\n  {}",
        stale.join("\n  ")
    );
    assert!(
        failures.is_empty(),
        "{} fixture(s) failed to deserialize:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

/// Fixtures must produce the same documents through the streaming serde
/// path as through the Composer.
///
/// Files using merge keys are skipped: the Composer applies `<<` semantics,
/// the serde path (like serde_yaml) surfaces `<<` as a literal key.
#[test]
fn fixtures_agree_with_composer() {
    let mut failures = Vec::new();
    let mut compared = 0_usize;

    for path in fixture_files() {
        let content = fs::read_to_string(&path).expect("fixture is valid UTF-8");
        if content.contains("<<:") {
            continue;
        }

        let composed = yamalgam_compose::from_str(&content);
        let deserialized: Result<Vec<Value>, _> = Deserializer::from_str(&content)
            .documents::<Value>()
            .collect();

        let rel = path
            .strip_prefix(fixtures_dir())
            .unwrap_or(&path)
            .display()
            .to_string();

        match (composed, deserialized) {
            (Ok(c), Ok(d)) => {
                compared += 1;
                if !docs_equal(&c, &d) {
                    failures.push(format!("{rel}: documents differ"));
                }
            }
            (Err(ce), Ok(_)) => failures.push(format!("{rel}: composer errored ({ce})")),
            (Ok(_), Err(de)) => failures.push(format!("{rel}: serde errored ({de})")),
            (Err(_), Err(_)) => {}
        }
    }

    assert!(compared > 100, "only {compared} fixtures compared");
    assert!(
        failures.is_empty(),
        "{} fixture(s) diverged from the Composer:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

/// NaN-aware structural equality (Value's PartialEq says NaN != NaN).
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Float(x), Value::Float(y)) => (x.is_nan() && y.is_nan()) || x == y,
        (Value::Sequence(xs), Value::Sequence(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| values_equal(x, y))
        }
        (Value::Mapping(xm), Value::Mapping(ym)) => {
            xm.len() == ym.len()
                && xm
                    .iter()
                    .zip(ym.iter())
                    .all(|((xk, xv), (yk, yv))| values_equal(xk, yk) && values_equal(xv, yv))
        }
        _ => a == b,
    }
}

fn docs_equal(a: &[Value], b: &[Value]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| values_equal(x, y))
}

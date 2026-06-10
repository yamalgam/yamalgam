#![allow(missing_docs)]
//! Serde round-trip test: verifies that the streaming serde Deserializer
//! (`documents::<Value>()`) produces the same `Value` documents as the
//! Composer for every YAML Test Suite case.
//!
//! The two pipelines share the parser and tag resolver but resolve anchors,
//! aliases, and document framing independently — agreement here means the
//! streaming path is semantically equivalent to DOM composition.

use std::path::Path;

use yamalgam_compare::test_case::extract_test_cases;
use yamalgam_compose::Composer;
use yamalgam_core::Value;
use yamalgam_serde::Deserializer;

/// Known divergences between Composer and serde that are understood and
/// acceptable.
///
/// STALENESS: if a case in this list agrees, the test panics — remove it.
const SERDE_ROUND_TRIP_ALLOWLIST: &[&str] = &[];

/// Structural equality with NaN-aware float comparison.
///
/// `Value`'s derived `PartialEq` says `NaN != NaN`, which would flag every
/// `.nan` test case as a divergence even when both pipelines agree.
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Float(x), Value::Float(y)) => (x.is_nan() && y.is_nan()) || x == y,
        (Value::Sequence(xs), Value::Sequence(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| values_equal(x, y))
        }
        (Value::Mapping(xm), Value::Mapping(ym)) => {
            // Both pipelines preserve insertion order, so zip is sound.
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

fn serde_round_trip_test(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let stem = path.file_stem().unwrap().to_string_lossy().to_string();
    let cases = extract_test_cases(&content);

    if cases.is_empty() {
        return Ok(());
    }

    let multi = cases.len() > 1;

    for case in &cases {
        let id = if multi {
            format!("{}#{}", stem, case.index)
        } else {
            stem.clone()
        };

        let in_allowlist = SERDE_ROUND_TRIP_ALLOWLIST.contains(&id.as_str());

        // Skip fail:true cases — the parser rejects them before either
        // consumer runs; rejection itself is covered by the compliance test.
        if case.fail {
            continue;
        }

        let composed = Composer::from_str(&case.yaml);
        let deserialized: Result<Vec<Value>, _> = Deserializer::from_str(&case.yaml)
            .documents::<Value>()
            .collect();

        let agree = match (&composed, &deserialized) {
            (Ok(c), Ok(d)) => docs_equal(c, d),
            // Both erroring is agreement — exact error text may differ.
            (Err(_), Err(_)) => true,
            _ => false,
        };

        if agree {
            eprintln!("SERDE_RT_PASS: {id}");
            if in_allowlist {
                panic!(
                    "STALE ALLOWLIST: {id} is in SERDE_ROUND_TRIP_ALLOWLIST but now agrees — remove it"
                );
            }
        } else {
            let comp_str = match &composed {
                Ok(c) => format!("Ok({} docs: {c:?})", c.len()),
                Err(e) => format!("Err({e})"),
            };
            let de_str = match &deserialized {
                Ok(d) => format!("Ok({} docs: {d:?})", d.len()),
                Err(e) => format!("Err({e})"),
            };
            eprintln!("SERDE_RT_FAIL: {id}\n  composer: {comp_str}\n  serde:    {de_str}");
            if !in_allowlist {
                panic!(
                    "SERDE_RT_FAIL: {id} — Composer and serde disagree, not in SERDE_ROUND_TRIP_ALLOWLIST\n  input: {:?}\n  composer: {comp_str}\n  serde:    {de_str}",
                    &case.yaml[..case.yaml.len().min(120)]
                );
            }
        }
    }

    Ok(())
}

datatest_stable::harness! {
    { test = serde_round_trip_test, root = "../../vendor/yaml-test-suite", pattern = r"\.yaml$" },
}

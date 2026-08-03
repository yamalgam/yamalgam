//! jq 1.8 manual examples run through the yg engine.

use std::collections::BTreeSet;

use serde::Deserialize;
use yamalgam_core::LoaderConfig;
use yamalgam_query::YamlVal;
use yamalgam_query::engine::{self, RunCtx};

#[derive(Deserialize)]
struct Manual {
    sections: Vec<Section>,
}

#[derive(Deserialize)]
struct Section {
    #[serde(default)]
    entries: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    #[serde(default)]
    examples: Vec<Example>,
}

#[derive(Deserialize)]
struct Example {
    program: String,
    input: String,
    output: Vec<String>,
}

#[derive(Deserialize)]
struct Allowlist {
    expected_failures: Vec<CaseRule>,
    skip: Vec<CaseRule>,
}

#[derive(Deserialize)]
struct CaseRule {
    program: String,
    input: String,
    reason: String,
}

impl CaseRule {
    fn matches(&self, example: &Example) -> bool {
        self.program == example.program && self.input == example.input
    }

    fn id(&self) -> (&str, &str) {
        (&self.program, &self.input)
    }
}

fn evaluate(program: &str, input: &str) -> Result<Vec<YamlVal>, String> {
    let filter = engine::compile_filter(program, &[]).map_err(|error| error.to_string())?;
    let context = RunCtx::new(LoaderConfig::moderate());
    let documents = yamalgam_query::parse_str(input).map_err(|error| error.to_string())?;
    let mut output = Vec::new();
    engine::run_filter(
        &filter,
        &context,
        documents.into_iter().map(Ok::<_, String>),
        |value| {
            output.push(value);
            Ok::<_, yamalgam_query::YgError>(())
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(output)
}

fn expected_values(example: &Example) -> Result<Vec<YamlVal>, String> {
    example
        .output
        .iter()
        .map(|source| {
            let mut documents =
                yamalgam_query::parse_str(source).map_err(|error| error.to_string())?;
            if documents.len() != 1 {
                return Err(format!(
                    "expected one document, parsed {} from {source:?}",
                    documents.len()
                ));
            }
            Ok(documents.remove(0))
        })
        .collect()
}

#[test]
fn jq_manual_examples() {
    let manual: Manual = yamalgam_serde::from_str(include_str!("fixtures/jq-manual.yml"))
        .expect("vendored jq manual parses through yamalgam");
    let allowlist: Allowlist =
        yamalgam_serde::from_str(include_str!("fixtures/manual-allowlist.yml"))
            .expect("manual allowlist parses through yamalgam");
    assert!(
        allowlist
            .expected_failures
            .iter()
            .chain(&allowlist.skip)
            .all(|rule| !rule.reason.trim().is_empty()),
        "every allowlist entry needs a reason"
    );

    let examples = manual
        .sections
        .iter()
        .flat_map(|section| &section.entries)
        .flat_map(|entry| &entry.examples)
        .collect::<Vec<_>>();
    assert!(examples.len() >= 250, "manual corpus unexpectedly small");

    let mut seen_allowlist = BTreeSet::new();
    let mut unexpected_failures = Vec::new();
    let mut unexpected_passes = BTreeSet::new();
    let (mut passed, mut expected_failed, mut skipped) = (0usize, 0usize, 0usize);

    for example in examples {
        if let Some(rule) = allowlist.skip.iter().find(|rule| rule.matches(example)) {
            seen_allowlist.insert(rule.id());
            skipped += 1;
            continue;
        }

        let result = match (
            evaluate(&example.program, &example.input),
            expected_values(example),
        ) {
            (Ok(actual), Ok(expected)) if actual == expected => Ok(()),
            (Ok(actual), Ok(expected)) => Err(format!(
                "value mismatch\n  input: {:?}\n  expected: {expected:?}\n  actual: {actual:?}",
                example.input
            )),
            (Err(error), _) => Err(format!("evaluation error: {error}")),
            (_, Err(error)) => Err(format!("expected-output parse error: {error}")),
        };
        let rule = allowlist
            .expected_failures
            .iter()
            .find(|rule| rule.matches(example));
        let allowlisted = rule.is_some();
        match (result, allowlisted) {
            (Ok(()), false) => passed += 1,
            (Err(_), true) => {
                seen_allowlist.insert(rule.unwrap().id());
                expected_failed += 1;
            }
            (Ok(()), true) => {
                unexpected_passes.insert(rule.unwrap().id());
            }
            (Err(error), false) => {
                unexpected_failures.push(format!("program: {:?}\n{error}", example.program))
            }
        }
    }

    let stale = allowlist
        .expected_failures
        .iter()
        .chain(&allowlist.skip)
        .map(CaseRule::id)
        .filter(|id| !seen_allowlist.contains(id))
        .collect::<Vec<_>>();
    eprintln!("jq-manual: {passed} pass, {expected_failed} expected-fail, {skipped} skipped");
    assert!(
        unexpected_failures.is_empty(),
        "unexpected failures (fix or allowlist with a reason):\n{}",
        unexpected_failures.join("\n\n")
    );
    assert!(
        unexpected_passes.is_empty(),
        "allowlisted programs now have passing examples:\n{unexpected_passes:#?}"
    );
    assert!(stale.is_empty(), "stale allowlist entries:\n{stale:#?}");
}

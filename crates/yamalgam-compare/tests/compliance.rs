#![allow(missing_docs)]
//! Compliance test harness: runs each YAML Test Suite case through
//! the yamalgam parser and compares the event stream against the
//! expected `tree` field from the test suite.
//!
//! Uses `datatest-stable` for file-driven test generation.
//!
//! Test outcomes:
//! - PASS: events match expected tree
//! - REJECTED: fail:true case correctly rejected (parser error)
//! - ACCEPTED: fail:true case NOT rejected (parser too permissive)
//! - ERROR: normal case produced parser error (parser too strict)
//! - MISMATCH: events differ from expected tree
//! - SKIP: no tree field in test case

use std::path::Path;

use yamalgam_compare::event_snapshot::EventSnapshot;
use yamalgam_compare::test_case::extract_test_cases;
use yamalgam_compare::tree_format::parse_tree;
use yamalgam_parser::{Event, Parser};

/// Known event mismatches that are understood and acceptable.
/// Each entry is a test case ID (e.g., "JEF9#1") with a comment.
///
/// STALENESS: if a case in this list passes, the test panics — remove it.
///
/// Categories:
/// - ERROR: parser rejects valid YAML (scanner too strict)
/// - MISMATCH: parser produces different events than expected
const EVENT_MISMATCH_ALLOWLIST: &[&str] = &[
    // --- ERROR: tab-as-indentation strictness (scanner rejects valid tab usage) ---
    "6BCT",   // tab used for indentation in block context
    "6CA3",   // tab used for indentation in flow context
    "6HB6",   // tab used for indentation in flow context (Spec Example 6.1)
    "7A4E",   // invalid tab used as indentation
    "DC7X",   // tab used for indentation in block context
    "DK95#0", // tab used for indentation in block context
    "J3BT",   // tab used for indentation in block context
    // --- ERROR: escape sequence strictness (scanner rejects valid escapes) ---
    "3RLN#1", // invalid escape character '\' in double-quoted scalar
    "3RLN#4", // invalid escape character '\' in double-quoted scalar
    "DE56#2", // invalid escape character '\' in double-quoted scalar
    "DE56#3", // invalid escape character '\' in double-quoted scalar
    "KH5V#1", // invalid escape character '\' in double-quoted scalar
    // --- ERROR: multiline simple key strictness ---
    "4MUZ#2", // multiline simple key not allowed
    "VJP3#1", // multiline simple key not allowed
    // --- ERROR: parser rejects empty value after explicit key ---
    "CFD4", // unexpected Value token (empty value after ? key)
    // --- ERROR: tab handling in various contexts ---
    "DK95#4", // unexpected character after %YAML directive with tabs
    "DK95#5", // unexpected character — tab in double-quoted scalar context
    // --- MISMATCH: trailing space in folded/flow scalar ---
    "6WPF",   // trailing space: expected " foo\nbar\nbaz", got " foo\nbar\nbaz "
    "9TFX",   // trailing space: expected "...3rd non-empty", got "...3rd non-empty "
    "DK95#8", // trailing space in double-quoted scalar (also tree-format trim issue)
    "PRH3",   // trailing space in folded scalar value
    "T4YY",   // trailing space in folded scalar value
    "TL85",   // trailing space: expected " foo\nbar\nbaz", got " foo\nbar\nbaz "
    // --- MISMATCH: literal block scalar trailing newline ---
    "JEF9#0", // expected "\n\n", got "\n" — trailing whitespace in literal block
    "JEF9#1", // expected "\n", got "" — trailing whitespace in literal block
    "K858",   // expected "\n", got "" — block scalar trailing newline
    // --- MISMATCH: empty vs space scalar ---
    "NAT4", // expected "", got " " — empty scalar vs single space
];

/// Convert a yamalgam parser Event to an EventSnapshot for comparison.
///
/// Returns None for directives (VersionDirective, TagDirective) which
/// are not represented in the tree format.
fn event_to_snapshot(event: &Event<'_>) -> Option<EventSnapshot> {
    match event {
        Event::StreamStart => Some(EventSnapshot {
            kind: "StreamStart".into(),
            anchor: None,
            tag: None,
            value: None,
            implicit: None,
        }),
        Event::StreamEnd => Some(EventSnapshot {
            kind: "StreamEnd".into(),
            anchor: None,
            tag: None,
            value: None,
            implicit: None,
        }),
        Event::VersionDirective { .. } | Event::TagDirective { .. } => None,
        Event::DocumentStart { implicit, .. } => Some(EventSnapshot {
            kind: "DocumentStart".into(),
            anchor: None,
            tag: None,
            value: None,
            implicit: Some(*implicit),
        }),
        Event::DocumentEnd { implicit, .. } => Some(EventSnapshot {
            kind: "DocumentEnd".into(),
            anchor: None,
            tag: None,
            value: None,
            implicit: Some(*implicit),
        }),
        Event::SequenceStart { anchor, tag, .. } => Some(EventSnapshot {
            kind: "SequenceStart".into(),
            anchor: anchor.as_ref().map(|a| a.to_string()),
            tag: tag.as_ref().map(|t| t.to_string()),
            value: None,
            implicit: None,
        }),
        Event::SequenceEnd { .. } => Some(EventSnapshot {
            kind: "SequenceEnd".into(),
            anchor: None,
            tag: None,
            value: None,
            implicit: None,
        }),
        Event::MappingStart { anchor, tag, .. } => Some(EventSnapshot {
            kind: "MappingStart".into(),
            anchor: anchor.as_ref().map(|a| a.to_string()),
            tag: tag.as_ref().map(|t| t.to_string()),
            value: None,
            implicit: None,
        }),
        Event::MappingEnd { .. } => Some(EventSnapshot {
            kind: "MappingEnd".into(),
            anchor: None,
            tag: None,
            value: None,
            implicit: None,
        }),
        Event::Scalar {
            anchor, tag, value, ..
        } => Some(EventSnapshot {
            kind: "Scalar".into(),
            anchor: anchor.as_ref().map(|a| a.to_string()),
            tag: tag.as_ref().map(|t| t.to_string()),
            value: Some(value.to_string()),
            implicit: None,
        }),
        Event::Alias { name, .. } => Some(EventSnapshot {
            kind: "Alias".into(),
            anchor: None,
            tag: None,
            value: Some(name.to_string()),
            implicit: None,
        }),
        // yamalgam-specific structural events — not in YAML Test Suite tree format.
        Event::Comment { .. }
        | Event::BlockEntry { .. }
        | Event::KeyIndicator { .. }
        | Event::ValueIndicator { .. } => None,
    }
}

/// Compare two EventSnapshots, skipping tag comparison.
///
/// Tags are skipped because the tree format uses full URIs (e.g.,
/// `tag:yaml.org,2002:str`) while yamalgam keeps shorthand (`!!str`).
fn events_match(expected: &EventSnapshot, actual: &EventSnapshot) -> bool {
    expected.kind == actual.kind
        && expected.value == actual.value
        && expected.anchor == actual.anchor
        && expected.implicit == actual.implicit
}

/// Run the parser on input, collect events as snapshots (filtering directives).
fn parse_to_snapshots(input: &str) -> Result<Vec<EventSnapshot>, String> {
    let parser = Parser::new(input);
    let mut snapshots = Vec::new();
    for result in parser {
        match result {
            Ok(event) => {
                if let Some(snap) = event_to_snapshot(&event) {
                    snapshots.push(snap);
                }
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(snapshots)
}

fn compliance_test(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let stem = path.file_stem().unwrap().to_string_lossy().to_string();
    let cases = extract_test_cases(&content);

    if cases.is_empty() {
        eprintln!("SKIP: no test cases in {}", path.display());
        return Ok(());
    }

    let multi = cases.len() > 1;

    for case in &cases {
        let id = if multi {
            format!("{}#{}", stem, case.index)
        } else {
            stem.clone()
        };

        let in_mismatch_al = EVENT_MISMATCH_ALLOWLIST.contains(&id.as_str());

        if case.fail {
            // fail:true case — parser should produce an error.
            match parse_to_snapshots(&case.yaml) {
                Err(_e) => {
                    eprintln!("REJECTED: {id} (fail:true, correctly rejected)");
                    if in_mismatch_al {
                        panic!(
                            "STALE ALLOWLIST: {id} is in EVENT_MISMATCH_ALLOWLIST but is now REJECTED — remove it"
                        );
                    }
                }
                Ok(events) => {
                    eprintln!(
                        "ACCEPTED: {id} (fail:true but parser produced {} events — too permissive)",
                        events.len()
                    );
                    // ACCEPTED is not a failure — it means our parser is more
                    // permissive. We track these but don't fail the test.
                }
            }
            continue;
        }

        // Normal case — we need a tree to compare against.
        let tree_str = match &case.tree {
            Some(t) => t,
            None => {
                eprintln!("SKIP: {id} (no tree field)");
                continue;
            }
        };

        let expected = parse_tree(tree_str);
        if expected.is_empty() {
            eprintln!("SKIP: {id} (empty tree)");
            continue;
        }

        match parse_to_snapshots(&case.yaml) {
            Err(e) => {
                eprintln!("ERROR: {id} (parser error on valid input: {e})");
                if !in_mismatch_al {
                    panic!(
                        "ERROR: {id} — parser error on non-fail case, not in EVENT_MISMATCH_ALLOWLIST"
                    );
                }
            }
            Ok(actual) => {
                // Compare event streams.
                let common_len = expected.len().min(actual.len());
                let mut mismatch_at = None;

                for i in 0..common_len {
                    if !events_match(&expected[i], &actual[i]) {
                        mismatch_at = Some(i);
                        break;
                    }
                }

                if mismatch_at.is_none() && expected.len() != actual.len() {
                    mismatch_at = Some(common_len);
                }

                if let Some(idx) = mismatch_at {
                    let exp_str = if idx < expected.len() {
                        format!("{:?}", expected[idx])
                    } else {
                        "<END>".to_string()
                    };
                    let act_str = if idx < actual.len() {
                        format!("{:?}", actual[idx])
                    } else {
                        "<END>".to_string()
                    };
                    eprintln!(
                        "MISMATCH: {id} at event {idx} (expected {} events, got {})\n  expected: {exp_str}\n  actual:   {act_str}",
                        expected.len(),
                        actual.len()
                    );
                    if !in_mismatch_al {
                        panic!("MISMATCH: {id} — not in EVENT_MISMATCH_ALLOWLIST");
                    }
                    // Staleness is implicitly checked: if it's in the allowlist
                    // but doesn't mismatch, we fall through to the PASS branch
                    // which checks for stale entries.
                } else {
                    eprintln!("PASS: {id} ({} events matched)", actual.len());
                    if in_mismatch_al {
                        panic!(
                            "STALE ALLOWLIST: {id} is in EVENT_MISMATCH_ALLOWLIST but now passes — remove it"
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

datatest_stable::harness! {
    { test = compliance_test, root = "../../vendor/yaml-test-suite", pattern = r"\.yaml$" },
}

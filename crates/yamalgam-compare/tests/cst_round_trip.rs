#![allow(missing_docs)]
//! CST round-trip test: verifies that parse_to_cst(input).to_text() == input
//! for every non-failing YAML Test Suite case.
//!
//! This is the ultimate correctness check for the CST — every byte of the
//! original source must be preserved.

use std::path::Path;

use yamalgam_compare::test_case::extract_test_cases;
use yamalgam_cst::parse_to_cst;

/// Known round-trip failures that are understood and acceptable.
///
/// STALENESS: if a case in this list passes, the test panics — remove it.
///
/// Categories:
/// - Flow collection close token duplication (extra `]` or `}`)
/// - Comment/whitespace ordering when comments arrive out of source order
/// - Edge cases with complex flow nesting or empty values
const ROUND_TRIP_ALLOWLIST: &[&str] = &[
    // --- Flow collection close token duplication ---
    // (Single-pair flow-SEQUENCE mappings were fixed by zero-width
    // MappingStart/End spans; the remaining cases involve flow-MAPPING
    // synthetic ends.)
    "57H4", // nested flow — extra `]`
    "6PBE", // flow mapping — extra `}`
    "7ZZ5", // flow mapping — extra `}`
    "AZ63", // flow mapping — extra `}`
    "RLU9", // flow mapping — extra `}`
    "S3PD", // flow mapping — extra `}`
    "S9E8", // flow mapping — extra `}`
    "SKE5", // flow mapping — extra `}`
];

fn cst_round_trip_test(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
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

        let in_allowlist = ROUND_TRIP_ALLOWLIST.contains(&id.as_str());

        // Skip fail:true cases — they produce errors, partial CSTs expected.
        if case.fail {
            continue;
        }

        let cst = parse_to_cst(&case.yaml);
        let round_tripped = cst.to_text();

        if round_tripped == case.yaml {
            eprintln!("ROUND_TRIP_PASS: {id}");
            if in_allowlist {
                panic!(
                    "STALE ALLOWLIST: {id} is in ROUND_TRIP_ALLOWLIST but now passes — remove it"
                );
            }
        } else {
            let diff_offset = round_tripped
                .bytes()
                .zip(case.yaml.bytes())
                .position(|(a, b)| a != b)
                .unwrap_or_else(|| round_tripped.len().min(case.yaml.len()));
            eprintln!(
                "ROUND_TRIP_FAIL: {id} at byte {diff_offset} (got {} bytes, expected {} bytes)",
                round_tripped.len(),
                case.yaml.len()
            );
            if diff_offset < round_tripped.len() && diff_offset < case.yaml.len() {
                eprintln!(
                    "  got byte:    {:?} (0x{:02x})",
                    round_tripped.as_bytes()[diff_offset] as char,
                    round_tripped.as_bytes()[diff_offset]
                );
                eprintln!(
                    "  expect byte: {:?} (0x{:02x})",
                    case.yaml.as_bytes()[diff_offset] as char,
                    case.yaml.as_bytes()[diff_offset]
                );
            }
            if !in_allowlist {
                panic!(
                    "ROUND_TRIP_FAIL: {id} — CST round-trip failed, not in ROUND_TRIP_ALLOWLIST\n  input:  {:?}\n  output: {:?}",
                    &case.yaml[..case.yaml.len().min(100)],
                    &round_tripped[..round_tripped.len().min(100)]
                );
            }
        }
    }

    Ok(())
}

datatest_stable::harness! {
    { test = cst_round_trip_test, root = "../../vendor/yaml-test-suite", pattern = r"\.yaml$" },
}

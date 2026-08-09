# Native JSON Parse Remediation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Close `serde-json-redundant-in-yaml-native-engine` by routing strict
JSON through yamalgam's scanner, parser, and `YamlVal` builder without a
runtime `serde_json::Value` DOM.

**Architecture:** Add a strict, allocation-free JSON syntax validator at the
query boundary, then materialize the validated text with the existing event
builder under `TagResolution::Json`. Keep `serde_json` only as a test oracle.
The validator closes the intentional YAML-superset gaps without introducing a
second value representation.

**Tech Stack:** Rust, yamalgam scanner/parser, cargo-nextest, Just.

## Task 1: Pin the current JSON contract

**Files:**

- Add: `crates/yamalgam-query/tests/json.rs`
- Modify: `crates/yamalgam-query/tests/jq_manual.rs`

**Steps:**

- [ ] Add RED tests requiring strict rejection of YAML-only syntax, trailing
  commas, extra documents, invalid escapes, and non-finite numeric results.
- [ ] Add RED tests for duplicate keys, large integers, Unicode escapes, and
  limits applied before materialization.
- [ ] Compare native values with `serde_json` across every JSON input and
  output text in the vendored jq-manual fixture.
- [ ] Run the focused tests and capture the expected failures.

## Task 2: Build through yamalgam

**Files:**

- Add: `crates/yamalgam-query/src/json.rs`
- Modify: `crates/yamalgam-query/src/build.rs`
- Modify: `crates/yamalgam-query/src/funs.rs`
- Modify: `crates/yamalgam-query/src/lib.rs`
- Modify: `crates/yamalgam-core/src/tag_resolution.rs`
- Modify: `crates/yamalgam-scanner/src/scanner.rs`

**Steps:**

- [ ] Validate one strict JSON value without constructing a second tree.
- [ ] Enforce depth, scalar, key, and per-collection limits while validating
  and building.
- [ ] Decode valid JSON surrogate pairs in the native scanner.
- [ ] Resolve strict JSON numbers to the same `i64`/`f64` boundary as the old
  serde transcode and reject non-finite results.
- [ ] Route `fromjson` and `parse_json_with_config` through the validator and
  event builder.

## Task 3: Remove the shipped parser dependency

**Files:**

- Modify: `crates/yamalgam-query/Cargo.toml`
- Modify: `Cargo.lock`

**Steps:**

- [ ] Move `serde_json` to dev-dependencies as the differential oracle and
  remove its `preserve_order` feature from the shipped graph.
- [ ] Confirm `cargo tree -p yamalgam -e normal` has no serde-json edge through
  yamalgam-query.
- [ ] Run scanner, core, and query tests, then `just clippy` and `just deny`.
- [ ] Commit code as `refactor(query): parse JSON through yamalgam`.

## Task 4: Record the closed finding

- [ ] Append a fixed ledger entry for
  `serde-json-redundant-in-yaml-native-engine` with the exact code SHA and
  update totals to `fixed: 19`, `open: 26`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record native JSON remediation`.

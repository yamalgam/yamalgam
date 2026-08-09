# Private Value Storage Remediation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Close `internal-value-representation-is-public` by exposing only the
supported value and metadata views, not their `Rc`/collection storage types.

**Architecture:** Make the `value` module private and keep `YamlVal`, `Meta`,
and `NodeStyle` available through the existing crate-root re-exports. Reduce
`Data`, `YgSeq`, and `YgMap` to crate visibility. Pin the boundary with a
compile-fail doctest while the crate remains unpublished.

**Tech Stack:** Rust 2024, rustdoc compile-fail tests, cargo-nextest, Just.

## Task 1: Seal the storage representation

**Files:**

- Modify: `crates/yamalgam-query/src/lib.rs`
- Modify: `crates/yamalgam-query/src/value.rs`
- Modify: `crates/yamalgam-query/tests/value.rs`

**Steps:**

- [ ] Add a compile-fail doctest that imports `value::Data` and confirm it
  fails because the current public representation remains importable.
- [ ] Make the `value` module private and make `Data`, `YgSeq`, and `YgMap`
  `pub(crate)`.
- [ ] Update integration tests to consume `Meta`, `NodeStyle`, and `YamlVal`
  from their supported crate-root exports.
- [ ] Run `cargo test --doc -p yamalgam-query`, the complete
  `yamalgam-query` nextest suite, `just fmt`, and `just clippy`.
- [ ] Commit code as `refactor(query): hide value storage internals`.

## Task 2: Record the closed finding

- [ ] Append a fixed ledger entry for `internal-value-representation-is-public`
  with the exact code SHA and update totals to `fixed: 15`, `open: 32`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record private value storage remediation`.

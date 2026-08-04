# Borrow Unsorted Mapping Entries Plan

> **For agentic workers:** Execute test-first and keep the allocation change
> inside the query emitter.

**Goal:** Close `mapping-entries-collected-into-vec-per-emit` by borrowing
mapping entries on the default unsorted path.

**Architecture:** Add a private double-ended iterator enum that wraps either
the mapping's borrowed `IndexMap` iterator or a sorted owned vector iterator.
Use it in validation and both emitters so allocation occurs only when
`sort_keys` is enabled.

**Tech Stack:** Rust 2024, IndexMap, cargo-nextest, Just.

## Task 1: Pin iterator selection

**Files:**

- Modify: `crates/yamalgam-query/src/emit/mod.rs`

**Steps:**

- [ ] Add a focused unit test proving the unsorted path returns the borrowed
  iterator variant and the sorted path preserves key ordering.
- [ ] Run the focused test and confirm it fails before implementation.

## Task 2: Branch before allocation

**Files:**

- Modify: `crates/yamalgam-query/src/emit/mod.rs`
- Modify: `crates/yamalgam-query/src/emit/json.rs`
- Modify: `crates/yamalgam-query/src/emit/yaml.rs`

**Steps:**

- [ ] Implement the private borrowed-or-sorted double-ended iterator.
- [ ] Replace all three unconditional mapping-entry collections.
- [ ] Run all `yamalgam-query` tests and `just clippy`.
- [ ] Commit as `perf(query): borrow unsorted mapping entries`.

## Task 3: Record the closed finding

- [ ] Append a fixed ledger entry for
  `mapping-entries-collected-into-vec-per-emit` with the exact code SHA and
  update totals to `fixed: 26`, `open: 19`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record mapping entry allocation remediation`.

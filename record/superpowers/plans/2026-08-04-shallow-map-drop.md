# Shallow Map Drop Plan

> **For agentic workers:** Keep nested-collection destruction iterative; only
> bypass the stack for maps proven to contain scalar entries.

**Goal:** Close `ygmap-drop-allocates-a-vec-per-mapping` by avoiding the
temporary drop stack for shallow scalar mappings.

**Architecture:** Add a private shape predicate over mapping keys and values.
When every entry is scalar, let the taken `IndexMap` drop directly; otherwise
retain the existing iterative `drop_values` path so deeply nested collections
remain stack-safe.

**Tech Stack:** Rust 2024, IndexMap, cargo-nextest, Just.

## Task 1: Pin the safe fast-path boundary

**Files:**

- Modify: `crates/yamalgam-query/src/value.rs`

**Steps:**

- [ ] Add a focused unit test proving scalar maps are shallow and any
  collection in a key or value selects the iterative path.
- [ ] Run the focused test and confirm it fails before implementation.

## Task 2: Avoid the shallow allocation

- [ ] Implement the scalar-only shape predicate and early return in
  `YgMap::drop`.
- [ ] Run all `yamalgam-query` tests and `just clippy`.
- [ ] Commit as `perf(query): drop shallow mappings directly`.

## Task 3: Record the closed finding

- [ ] Append a fixed ledger entry for
  `ygmap-drop-allocates-a-vec-per-mapping` with the exact code SHA and update
  totals to `fixed: 27`, `open: 17`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record shallow mapping drop remediation`.

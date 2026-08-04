# Concrete Keys Iterator Plan

> **For agentic workers:** Match the existing `entries()` API and preserve
> insertion order without boxing.

**Goal:** Close `keys-returns-boxed-trait-object` by returning a concrete
optional iterator.

**Architecture:** Change `YamlVal::keys` to return
`Option<impl ExactSizeIterator<Item = &YamlVal>>`, mirroring `entries()` and
representing non-mappings explicitly. Update the two public-API tests.

**Tech Stack:** Rust 2024, IndexMap, cargo-nextest, Just.

## Task 1: Pin mapping discrimination

**Files:**

- Modify: `crates/yamalgam-query/tests/value.rs`

**Steps:**

- [ ] Add a focused assertion that mapping keys are present and scalar keys
  return `None`.
- [ ] Run the focused test and confirm it fails against the boxed API.

## Task 2: Return the concrete iterator

**Files:**

- Modify: `crates/yamalgam-query/src/value.rs`
- Modify: `crates/yamalgam-query/tests/value.rs`
- Modify: `crates/yamalgam-query/tests/emit.rs`

**Steps:**

- [ ] Return `Option<indexmap::map::Keys<...>>` through `impl Iterator`.
- [ ] Update mapping callers to unwrap the now-explicit mapping case.
- [ ] Run all `yamalgam-query` tests and `just clippy`.
- [ ] Commit as `perf(query): return concrete mapping keys`.

## Task 3: Record the closed finding

- [ ] Append a fixed ledger entry for `keys-returns-boxed-trait-object` with
  the exact code SHA and update totals to `fixed: 30`, `open: 13`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record concrete keys iterator remediation`.

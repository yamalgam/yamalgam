# Resolved Type Alias Plan

> **For agentic workers:** Document the architectural alias instead of
> inventing unresolved state the value model does not retain.

**Goal:** Close `resolved-type-is-an-exact-alias-of-type` by explaining and
testing why `resolved_type` and `type` are identical in M10.

**Architecture:** Keep both filters for the draft-01 vocabulary. Document at
registration that parsing resolves scalars before query evaluation, add an
implementation note for the draft-02 decision, and pin equivalence across all
value kinds and explicit `resolve_as`.

**Tech Stack:** Rust 2024, jaq, cargo-nextest, rumdl, Just.

## Task 1: Document and test the alias

**Files:**

- Modify: `crates/yamalgam-query/src/funs.rs`
- Modify: `crates/yamalgam-query/tests/engine.rs`
- Modify: `docs/spec/yg-spec-draft-01-implementation-notes.md`

**Steps:**

- [ ] Explain the parse-time-resolution invariant at `resolved_type`.
- [ ] Test equivalence for every value kind and after `resolve_as`.
- [ ] Record the draft-02 choice: drop the alias or define distinct behavior.
- [ ] Run focused query tests, Markdown lint, and `just clippy`.
- [ ] Commit as `docs(query): explain resolved type alias`.

## Task 2: Record the remediation

- [ ] Append a fixed ledger entry for `resolved-type-is-an-exact-alias-of-type`
  with the exact code SHA and update totals to `fixed: 37`, `open: 6`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record resolved type alias remediation`.

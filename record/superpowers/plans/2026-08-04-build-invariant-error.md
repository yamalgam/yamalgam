# Build Invariant Error Remediation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Close `build-invariant-errors-labeled-as-limit` by preserving the
semantic distinction between user-configured resource exhaustion and an
internal parser/builder contract violation.

**Architecture:** Add one public `BuildError::Invariant(String)` variant and
route the two impossible-event paths through it. Keep both classes under
`YgError::InputParse` at the CLI boundary while allowing library callers to
match them accurately.

**Tech Stack:** Rust 2024, thiserror, cargo-nextest, Just.

## Task 1: Type invariant failures correctly

**Files:**

- Modify: `crates/yamalgam-query/src/error.rs`
- Modify: `crates/yamalgam-query/src/build.rs`

**Steps:**

- [ ] Add a focused error-variant test with an invariant-specific diagnostic.
- [ ] Add `BuildError::Invariant(String)` with public documentation.
- [ ] Replace both non-limit `BuildError::Limit` constructions.
- [ ] Confirm genuine limit sites remain `BuildError::Limit`.
- [ ] Run all `yamalgam-query` tests and `just clippy`.
- [ ] Commit as `fix(query): type builder invariant errors`.

## Task 2: Record the closed finding

- [ ] Append a fixed ledger entry for
  `build-invariant-errors-labeled-as-limit` with the exact code SHA and update
  totals to `fixed: 23`, `open: 22`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record builder invariant remediation`.

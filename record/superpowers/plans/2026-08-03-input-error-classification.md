# Input Error Classification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Close `input-error-class-flattened-through-string` by preserving the
original `YgError` variant across jaq's string-only input iterator boundary.

**Architecture:** Pair the string handed to jaq with the original typed error
in a run-local side slot. Restore the typed value when either the outer driver
or the native `input`/`inputs` path surfaces the matching error; retain normal
runtime classification for unrelated jaq failures.

**Tech Stack:** Rust 2024, jaq-core/jaq-std, cargo-nextest, Just.

## Task 1: Preserve typed input errors

**Files:**

- Modify: `crates/yamalgam-query/src/engine.rs`
- Modify: `crates/yamalgam-query/src/error.rs`
- Test: `crates/yamalgam-query/tests/engine.rs`
- Test: `crates/yamalgam-query/tests/jq_manual.rs`

**Steps:**

- [ ] Add RED tests asserting `YgError::InputRead` survives the outer driver
  and native `input` filter paths.
- [ ] Store the original error and its rendered jaq message when adapting the
  typed input iterator; clear stale state whenever a later input succeeds.
- [ ] Restore a matching original error before the current unconditional
  `InputParse`/`Runtime` conversion.
- [ ] Delete `From<String> for YgError` so new call sites must choose an exit
  class explicitly; update success-only test iterators to use `YgError`.
- [ ] Run focused tests, the full query suite, `just fmt`, and `just clippy`.
- [ ] Commit code as `fix(query): preserve input error classes`.

## Task 2: Record the closed finding

- [ ] Append a fixed ledger entry for
  `input-error-class-flattened-through-string` with the exact code SHA and
  update totals to `fixed: 9`, `open: 38`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record input error remediation`.

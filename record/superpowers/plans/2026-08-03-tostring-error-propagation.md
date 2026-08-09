# `tostring` Error Propagation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Close `into-string-substitutes-error-text-as-data` by turning JSON
conversion failures inside jaq's infallible `into_string` contract into engine
runtime errors.

**Architecture:** Render with the active run's limits. When conversion fails
inside an active engine guard, record the message in the scoped pending-error
slot added for infallible constructors; the engine already rejects pending
errors before accepting output.

**Tech Stack:** Rust 2024, jaq-core, cargo-nextest, Just.

## Task 1: Propagate conversion failures

**Files:**

- Modify: `crates/yamalgam-query/src/valt.rs`
- Test: `crates/yamalgam-query/tests/engine.rs`

**Steps:**

- [ ] Add RED engine tests proving duplicate JSON keys and an invocation-specific
  depth limit currently become successful string data.
- [ ] Build `EmitOpts` with `runtime_limits::current()` inside `into_string`.
- [ ] On emit failure during an active run, record the error and return an
  unreachable placeholder value; preserve the prior fallback only when no
  engine guard can consume a pending error.
- [ ] Assert the visitor receives no output and the engine returns a contextual
  runtime error.
- [ ] Run focused tests, the full query suite, `just fmt`, and `just clippy`.
- [ ] Commit code as `fix(query): propagate tostring failures`.

## Task 2: Record the closed finding

- [ ] Append a fixed ledger entry for
  `into-string-substitutes-error-text-as-data` with the exact code SHA and
  update totals to `fixed: 8`, `open: 39`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record tostring error remediation`.

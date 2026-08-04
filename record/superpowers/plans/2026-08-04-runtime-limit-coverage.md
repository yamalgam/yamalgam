# Runtime Limit Coverage Disposition Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Close `runtime-limits-cover-three-call-sites` with a unique contract
commit proving that filter-constructed arrays cannot escape as silently
truncated output.

**Architecture:** Retain the runtime pending-error channel introduced by the
growth-operator remediation. Pin its load-bearing contract at the engine
boundary: an infallible jaq constructor may return a placeholder internally,
but the engine must drain the recorded violation before invoking the output
visitor.

**Tech Stack:** Rust 2024, jaq-core, cargo-nextest, Just.

## Task 1: Pin the engine-boundary contract

**Files:**

- Modify: `crates/yamalgam-query/tests/engine.rs`
- Modify: `crates/yamalgam-query/src/runtime_limits.rs`

**Steps:**

- [ ] Add a focused test constructing an array beyond a tiny active limit.
- [ ] Assert the run returns a collection-size error and invokes the output
  visitor zero times.
- [ ] Document why the pending-error slot is required for jaq's infallible
  `FromIterator` boundary.
- [ ] Run all `yamalgam-query` tests and `just clippy`.
- [ ] Commit as `test(query): pin runtime constructor limits`.

## Task 2: Record the closed finding

- [ ] Append a fixed ledger entry for
  `runtime-limits-cover-three-call-sites` with the exact contract SHA and
  update totals to `fixed: 20`, `open: 25`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record runtime limit coverage`.

# Buffered CLI Output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Close `cli-output-is-unbuffered` by fully buffering normal stdout and
in-place temporary-file output around `run_records`.

**Architecture:** Keep `run_records` generic over `Write`; wrap its two concrete
sinks at the command boundary with `std::io::BufWriter`. Explicitly flush the
buffer before existing completion and durability checks.

**Tech Stack:** Rust 2024, `std::io::BufWriter`, cargo-nextest, Just.

## Task 1: Buffer both output paths

**File:**

- Modify: `crates/yamalgam/src/commands/query.rs`

**Steps:**

- [ ] Wrap `stdout.lock()` in `BufWriter`; retain the explicit final flush and
  broken-pipe classification.
- [ ] Scope a `BufWriter` around each in-place temporary file, flush it before
  dropping the borrow, then call the existing `sync_all` durability check.
- [ ] Run `cargo nextest run -p yamalgam --test yg`, the full workspace-relevant
  query tests, `just fmt`, and `just clippy`.
- [ ] Commit code as `perf(cli): buffer query output`.

## Task 2: Record the closed finding

- [ ] Append a fixed ledger entry for `cli-output-is-unbuffered` with the exact
  code SHA and update totals to `fixed: 11`, `open: 36`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record buffered output remediation`.

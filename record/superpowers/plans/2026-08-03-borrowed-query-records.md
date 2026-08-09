# Borrowed Query Records Remediation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Close `run-records-static-bound-forces-clones` by borrowing command
inputs and the run context for the duration of filter execution.

**Architecture:** Construct one owned `RunCtx` after compilation. Let
`deferred_inputs` borrow the file list, arguments, and the context's loader
configuration. Give `run_records` the iterator's real lifetime and a borrowed
`RunCtx`; pass that same context through every in-place file.

**Tech Stack:** Rust 2024 lifetimes, jaq 3, cargo-nextest, Just.

## Task 1: Remove the artificial static boundary

**Files:**

- Modify: `crates/yamalgam/src/commands/query.rs`
- Verify: `crates/yamalgam/tests/yg.rs`

**Steps:**

- [ ] Run the complete `yg` CLI integration suite as the refactor's behavioral
  characterization.
- [ ] Change `deferred_inputs` to borrow its files, arguments, and loader
  configuration.
- [ ] Change `run_records` to accept a borrowed `RunCtx` and an iterator with a
  matching non-static lifetime.
- [ ] Build one `RunCtx` in `cmd_query` and reuse it for normal and every
  in-place execution, removing per-run `Rc`, argument, configuration, and
  variable clones.
- [ ] Run the complete `yg` CLI integration suite, `just fmt`, and
  `just clippy`.
- [ ] Commit code as `refactor(cli): borrow query execution state`.

## Task 2: Record the closed finding

- [ ] Append a fixed ledger entry for `run-records-static-bound-forces-clones`
  with the exact code SHA and update totals to `fixed: 16`, `open: 31`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record borrowed query state remediation`.

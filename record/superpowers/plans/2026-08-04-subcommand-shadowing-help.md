# Subcommand Shadowing Help Plan

> **For agentic workers:** Keep Clap's conventional dispatch and document the
> escape hatch where users will see it.

**Goal:** Close `subcommand-name-shadowing-not-documented-in-help` by warning
that leading `doctor` and `info` tokens select subcommands.

**Architecture:** Extend the existing root-help epilog with a concise filter
note. Document `yg -- FILTER` for forcing first-position filter parsing and
`.doctor` / `.info` for selecting mapping keys; pin both help and dispatch.

**Tech Stack:** Rust 2024, clap, cargo-nextest, Just.

## Task 1: Document the dispatch boundary

**Files:**

- Modify: `crates/yamalgam/src/lib.rs`
- Modify: `crates/yamalgam/tests/yg.rs`

**Steps:**

- [ ] Add the subcommand-shadowing note to root help.
- [ ] Test that help contains the escape hatch.
- [ ] Test that `yg -- doctor` is parsed as a filter rather than dispatched.
- [ ] Run focused CLI tests and `just clippy`.
- [ ] Commit as `docs(cli): explain subcommand filter shadowing`.

## Task 2: Record the remediation

- [ ] Append a fixed ledger entry for
  `subcommand-name-shadowing-not-documented-in-help` with the exact code SHA
  and update totals to `fixed: 39`, `open: 4`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record subcommand shadowing remediation`.

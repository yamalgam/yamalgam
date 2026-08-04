# Jaq Time Feature Remediation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Close `jaq-std-time-feature-bundles-tzdb` by keeping jq date/time
builtins out of the query engine until the language specification requests
them.

**Architecture:** Disable `jaq-std` defaults and explicitly retain its
load-bearing `std`, `format`, `log`, `math`, and `regex` features. Omit `time`,
which is the sole edge to `jiff` and its platform time-zone database support.

**Tech Stack:** Cargo feature resolution, cargo-nextest, cargo-bloat, Just.

## Task 1: Remove the unrequested time subsystem

**Files:**

- Modify: `crates/yamalgam-query/Cargo.toml`
- Modify: `Cargo.lock`

**Steps:**

- [ ] Confirm the query specification and tests do not require jq date/time
  builtins and record the current `jiff` reverse dependency.
- [ ] Disable `jaq-std` defaults, retain the five used features, and document
  why `time` is omitted.
- [ ] Confirm `jiff` leaves the `yamalgam` dependency tree and release binary.
- [ ] Run all `yamalgam-query` tests, `just clippy`, and `just deny`.
- [ ] Commit code as `build(query): drop jaq time feature`.

## Task 2: Record the closed finding

- [ ] Append a fixed ledger entry for `jaq-std-time-feature-bundles-tzdb` with
  the exact code SHA and update totals to `fixed: 19`, `open: 27`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record jaq time feature remediation`.

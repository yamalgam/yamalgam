# Jaq Time Feature Remediation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Determine whether `jaq-std-time-feature-bundles-tzdb` can be closed
by keeping jq date/time builtins out of the query engine until the language
specification requests them.

**Architecture:** Disable `jaq-std` defaults and explicitly retain its
load-bearing `std`, `format`, `log`, `math`, and `regex` features. Omit `time`,
which is the sole edge to `jiff` and its platform time-zone database support.

**Tech Stack:** Cargo feature resolution, cargo-nextest, cargo-bloat, Just.

## Task 1: Remove the unrequested time subsystem

**Files:**

- Modify: `crates/yamalgam-query/Cargo.toml`
- Modify: `Cargo.lock`

**Steps:**

- [x] Confirm the query specification and tests do not require jq date/time
  builtins and record the current `jiff` reverse dependency.
- [x] Disable `jaq-std` defaults, retain the five used features, and document
  why `time` is omitted.
- [x] Confirm the build fails because the public `funs()` registry is not
  compiled without `time`.
- [x] Restore `time` as an explicit feature and record the upstream constraint
  in ADR 0009.
- [x] Run all `yamalgam-query` tests, `just clippy`, and `just deny`.
- [ ] Commit the manifest and decision as
  `build(query): make jaq feature cost explicit`.

## Task 2: Record the closed finding

- [ ] Append an accepted ledger entry for `jaq-std-time-feature-bundles-tzdb`
  with the exact code SHA and update totals to `accepted: 1`, `open: 27`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record jaq time feature decision`.

## Execution Outcome

The first test run failed because jaq-std 3.0.1 compiles its public aggregate
`funs()` registry only when all six default features, including `time`, are
enabled. Its non-time supplementary registries are private. ADR 0009 records
the decision to keep the complete feature set explicit instead of forking or
copying upstream internals. The audit disposition is accepted, not fixed.

# Emitter Dispatch Plan

> **For agentic workers:** Make the existing public format enum part of the
> public emission contract without removing the focused format functions.

**Goal:** Close `emit-opts-mixes-emitter-specific-options` by linking
`EmitFormat` to a public dispatch entry point and using it at the CLI boundary.

**Architecture:** Add `emit(value, format, options)` as the format-aware
front door over `to_yaml` and `to_json`. Keep the focused functions for callers
that already choose a format statically. Map the CLI's resolved output mode to
`EmitFormat` once.

**Tech Stack:** Rust 2024, cargo-nextest, Just.

## Task 1: Pin format dispatch

**Files:**

- Modify: `crates/yamalgam-query/tests/emit.rs`

**Steps:**

- [ ] Add a focused test proving YAML and JSON dispatch produce their existing
  format-specific output.
- [ ] Run the focused test and confirm it fails before implementation.

## Task 2: Add and consume dispatch

**Files:**

- Modify: `crates/yamalgam-query/src/emit/mod.rs`
- Modify: `crates/yamalgam/src/commands/query.rs`

**Steps:**

- [ ] Implement the public `emit` match over `EmitFormat`.
- [ ] Route the CLI's JSON and YAML branches through the dispatch function.
- [ ] Run `yamalgam-query` and `yamalgam` tests plus `just clippy`.
- [ ] Commit as `refactor(query): dispatch emission by format`.

## Task 3: Record the closed finding

- [ ] Append a fixed ledger entry for
  `emit-opts-mixes-emitter-specific-options` with the exact code SHA and update
  totals to `fixed: 31`, `open: 12`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record emitter dispatch remediation`.

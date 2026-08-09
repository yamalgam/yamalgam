# Emitter Control Escape Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Close `emitter-omits-c1-control-escapes` by ensuring YAML and JSON
emitters escape interoperability-sensitive Unicode control and separator
characters.

**Architecture:** Keep string classification and escaping centralized in the
existing emitter helpers. Force YAML double quoting for C1 controls and Unicode
line/paragraph separators, emit YAML-native escape forms, and mirror C1
escaping in JSON.

**Tech Stack:** Rust 2024, cargo-nextest, Just.

## Constraints

- Add no dependency and change no public API.
- Preserve current C0, quote, slash, and whitespace escapes.
- Escape U+007F through U+009F in YAML and JSON output.
- Escape U+2028/U+2029 as YAML `\L`/`\P`.
- Commit code before appending the ledger action.

## Task 1: Extend emitter escapes

**Files:**

- Modify: `crates/yamalgam-query/src/value.rs`
- Modify: `crates/yamalgam-query/src/emit/json.rs`
- Test: `crates/yamalgam-query/tests/emit.rs`

**Steps:**

- [ ] Extend the YAML control-character regression test with DEL, NEL, the C1
  upper bound, U+2028, and U+2029; add matching JSON assertions.
- [ ] Run the focused tests and verify RED because the characters appear raw.
- [ ] Make `needs_quotes` classify U+2028/U+2029 as requiring YAML quotes.
- [ ] Make YAML double quoting emit `\xNN` for U+007F through U+009F and
  `\L`/`\P` for U+2028/U+2029.
- [ ] Make JSON quoting emit `\u00NN` for U+007F through U+009F.
- [ ] Run focused tests, `cargo nextest run -p yamalgam-query`, `just fmt`, and
  `just clippy`.
- [ ] Commit the focused code action as
  `fix(query): escape extended control characters`.

## Task 2: Record the closed finding

**File:**

- Modify: `record/audits/2026-08-03-21-m10-fresh/actions-taken.md`

**Steps:**

- [ ] Append a `fixed` entry for the exact
  `emitter-omits-c1-control-escapes` slug with the code SHA.
- [ ] Update front matter to `fixed: 5`, `open: 42`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit the
  ledger-only action as `docs(audit): record control escape remediation`.

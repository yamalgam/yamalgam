# YAML Document Marker Quoting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Close `emitter-plain-document-end-marker` by ensuring strings that
look like YAML document boundary markers are always quoted.

**Architecture:** Extend the centralized `needs_quotes` syntax check rather
than threading root position through scalar rendering. Quote both document-end
and document-start spellings in every position for explicit, stable behavior.

**Tech Stack:** Rust 2024, cargo-nextest, Just.

## Task 1: Quote marker-like strings

**Files:**

- Modify: `crates/yamalgam-query/src/value.rs`
- Test: `crates/yamalgam-query/tests/emit.rs`

**Steps:**

- [ ] Add a root-scalar round-trip test for `...`, `... text`, `---`, and
  `--- text`, requiring double-quoted output.
- [ ] Run the focused test and verify RED for the document-end spellings.
- [ ] Make `needs_quotes` return true for exact marker strings and marker
  prefixes followed by whitespace.
- [ ] Run the focused test, full query tests, `just fmt`, and `just clippy`.
- [ ] Commit the code as `fix(query): quote YAML document markers`.

## Task 2: Record the closed finding

**File:**

- Modify: `record/audits/2026-08-03-21-m10-fresh/actions-taken.md`

**Steps:**

- [ ] Append a fixed entry for `emitter-plain-document-end-marker` with the
  exact code SHA and update totals to `fixed: 6`, `open: 41`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record document marker remediation`.

# JSON Emitter Allocation Remediation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Close `json-emitter-allocates-indent-and-triple-allocates-keys` by
removing per-node indentation buffers and avoidable scalar, number, and mapping
key copies from JSON emission.

**Architecture:** Keep the emitter's recursive traversal and public API intact.
Append indentation, quoted strings, and formatted numbers directly to the final
`String`. Track JSON object names as borrowed-or-owned `Cow<str>` values so
string keys borrow their existing `Rc<str>` allocation and non-string keys own
only the representation they require.

**Tech Stack:** Rust 2024, `std::borrow::Cow`, `core::fmt::Write`, cargo-nextest,
Just.

## Task 1: Remove transient emitter allocations

**Files:**

- Modify: `crates/yamalgam-query/src/emit/json.rs`
- Verify: `crates/yamalgam-query/tests/emit.rs`

**Steps:**

- [ ] Run the existing JSON pretty-print, escaping, sorting, and duplicate-key
  characterization tests before editing.
- [ ] Append number formatting and quoted strings directly to the destination
  buffer.
- [ ] Extend the destination with indentation spaces without constructing a
  temporary `String`.
- [ ] Store duplicate-detection keys as `Cow<str>`: borrow string keys and own
  only converted non-string keys, preserving the duplicate error payload.
- [ ] Run the focused emitter tests, the complete `yamalgam-query` suite,
  `just fmt`, and `just clippy`.
- [ ] Commit code as `perf(query): reduce JSON emitter allocations`.

## Task 2: Record the closed finding

- [ ] Append a fixed ledger entry for
  `json-emitter-allocates-indent-and-triple-allocates-keys` with the exact code
  SHA and update totals to `fixed: 12`, `open: 35`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record JSON emitter remediation`.

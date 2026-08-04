# YAML Paint Allocation Remediation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Close `yaml-emitter-paint-copies-every-scalar-when-color-is-off` by
letting uncolored rendering reuse borrowed or already-owned scalar text.

**Architecture:** Represent inline and literal scalar renderings as `Cow<str>`.
Make `paint` accept and return that `Cow`: color-disabled output returns it
unchanged, while color-enabled output owns the required ANSI-wrapped string.
Keep the emitter traversal and public API unchanged.

**Tech Stack:** Rust 2024, `std::borrow::Cow`, cargo-nextest, Just.

## Task 1: Eliminate color-disabled paint copies

**Files:**

- Modify: `crates/yamalgam-query/src/emit/yaml.rs`
- Verify: `crates/yamalgam-query/tests/emit.rs`
- Verify: `crates/yamalgam/tests/yg.rs`

**Steps:**

- [ ] Run the existing YAML emitter and CLI color characterization tests before
  editing.
- [ ] Carry borrowed plain strings and literal blocks through `Rendered` with
  `Cow<str>`; retain owned text only where quoting or formatting requires it.
- [ ] Make the no-color `paint` branch return its input unchanged and make the
  color branch construct the ANSI-wrapped owned value.
- [ ] Update alias, property, and literal-line callers without changing output.
- [ ] Run the focused emitter and CLI color tests, the complete
  `yamalgam-query` suite, `just fmt`, and `just clippy`.
- [ ] Commit code as `perf(query): avoid uncolored YAML paint copies`.

## Task 2: Record the closed finding

- [ ] Append a fixed ledger entry for
  `yaml-emitter-paint-copies-every-scalar-when-color-is-off` with the exact code
  SHA and update totals to `fixed: 13`, `open: 34`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record YAML paint remediation`.

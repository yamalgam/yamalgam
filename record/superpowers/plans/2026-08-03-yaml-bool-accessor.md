# YAML Boolean Accessor Remediation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Close `as-bool-inherent-shadows-jaq-truthiness` by making natural
`as_bool` method syntax select jaq truthiness whenever `jaq_core::ValT` is in
scope.

**Architecture:** Rename the resolved YAML boolean extractor to `as_boolean`
while the crate remains unpublished. Keep jaq's trait method named `as_bool`.
Remove the duplicate numeric match by delegating `jaq_std::ValT::as_f64` to the
inherent numeric accessor.

**Tech Stack:** Rust 2024, jaq 3 traits, cargo-nextest, Just.

## Task 1: Separate YAML extraction from jq truthiness

**Files:**

- Modify: `crates/yamalgam-query/src/value.rs`
- Modify: `crates/yamalgam-query/src/valt.rs`
- Modify: `crates/yamalgam-query/tests/value.rs`

**Steps:**

- [ ] Change the jaq truthiness regression to use natural `.as_bool()` method
  syntax with `jaq_core::ValT` imported and confirm it fails while the inherent
  accessor shadows the trait.
- [ ] Rename the inherent extractor to `as_boolean` and update its public
  accessor test.
- [ ] Delegate the `jaq_std::ValT::as_f64` implementation to
  `YamlVal::as_f64`.
- [ ] Run focused value/trait tests, the complete `yamalgam-query` suite,
  `just fmt`, and `just clippy`.
- [ ] Commit code as `refactor(query): separate YAML bool from jq truthiness`.

## Task 2: Record the closed finding

- [ ] Append a fixed ledger entry for
  `as-bool-inherent-shadows-jaq-truthiness` with the exact code SHA and update
  totals to `fixed: 14`, `open: 33`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record boolean accessor remediation`.

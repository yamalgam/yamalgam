# Cheap Emitter Anchor Validation Plan

> **For agentic workers:** Preserve the shared preflight contract and remove
> its avoidable anchor-name allocation.

**Goal:** Mitigate `emit-validation-walks-the-tree-a-second-time` without
duplicating resource-limit and alias-expansion policy across two emitters.

**Architecture:** Keep the shared validation walk as the single policy gate
for YAML and JSON, but store cloned `Rc<str>` anchor handles rather than newly
allocated `String` copies. Record the retained traversal as a deliberate
clarity and consistency tradeoff.

**Tech Stack:** Rust 2024, cargo-nextest, Just.

## Task 1: Remove anchor-name copies

**Files:**

- Modify: `crates/yamalgam-query/src/emit/mod.rs`

**Steps:**

- [ ] Change the validation anchor set to `HashSet<Rc<str>>` by cloning the
  existing reference-counted anchor handle.
- [ ] Run all `yamalgam-query` tests and `just clippy`.
- [ ] Commit as `perf(query): reuse anchor names during emit validation`.

## Task 2: Record the mitigation

- [ ] Append a mitigated ledger entry for
  `emit-validation-walks-the-tree-a-second-time` with the exact code SHA and
  update totals to `mitigated: 1`, `open: 18`.
- [ ] State that the shared preflight walk remains to keep resource-limit and
  alias policy centralized across both emitters.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record emit validation mitigation`.

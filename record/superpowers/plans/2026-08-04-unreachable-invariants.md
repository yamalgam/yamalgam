# Unreachable Invariant Documentation Plan

> **For agentic workers:** Verify existing behavior; this remediation changes
> documentation only and does not require a RED test.

**Goal:** Close `unreachable-macros-guard-unstated-invariants` by making each
caller-side contract visible beside the four production `unreachable!` arms.

**Architecture:** Preserve the existing panic-free external behavior and add
localized comments naming the check or type contract that excludes each arm.
Avoid a new sum type or emitter refactor for paths the audit independently
verified are unreachable.

**Tech Stack:** Rust 2024, cargo-nextest, Just.

## Task 1: Document the four contracts

**Files:**

- Modify: `crates/yamalgam-query/src/emit/yaml.rs`
- Modify: `crates/yamalgam-query/src/build.rs`

**Steps:**

- [ ] Document the `is_block_collection` gate before `block_collection`.
- [ ] Document why `Position::Flow` cannot produce a literal scalar.
- [ ] Document the scalar caller's prior exclusion of collections.
- [ ] Document the `TagResolver::resolve_scalar` scalar-only contract.
- [ ] Run all `yamalgam-query` tests and `just clippy`.
- [ ] Commit as `docs(query): state unreachable invariants`.

## Task 2: Record the closed finding

- [ ] Append a fixed ledger entry for
  `unreachable-macros-guard-unstated-invariants` with the exact documentation
  SHA and update totals to `fixed: 25`, `open: 20`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record unreachable invariant remediation`.

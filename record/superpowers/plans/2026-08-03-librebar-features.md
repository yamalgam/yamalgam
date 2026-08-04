# Librebar Feature Scope Remediation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Close `librebar-default-features-not-disabled` by making yamalgam's
declared librebar feature set authoritative.

**Architecture:** Disable librebar default features at the dependency edge and
retain the explicitly used `cli`, `config`, `logging`, and `diagnostics`
features. Do not change any command behavior or add dependencies.

**Tech Stack:** Cargo feature resolution, cargo-nextest, Just.

## Task 1: Scope librebar to used features

**File:**

- Modify: `crates/yamalgam/Cargo.toml`

**Steps:**

- [ ] Record the current resolved librebar features and confirm source usage is
  limited to the four named modules.
- [ ] Add `default-features = false` to the librebar dependency.
- [ ] Confirm the resolved graph no longer enables librebar `default`, `cache`,
  or `crash` while retaining all four requested features.
- [ ] Run the `yamalgam` tests, `just clippy`, and `just deny`.
- [ ] Commit code as `build(cli): scope librebar features`.

## Task 2: Record the closed finding

- [ ] Append a fixed ledger entry for
  `librebar-default-features-not-disabled` with the exact code SHA and update
  totals to `fixed: 18`, `open: 29`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record librebar feature remediation`.

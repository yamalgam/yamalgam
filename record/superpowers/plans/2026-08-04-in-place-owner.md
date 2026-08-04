# In-Place Ownership Remediation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Close `in-place-chown-failure-aborts-the-edit` by allowing an atomic
edit to proceed when the caller lacks permission to reproduce source
ownership, while keeping genuine ownership I/O failures fatal.

**Architecture:** Classify the `chown` error at one helper. Permission denied
emits a warning and returns success, matching the existing hard-link metadata
warning policy; all other error kinds remain `YgError::Runtime`. Permission
bits and atomic staging remain unchanged.

**Tech Stack:** Rust 2024, Unix filesystem APIs, cargo-nextest, Just.

## Task 1: Make the ownership policy explicit

**Files:**

- Modify: `crates/yamalgam/src/commands/query.rs`

**Steps:**

- [ ] Add RED unit coverage classifying permission denied as nonfatal and an
  unrelated I/O error as fatal.
- [ ] Route Unix `chown` failures through the classification helper.
- [ ] Warn that the replacement will be owned by the current user when
  ownership cannot be preserved.
- [ ] Leave non-permission failures fatal with the existing path context.
- [ ] Run all `yamalgam` package tests and `just clippy`.
- [ ] Commit as `fix(cli): tolerate in-place chown denial`.

## Task 2: Record the closed finding

- [ ] Append a fixed ledger entry for
  `in-place-chown-failure-aborts-the-edit` with the exact code SHA and update
  totals to `fixed: 24`, `open: 21`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record in-place ownership remediation`.

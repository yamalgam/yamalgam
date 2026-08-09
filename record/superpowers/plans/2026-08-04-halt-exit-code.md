# Halt Exit-Code Remediation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Close `halt-exit-code-truncated-to-u8` so a nonzero halt request can
never wrap to shell success.

**Architecture:** Centralize the i32-to-POSIX conversion. Preserve codes in
`0..=255` and jq's conventional `halt(-1) -> 255`; map every other out-of-range
nonzero code to generic failure 255 instead of applying a lossy modulo cast.

**Tech Stack:** Rust 2024, cargo-nextest, Just.

## Task 1: Pin and correct exit conversion

**Files:**

- Modify: `crates/yamalgam/src/commands/query.rs`
- Modify: `crates/yamalgam/tests/yg.rs`

**Steps:**

- [ ] Add a RED assertion that `halt(256)` must not exit successfully.
- [ ] Replace the direct `as u8` cast with a named conversion helper.
- [ ] Preserve valid POSIX values and the existing `halt(-1) -> 255` behavior.
- [ ] Document why out-of-range nonzero values become 255.
- [ ] Run all `yamalgam` package tests and `just clippy`.
- [ ] Commit as `fix(cli): prevent halt code wraparound`.

## Task 2: Record the closed finding

- [ ] Append a fixed ledger entry for `halt-exit-code-truncated-to-u8` with the
  exact code SHA and update totals to `fixed: 22`, `open: 23`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record halt exit-code remediation`.

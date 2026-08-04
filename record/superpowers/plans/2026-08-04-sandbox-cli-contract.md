# Sandbox CLI Contract Plan

> **For agentic workers:** Give the existing sandbox selector an explicit,
> user-visible contract distinct from input resource limits.

**Goal:** Close `sandboxed-compile-options-unreachable-from-cli` with a focused
SHA proving that the shipped `--no-env` path is specified and discoverable.

**Architecture:** Retain jq-compatible environment access by default and the
already-wired `--no-env` selector for `CompileOptions::sandboxed()`. Specify
the flag alongside security controls, clarify that it is independent of
`--strict` input limits, and pin the help contract in a CLI test.

**Tech Stack:** Rust 2024, clap, cargo-nextest, Just.

## Task 1: Specify the reachable sandbox path

**Files:**

- Modify: `docs/spec/yg-query-language-spec-draft-01.md`
- Modify: `crates/yamalgam/src/lib.rs`
- Modify: `crates/yamalgam/tests/yg.rs`

**Steps:**

- [ ] Specify `--no-env` and its independence from `--strict`.
- [ ] Clarify the flag's CLI help text.
- [ ] Add a help regression test alongside the existing behavior test.
- [ ] Run focused CLI tests, Markdown lint, and `just clippy`.
- [ ] Commit as `docs(cli): specify no-env sandbox control`.

## Task 2: Record the remediation

- [ ] Append a fixed ledger entry for
  `sandboxed-compile-options-unreachable-from-cli` with the exact code SHA and
  update totals to `fixed: 35`, `open: 8`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record sandbox CLI remediation`.

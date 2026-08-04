# In-Place Conflict Contract Plan

> **For agentic workers:** Preserve the enforced safety policy and reconcile
> the paper trail that still describes the superseded plan.

**Goal:** Close `in-place-conflicts-diverge-from-plan-undocumented` by stating
the shipped exclusions and their rationale in implementation notes and help.

**Architecture:** Keep Clap's `-i` conflicts with `-n`, `-s`, and `-j`, plus
the runtime guard for global `--format json`. Document that null/slurp lack
coherent per-file semantics and that JSON-in-YAML replacement is rejected as
an accidental format rewrite.

**Tech Stack:** Rust 2024, clap, cargo-nextest, rumdl, Just.

## Task 1: Publish the conflict policy

**Files:**

- Modify: `crates/yamalgam/src/lib.rs`
- Modify: `crates/yamalgam/tests/yg.rs`
- Modify: `docs/spec/yg-spec-draft-01-implementation-notes.md`

**Steps:**

- [ ] State the exclusions in `-i` help.
- [ ] Reconcile the implementation notes with the shipped policy and rationale.
- [ ] Add a help regression alongside the existing conflict behavior test.
- [ ] Run focused CLI tests, Markdown lint, and `just clippy`.
- [ ] Commit as `docs(cli): explain in-place exclusions`.

## Task 2: Record the remediation

- [ ] Append a fixed ledger entry for
  `in-place-conflicts-diverge-from-plan-undocumented` with the exact code SHA
  and update totals to `fixed: 38`, `open: 5`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record in-place conflict remediation`.

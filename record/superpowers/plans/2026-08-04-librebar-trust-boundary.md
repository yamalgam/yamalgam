# Librebar Trust Boundary Plan

> **For agentic workers:** Make the unavoidable transitive boundary reviewable
> and keep the residual upstream work explicit.

**Goal:** Mitigate `librebar-upgrade-expands-trust-boundary` by recording the
`rust-zstd` and `leakguard` edges, the controls in place, and the removal path.

**Architecture:** Keep librebar's required `logging` and `diagnostics`
features explicitly scoped with defaults disabled. Add a manifest pointer to a
focused implementation note documenting locked versions, safe-Rust evidence,
advisory checks, and the need for upstream `rust-zstd` optionality.

**Tech Stack:** Cargo, Rust 2024, rumdl, Cargo Deny, Cargo Audit.

## Task 1: Record the reviewed boundary

**Files:**

- Modify: `crates/yamalgam/Cargo.toml`
- Modify: `docs/spec/yg-spec-draft-01-implementation-notes.md`

**Steps:**

- [ ] Name both new transitive crates and their feature paths.
- [ ] Record existing controls and current safe-Rust evidence.
- [ ] State that `rust-zstd` removal requires upstream feature optionality.
- [ ] Run locked graph checks, Markdown lint, `just deny`, and `cargo audit`.
- [ ] Commit as `docs(deps): record librebar trust boundary`.

## Task 2: Record the mitigation

- [ ] Append a mitigated ledger entry for
  `librebar-upgrade-expands-trust-boundary` with the exact code SHA and update
  totals to `mitigated: 3`, `open: 2`.
- [ ] Explain residual low-maturity transitive dependency risk.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record librebar trust mitigation`.

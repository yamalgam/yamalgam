# jaq Patch Update Plan

> **For agentic workers:** Admit patch fixes automatically while keeping
> trait-surface minor upgrades explicit.

**Goal:** Close `jaq-exact-pins-block-patch-updates` by replacing exact jaq
pins with patch-compatible tilde requirements.

**Architecture:** Use `~3.1.0` for jaq-core and `~3.0.1` for jaq-std. This lets
`cargo update` resolve patch and security fixes but excludes new minor versions
that may change the `ValT` / `DataT` traits implemented by `YamlVal`.

**Tech Stack:** Cargo, Rust 2024, cargo-nextest, Just.

## Task 1: Relax patch constraints

**Files:**

- Modify: `crates/yamalgam-query/Cargo.toml`

**Steps:**

- [ ] Replace both exact jaq pins with patch-compatible tilde requirements.
- [ ] Document why minor upgrades remain review-gated.
- [ ] Confirm locked metadata resolves unchanged jaq versions.
- [ ] Run all `yamalgam-query` tests, `just clippy`, and `just deny`.
- [ ] Commit as `build(query): allow jaq patch updates`.

## Task 2: Record the remediation

- [ ] Append a fixed ledger entry for `jaq-exact-pins-block-patch-updates` with
  the exact code SHA and update totals to `fixed: 40`, `open: 3`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record jaq patch update remediation`.

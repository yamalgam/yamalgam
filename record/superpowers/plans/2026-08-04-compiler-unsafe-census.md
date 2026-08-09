# Compiler Unsafe Census Plan

> **For agentic workers:** Drop the silently empty Geiger path and use the
> compiler's syntax-aware unsafe lint for the shipped binary graph.

**Goal:** Close `geiger-census-produced-no-unsafe-data` with a reproducible
unsafe-code census that emits real evidence and cannot report success empty.

**Architecture:** Add `just unsafe-census`, running locked `cargo check` for
the `yamalgam` package with `RUSTFLAGS=--force-warn unsafe-code`, one build job,
no compiler wrapper, and an isolated target directory. This uses rustc's
parser rather than textual matching and covers the shipped normal dependency
graph while excluding benchmark/dev-only crates.

**Tech Stack:** Rust 2024, rustc unsafe_code lint, Cargo, Just.

## Task 1: Replace the failed census path

**Files:**

- Modify: `.justfile`
- Modify: `README.md`
- Modify: `SECURITY.md`

**Steps:**

- [ ] Add the isolated, single-job `unsafe-census` recipe.
- [ ] Document its shipped-graph scope and why cargo-geiger is not evidence.
- [ ] Run the recipe and confirm unsafe diagnostics are emitted for
  dependencies and none for workspace sources.
- [ ] Run focused Markdown lint and `just clippy`.
- [ ] Commit as `build(security): add compiler unsafe census`.

## Task 2: Record the remediation

- [ ] Append a fixed ledger entry for `geiger-census-produced-no-unsafe-data`
  with the exact code SHA and update totals to `fixed: 41`, `open: 1`.
- [ ] Record the observed diagnostic total and zero first-party diagnostics.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record unsafe census remediation`.

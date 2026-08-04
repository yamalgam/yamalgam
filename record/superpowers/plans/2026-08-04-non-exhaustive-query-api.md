# Non-Exhaustive Query API Plan

> **For agentic workers:** Apply the compatibility attributes before publish
> and update downstream construction without changing runtime behavior.

**Goal:** Close `public-enums-lack-non-exhaustive` by reserving future public
error variants and emitter option fields.

**Architecture:** Mark `BuildError`, `YgError`, `EmitError`, and `EmitOpts`
non-exhaustive. Keep fields public, but update external crates/tests to mutate
`EmitOpts::default()` or `EmitOpts::compact()` rather than use struct literals.
Give the CLI's downstream error match a generic runtime-failure fallback.

**Tech Stack:** Rust 2024, cargo-nextest, Just.

## Task 1: Reserve the public surface

**Files:**

- Modify: `crates/yamalgam-query/src/error.rs`
- Modify: `crates/yamalgam-query/src/emit/mod.rs`

**Steps:**

- [ ] Add `#[non_exhaustive]` to the three error enums and options struct.
- [ ] Compile the workspace and use the expected downstream failures to locate
  exhaustive matches and external struct literals.

## Task 2: Update downstream callers

**Files:**

- Modify: `crates/yamalgam/src/commands/query.rs`
- Modify: `crates/yamalgam-query/tests/emit.rs`

**Steps:**

- [ ] Build options from defaults and assign the required public fields.
- [ ] Add a fallback exit class for future query errors.
- [ ] Run `yamalgam-query` and `yamalgam` tests plus `just clippy`.
- [ ] Commit as `refactor(query): make public API non-exhaustive`.

## Task 3: Record the closed finding

- [ ] Append a fixed ledger entry for `public-enums-lack-non-exhaustive` with
  the exact code SHA and update totals to `fixed: 32`, `open: 11`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record non-exhaustive API remediation`.

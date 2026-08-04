# Resolver String Contract Plan

> **For agentic workers:** Treat optimized string predicates as behaviorally
> identical to full scalar resolution.

**Goal:** Close `resolves-as-string-overrides-restate-resolution` by making the
equivalence requirement explicit and exercising every duplicated branch.

**Architecture:** Keep the allocation-avoiding resolver overrides, document
their observational-equivalence contract on `TagResolver`, and strengthen the
shared regression corpus. This is contract and test hardening; the audit found
no current behavior divergence.

**Tech Stack:** Rust 2024, cargo-nextest, Just.

## Task 1: Define and verify equivalence

**Files:**

- Modify: `crates/yamalgam-core/src/tag_resolution.rs`

**Steps:**

- [ ] Document that every `resolves_as_string` override must equal
  `matches!(resolve_scalar(value), Value::String(_))` for every input.
- [ ] Expand the shared equivalence corpus across empty/null, boolean, radix,
  separator, sign, decimal, float, special-float, overflow, and invalid
  lookalike branches.
- [ ] Run the focused equivalence test, all `yamalgam-core` tests, and
  `just clippy`.
- [ ] Commit as `docs(core): define resolver string contract`.

## Task 2: Record the remediation

- [ ] Append a fixed ledger entry for
  `resolves-as-string-overrides-restate-resolution` with the exact code SHA
  and update totals to `fixed: 33`, `open: 10`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record resolver string contract remediation`.

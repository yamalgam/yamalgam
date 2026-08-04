# Runtime Growth Limits Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development and superpowers:systematic-debugging to
> implement this plan task-by-task.

**Goal:** Close `growth-operators-bypass-runtime-limits` by enforcing active
scalar and collection budgets on fallible growth operators and jaq's
infallible sequence-construction boundary.

**Architecture:** Check sizes before allocating in `Add` and fallible `ValT`
constructors. For jaq's infallible `FromIterator<YamlVal>` contract, collect
incrementally, stop at the configured limit, record the violation in the
existing thread-local runtime state, and make the engine convert that pending
violation into a contextual runtime error before accepting output.

**Tech Stack:** Rust 2024, jaq-core, cargo-nextest, Just.

## Constraints

- Add no dependency and change no public API.
- Check arithmetic with `checked_add` before allocation or extension.
- Preserve unlimited behavior when the active profile has no relevant cap.
- Never return a truncated constructed value as successful output.
- Keep nested runtime-limit guards isolated and restore prior state on drop.
- Commit code before appending the ledger action.

## Task 1: Bound direct growth operators

**Files:**

- Modify: `crates/yamalgam-query/src/ops.rs`
- Test: `crates/yamalgam-query/tests/engine.rs`

**Steps:**

- [ ] Add RED tests proving string concatenation and sequence/mapping addition
  exceed small active limits without an error.
- [ ] Check summed string bytes before formatting or copying.
- [ ] Check summed sequence length before extending.
- [ ] Check the mapping's resulting unique-key count before extending.

## Task 2: Bound jaq collection constructors

**Files:**

- Modify: `crates/yamalgam-query/src/runtime_limits.rs`
- Modify: `crates/yamalgam-query/src/value.rs`
- Modify: `crates/yamalgam-query/src/valt.rs`
- Modify: `crates/yamalgam-query/src/engine.rs`
- Test: `crates/yamalgam-query/tests/engine.rs`

**Steps:**

- [ ] Add RED coverage for array construction, mapping construction, and
  sequence `map_values` expansion under a small collection cap.
- [ ] Add a scoped pending-error slot beside active limits; activation clears
  it, nested guards restore it, and the first violation wins.
- [ ] Make `FromIterator<YamlVal>` collect incrementally and stop before the
  first over-limit insertion while recording the violation.
- [ ] Make fallible `ValT::from_map` and `map_values` collect incrementally and
  return jaq errors directly when the cap is exceeded.
- [ ] Check and contextualize a pending infallible-construction error after
  each jaq result and after result exhaustion, before reporting success.

## Task 3: Verify and record

- [ ] Run focused tests, `cargo nextest run -p yamalgam-query`, `just fmt`, and
  `just clippy`.
- [ ] Commit code as `fix(query): enforce runtime growth limits`.
- [ ] Append a fixed ledger entry for
  `growth-operators-bypass-runtime-limits` with the exact code SHA and update
  totals to `fixed: 7`, `open: 40`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record runtime growth remediation`.

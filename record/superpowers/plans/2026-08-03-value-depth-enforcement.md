# Value Depth Enforcement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development and superpowers:systematic-debugging to
> implement this plan task-by-task.

**Goal:** Close `value-tree-recursion-has-no-depth-bound` by ensuring all
untrusted input and filter-produced values satisfy the active resulting-tree
depth limit before recursive traits can consume them.

**Architecture:** Add one iterative `YamlVal` depth validator modeled on the
emitter's worklist. Apply it when builder nodes are finished (capturing alias
expanded depth), at jaq's infallible array-construction boundary (recording a
pending runtime error and returning a safe placeholder), and before the engine
accepts any filter output.

**Tech Stack:** Rust 2024, `ResourceLimits`, jaq-core, cargo-nextest, Just.

## Constraints

- Add no dependency and change no public API.
- Never recurse while measuring value depth.
- Use the existing root-at-depth-zero convention.
- Reject alias-expanded input before inserting it as an anchor or map key.
- Prevent an over-depth infallible construction from continuing to grow.
- Preserve unlimited behavior under profiles with no depth cap.

## Task 1: Enforce resulting input depth

**Files:**

- Modify: `crates/yamalgam-query/src/value.rs`
- Modify: `crates/yamalgam-query/src/build.rs`
- Test: `crates/yamalgam-query/tests/value.rs`

**Steps:**

- [ ] Add a RED test with a syntactically shallow anchor chain whose expanded
  value exceeds a small `max_depth`.
- [ ] Add an iterative `YamlVal::check_depth` worklist over sequence items and
  mapping keys/values.
- [ ] Call it from `Builder::finish` before anchor insertion.

## Task 2: Enforce filter-produced depth

**Files:**

- Modify: `crates/yamalgam-query/src/value.rs`
- Modify: `crates/yamalgam-query/src/engine.rs`
- Test: `crates/yamalgam-query/tests/engine.rs`

**Steps:**

- [ ] Add RED coverage for a reduction that constructs nesting beyond the
  active limit and then attempts a recursive comparison.
- [ ] Validate `FromIterator<YamlVal>` results iteratively; on active-run
  failure record the error and return `null` so evaluation cannot deepen the
  unsafe placeholder.
- [ ] Validate every jaq output against the active context before truthiness or
  visitor delivery, covering constructors outside `FromIterator`.
- [ ] Run focused tests, the full query suite, `just fmt`, and `just clippy`.
- [ ] Commit code as `fix(query): enforce resulting value depth`.

## Task 3: Record the closed finding

- [ ] Append a fixed ledger entry for
  `value-tree-recursion-has-no-depth-bound` with the exact code SHA and update
  totals to `fixed: 10`, `open: 37`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record value depth remediation`.

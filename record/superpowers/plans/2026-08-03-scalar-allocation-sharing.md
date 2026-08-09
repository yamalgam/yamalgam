# Scalar Allocation Sharing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close `build-raw-metadata-duplicates-every-scalar` and
`resolver-allocates-throwaway-string-per-scalar` by retaining one shared
allocation for string scalar data and raw metadata.

**Architecture:** Allocate each scalar's source text once as `Rc<str>` at the
builder boundary. Pass a clone of that reference through tag resolution and
store the original reference in `Meta::raw`. Use `resolves_as_string` before
full resolution so common string scalars skip the temporary owned
`Value::String`; typed scalars retain only their raw-text allocation.

**Tech Stack:** Rust 2024, `std::rc::Rc`, `yamalgam-core::TagResolver`,
cargo-nextest, Just.

## Global Constraints

- Add no dependency and change no public API.
- Preserve scalar values, tags, styles, anchors, and raw metadata.
- Share the exact `Rc<str>` allocation for every string scalar path.
- Preserve one distinct raw allocation for typed scalars.
- Commit code before appending the fixed ledger entry.

---

### Task 1: Share scalar text and avoid throwaway strings

**Files:**

- Modify: `crates/yamalgam-query/src/build.rs`

**Interfaces:**

- Consumes: parser scalar `Cow<str>`, configured `TagResolution`, and explicit
  YAML tag semantics.
- Produces: `Data` plus `Meta::raw` that share one `Rc<str>` whenever the
  resolved data remains a string.

**Implementation steps:**

- [ ] **Step 1: Add the failing allocation-sharing regression test**

Add an internal builder test covering plain, quoted, and explicitly tagged
string scalars. For each value, destructure `Data::Str`, obtain `Meta::raw`,
and assert `Rc::ptr_eq(data, raw)`.

- [ ] **Step 2: Run the focused test and verify RED**

```bash
cargo nextest run -p yamalgam-query --lib \
  -E 'test(string_scalars_share_resolved_and_raw_allocation)' \
  --success-output=immediate
```

Expected: FAIL because `resolve_scalar` and `finish` each allocate an
independent `Rc<str>`.

- [ ] **Step 3: Allocate scalar source text once**

In the scalar event arm, convert the parser value to `Rc<str>` once. Pass a
clone into `resolve_scalar` and move the original reference into `finish`.
Change `finish` to accept `Option<Rc<str>>` and store it directly.

- [ ] **Step 4: Reuse the reference for string resolution**

Change `resolve_scalar` and `resolve_tagged` to accept `Rc<str>`. Quoted,
explicit string-tagged, unknown-tagged, and failed typed-tag resolution paths
must return that same reference as `Data::Str`.

- [ ] **Step 5: Bypass owned `Value::String` construction**

For untagged plain scalars, call the configured resolver's
`resolves_as_string` first. Return `Data::Str(text)` immediately when true;
only invoke full `resolve_scalar` for typed values.

- [ ] **Step 6: Run focused and crate tests**

```bash
cargo nextest run -p yamalgam-query --lib \
  -E 'test(string_scalars_share_resolved_and_raw_allocation)' \
  --success-output=immediate
cargo nextest run -p yamalgam-query
```

- [ ] **Step 7: Run formatting and lint gates**

```bash
just fmt
just clippy
```

- [ ] **Step 8: Commit the code action**

Review the diff, stage only `build.rs`, and commit with:

```bash
git commit -m "perf(query): share scalar text allocations"
```

---

### Task 2: Record both closed findings

**Files:**

- Modify: `record/audits/2026-08-03-21-m10-fresh/actions-taken.md`

**Interfaces:**

- Consumes: Task 1's exact code-fix SHA.
- Produces: both allocation findings become `fixed`; front matter becomes
  `fixed: 4`, `open: 43`.

**Ledger steps:**

- [ ] **Step 1: Append one fixed action for both exact slugs**

Append an entry linking
`build-raw-metadata-duplicates-every-scalar` and
`resolver-allocates-throwaway-string-per-scalar`. Record the Task 1 SHA and
describe both the shared allocation and the string-resolution fast path.

- [ ] **Step 2: Validate and commit**

Run markdown lint, confirm the immutable audit hashes, stage only the ledger,
and commit:

```bash
git commit -m "docs(audit): record scalar allocation remediation"
```

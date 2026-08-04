# Bounded Value Display Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish `explode-aliases-unbounded-expansion` remediation by bounding
`YamlVal` rendering used inside jaq runtime errors.

**Architecture:** Expose a clone of the active thread-local `ResourceLimits`
inside `yamalgam-query`. `Display for YamlVal` renders through a private buffer
that never grows beyond `max_output_bytes`, stops walking once truncated, and
substitutes `...` when `max_depth` is exceeded. Unlimited profiles preserve
the existing behavior.

**Tech Stack:** Rust 2024, `core::fmt`, `yamalgam-core::ResourceLimits`,
cargo-nextest, Just.

## Global Constraints

- Add no dependency and change no public API.
- Preserve existing compact `Display` output below configured limits.
- Bound traversal before building the complete error string.
- Keep the existing `runtime_limits::Guard` restoration semantics.
- Commit code before appending the superseding fixed ledger entry.

---

### Task 1: Bound display traversal and allocation

**Files:**

- Modify: `crates/yamalgam-query/src/runtime_limits.rs`
- Modify: `crates/yamalgam-query/src/value.rs`
- Test: `crates/yamalgam-query/tests/engine.rs`

**Interfaces:**

- Consumes: active `ResourceLimits`, `YamlVal` compact display semantics, and
  jaq type-error formatting.
- Produces: `runtime_limits::current() -> ResourceLimits` and bounded
  `Display for YamlVal` behavior.

**Implementation steps:**

- [ ] **Step 1: Add the failing output-budget regression test**

Append beside the alias-limit tests:

```rust
#[test]
fn runtime_error_display_obeys_output_limit() {
    let document = concat!(
        "a: &a [1234567890]\n",
        "b: &b [*a, *a, *a]\n",
        "c: &c [*b, *b, *b]\n",
        "d: [*c, *c, *c]\n",
    );
    let filter = engine::compile_filter(".d | merge", &[]).unwrap();
    let mut config = LoaderConfig::moderate();
    config.limits.max_alias_expansions = None;
    config.limits.max_collection_size = None;
    config.limits.max_output_bytes = Some(48);
    let context = RunCtx::new(config);
    let documents = yamalgam_query::parse_str(document).unwrap();

    let error = engine::run_filter(
        &filter,
        &context,
        documents.into_iter().map(Ok::<_, String>),
        |_| Ok::<_, yamalgam_query::YgError>(()),
    )
    .unwrap_err();
    let message = error.to_string();

    assert!(message.contains("..."), "untruncated error: {message}");
    assert!(message.len() <= 96, "oversized error: {} bytes", message.len());
}
```

This catches removal of the output buffer cap and early traversal stop.

- [ ] **Step 2: Run the output-budget test and verify RED**

```bash
cargo nextest run -p yamalgam-query --test engine \
  -E 'test(runtime_error_display_obeys_output_limit)' \
  --success-output=immediate
```

Expected: FAIL because the current error contains the complete logical alias
tree and no truncation marker.

- [ ] **Step 3: Add the failing depth-budget regression test**

```rust
#[test]
fn runtime_error_display_obeys_depth_limit() {
    let filter = engine::compile_filter("merge", &[]).unwrap();
    let mut config = LoaderConfig::moderate();
    config.limits.max_depth = Some(2);
    config.limits.max_output_bytes = None;
    let context = RunCtx::new(config);
    let documents = yamalgam_query::parse_str("[[[[1]]]]\n").unwrap();

    let error = engine::run_filter(
        &filter,
        &context,
        documents.into_iter().map(Ok::<_, String>),
        |_| Ok::<_, yamalgam_query::YgError>(()),
    )
    .unwrap_err();

    assert!(error.to_string().contains("[[[...]]]"));
}
```

This catches removal of depth-aware truncation.

- [ ] **Step 4: Run the depth-budget test and verify RED**

```bash
cargo nextest run -p yamalgam-query --test engine \
  -E 'test(runtime_error_display_obeys_depth_limit)' \
  --success-output=immediate
```

Expected: FAIL because the current display contains `[[[[1]]]]`.

- [ ] **Step 5: Expose the active limits inside the crate**

Add to `runtime_limits.rs`:

```rust
pub fn current() -> ResourceLimits {
    ACTIVE.with(|active| active.borrow().clone())
}
```

- [ ] **Step 6: Add a private bounded display buffer**

Add a private `DisplayBuffer` in `value.rs` with fields `output: String`,
`max_bytes: Option<usize>`, and `truncated: bool`. Implement
`core::fmt::Write` so:

- writes within the limit append normally;
- the first oversized write appends the largest UTF-8-safe prefix that leaves
  room for up to three ASCII dots;
- the final buffer length never exceeds `max_bytes`; and
- subsequent writes are ignored once `truncated` is true.

**Remaining implementation steps:**

- [ ] **Step 7: Move recursive display into a bounded helper**

Add `YamlVal::fmt_bounded(&mut DisplayBuffer, &ResourceLimits, depth)` and move
the existing `Display` match into it. Before matching, return immediately when
the buffer is truncated; when `check_depth(depth)` fails, append `...` instead
of descending. Sequence and mapping loops must stop when the buffer truncates.

Use the existing scalar spellings, `needs_quotes`, `quote_double`, and
`fmt_float`. Recursive children receive `depth + 1` with checked overflow.

- [ ] **Step 8: Route `Display` through the bounded helper**

Replace the recursive `Display` body with:

```rust
let limits = crate::runtime_limits::current();
let mut output = DisplayBuffer::new(limits.max_output_bytes);
self.fmt_bounded(&mut output, &limits, 0);
formatter.write_str(&output.output)
```

- [ ] **Step 9: Run focused tests and verify GREEN**

```bash
cargo nextest run -p yamalgam-query --test engine \
  -E 'test(runtime_error_display_obeys_output_limit) \
      | test(runtime_error_display_obeys_depth_limit)' \
  --success-output=immediate
```

Expected: 2 tests pass.

- [ ] **Step 10: Run regression and lint gates**

```bash
cargo nextest run -p yamalgam-query
just fmt
just clippy
```

- [ ] **Step 11: Commit the code action**

Review the diff, stage only `runtime_limits.rs`, `value.rs`, and `engine.rs`,
then commit with:

```bash
git commit -m "fix(query): bound runtime value display"
```

---

### Task 2: Supersede the mitigation in the ledger

**Files:**

- Modify: `record/audits/2026-08-03-21-m10-fresh/actions-taken.md`

**Interfaces:**

- Consumes: Task 1's exact code-fix SHA.
- Produces: latest disposition for
  `explode-aliases-unbounded-expansion` becomes `fixed`; front matter becomes
  `fixed: 2`, `mitigated: 0`, `open: 45`.

**Ledger steps:**

- [ ] **Step 1: Append a superseding fixed entry**

Do not edit the prior mitigation entry. Append an entry titled
`Runtime value display bounded`, link the same exact slug, record the Task 1
SHA, and state that active output/depth limits now prevent error formatting
from traversing the complete logical alias tree.

- [ ] **Step 2: Validate and commit**

Run markdown lint, confirm immutable hashes, stage only the ledger, and commit:

```bash
git commit -m "docs(audit): close alias expansion finding"
```

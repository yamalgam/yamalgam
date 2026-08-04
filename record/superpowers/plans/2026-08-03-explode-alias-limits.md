# Alias Explosion Limits Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve `explode-aliases-unbounded-expansion` by enforcing the active
loader limits while materializing aliases.

**Architecture:** Pass `RunCtx`'s existing `ResourceLimits` into
`YamlVal::explode_aliases`. A per-invocation budget counts logical alias nodes
and total collection entries materialized across the output tree; recursive
calls also enforce `max_depth`. Limit failures use jaq's existing `Error::str`
channel and reach the CLI as runtime errors.

**Tech Stack:** Rust 2024, `yamalgam-query`, `yamalgam-core::ResourceLimits`,
jaq, cargo-nextest, Just.

## Global Constraints

- Add no dependency and do not change public configuration fields.
- Preserve `explode_aliases` output when all limits are satisfied.
- Keep the implementation in `yamalgam-query`; the CLI remains unchanged.
- Do not edit immutable audit artifacts.
- Commit the code action before appending its ledger entry.

---

### Task 1: Enforce alias and materialization budgets

**Files:**

- Modify: `crates/yamalgam-query/src/value.rs`
- Modify: `crates/yamalgam-query/src/funs.rs`
- Test: `crates/yamalgam-query/tests/engine.rs`

**Interfaces:**

- Consumes: `RunCtx.config.limits`, `ResourceLimits::check_depth`,
  `check_alias_expansions`, and `check_collection_size`.
- Produces:
  `YamlVal::explode_aliases(&ResourceLimits) -> jaq_core::ValR<YamlVal>`.

**Implementation steps:**

- [ ] **Step 1: Add the failing alias-budget regression test**

Append this test beside `alias_audit_filters_report_and_strip_identity`:

```rust
#[test]
fn explode_aliases_obeys_alias_expansion_limit() {
    let document = "a: &a [1]\nb: &b [*a, *a]\nc: [*b, *b]\n";
    let filter = engine::compile_filter("explode_aliases", &[]).unwrap();
    let mut config = LoaderConfig::moderate();
    config.limits.max_alias_expansions = Some(2);
    config.limits.max_collection_size = None;
    let context = RunCtx::new(config);
    let documents = yamalgam_query::parse_str(document).unwrap();

    let error = engine::run_filter(
        &filter,
        &context,
        documents.into_iter().map(Ok::<_, String>),
        |_| Ok::<_, yamalgam_query::YgError>(()),
    )
    .unwrap_err();

    assert!(error.to_string().contains("alias expansions"));
}
```

This catches removal or bypass of the logical alias counter.

- [ ] **Step 2: Run the alias-budget test and verify RED**

```bash
cargo nextest run -p yamalgam-query --test engine \
  -E 'test(explode_aliases_obeys_alias_expansion_limit)' \
  --success-output=immediate
```

Expected: FAIL because the current filter succeeds.

- [ ] **Step 3: Add the failing materialization-budget regression test**

```rust
#[test]
fn explode_aliases_obeys_materialized_entry_budget() {
    let filter = engine::compile_filter("explode_aliases", &[]).unwrap();
    let mut config = LoaderConfig::moderate();
    config.limits.max_alias_expansions = None;
    config.limits.max_collection_size = Some(2);
    let context = RunCtx::new(config);
    let documents = yamalgam_query::parse_str("[[1, 2], [3, 4]]\n").unwrap();

    let error = engine::run_filter(
        &filter,
        &context,
        documents.into_iter().map(Ok::<_, String>),
        |_| Ok::<_, yamalgam_query::YgError>(()),
    )
    .unwrap_err();

    assert!(error.to_string().contains("collection size"));
}
```

This catches removal of the total materialized-entry budget. A single
two-element collection remains legal; recursively copying three or more
entries during one expansion does not.

- [ ] **Step 4: Run the materialization test and verify RED**

```bash
cargo nextest run -p yamalgam-query --test engine \
  -E 'test(explode_aliases_obeys_materialized_entry_budget)' \
  --success-output=immediate
```

Expected: FAIL because the current filter succeeds.

- [ ] **Step 5: Add a private expansion budget**

Add this private helper near `YamlVal` in `value.rs`:

```rust
#[derive(Default)]
struct ExpansionBudget {
    aliases: usize,
    entries: usize,
}

impl ExpansionBudget {
    fn count_alias(
        &mut self,
        limits: &yamalgam_core::ResourceLimits,
    ) -> jaq_core::ValR<(), YamlVal> {
        self.aliases = self
            .aliases
            .checked_add(1)
            .ok_or_else(|| jaq_core::Error::str("alias expansion count overflow"))?;
        limits
            .check_alias_expansions(self.aliases)
            .map_err(jaq_core::Error::str)
    }

    fn count_entries(
        &mut self,
        count: usize,
        limits: &yamalgam_core::ResourceLimits,
    ) -> jaq_core::ValR<(), YamlVal> {
        self.entries = self
            .entries
            .checked_add(count)
            .ok_or_else(|| jaq_core::Error::str("materialized entry count overflow"))?;
        limits
            .check_collection_size(self.entries)
            .map_err(jaq_core::Error::str)
    }
}
```

- [ ] **Step 6: Make alias explosion fallible and bounded**

Change the public crate-level entry point to initialize one budget:

```rust
pub(crate) fn explode_aliases(
    &self,
    limits: &yamalgam_core::ResourceLimits,
) -> jaq_core::ValR<Self> {
    self.explode_aliases_bounded(limits, &mut ExpansionBudget::default(), 0)
}
```

Add a private recursive method that:

1. calls `limits.check_depth(depth).map_err(jaq_core::Error::str)?`;
2. calls `budget.count_alias(limits)?` when `meta.alias_of` is present;
3. calls `budget.count_entries(items.len(), limits)?` before copying a
   sequence;
4. calls `budget.count_entries(mapping.0.len(), limits)?` before copying a
   mapping;
5. recursively collects sequences and mapping pairs into
   `jaq_core::ValR<Vec<YamlVal>>` and `jaq_core::ValR<YgMap>`;
6. clears `anchor` and `alias_of` exactly as the existing method does; and
7. returns `Ok(Self { data, meta })`.

- [ ] **Step 7: Pass active limits from the native filter**

Replace the `explode_aliases` registration body in `funs.rs` with:

```rust
("explode_aliases", v(0), |context_value| {
    let limits = &context_value.0.data().context.config.limits;
    bome(context_value.1.explode_aliases(limits))
}),
```

- [ ] **Step 8: Run focused tests and verify GREEN**

```bash
cargo nextest run -p yamalgam-query --test engine \
  -E 'test(explode_aliases_obeys_alias_expansion_limit) \
      | test(explode_aliases_obeys_materialized_entry_budget)' \
  --success-output=immediate
```

Expected: 2 tests pass.

- [ ] **Step 9: Run affected-crate and lint checks**

```bash
cargo nextest run -p yamalgam-query
just fmt
just clippy
```

Expected: all query tests pass and clippy exits with warnings denied.

- [ ] **Step 10: Commit the code action**

```bash
git --no-pager diff --check
git --no-pager diff --stat
git --no-pager diff
git add crates/yamalgam-query/src/value.rs \
  crates/yamalgam-query/src/funs.rs \
  crates/yamalgam-query/tests/engine.rs
git commit -m "fix(query): bound alias materialization"
```

---

### Task 2: Append the remediation ledger

**Files:**

- Modify: `record/audits/2026-08-03-21-m10-fresh/actions-taken.md`

**Interfaces:**

- Consumes: Task 1's exact 40-character code-fix SHA.
- Produces: ledger status `fixed: 2`, `open: 45`.

**Ledger steps:**

- [ ] **Step 1: Verify the code-fix SHA**

```bash
git --no-pager show -s --format='%H%n%s' HEAD
```

Expected subject: `fix(query): bound alias materialization`.

- [ ] **Step 2: Append one action**

Update only the ledger front matter counts and append an entry titled
`Alias materialization bounded` with disposition `fixed`, exact slug
`explode-aliases-unbounded-expansion`, the verified Task 1 SHA, author
`Codex`, and a concise description of both budgets and their regression tests.

- [ ] **Step 3: Validate and commit the ledger**

Run markdown lint, verify the immutable artifact hashes against the campaign
design, stage only `actions-taken.md`, and commit with:

```bash
git commit -m "docs(audit): record alias expansion remediation"
```

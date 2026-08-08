# Specialized Resolver Upstreams Implementation Plan

> **For Codex:** Follow the focused RED -> GREEN -> verification -> review ->
> signed code commit -> exact-SHA ledger commit workflow.

**Goal:** Remove the measured parser-to-resolver vtable cost on ordinary loading
paths while preserving boxed compatibility and serde's intentional erased
public boundary.

**Architecture:** ADR-0012 gives `ResolvedEvents` a generic upstream iterator
parameter whose default is the existing boxed iterator. Parser-backed internal
callers pass a concrete named adapter; explicitly boxed callers retain the
existing two-parameter type. The permanent dispatch benchmark compares both
production paths.

**ADR:** `record/decisions/0012-specialize-parser-backed-resolver-upstreams.md`.

**Tech stack:** Rust 2024, existing resolver/composer/serde APIs, Divan,
`cargo nextest`, `just check`.

## Task 1: Generalize the resolver adapter compatibly

**Files:**

- Modify and test: `crates/yamalgam-parser/src/resolve.rs`

1. Add a generic upstream iterator parameter to `ResolvedEvents`, defaulted to
   the current boxed iterator type.
2. Generalize constructors and the `Iterator` implementation over that type.
3. Add a compile/runtime regression proving the explicit legacy
   `ResolvedEvents<'_, NoopResolver>` boxed form and inferred concrete form both
   work and produce identical events.

## Task 2: Specialize parser-backed consumers

**Files:**

- Modify: `crates/yamalgam-compose/src/lib.rs`
- Modify: `crates/yamalgam-serde/src/de.rs`
- Modify: `crates/yamalgam-cst/src/lib.rs`
- Modify: `crates/yamalgam-query/src/build.rs`

1. Add one named parse-result adapter per crate.
2. Pass concrete parser adapters directly to `ResolvedEvents` in ordinary,
   configured, unchecked, and custom resolver string paths.
3. Retain serde's outer boxed event iterator and boxed tag resolver.
4. Keep explicitly heterogeneous or caller-supplied iterator paths boxed.

## Task 3: Convert the prototype into a production benchmark

**Files:**

- Modify: `crates/yamalgam-bench/benches/dispatch.rs`

1. Remove the duplicate prototype adapter.
2. Compare production `ResolvedEvents` with boxed and concrete upstreams.
3. Assert setup parity before timing representative event and composer paths.
4. Retain erased-serde and concrete/dynamic tag-resolution baselines.

## Task 4: Verify, review, commit, and record

1. Run parser, composer, serde, CST, and query focused suites.
2. Run the dispatch benchmark and confirm the production typed path remains
   faster in representative medians without treating wall-clock values as a CI
   gate.
3. Run `just check` and `git --no-pager diff --check`.
4. Obtain independent architecture and code review and address every confirmed
   issue.
5. Create one focused signed implementation-and-benchmark commit.
6. Append one immutable ledger entry citing the exact full code SHA, update
   counters from fixed 23/open 4 to fixed 24/open 3, independently review the
   append, and create a separate signed ledger commit.

## Verification commands

```bash
cargo nextest run -p yamalgam-parser -p yamalgam-compose -p yamalgam-serde \
  -p yamalgam-cst -p yamalgam-query
cargo bench -p yamalgam-bench --bench dispatch
just check
git --no-pager diff --check
```

# Resolver Consumer Reachability Implementation Plan

> **For Codex:** Follow the focused RED -> GREEN -> verification -> review ->
> signed code commit -> exact-SHA ledger commit workflow.

**Goal:** Make ADR-0007 resolver middleware reachable from serde, CST, and
query consumers through public, config-aware entry points without changing the
behavior or structured parser errors of existing convenience APIs.

**Architecture:** ADR-0010 supersedes the unimplemented parts of ADR-0007 and
formalizes the shipped stateful per-event `Resolver` plus `ResolvedEvents`
adapter. `yamalgam_parser::ResolveError` is the shared resolved-event error
contract. Each consumer routes its ordinary parser path through
`ResolvedEvents<NoopResolver>` and exposes resolver-aware constructors or free
functions. Parse errors nested in `ResolveError::Parse` are normalized back to
the consumer's existing parse-error variant. Resolver failures retain their
typed variant. Existing tree-only, serde, and query entry points remain source
compatible.

**ADR:** `record/decisions/0010-use-stateful-event-resolver-adapters.md`.

**Tech stack:** Rust 2024, existing `yamalgam-parser` resolver types, existing
consumer error types, `cargo nextest`, `just check`.

---

## Task 1: Complete the shared resolver adapter contract

**Files:**

- Modify: `crates/yamalgam-parser/src/resolve.rs`
- Test: `crates/yamalgam-parser/src/resolve.rs`
- Modify: `crates/yamalgam-compose/src/lib.rs`
- Test: `crates/yamalgam-compose/tests/compose.rs`

1. Add failing coverage proving a structured clone of every `ResolveError`
   variant retains its variant and rendered message, with `Parse` retaining
   the concrete `ParseError` and I/O-backed variants retaining their error
   kind where applicable.
2. Add a failing resolver test proving config-aware dispatch observes the
   selected `LoaderConfig` while a legacy `on_event` implementation continues
   to work unchanged.
3. Add a `clone_structured` method that reconstructs non-cloneable error
   sources without flattening the outer resolver variant.
4. Add config storage and a config-aware callback to `ResolvedEvents`, with a
   compatibility default that delegates to `Resolver::on_event`.
5. Preserve the public two-argument `ResolvedEvents::new` constructor and add a
   config-aware constructor.
6. Publish `Composer::with_resolver_and_config`, then route composer default,
   unchecked, and selected-config resolver paths through the config-aware
   adapter constructor. Add tests proving moderate, unchecked, and an exact
   caller-selected profile reach the resolver.
7. Run the resolver and composer tests.

## Task 2: Add resolver-aware serde entry points

**Files:**

- Modify: `crates/yamalgam-serde/src/de.rs`
- Modify: `crates/yamalgam-serde/src/error.rs`
- Modify: `crates/yamalgam-serde/src/lib.rs`
- Test: `crates/yamalgam-serde/tests/`

1. Add failing integration tests with a resolver that rewrites a scalar before
   deserialization and a resolver that returns a typed custom failure.
2. Change the private event source to the shared `ResolveError` item type.
3. Add `Deserializer::with_resolver` and
   `Deserializer::with_resolver_and_config`, and route existing string
   constructors through `NoopResolver`.
4. Add single-document `from_str_with_resolver` and
   `from_str_with_resolver_and_config` free functions.
5. Normalize `ResolveError::Parse` to `Error::Parse`; preserve every other
   resolver failure as `Error::Resolve`, including across the structured error
   stash.
6. Run the complete serde test suite.

## Task 3: Add resolver-aware CST entry points

**Files:**

- Modify: `crates/yamalgam-cst/src/lib.rs`
- Test: `crates/yamalgam-cst/src/lib.rs`

1. Add failing tests proving a successful resolver rewrite changes observable
   CST token classification without changing source bytes. Add cases for event
   suppression, zero-width expansion, duplicate or reordered visible spans,
   and resolver failure; every case must reproduce the original source bytes.
2. Add a resolved-report type whose diagnostics use `ResolveError`, leaving
   the existing `CstReport<ParseError>` contract intact.
3. Add `CstBuilder::build_resolved_report` and resolver-aware tree/report
   convenience functions, including a config-aware form. Document that CST
   classification follows resolved events while token text remains the
   original source slice.
4. Make the CST builder own source coverage: emit source text instead of
   resolver-supplied comment text, and ignore visible events that begin before
   the already-covered offset. Suppressed or skipped source returns through
   gap or recovery tokens.
5. Establish the stream root before consuming resolved events, independent of
   whether the resolver passes through `StreamStart`. Test suppression of
   `StreamStart`, suppression of every event, and failure on the first event;
   each report must retain every source byte.
6. Route existing string convenience functions through `NoopResolver` while
   normalizing `ResolveError::Parse` back to existing `ParseError`
   diagnostics.
7. Run the complete CST suite and all 351 lossless round-trip cases.

## Task 4: Add resolver-aware query builder entry points

**Files:**

- Modify: `crates/yamalgam-query/src/build.rs`
- Modify: `crates/yamalgam-query/src/error.rs`
- Modify: `crates/yamalgam-query/src/lib.rs`
- Test: `crates/yamalgam-query/tests/value.rs`

1. Add failing integration tests proving a resolver rewrite reaches `YamlVal`
   construction and a resolver failure remains `BuildError::Resolve`.
2. Erase the private resolved-event iterator behind the existing `Documents`
   type so ordinary and custom resolver paths share one builder.
3. Add resolver-aware eager and incremental config-aware entry points, plus a
   moderate-default eager convenience function.
4. Normalize `ResolveError::Parse` to the existing `BuildError::Parse` variant
   and preserve non-parse resolver failures in a new `BuildError::Resolve`
   variant.
5. Run the complete query suite.

## Task 5: Verify, review, and record

1. Run `just fmt`.
2. Run focused resolver, serde, CST, and query tests.
3. Run `cargo nextest run -p yamalgam-compare --test cst_round_trip`.
4. Run `just check`.
5. Run `git diff --check` and inspect the complete diff for API compatibility,
   typed-error preservation, and unrelated changes.
6. Obtain independent code review and address every confirmed issue.
7. Create one focused signed code commit.
8. Append one immutable ledger entry for
   `resolver-middleware-cannot-reach-most-documented-consumers`, citing the
   exact full code SHA; update counters from fixed 11/open 16 to fixed 12/open
   15; obtain independent ledger review; create a separate signed ledger
   commit.

## Verification commands

```bash
cargo nextest run -p yamalgam-parser
cargo nextest run -p yamalgam-compose
cargo nextest run -p yamalgam-serde
cargo nextest run -p yamalgam-cst
cargo nextest run -p yamalgam-query
cargo nextest run -p yamalgam-compare --test cst_round_trip
just check
git --no-pager diff --check
```

# Unhandled Resolution Policy Implementation Plan

> **For Codex:** Follow the focused RED -> GREEN -> verification -> review ->
> signed code commit -> exact-SHA ledger commit workflow.

**Goal:** Make enabled `LoaderConfig::resolution` fail explicitly on every
ordinary high-level loading path until built-in resolvers ship, while preserving
custom policy-aware resolver paths.

**Architecture:** ADR-0011 assigns the boundary to `NoopResolver` inside the
shared config-aware `ResolvedEvents` adapter. A structured resolver error carries
the enabled include/ref flags. Resolver-origin failures terminate the adapter;
upstream parser recovery is unchanged. The compatibility CST report maps this
specific failure to an equivalent `ParseError` variant, while resolved reports
retain `ResolveError`.

**ADR:** `record/decisions/0011-reject-unhandled-resolution-policy.md`.

**Tech stack:** Rust 2024, existing resolver adapter and consumer error types,
`cargo nextest`, `just check`.

## Task 1: Establish the typed policy boundary

**Files:**

- Modify and test: `crates/yamalgam-core/src/loader.rs`
- Modify and test: `crates/yamalgam-parser/src/resolve.rs`
- Modify: `crates/yamalgam-parser/src/error.rs`

1. Add RED tests for include-only, ref-only, and combined enabled policies
   through `ResolvedEvents<NoopResolver>`.
2. Add `ResolutionPolicy::is_enabled` and a structured
   `ResolveError::UnsupportedResolutionPolicy` carrying both flags.
3. Override config-aware no-op dispatch to reject enabled policy.
4. Terminate `ResolvedEvents` after a resolver-origin error so the unsupported
   request yields one error; preserve continued upstream parse-error recovery.
5. Add the compatibility `ParseError` variant used by ordinary CST reports.
6. Prove a custom resolver still receives enabled policy and succeeds.

## Task 2: Prove every consumer exposes the failure

**Files:**

- Modify and test: `crates/yamalgam-compose/src/lib.rs`
- Modify and test: `crates/yamalgam-serde/tests/config.rs`
- Modify and test: `crates/yamalgam-cst/src/lib.rs`
- Modify and test: `crates/yamalgam-query/tests/value.rs`

1. Add RED integration coverage for ordinary configured composer, serde, CST,
   and query entry points with enabled resolution.
2. Map the no-op policy error into the compatibility CST diagnostic without
   changing `CstReport` or losing source bytes.
3. Assert every other consumer retains the structured resolver variant.
4. Assert a resolver-aware configured consumer can handle enabled policy.

## Task 3: Document, verify, review, and record

1. Update loader and resolver Rustdoc to distinguish policy-neutral parsing,
   rejecting no-op loading, and custom resolver responsibility.
2. Run focused core, parser, composer, serde, CST, and query tests.
3. Run all 351 CST round-trip cases and workspace clippy.
4. Run `just check` and `git --no-pager diff --check`.
5. Obtain independent review and address every confirmed issue.
6. Create one focused signed code-and-ADR commit.
7. Append one immutable ledger entry citing the exact full code SHA, update
   counters from fixed 19/open 8 to fixed 20/open 7, independently review the
   append, and create a separate signed ledger commit.

## Verification commands

```bash
cargo nextest run -p yamalgam-core
cargo nextest run -p yamalgam-parser
cargo nextest run -p yamalgam-compose
cargo nextest run -p yamalgam-serde
cargo nextest run -p yamalgam-cst
cargo nextest run -p yamalgam-query
cargo nextest run -p yamalgam-compare --test cst_round_trip
just check
git --no-pager diff --check
```

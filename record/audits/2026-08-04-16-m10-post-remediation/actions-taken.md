---
audit: 2026-08-04-16-m10-post-remediation
last_updated: 2026-08-08
status:
  fixed: 22
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 5
---

# Actions Taken: Full Repository Post-Remediation Audit

Summary of remediation status for the
[2026-08-04 full repository post-remediation audit](README.md).

---

## 2026-08-06 — Default composition bounded

**Disposition:** fixed
**Addresses:** [composer-default-path-has-unbounded-recursion](README.md#composer-default-path-has-unbounded-recursion)
**Commit:** 809f867b90dac17d34f43d30309331fbafa4abb0
**Author:** Codex

Ordinary composer string and event-stream constructors now apply
`LoaderConfig::default()`. Explicitly named unchecked entry points preserve
trusted-input compatibility without making unlimited loading the default.

The composer also enforces depth and collection limits locally, including raw
mapping pairs and materialized merge entries, so caller-supplied event streams
cannot bypass parser-side controls. Regression tests cover default and
unchecked APIs, exact custom-depth behavior, duplicate mapping entries,
sequences, and merged mappings.

---

## 2026-08-06 — Serde defaults bounded

**Disposition:** fixed
**Addresses:** [serde-default-disables-loader-limits](README.md#serde-default-disables-loader-limits)
**Commit:** 0c18a346c54ac3a001762562301ef1adfe48c0a2
**Author:** Codex

Ordinary serde string, reader, and document-stream entry points now apply
`LoaderConfig::default()`. Explicit unchecked entry points preserve the prior
unlimited behavior for trusted input.

The free functions share one single-document driver, so structured error
restoration and end-of-document checks remain consistent across default,
configured, and unchecked paths. Regression tests cover default rejection and
unchecked acceptance beyond the moderate depth limit.

---

## 2026-08-08 — CST construction and traversal bounded

**Disposition:** fixed
**Addresses:** [cst-recursive-traversals-have-no-depth-bound](README.md#cst-recursive-traversals-have-no-depth-bound)
**Commit:** 5a71b87c590c1fbdd72dbe233d9965959f1f0c78
**Author:** Codex

Ordinary CST convenience and builder constructors now apply moderate resource
limits, while configured and explicitly unchecked variants preserve caller
control. The builder enforces collection depth locally even for caller-supplied
unlimited parser events and preserves the complete source in the existing
recovery tree on failure.

Whole-tree text reconstruction, offset lookup, leaf counting, formatting, and
destruction now use iterative traversal. Regression tests cover moderate and
unchecked construction, the exact custom depth boundary, lossless local
recovery, and a 100,000-node publicly constructed CST in isolated processes.

---

## 2026-08-08 — Scalar mapping operations indexed

**Disposition:** fixed
**Addresses:** [composer-mapping-insertion-is-quadratic](README.md#composer-mapping-insertion-is-quadratic), [borrowed-value-get-allocates-key](README.md#borrowed-value-get-allocates-key)
**Commit:** 549fa0e6bd50262ee1bb7ba5065a1eefb5850a44
**Author:** Codex

`Mapping` retains its ordered entry vector as semantic state and lazily adds a
randomized fingerprint index for null, bool, integer, and string keys. Hash
bucket collisions compare the actual keys before lookup or replacement, while
float and collection keys preserve their existing linear `PartialEq`
semantics. Clone, equality, debug output, insertion order, and the const empty
constructor remain compatible.

Borrowed string lookup now hashes `&str` directly without constructing an
owned `String` or temporary `Value`. Focused tests cover duplicate replacement,
NaN, signed zero, collection keys, synthetic collisions, and absent scalar
lookups. Divan benchmarks cover insertion and lookup at fixed sizes through
10,000 entries.

---

## 2026-08-08 — Alias materialization size bounded

**Disposition:** fixed
**Addresses:** [composer-alias-deep-clone-is-size-unbudgeted](README.md#composer-alias-deep-clone-is-size-unbudgeted)
**Commit:** f064f64656937f8d4e378a7e4518c5c4a94113bb
**Author:** Codex

Composer anchors now cache an iterative recursive materialization weight that
accounts for every `Value` slot and owned UTF-8 string byte. Each alias charges
that cached weight to a checked stream-cumulative total and validates it
against `max_output_bytes` before cloning. The independent alias-count limit
remains enforced, and arithmetic overflow produces `ComposeError::LimitExceeded`.

Anchor names remain document-local, while materialized-byte accounting remains
cumulative across documents. Redefinition replaces both the cached value and
its weight without inflating the distinct-anchor count. Regression tests cover
exact output-budget boundaries, differently sized anchors, redefinition,
cross-document scope, unlimited configuration, overflow, and iterative weight
calculation over a 100,000-node public value.

---

## 2026-08-08 — Parser span probes preserve scanner errors

**Disposition:** fixed
**Addresses:** [flow-pair-finalization-swallows-scanner-errors](README.md#flow-pair-finalization-swallows-scanner-errors)
**Commit:** a4de07e4b02f6522b37d3cab7a99f842d31f07a2
**Author:** Codex

All seven optional-span probes now propagate `ParseError::Scan` and use a
default span only for genuine end-of-stream. Valid-input event ordering and
span construction remain unchanged, including the zero-width synthetic
`MappingEnd` span for implicit mappings in flow sequences.

The regression replaces the closing token of a valid implicit flow pair with
a typed scanner failure, proving finalization preserves
`ScanErrorKind::UnexpectedCharacter` rather than consuming it and later
reporting a generic parser error. Focused parser tests, the full workspace test
suite, clippy, and doc tests pass.

---

## 2026-08-08 — CST recovery diagnostics retained

**Disposition:** fixed
**Addresses:** [cst-discards-parse-error](README.md#cst-discards-parse-error)
**Commit:** 8c8e88646e5db1cf5593ae356a5d751bfe218547
**Author:** Codex

The new `CstReport` path returns the recovered, byte-lossless CST alongside its
original structured diagnostics. `CstBuilder::build_report` retains upstream
`ParseError` values and builder-generated resource-limit errors; bounded,
configured, and explicitly unchecked convenience functions expose the same
report contract.

Existing `CstBuilder::build` and `parse_to_cst*` functions remain compatible
tree-only recovery APIs. Regressions prove an unterminated scalar preserves its
typed scanner error and a builder-local depth failure preserves
`ParseError::LimitExceeded`, while both recovered trees reproduce their full
source. Focused CST tests, the full workspace test suite, clippy, and doc tests
pass.

---

## 2026-08-08 — Serde reader I/O errors preserved

**Disposition:** fixed
**Addresses:** [from-reader-erases-io-errors](README.md#from-reader-erases-io-errors)
**Commit:** ada1c434113bda59da03d6640145b5c9e7a10098
**Author:** Codex

Serde deserialization errors now include a dedicated `Error::Io` variant that
owns the original `std::io::Error`, preserves its `ErrorKind`, and exposes it
through `std::error::Error::source`. Both bounded and explicitly unchecked
reader entry points use the typed variant, while `Custom` remains reserved for
serde data-model failures and the existing reader error text remains stable.

The internal structured-clone path reconstructs I/O errors with the same kind
and rendered message instead of degrading them to `Custom`. Regressions cover
both reader entry points, source downcasting, error kind, display text, and the
clone path. Focused serde tests, the full workspace test suite, clippy, and doc
tests pass.

---

## 2026-08-08 — Configuration schema enforced

**Disposition:** fixed
**Addresses:** [config-template-advertises-silently-ignored-settings](README.md#config-template-advertises-silently-ignored-settings)
**Commit:** acd88bc19bc0cb8e332ddd4286ed39268b88362d
**Author:** Codex

The CLI configuration type now rejects unknown fields instead of silently
accepting misspellings and nonexistent capabilities. The shipped TOML and YAML
templates advertise only the consumed `log_level` and optional `log_dir`
settings plus the supported logging path environment overrides.

An end-to-end regression proves a misspelled `log_level` fails with an unknown
field diagnostic. A template contract test prevents the removed OpenTelemetry,
environment-mode, API, feature, telemetry, format, and color placeholders from
returning without matching implementation. Configuration integration tests,
the full workspace test suite, clippy, and doc tests pass.

---

## 2026-08-08 — Indentless sequence spans made synthetic

**Disposition:** fixed
**Addresses:** [lossless-cst-corrupts-eight-valid-flow-documents](README.md#lossless-cst-corrupts-eight-valid-flow-documents)
**Commit:** f0bf38f4adcad0e8f07ce62b86e6e502a6de75dc
**Author:** Codex

The eight valid documents were corrupted by indentless block-sequence boundary
events, not flow-collection closing events. Their synthetic `SequenceStart`
and `SequenceEnd` events reused the next visible token's span, so CST
reconstruction emitted the first block-entry dash twice and, for an explicit
mapping key, emitted the following value indicator twice.

Indentless sequence boundaries now use zero-width spans at the corresponding
source position, leaving the visible dash and colon spans owned solely by
`BlockEntry` and `ValueIndicator`. Parser regressions prove that ownership at
both boundaries. The complete round-trip allowlist was removed, all 351 valid
YAML Test Suite documents now round-trip byte-for-byte, and the full repository
check passes 1,896 tests plus clippy, dependency policy, and doc tests.

---

## 2026-08-08 — Resolver middleware exposed across consumers

**Disposition:** fixed
**Addresses:** [resolver-middleware-cannot-reach-most-documented-consumers](README.md#resolver-middleware-cannot-reach-most-documented-consumers)
**Commit:** 99d1983099cf6c5cf02c15fa4ddb275d27d1fdf5
**Author:** Codex

The parser's stateful event resolver now receives the selected loader
configuration, preserves structured resolver errors, and feeds shared public
resolver-aware entry points in composer, serde, CST, and query. Existing
resolver implementations remain source-compatible, ordinary constructors use
the no-op resolver, and configured and explicitly unchecked paths pass their
exact policy to middleware. ADR-0010 records the implemented callback-and-
adapter contract and supersedes the earlier stream-transformer design.

CST resolver paths retain the complete original source under suppression,
expansion, reordering, failure, and invalid visible spans. Nonzero spans are
validated for ordering, source bounds, and UTF-8 boundaries before slicing or
advancing byte coverage, while zero-width synthetic events remain supported.
Regressions cover every downstream consumer, exact configuration propagation,
typed failures, and lossless CST behavior. The full repository check and all
351 YAML Test Suite compliance cases pass.

---

## 2026-08-08 — Query builder subtree heights cached

**Disposition:** fixed
**Addresses:** [query-builder-rewalks-every-completed-node](README.md#query-builder-rewalks-every-completed-node)
**Commit:** 6ffeac4ef775036eefedbec282643adb6013a105
**Author:** Codex

The private query builder now carries each completed value's subtree height
through the existing recursive build. Scalars and empty collections have
height zero; nonempty sequences and mappings derive height from their surviving
children with checked arithmetic. Anchors cache the value-height pair, so an
alias validates its placement depth plus the cached target height without
allocating a traversal stack or revisiting completed descendants.

Mapping height bookkeeping remains aligned with final `IndexMap` entries:
duplicate keys retain the original key height and replace the value height, so
overwritten values cannot cause false depth failures. Same-name anchor
redefinition replaces both value and height without consuming another anchor
slot. Regressions prove the builder performs zero whole-tree depth walks and
cover exact boundaries for nested aliases, empty collections, mapping keys and
values, duplicate replacement, and anchor redefinition. All 126 query tests and
the full repository check pass.

---

## 2026-08-08 — Serde peek clones removed

**Disposition:** fixed
**Addresses:** [serde-peek-clones-owned-event](README.md#serde-peek-clones-owned-event)
**Commit:** fd023914d307b0e1cb2a218622afa9cb916ac7f1
**Author:** Codex

Unit deserialization now reduces a borrowed peek to a one-byte copyable
discriminant before consuming the original event. Null probing temporarily
moves the next event and restores it to the replay buffer, and enum
deserialization consumes its event directly. `EnumAccess` moves the variant
name into serde's seed instead of cloning the `Cow` and retains only the state
needed for variant payload access.

The former peek-clone-consume and scalar-Cow clone patterns are absent. A
custom resolver regression forces owned scalar payloads through unit, null and
non-null option, and enum paths, while the existing borrowed-string and anchor
coverage remains green. All 140 serde tests and the full repository check pass.

---

## 2026-08-08 — Borrowed tag resolvers accepted

**Disposition:** fixed
**Addresses:** [custom-tag-resolver-requires-static](README.md#custom-tag-resolver-requires-static)
**Commit:** 7097b1340e1a104dee890989ccdab6fdbff7f621
**Author:** Codex

The composer now stores its tag resolver as a concrete generic parameter with
`TagResolution` as the source-compatible default. Custom synchronous string
entry points no longer require `'static` and avoid the former trait-object box,
while the default, configured, and resolver-middleware constructors preserve
their existing APIs and selected resource policies.

A regression uses a resolver borrowing a stack-local lookup table through both
the bounded and explicitly unchecked custom entry points. All 58 composer tests,
workspace clippy, and the full repository check with 1,918 tests pass. The
change is a private storage and public-bound refinement, so no ADR was needed.

---

## 2026-08-08 — Mapping iteration standardized

**Disposition:** fixed
**Addresses:** [mapping-omits-intoiterator](README.md#mapping-omits-intoiterator)
**Commit:** e24479a23bfeeb8dc04d8cba28a81ccb19f79e61
**Author:** Codex

`Mapping` now implements `IntoIterator` for owned, shared-borrowed, and
mutable-borrowed values while preserving insertion order. Mutable iteration
exposes immutable keys and mutable values, matching standard map behavior and
preventing callers from invalidating the private scalar-key side index.

Regression coverage consumes a mapping, borrows it in a standard loop, mutates
values through `&mut Mapping`, and verifies indexed lookup remains correct.
All 68 core tests, workspace clippy, and the full repository check with 1,919
tests pass. This standard trait addition did not require an ADR.

---

## 2026-08-08 — Mapping default derived

**Disposition:** fixed
**Addresses:** [mapping-default-manually-matches-derive](README.md#mapping-default-manually-matches-derive)
**Commit:** 729f59bac3069ab88e5382675c22f16db409e21e
**Author:** Codex

`Mapping` now derives `Default`; the redundant manual implementation was
removed. Derived initialization remains exactly equivalent to `Mapping::new()`:
an empty entries vector and no lazily-created scalar index. The public const
constructor remains unchanged.

All 68 core tests, workspace clippy, and the full repository check with 1,919
tests pass. This behavior-preserving implementation cleanup did not require an
ADR.

---

## 2026-08-08 — Stream comparisons zipped

**Disposition:** fixed
**Addresses:** [compare-streams-use-index-loops](README.md#compare-streams-use-index-loops)
**Commit:** 0a6a61d37af58a12b9f66c4a78f421901547a9ab
**Author:** Codex

Token comparison and both event comparison modes now traverse their input
slices with `zip` and `enumerate`. The code retains the first mismatch index,
context window, and shorter-stream EOF reporting while cloning only the pair
that actually differs.

All 1,113 compare-crate tests, workspace clippy, and the full repository check
with 1,919 tests pass. This mechanical iterator refactor did not require an ADR.

---

## 2026-08-08 — Collection targets made explicit

**Disposition:** fixed
**Addresses:** [with-variables-collect-target-inferred](README.md#with-variables-collect-target-inferred)
**Commit:** e9dc31e2248652282794d79513434806e360f5b6
**Author:** Codex

`RunCtx::with_variables` now names its `Vec` collection target locally. The
analogous query string-splitting branches and CLI raw-input assembly do the
same, making each allocation shape visible without relying on a distant field
or sibling branch for inference.

All 220 focused query and CLI tests, workspace clippy, and the full repository
check with 1,919 tests pass. This type-annotation cleanup did not require an
ADR.

---

## 2026-08-08 — Unhandled resolution policy rejected

**Disposition:** fixed
**Addresses:** [loader-resolution-policy-is-entirely-unwired](README.md#loader-resolution-policy-is-entirely-unwired)
**Commit:** 37738175ffa35aca0ded64b83763fc1b2764d9b5
**Author:** Codex

`NoopResolver` now rejects enabled include or reference policy with a single
typed error instead of silently accepting an unsupported request. The shared
config-aware resolver adapter makes that boundary consistent across ordinary
composer, serde, CST, and query loading paths, while explicit custom resolvers
continue to receive and own enforcement of caller-selected policy. Resolver
failures terminate the adapter after one diagnostic; upstream parser recovery
retains its previous behavior.

ADR-0011 records the boundary and the deliberate pre-first-tag 0.1.0 source
break required to add typed parser and resolver diagnostics. Both public error
enums are now non-exhaustive, preventing later M11 variants from repeating that
break. The compatibility CST report mirrors the policy diagnostic without
changing its public type and remains byte-lossless.

All 527 affected core, parser, composer, serde, CST, and query tests, all 351
CST round-trip cases, workspace clippy, and the full repository check with
1,926 tests pass. Independent review found and verified the explicit public-
error compatibility treatment before commit.

---

## 2026-08-08 — Multi-document example compiled

**Disposition:** fixed
**Addresses:** [multidocument-rustdoc-example-is-never-compiled](README.md#multidocument-rustdoc-example-is-never-compiled)
**Commit:** 28af252aab5892420c63aeaacbeda267f5bbfcf0
**Author:** Codex

The public `Deserializer::documents` example now imports `Deserializer` and
uses a normal Rust code fence. Rustdoc therefore compiles and executes the
documented two-document collection contract instead of reporting the example
as ignored.

The focused serde doc-test run passes one test with zero ignored, and the full
repository check passes 1,926 workspace tests plus all doc tests. Independent
review confirmed the example is complete, public, and idiomatic. This
documentation test correction did not require an ADR.

---

## 2026-08-08 — Identity paths materialized on demand

**Disposition:** fixed
**Addresses:** [identity-inventory-allocates-path-node-per-map-key](README.md#identity-inventory-allocates-path-node-per-map-key)
**Commit:** c40f99da91299b1565bea3c82f87ba47be61d494
**Author:** Codex

Identity inventory traversal now keeps sequence indices and borrowed mapping
keys in a reusable private path-part vector. It materializes the public
`YamlVal` path encoding only when an alias is actually recorded, preserving
numeric indices, ordinary mapping-key components, and the existing
`{"$key": key}` encoding for aliases encountered in key position.

A test-only counter proves alias-free mapping traversal materializes zero path
nodes and that a nested encountered alias materializes exactly its three output
components. Existing mapping-key alias coverage preserves the public format.
All 128 query tests, workspace clippy, and the full repository check with 1,927
tests pass. Independent review confirmed the borrowed lifetimes and traversal
semantics. This private hot-path refactor did not require an ADR.

---

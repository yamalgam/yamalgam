---
audit: 2026-08-04-16-m10-post-remediation
last_updated: 2026-08-08
status:
  fixed: 10
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 17
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

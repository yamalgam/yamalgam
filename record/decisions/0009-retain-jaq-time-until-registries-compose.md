---
status: accepted
date: 2026-08-03
decision-makers: [Clay Loveless]
consulted: [Codex M10 audit]
---

# 0009: Retain jaq time until native registries are composable

## Context and Problem Statement

`jaq-std` enables `jiff` and time-zone data through its `time` feature. The yg
query spec does not require date filters. Can we drop this feature while
keeping the format, log, math, and regex filters that yg already exposes?

In jaq-std 3.0.1, we cannot do that through its public API. The public `funs()`
registry needs all six default features, including `time`. Each non-time extra
registry is private. Disabling `time` therefore removes access to every extra
native filter and makes yamalgam-query fail to compile.

## Decision Drivers

- Existing query tests use supplementary jaq filters.
- Copying jaq's private registries would create local upstream code.
- A fork adds a new source and update path for one feature split.
- The 171 KiB `jiff` cost is real on every target.
- Windows builds may also bundle time-zone data.

## Considered Options

- Disable `time` and keep calling the aggregate `funs()` registry.
- Use only jaq's public `base_funs()` registry.
- Fork or copy jaq-std to expose each extra registry.
- Keep the full feature set explicit until upstream makes registries composable.

## Decision Outcome

Chosen option: "Keep the full feature set explicit." This preserves the jq
surface that yg already tests. The manifest now names `time` as a required cost
of jaq-std 3.0.1's aggregate registry, not an unnoticed default.

We will revisit this choice when jaq-std exposes public feature-specific
registries or an aggregate registry that does not require `time`.

### Consequences

- Good, because yg keeps its tested format, log, math, and regex filters.
- Good, because we avoid a fork and do not copy private upstream code.
- Good, because the manifest makes the full feature choice visible.
- Bad, because `jiff` remains in every shipped `yg` binary.
- Bad, because Windows builds may still bundle the time-zone database.
- Bad, because users can reach date filters outside yg's written query spec.

### Confirmation

- `jaq-std` must resolve the six named features without its Cargo default flag.
- All query and jq-manual tests must pass.
- `cargo tree -p yamalgam -i jiff -e normal` must show only the jaq-std edge.
- Revisit after a jaq-std release makes the extra registries public or
  feature-composable.

## Pros and Cons of the Options

### Disable time and keep `funs()`

- Good, because it would remove `jiff` with one manifest edit.
- Bad, because jaq-std does not compile that API without `time`.

### Use only `base_funs()`

- Good, because it compiles without `time`.
- Bad, because it drops all supplementary native filters, not just dates.

### Fork or copy jaq-std

- Good, because it can produce the exact filter set yg wants.
- Bad, because it creates an ongoing upstream sync burden.
- Bad, because it expands remediation far beyond a feature edit.

### Keep the complete feature set explicit

- Good, because it preserves behavior with no new dependency source.
- Bad, because it accepts the measured binary and time-zone cost.

## More Information

- [M10 audit finding](../audits/2026-08-03-21-m10-fresh/README.md#jaq-std-time-feature-bundles-tzdb)
- [Attempted remediation plan](../superpowers/plans/2026-08-03-jaq-time-feature.md)
- [yg query language specification](../../docs/spec/yg-query-language-spec-draft-01.md)

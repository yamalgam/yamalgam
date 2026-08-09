# Post-Remediation Loading and Composition Design

**Date:** 2026-08-06
**Status:** Accepted

## Overview

This design closes the latest audit's loading and composition findings. It
keeps the ordered `Value` model and the low-level scanner/parser contract. It
also keeps unlimited behavior out of ordinary convenience APIs. Owned-tree
consumers enforce moderate defaults locally. Trusted callers retain explicit
unchecked paths.

## Context

The post-remediation audit found existing resource controls. Several public
convenience paths bypass them before they build recursive owned trees. Wide
mappings and aliases also add CPU and memory costs after parsing. Parser
configuration alone cannot make these consumers safe. This campaign covers
five significant findings and one coupled advisory:

- `composer-default-path-has-unbounded-recursion`
- `serde-default-disables-loader-limits`
- `cst-recursive-traversals-have-no-depth-bound`
- `composer-mapping-insertion-is-quadratic`
- `composer-alias-deep-clone-is-size-unbudgeted`
- `borrowed-value-get-allocates-key`

The remaining audit findings stay open for later campaigns.

### Baseline and integrity

The campaign starts in `/Users/clay/source/claylo/yamalgam` on branch
`m10-yg-cli` at `be2bc8aee369b327dcfdef187ce604d99b40fc17`. The source audit is
`record/audits/2026-08-04-16-m10-post-remediation/`.

`README.md`, `report.html`, `findings.yaml`, `recon.yaml`,
`reviewer-verdicts.yaml`, `AGENTS.md`, and `assets/` in that directory remain
immutable. The only mutable audit artifact is the append-only
`actions-taken.md`, created after the first completed action.

No worktree will be created. Existing untracked artifacts from
`record/audits/2026-08-03-21-m10-fresh/` remain untouched and unstaged.

## Approach

We enforce each limit at the layer that incurs the cost. High-level APIs select
the moderate policy. Composers and CST builders enforce structural limits even
for caller-supplied event streams. Iterative tree operations remove stack
dependence from public values. We retain the ordered models and add only private
accounting or indexes.

## Alternatives considered

### Safe convenience APIs only

Route composer, serde, and CST string helpers through `LoaderConfig::default()`.
Leave their internal builders unchanged. This small change protects only the
standard parser path. Custom event iterators could still drive unbounded
composer/CST recursion. The structural findings would remain open.

### Structural limits with explicit unchecked escape hatches (chosen)

Make public high-level constructors moderate by default. Add explicit unchecked
variants for trusted-input compatibility. Enforce the relevant limits inside
the owned-tree consumers. This follows ADR-0006: safe defaults are the ordinary
path and unlimited behavior is opt-in. It preserves the existing architecture
and needs no new dependency.

### Immediate representation rewrite

Replace recursive owned trees with arenas or shared immutable nodes. Make the
composer and CST fully iterative. This could remove several costs at once. It
would also change public representation and equality/clone behavior far beyond
the audit findings. It is not proportionate for this campaign.

## Detailed design

### Constructor policy

`LoaderConfig::default()` remains the single safe-default policy. High-level
composer and serde string/reader entry points use it. Their previous unlimited
behavior remains available through explicit `*_unchecked` functions.

`Composer::new(events)` and `CstBuilder::new(source)` also become moderate by
default. Callers can feed them external iterators that still reach recursive
owned trees. Each constructor gains a `new_unchecked` counterpart. Existing
`new_with_config` constructors remain the path for strict, trusted, or custom
limits.

`Scanner::new` and `Parser::new` remain unchanged. Their current tests document
an unlimited low-level compatibility contract. Changing that contract is
outside these findings. Safe consumers must use `Parser::with_config`. They
cannot assume an upstream iterator enforced their limits.

### Composer depth and collection enforcement

`compose_node` carries an explicit depth beginning at zero for a document root.
It checks the configured maximum on entry and uses `checked_add(1)` before
descending into sequence items, mapping keys, or mapping values. Overflow and
limit failures become `ComposeError::LimitExceeded`.

The composer also checks `max_collection_size` while it adds sequence items and
mapping pairs. This includes merged mappings. These checks remain in the
composer even with a configured upstream parser. Parser depth does not bound
the recursive Rust calls that consume an arbitrary event iterator.

### Serde safe defaults

`yamalgam_serde::from_str`, `from_reader`, and
`Deserializer::from_str` use `LoaderConfig::default()`. The existing
`from_str_with_config` path remains unchanged. New `from_str_unchecked` entry
points preserve the old all-`None` limit profile for callers that explicitly
accept it.

Regression tests cover input beyond the moderate depth limit. The default free
function and streaming deserializer must return a structured limit error. The
configured path must still honor caller-selected limits.

### CST bounded construction and stack-safe operations

`parse_to_cst` and `CstBuilder::new` use moderate limits. Config-aware and
unchecked variants preserve caller control. The builder tracks nesting only for
mapping and sequence nodes; stream, document, and entry wrappers do not increase
YAML depth. It uses checked arithmetic and validates the configured maximum
before opening each collection.

On a local depth failure, the builder adds a `NodeKind::Error` node containing
the untouched source slice from `last_offset` through end of input. It then
stops consuming events. The error node keeps the existing lossless `to_text`
contract without changing `build` to return `Result`.

Publicly constructible `CstNode` values can still be deeper than any builder
limit, so the three whole-tree operations become iterative:

- `to_text` uses an explicit LIFO worklist while preserving source order;
- `node_at_offset` performs depth-first traversal with child order preserved;
- `leaf_count` counts tokens with the same worklist pattern.

`CstNode` receives a safe custom `Drop` implementation. It drains descendant
children into a worklist before each empty node drops. This removes recursive
destruction without `unsafe` code or a representation change.

### Ordered mapping index

`Mapping` keeps its insertion-order `Vec<(Value, Value)>` as the source of truth.
An internal randomized fingerprint index accelerates equality-safe scalar keys:
null, bool, integer, and string. Float, sequence, and mapping keys retain the
linear fallback because their current `PartialEq` behavior is not a sound
`Eq + Hash` contract.

The private index initializes on the first indexable insertion. This preserves
the existing `const Mapping::new()` API and avoids index state for empty maps.

The index maps a randomized scalar fingerprint to one or more entry positions.
It resolves collisions by comparing the actual keys in the ordered vector.
Duplicate insertion replaces the existing value without moving the entry.
Clone, equality, and debug behavior continue to describe the ordered entries.
The private index is not semantic state.

The same index backs borrowed string lookup. `Value::get(&str)` no longer
allocates a temporary `String`. One implementation action therefore resolves
the significant insertion finding and the advisory lookup finding. The
unrelated `IntoIterator` and `Default` notes remain separate.

### Alias materialization budget

Composer anchors store the owned `Value` plus a cached recursive byte weight.
Before cloning an alias, the composer adds that weight to a checked total. It
validates the total against `max_output_bytes`. The existing alias-count limit
remains in force. Count bounds small repeated aliases. Cumulative materialized
size bounds large aliases.

This is an internal composer accounting change. It does not change public
`Value::Clone` semantics or add a new field to the public `ResourceLimits`
struct. Overflow is reported as `ComposeError::LimitExceeded` before cloning.

## Testing and action cadence

Each action follows red-green-refactor with `cargo nextest`. We run the smallest
affected crate target before broader checks. No production change lands before
its focused regression test fails for the expected reason.

The implementation order is:

1. Composer safe defaults, local depth, and collection checks.
2. Serde safe defaults and explicit unchecked entry points.
3. CST bounded construction, iterative traversal, and iterative drop.
4. Mapping scalar index and borrowed string lookup.
5. Size-weighted alias materialization budget.

Each implementation receives a focused conventional code commit. Its ledger
entry records that real code SHA in a separate audit-only commit. One entry may
address both mapping slugs because the same index resolves both mechanisms. We
do not bundle other findings merely because they share a surface.

Focused tests run after every action. The affected crate suite and clippy run
before the code commit. `just check` runs after the loading/composition campaign
and again before the full audit remediation is declared complete.

## Non-goals

- Replacing `Value` or `CstNode` with arenas, `Rc`, or another public model.
- Changing `Scanner::new` or `Parser::new` compatibility semantics.
- Implementing resolver policy, CST error diagnostics, or the known flow-token
  round-trip fixes in this campaign.
- Combining unrelated API ergonomics or style findings into these commits.
- Editing any immutable source-audit artifact.

## Consequences

We gain bounded ordinary loading paths and local enforcement for custom event
sources. CST operations become stack-safe. Scalar mapping lookup becomes nearly
constant-time. Aliases gain a materialization budget. The public ordered value
and CST models stay intact. The campaign adds no dependency.

We add explicit unchecked APIs and private state in `Mapping` and the composer.
Scalar fingerprint collisions require equality checks. Non-scalar mapping keys
retain linear lookup. CST construction must preserve lossless recovery when it
rejects excessive depth. This adds tests at the builder boundary.

We defer public representation changes, resolver policy, known CST flow-token
diagnostics, and the remaining performance findings. Changing the low-level
scanner/parser constructor contract also remains outside this campaign.

## Related decisions

- [ADR-0006: LoaderConfig for resource limits and security policy](../../../record/decisions/0006-loaderconfig-for-resource-limits-and-security-policy.md)

This design introduces three candidate decisions for separate ADRs. They are
safe constructor defaults with unchecked escape hatches, a private randomized
scalar mapping index, and alias accounting against `max_output_bytes`. We will
create ADRs only if requested. The implementation plan can proceed from this
campaign design after it is accepted.

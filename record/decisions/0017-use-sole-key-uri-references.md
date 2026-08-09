---
status: accepted
date: 2026-08-08
decision-makers: [Clay Loveless]
consulted: [Codex M11 design]
---

# 0017: Replace sole-key URI references with RFC 6901 targets

## Context and Problem Statement

YAML has no standard cross-resource reference feature. A `$ref` mapping can
name a local file, a remote URL, a whole document, or one node. Other systems
disagree about sibling keys and fragment behavior.

What `$ref` contract gives Yamalgam predictable substitution without inventing
merge semantics?

## Decision Drivers

- Use existing URI-reference and JSON Pointer standards.
- Preserve YAML-native scalar and collection types.
- Resolve relative targets from the containing resource.
- Reject ambiguous sibling behavior.
- Parse every target with Yamalgam and the active limits.
- Detect cycles across resources and fragments.

## Considered Options

- Treat `$ref` as a string macro and leave fragments application-defined.
- Merge sibling keys into the referenced mapping.
- Ignore siblings as annotations.
- Replace only a sole-key `$ref` mapping and reject every ambiguous form.

## Decision Outcome

Chosen option: replace only a sole-key `$ref` mapping.

The mapping must contain exactly one `$ref` entry. Its value must be a string
URI reference. Siblings are errors. Yamalgam never merges or discards them.

The fragmentless resource is the fetch and parse identity. It must contain
exactly one valid YAML document. No fragment, or an empty fragment, selects the
document root. An RFC 6901 fragment selects a mapping value by string key or a
sequence item by index. RFC 6901 cannot address non-string YAML mapping keys.

The selected node replaces the `$ref` mapping and keeps its native YAML type.
Nested references resolve in the same session. Relative file references use
the containing file. Relative remote references use the containing URL. HTTP
and HTTPS work only when policy allows their scheme and host.

The cycle key is the canonical resource plus fragment. One session may select
different fragments from a cached document. It rejects a chain that returns to
the same selected target.

Missing pointers, malformed fragments, invalid YAML, empty streams, multiple
documents, policy denials, and cycles are hard errors. A selected fragment that
depends on an anchor outside its subtree is also an error.

### Consequences

- Good, because URI references and RFC 6901 define familiar behavior.
- Good, because targets keep mappings, sequences, tags, and scalar types.
- Good, because sibling mistakes fail instead of producing an implicit merge.
- Good, because local and remote relative references share one rule.
- Bad, because users cannot annotate or override a `$ref` with sibling keys.
- Bad, because RFC 6901 cannot select non-string YAML mapping keys.
- Bad, because each target must be a complete single-document YAML resource.

### Confirmation

- Whole-document, empty-fragment, and nested-pointer tests preserve node types.
- Sibling mappings and non-string `$ref` values fail.
- File and URL references resolve relative to their containing resource.
- Same-resource fragment reuse hits the cache without hiding cycles.
- External-anchor fragment dependencies fail as non-self-contained.

## Pros and Cons of the Options

### Merge siblings

- Good, because local overrides are concise.
- Bad, because merge order and non-mapping targets become ambiguous.

### Ignore siblings

- Good, because annotation keys remain possible.
- Bad, because misspelled or meaningful data can disappear silently.

### Require a sole-key mapping

- Good, because substitution has one meaning for every target type.
- Good, because mistakes fail at the directive site.
- Bad, because overrides require a separate explicit operation.

## More Information

- [M11 external resource resolution design](../superpowers/specs/2026-08-08-m11-external-resource-resolution-design.md).
- [Draft-01 external resolution](../../docs/spec/yg-query-language-spec-draft-01.md#72-ref-resolution).
- [ADR-0015: Pluggable retrievers and resolution sessions](0015-pluggable-retrievers-with-shared-resolution-sessions.md).

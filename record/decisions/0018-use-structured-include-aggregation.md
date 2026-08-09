---
status: accepted
date: 2026-08-08
decision-makers: [Clay Loveless]
consulted: [Codex M11 design]
---

# 0018: Use structured includes for deterministic typed aggregation

## Context and Problem Statement

A scalar `!include file.yaml` handles node substitution but not common
composition work. Users also need mapping merges, sequence splicing, directory
or glob aggregation, and raw text. Several tools solve this with preprocessors
that accept syntax outside YAML.

How should Yamalgam support those forms while keeping every input valid YAML?

## Decision Drivers

- Keep scalar node substitution concise.
- Support mappings, sequences, collections, and text without a second grammar.
- Make file ordering and key derivation deterministic.
- Reject type mismatches and conflicts unless policy names a resolution.
- Apply the same sandbox, cycle, edge, and byte limits to every matched file.
- Preserve normal YAML merge-key precedence.

## Considered Options

- Support only scalar node substitution.
- Add a line-oriented preprocessor for standalone include statements.
- Define separate tags for every include behavior.
- Use one structured `!include` mapping with typed modes and options.

## Decision Outcome

Chosen option: keep scalar shorthand and add one structured include form.

`!include path.yaml` substitutes one YAML document root. `<<: !include
defaults.yaml` merges a mapping through valid YAML merge-key syntax. Explicit
entries in the containing mapping override included entries.

A structured include accepts exactly one of `path`, `glob`, or `dir`. Its
`format` is `yaml` or `text`. Its mode is `node`, `list`, `named`, `merge`, or
`splice`.

- `node` substitutes one matched root.
- `list` returns one result per match.
- `named` maps filename stems or extensionless relative paths to results.
- `merge` combines homogeneous mappings or homogeneous sequences.
- `splice` replaces one sequence item with items from sequence roots.

Directory selectors may set `recursive` and filename `match` patterns. Named
mode may select `stem` or `relative` keys. Mapping merge may choose `error`,
`first`, or `last` for conflicts among included roots. Error is the default.

Text format accepts valid UTF-8 and supports `node`, `list`, and `named` only.
YAML format requires exactly one valid document per resource. Sorting uses
normalized root-relative path bytes. Zero matches, duplicate names, mixed root
types, invalid option combinations, and wrong insertion contexts are errors.

Yamalgam does not accept a standalone include line between mapping entries.
That form is not YAML. It would force the scanner, parser, CST, and diagnostics
to understand a second language.

### Consequences

- Good, because common composition forms remain valid YAML.
- Good, because one options shape covers files, globs, directories, and text.
- Good, because deterministic ordering makes CI and generated output stable.
- Good, because every aggregation validates its insertion and root types.
- Bad, because the structured form is more verbose than preprocessor syntax.
- Bad, because strict conflict and zero-match errors reject permissive habits.
- Bad, because aggregation requires buffering matched resources before output.

### Confirmation

- Scalar, merge-key, list, named, merge, splice, and text tests cover success.
- Glob and directory tests prove stable ordering across enumeration order.
- Conflict, duplicate-name, zero-match, mixed-type, and context tests fail.
- Every matched path passes the shared filesystem policy and session budgets.
- Source files remain valid YAML before resolution.

## Pros and Cons of the Options

### Scalar substitution only

- Good, because the feature remains small.
- Bad, because callers must build collection composition outside Yamalgam.

### Add preprocessor syntax

- Good, because standalone include lines can look concise.
- Bad, because the input stops being YAML.
- Bad, because source spans and CST fidelity become ambiguous.

### Use structured modes

- Good, because behavior and type expectations are explicit.
- Good, because new source selectors can reuse one contract.
- Bad, because option validation adds a larger error surface.

## More Information

- [M11 external resource resolution design](../superpowers/specs/2026-08-08-m11-external-resource-resolution-design.md).
- [ADR-0016: Three resolution modes and security boundaries](0016-use-three-mode-resolution-policy.md).

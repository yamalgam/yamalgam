# Tracey Spec Version Migration Design

## Date: 2026-03-08

## Goal

Track the full evolution of the YAML specification from 1.0 (2004) through 1.2.2 (2021)
using tracey's versioning system. A single tracey spec (`yaml`, prefix `y`) spans all
five published spec versions. The version suffix on each requirement marker records
when that requirement was last revised.

## Version mapping

| Tracey version | YAML spec | Date | Source format |
|---|---|---|---|
| v1 (implicit) | 1.0 | 2004-01-29 | Markdown (have) |
| v2 (`+2`) | 1.1 | 2005-01-18 | Markdown (have) |
| v3 (`+3`) | 1.2.0 | 2009-07-21 | HTML (need to convert) |
| v4 (`+4`) | 1.2.1 | 2009-10-01 | HTML (need to convert) |
| v5 (`+5`) | 1.2.2 | 2021-10-01 | Markdown (have, 459 markers) |

## Versioning semantics

A requirement's version bumps to N **only when its text changes** in spec version N.
Unchanged requirements keep their previous version number.

- If `c-printable` is identical in 1.0 through 1.2.2, it stays `y[char.c-printable]` (v1) in all files.
- If `c-printable` changed in 1.1 but not since, all files from 1.1 onward use `y[char.c-printable+2]`.
- If it changed again in 1.2.2, the 1.2.2 file uses `y[char.c-printable+5]`.
- Tracey tracks the highest version; code references the version it implements.
- Tracey's stale detection flags code that references an older version than the current spec.

## File structure

```
docs/spec/
  yaml-1.0/spec-1.0.md          # v1 markers
  yaml-1.1/spec-1.1.md           # v1 or v2 markers
  yaml-1.2.0/spec-1.2.0.md       # v1, v2, or v3 markers
  yaml-1.2.1/spec-1.2.1.md       # v1, v2, v3, or v4 markers
  yaml-1.2.2/ch05-...md (8 files) # v1 through v5 markers
```

Each spec file is marked up with ALL requirements present in that version.
Tracey silently deduplicates same-ID same-version markers across files.
Different-version markers for the same ID coexist, with tracey tracking the highest.

## Tracey config

Single `yaml` spec with all five directories in `include`. Config unchanged from
the original plan (prefix `y` inferred from markers).

## Requirement ID naming

Use the existing 1.2.2 naming convention across all versions. The 459 existing IDs
serve as the canonical namespace. Where 1.0 or 1.1 has a concept that doesn't map
to a 1.2.2 ID (removed feature, different decomposition), create IDs following
the same `category.subcategory.name` pattern.

Categories: `char`, `struct`, `flow`, `block`, `doc`, `schema`, `model`, `overview`,
`intro`, `syntax`.

## Key cross-version differences

### 1.0 -> 1.1 (major restructuring)
- 1.0 Chapter 4 "Syntax" → 1.1 Chapters 5-10 (split into characters, primitives, stream, nodes, scalars, collections)
- Different production numbering
- Added escape sequences (`\e`, `\/`, `\N`, `\L`, `\P`)
- Added NEL (0x85) as line break
- Added merge key `<<`
- Explicit type system with tag repository

### 1.1 -> 1.2.0 (spec revision)
- Chapter reorganization (1.1 Ch7 Stream/Ch8 Nodes/Ch9 Scalars/Ch10 Collections → 1.2 Ch6 Structural/Ch7 Flow/Ch8 Block/Ch9 Documents)
- Removed NEL/LS/PS as line breaks (restricted to CR/LF/CRLF)
- Removed merge key from core spec (moved to type repository)
- Added JSON compatibility requirements
- Added Chapter 10: Recommended Schemas (Core, JSON, Failsafe)
- Changed boolean resolution (removed yes/no/on/off)
- Removed sexagesimal integers, octal with leading 0

### 1.2.0 -> 1.2.1 (errata)
- Bug fixes and clarifications only
- No production changes expected

### 1.2.1 -> 1.2.2 (revision)
- Copyright transfer to YAML Language Development Team
- Clarified wording throughout
- Added explicit JSON compatibility notes
- Minor production refinements

## Execution approach

1. Convert 1.2.0 and 1.2.1 HTML specs to markdown (pandoc + cleanup)
2. Mark up 1.2.0 and 1.2.1 (structurally near-identical to 1.2.2, mechanical)
3. Compare 1.2.x versions to determine version bumps (mostly errata, few bumps)
4. Mark up 1.1 (different chapter structure, requires conceptual mapping)
5. Compare 1.1 → 1.2.0 for version bumps (the big diff)
6. Mark up 1.0 (very different structure, fewer formal productions)
7. Compare 1.0 → 1.1 for version bumps
8. Bump existing 1.2.2 markers to correct versions (v1 through v5)
9. Update tracey config
10. Add `y[impl ...]` code annotations to scanner/parser, starting with Ch5 (characters)
11. Verify with `tracey query status`

## Naming fix needed

Three existing 1.2.2 markers have `InvalidNaming` errors (contain `1.1` and `1.2` literals):
- `struct.yaml-directive.must-accept-1.1`
- `struct.yaml-directive.must-accept-1.2`
- `struct.yaml-directive.should-process-1.1-as-1.2`

These need renaming to avoid the naming violation (e.g., `struct.yaml-directive.must-accept-v11`).

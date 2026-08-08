# Tracey Version Migration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Track the full evolution of the YAML specification from 1.0 to 1.2.2 using tracey versioning, then annotate implementation code.

**Architecture:** Single tracey spec (`yaml`, prefix `y`) spanning five published spec versions. Version suffix on each requirement marker records when that requirement was last normatively revised: v1=1.0, v2=1.1, v3=1.2.0, v4=1.2.1. 1.2.2 has NO normative changes so no marker reaches v5 — markers stay at whatever version they were last revised.

**Tech Stack:** tracey v1.3.0, diff/grep for cross-referencing

**Key reference:** `~/source/reference/yaml-spec/spec/1.2.2/ext/changes.md` — documents all normative changes between versions.

---

## Phase 1: File Setup — COMPLETE

All five spec versions are in place. Tracey config updated. Naming errors fixed. `tracey query validate` = 0 errors.

```
docs/spec/
  yaml-1.0/spec-1.0.md       # from docs/test/1.0/index.md
  yaml-1.1/spec-1.1.md       # already had
  yaml-1.2.0/spec-1.2.0.md   # from docs/test/1.2.0/index.md
  yaml-1.2.1/spec-1.2.1.md   # from docs/test/1.2.1/index.md
  yaml-1.2.2/*.md             # 8 chapter files, 459 markers
```

Naming fixes applied:
- `struct.yaml-directive.must-accept-1.1` → `struct.yaml-directive.must-accept-prior`
- `struct.yaml-directive.must-accept-1.2` → `struct.yaml-directive.must-accept-current`
- `struct.yaml-directive.should-process-1.1-as-1.2` → `struct.yaml-directive.should-process-prior-as-current`

Cleanup: removed `docs/test/`, `vendor/yaml-spec-1.1/clean-spec.pl`, `vendor/yaml-spec-1.1/spec-1.1.md`

---

## Phase 2: Determine Version Bumps for 1.2.2 Markers

This phase identifies which of the 459 existing markers need version bumps. No marker reaches v5 since 1.2.2 has no normative changes.

### Task 6: Classify 1.2.2 markers by change origin

**Reference:** `~/source/reference/yaml-spec/spec/1.2.2/ext/changes.md`

**Key changes 1.1 → 1.2.0 (v2 → v3 bumps):**
- Line breaks: NEL/LS/PS removed → affects `char.b-*`, `char.line-break.*`
- Booleans/numbers: Core schema replaces type library → affects `schema.*`
- JSON compatibility added → affects `char.set.json-compat`, `char.nb-json`, `flow.c-s-implicit-json-key`, etc.
- Tag shorthand restrictions (no `,[]{}`) → affects `char.ns-tag-char`, `struct.ns-anchor-char`
- Anchor restrictions → affects `struct.ns-anchor-char`
- `\/` escape added → affects `char.c-ns-esc-char`
- No space after `:` in flow with quoted key → affects `flow.c-ns-flow-map-adjacent-value`
- 1024-char key limit removed → affects `flow.implicit-key.must-limit-1024`
- Documents independent → affects `doc.*`
- Explicit doc-end before directive → affects `doc.*`
- Chapter 10 (Schemas) entirely new in 1.2 → all `schema.*` markers are v3

**Key changes 1.2.0 → 1.2.1 (v3 → v4 bumps):**
- `ns-plain-first(c)` lookahead fix → `flow.ns-plain-first`
- `ns-plain-safe(c)` scope change → `flow.ns-plain-safe-in`, `flow.ns-plain-safe-out`
- `ns-plain-char(c)` lookahead fix → `flow.ns-plain-char`
- `c-ns-flow-map-separate-value` lookahead fix → `flow.c-ns-flow-map-separate-value`
- `b-chomped-last` EOF fix → `block.b-chomped-last`

**Step 1:** Create a version assignment file at `docs/spec/marker-versions.txt`:
```
# Format: marker_id version reason
# v1 = unchanged since 1.0
# v2 = changed in 1.1
# v3 = changed in 1.2.0
# v4 = changed in 1.2.1
```

**Step 2:** For each of the 459 markers, determine the correct version by cross-referencing:
- The changes.md document
- Diffing 1.2.0 vs 1.2.2 markdown (for production-level changes)
- Checking whether the concept existed in 1.0/1.1

**Step 3:** Group markers by category and assign versions in bulk where possible:
- All `schema.*` markers → v3 (schemas new in 1.2.0)
- All `intro.*` and `overview.*` markers → likely v1 (informational, stable)
- `char.*` markers → check individually (line breaks changed in 1.2.0)
- `struct.*` markers → check individually (tags/anchors changed)
- `flow.*` markers → check individually (plain scalar fixes in 1.2.1)
- `block.*` markers → mostly v1, except `b-chomped-last` (v4)
- `doc.*` markers → check individually (document independence changed in 1.2.0)
- `model.*` markers → likely v1 (process model stable)
- `syntax.*` markers → likely v1 (production conventions stable)

### Task 7: Apply version bumps to 1.2.2 markers

**Files:**
- Modify: all 8 files in `docs/spec/yaml-1.2.2/`

**Step 1:** For each marker that needs bumping, change `y[foo]` to `y[foo+N]` where N is the determined version.

Example: `y[char.c-printable]` — if this production is identical across all versions, stays as `y[char.c-printable]` (v1). If it changed in 1.2.0 (NEL removal), becomes `y[char.c-printable+3]`.

**Step 2:** Run `tracey query status` — should show 459 requirements, 0 covered.

**Step 3:** Commit
```bash
git add docs/spec/yaml-1.2.2/
git commit -m "chore(spec): apply tracey version bumps to 1.2.2 markers"
```

---

## Phase 3: Mark Up 1.2.0 and 1.2.1

These are structurally identical to 1.2.2 (same chapter layout, same production names/numbers). The markup is largely mechanical — copy 1.2.2 markers, adjust versions.

### Task 8: Mark up 1.2.0 spec

**Files:**
- Modify: `docs/spec/yaml-1.2.0/spec-1.2.0.md`

**Step 1:** For each requirement in the 1.2.2 spec that exists in 1.2.0, add the marker with version = min(marker_version, 3). If a marker's version is v4 (changed in 1.2.1), the 1.2.0 file uses v3 (the pre-errata version).

**Step 2:** Requirements new in 1.2.1 or 1.2.2 → don't add to 1.2.0.

**Step 3:** Run `tracey query status` — requirement count should stay at 459.

### Task 9: Mark up 1.2.1 spec

**Files:**
- Modify: `docs/spec/yaml-1.2.1/spec-1.2.1.md`

Same approach as Task 8 but versions go up to v4. 1.2.1 has the errata fixes applied.

### Task 10: Commit Phase 3

```bash
git add docs/spec/yaml-1.2.0/ docs/spec/yaml-1.2.1/
git commit -m "chore(spec): mark up YAML 1.2.0 and 1.2.1 specs with tracey markers"
```

---

## Phase 4: Mark Up 1.1

The 1.1 spec has a different chapter structure from 1.2.x. This requires conceptual mapping.

### Task 11: Map 1.1 chapters to 1.2.2 requirement IDs

**Chapter mapping:**
| 1.1 Chapter | 1.2.2 Chapter | Notes |
|---|---|---|
| Ch5: Characters | Ch5: Character Productions | Direct mapping, production names mostly match |
| Ch6: Syntax Primitives | Ch6: Structural Productions (partial) | Indentation, comments, separation |
| Ch7: YAML Character Stream | Ch6 (directives), Ch9 (documents) | Split across two 1.2 chapters |
| Ch8: Nodes | Ch6 (properties), Ch7/Ch8 (flow/block nodes) | Split across three 1.2 chapters |
| Ch9: Scalar Styles | Ch7 (flow scalars), Ch8 (block scalars) | Split into flow/block |
| Ch10: Collection Styles | Ch7 (flow collections), Ch8 (block collections) | Split into flow/block |

**Step 1:** Create a cross-reference mapping of 1.1 production numbers → 1.2.2 requirement IDs.

**Step 2:** Identify 1.1-only concepts (no 1.2.2 equivalent):
- Merge key `<<` support
- Value key `=` support
- Sexagesimal integers
- Octal with leading 0
- Boolean yes/no/on/off
- NEL/LS/PS as line breaks
- !!pairs, !!omap, !!set, !!timestamp, !!binary types

For these, create NEW requirement IDs with a `v11-only.` prefix or similar.

### Task 12: Mark up 1.1 spec

**Files:**
- Modify: `docs/spec/yaml-1.1/spec-1.1.md`

**Step 1:** Add markers for each production and prose requirement.
- Requirements unchanged from 1.0: use v1 (`y[foo]`)
- Requirements new or changed in 1.1: use v2 (`y[foo+2]`)

**Step 2:** Cross-reference with the 1.0 spec to determine which are v1 vs v2. The changes.md lists 1.0→1.1 changes: tag refactoring, `\t` escape, `\^` removal, directive separator change.

**Step 3:** Run `tracey query status`.

### Task 13: Commit Phase 4

```bash
git add docs/spec/yaml-1.1/
git commit -m "chore(spec): mark up YAML 1.1 spec with tracey markers"
```

---

## Phase 5: Mark Up 1.0

The 1.0 spec is the most different structurally — everything is in Chapter 4 "Syntax" with less formal BNF.

### Task 14: Mark up 1.0 spec

**Files:**
- Modify: `docs/spec/yaml-1.0/spec-1.0.md`

**Step 1:** All markers use v1 (`y[foo]`) — this is the baseline.

**Step 2:** Map 1.0 concepts to 1.2.2 requirement IDs where possible.

**Step 3:** For concepts unique to 1.0 (removed in 1.1), create new IDs.

**Step 4:** Run `tracey query status`.

### Task 15: Commit Phase 5

```bash
git add docs/spec/yaml-1.0/
git commit -m "chore(spec): mark up YAML 1.0 spec with tracey markers"
```

---

## Phase 6: Code Annotations

Add `y[impl ...]` markers to scanner and parser code, linking implementation to spec requirements.

### Task 16: Annotate scanner — character productions (Ch5)

**Files:**
- Modify: `crates/yamalgam-scanner/src/scanner.rs` (and any sub-modules)

**Step 1:** For each character-related function/method, add `// y[impl char.xxx]` comments linking to the relevant requirement.

Focus areas:
- `scan_to_next_token` → `y[impl char.s-white]`, `y[impl char.b-break]`, etc.
- `scan_uri_escapes` → `y[impl char.ns-uri-char]`
- `scan_tag` → `y[impl char.c-tag]`, `y[impl char.ns-tag-char]`
- Escape sequence handling → `y[impl char.c-ns-esc-char]`, `y[impl char.ns-esc-*]`

### Task 17: Annotate scanner — structural productions (Ch6)

Focus: indentation, comments, separation, directives, tags, anchors.

### Task 18: Annotate scanner — flow productions (Ch7)

Focus: flow collections, quoted scalars, plain scalars.

### Task 19: Annotate scanner — block productions (Ch8)

Focus: block scalars, block collections.

### Task 20: Annotate parser — all productions

**Files:**
- Modify: `crates/yamalgam-parser/src/parser.rs`

Focus: document stream (Ch9), node construction, state machine mapping to productions.

### Task 21: Annotate core — schema/resolver (Ch10)

**Files:**
- Modify: `crates/yamalgam-core/src/tag.rs` (resolve_plain_scalar)
- Modify: `crates/yamalgam-core/src/value.rs`

Focus: Core Schema tag resolution, type detection.

### Task 22: Commit code annotations

```bash
git add crates/
git commit -m "chore: add tracey y[impl] annotations to scanner, parser, and core"
```

---

## Phase 7: Verify and Finalize

### Task 23: Run full tracey validation

**Step 1:** `tracey query validate` — expect 0 errors
**Step 2:** `tracey query status` — check coverage percentages
**Step 3:** `tracey query uncovered` — review what's not yet covered (expected: many requirements won't have impl annotations yet)
**Step 4:** `tracey query stale` — should be empty (no stale references)

### Task 24: Update MEMORY.md

Update the tracey section in memory with:
- Version mapping (v1-v4, no v5)
- File structure for all 5 spec versions
- Marker count per spec version
- Coverage percentages
- Naming convention for 1.1-only concepts

### Task 25: Final commit and handoff

```bash
git add .
git commit -m "chore: complete tracey spec version migration"
```

---

## Execution notes

- **Phases 1-3 are mechanical** and can be largely parallelized with subagents.
- **Phase 4 (1.1 markup) is the hardest** due to chapter structure differences. Requires careful cross-referencing.
- **Phase 5 (1.0 markup) is medium difficulty** — less formal spec, but baseline so all markers are v1.
- **Phase 6 (code annotations) is independent** of the spec markup and can start after Phase 2 (once version bumps are applied to 1.2.2).
- **The changes.md document** at `~/source/reference/yaml-spec/spec/1.2.2/ext/changes.md` is the primary cross-reference source for version bumps.
- **The 1.2 markdown spec** at `~/source/reference/yaml-spec/spec/1.2/markdown/spec.md` can serve as a reference for 1.2.0 content (it's the "3rd Edition, Patched" = 1.2.1).

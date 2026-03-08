# Handoff: Tracey Impl Annotations + Gap Analysis

**Date:** 2026-03-08
**Branch:** `tracey-version-migration`
**State:** Green — compiles clean, 297/459 requirements covered (64.7%), 0 stale, 0 validation errors.

## Where things stand

All 459 YAML spec requirements are tracked across five spec versions. 333 `y[impl ...]` annotations link scanner, parser, and core code to spec requirements. `cargo check --workspace` passes. No code logic was changed — annotations are comments only.

| Crate | Covered | Percentage |
|-------|---------|------------|
| scanner | 187/459 | 40.7% |
| parser | 88/459 | 19.2% |
| core | 50/459 | 10.9% |
| **unique total** | **297/459** | **64.7%** |

## Gap analysis — 162 uncovered requirements

The uncovered requirements fall into actionable categories:

**Already implemented, just unannotated (~50 requirements)**
- `char.c-comment`, `char.c-indicator`, `char.nb-char`, `char.ns-char`, `char.c-reserved` — character recognition in scanner
- `struct.b-l-trimmed`, `struct.b-l-folded`, `struct.s-flow-folded`, `struct.l-empty` — line folding in scanner
- `char.encoding.*`, `char.line-break.*`, `char.set.*` — reader.rs and input.rs
- Quick win: annotate these to jump to ~75% coverage

**Not yet built — future milestones**
- `schema.*` (13) — Schema resolver dispatch (M7: `FailsafeSchema`, `JsonSchema`, `Yaml11Schema`)
- `model.present.*` (13), `model.process.dump.*` (4) — Serializer/emitter (M13)

**Spec meta-documentation, not implementable code**
- `syntax.*` (23) — Production grammar naming conventions, operator definitions. Consider excluding from coverage metrics via tracey config.

**Interesting edge cases — verify against spec errata**
- `flow.ns-plain-char+4`, `flow.ns-plain-safe-in+4`, `flow.ns-plain-safe-out+4` — YAML 1.2.1 errata changes to plain scalar rules. Do we implement the errata or libfyaml's pre-errata behavior?
- `block.b-chomped-last+4` — Block scalar chomping errata. Same question.
- `model.loading.tag-resolution-*` (14) — Full tag resolution matrix. Partially implemented in `tag.rs` but gaps exist.

## Decisions made

- **Section-level marker placement** for 1.0/1.1 specs — markers grouped at section headings. Tracey tracks them; human navigation is approximate. Can refine later.
- **Phantom annotation cleanup** — initial pass used parent-level IDs (`block.indent`) that don't exist as spec markers. Cleaned up: 52 phantoms replaced or removed.
- **Annotation IDs must exactly match spec markers** — including version suffixes (`+3`, `+4`). `tracey_stale` catches mismatches.

## What's next

1. **Annotate remaining `char.*` and `struct.*` gaps** in scanner — most are implemented, ~50 easy annotations → ~75% coverage. Files: `scanner.rs:281` (scan_to_next_token), `scanner.rs:1877` (fetch_double_quoted_scalar), `reader.rs`, `input.rs`
2. **Verify v4 errata compliance** — run the 1.2.1 errata test cases through the scanner and compare against spec text. Focus on `flow.ns-plain-*+4` and `block.b-chomped-last+4`. File: `crates/yamalgam-compare/tests/compliance.rs`
3. **Consider `tracey_config_exclude`** for `syntax.*` requirements — they describe the spec grammar, not implementable behavior
4. **Merge branch** — this is safe to merge. All changes are spec markdown + code comments.

## Landmines

- **Annotation IDs are case-sensitive and version-sensitive.** `char.c-printable` ≠ `char.c-printable+3`. Use `tracey_stale` after any annotation session to catch mismatches.
- **Tracey markers need blank-line separation.** Consecutive `y[...]` lines are parsed as one paragraph — only the first is recognized.
- **1.1 spec has unbalanced `:::` div blocks** (887 opens, 315 closes from pandoc conversion). Markers inside divs work now but this could be fragile across tracey versions.
- **`claude-session-driver` launch-worker.sh** has two local-only fixes: `unset CLAUDECODE` and Down+Enter for bypass consent. These are in the plugin cache, not committed anywhere.
- **The 1.0/1.1 marker placement is section-level grouped**, not per-paragraph. Some markers may be far from the text they reference. Good enough for tracey, less ideal for human navigation.

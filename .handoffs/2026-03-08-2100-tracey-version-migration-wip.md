# Handoff: Tracey Spec Version Migration (WIP)

**Date:** 2026-03-08
**Branch:** `tracey-version-migration` (2 commits ahead of `main`)
**State:** Yellow — version bumps committed but 1.2.0/1.2.1 markup blocked by conversion artifact.

## Where things stand

The tracey version migration is partially complete. The 459 existing 1.2.2 markers have been classified and version-bumped (349 v1, 29 v2, 74 v3, 7 v4) across two commits. All five spec versions are in `docs/spec/`. The tracey config includes all five directories.

The current blocker: the html-to-markdown converted 1.2.0 and 1.2.1 spec files have unbalanced code fences (109 each, odd) that break tracey's markdown parser. Tracey enters an unclosed code block around line 563 of each file and can't see markers after that point (only 29 of 81 markers are visible).

## Decisions made

- **Each requirement ID appears in exactly one file** — the spec version where it was last normatively changed. Tracey rejects duplicate rule bases across files, even at different versions (`DuplicateRequirement` error). This means 1.2.2 has zero markers (no normative changes), 1.2.0 has 74, 1.2.1 has 7.
- **No v5** — YAML 1.2.2 explicitly states "no normative changes from YAML 1.2." Version bumps stop at v4 (1.2.1 errata).
- **Version = last normative change**, not spec era. A requirement unchanged since 1.0 stays v1 even in the 1.2.2 file. See design doc: `docs/plans/2026-03-08-tracey-version-migration-design.md`.
- **Renamed three markers** for tracey naming compliance: `must-accept-1.1` → `must-accept-prior`, `must-accept-1.2` → `must-accept-current`, `should-process-1.1-as-1.2` → `should-process-prior-as-current`.

## What's next

1. **Fix code fence balance in 1.2.0/1.2.1 spec files.** The html-to-markdown conversion produced 109 standalone ``` ``` ``` lines (odd = unbalanced). The root cause is chapter 2 examples: code blocks within table cells where opening/closing fences don't pair. Run `python3 -c "..."` fence-tracking script (in session history) to find exact break points. Fix by properly closing or removing the orphan fences. Both files have the same pattern.
2. **Verify tracey sees all 81 markers** (74 in 1.2.0 + 7 in 1.2.1) after fence fix. Target: `tracey query status` shows 81 requirements.
3. **Restore 1.2.2 markers.** The 1.2.2 chapter files currently have zero markers (stripped during redistribution). They need all 459 markers restored since each marker appears in exactly ONE file — the 349 v1 markers belong in 1.0 (not yet marked up), the 29 v2 in 1.1 (not yet marked up). Until 1.0 and 1.1 are marked up, the v1/v2 markers need to stay in 1.2.2. Restore from git: `git checkout af3740a -- docs/spec/yaml-1.2.2/`.
4. **Mark up 1.1 spec** (29 v2 markers) — then remove those 29 from 1.2.2.
5. **Mark up 1.0 spec** (349 v1 markers) — then remove those 349 from 1.2.2.
6. **Add `y[impl ...]` code annotations** to scanner/parser/core.

## Landmines

- **1.2.2 markers are currently ALL GONE** from uncommitted working tree. The version-bumped markers exist in commit `af3740a`. Restore with `git checkout af3740a -- docs/spec/yaml-1.2.2/` before doing anything else.
- **Tracey `DuplicateRequirement` error** fires on same rule base across files even with different version suffixes (e.g., `foo+3` in 1.2.0 and `foo+4` in 1.2.1). Each ID must appear in exactly ONE file.
- **Tracey silent duplicate handling** — same ID at same version across files is silently deduplicated (one dropped, no error). Different versions of same ID IS an error. Both are documented behaviors despite conflicting with what the versioning docs imply.
- **html-to-markdown conversion artifacts** — The `- |` list-table prefix (from `<li>` blocks) was already removed. The remaining issue is unbalanced ``` ``` ``` fences from example code blocks in chapter 2 tables. Line 563 opens a fence that pairs with a double-fence on line 591, leaving the file in an open code block state.
- **The `spec-1.2.0.md` and `spec-1.2.1.md` files are deleted** — Clay re-converted with different filenames (`spec.md`). The old truncated versions (1478/2060 lines) are gone. The new complete versions (4499/4511 lines) are the working files.
- **`docs/spec/yaml-1.2.0/` and `yaml-1.2.1/` now contain full spec repos** (HTML, images, CSS, etc.) from Clay's conversion. Only `spec.md` is tracked by tracey; the rest should probably be gitignored or removed.

# Agent Briefing — M11 external Resolution changes from c85f7e0 through 246f707

You are in a `cased` audit output directory. This file exists to help you pick
up remediation work without thrashing. Read it once, then act.

**Audit:** `2026-08-09-1537-m11-post-remediation`
**Date:** 2026-08-09
**Findings:** 11 total

## Files in this directory

- `README.md` — authored narrative report (markdown, GitHub-rendered companion to report.html). Read-only for remediation work.
- `report.html` — interactive rendered report (primary deliverable). Read-only.
- `findings.yaml` — structured findings (source for the build). Read-only.
- `recon.yaml` — structural model. Read-only.
- `assets/` — generated sparkline SVGs. Don't edit.
- `actions-taken.md` — append-only remediation ledger. May not exist yet;
  create it the first time you log an action.
- `AGENTS.md` — this file.

## The loop

For each finding you address:

1. Find it in `README.md` or `report.html` by its slug. Anchors match the slug
   exactly; every finding is pre-listed in the index below so you don't need
   to grep.
2. Read the concern, location, and remediation text.
3. Make the code change in the target repository.
4. Append one entry to `actions-taken.md`. **One entry per action**, even
   when a single action resolves multiple findings — put every slug it
   addresses in the `Addresses` field.

## `actions-taken.md` format

YAML front matter plus chronological markdown entries. Front matter is
mandatory; update `last_updated` and the `status` counts every time you
add an entry. The `open` count is `11 - (fixed + mitigated +
accepted + disputed + deferred)`.

```markdown
---
audit: 2026-08-09-1537-m11-post-remediation
last_updated: YYYY-MM-DD
status:
  fixed: 0
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 11
---

# Actions Taken: M11 external Resolution changes from c85f7e0 through 246f707

Summary of remediation status for the [2026-08-09 M11 external-resolution changes from c85f7e0 through 246f707 audit](README.md).

---

## YYYY-MM-DD — brief description of the action

**Disposition:** fixed
**Addresses:** [finding-slug](README.md#finding-slug)
**Commit:** {SHA or PR link}
**Author:** {who did the work}

One to three paragraphs describing what changed, in which files, and why
this approach. If the disposition is `accepted` or `disputed`, the rationale
must be here. If `deferred`, include the target date or milestone.
```

## Dispositions

- `fixed` — code change deployed; commit SHA or PR link required
- `mitigated` — compensating control in place; root cause remains; explain
  the residual risk
- `accepted` — risk acknowledged; rationale mandatory (who decided, why).
  This is not a euphemism for "ignored"
- `disputed` — finding contested with evidence; not a dismissal. The
  original finding stays in `README.md`; this entry records the counterargument
- `deferred` — scheduled for later; target date or milestone reference
  required. A deferred finding without a target is an accepted finding in
  disguise

## What you must not do

- Do not edit `README.md`, `report.html`, `findings.yaml`, `recon.yaml`, or
  anything in `assets/`. They are the audit artifact and must stay immutable.
- Do not edit past `actions-taken.md` entries. The file is append-only. If
  a previous action is superseded, add a new entry referencing the old one.
- Do not invent finding slugs. Use the ones in the index below, verbatim.
- Do not create an empty `actions-taken.md` until you have at least one
  action to log.

## Finding index

Every finding in this audit. Use these exact slugs in the `Addresses` field
of your `actions-taken.md` entries.

### The Resolution Policy and Graph Integrity Surface

- `relative-includes-can-retrieve-non-file-schemes` (significant) — `crates/yamalgam-resolve/src/session.rs:1277-1292`
- `explicit-only-sessions-skip-root-anchor-reservation` (significant) — `crates/yamalgam-resolve/src/session.rs:99-120`
- `builtin-resolver-mixes-caller-and-session-configs` (significant) — `crates/yamalgam-resolve/src/builtin.rs:58-90`

### The Session Budgets and Discovery Surface

- `resolution-cache-bypasses-session-byte-budget` (significant) — `crates/yamalgam-resolve/src/session.rs:870-899`
- `glob-discovery-scans-entire-base` (moderate) — `crates/yamalgam-resolve/src/retriever/file.rs:373-388`
- `symlink-aliases-consume-match-limit-before-deduplication` (moderate) — `crates/yamalgam-resolve/src/retriever/file.rs:179-197`

### The Query Runtime and Reporting Surface

- `caught-resolution-errors-accumulate-until-run-end` (moderate) — `crates/yamalgam-query/src/runtime_resolution.rs:142-185`
- `post-resolution-conversion-drops-session-report` (moderate) — `crates/yamalgam-query/src/resolution.rs:119-137`
- `direct-include-preflight-can-omit-the-failed-report-edge` (moderate) — `crates/yamalgam-resolve/src/session.rs:408-435`
- `resolved-owned-tree-conversion-remains-recursive` (significant) — `crates/yamalgam-query/src/resolution.rs:323-342`
- `query-session-assignment-recurses-without-a-bound` (significant) — `crates/yamalgam-query/src/resolution.rs:248-268`

## If you have the `cased` skill loaded

Invoke it. The skill's Phase 5 covers remediation tracking with the full
schema reference and worked examples. This briefing exists for the case
where you land in the directory without the skill available.

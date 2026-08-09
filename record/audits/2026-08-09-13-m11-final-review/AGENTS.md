# Agent Briefing — M11 external Resolution changes from c85f7e0 through 56b8b3e

You are in a `cased` audit output directory. This file exists to help you pick
up remediation work without thrashing. Read it once, then act.

**Audit:** `2026-08-09-13-m11-final-review`
**Date:** 2026-08-09
**Findings:** 20 total

## Files in this directory

- `README.md`        — authored narrative report (markdown, GitHub-rendered companion to report.html). Read-only for remediation work.
- `report.html`      — interactive rendered report (primary deliverable). Read-only.
- `findings.yaml`    — structured findings (source for the build). Read-only.
- `recon.yaml`       — structural model. Read-only.
- `assets/`          — generated sparkline SVGs. Don't edit.
- `actions-taken.md` — append-only remediation ledger. May not exist yet;
  create it the first time you log an action.
- `AGENTS.md`        — this file.

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
add an entry. The `open` count is `20 - (fixed + mitigated +
accepted + disputed + deferred)`.

```markdown
---
audit: 2026-08-09-13-m11-final-review
last_updated: YYYY-MM-DD
status:
  fixed: 0
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 20
---

# Actions Taken: M11 external Resolution changes from c85f7e0 through 56b8b3e

Summary of remediation status for the [2026-08-09 M11 external-resolution changes from c85f7e0 through 56b8b3e audit](README.md).

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

### Resource Identity and Policy Dispatch

- `scheme-relative-ref-becomes-local-file` (significant) — `crates/yamalgam-resolve/src/resource.rs:275-282`
- `include-uri-bypasses-relative-selector-contract` (moderate) — `crates/yamalgam-resolve/src/include.rs:252-259`
- `resource-id-public-variants-bypass-canonical-invariants` (moderate) — `crates/yamalgam-resolve/src/resource.rs:16-24`

### Filesystem Retrieval and Discovery

- `file-policy-checks-race-path-reopen` (significant) — `crates/yamalgam-resolve/src/retriever/file.rs:219-247`
- `symlink-discovery-skips-alias-policy` (significant) — `crates/yamalgam-resolve/src/retriever/file.rs:181-191`
- `recursive-file-discovery-has-no-depth-bound` (moderate) — `crates/yamalgam-resolve/src/retriever/file.rs:181-200`
- `symlink-aliases-make-glob-results-enumeration-dependent` (moderate) — `crates/yamalgam-resolve/src/retriever/file.rs:158-188`
- `discovery-results-do-not-enforce-deterministic-order` (moderate) — `crates/yamalgam-resolve/src/session.rs:575-590`

### YAML Anchor and Alias Integrity

- `imported-anchor-prefix-can-collide-with-root` (significant) — `crates/yamalgam-resolve/src/session.rs:1055-1056`
- `fragment-self-containment-ignores-anchor-binding-order` (moderate) — `crates/yamalgam-resolve/src/anchors.rs:8-33`

### Session Identity and Query Runtime

- `run-local-resolution-keys-collide-across-contexts` (significant) — `crates/yamalgam-query/src/engine.rs:142-153`
- `slurp-drops-resolution-base` (moderate) — `crates/yamalgam/src/commands/query.rs:605-625`
- `caught-resolution-error-remains-terminal` (moderate) — `crates/yamalgam-query/src/runtime_resolution.rs:140-149`
- `query-resolution-conversion-recurses-without-configured-bound` (significant) — `crates/yamalgam-query/src/resolution.rs:388-411`

### Resolution Reporting

- `automatic-resolution-failure-discards-report` (significant) — `crates/yamalgam-query/src/resolution.rs:111-115`
- `resolution-report-retains-raw-secret-reference` (significant) — `crates/yamalgam-resolve/src/report.rs:118-134`
- `collection-report-never-records-matches` (moderate) — `crates/yamalgam-resolve/src/session.rs:599-611`

### Resolution Efficiency

- `owned-nodes-clone-resource-identities-per-node` (significant) — `crates/yamalgam-resolve/src/document.rs:204-208`
- `include-mapping-merge-is-quadratic` (significant) — `crates/yamalgam-resolve/src/include.rs:498-511`

### Dependency Hygiene

- `resolver-directly-depends-on-unused-scanner` (advisory) — `crates/yamalgam-resolve/Cargo.toml:21`

## If you have the `cased` skill loaded

Invoke it. The skill's Phase 5 covers remediation tracking with the full
schema reference and worked examples. This briefing exists for the case
where you land in the directory without the skill available.

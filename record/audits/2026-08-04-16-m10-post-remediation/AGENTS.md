# Agent Briefing — Full Repository Post Remediation

You are in a `cased` audit output directory. This file exists to help you pick
up remediation work without thrashing. Read it once, then act.

**Audit:** `2026-08-04-16-m10-post-remediation`
**Date:** 2026-08-04
**Findings:** 27 total

## Files in this directory

- `README.md`        — authored narrative report (markdown, GitHub-rendered companion to report.html). Read-only for remediation work.
- `report.html` — interactive rendered report (primary deliverable). Read-only.
- `findings.yaml`    — structured findings (source for the build). Read-only.
- `recon.yaml` — structural model. Read-only.
- `assets/` — generated sparkline SVGs. Don't edit.
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
add an entry. The `open` count is `27 - (fixed + mitigated +
accepted + disputed + deferred)`.

```markdown
---
audit: 2026-08-04-16-m10-post-remediation
last_updated: YYYY-MM-DD
status:
  fixed: 0
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 27
---

# Actions Taken: Full Repository Post Remediation

Summary of remediation status for the [2026-08-04 Full Repository Post-Remediation audit](README.md).

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

### The Loading and Composition Surface

- `composer-default-path-has-unbounded-recursion` (significant) — `crates/yamalgam-compose/src/lib.rs:272-285`
- `serde-default-disables-loader-limits` (significant) — `crates/yamalgam-serde/src/de.rs:52-59`
- `cst-recursive-traversals-have-no-depth-bound` (significant) — `crates/yamalgam-cst/src/lib.rs:141-152`
- `composer-mapping-insertion-is-quadratic` (significant) — `crates/yamalgam-core/src/value.rs:67-77`
- `composer-alias-deep-clone-is-size-unbudgeted` (significant) — `crates/yamalgam-compose/src/lib.rs:343-355`

### The Error Preservation Surface

- `flow-pair-finalization-swallows-scanner-errors` (moderate) — `crates/yamalgam-parser/src/parser.rs:1167-1179`
- `cst-discards-parse-error` (moderate) — `crates/yamalgam-cst/src/lib.rs:548`
- `from-reader-erases-io-errors` (moderate) — `crates/yamalgam-serde/src/lib.rs:84-91`

### The Public API and Type Surface

- `borrowed-value-get-allocates-key` (advisory) — `crates/yamalgam-core/src/value.rs:128-134`
- `serde-peek-clones-owned-event` (advisory) — `crates/yamalgam-serde/src/de.rs:647-656`
- `custom-tag-resolver-requires-static` (advisory) — `crates/yamalgam-compose/src/lib.rs:106-123`
- `mapping-omits-intoiterator` (note) — `crates/yamalgam-core/src/value.rs:86-99`
- `mapping-default-manually-matches-derive` (note) — `crates/yamalgam-core/src/value.rs:114-118`
- `compare-streams-use-index-loops` (note) — `crates/yamalgam-compare/src/compare.rs:66-77`
- `with-variables-collect-target-inferred` (note) — `crates/yamalgam-query/src/engine.rs:103-107`

### The Feature Completeness Surface

- `resolver-middleware-cannot-reach-most-documented-consumers` (significant) — `crates/yamalgam-serde/src/de.rs:22-28`
- `loader-resolution-policy-is-entirely-unwired` (advisory) — `crates/yamalgam-core/src/loader.rs:32-48`
- `lossless-cst-corrupts-eight-valid-flow-documents` (significant) — `crates/yamalgam-compare/tests/cst_round_trip.rs:13-34`
- `config-template-advertises-silently-ignored-settings` (moderate) — `config/yamalgam.toml.example:33-52`
- `multidocument-rustdoc-example-is-never-compiled` (note) — `crates/yamalgam-serde/src/de.rs:87-100`

### The Runtime Performance Surface

- `query-builder-rewalks-every-completed-node` (significant) — `crates/yamalgam-query/src/value.rs:210-230`
- `identity-inventory-allocates-path-node-per-map-key` (advisory) — `crates/yamalgam-query/src/value.rs:763-780`
- `string-range-retains-intermediate-string` (advisory) — `crates/yamalgam-query/src/valt.rs:135-150`
- `parser-consumers-erase-concrete-hot-path-types` (note) — `crates/yamalgam-serde/src/de.rs:22-26`
- `scanner-error-branches-remain-in-hot-functions` (note) — `crates/yamalgam-scanner/src/scanner.rs:354-375`

### The Supply Chain Surface

- `xtask-unused-serde-and-toml` (note) — `xtask/Cargo.toml:14-15`
- `yamlstar-pulls-unmaintained-dlopen-derive` (advisory) — `.config/deny.toml:55-58`

## If you have the `cased` skill loaded

Invoke it. The skill's Phase 5 covers remediation tracking with the full
schema reference and worked examples. This briefing exists for the case
where you land in the directory without the skill available.

# Agent Briefing — M10 surface — the yamalgam Query crate and the yg binary, as introduced across main...m10 Yg Cli (21 commits, ~5k lines of new src, ~1.3k lines of tests)

You are in a `cased` audit output directory. This file exists to help you pick
up remediation work without thrashing. Read it once, then act.

**Audit:** `.`
**Date:** 2026-08-03
**Findings:** 36 total

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
add an entry. The `open` count is `36 - (fixed + mitigated +
accepted + disputed + deferred)`.

```markdown
---
audit: .
last_updated: YYYY-MM-DD
status:
  fixed: 0
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 36
---

# Actions Taken: M10 surface — the yamalgam Query crate and the yg binary, as introduced across main...m10 Yg Cli (21 commits, ~5k lines of new src, ~1.3k lines of tests)

Summary of remediation status for the [2026-08-03 M10 surface — the yamalgam-query crate and the yg binary, as introduced across main...m10-yg-cli (21 commits, ~5k lines of new src, ~1.3k lines of tests) audit](README.md).

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

### The YAML Round-Trip Surface

- `flow-context-plain-scalar-injection` (critical) — `crates/yamalgam-query/src/value.rs:705-727`
- `alias-metadata-survives-mutation-dropping-edits` (critical) — `crates/yamalgam-query/src/valt.rs:199-205`
- `emitter-quoting-ignores-active-schema` (significant) — `crates/yamalgam-query/src/value.rs:705-713`
- `json-nonstring-keys-collide` (significant) — `crates/yamalgam-query/src/emit/json.rs:76-81`
- `plain-scalar-control-characters` (moderate) — `crates/yamalgam-query/src/value.rs:717-726`
- `stale-raw-metadata-after-string-splice` (moderate) — `crates/yamalgam-query/src/valt.rs:300-307`

### The In-Place Editing Surface

- `in-place-empty-output-truncates-file` (critical) — `crates/yamalgam/src/commands/query.rs:172-203`
- `in-place-replaces-symlinks-and-drops-ownership` (moderate) — `crates/yamalgam/src/commands/query.rs:190-203`
- `in-place-multi-file-partial-application` (moderate) — `crates/yamalgam/src/commands/query.rs:152-157`
- `slurp-silently-per-file-under-in-place` (moderate) — `crates/yamalgam/src/commands/query.rs:154-157`
- `in-place-with-from-file-rejected-by-clap` (moderate) — `crates/yamalgam/src/lib.rs:111-114`

### The Resource Bounds Surface

- `yaml-bomb-materialized-by-emitters` (critical) — `crates/yamalgam-query/src/build.rs:154-171`
- `filter-constructed-values-recurse-without-depth-bound` (significant) — `crates/yamalgam-query/src/emit/json.rs:24-35`
- `map-index-allocates-directly-from-untrusted-index` (significant) — `crates/yamalgam-query/src/valt.rs:178-187`
- `fromjson-ignores-configured-limits` (moderate) — `crates/yamalgam-query/src/funs.rs:312-328`

### The Process Contract Surface

- `null-input-still-blocks-on-stdin` (significant) — `crates/yamalgam/src/commands/query.rs:242-249`
- `stdout-epipe-becomes-runtime-error` (moderate) — `crates/yamalgam/src/commands/query.rs:110-122`
- `exit-code-mapping-diverges-from-spec` (moderate) — `crates/yamalgam/src/commands/query.rs:389-396`
- `halt-prints-a-spurious-error-line` (moderate) — `crates/yamalgam/src/commands/query.rs:380-383`
- `runtime-errors-drop-document-and-file-identity` (moderate) — `crates/yamalgam-query/src/engine.rs:180-187`

### The Filter Parameterization Surface

- `no-argument-binding-forces-filter-concatenation` (moderate) — `crates/yamalgam-query/src/engine.rs:129-137`
- `env-filter-registered-unconditionally` (advisory) — `crates/yamalgam-query/src/funs.rs:11-22`

### The Value Model Surface

- `no-public-read-access-to-yamlval` (advisory) — `crates/yamalgam-query/src/value.rs:71-75`
- `input-meta-refcell-races-with-input-lookahead` (moderate) — `crates/yamalgam/src/commands/query.rs:85-89`
- `integer-only-indexing-diverges-from-equal-float-values` (moderate) — `crates/yamalgam-query/src/valt.rs:331-336`
- `run-filter-signature-overconstrains-callers` (moderate) — `crates/yamalgam-query/src/engine.rs:157-163`
- `rc-makes-value-model-non-send` (advisory) — `crates/yamalgam-query/src/value.rs:71-75`

### The Emission Performance Surface

- `yaml-inline-emitter-string-per-node` (moderate) — `crates/yamalgam-query/src/emit/yaml.rs:191`
- `needs-quotes-copies-every-scalar-it-inspects` (moderate) — `crates/yamalgam-query/src/value.rs:705-713`
- `emitter-anchor-table-is-a-linear-scanned-vec` (moderate) — `crates/yamalgam-query/src/emit/yaml.rs:321-326`
- `query-cli-materializes-all-inputs-before-filtering` (moderate) — `crates/yamalgam/src/commands/query.rs:242-249`

### The Spec Conformance Surface

- `alias-audit-skips-mapping-keys` (moderate) — `crates/yamalgam-query/src/value.rs:517-523`
- `regex-is-not-pcre` (moderate) — `docs/spec/yg-query-language-spec-draft-01.md:265-266`

### The Supply Chain Surface

- `release-check-runs-cargo-deny-without-config` (moderate) — `.justfile:176-180`
- `jaq-trait-coupling-outruns-version-constraint` (advisory) — `crates/yamalgam-query/Cargo.toml:12-14`
- `unused-dev-dep-pretty-assertions` (note) — `crates/yamalgam-query/Cargo.toml:20-22`

## If you have the `cased` skill loaded

Invoke it. The skill's Phase 5 covers remediation tracking with the full
schema reference and worked examples. This briefing exists for the case
where you land in the directory without the skill available.

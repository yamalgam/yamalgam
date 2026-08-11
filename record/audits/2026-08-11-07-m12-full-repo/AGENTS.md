# Agent Briefing — Full Yamalgam workspace at the M12 schema Validation and language Server release boundary

You are in a `cased` audit output directory. This file exists to help you pick
up remediation work without thrashing. Read it once, then act.

**Audit:** `2026-08-11-07-m12-full-repo`
**Date:** 2026-08-11
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
audit: 2026-08-11-07-m12-full-repo
last_updated: YYYY-MM-DD
status:
  fixed: 0
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 20
---

# Actions Taken: Full Yamalgam workspace at the M12 schema Validation and language Server release boundary

Summary of remediation status for the [2026-08-11 Full Yamalgam workspace at the M12 schema-validation and language-server release boundary audit](README.md).

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

## Recording fixes in git

One focused commit per logical fix. Each fix commit names the finding(s)
it addresses with a git trailer, one line per slug:

    Audit-Finding: <finding-slug>

Do not pair every fix commit with a ledger commit — a ledger entry cannot
cite a SHA that doesn't exist yet, and per-fix ledger commits double the
history noise. Land the fix commits first, then append ledger entries
(batching several per entry-commit is fine) citing the real SHAs. The
trailers keep the finding-to-commit mapping recoverable before the ledger
catches up:

    git log --format='%h %(trailers:key=Audit-Finding,valueonly)'

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

### LSP Semantic Positioning and Request Lifecycle

- `leading-document-marker-shifts-lsp-document-indexes` (significant) — `docs/yg/language-server.md:164-171`
- `sequence-item-schema-context-is-unreachable` (significant) — `record/superpowers/plans/2026-08-09-m12-schema-validation-lsp.md:1307-1310`
- `ropey-unicode-lines-desynchronize-lsp-positions` (moderate) — `crates/yamalgam-cst/Cargo.toml:12`
- `documented-example-hover-is-not-implemented` (moderate) — `docs/yg/language-server.md:166`
- `lsp-cancellation-cannot-interrupt-work-and-retains-request-ids` (moderate) — `crates/yamalgam-lsp/src/server.rs:52-61`
- `stdio-error-path-can-hang-joining-reader` (moderate) — `crates/yamalgam-lsp/src/server.rs:28-33`
- `lsp-definition-resource-errors-erased` (moderate) — `crates/yamalgam-lsp/src/definition.rs:67-93`

### Schema Compilation, Validation, and Retrieval Boundaries

- `schema-validator-cache-key-erases-identity-and-options` (significant) — `crates/yamalgam-lsp/src/diagnostics.rs:48-58`
- `schema-native-adapter-unbounded-recursion` (significant) — `crates/yamalgam-schema/src/value.rs:318-346`
- `schema-diagnostic-cap-does-not-bound-work` (significant) — `crates/yamalgam-schema/src/schema.rs:171-206`
- `schema-session-holds-global-lock-across-retrieval` (moderate) — `crates/yamalgam-schema/src/retrieval.rs:122-126`
- `schema-session-cache-clones-located-documents` (moderate) — `crates/yamalgam-schema/src/retrieval.rs:272-275`
- `schema-navigation-recompiles-patterns-per-request` (advisory) — `crates/yamalgam-schema/src/navigation.rs:351-355`

### Recovering CST and Editor Update Cost

- `recovering-cst-span-normalization-is-quadratic` (significant) — `crates/yamalgam-cst/src/recovery.rs:78-87`
- `lsp-didchange-rebuilds-full-document-stages` (significant) — `crates/yamalgam-cst/src/incremental.rs:82-100`
- `lsp-diagnostic-positioning-recopies-source` (significant) — `crates/yamalgam-lsp/src/diagnostics.rs:127-143`

### Located Value Representation

- `located-value-allows-invalid-parallel-shapes` (moderate) — `crates/yamalgam-core/src/located.rs:5-20`
- `located-composition-clones-location-subtrees` (moderate) — `crates/yamalgam-compose/src/lib.rs:269-276`

### Retrieval and Dependency Lifecycle

- `http-worker-panic-result-discarded` (moderate) — `crates/yamalgam-resolve/src/retriever/http.rs:144-156`
- `librebar-http-enables-unused-tokio-subsystems` (advisory) — `crates/yamalgam-resolve/Cargo.toml:15-18`

## If you have the `cased` skill loaded

Invoke it. The skill's Phase 5 covers remediation tracking with the full
schema reference and worked examples. This briefing exists for the case
where you land in the directory without the skill available.

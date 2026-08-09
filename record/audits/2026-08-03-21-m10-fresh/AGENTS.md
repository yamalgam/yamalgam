# Agent Briefing — M10 milestone surface — 24 commits on branch m10 Yg Cli vs main (merge Base 6e3a646): the new yamalgam Query crate, the yg CLI, and M10's yamalgam Core tag Resolution additions

You are in a `cased` audit output directory. This file exists to help you pick
up remediation work without thrashing. Read it once, then act.

**Audit:** `2026-08-03-21-m10-fresh`
**Date:** 2026-08-03
**Findings:** 47 total

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
add an entry. The `open` count is `47 - (fixed + mitigated +
accepted + disputed + deferred)`.

```markdown
---
audit: 2026-08-03-21-m10-fresh
last_updated: YYYY-MM-DD
status:
  fixed: 0
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 47
---

# Actions Taken: M10 milestone surface — 24 commits on branch m10 Yg Cli vs main (merge Base 6e3a646): the new yamalgam Query crate, the yg CLI, and M10's yamalgam Core tag Resolution additions

Summary of remediation status for the [2026-08-03 M10 milestone surface — 24 commits on branch m10-yg-cli vs main (merge-base 6e3a646): the new yamalgam-query crate, the yg CLI, and M10's yamalgam-core tag-resolution additions audit](README.md).

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

### The Emitter and Expansion Surface

- `emitter-replays-tags-verbatim` (significant) — `crates/yamalgam-query/src/emit/yaml.rs:422-430`
- `explode-aliases-unbounded-expansion` (significant) — `crates/yamalgam-query/src/value.rs:503-526`
- `emitter-omits-c1-control-escapes` (moderate) — `crates/yamalgam-query/src/value.rs:865-887`
- `emitter-plain-document-end-marker` (moderate) — `crates/yamalgam-query/src/value.rs:832-849`
- `runtime-limits-cover-three-call-sites` (advisory) — `crates/yamalgam-query/src/value.rs:685-690`
- `env-access-always-compiled-in` (advisory) — `crates/yamalgam-query/src/engine.rs:36-40`

### The Error and Program Flow Surface

- `value-tree-recursion-has-no-depth-bound` (moderate) — `crates/yamalgam-query/src/value.rs:505-526`
- `input-error-class-flattened-through-string` (moderate) — `crates/yamalgam-query/src/engine.rs:226-230`
- `growth-operators-bypass-runtime-limits` (moderate) — `crates/yamalgam-query/src/ops.rs:57-69`
- `into-string-substitutes-error-text-as-data` (moderate) — `crates/yamalgam-query/src/valt.rs:350-356`
- `halt-exit-code-truncated-to-u8` (advisory) — `crates/yamalgam/src/commands/query.rs:610-619`
- `build-invariant-errors-labeled-as-limit` (advisory) — `crates/yamalgam-query/src/build.rs:172-174`
- `in-place-chown-failure-aborts-the-edit` (advisory) — `crates/yamalgam/src/commands/query.rs:326-337`
- `unreachable-macros-guard-unstated-invariants` (note) — `crates/yamalgam-query/src/emit/yaml.rs:74-80`

### The Allocation and Throughput Surface

- `build-raw-metadata-duplicates-every-scalar` (significant) — `crates/yamalgam-query/src/build.rs:189-207`
- `resolver-allocates-throwaway-string-per-scalar` (moderate) — `crates/yamalgam-query/src/build.rs:246-257`
- `json-emitter-allocates-indent-and-triple-allocates-keys` (moderate) — `crates/yamalgam-query/src/emit/json.rs:79-98`
- `yaml-emitter-paint-copies-every-scalar-when-color-is-off` (moderate) — `crates/yamalgam-query/src/emit/yaml.rs:413-419`
- `cli-output-is-unbuffered` (moderate) — `crates/yamalgam/src/commands/query.rs:80-82`
- `mapping-entries-collected-into-vec-per-emit` (advisory) — `crates/yamalgam-query/src/emit/yaml.rs:328-334`
- `emit-validation-walks-the-tree-a-second-time` (advisory) — `crates/yamalgam-query/src/emit/mod.rs:66-73`
- `ygmap-drop-allocates-a-vec-per-mapping` (advisory) — `crates/yamalgam-query/src/value.rs:117-125`
- `substring-values-are-allocated-twice` (advisory) — `crates/yamalgam-query/src/valt.rs:468-475`
- `slice-bound-keys-allocated-on-every-index` (note) — `crates/yamalgam-query/src/valt.rs:87-94`
- `cli-materializes-every-document-in-a-file-before-emitting` (advisory) — `crates/yamalgam-query/src/build.rs:24-38`

### The Value Model and API Surface

- `as-bool-inherent-shadows-jaq-truthiness` (moderate) — `crates/yamalgam-query/src/value.rs:211-218`
- `internal-value-representation-is-public` (moderate) — `crates/yamalgam-query/src/value.rs:36-48`
- `keys-returns-boxed-trait-object` (advisory) — `crates/yamalgam-query/src/value.rs:582-588`
- `run-records-static-bound-forces-clones` (moderate) — `crates/yamalgam/src/commands/query.rs:103-116`
- `three-bool-parameters-at-command-boundary` (moderate) — `crates/yamalgam/src/commands/query.rs:26-31`
- `emit-opts-mixes-emitter-specific-options` (advisory) — `crates/yamalgam-query/src/emit/mod.rs:36-51`
- `public-enums-lack-non-exhaustive` (note) — `crates/yamalgam-query/src/error.rs:28-33`
- `resolves-as-string-overrides-restate-resolution` (note) — `crates/yamalgam-core/src/tag_resolution.rs:20-28`

### The Spec Conformance Surface

- `yg-color-flags-replaced-without-spec-amendment` (advisory) — `docs/spec/yg-query-language-spec-draft-01.md:520-521`
- `sandboxed-compile-options-unreachable-from-cli` (advisory) — `crates/yamalgam-query/src/engine.rs:21-40`
- `spec-features-deferred-while-spec-text-still-promises-them` (note) — `docs/spec/yg-query-language-spec-draft-01.md:534-538`
- `resolved-type-is-an-exact-alias-of-type` (note) — `crates/yamalgam-query/src/funs.rs:150-153`
- `in-place-conflicts-diverge-from-plan-undocumented` (note) — `crates/yamalgam/src/lib.rs:119-126`
- `subcommand-name-shadowing-not-documented-in-help` (note) — `crates/yamalgam/src/lib.rs:31-36`

### The Supply Chain Surface

- `librebar-default-features-not-disabled` (moderate) — `crates/yamalgam/Cargo.toml:42-54`
- `yg-ships-competing-yaml-parser-for-its-own-config` (moderate) — `Cargo.lock:515-518`
- `jaq-std-time-feature-bundles-tzdb` (moderate) — `crates/yamalgam-query/Cargo.toml:11-19`
- `jaq-exact-pins-block-patch-updates` (advisory) — `crates/yamalgam-query/Cargo.toml:13-14`
- `serde-json-redundant-in-yaml-native-engine` (moderate) — `crates/yamalgam-query/src/funs.rs:310-314`
- `librebar-upgrade-expands-trust-boundary` (advisory) — `crates/yamalgam/Cargo.toml:46`
- `geiger-census-produced-no-unsafe-data` (advisory) — `.crustoleum/geiger.txt:1205-1207`
- `deny-advisory-ignore-matches-only-under-all-features` (note) — `.config/deny.toml:53-58`

## If you have the `cased` skill loaded

Invoke it. The skill's Phase 5 covers remediation tracking with the full
schema reference and worked examples. This briefing exists for the case
where you land in the directory without the skill available.

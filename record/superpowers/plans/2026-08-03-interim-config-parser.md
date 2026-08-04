# Interim Config Parser Decision Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> building-in-the-open:capturing-decisions to record the accepted sequencing
> decision.

**Goal:** Resolve `yg-ships-competing-yaml-parser-for-its-own-config` as a
bounded deferral tied to yamalgam publication, without attempting the already
chosen cross-repository backend swap prematurely.

**Architecture:** Record an accepted MADR decision that keeps librebar's
current YAML config backend only until yamalgam can be published and consumed
upstream. Route every mutating Just dependency-update recipe through a common
update step that restores the audited `serde-saphyr` and `granit-parser`
versions, making any change to those pins deliberate.

**Tech Stack:** MADR 4.0.0, Cargo lockfile controls, Just, bito, rumdl.

## Task 1: Bound the interim dependency

**Files:**

- Create: `record/decisions/0008-bound-config-parser-interim.md`
- Create: `record/decisions/README.md`
- Modify: `.justfile`

**Steps:**

- [ ] Record the chosen bounded-interim option, the rejected immediate-swap and
  YAML-removal options, explicit consequences, and the publication trigger.
- [ ] Index the new ADR alongside links to the seven legacy ADRs.
- [ ] Make `update`, `upgrade`, `upgrade-breaking`, and therefore `monthly`
  restore `serde-saphyr` and `granit-parser` to 1.0.0 after broad updates.
- [ ] Validate the ADR with bito and rumdl; dry-run the Just recipes and Cargo
  precise-update commands without broadly refreshing dependencies.
- [ ] Commit as `build(deps): bound interim config parser`.

## Task 2: Record the deferred finding

- [ ] Append a deferred ledger entry for
  `yg-ships-competing-yaml-parser-for-its-own-config` with the exact decision
  commit SHA and update totals to `deferred: 1`, `open: 28`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record config parser deferral`.

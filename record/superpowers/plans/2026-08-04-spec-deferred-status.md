# Deferred Spec Status Plan

> **For agentic workers:** Preserve future requirements while making the
> current milestone boundary visible from the canonical spec entry point.

**Goal:** Close `spec-features-deferred-while-spec-text-still-promises-them`
by surfacing the already-recorded M10 deferrals in draft 01 itself.

**Architecture:** Add one compact M10 implementation-status section near the
top of the specification. Link the strict-comparison, chomping, and external
resolution rules to their concrete destination and to the complete
implementation-notes document; do not duplicate its rationale.

**Tech Stack:** Markdown, rumdl.

## Task 1: Add the canonical status entry point

**Files:**

- Modify: `docs/spec/yg-query-language-spec-draft-01.md`

**Steps:**

- [ ] Mark strict comparison as deferred pending the draft-02 grammar decision.
- [ ] Mark `chomping` as deferred to the Phase 3 CST milestone.
- [ ] Mark external-resolution flags as deferred to M11 / Phase 4.
- [ ] Link the implementation notes for full rationale.
- [ ] Run focused Markdown lint.
- [ ] Commit as `docs(spec): surface deferred M10 features`.

## Task 2: Record the remediation

- [ ] Append a fixed ledger entry for
  `spec-features-deferred-while-spec-text-still-promises-them` with the exact
  code SHA and update totals to `fixed: 36`, `open: 7`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record deferred spec status remediation`.

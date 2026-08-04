# Cargo Deny All-Features Plan

> **For agentic workers:** Keep Cargo Deny's resolved graph identical in local
> checks and CI so advisory policy cannot become silently inactive.

**Goal:** Close `deny-advisory-ignore-matches-only-under-all-features` by
making `--all-features` an explicit, documented invariant for every Cargo Deny
entry point.

**Architecture:** Preserve the existing `just deny` command and align the
GitHub Actions Cargo Deny invocation with it. Document beside the advisory
ignore that the optional benchmark dependency is resolved only under the
all-features graph.

**Tech Stack:** Cargo Deny, Just, GitHub Actions, TOML.

## Task 1: Align and document the dependency graph

**Files:**

- Modify: `.github/workflows/ci.yml`
- Modify: `.config/deny.toml`

**Steps:**

- [ ] Add `--all-features` to the Cargo Deny action arguments.
- [ ] Document the flag requirement beside `RUSTSEC-2023-0051`.
- [ ] Verify every repository Cargo Deny entry point enables all features.
- [ ] Run `just deny` and confirm the configured advisory matches.
- [ ] Commit as `ci(security): exercise advisory ignores with all features`.

## Task 2: Record the remediation

- [ ] Append a fixed ledger entry with the exact code SHA and update totals to
  `fixed: 42`, `open: 0`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record deny feature-policy remediation`.

# Canonical Color CLI Plan

> **For agentic workers:** Keep librebar's shared flag vocabulary and make the
> specification and help text tell the same story.

**Goal:** Close `yg-color-flags-replaced-without-spec-amendment` by specifying
the shipped `--color` interface and making the `-C` collision explicit.

**Architecture:** Amend the draft specification to use librebar's canonical
`--color auto|always|never` control. Add a root-help note that `-C` means
`--chdir`, so jq-style color users get an immediate pointer to `--color`
without adding overlapping aliases.

**Tech Stack:** Rust 2024, clap, cargo-nextest, Just.

## Task 1: Align the specification and help

**Files:**

- Modify: `docs/spec/yg-query-language-spec-draft-01.md`
- Modify: `crates/yamalgam/src/lib.rs`
- Modify: `crates/yamalgam/tests/yg.rs`

**Steps:**

- [ ] Replace the stale `-C`/`-M` requirement with the shipped `--color`
  values and behavior.
- [ ] Add a root-help note that `-C` is `--chdir`, not color control.
- [ ] Add a CLI regression test for the help contract.
- [ ] Run the focused CLI tests and `just clippy`.
- [ ] Commit as `docs(cli): specify canonical color control`.

## Task 2: Record the remediation

- [ ] Append a fixed ledger entry for
  `yg-color-flags-replaced-without-spec-amendment` with the exact code SHA and
  update totals to `fixed: 34`, `open: 9`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record color CLI remediation`.

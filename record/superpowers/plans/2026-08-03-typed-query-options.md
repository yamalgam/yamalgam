# Typed Query Options Remediation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Close `three-bool-parameters-at-command-boundary` by making resolved
format, color, and quiet state structurally distinct at every query call site.

**Architecture:** Introduce `OutputMode::{Yaml { color }, Json}` and a
`RunOptions { output, quiet }` command-boundary value. Construct it in `main`
after format and terminal resolution. Thread `OutputMode` through in-place and
record execution so JSON cannot carry an independent color boolean.

**Tech Stack:** Rust 2024 enums and structs, cargo-nextest, Just.

## Task 1: Replace positional booleans

**Files:**

- Modify: `crates/yamalgam/src/main.rs`
- Modify: `crates/yamalgam/src/commands/query.rs`
- Verify: `crates/yamalgam/tests/yg.rs`

**Steps:**

- [ ] Run the format, color, warning, and in-place CLI characterization tests.
- [ ] Add the typed output mode and named command run-options value.
- [ ] Resolve the output mode once in `main` and call `cmd_query` with the
  options value instead of three positional booleans.
- [ ] Thread `OutputMode` through `run_in_place` and `run_records`, deriving
  JSON selection and YAML color from the enum.
- [ ] Run the complete `yg` CLI integration suite, `just fmt`, and
  `just clippy`.
- [ ] Commit code as `refactor(cli): type query output options`.

## Task 2: Record the closed finding

- [ ] Append a fixed ledger entry for
  `three-bool-parameters-at-command-boundary` with the exact code SHA and update
  totals to `fixed: 17`, `open: 30`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record typed query options remediation`.

# Single-Copy Substrings Plan

> **For agentic workers:** Preserve lossy UTF-8 semantics while removing the
> valid-substring intermediate `String`.

**Goal:** Close `substring-values-are-allocated-twice` by constructing the
final `Rc<str>` directly from the borrowed `Cow<str>` view.

**Architecture:** Add a private byte-to-`Rc<str>` helper shared by
`as_sub_str` and `from_utf8_bytes`. Valid UTF-8 takes the borrowed Cow path and
performs only the final reference-counted allocation; invalid UTF-8 retains
the standard lossy replacement behavior.

**Tech Stack:** Rust 2024, cargo-nextest, Just.

## Task 1: Pin conversion semantics

**Files:**

- Modify: `crates/yamalgam-query/src/valt.rs`

**Steps:**

- [ ] Add a focused unit test for valid and invalid UTF-8 through the shared
  conversion helper.
- [ ] Run the focused test and confirm it fails before implementation.

## Task 2: Remove the intermediate owned string

- [ ] Implement the helper using `Rc::from(String::from_utf8_lossy(...).as_ref())`.
- [ ] Route both jaq string hooks through it.
- [ ] Run all `yamalgam-query` tests and `just clippy`.
- [ ] Commit as `perf(query): avoid duplicate substring copies`.

## Task 3: Record the closed finding

- [ ] Append a fixed ledger entry for `substring-values-are-allocated-twice`
  with the exact code SHA and update totals to `fixed: 28`, `open: 16`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record substring allocation remediation`.

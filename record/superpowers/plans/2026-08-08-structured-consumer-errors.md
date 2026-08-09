# Structured Consumer Errors Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Preserve structured parser diagnostics through CST recovery and
preserve `std::io::Error` through serde reader entry points.

**Architecture:** Add an opt-in CST report path that returns the recovered,
lossless tree alongside every `ParseError`, while leaving existing tree-only
recovery APIs source-compatible. Add a dedicated serde `Error::Io` variant and
map both reader entry points through it so callers retain `ErrorKind` and error
chaining.

**Tech Stack:** Rust 2024, `yamalgam-cst`, `yamalgam-serde`, cargo-nextest,
Just.

## Constraints

- Add no dependency and remove no public API.
- Preserve byte-for-byte CST recovery and the existing `parse_to_cst` return
  type.
- Capture parser-stream and builder-generated limit errors in the CST report.
- Keep serde `Custom` reserved for serde data-model errors.
- Preserve the current rendered `I/O error: ...` reader message.
- Accept the pre-1.0 source-compatibility cost of adding a public `Error`
  variant; verify every workspace match site.
- Do not edit immutable audit source artifacts.
- Give each audit finding its own signed code SHA and separate ledger commit.
- Follow RED -> GREEN before broad verification.

---

### Task 1: Return structured CST recovery diagnostics

**Files:** Modify and test `crates/yamalgam-cst/src/lib.rs`.

- [ ] **Step 1: Add focused failing report tests**

Add a malformed-input test requiring all of the following:

- `parse_to_cst_report("'unterminated")` exists;
- `report.cst.to_text()` reproduces the entire input;
- the report contains one `ParseError::Scan` whose kind is
  `ScanErrorKind::UnterminatedScalar`;
- the existing `parse_to_cst` recovery API still returns the same lossless
  tree.

Add a builder-limit test using `CstBuilder::new_with_config` with
`max_depth = 1` and an unconstrained parser over `[[0]]`. Require lossless CST
recovery plus a reported `ParseError::LimitExceeded` so builder-generated
diagnostics are covered separately from upstream parser errors.

- [ ] **Step 2: Run focused tests to verify RED**

```bash
cargo nextest run -p yamalgam-cst -E 'test(cst_report_preserves_structured_parse_error) | test(builder_report_preserves_generated_limit_error)'
```

Expected: compilation fails because the report type and report-returning APIs
do not exist.

- [ ] **Step 3: Add the report path without changing recovery**

Add public `CstReport<'input>` with a recovered `cst: CstNode<'input>` and
`diagnostics: Vec<ParseError>`. Store diagnostics privately in `CstBuilder`,
append the original error in `handle_error`, and add `build_report`.

Keep `build` returning `CstNode` by delegating to `build_report` and discarding
only the report wrapper. Add bounded, unchecked, and configured
`parse_to_cst_report*` convenience functions parallel to the existing
tree-only functions.

- [ ] **Step 4: Verify, review, and commit**

```bash
cargo nextest run -p yamalgam-cst
just fmt
just clippy
just test
just doc-test
git --no-pager diff --check
```

After independent review, create the focused signed code commit, then append
and separately commit its exact SHA in the audit ledger.

---

### Task 2: Preserve reader I/O errors

**Files:** Modify `crates/yamalgam-serde/src/error.rs`,
`crates/yamalgam-serde/src/lib.rs`, and
`crates/yamalgam-serde/tests/reader.rs`.

- [ ] **Step 1: Add focused failing reader tests**

Implement a deterministic `Read` fixture that returns a chosen
`std::io::ErrorKind`. Require both `from_reader` and `from_reader_unchecked` to
return `Error::Io`, preserve that kind, and expose the same `io::Error` through
`std::error::Error::source`.

Add an `error.rs` unit test requiring the internal structured clone path to
retain `Error::Io`, its `ErrorKind`, and its rendered message.

- [ ] **Step 2: Run focused tests to verify RED**

```bash
cargo nextest run -p yamalgam-serde -E 'test(from_reader_preserves_io_error) | test(from_reader_unchecked_preserves_io_error)'
```

Expected: compilation fails because `Error::Io` does not exist.

- [ ] **Step 3: Add and wire the typed I/O variant**

Add `Error::Io(std::io::Error)`, render it as `I/O error: {error}`, return it
from `Error::source`, and implement `From<std::io::Error>`. Because
`std::io::Error` is not cloneable, reconstruct the internal structured clone
with `std::io::Error::new(error.kind(), error.to_string())`. This retains the
typed variant, error kind, and display text without routing through `Custom`.
Keep the existing string fallback only for non-cloneable `ResolveError`.

Use `?` or `map_err(Error::Io)` in both reader functions; leave string and
deserialization behavior unchanged.

- [ ] **Step 4: Verify, review, and commit**

```bash
cargo nextest run -p yamalgam-serde
just fmt
just clippy
just test
just doc-test
git --no-pager diff --check
```

After independent review, create the focused signed code commit, then append
and separately commit its exact SHA in the audit ledger.

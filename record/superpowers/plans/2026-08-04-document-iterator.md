# Document Iterator Plan

> **For agentic workers:** Add the library streaming boundary without a new
> self-referential owner dependency in the CLI.

**Goal:** Mitigate `cli-materializes-every-document-in-a-file-before-emitting`
by exposing document-at-a-time parsing and making eager parsing a thin collect.

**Architecture:** Wrap the parser's semantic event stream in a public iterator
that owns builder state and yields one fully owned `YamlVal` per document. The
input remains borrowed. This removes library-mandated document accumulation;
the CLI's owned `String`/iterator lifetime integration remains a separate step.

**Tech Stack:** Rust 2024, yamalgam parser, cargo-nextest, Just.

## Task 1: Pin incremental delivery

**Files:**

- Modify: `crates/yamalgam-query/tests/value.rs`

**Steps:**

- [ ] Add a regression proving the first valid document is yielded before a
  later malformed document reports its error.
- [ ] Run the focused test and confirm it fails before implementation.

## Task 2: Add the document iterator

**Files:**

- Modify: `crates/yamalgam-query/src/build.rs`
- Modify: `crates/yamalgam-query/src/lib.rs`

**Steps:**

- [ ] Implement a structural-event filter and public `Documents` iterator.
- [ ] Add `parse_documents_with_config`; collect it in
  `parse_str_with_config` for compatibility.
- [ ] Run all `yamalgam-query` tests and `just clippy`.
- [ ] Commit as `feat(query): stream parsed documents`.

## Task 3: Record the mitigation

- [ ] Append a mitigated ledger entry for
  `cli-materializes-every-document-in-a-file-before-emitting` with the exact
  code SHA and update totals to `mitigated: 2`, `open: 14`.
- [ ] Record that the CLI still needs an owned-source iterator integration to
  consume the new borrowing API lazily.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record document streaming mitigation`.

# Borrowed String Map Lookup Plan

> **For agentic workers:** Match `YamlVal` string hashing exactly so IndexMap
> borrowed lookup remains sound.

**Goal:** Close `slice-bound-keys-allocated-on-every-index` by looking up
string mapping keys without constructing temporary `YamlVal`/`Rc<str>` keys.

**Architecture:** Add a private borrowed string key implementing IndexMap's
`Equivalent<YamlVal>` plus the same type-rank and string hash as
`YamlVal::Data::Str`. Expose a crate-private `YgMap::get_str` and route slice
bounds plus the public value convenience lookup through it.

**Tech Stack:** Rust 2024, IndexMap, cargo-nextest, Just.

## Task 1: Pin borrowed lookup

**Files:**

- Modify: `crates/yamalgam-query/src/value.rs`

**Steps:**

- [ ] Add a focused unit test proving borrowed lookup finds a string key and
  does not match a non-string key with similar display text.
- [ ] Run the focused test and confirm it fails before implementation.

## Task 2: Remove temporary keys

**Files:**

- Modify: `crates/yamalgam-query/src/value.rs`
- Modify: `crates/yamalgam-query/src/valt.rs`

**Steps:**

- [ ] Implement the equivalent borrowed string key and `YgMap::get_str`.
- [ ] Route both slice-bound lookup sites and `YamlVal::get_str` through it.
- [ ] Run all `yamalgam-query` tests and `just clippy`.
- [ ] Commit as `perf(query): borrow string keys for map lookup`.

## Task 3: Record the closed finding

- [ ] Append a fixed ledger entry for
  `slice-bound-keys-allocated-on-every-index` with the exact code SHA and
  update totals to `fixed: 29`, `open: 15`.
- [ ] Run focused Markdown lint, confirm immutable audit hashes, and commit as
  `docs(audit): record borrowed slice key remediation`.

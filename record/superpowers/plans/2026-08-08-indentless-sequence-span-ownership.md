# Indentless Sequence Span Ownership Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make every valid YAML Test Suite document round-trip byte-for-byte
through the CST with no allowlist.

**Architecture:** Correct parser event ownership for indentless block
sequences. Their collection start and end events are synthetic and therefore
receive zero-width spans; visible `-`, `?`, and `:` bytes remain owned only by
the corresponding structural events. The CST representation and traversal do
not need special cases.

**Tech Stack:** Rust 2024, `yamalgam-parser`, `yamalgam-cst`,
`yamalgam-compare`, YAML Test Suite, cargo-nextest, Just.

## Reproduced Mechanism

Current HEAD still fails the audit's eight cases, but the live outputs identify
indentless block-sequence ownership rather than the audit's proposed flow-end
mechanism:

- seven cases duplicate the first `-` because `SequenceStart` and
  `BlockEntry` share its nonzero span;
- `6PBE` additionally duplicates `:` because the synthetic `SequenceEnd` and
  following `ValueIndicator` share its nonzero span.

## Constraints

- Add no dependency and change no event variant shape.
- Preserve event order and all semantic parser output.
- A synthetic collection boundary must have a zero-width span at its source
  position; the visible structural event retains the original nonzero span.
- Remove the entire valid-input round-trip allowlist; do not replace it with a
  new exemption.
- Do not edit immutable audit source artifacts.
- Follow RED -> GREEN before broad verification.

---

### Task 1: Establish exclusive span ownership

**Files:** Modify `crates/yamalgam-parser/tests/parser.rs`,
`crates/yamalgam-parser/src/parser.rs`, and
`crates/yamalgam-compare/tests/cst_round_trip.rs`.

- [ ] **Step 1: Add focused parser ownership regressions**

Parse `key:\n- a\nnext: b` and require:

- the indentless block `SequenceStart` span is zero-width at the first dash;
- the first `BlockEntry` span alone covers that dash;
- the synthetic `SequenceEnd` span is zero-width at the following mapping key.

Parse `---\n?\n- a\n:\n- b` and require the key-side synthetic
`SequenceEnd` to be zero-width at the `:` while `ValueIndicator` alone covers
the colon.

- [ ] **Step 2: Remove the corpus allowlist and verify RED**

Delete `ROUND_TRIP_ALLOWLIST` and its stale-pass branches so every non-failing
case requires exact equality. Run:

```bash
cargo nextest run -p yamalgam-parser -E 'test(indentless_sequence_boundaries_are_synthetic) | test(indentless_sequence_end_does_not_own_following_colon)'
cargo nextest run -p yamalgam-compare --test cst_round_trip -E 'test(57H4) | test(6PBE) | test(7ZZ5) | test(AZ63) | test(RLU9) | test(S3PD) | test(S9E8) | test(SKE5)'
```

Expected: parser span assertions fail and all eight CST cases fail without the
allowlist.

- [ ] **Step 3: Make synthetic boundaries zero-width**

In the parser's indentless-sequence path, retain the current source position
but set `end = start` for both the synthetic `SequenceStart` and synthetic
`SequenceEnd`. Do not change the following token's own span or consume it.

- [ ] **Step 4: Verify the universal contract**

```bash
cargo nextest run -p yamalgam-parser
cargo nextest run -p yamalgam-compare --test cst_round_trip
just fmt
just clippy
just test
just doc-test
git --no-pager diff --check
```

The full round-trip corpus must pass all 351 harness files with no valid-input
exemptions.

- [ ] **Step 5: Review and commit**

After independent review, create the focused signed code commit, then append
and separately commit its exact SHA in the audit ledger. Record the reproduced
mechanism rather than repeating the audit's disproven flow-end hypothesis.

# Preserve Parser Span Errors Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Preserve scanner errors encountered while deriving optional event spans,
including implicit mapping finalization inside flow sequences.

**Architecture:** Keep the parser state machine and existing span rules unchanged.
Replace each lossy `Result`-to-`Option` conversion with `?`, then derive a span
from the resulting `Option<Token>`; only genuine end-of-stream retains the
existing default span.

**Tech Stack:** Rust 2024, `yamalgam-parser`, `yamalgam-scanner`, cargo-nextest,
Just.

## Constraints

- Add no dependency and change no public API.
- Preserve existing event order and span behavior for valid input.
- Fix all seven instances of the lossy probe pattern, not only the reported
  flow-pair state.
- Match the scanner error kind in the regression; do not assert only that some
  error occurred.
- Do not edit immutable audit source artifacts.
- Follow RED -> GREEN before broad verification.

---

### Task 1: Prove and remove scanner-error loss

**Files:** Modify `crates/yamalgam-parser/tests/parser.rs` and
`crates/yamalgam-parser/src/parser.rs`.

- [ ] **Step 1: Add the malformed flow-pair regression**

Add a test that parses `[a: b @]`, obtains the parser error, and requires:

```rust
ParseError::Scan(error) if error.kind == ScanErrorKind::UnexpectedCharacter
```

The input must reach `FlowSequenceEntryMappingEnd` after the `b` scalar, then
surface the scanner failure produced by `@`.

- [ ] **Step 2: Run the focused test to verify RED**

```bash
cargo nextest run -p yamalgam-parser -E 'test(flow_pair_finalization_preserves_scanner_error_kind)'
```

Expected: the current `.ok().flatten()` probe consumes the scanner error, so
the required `ParseError::Scan(UnexpectedCharacter)` is not returned.

- [ ] **Step 3: Propagate all seven optional-span probe errors**

In these parser states, replace
`self.peek_token().ok().flatten().map_or_else(...)` with
`self.peek_token()?.map_or_else(...)`:

1. `BlockMappingValue`
2. `IndentlessSequenceEntry`
3. `FlowSequenceEntryMappingKey`
4. `FlowSequenceEntryMappingValue`
5. `FlowSequenceEntryMappingEnd`
6. `FlowMappingValue`
7. `FlowMappingEmptyValue`

Keep each existing span closure unchanged, including the zero-width
`MappingEnd` span.

- [ ] **Step 4: Run focused verification to verify GREEN**

```bash
cargo nextest run -p yamalgam-parser -E 'test(flow_pair_finalization_preserves_scanner_error_kind)'
cargo nextest run -p yamalgam-parser
```

- [ ] **Step 5: Run repository verification**

```bash
just fmt
just clippy
just test
just doc-test
git --no-pager diff --check
```

- [ ] **Step 6: Review and commit**

Review the focused diff and test evidence, then create one signed code commit.
Afterward, append one immutable-ledger entry containing that exact code SHA and
create a separate signed ledger commit.

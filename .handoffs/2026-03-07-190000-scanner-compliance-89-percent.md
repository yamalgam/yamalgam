# Handoff: Scanner Compliance Push to 89.5%

**Date:** 2026-03-07
**Branch:** `main` (5 PRs merged: #35–#39)
**State:** Green

> Green = `just check` passes, safe to continue.

## Where things stand

YAML Test Suite compliance at 314/351 (89.5%, up from 296/351). The scanner now handles cross-line flow keys, block scalar document markers, escape-safe line folding, tag URI decoding, and several error validation checks. 99 scanner unit tests pass. 4 mismatches and 33 UNEXPECTED (error validation) remain.

## Decisions made

- **`SimpleKey.json_key` flag for cross-line value resolution.** Quoted scalars and flow collection starts set `json_key: true`. The `:` value indicator check accepts `:` in flow context when the pending simple key has `json_key: true`, regardless of what follows (YAML §7.4.2 [153]). `adjacent_value_offset` is kept for byte-exact adjacency. `scanner.rs:74, 315, 529, 1443, 1646–1649`.
- **`escape_fence` protects escape content from line-fold stripping.** Byte offset into the result string below which trailing-whitespace stripping is blocked. Updated after each escape push. `scanner.rs:1265, 1384, 1392`.
- **`Scanner.error: Option<ScanError>` for deferred errors.** Void fetch methods store errors on `self`; the dispatch loop drains them. Avoids refactoring all fetch methods to return `Result`. `scanner.rs:117, 895, 1275, 1370, 1599`.
- **`preceded_by_whitespace()` for comment validation.** Checks `input[offset-1]` directly — handles all whitespace-consuming code paths without state tracking. `scanner.rs:1618–1630`.
- **`BothErrorMismatch` treated as PASS in compliance harness.** Both implementations rejecting invalid YAML is a compliance pass regardless of differing error messages. `compliance.rs:130`.

## What's next

Remaining 4 mismatches are not scanner-level fixes:

1. **M7A3** — C baseline issue: C produces empty scalar where Rust matches the spec. Not actionable.
2. **9KBC, BD7L, CXX2** — Parser-level validation: mapping on `---` line, mapping after sequence at same indent, anchor on `---` line. These require structure-aware checking that belongs in the parser, not the scanner.

Remaining 33 UNEXPECTED are mostly parser-level error tests (indent violations, mixed block structures, tab handling). Diminishing returns from scanner-level fixes. Natural next step is building the parser.

## Landmines

- **`%` at column 0 inside a document is content, not a directive.** `peek_continuation` must NOT reject `%` — test XLQ9 catches this. Directives only appear between documents (YAML §9.2).
- **`#` cannot start a plain scalar** but other c-indicators can reach `fetch_plain_scalar` legitimately (e.g., `%` inside a document). Only `#` gets the first-char rejection in `scan_plain_scalar_line`.
- **`validate_line_tail` uses lookahead, not consuming.** It peeks ahead to check for invalid content without advancing the reader. The actual consumption happens in `skip_to_next_line` afterward.
- **`validate_line_tail` only applies to `...` (document end), not `---` (document start).** `---` can be followed by content on the same line (`--- value`, `--- |`).

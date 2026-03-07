# Handoff: Scanner State Machine (Milestone 2)

**Date:** 2026-03-06
**Branch:** `main`
**State:** Green

> Green = `just check` passes, safe to continue.

## Where things stand

Milestone 2 is complete: 10 PRs merged (#9-#18). The scanner state machine handles all YAML token types — stream/document markers, flow indicators, block indicators with indent tracking, plain/quoted/block scalars, anchors, aliases, and tags. 157/351 YAML Test Suite compliance tests fully match (45%). `just check` passes clean. 83 scanner unit tests, 1224 lines of scanner implementation.

## Decisions made

- **Eager simple key resolution** instead of libfyaml's deferred mechanism. After scanning a scalar, the scanner peeks for `: ` and retroactively inserts `BlockMappingStart` + `Key` + `Value`. Handles the common `key: value` pattern without maintaining a separate simple key list or retroactive token insertion. Trade-off: can't handle anchor/tag before key (`&a key: val`).
- **Token queue (`VecDeque`)** for multi-token emission. One scanner fetch can produce multiple tokens (e.g., `BlockSequenceStart` + `BlockEntry` on indent increase, or `Key` + `Scalar` + `Value` from simple key resolution).
- **Character-by-character content scanning** replaces line-by-line skip. Every unrecognized character feeds into `fetch_plain_scalar`, which reads until a terminator (`: `, newline, flow indicator, `#` after space).
- **Flow context simple key** works the same as block context — `: ` after a scalar triggers Key/Value in both.

## What's next

**Goal: 351/351 compliance.** The remaining 194 mismatches fall into these categories, ordered by estimated impact and difficulty:

1. **Fix the C harness TAG limitation (~68 cases).** `fy_scan()` without document state initialization causes `fy_fetch_tag` to fail silently — tags are absorbed as plain scalars with escaped `!`. Fix: initialize default document state in `tools/fyaml-tokenize/main.c` before scanning, or switch to the event-level API (`fy_parser_parse`). This is the single highest-impact fix. Reference: `fy_parse_stream_start` (fy-parse.c:6015) for how document state is initialized.

2. **Multi-line plain scalar continuation (~54 cases).** Our `scan_plain_scalar_text` reads a single line. libfyaml's `fy_reader_fetch_plain_scalar_handle` continues across newlines when the next line has sufficient indentation. The continuation folds newlines to spaces (like folded block scalars). Implement in `crates/yamalgam-scanner/src/scanner.rs:445` (`scan_plain_scalar_text`).

3. **Deferred simple key mechanism (~25 cases).** Cases like `&a key: val` or `!!tag key: val` where anchor/tag precedes a key. Our eager resolution inserts Key before the scalar, but BlockMappingStart/Key should precede the anchor/tag. Options: (a) buffer tokens and defer yielding until simple key is resolved, or (b) port libfyaml's `fy_save_simple_key_mark` / `fy_purge_stale_simple_keys` mechanism.

4. **Unknown directive handling (~6 cases).** Our scanner returns `Err` on unknown `%` directives. libfyaml skips them with a warning. Fix: skip the line and continue instead of erroring. In `fetch_directive` at `scanner.rs:252`.

5. **Remaining edge cases (~5).** Complex keys, indentless sequences in mappings, colon handling details.

**Workflow:** Use the `using-git-like-clay` skill for all commits/PRs. Stage files with `git add`, write `commit.txt`, prompt for `gtxt`, then `git pm` to push-merge.

## Landmines

- **C harness must be built first:** `cd tools/fyaml-tokenize && make`. The comparison tests need the `fyaml-tokenize` binary.
- **C harness does NOT produce TAG tokens.** Any compliance test involving tags (`!`, `!!`, `!<uri>`) will mismatch because the C harness emits SCALAR with escaped `!` instead of TAG. Fix the harness before chasing tag-related scanner bugs.
- **52 UNEXPECTED results** (Rust succeeds, C fails) are likely caused by the C harness tag limitation — the C scanner errors out on undefined tag prefixes while our scanner doesn't validate tag prefixes.
- **`scan_plain_scalar_text` is single-line only.** Multi-line plain scalars (`key: long\n  value`) are not handled — each line is scanned independently. This causes scalar value mismatches and missed continuation content.
- **Borrow checker pattern:** `self.queue.push_back(self.method())` requires a `let token = self.method(); self.queue.push_back(token);` local to avoid double mutable borrow. This applies to all `fetch_*` methods that return tokens.

# Handoff: Scanner Compliance Push (Milestone 3 start)

**Date:** 2026-03-06
**Branch:** `main`
**State:** Green

> Green = `just check` passes, safe to continue.

## Where things stand

Six PRs merged (#20–#25), pushing YAML Test Suite compliance from 157/351 (45%) to 235/351 (67%). The scanner now handles multi-line plain scalars, quoted scalar folding with empty lines, flow context key detection, and deferred simple key resolution for anchor/tag prefixes. The C harness (`fyaml-tokenize`) properly produces TAG tokens using raw shorthand values. 110 scanner unit tests pass. `just check` is clean.

## Decisions made

- **C harness uses internal libfyaml headers** for `fy_parser_set_default_document_state()`. Acceptable because we're pinned to vendored libfyaml 0.9.5 and this is a test tool, not production code.
- **TAG/TAG_DIRECTIVE values output as raw shorthand** (e.g., `!!str` not `tag:yaml.org,2002:str`) using `fy_tag_token_handle()`/`fy_tag_token_suffix()` to match the Rust scanner's output format.
- **Deferred simple key via `pending_prefix` buffer** instead of porting libfyaml's full simple key stack. Handles same-line anchor/tag-before-key patterns. Trade-off: can't handle cross-line or multi-token simple keys. Good enough for +3 cases; full mechanism needed later.
- **Unknown directives skipped** per YAML 1.2 §6.8.1 instead of returning an error.

## What's next

Remaining 116 failures break down into four tiers, ordered by impact:

1. **Scalar value mismatches (~18).** Mixed causes:
   - Block scalar explicit indent indicator: `content_indent` uses the raw indicator value instead of adding it to the current indent level. Fix in `fetch_block_scalar` around `scanner.rs:741`.
   - Double-quoted `\` + newline between lines with trailing whitespace (6WPF, NP9H, TL85). The trailing whitespace before the fold needs to be trimmed.
   - Test suite visual tab markers (`———»` → `\t`): `extract_yaml_input` in `compliance.rs` doesn't convert them. Affects ~5 cases.

2. **Simple key edge cases (~20).** The `pending_prefix` mechanism handles same-line patterns but can't resolve: cross-line keys, alias-as-key (`*a: val`), or complex flow patterns. Next step: port libfyaml's `fy_save_simple_key_mark` / `fy_purge_stale_simple_keys` for full coverage.

3. **Flow context gaps (~8).** Explicit keys (`? foo :`), nested flow collections, and indentation edge cases within flow mappings/sequences.

4. **UNEXPECTED (52).** Error test cases (`fail: true`) where the Rust scanner succeeds instead of detecting invalid YAML. Not a priority — these are correctness gaps in validation, not token production.

**Workflow:** Feature branches, `using-git-like-clay` skill. Stage → `commit.txt` → `gtxt` → `git pm` → `git switch main && git up`.

## Landmines

- **zsh `!!` history expansion.** Shell commands with `!!` in test input will have `!` escaped to `\!`. Use `python3 -c 'sys.stdout.buffer.write(b"!!")'` or read from files to get actual `!` bytes. This burned ~20 minutes of debugging.
- **C harness must be rebuilt** after any change to `tools/fyaml-tokenize/main.c`: `cd tools/fyaml-tokenize && make clean && make`. The Makefile now includes internal libfyaml headers (`-I$(FYAML_SRC)/src/lib -I$(FYAML_SRC)/src/util`).
- **`pending_prefix` flushes on line change.** An anchor/tag at end of a line is flushed immediately when the scanner moves to the next line. This is correct but means the mechanism only works for same-line `&anchor key: val` patterns.
- **Quoted scalar `is_key` in flow context** now accepts `:` without trailing blank. This is correct per YAML 1.2 but means `"foo":bar` in block context would also trigger — currently not an issue because block context plain scalars include `:` + non-blank as content, but watch for edge cases.
- **`scan_plain_scalar_text` returns `Cow<str>`** — single-line returns `Borrowed` (zero-copy), multi-line returns `Owned` (folded). Callers must handle both via `scalar_token()` not `data_token()`.

# Handoff: Block Scalar, Folding, and Compliance Infrastructure

**Date:** 2026-03-06
**Branch:** `fix/scanner-block-scalar-and-folding` (pending merge to `main`)
**State:** Green

> Green = `just check` passes, safe to continue.

## Where things stand

YAML Test Suite compliance at 246/351 (70%, up from 235/351). Five scanner fixes address block scalar indentation, folded scalar line folding, blank line whitespace preservation, and quoted scalar trailing whitespace. The compliance test harness now correctly converts visual markers to real whitespace characters. 110 scanner unit tests pass.

## Decisions made

- **Block scalar explicit indent is relative to current indent.** YAML 1.2 §8.1.1.2. Previously treated as absolute column number. `scanner.rs:778`.
- **Auto-detect minimum uses `(indent+1).max(0)` not `.max(1)`.** Fixes zero-indent block scalars after `---`. `scanner.rs:783`.
- **Folded block scalar uses forward lookahead** to decide content→empty trim. §6.5 says discard content's line break before empty lines, but §8.2.1 overrides when the destination is more-indented. `fold_block_scalar` scans ahead in the lines slice. O(n²) worst case but fine for real YAML.
- **Test harness converts `␣` (U+2423) → space and `»` (U+00BB) → tab.** This unmasked 2 false passes (previously both scanners got matching wrong answers from literal marker characters).

## What's next

Remaining 105 failures break into three tiers:

1. **Simple key stack (~20 tests).** The `pending_prefix` mechanism handles same-line anchor/tag-before-key but can't resolve: bare `: value` (needs BlockMappingStart + Key in `fetch_value`, `scanner.rs:424`), cross-line keys, alias-as-key (`*a: val`). Next step: port libfyaml's `fy_save_simple_key_mark` / `fy_purge_stale_simple_keys`.

2. **Flow context (~8 tests).** Explicit keys (`? foo :` in flow), nested flow collections. Lower priority than simple key stack.

3. **UNEXPECTED (~46 tests).** Error test cases where Rust scanner succeeds instead of detecting invalid YAML. Validation gaps, not token production bugs. Lowest priority.

4. **Remaining scalar mismatches (~5 tests).** Quoted scalar edge cases with tab/whitespace combinations in double-quoted folding context.

## Landmines

- **`fetch_value` emits bare `Value` without Key/BlockMappingStart.** For `? key\n: val` this is correct (Key was emitted by `fetch_key`). For `: val` (empty key) it's wrong. Fixing this requires distinguishing the two cases — 16 explicit-key tests currently pass and would break if Key is always emitted in `fetch_value`. A proper simple key stack is the solution.
- **`fold_block_scalar` forward lookahead is O(n²).** Scans `lines[i+1..line_count]` for each content→empty transition. Not a concern for normal YAML; note if processing adversarial inputs.
- **Rust toolchain 1.94.0 components may need reinstall.** `rustfmt` and `clippy` components can arrive in a broken state after toolchain sync. Fix: `rustup component remove <comp> --toolchain 1.94.0 && rustup component add <comp> --toolchain 1.94.0`. Possibly related to a brew/cargo binary mismatch on the dev machine.
- **The 2 newly-unmasked MISMATCH tests (26DV, DE56)** are real scanner bugs exposed by the `␣` marker conversion. 26DV is a simple key issue (bare `:`). DE56 needs investigation.

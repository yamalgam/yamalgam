# Handoff: Code Review Fixes (Codex-53)

**Date:** 2026-03-08
**Branch:** `main` (merged via PRs #72, #73)
**State:** Green — 1072 tests pass, `just check` clean.

## Where things stand

All 7 findings from `docs/code-reviews/codex-53-high-03-08-2026.md` are addressed. Compliance harness now fails on real divergence with self-policing allowlists. Composer enforces alias/anchor/merge limits. Invalid merge values error per spec. 10 UNEXPECTED sub-cases were discovered during multi-case extraction and are allowlisted pending scanner fixes.

## What was done

- **Compliance assertions** — `MISMATCH`, `UNEXPECTED`, `EVENT_MISMATCH`, `EVENT_UNEXPECTED` now panic unless allowlisted. Stale entries (fixed case still in allowlist) also panic.
- **Multi-case extraction** — `extract_all_yaml_inputs()` in `c_baseline.rs` parses all cases per file. 55 additional sub-cases now exercised. 6 multi-case files: 2G84, 9MQT, SM9W, DK95, MUS6, Y79Y.
- **Composer limits** — `LimitExceeded` error variant. Anchor count, alias expansion count, merge depth all enforced. `collect_merge_pairs` returns `Result` and takes `max_merge_depth`.
- **Invalid merge values** — `<<: non_mapping` now returns `ComposeError::UnexpectedEvent` instead of silently dropping.
- **Config-aware APIs** — `from_str_with_config`, `from_str_single_with_config`, `Composer::from_str_with_config`.
- **C harness** — `strtoull` with 256MB cap replaces `atol` in batch frame length parsing.
- **Strict tag comparison** — `compare_event_streams_with_tags` for future use.

## What's next

**10 UNEXPECTED sub-cases need scanner fixes** (two root causes):

### Tab rejection too narrow (9 cases)

DK95#1, DK95#3, Y79Y#3 through Y79Y#9. Our scanner only rejects tabs at line start in block context with active indent (`scanner.rs:305`). libfyaml also rejects:

1. **After block indicators** (`-`, `?`, `:`) — e.g., `-\tfoo` should error
2. **In flow collections at line start** — e.g., `[\n\tfoo]` should error
3. **Inside scalar continuation lines** — tabs in quoted/block scalar continuations

Files: `crates/yamalgam-scanner/src/scanner.rs` — `scan_to_next_token()` (main tab check), `fetch_block_entry()`, `fetch_key()`, `fetch_value()` (indicator handlers), `fetch_block_scalar()`, `fetch_single_quoted_scalar()`, `fetch_double_quoted_scalar()` (scalar continuations).

### %YAML version too permissive (1 case)

ZYU8#3 (`%YAML 1.12345`). Version parsing at `scanner.rs:710-716` accepts any sequence of digits and dots. Should validate `MAJOR.MINOR` format with reasonable constraints.

File: `crates/yamalgam-scanner/src/scanner.rs` — `fetch_version_directive()`.

### After fixing

Each fix should cause the corresponding compliance test to move from UNEXPECTED to PASS (BothErrorMatch). The staleness check will then panic, requiring removal of the entry from `TOKEN_UNEXPECTED_ALLOWLIST` / `EVENT_UNEXPECTED_ALLOWLIST` in `compliance.rs`.

## Decisions made

- **Allowlists are self-policing** — stale entries cause test failure, not silent pass.
- **Invalid merge values error** — `<<: 1` returns `ComposeError::UnexpectedEvent`, not silent drop. Behavioral change from prior code.
- **Convenience APIs stay unlimited** — `from_str()` uses `ResourceLimits::none()`. Safe defaults come through `from_str_with_config()`.
- **JEF9#1, JEF9#2 added to MISMATCH allowlists** — trailing whitespace differences in literal block scalars. Both scanners produce different values; needs investigation.

## Landmines

- **`just c-baseline` must be re-run** after any C harness or test suite change. Multi-case IDs (`FILE#N`) are now in the cache format. Old caches without `#N` keys will trigger subprocess fallback (slower but correct).
- **`collect_merge_pairs` signature changed** — now takes `&ResourceLimits` and `depth: usize`, returns `Result<(), ComposeError>`. Any code calling it directly needs updating.
- **Merge key test (`compose::tests::merge_key`)** uses `from_str` (unlimited). If you change `from_str` to use defaults, that test will need a config with merge limits high enough for the test data.
- **Tab fixes will be tricky** — the 9 tab cases test different positions (after indicators, in flow, in scalars). Each may need a separate check point in the scanner. Don't try to solve them all with one rule.

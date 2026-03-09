# Handoff: Scanner Tab/Version Fixes + Tooling Overhaul

**Date:** 2026-03-09
**Branch:** `main` (uncommitted changes)
**State:** Green — `just check` passes, 0 UNEXPECTED compliance cases.

## Where things stand

All 10 UNEXPECTED compliance sub-cases are fixed. The scanner now matches libfyaml's tab rejection behavior and validates `%YAML` version format. Compliance: 372 PASS, 0 UNEXPECTED, 30 EXPECTED, 4 MISMATCH. Changes are uncommitted on `main`, ready for review.

Separately, Claude Code permissions were overhauled to eliminate approval fatigue, agent teams were enabled, and a YAML processing model diagram was created in pikchr.

## What was done

### Scanner fixes (3 files, 184 lines changed)
- **Version validation** (ZYU8#3): `fetch_version_directive()` now validates `MAJOR.MINOR` format — each component 1–4 digits, exactly one dot.
- **Tab in double-quoted continuation** (DK95#1): `fetch_double_quoted_scalar()` rejects tab at column 0 on continuation lines.
- **Lone tab at document level** (DK95#3): `fetch_next_token()` EOF handler rejects tab consumed as whitespace when no tokens were produced.
- **Tab at line start in flow context** (Y79Y#3): `scan_to_next_token()` tracks `tab_at_flow_line_start` flag; rejects when content follows on the same line (blank lines are fine).
- **Tab after block indicators** (Y79Y#4–#9): `tab_in_preceding_whitespace` field on `Scanner`; checked in `fetch_block_entry()`, `fetch_key()`, `fetch_value()`.
- **Allowlists emptied**: both `TOKEN_UNEXPECTED_ALLOWLIST` and `EVENT_UNEXPECTED_ALLOWLIST` are now `&[]`.
- **94 new unit tests** in `scanner.rs` covering all 10 fix cases plus must-still-pass guardrails.

### Tooling changes
- **`scripts/extract-test-yaml.py`**: Reusable CLI for extracting YAML inputs from test suite files with marker conversion. Flags: `--case N`, `--pipe-to-c`, `--raw`.
- **`docs/diagrams/yaml-processing-model.pikchr` + `.svg`**: YAML 1.2.1 spec processing model (Figure 3.1) recreated in pikchr. Replaces the raster PNG reference.
- **`docs/diagrams/reference/`**: Curated pikchr examples (compiler flow, component layout, railroad diagrams, etc.) for visual language reference.

### Claude Code configuration
- **Global permissions** (`~/.claude/settings.json`): Replaced 30+ granular Bash patterns with `Bash(*)`. Added unrestricted `Edit` and `Write`. Added deny rules for `rm -rf`, `git push --force`, `git reset --hard`.
- **Project permissions** (`.claude/settings.json`): Same consolidation — `Bash(cargo:*)` instead of per-subcommand patterns. Added `Edit` and `Write`.
- **Local settings** (`.claude/settings.local.json`): Cleaned from 40 one-off entries to 8.
- **Agent teams enabled**: `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1` in global `env` settings.
- **Status line**: Added `current_dir` with `~` substitution for `/Users/clay`.

## What's next

1. **Review and commit scanner changes** — the `.claude/settings.json` diff is mixed in; separate the scanner changes from config changes for clean commits.
2. **Diagram #2: Event fan-out architecture** — pikchr diagram showing events as the pivot point, resolver middleware, and the 4 consumer fan-out (serde, Value, CST, SAX). The test55 (SQLite) or autochop09 (hub-and-spoke) patterns from `docs/diagrams/reference/` are good starting points.
3. **Try agent teams** on the next multi-task session — enabled but untested. Default `auto` teammate mode (in-process unless inside tmux).

## Decisions made

- **`Bash(*)` with deny list** over granular allow patterns — the allow-list approach fundamentally cannot handle shell constructs like `$()` substitution or variable expansion. Deny list covers actual dangerous operations.
- **Agent teams over session-driver** for future parallel work — first-party, less ceremony, shared task lists, direct teammate messaging.
- **Pikchr for diagrams** — vector, version-controllable, iteratable via MCP server. Multiple diagrams needed: spec model (done), event fan-out (next), resolver middleware (later).

## Landmines

- **Settings changes mixed with scanner changes** in `git diff` — the worker shared the working directory and picked up our `.claude/settings.json` edits. Stage scanner files separately: `crates/yamalgam-scanner/src/scanner.rs`, `crates/yamalgam-scanner/tests/scanner.rs`, `crates/yamalgam-compare/tests/compliance.rs`.
- **`tab_in_preceding_whitespace`** is a new field on `Scanner` — it persists across `scan_to_next_token` calls and is cleared on newline consumption. If you add new whitespace-consuming code paths, this flag needs to be considered.
- **EXPECTED count went from 25 to 30** — 5 cases moved from UNEXPECTED (Rust accepts, C rejects) to EXPECTED (C accepts, Rust rejects) or PASS. This is correct: our scanner now rejects these inputs, matching C.
- **`scripts/extract-test-yaml.py`** is not yet in `.gitignore` or committed — decide whether it belongs in the repo or is a dev-only utility.

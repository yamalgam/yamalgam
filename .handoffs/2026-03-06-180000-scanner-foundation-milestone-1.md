# Handoff: Scanner Foundation (Milestone 1)

**Date:** 2026-03-06
**Branch:** `main`
**State:** Green

> Green = tests pass, safe to continue.

## Where things stand

Milestone 1 is complete: 7 PRs merged to main. The scanner has core types, input layer (BOM detection, UTF-8/16/32 transcoding), and comparison infrastructure (C harness, comparison library with 351 YAML Test Suite cases, MCP server with 3 tools). The scanner has no state machine yet — all compliance tests produce `CSuccessRustError`. `just check` passes clean.

## Decisions made

- Hybrid porting approach: libfyaml state machine faithful, data structures idiomatic Rust — [ADR-0001](../docs/decisions/0001-port-libfyaml-state-machine-redesign-data-structures.md)
- Atom concept preserved for CST round-trip fidelity — [ADR-0002](../docs/decisions/0002-preserve-atom-concept-for-cst-fidelity.md)
- Comparison MCP server as primary correctness tool — [ADR-0003](../docs/decisions/0003-comparison-mcp-server-for-correctness-verification.md)
- UTF-8/16/32 encoding from day one — [ADR-0004](../docs/decisions/0004-utf8-16-32-encoding-support-from-day-one.md)
- `TokenKind::Directive` split into `VersionDirective`/`TagDirective` to match libfyaml's `FYTT_VERSION_DIRECTIVE`/`FYTT_TAG_DIRECTIVE`
- Observability module moved to `yamalgam-core` so CLI and MCP server share JSONL structured logging

## What's next

1. **Start scanner state machine porting** — begin with stream markers (`StreamStart`/`StreamEnd`), branch `feat/scanner/stream-markers`. Reference: `vendor/libfyaml-0.9.5/src/lib/fy-parse.c` and `fy-parse.h` for `enum fy_parser_state`. See [implementation plan](../docs/plans/2026-03-06-scanner-foundation-plan.md) Tasks 8-18 for the full porting sequence.
2. **Wire the MCP server into Claude Code** — add to `.claude/mcp.json` so the comparison tools are available during development sessions.
3. **Track compliance counts per PR** — each scanner state machine PR should report N/351 passing in the commit message.

## Landmines

- **C harness must be built first:** `cd tools/fyaml-tokenize && make` before running compliance tests. The comparison library looks for the `fyaml-tokenize` binary via `FYAML_TOKENIZE_PATH` env var or by walking up from the crate directory.
- **`add-crate` script prompts interactively** for proc-macro even when all args are provided for internal crates. Pipe `echo "n" |` or add `--derive` explicitly.
- **Token comparison ignores spans currently** — only kind/value/style are compared. Span accuracy tracking will need to be added when the state machine is further along.
- **libfyaml leaks native error lines to stderr** alongside the C harness's JSON error output. The comparison library should parse JSON lines from stderr and ignore non-JSON lines.

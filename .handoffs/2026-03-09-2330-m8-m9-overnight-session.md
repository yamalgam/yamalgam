# Handoff: M8 + M9 overnight session

**Date:** 2026-03-09
**Branch:** `main` (clean, PRs #78 and #79 merged)
**State:** Green — 819 tests, `just check` passes, no uncommitted work.

## Where things stand

M7 (Tag Resolution) is complete. Two additional PRs landed this session:
- **PR #78:** Scanner emits `TokenKind::Comment` inline in the token stream (first step of full-fidelity event stream). Parser skips them transparently.
- **PR #79:** Archived libfyaml. Removed 34MB of C source, C harness, and all comparison infrastructure. Compliance tests now compare directly against the YAML Test Suite `tree` field. No C toolchain dependency.

The project is at 819 tests, all passing. Vendor directory is 2.2MB (yaml-test-suite only).

## What Clay wants

An autonomous overnight session (or series of sessions) executing M8 and M9 with subagent-driven development. Between M8 and M9, run a performance tuning pass. Clay wants to sleep while this runs.

**Execution model:** You are Surrogate Clay. Follow the `using-git-like-clay` skill for all git operations. Use subagent-driven-development with TDD and code reviews. Commit frequently, push PRs. Do not cut corners.

### M8: CST (Concrete Syntax Tree)

**Prereqs (do these first):**
1. Parser: emit `Event::Comment` (pass Comment tokens through as events, not skip)
2. Parser: emit structural events (`BlockEntry`, `KeyIndicator`, `ValueIndicator`) after using them for state transitions
3. Composer: skip the new event variants (backward compatible)
4. Compliance harness: filter new events from expected tree comparison

Design doc: `docs/plans/2026-03-09-full-fidelity-event-stream-design.md`

**Then CST itself:**
- Arena vs Box allocation decision
- Trivia attachment model (comments, whitespace)
- Error nodes for partial parses (LSP error recovery)
- CST builder as an event consumer (same pipeline as Value, serde, SAX)
- Round-trip: parse → mutate → emit with comments intact

Roadmap entry: `docs/plans/README.md` lines 80-89

### M9: Streaming serde Deserializer

- `yamalgam::from_str::<T>()` for library consumers
- Erased-serde pattern internally — parser never monomorphizes
- Zero materialization for large files
- Standard serde integration tests

Roadmap entry: `docs/plans/README.md` lines 91-94

### Performance tuning (between M8 and M9)

- `yamalgam-bench` crate has 27 divan benchmarks, 7 peers, 3 input sizes
- yamalgam is fastest on small/medium, loses on large input
- Investigate: allocation patterns, string copying, iterator overhead
- Scanner is the hot path — focus there
- Goal: fastest by a clear margin, not just competitive

## Decisions made

- **Milestone order swap confirmed:** CST is M8, yg CLI is M10 (per `docs/plans/README.md`)
- **libfyaml comparison removed entirely** — compliance tests now use YAML Test Suite `tree` field directly. `yamalgam-compare` crate kept for future cross-implementation comparison (yamlstar, nodejs yaml, etc.)
- **Comment tokens are NOT content tokens** — `is_content()` returns false, no impact on simple keys or indent management

## Landmines

- **Full-fidelity event stream steps 2-5 must come BEFORE CST design.** The CST consumes the enriched event stream. Without `Event::Comment` and structural events, the CST can't preserve comments or indicators. Don't jump straight to CST node design.
- **Compliance tests filter yamalgam-only events.** The new compliance harness in `compliance.rs` compares against YAML Test Suite `tree` format, which doesn't include Comment or structural events. When adding `Event::Comment` to the parser, filter it in the compliance test's `event_to_snapshot()` (returns `None` for yamalgam-only events).
- **`compare.rs` still has C-flavored field names** in `CompareResult` and `CompareEventResult` (e.g., `c_error`, `c_token`). These are cosmetic — they work fine with any two implementations. Rename if it bothers you, but not a blocker.
- **Scanner `scan_to_next_token` comment emission** happens inside the main whitespace-skipping loop. Block scalar headers also consume comments (via `skip_to_next_line` at scanner.rs:1765). Those header comments are NOT emitted as tokens yet — they're structurally bound to the scalar. Address in CST trivia design if needed.
- **`just check` is the gate.** Run it before every PR. It includes fmt + clippy + deny + nextest + doc-tests. Don't use `cargo test` — use `cargo nextest run`.
- **MEMORY.md is over 200 lines.** Only the first 200 are loaded. Move detailed content to separate topic files if you need to update memory.

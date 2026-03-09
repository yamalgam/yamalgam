# Handoff: M7 Complete + Full-Fidelity Event Stream Architecture

**Date:** 2026-03-09
**Branch:** `main` (PR #77 merged, uncommitted design docs)
**State:** Yellow — M7 shipped, but M8 prerequisites identified. Design docs uncommitted.

## Where things stand

M7 (Tag Resolution Trait) is merged. `TagResolver` trait, `TagResolution` enum, four built-in resolvers, wired into Composer and LoaderConfig. 1106 tests, `just check` green.

During CST brainstorming, a foundational architectural gap was identified: the parser event model discards structural tokens (BlockEntry `-`, Key `?`, Value `:` positions) and comments. This makes CST, SAX, and streaming yg queries impossible from the event stream alone. A corrected architecture design is written but not yet committed.

## What was done

### M7: Tag Resolution Trait (PR #77)
- `TagResolver` trait + `TagResolution` enum in `yamalgam-core::tag_resolution`
- Built-in resolvers: `Yaml12TagResolver`, `FailsafeTagResolver`, `JsonTagResolver`, `Yaml11TagResolver`
- `LoaderConfig.tag_resolution` field (Copy, const-fn safe)
- Composer: `Box<dyn TagResolver>`, `from_str_with_tag_resolver()` for custom resolvers
- Code review found + fixed JSON Schema leading-zero integer bug

### Full-fidelity event stream design (uncommitted)
- `docs/plans/2026-03-09-full-fidelity-event-stream-design.md` — corrects M6 architecture
- `docs/plans/2026-03-09-tag-resolution-trait-design.md` + `...-plan.md` — M7 design/plan docs
- `docs/plans/README.md` — M7 marked complete, M8/M10 swapped (CST before yg CLI)

## Decisions made

- **"Tag resolution" not "schema"** — avoids collision with validation schemas (M12). `TagResolver` not `SchemaResolver`, `Yaml12` not `Core`.
- **M8 and M10 swapped** — CST (formerly M10) is now M8. yg CLI (formerly M8) is now M10. CST is foundational infrastructure; CLI can be built on whatever layers exist.
- **Full-fidelity event stream** — the emitter preserves everything, the receiver decides what to ignore. Comments and structural indicators become first-class events. Supersedes the M4 plan for CST to "consume the full token stream directly" and corrects the M6 architecture table's "Preserves comments?" column.

## What's next

1. **Commit design docs** — the three plan files and README update are uncommitted on main.
2. **Full-fidelity event stream (M8 prerequisite):**
   - Scanner: add `TokenKind::Comment`, emit inline instead of discarding
   - Parser: add `Event::Comment`, `Event::BlockEntry`, `Event::KeyIndicator`, `Event::ValueIndicator`
   - Composer: skip new events (backward compatible)
   - Compliance harness: filter new events from libfyaml comparison
3. **CST node design** — after event stream is enriched: arena vs Box, trivia attachment, error recovery.

## Landmines

- **Uncommitted files on main.** `git status` shows modified `docs/plans/README.md` and three untracked plan files. Commit these before branching for event stream work.
- **The M6 architecture diagram (`docs/diagrams/event-stream-architecture.svg`) is now wrong.** It shows CST as a peer event consumer with "Preserves comments? Yes" — but the event stream doesn't carry comments yet. Update the diagram after the event stream enrichment.
- **ADR-0005 needs an amendment**, not replacement. The neutral consequence about comment handling (line 72) should be updated per the full-fidelity design doc's §Corrections section.
- **libfyaml's `fy_comment_placement` (top/right/bottom)** is a useful concept for CST trivia attachment. The C implementation is in `fy-token.h:122-128` and `libfyaml.h:1452-1456`. Worth studying when designing comment attachment to CST nodes.
- **Scanner `scan_to_next_token()` (scanner.rs:307)** is where comments are currently consumed and discarded. The comment-parsing loop is at lines 375-378. This is the code that needs to emit `Comment` tokens instead of silently advancing.

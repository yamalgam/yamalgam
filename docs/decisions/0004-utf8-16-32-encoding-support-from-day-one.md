---
status: accepted
date: 2026-03-06
decision-makers: [Clay Loveless]
---

# 0004: Support UTF-8, UTF-16, and UTF-32 encoding from day one

## Context and Problem Statement

The YAML spec requires parsers to handle UTF-8, UTF-16, and UTF-32 encoded input, with BOM detection to identify the encoding. In practice, nearly all YAML is UTF-8. Should we support all encodings from the start, or ship UTF-8-only and add the rest later?

## Decision Drivers

- The YAML Test Suite includes UTF-16 and UTF-32 test cases — we cannot achieve 100% compliance without encoding support
- We committed to building the foundation right, not patching it later
- The encoding boundary belongs at the input layer, not scattered through the scanner
- `encoding_rs` is a mature, well-maintained crate that handles the transcoding

## Considered Options

- **Option A: UTF-8 only** — detect non-UTF-8 BOM and reject with a clear error message
- **Option B: Full encoding support from day one** — BOM detection, transcode to internal UTF-8 at the input boundary

## Decision Outcome

Chosen option: "Full encoding support from day one (Option B)," because the YAML Test Suite includes non-UTF-8 test cases. Deferring encoding support means accepting known test failures in the foundation — the opposite of building it right.

The encoding boundary is at the input layer: `Input::from_bytes()` detects BOM, transcodes to UTF-8 if necessary, and all downstream code operates on `&str`. This keeps encoding concerns out of the scanner state machine entirely.

### Consequences

- Good, because we can target 100% YAML Test Suite compliance from the first scanner milestone
- Good, because the encoding boundary is clean — one place in the codebase, not scattered through the scanner
- Good, because `encoding_rs` is battle-tested (used by Firefox) and adds minimal dependency weight
- Bad, because `encoding_rs` is an additional dependency that most users will never exercise
- Bad, because the transcode path cannot borrow from the original input — non-UTF-8 input always takes the owned `Cow::Owned` path, which is slightly slower

### Confirmation

UTF-16 and UTF-32 YAML Test Suite cases pass through the scanner and produce correct tokens. The comparison MCP server verifies these cases against libfyaml's output.

## Pros and Cons of the Options

### Option A: UTF-8 only

Detect non-UTF-8 BOM and return a `Diagnostic` error suggesting the user transcode first.

- Good, because no `encoding_rs` dependency
- Good, because simpler input layer
- Bad, because known test suite failures from day one — we'd be shipping a scanner we know is incomplete
- Bad, because adding encoding support later requires restructuring the input layer (the `Cow` lifetime story changes when owned buffers are introduced)

### Option B: Full encoding support (chosen)

BOM detection at input boundary, transcode to UTF-8, scanner operates on `&str`.

- Good, because complete — no known gaps in encoding handling
- Good, because the input layer is designed for both borrowed and owned paths from the start
- Bad, because adds `encoding_rs` dependency (~150KB, no transitive deps)
- Bad, because UTF-16/32 paths are rarely exercised in real-world usage

## More Information

- YAML spec encoding requirements: [YAML 1.2.2, Section 5.2](https://yaml.org/spec/1.2.2/#52-character-encodings)
- `encoding_rs` crate: [docs.rs/encoding_rs](https://docs.rs/encoding_rs)
- Design doc: [`docs/plans/2026-03-06-scanner-foundation-design.md`](../plans/2026-03-06-scanner-foundation-design.md)

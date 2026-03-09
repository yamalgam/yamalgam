# Full-fidelity event stream — corrected architecture

**Date:** 2026-03-09
**Status:** Draft
**Supersedes:** Consumer table in `2026-03-08-event-consumers-and-resolver-design.md`, partial correction to ADR-0005 §Consequences

## The problem we created

The parser event model (ADR-0005, M4) followed the libyaml/saphyr/PyYAML
pattern: emit semantic events, discard structural detail. Comments get
swallowed by the scanner. Block entry dashes, key indicators, value
indicators — the parser consumes them to figure out structure, then throws
them away before emitting `SequenceStart` and `MappingStart`.

Every other YAML library does this. That's exactly why every other YAML
library can't round-trip a file without destroying the author's formatting.

yamalgam was supposed to be different. ADR-0002 preserved atom-level
presentation metadata. ADR-0005 preserved directives as events. But the
parser still strips structural tokens, and the scanner still swallows
comments. The M6 architecture diagram put CST as a "peer consumer" of
events alongside Value and serde — but the events don't carry enough
information for the CST to do its job.

The M4 parser design doc (§Comments — Future Plan) tried to fix this by
having the CST builder consume tokens directly, bypassing the event
stream. That's the wrong answer. It creates two parallel pipelines —
tokens for structural consumers, events for semantic consumers — and
the resolver middleware only works on events. Now the CST can't benefit
from `!include` resolution, and SAX callbacks can't see comments.

## The principle

**The emitter preserves everything. The receiver decides what to ignore.**

YAML is a human-first format. Comments, blank lines, indentation choices,
quoting style — these are all conscious decisions the author made. Any tool
that strips them is telling the author their choices don't matter.

If you don't care about comments, skip the `Comment` event. If you don't
care about structural indicators, skip `BlockEntry`. But the events are
there, in the stream, for consumers that do care. One pipeline, full
fidelity, all the way through.

## The corrected architecture

```
YAML bytes
    |
yamalgam-scanner (tokens — including Comment)
    |
yamalgam-parser (events — including Comment, structural indicators)
    |
resolver middleware (trait-based, composable — passes through all events)
    |
    +-- Streaming serde Deserializer   (skips Comment, structural)
    +-- Value / DOM                    (skips Comment, structural)
    +-- CST                            (consumes everything)
    +-- SAX / Callbacks                (consumes everything)
    +-- yg streaming queries           (consumes everything)
```

**There is no "Preserves comments?" column.** Every consumer gets comments.
Value and serde choose to ignore them. CST, SAX, and yg choose to use them.
That's the consumer's business, not the pipeline's.

## What changes

### Scanner: emit Comment tokens

Add `TokenKind::Comment` to the scanner. When `scan_to_next_token()` hits
`#` preceded by whitespace, instead of consuming the comment text and
discarding it, emit a `Comment` token with the full text (including the
`#`) and its span.

The scanner already knows where comments start and end — it already
consumes them character by character (scanner.rs:375-378). The change is
to capture what it currently discards.

Comments are emitted inline in the token stream, in source order, between
the structural tokens they appear between. No sidecar. No second pass.

### Parser: pass through Comment events + emit structural events

**New event variants:**

```rust
/// A YAML comment (text includes the `#` prefix).
Comment {
    /// The comment text, including `#`.
    text: Cow<'input, str>,
    /// Source span.
    span: Span,
},

/// `-` block sequence entry indicator.
BlockEntry {
    /// Source span of the `-`.
    span: Span,
},

/// `?` explicit key indicator.
KeyIndicator {
    /// Source span of the `?`.
    span: Span,
},

/// `:` value indicator.
ValueIndicator {
    /// Source span of the `:`.
    span: Span,
},
```

The parser currently consumes `TokenKind::BlockEntry`, `TokenKind::Key`,
and `TokenKind::Value` tokens to drive its state machine. After using them
for state transitions, it now also emits them as events so downstream
consumers see them.

`Comment` tokens pass straight through — the parser doesn't need them for
state management, but it must not swallow them.

### Whitespace: spans are sufficient

Explicit `Whitespace` events are not needed. Token spans encode exact byte
offsets. The region between two consecutive token spans in the source text
is whitespace (spaces, tabs, newlines, blank lines). Any consumer that
needs whitespace content can derive it from `source[prev_span.end.offset..next_span.start.offset]`.

This keeps the event stream lean. Comments and structural indicators are
events because they carry semantic content that isn't recoverable from
position alone. Whitespace is pure position — and position is already
encoded.

### Composer: skip new events

The Composer (Value builder) adds matches for `Comment`, `BlockEntry`,
`KeyIndicator`, and `ValueIndicator` in its event consumption loop —
and skips them. Existing behavior preserved. No API change for Value
consumers.

### Resolver middleware: pass through

The resolver trait already operates on `Event` iterators. New event
variants pass through unchanged — resolvers that don't know about them
ignore them naturally (they're not tags, not nodes, not structural
boundaries that affect resolution logic).

## What this enables

- **yg comment queries**: `yg '.[] | comments'` — stream through a file,
  extract comments without materializing a tree
- **Streaming round-trip edits**: modify values in a large YAML file without
  loading it all into memory, while preserving comments and formatting
- **SAX with full fidelity**: callback-based processing that sees every
  `-`, `:`, `?`, and `#` in source order
- **CST as an event consumer**: CST builder works from the same event stream
  as every other consumer. One pipeline. Resolver middleware just works.
- **Comment-aware linting**: stream-based lint rules that check comment
  placement, style, or content

## What this does NOT change

- **Scanner token types for non-comment whitespace** — whitespace stays
  implicit (span gaps). If we find a concrete use case that requires
  explicit whitespace events, we add them then.
- **Atom metadata** — AtomFlags, ScalarStyle, Chomp stay exactly as
  designed in ADR-0002. They complement structural events, not replace them.
- **Resolver trait signature** — `Resolver<'input>` operates on
  `Iterator<Item = Result<Event, ResolveError>>`. Adding event variants
  to the enum is backward-compatible.
- **Existing compliance tests** — new events are additive. Token and event
  compliance tests compare against libfyaml's output, which doesn't
  include comments or structural indicators. The comparison harness
  filters them.

## Corrections to prior documents

### M6 design doc consumer table (replace)

| API | Materializes? | Primary use case |
|-----|:---:|---|
| Streaming serde | No | `from_str::<T>()`, large files, library consumers |
| Value (DOM) | Yes (lossy) | Data extraction, `yg get`, serde two-phase |
| CST (lossless) | Yes (lossless) | Round-trip editing, `yg set`, LSP, linter, emitter |
| SAX / Callbacks | No | FFI, WASM, large-file streaming |

The "Preserves comments?" column is removed. All consumers receive
comment events. Whether they use them is their business.

### ADR-0005 neutral consequence (amend)

The original text:
> Comment handling deferred — scanner will emit TokenKind::Comment inline
> when CST milestone begins; parser will skip them

Amended to:
> Comment handling deferred to CST milestone. Scanner will emit
> `TokenKind::Comment` inline. **Parser will emit `Comment` events** —
> it does not skip or consume them. Structural indicators (`BlockEntry`,
> `KeyIndicator`, `ValueIndicator`) are also emitted as events after being
> used for state transitions. Consumers that don't need structural events
> skip them.

### M4 parser design doc §Comments (replace)

The original plan for CST to "consume the full token stream" directly is
superseded. CST consumes the enriched event stream like every other
consumer. One pipeline.

## Implementation order

1. **Scanner: `TokenKind::Comment`** — capture comment text and span
   instead of discarding. Emit inline.
2. **Parser: `Event::Comment`** — pass Comment tokens through as events.
3. **Parser: structural events** — emit `BlockEntry`, `KeyIndicator`,
   `ValueIndicator` events after using their tokens for state transitions.
4. **Composer: skip new events** — update `compose_node()` and
   `compose_stream()` to skip the new event variants. Existing tests pass.
5. **Compliance harness: filter new events** — comparison against libfyaml
   must exclude Comment and structural events (libfyaml doesn't emit them).
6. **CST design** — now that the event stream carries full fidelity, design
   the CST node representation, trivia attachment model, and error recovery.

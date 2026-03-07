# Parser Layer Design

**Date:** 2026-03-07
**Status:** Approved
**Depends on:** Scanner (complete, 343/351 compliance)

## Overview

The parser is a **pull parser** (StAX-style) — a hand-rolled state machine where the caller drives iteration. It is the primary interface for streaming YAML processing, not an intermediate layer that feeds a tree builder. Tree-building (DOM, CST) is an optional consumer above it.

This is the second layer in yamalgam's pipeline: scanner (tokens) -> parser (events). Consumers choose what to do with the event stream:

```
yamalgam-parser (pull parser — Iterator<Event>)
       |
       |--- yg: streaming queries, never builds a tree
       |--- merge-all-of: stream-validate, track state, bail early
       |--- serde Deserializer: event-to-struct, no intermediate tree
       |--- yglint: streaming validation rules
       |
       +--- composer/builder (optional consumer, builds tree)
              |--- DOM: node tree for programmatic access
              |--- CST: full-fidelity tree with comments/style
```

The pull parser is the foundation. Streaming consumers (yg, linting, serde, validation) use it directly and never pay the cost of tree construction. Only DOM/CST consumers build trees, and only when explicitly requested.

No semantic interpretation, no anchor resolution, no merge key expansion — the parser is a pure state machine that transforms tokens into events.

## Crate Structure

New crate: `yamalgam-parser` under `crates/`.

**Dependencies:** `yamalgam-scanner`, `yamalgam-core` (for `Mark`, `Span`), `thiserror`.

**Workspace graph:**
```
yamalgam-scanner (tokens)
       |
yamalgam-parser (pull parser, events)
       |
       +--- direct consumers: yg, serde, linter, validators
       +--- yamalgam-core (composer, DOM, CST — future)
       |
yamalgam (CLI)
```

## Event Model

Flat event enum with anchors and tags attached to node events (libfyaml model). Directives are separate events — they are not consumed or hidden from consumers.

```rust
pub enum Event<'input> {
    StreamStart,
    StreamEnd,

    // Directives — separate events, not attached to DocumentStart
    VersionDirective { major: u8, minor: u8, span: Span },
    TagDirective { handle: Cow<'input, str>, prefix: Cow<'input, str>, span: Span },

    DocumentStart { implicit: bool, span: Span },
    DocumentEnd { implicit: bool, span: Span },

    SequenceStart { anchor: Option<Cow<'input, str>>, tag: Option<Cow<'input, str>>, style: CollectionStyle, span: Span },
    SequenceEnd { span: Span },

    MappingStart { anchor: Option<Cow<'input, str>>, tag: Option<Cow<'input, str>>, style: CollectionStyle, span: Span },
    MappingEnd { span: Span },

    Scalar { anchor: Option<Cow<'input, str>>, tag: Option<Cow<'input, str>>, value: Cow<'input, str>, style: ScalarStyle, span: Span },

    Alias { name: Cow<'input, str>, span: Span },
}

pub enum CollectionStyle {
    Block,
    Flow,
}
```

### Why directives are separate events

Most YAML libraries consume directives during `DocumentStart` processing and attach them as metadata. This destroys information — a CST layer cannot reconstruct the original directive text, ordering, or position without it. yamalgam preserves everything: directives are events in the stream, emitted in source order, before `DocumentStart`. Consumers that don't care (serde, DOM) skip them. CST gets full fidelity.

See ADR-0005 for the event model decision and the alternative considered.

## Public API

```rust
impl<'input> Parser<'input> {
    /// Common path: parser owns a scanner over input
    pub fn new(input: &'input str) -> Self;

    /// Advanced: parser wraps an existing token iterator
    pub fn from_tokens(tokens: impl Iterator<Item = Result<Token<'input>, ScanError>> + 'input) -> Self;
}

impl<'input> Iterator for Parser<'input> {
    type Item = Result<Event<'input>, ParseError>;
}
```

The parser is a standard Rust `Iterator` — the pull parser primitive. Composable with the full iterator ecosystem: `take_while()`, `filter()`, `map()`, early `break`. Streaming consumers never allocate a tree; they process events as they arrive and discard them.

## Error Handling

`ParseError` uses structured variants with `thiserror`, wrapping `ScanError` for scanner-originated errors.

```rust
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error(transparent)]
    Scan(#[from] ScanError),

    #[error("unexpected {got:?}, expected {expected} at {span}")]
    UnexpectedToken { expected: &'static str, got: TokenKind, span: Span },

    #[error("unexpected end of input, expected {expected} at {span}")]
    UnexpectedEof { expected: &'static str, span: Span },

    #[error("duplicate %YAML directive at {span}")]
    DuplicateVersionDirective { span: Span },

    #[error("duplicate %TAG directive for handle {handle:?} at {span}")]
    DuplicateTagDirective { handle: String, span: Span },

    // Additional variants discovered during implementation
}
```

Principle: every error carries its span and structured context. No bare string messages.

## State Machine

~26 states matching libfyaml's `fy_parser_state` enum, stack-based (no recursion). All state handlers annotated with `// cref: fy-parse.c:NNNN`.

**State groups:**
- **Stream:** `StreamStart`, `End`
- **Document:** `ImplicitDocumentStart`, `DocumentStart`, `DocumentContent`, `DocumentEnd`
- **Block:** `BlockNode`, `BlockSequenceFirstEntry`, `BlockSequenceEntry`, `IndentlessSequenceEntry`, `BlockMappingFirstKey`, `BlockMappingKey`, `BlockMappingValue`
- **Flow:** `FlowSequenceFirstEntry`, `FlowSequenceEntry`, `FlowSequenceEntryMappingKey`, `FlowSequenceEntryMappingValue`, `FlowSequenceEntryMappingEnd`, `FlowMappingFirstKey`, `FlowMappingKey`, `FlowMappingValue`, `FlowMappingEmptyValue`

**State stack:** `Vec<ParserState>` — push on entering collections, pop on exiting.

## Scope Boundary

The parser is **only** a state machine. It does not:

- Maintain an anchor table or validate anchor uniqueness
- Resolve aliases to their targets
- Expand merge keys (`<<`)
- Process `!include` or `$ref`
- Build any tree structure

These are all composer/document-builder concerns for a future layer above the parser.

`<<` is emitted as `Scalar { value: "<<", ... }`. `!include` is emitted as a tag on a node event. Aliases are emitted as `Alias { name }` with no resolution.

## Comparison Harness

Extend the existing `tools/fyaml-tokenize/fyaml-tokenize` binary with a `--events` flag:
- Without flag: current behavior (token JSON output)
- With `--events`: run libfyaml's parser, output events as JSON

The `yamalgam-compare` crate gains a new compliance test that compares parser event streams (same PASS/UNEXPECTED/EXPECTED/MISMATCH categories).

## Comments — Future Plan

Not in this milestone. When we build the CST layer:

1. Scanner gains `TokenKind::Comment` — comments emitted inline in the token stream
2. Parser skips comment tokens (they don't affect event generation)
3. CST builder consumes the full token stream and attaches comments to nodes during single-pass tree construction

No sidecar index. No dual-index maintenance on mutations. Comments travel with their nodes in the CST.

## Testing Strategy

Three levels (same pattern as scanner):

1. **Unit tests** — synthetic token sequences via `from_tokens()`, verify event output
2. **YAML Test Suite compliance** — comparison harness with `--events`, all 351 cases
3. **`just check`** — fmt + clippy + deny + nextest + doc-test

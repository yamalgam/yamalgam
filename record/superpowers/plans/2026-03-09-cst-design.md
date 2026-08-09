# CST (Concrete Syntax Tree) — Design

**Date:** 2026-03-09
**Status:** Accepted (autonomous overnight session)
**Milestone:** M8
**Location:** `yamalgam-parser::cst`

## Problem

yamalgam has a lossy Value DOM (Composer) and a streaming event API. Neither
preserves the original source formatting: Value drops comments, whitespace,
quoting style; events are transient. To support round-trip editing (`yg -i`),
linting, LSP, and pretty emission, we need a lossless tree representation.

## Decisions

### Box allocation, not arena

Owned `Vec<CstElement>` children on each node. No arena, no indices, no
unsafe. If profiling shows allocation pressure matters, add arena later.
Premature optimization would complicate the API for no proven benefit.

### Whitespace from span gaps

The event stream carries byte-accurate spans on every event. The region
`source[prev_span.end.offset..next_span.start.offset]` is whitespace
(spaces, tabs, newlines). The CstBuilder holds `&'input str` and inserts
`CstToken::Whitespace` elements for these gaps. No explicit whitespace
events needed in the parser.

### Comments as regular tokens

`Event::Comment` becomes `CstToken { kind: Comment, .. }` in the tree,
positioned exactly where the scanner encountered it. No "leading/trailing
trivia" model — comments are children of their containing node, in source
order. Consumers that want to associate a comment with a specific node can
check adjacency via spans.

### Error nodes for partial parses

`CstNode::Error { message, children }` wraps content the parser couldn't
make sense of. The builder catches `ParseError` from the event stream and
wraps the remaining text in an error node rather than aborting. This enables
LSP to provide partial diagnostics on malformed files.

### Round-trip guarantee

`cst.to_string()` concatenates all leaf token texts in tree order and
reproduces the original source byte-for-byte. This is the correctness
invariant for the entire CST.

## Data Model

```rust
/// Interior node in the CST.
pub struct CstNode<'input> {
    pub kind: NodeKind,
    pub span: Span,
    pub children: Vec<CstElement<'input>>,
}

/// Leaf token in the CST.
pub struct CstToken<'input> {
    pub kind: TokenKind,
    pub text: Cow<'input, str>,
    pub span: Span,
}

/// A child element — either an interior node or a leaf token.
pub enum CstElement<'input> {
    Node(CstNode<'input>),
    Token(CstToken<'input>),
}

/// Interior node kinds.
pub enum NodeKind {
    /// Root of the CST (one per input).
    Stream,
    /// Directive prologue (before `---`).
    Directives,
    /// A YAML document (implicit or explicit).
    Document,
    /// Block or flow mapping.
    Mapping,
    /// One key-value pair inside a mapping.
    MappingEntry,
    /// Block or flow sequence.
    Sequence,
    /// One entry inside a sequence.
    SequenceEntry,
    /// Anchor + tag property group on a node.
    Properties,
    /// Error recovery — wraps unparseable content.
    Error,
}

/// Leaf token kinds.
pub enum TokenKind {
    // -- Indicators --
    BlockEntry,       // -
    Key,              // ?
    Value,            // :
    DocumentStart,    // ---
    DocumentEnd,      // ...
    FlowSeqStart,     // [
    FlowSeqEnd,       // ]
    FlowMapStart,     // {
    FlowMapEnd,       // }
    FlowEntry,        // ,

    // -- Content --
    Scalar,           // plain, quoted, or block scalar text
    Anchor,           // &name
    Alias,            // *name
    Tag,              // !tag or !!tag or !prefix!suffix

    // -- Directives --
    VersionDirective, // %YAML x.y
    TagDirective,     // %TAG !handle! prefix

    // -- Trivia --
    Comment,          // # text
    Whitespace,       // spaces, tabs, newlines between tokens

    // -- Error --
    ErrorToken,       // unrecognized content in error recovery
}
```

## CstBuilder Algorithm

The builder consumes the enriched event stream and builds a tree:

```
1. Initialize stack with a Stream node
2. For each event:
   a. Insert whitespace token for gap between last_offset and event span
   b. Match event:
      - StreamStart/StreamEnd: open/close Stream
      - DocumentStart/End: open/close Document
      - SequenceStart/End: open/close Sequence
      - MappingStart/End: open/close Mapping
      - BlockEntry: close previous SequenceEntry (if any), open new one
      - KeyIndicator: close previous MappingEntry (if any), open new one
      - ValueIndicator: add token to current MappingEntry
      - Scalar: add token leaf
      - Alias: add token leaf
      - Comment: add token to current node
      - VersionDirective/TagDirective: add token
   c. Update last_offset to event span end
3. Close remaining open nodes
4. Insert trailing whitespace
```

The builder is fallible: when it encounters a `ParseError` from the event
stream, it wraps the remaining source text in an error node and closes the
tree. This produces a partial-but-valid CST.

## Builder API

```rust
pub struct CstBuilder<'input> {
    source: &'input str,
    stack: Vec<CstNode<'input>>,
    last_offset: usize,
}

impl<'input> CstBuilder<'input> {
    pub fn new(source: &'input str) -> Self;

    pub fn build(
        self,
        events: impl Iterator<Item = Result<Event<'input>, ParseError>>,
    ) -> CstNode<'input>;
}

/// Convenience: parse source to CST in one call.
pub fn parse_to_cst(source: &str) -> CstNode<'_>;
```

## CstNode API

```rust
impl<'input> CstNode<'input> {
    /// Concatenate all leaf token texts — reproduces original source.
    pub fn to_text(&self) -> String;

    /// Iterate all direct children.
    pub fn children(&self) -> &[CstElement<'input>];

    /// Find first child node of a given kind.
    pub fn child_node(&self, kind: NodeKind) -> Option<&CstNode<'input>>;

    /// Find the node at a given byte offset (for LSP cursor).
    pub fn node_at_offset(&self, offset: usize) -> Option<&CstNode<'input>>;

    /// Iterate all descendant nodes depth-first.
    pub fn descendants(&self) -> impl Iterator<Item = &CstElement<'input>>;
}
```

## What This Does NOT Include (M8 scope)

- **Incremental reparsing** — deferred to M12 (LSP). Requires green/red tree
  split (rowan pattern). Not needed until we have a language server.
- **Mutation API** — `set_value()`, `insert_key()`, etc. Deferred to M10
  (`yg -i`). M8 builds read-only CST + `to_text()` round-trip.
- **Emitter** — deferred to M13. For now, `to_text()` is byte-for-byte
  source reconstruction, not pretty-printing.

## Testing Strategy

1. **Round-trip property test**: for every YAML Test Suite case,
   `parse_to_cst(input).to_text() == input`.
2. **Node structure tests**: verify correct nesting (Document contains
   Mapping, Mapping contains MappingEntry, etc.).
3. **Comment preservation**: verify comments appear in correct tree position.
4. **Error recovery**: verify partial CST is produced on malformed input.
5. **Span accuracy**: verify every leaf token's span matches its text.

## Implementation Order

1. Data types (`CstNode`, `CstToken`, `CstElement`, `NodeKind`, `TokenKind`)
2. `to_text()` method (the round-trip primitive)
3. `CstBuilder` — basic structure (Stream, Document, Scalar, Alias)
4. CstBuilder — collections (Mapping, MappingEntry, Sequence, SequenceEntry)
5. CstBuilder — whitespace insertion from span gaps
6. CstBuilder — comment tokens
7. CstBuilder — error recovery
8. `parse_to_cst()` convenience function
9. Round-trip compliance test (YAML Test Suite)
10. Navigation API (`node_at_offset`, `descendants`)

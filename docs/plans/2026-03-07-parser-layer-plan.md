# Parser Layer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a pull parser (StAX-style) that consumes scanner tokens and emits YAML events as a Rust Iterator.

**Architecture:** Hand-rolled stack-based state machine ported from libfyaml's `fy_parse_internal()` (`fy-parse.c:6044-7060`). ~24 parser states, ~12 event types. Pure token-to-event translation with no semantic interpretation. All state handlers annotated with `// cref:` to libfyaml source.

**Tech Stack:** Rust (edition 2024), `yamalgam-scanner`, `yamalgam-core`, `thiserror`. No parser combinator libraries.

**Reference files:**
- Design: `docs/plans/2026-03-07-parser-layer-design.md`
- ADR: `docs/decisions/0005-parser-event-model-with-inline-directives.md`
- libfyaml parser: `vendor/libfyaml-0.9.5/src/lib/fy-parse.c` (9208 lines)
- libfyaml header: `vendor/libfyaml-0.9.5/src/lib/fy-parse.h` (states at line 86)
- libfyaml events: `vendor/libfyaml-0.9.5/include/libfyaml.h` (events at line 501)
- Scanner types: `crates/yamalgam-scanner/src/token.rs`, `crates/yamalgam-scanner/src/style.rs`
- Scanner API: `crates/yamalgam-scanner/src/scanner.rs` (Iterator impl at line 2283)
- Core types: `crates/yamalgam-core/src/diagnostic.rs` (Mark, Span)
- Comparison harness: `crates/yamalgam-compare/src/harness.rs`
- C harness: `tools/fyaml-tokenize/main.c`

---

## Task 1: Scaffold the yamalgam-parser crate

**Files:**
- Create: `crates/yamalgam-parser/Cargo.toml`
- Create: `crates/yamalgam-parser/src/lib.rs`
- Verify: root `Cargo.toml` (workspace members use glob `crates/*`, should auto-detect)

**Step 1: Create the crate directory**

```bash
mkdir -p crates/yamalgam-parser/src
```

**Step 2: Write Cargo.toml**

Create `crates/yamalgam-parser/Cargo.toml`:

```toml
[package]
name = "yamalgam-parser"
description = "YAML 1.2 pull parser — events from tokens"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
publish = false

[lints]
workspace = true

[dependencies]
thiserror = "2.0"
yamalgam-core = { path = "../yamalgam-core" }
yamalgam-scanner = { path = "../yamalgam-scanner" }

[dev-dependencies]
pretty_assertions = "1"
```

Note: Do NOT use `scripts/add-crate` — it has a bug where it prompts for proc-macro interactively on internal crates even with all args provided. Manual creation is cleaner here.

**Step 3: Write initial lib.rs**

Create `crates/yamalgam-parser/src/lib.rs`:

```rust
pub mod event;
pub mod error;
pub mod parser;
```

**Step 4: Verify it compiles**

Run: `cargo check -p yamalgam-parser`
Expected: success (will fail until we create the modules — that's the next task)

**Step 5: Commit**

```
feat(parser): scaffold yamalgam-parser crate
```

---

## Task 2: Define core types — Event, CollectionStyle, ParseError, ParserState

**Files:**
- Create: `crates/yamalgam-parser/src/event.rs`
- Create: `crates/yamalgam-parser/src/error.rs`
- Create: `crates/yamalgam-parser/src/parser.rs` (just the state enum for now)
- Modify: `crates/yamalgam-parser/src/lib.rs`
- Create: `crates/yamalgam-parser/tests/types.rs`

**Step 1: Write the failing test**

Create `crates/yamalgam-parser/tests/types.rs`:

```rust
use yamalgam_parser::event::{CollectionStyle, Event};
use yamalgam_parser::error::ParseError;

#[test]
fn event_variants_exist() {
    // Verify all event variants are constructable
    let _stream_start = Event::StreamStart;
    let _stream_end = Event::StreamEnd;
}

#[test]
fn collection_style_variants() {
    let _block = CollectionStyle::Block;
    let _flow = CollectionStyle::Flow;
}

#[test]
fn parse_error_from_scan_error() {
    use yamalgam_scanner::scanner::ScanError;
    let scan_err = ScanError {
        message: "test".to_string(),
    };
    let parse_err: ParseError = scan_err.into();
    assert!(matches!(parse_err, ParseError::Scan(_)));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo nextest run -p yamalgam-parser --test types`
Expected: FAIL — modules don't exist yet

**Step 3: Write event.rs**

Create `crates/yamalgam-parser/src/event.rs`:

```rust
use std::borrow::Cow;

use yamalgam_core::Span;
use yamalgam_scanner::ScalarStyle;

/// Style of a YAML collection (sequence or mapping).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionStyle {
    /// Block style (indentation-based).
    Block,
    /// Flow style (`[...]` or `{...}`).
    Flow,
}

/// A YAML parse event.
///
/// The parser is a pull parser (StAX-style): callers iterate events one at a
/// time. Anchors and tags are attached to node events. Directives are emitted
/// as separate events (not consumed into DocumentStart) to preserve full
/// fidelity for CST round-tripping.
// cref: libfyaml.h:501-513 (fy_event_type)
#[derive(Clone, Debug, PartialEq)]
pub enum Event<'input> {
    StreamStart,
    StreamEnd,

    // Directives — emitted as separate events before DocumentStart.
    // cref: fy-parse.c:6199-6227 (directive processing)
    VersionDirective {
        major: u8,
        minor: u8,
        span: Span,
    },
    TagDirective {
        handle: Cow<'input, str>,
        prefix: Cow<'input, str>,
        span: Span,
    },

    DocumentStart {
        implicit: bool,
        span: Span,
    },
    DocumentEnd {
        implicit: bool,
        span: Span,
    },

    SequenceStart {
        anchor: Option<Cow<'input, str>>,
        tag: Option<Cow<'input, str>>,
        style: CollectionStyle,
        span: Span,
    },
    SequenceEnd {
        span: Span,
    },

    MappingStart {
        anchor: Option<Cow<'input, str>>,
        tag: Option<Cow<'input, str>>,
        style: CollectionStyle,
        span: Span,
    },
    MappingEnd {
        span: Span,
    },

    Scalar {
        anchor: Option<Cow<'input, str>>,
        tag: Option<Cow<'input, str>>,
        value: Cow<'input, str>,
        style: ScalarStyle,
        span: Span,
    },

    Alias {
        name: Cow<'input, str>,
        span: Span,
    },
}
```

**Step 4: Write error.rs**

Create `crates/yamalgam-parser/src/error.rs`:

```rust
use yamalgam_core::Span;
use yamalgam_scanner::{scanner::ScanError, TokenKind};

/// A parse error with structured context.
#[derive(Clone, Debug, thiserror::Error)]
pub enum ParseError {
    #[error(transparent)]
    Scan(#[from] ScanError),

    #[error("unexpected {got:?}, expected {expected}")]
    UnexpectedToken {
        expected: &'static str,
        got: TokenKind,
        span: Span,
    },

    #[error("unexpected end of input, expected {expected}")]
    UnexpectedEof {
        expected: &'static str,
        span: Span,
    },

    #[error("duplicate %YAML directive")]
    DuplicateVersionDirective { span: Span },

    #[error("duplicate %TAG directive for handle {handle:?}")]
    DuplicateTagDirective { handle: String, span: Span },

    #[error("tag prefix {prefix:?} is not defined")]
    UndefinedTagPrefix { prefix: String, span: Span },
}
```

**Step 5: Write parser.rs stub with state enum**

Create `crates/yamalgam-parser/src/parser.rs`:

```rust
/// Parser states matching libfyaml's `fy_parser_state`.
// cref: fy-parse.h:86-135
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParserState {
    StreamStart,
    ImplicitDocumentStart,
    DocumentStart,
    DocumentContent,
    DocumentEnd,
    BlockNode,
    BlockSequenceFirstEntry,
    BlockSequenceEntry,
    IndentlessSequenceEntry,
    BlockMappingFirstKey,
    BlockMappingKey,
    BlockMappingValue,
    FlowSequenceFirstEntry,
    FlowSequenceEntry,
    FlowSequenceEntryMappingKey,
    FlowSequenceEntryMappingValue,
    FlowSequenceEntryMappingEnd,
    FlowMappingFirstKey,
    FlowMappingKey,
    FlowMappingValue,
    FlowMappingEmptyValue,
    End,
}
```

**Step 6: Update lib.rs with re-exports**

```rust
pub mod error;
pub mod event;
pub mod parser;

pub use error::ParseError;
pub use event::{CollectionStyle, Event};
pub use yamalgam_scanner::ScalarStyle;
```

**Step 7: Run tests to verify they pass**

Run: `cargo nextest run -p yamalgam-parser`
Expected: 3 tests PASS

**Step 8: Commit**

```
feat(parser): define Event, ParseError, and ParserState types
```

---

## Task 3: Parser struct + Iterator — StreamStart / StreamEnd

**Files:**
- Modify: `crates/yamalgam-parser/src/parser.rs`
- Modify: `crates/yamalgam-parser/src/lib.rs`
- Create: `crates/yamalgam-parser/tests/parser.rs`

**Step 1: Write the failing test**

Create `crates/yamalgam-parser/tests/parser.rs`:

```rust
use pretty_assertions::assert_eq;
use yamalgam_parser::{Event, Parser};

#[test]
fn empty_stream() {
    let events: Vec<_> = Parser::new("")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0], Event::StreamStart));
    assert!(matches!(events[1], Event::StreamEnd));
}

#[test]
fn whitespace_only_stream() {
    let events: Vec<_> = Parser::new("   \n\n  ")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0], Event::StreamStart));
    assert!(matches!(events[1], Event::StreamEnd));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo nextest run -p yamalgam-parser --test parser`
Expected: FAIL — `Parser` struct doesn't exist

**Step 3: Implement Parser struct with StreamStart/StreamEnd**

In `crates/yamalgam-parser/src/parser.rs`, add:

```rust
use std::borrow::Cow;

use yamalgam_core::Span;
use yamalgam_scanner::scanner::ScanError;
use yamalgam_scanner::{Scanner, Token, TokenKind};

use crate::error::ParseError;
use crate::event::Event;

// ... (ParserState enum from Task 2 stays) ...

/// A YAML pull parser.
///
/// Consumes scanner tokens and emits semantic events. This is a StAX-style
/// pull parser: the caller drives iteration via `Iterator::next()`.
///
/// The parser is a pure state machine — no anchor tables, no merge key
/// expansion, no tree construction.
// cref: fy-parse.c:6044-7060 (fy_parse_internal)
pub struct Parser<'input> {
    tokens: Box<dyn Iterator<Item = Result<Token<'input>, ScanError>> + 'input>,
    state: ParserState,
    state_stack: Vec<ParserState>,
    peeked: Option<Token<'input>>,
    done: bool,
}

impl<'input> Parser<'input> {
    /// Create a parser over a YAML input string.
    pub fn new(input: &'input str) -> Self {
        Self {
            tokens: Box::new(Scanner::new(input)),
            state: ParserState::StreamStart,
            state_stack: Vec::new(),
            peeked: None,
            done: false,
        }
    }

    /// Create a parser from an existing token iterator.
    pub fn from_tokens(
        tokens: impl Iterator<Item = Result<Token<'input>, ScanError>> + 'input,
    ) -> Self {
        Self {
            tokens: Box::new(tokens),
            state: ParserState::StreamStart,
            state_stack: Vec::new(),
            peeked: None,
            done: false,
        }
    }

    /// Peek at the next token without consuming it.
    fn peek_token(&mut self) -> Result<Option<&Token<'input>>, ParseError> {
        if self.peeked.is_none() {
            self.peeked = match self.tokens.next() {
                Some(Ok(t)) => Some(t),
                Some(Err(e)) => return Err(e.into()),
                None => None,
            };
        }
        Ok(self.peeked.as_ref())
    }

    /// Consume and return the next token.
    fn next_token(&mut self) -> Result<Option<Token<'input>>, ParseError> {
        if let Some(t) = self.peeked.take() {
            return Ok(Some(t));
        }
        match self.tokens.next() {
            Some(Ok(t)) => Ok(Some(t)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// Push current state for later restoration.
    // cref: fy-parse.c:5673-5686 (fy_parse_state_push)
    fn push_state(&mut self, state: ParserState) {
        self.state_stack.push(state);
    }

    /// Pop and restore previous state.
    // cref: fy-parse.c:5688-5702 (fy_parse_state_pop)
    fn pop_state(&mut self) -> ParserState {
        self.state_stack.pop().unwrap_or(ParserState::End)
    }

    /// Main parse dispatch — one call produces one event.
    // cref: fy-parse.c:6044-7060 (fy_parse_internal)
    fn parse_next(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        if self.done {
            return Ok(None);
        }

        match self.state {
            // cref: fy-parse.c:6127-6154
            ParserState::StreamStart => {
                // Consume StreamStart token from scanner
                let token = self.next_token()?;
                match token {
                    Some(t) if t.kind == TokenKind::StreamStart => {}
                    Some(t) => {
                        return Err(ParseError::UnexpectedToken {
                            expected: "stream start",
                            got: t.kind,
                            span: t.atom.span,
                        });
                    }
                    None => {
                        return Err(ParseError::UnexpectedEof {
                            expected: "stream start",
                            span: Span::default(),
                        });
                    }
                }
                self.state = ParserState::ImplicitDocumentStart;
                Ok(Some(Event::StreamStart))
            }

            ParserState::End => {
                self.done = true;
                Ok(None)
            }

            // Temporary catch-all — will be replaced as states are implemented
            _ => {
                // Check for stream end in unimplemented states
                let token = self.peek_token()?;
                match token.map(|t| t.kind) {
                    Some(TokenKind::StreamEnd) | None => {
                        self.next_token()?; // consume StreamEnd
                        self.state = ParserState::End;
                        Ok(Some(Event::StreamEnd))
                    }
                    Some(kind) => Err(ParseError::UnexpectedToken {
                        expected: "parser state not yet implemented",
                        got: kind,
                        span: token.unwrap().atom.span.clone(),
                    }),
                }
            }
        }
    }
}

impl<'input> Iterator for Parser<'input> {
    type Item = Result<Event<'input>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.parse_next() {
            Ok(Some(event)) => Some(Ok(event)),
            Ok(None) => None,
            Err(e) => {
                self.done = true;
                Some(Err(e))
            }
        }
    }
}
```

**Step 4: Update lib.rs to re-export Parser**

Add to `crates/yamalgam-parser/src/lib.rs`:

```rust
pub use parser::Parser;
```

**Step 5: Run tests**

Run: `cargo nextest run -p yamalgam-parser`
Expected: all tests PASS

**Step 6: Run `just clippy` to verify**

Run: `just clippy`
Expected: no warnings for yamalgam-parser

**Step 7: Commit**

```
feat(parser): implement Parser struct with StreamStart/StreamEnd
```

---

## Task 4: Document handling — directives, DocumentStart, DocumentEnd

This is the most complex state group. The parser must emit VersionDirective and TagDirective as separate events, then DocumentStart.

**Files:**
- Modify: `crates/yamalgam-parser/src/parser.rs`
- Modify: `crates/yamalgam-parser/tests/parser.rs`

**Step 1: Write failing tests**

Add to `crates/yamalgam-parser/tests/parser.rs`:

```rust
#[test]
fn implicit_document_with_scalar() {
    let events: Vec<_> = Parser::new("hello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // StreamStart, DocStart(implicit), Scalar, DocEnd(implicit), StreamEnd
    assert_eq!(events.len(), 5);
    assert!(matches!(events[0], Event::StreamStart));
    assert!(matches!(events[1], Event::DocumentStart { implicit: true, .. }));
    assert!(matches!(events[2], Event::Scalar { .. }));
    assert!(matches!(events[3], Event::DocumentEnd { implicit: true, .. }));
    assert!(matches!(events[4], Event::StreamEnd));
}

#[test]
fn explicit_document_start() {
    let events: Vec<_> = Parser::new("---\nhello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(events[1], Event::DocumentStart { implicit: false, .. }));
}

#[test]
fn explicit_document_end() {
    let events: Vec<_> = Parser::new("hello\n...")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(events[3], Event::DocumentEnd { implicit: false, .. }));
}

#[test]
fn version_directive_as_event() {
    let events: Vec<_> = Parser::new("%YAML 1.2\n---\nhello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // StreamStart, VersionDirective, DocStart(explicit), Scalar, DocEnd, StreamEnd
    assert!(matches!(
        events[1],
        Event::VersionDirective { major: 1, minor: 2, .. }
    ));
    assert!(matches!(events[2], Event::DocumentStart { implicit: false, .. }));
}

#[test]
fn tag_directive_as_event() {
    let events: Vec<_> = Parser::new("%TAG !e! tag:example.com,2000:\n---\nhello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // StreamStart, TagDirective, DocStart(explicit), Scalar, DocEnd, StreamEnd
    assert!(matches!(events[1], Event::TagDirective { .. }));
    if let Event::TagDirective { ref handle, ref prefix, .. } = events[1] {
        assert_eq!(handle.as_ref(), "!e!");
        assert_eq!(prefix.as_ref(), "tag:example.com,2000:");
    }
}

#[test]
fn multi_document() {
    let events: Vec<_> = Parser::new("hello\n---\nworld")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // StreamStart, DocStart(i), Scalar(hello), DocEnd(i),
    // DocStart(e), Scalar(world), DocEnd(i), StreamEnd
    let doc_starts: Vec<_> = events.iter()
        .filter(|e| matches!(e, Event::DocumentStart { .. }))
        .collect();
    assert_eq!(doc_starts.len(), 2);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo nextest run -p yamalgam-parser --test parser`
Expected: FAIL — document states not implemented

**Step 3: Implement document states**

Add these state handlers to the `parse_next` match in `parser.rs`. Reference `fy-parse.c:6156-6428` for the logic. Key points:

- `ImplicitDocumentStart`: peek token. If VersionDirective or TagDirective, emit as event (DO NOT consume into DocumentStart). If DocumentStart (`---`), transition to `DocumentStart` state. If StreamEnd, emit StreamEnd. Otherwise, emit implicit DocumentStart and push `DocumentEnd` state.

- `DocumentStart`: handle explicit `---`. Consume the token, emit `DocumentStart { implicit: false }`, push `DocumentEnd`, transition to `DocumentContent`.

- `DocumentContent`: peek token. If it's a content token, transition to `BlockNode` to parse it. If it's DocumentEnd/DocumentStart/StreamEnd, emit an empty scalar (implicit empty document) and pop state.

- `DocumentEnd`: peek token. If `...` (DocumentEnd token), consume and emit `DocumentEnd { implicit: false }`. If `---` or StreamEnd, emit `DocumentEnd { implicit: true }`. Transition to `ImplicitDocumentStart` (or `End` on StreamEnd).

The directive handling is the yamalgam-specific part: when the parser sees VersionDirective or TagDirective tokens, it emits them as events and loops back to check the next token (staying in `ImplicitDocumentStart` or `DocumentStart`). This means the state handler may need to loop internally for consecutive directives, or re-enter the same state after emitting a directive event.

**Step 4: Run tests**

Run: `cargo nextest run -p yamalgam-parser --test parser`
Expected: PASS

**Step 5: Commit**

```
feat(parser): implement document start/end with directive events
```

---

## Task 5: Node parsing — scalars with anchor/tag collection

The core of `fy_parse_node()` — collect optional anchor and tag tokens, then emit the appropriate node event.

**Files:**
- Modify: `crates/yamalgam-parser/src/parser.rs`
- Modify: `crates/yamalgam-parser/tests/parser.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn plain_scalar() {
    let events: Vec<_> = Parser::new("hello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    if let Event::Scalar { ref value, style, ref anchor, ref tag, .. } = events[2] {
        assert_eq!(value.as_ref(), "hello");
        assert_eq!(style, ScalarStyle::Plain);
        assert!(anchor.is_none());
        assert!(tag.is_none());
    } else {
        panic!("expected Scalar, got {:?}", events[2]);
    }
}

#[test]
fn anchored_scalar() {
    let events: Vec<_> = Parser::new("&foo hello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    if let Event::Scalar { ref anchor, ref value, .. } = events[2] {
        assert_eq!(anchor.as_deref(), Some("foo"));
        assert_eq!(value.as_ref(), "hello");
    } else {
        panic!("expected Scalar, got {:?}", events[2]);
    }
}

#[test]
fn tagged_scalar() {
    let events: Vec<_> = Parser::new("!!str hello")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    if let Event::Scalar { ref tag, ref value, .. } = events[2] {
        assert_eq!(tag.as_deref(), Some("!!str"));
        assert_eq!(value.as_ref(), "hello");
    } else {
        panic!("expected Scalar, got {:?}", events[2]);
    }
}

#[test]
fn quoted_scalar_styles() {
    let single = Parser::new("'hello'").collect::<Result<Vec<_>, _>>().unwrap();
    let double = Parser::new("\"hello\"").collect::<Result<Vec<_>, _>>().unwrap();

    assert!(matches!(single[2], Event::Scalar { style: ScalarStyle::SingleQuoted, .. }));
    assert!(matches!(double[2], Event::Scalar { style: ScalarStyle::DoubleQuoted, .. }));
}

#[test]
fn block_scalar() {
    let events: Vec<_> = Parser::new("|\n  hello\n  world")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    if let Event::Scalar { ref value, style, .. } = events[2] {
        assert_eq!(style, ScalarStyle::Literal);
        assert!(value.contains("hello"));
    } else {
        panic!("expected Scalar, got {:?}", events[2]);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo nextest run -p yamalgam-parser --test parser`
Expected: FAIL

**Step 3: Implement parse_node and BlockNode state**

Add a `parse_node()` method to Parser that:
1. Peeks token
2. If Anchor: consume, store anchor value
3. If Tag: consume, store tag value
4. Match on next token kind:
   - `Scalar` → consume, emit `Event::Scalar { anchor, tag, value, style, span }`
   - `FlowSequenceStart` / `BlockSequenceStart` → will be handled in later tasks
   - `FlowMappingStart` / `BlockMappingStart` → will be handled in later tasks
   - Other → error

Wire `BlockNode` state to call `parse_node()` and pop state afterward.

Reference: `fy-parse.c:5715-5983` (fy_parse_node)

**Step 4: Run tests**

Run: `cargo nextest run -p yamalgam-parser --test parser`
Expected: PASS

**Step 5: Commit**

```
feat(parser): implement scalar node parsing with anchor/tag collection
```

---

## Task 6: Node parsing — aliases

**Files:**
- Modify: `crates/yamalgam-parser/src/parser.rs`
- Modify: `crates/yamalgam-parser/tests/parser.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn alias_event() {
    let events: Vec<_> = Parser::new("- &anchor hello\n- *anchor")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let alias = events.iter().find(|e| matches!(e, Event::Alias { .. }));
    assert!(alias.is_some());
    if let Some(Event::Alias { ref name, .. }) = alias {
        assert_eq!(name.as_ref(), "anchor");
    }
}
```

**Step 2: Run test to verify it fails**

**Step 3: Implement alias handling in parse_node**

In `parse_node()`, before anchor/tag collection, check if the token is an Alias:
- If `Alias` → consume, emit `Event::Alias { name, span }`, pop state
- Aliases never have anchors or tags attached

Reference: `fy-parse.c:5738-5750`

**Step 4: Run tests, verify PASS**

**Step 5: Commit**

```
feat(parser): implement alias event parsing
```

---

## Task 7: Block sequences

**Files:**
- Modify: `crates/yamalgam-parser/src/parser.rs`
- Modify: `crates/yamalgam-parser/tests/parser.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn block_sequence() {
    let events: Vec<_> = Parser::new("- a\n- b\n- c")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // StreamStart, DocStart, SeqStart, Scalar(a), Scalar(b), Scalar(c), SeqEnd, DocEnd, StreamEnd
    assert!(matches!(events[2], Event::SequenceStart { style: CollectionStyle::Block, .. }));
    assert!(matches!(events[3], Event::Scalar { .. }));
    assert!(matches!(events[4], Event::Scalar { .. }));
    assert!(matches!(events[5], Event::Scalar { .. }));
    assert!(matches!(events[6], Event::SequenceEnd { .. }));
}

#[test]
fn nested_block_sequence() {
    let events: Vec<_> = Parser::new("- - a\n  - b\n- c")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let seq_starts: Vec<_> = events.iter()
        .filter(|e| matches!(e, Event::SequenceStart { .. }))
        .collect();
    assert_eq!(seq_starts.len(), 2);
}

#[test]
fn anchored_sequence() {
    let events: Vec<_> = Parser::new("&seq\n- a\n- b")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    if let Event::SequenceStart { ref anchor, .. } = events[2] {
        assert_eq!(anchor.as_deref(), Some("seq"));
    } else {
        panic!("expected SequenceStart");
    }
}
```

**Step 2: Run tests, verify FAIL**

**Step 3: Implement block sequence states**

Add handlers for:
- `BlockSequenceFirstEntry` — emit `SequenceStart`, expect `BlockEntry` token, push state, transition to `BlockNode`
- `BlockSequenceEntry` — if `BlockEntry`, push state, transition to `BlockNode`. If `BlockEnd`, emit `SequenceEnd`, pop state.

Wire `parse_node()` to handle `BlockSequenceStart` token: emit `SequenceStart`, push return state, transition to `BlockSequenceFirstEntry`.

Reference:
- `fy-parse.c:5866-5902` (block sequence in parse_node)
- `fy-parse.c:6465-6550` (block sequence states)

**Step 4: Run tests, verify PASS**

**Step 5: Commit**

```
feat(parser): implement block sequence parsing
```

---

## Task 8: Block mappings

**Files:**
- Modify: `crates/yamalgam-parser/src/parser.rs`
- Modify: `crates/yamalgam-parser/tests/parser.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn block_mapping() {
    let events: Vec<_> = Parser::new("a: b\nc: d")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // StreamStart, DocStart, MapStart, Scalar(a), Scalar(b), Scalar(c), Scalar(d), MapEnd, DocEnd, StreamEnd
    assert!(matches!(events[2], Event::MappingStart { style: CollectionStyle::Block, .. }));
    assert_eq!(events.iter().filter(|e| matches!(e, Event::Scalar { .. })).count(), 4);
    assert!(matches!(events[7], Event::MappingEnd { .. }));
}

#[test]
fn mapping_with_explicit_key() {
    let events: Vec<_> = Parser::new("? a\n: b")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(events[2], Event::MappingStart { .. }));
}

#[test]
fn empty_value_in_mapping() {
    let events: Vec<_> = Parser::new("a:\nb: c")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // a's value should be an empty scalar
    // MapStart, Scalar(a), Scalar(""), Scalar(b), Scalar(c), MapEnd
    let scalars: Vec<_> = events.iter()
        .filter_map(|e| if let Event::Scalar { ref value, .. } = e { Some(value.as_ref()) } else { None })
        .collect();
    assert_eq!(scalars, vec!["a", "", "b", "c"]);
}

#[test]
fn nested_mapping_in_sequence() {
    let events: Vec<_> = Parser::new("- a: b\n  c: d\n- e")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(events.iter().any(|e| matches!(e, Event::MappingStart { .. })));
}
```

**Step 2: Run tests, verify FAIL**

**Step 3: Implement block mapping states**

Add handlers for:
- `BlockMappingFirstKey` — expect `Key` token, push state, transition to `BlockNode`
- `BlockMappingKey` — if `Key`, push state, parse key. If `BlockEnd`, emit `MappingEnd`, pop state. If `Value` without preceding key, emit empty scalar for key.
- `BlockMappingValue` — if `Value`, push state, parse value. Otherwise emit empty scalar for value and transition to `BlockMappingKey`.

Wire `parse_node()` to handle `BlockMappingStart` token: emit `MappingStart`, push return state, transition to `BlockMappingFirstKey`.

Also implement `emit_empty_scalar()` helper that generates `Event::Scalar` with empty `Cow::Borrowed("")` and `ScalarStyle::Plain`.

Reference:
- `fy-parse.c:5904-5946` (block mapping in parse_node)
- `fy-parse.c:6551-6664` (block mapping states)
- `fy-parse.c:5987-6012` (fy_parse_empty_scalar)

**Step 4: Run tests, verify PASS**

**Step 5: Commit**

```
feat(parser): implement block mapping parsing
```

---

## Task 9: Flow sequences (including implicit flow mapping entries)

**Files:**
- Modify: `crates/yamalgam-parser/src/parser.rs`
- Modify: `crates/yamalgam-parser/tests/parser.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn flow_sequence() {
    let events: Vec<_> = Parser::new("[a, b, c]")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(events[2], Event::SequenceStart { style: CollectionStyle::Flow, .. }));
    let scalars: Vec<_> = events.iter()
        .filter_map(|e| if let Event::Scalar { ref value, .. } = e { Some(value.as_ref()) } else { None })
        .collect();
    assert_eq!(scalars, vec!["a", "b", "c"]);
    assert!(matches!(events[6], Event::SequenceEnd { .. }));
}

#[test]
fn empty_flow_sequence() {
    let events: Vec<_> = Parser::new("[]")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(events[2], Event::SequenceStart { style: CollectionStyle::Flow, .. }));
    assert!(matches!(events[3], Event::SequenceEnd { .. }));
}

#[test]
fn nested_flow_sequences() {
    let events: Vec<_> = Parser::new("[[a, b], [c]]")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let seq_starts = events.iter().filter(|e| matches!(e, Event::SequenceStart { .. })).count();
    assert_eq!(seq_starts, 3); // outer + 2 inner
}

#[test]
fn flow_sequence_with_implicit_mapping() {
    // [a: b] creates an implicit mapping entry inside the sequence
    let events: Vec<_> = Parser::new("[a: b]")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(events.iter().any(|e| matches!(e, Event::MappingStart { .. })));
}
```

**Step 2: Run tests, verify FAIL**

**Step 3: Implement flow sequence states**

Add handlers for:
- `FlowSequenceFirstEntry` — if `FlowSequenceEnd`, emit `SequenceEnd`, pop state. Otherwise parse entry.
- `FlowSequenceEntry` — handle `FlowEntry` (`,`), `FlowSequenceEnd` (`]`). Parse entries.
- `FlowSequenceEntryMappingKey` — implicit mapping key inside flow sequence
- `FlowSequenceEntryMappingValue` — implicit mapping value
- `FlowSequenceEntryMappingEnd` — emit `MappingEnd` for implicit mapping

Wire `parse_node()` to handle `FlowSequenceStart` token.

Reference:
- `fy-parse.c:5832-5847` (flow sequence in parse_node)
- `fy-parse.c:6666-6826` (flow sequence states)

**Step 4: Run tests, verify PASS**

**Step 5: Commit**

```
feat(parser): implement flow sequence parsing with implicit mappings
```

---

## Task 10: Flow mappings

**Files:**
- Modify: `crates/yamalgam-parser/src/parser.rs`
- Modify: `crates/yamalgam-parser/tests/parser.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn flow_mapping() {
    let events: Vec<_> = Parser::new("{a: b, c: d}")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(events[2], Event::MappingStart { style: CollectionStyle::Flow, .. }));
    let scalars: Vec<_> = events.iter()
        .filter_map(|e| if let Event::Scalar { ref value, .. } = e { Some(value.as_ref()) } else { None })
        .collect();
    assert_eq!(scalars, vec!["a", "b", "c", "d"]);
}

#[test]
fn empty_flow_mapping() {
    let events: Vec<_> = Parser::new("{}")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(matches!(events[2], Event::MappingStart { style: CollectionStyle::Flow, .. }));
    assert!(matches!(events[3], Event::MappingEnd { .. }));
}

#[test]
fn flow_mapping_empty_value() {
    let events: Vec<_> = Parser::new("{a:, b: c}")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let scalars: Vec<_> = events.iter()
        .filter_map(|e| if let Event::Scalar { ref value, .. } = e { Some(value.as_ref()) } else { None })
        .collect();
    assert_eq!(scalars, vec!["a", "", "b", "c"]);
}

#[test]
fn nested_flow_in_block() {
    let events: Vec<_> = Parser::new("key: {a: b}\nother: [1, 2]")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(events.iter().any(|e| matches!(e, Event::MappingStart { style: CollectionStyle::Flow, .. })));
    assert!(events.iter().any(|e| matches!(e, Event::SequenceStart { style: CollectionStyle::Flow, .. })));
}
```

**Step 2: Run tests, verify FAIL**

**Step 3: Implement flow mapping states**

Add handlers for:
- `FlowMappingFirstKey` — if `FlowMappingEnd`, emit `MappingEnd`, pop state. Otherwise parse key.
- `FlowMappingKey` — handle `FlowEntry` (`,`), `FlowMappingEnd` (`}`). Parse keys.
- `FlowMappingValue` — if `Value`, parse value node. Otherwise emit empty scalar.
- `FlowMappingEmptyValue` — emit empty scalar, transition to `FlowMappingKey`.

Wire `parse_node()` to handle `FlowMappingStart` token.

Reference:
- `fy-parse.c:5849-5864` (flow mapping in parse_node)
- `fy-parse.c:6827-6964` (flow mapping states)

**Step 4: Run tests, verify PASS**

**Step 5: Commit**

```
feat(parser): implement flow mapping parsing
```

---

## Task 11: Indentless sequences

**Files:**
- Modify: `crates/yamalgam-parser/src/parser.rs`
- Modify: `crates/yamalgam-parser/tests/parser.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn indentless_sequence_in_mapping() {
    let events: Vec<_> = Parser::new("key:\n- a\n- b")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // DocStart, MapStart, Scalar(key), SeqStart, Scalar(a), Scalar(b), SeqEnd, MapEnd, DocEnd
    assert!(events.iter().any(|e| matches!(e, Event::SequenceStart { .. })));
    let scalars: Vec<_> = events.iter()
        .filter_map(|e| if let Event::Scalar { ref value, .. } = e { Some(value.as_ref()) } else { None })
        .collect();
    assert_eq!(scalars, vec!["key", "a", "b"]);
}

#[test]
fn indentless_sequence_with_nested_mapping() {
    let events: Vec<_> = Parser::new("key:\n- a: b\n- c: d")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let map_starts = events.iter().filter(|e| matches!(e, Event::MappingStart { .. })).count();
    assert_eq!(map_starts, 3); // outer + 2 entries
}
```

**Step 2: Run tests, verify FAIL**

**Step 3: Implement IndentlessSequenceEntry state**

When `BlockMappingValue` encounters `BlockEntry` instead of a regular node, it triggers an indentless sequence — a sequence that lives at the mapping value's indent level.

In `parse_node()`, when in `BlockMappingValue` or `BlockMappingFirstKey` context and token is `BlockEntry`:
- Emit `SequenceStart { style: Block }`
- Transition to `IndentlessSequenceEntry`

`IndentlessSequenceEntry` handler:
- If `BlockEntry` → push state, transition to `BlockNode`
- If not `BlockEntry` → emit `SequenceEnd`, pop state

Reference: `fy-parse.c:5781-5812` (indentless in parse_node), `fy-parse.c:6465-6478` (FYPS_INDENTLESS_SEQUENCE_ENTRY)

**Step 4: Run tests, verify PASS**

**Step 5: Commit**

```
feat(parser): implement indentless sequence parsing
```

---

## Task 12: Extend C harness with --events flag

**Files:**
- Modify: `tools/fyaml-tokenize/main.c`

**Step 1: Add --events flag parsing**

At the top of `main()`, check `argv` for `--events`:

```c
int events_mode = 0;
for (int i = 1; i < argc; i++) {
    if (strcmp(argv[i], "--events") == 0) {
        events_mode = 1;
    }
}
```

**Step 2: Add event output function**

After the existing token scan loop, add an alternative path for events mode using `fy_parser_parse()`:

```c
if (events_mode) {
    struct fy_event *fye;
    while ((fye = fy_parser_parse(fyp)) != NULL) {
        // Output JSON line per event: type, anchor, tag, value, implicit
        // Map fye->type to string name
        // Extract anchor/tag/value from event-specific data
        fprintf(stdout, "{\"type\":\"%s\"", event_type_str(fye->type));
        // ... add fields based on event type
        fprintf(stdout, "}\n");
        fy_parser_event_free(fyp, fye);
    }
}
```

Map event types to strings:
- `FYET_STREAM_START` → `"StreamStart"`
- `FYET_STREAM_END` → `"StreamEnd"`
- `FYET_DOCUMENT_START` → `"DocumentStart"` (include `implicit` field)
- `FYET_DOCUMENT_END` → `"DocumentEnd"` (include `implicit` field)
- `FYET_MAPPING_START` → `"MappingStart"` (include anchor, tag)
- `FYET_MAPPING_END` → `"MappingEnd"`
- `FYET_SEQUENCE_START` → `"SequenceStart"` (include anchor, tag)
- `FYET_SEQUENCE_END` → `"SequenceEnd"`
- `FYET_SCALAR` → `"Scalar"` (include anchor, tag, value)
- `FYET_ALIAS` → `"Alias"` (include name)

**Step 3: Rebuild the C harness**

```bash
cd tools/fyaml-tokenize && make clean && make && cd ../..
```

**Step 4: Verify with a simple test**

```bash
echo "a: b" | ./tools/fyaml-tokenize/fyaml-tokenize --events
```

Expected output (JSON lines):
```json
{"type":"StreamStart"}
{"type":"DocumentStart","implicit":true}
{"type":"MappingStart","anchor":null,"tag":null}
{"type":"Scalar","anchor":null,"tag":null,"value":"a"}
{"type":"Scalar","anchor":null,"tag":null,"value":"b"}
{"type":"MappingEnd"}
{"type":"DocumentEnd","implicit":true}
{"type":"StreamEnd"}
```

**Step 5: Commit**

```
feat(harness): add --events mode to fyaml-tokenize C harness
```

---

## Task 13: Event comparison in yamalgam-compare

**Files:**
- Create: `crates/yamalgam-compare/src/event_snapshot.rs`
- Modify: `crates/yamalgam-compare/src/harness.rs`
- Modify: `crates/yamalgam-compare/src/compare.rs`
- Modify: `crates/yamalgam-compare/src/lib.rs`
- Modify: `crates/yamalgam-compare/Cargo.toml` (add `yamalgam-parser` dependency)

**Step 1: Add yamalgam-parser dependency**

In `crates/yamalgam-compare/Cargo.toml`, add:

```toml
yamalgam-parser = { path = "../yamalgam-parser" }
```

**Step 2: Create EventSnapshot type**

Create `crates/yamalgam-compare/src/event_snapshot.rs`:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct EventSnapshot {
    pub kind: String,
    pub anchor: Option<String>,
    pub tag: Option<String>,
    pub value: Option<String>,
    pub implicit: Option<bool>,
}
```

**Step 3: Add event harness functions**

In `harness.rs`, add:
- `run_c_events(input: &[u8]) -> Result<Vec<EventSnapshot>, String>` — spawn C harness with `--events`, parse JSON output
- `run_rust_parser(input: &[u8]) -> Result<Vec<EventSnapshot>, String>` — run Rust parser, convert Events to EventSnapshots
- `compare_events(input: &[u8]) -> CompareEventResult` — orchestrate both

**Step 4: Add event comparison**

In `compare.rs`, add `CompareEventResult` enum and `compare_event_streams()` function, following the same pattern as the existing token comparison.

**Step 5: Wire into lib.rs**

Export the new module and types.

**Step 6: Write unit tests**

Add tests to `crates/yamalgam-compare/tests/compare_tests.rs` that verify event comparison works for simple cases.

**Step 7: Commit**

```
feat(compare): add parser event comparison harness
```

---

## Task 14: Event compliance testing

**Files:**
- Modify: `crates/yamalgam-compare/tests/compliance.rs`

**Step 1: Add event compliance test**

Add a second test function (or extend the existing one) that runs event-level comparison for all 351 YAML Test Suite cases. Use the same categorization scheme (PASS/UNEXPECTED/EXPECTED/MISMATCH).

Note: the event-level results may differ from token-level results. Some tests that PASS at the token level may MISMATCH at the event level (different event ordering). Track these separately.

**Step 2: Run compliance**

```bash
cargo nextest run -p yamalgam-compare --test compliance --success-output=immediate 2>&1 | grep -oE "^    (PASS|UNEXPECTED|MISMATCH|EXPECTED)" | sort | uniq -c | sort -rn
```

**Step 3: Fix mismatches iteratively**

This is the "grind" phase — same pattern as the scanner compliance push. Fix one category at a time:
1. UNEXPECTED first (our parser is too permissive)
2. MISMATCH next (both succeed but different events)
3. EXPECTED last (evaluate whether our strictness is correct)

Note: yamalgam emits `VersionDirective` and `TagDirective` as separate events, which libfyaml does NOT. The comparison harness must filter these yamalgam-specific events before comparing against libfyaml output. This is expected and by design.

**Step 4: Commit after each batch of fixes**

```
fix(parser): resolve N compliance mismatches — description
```

---

## Task 15: Final validation and cleanup

**Files:**
- Various parser files for cleanup
- Modify: `crates/yamalgam-parser/src/lib.rs` (finalize exports)

**Step 1: Run full check**

```bash
just check
```

Expected: all green — fmt, clippy, deny, nextest, doc-test.

**Step 2: Run scanner tests to verify no regressions**

```bash
cargo nextest run -p yamalgam-scanner
```

Expected: 126+ tests PASS (unchanged from before parser work)

**Step 3: Run event compliance and record final numbers**

```bash
cargo nextest run -p yamalgam-compare --test compliance --success-output=immediate
```

Record PASS/UNEXPECTED/MISMATCH/EXPECTED counts for events.

**Step 4: Commit**

```
docs(parser): record parser compliance results
```

---

## Commit and PR sequence

After all tasks complete:

1. Create PR with title: `feat(parser): YAML pull parser — tokens to events`
2. PR body should include:
   - Event count and compliance numbers
   - Link to design doc and ADR-0005
   - Note about directive events being yamalgam-specific (not in libfyaml)

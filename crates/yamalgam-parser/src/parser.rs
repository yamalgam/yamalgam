//! YAML pull parser — consumes tokens, emits events.

use std::borrow::Cow;

use yamalgam_core::Span;
use yamalgam_scanner::scanner::{ScanError, Scanner};
use yamalgam_scanner::{ScalarStyle, Token, TokenKind};

use crate::error::ParseError;
use crate::event::{CollectionStyle, Event};

/// Parser states matching libfyaml's `fy_parser_state`.
// cref: fy-parse.h:86-135
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParserState {
    /// Expect `StreamStart` token.
    StreamStart,
    /// Expect the beginning of an implicit document.
    ImplicitDocumentStart,
    /// Expect an explicit `DocumentStart` or `StreamEnd`.
    DocumentStart,
    /// Expect the content of a document (a single node).
    DocumentContent,
    /// Expect `DocumentEnd` or a new document.
    DocumentEnd,
    /// Expect a block-level node.
    BlockNode,
    /// Expect the first entry of a block sequence.
    BlockSequenceFirstEntry,
    /// Expect an entry of a block sequence.
    BlockSequenceEntry,
    /// Expect an entry of an indentless sequence.
    IndentlessSequenceEntry,
    /// Expect the first key of a block mapping.
    BlockMappingFirstKey,
    /// Expect a block mapping key.
    BlockMappingKey,
    /// Expect a block mapping value.
    BlockMappingValue,
    /// Expect the first entry of a flow sequence.
    FlowSequenceFirstEntry,
    /// Expect an entry of a flow sequence.
    FlowSequenceEntry,
    /// Expect a key of an ordered mapping inside a flow sequence.
    #[allow(dead_code)] // Reserved for future complex implicit mapping flows.
    FlowSequenceEntryMappingKey,
    /// Expect a value of an ordered mapping inside a flow sequence.
    FlowSequenceEntryMappingValue,
    /// Expect the end of an ordered mapping inside a flow sequence.
    FlowSequenceEntryMappingEnd,
    /// Expect the first key of a flow mapping.
    FlowMappingFirstKey,
    /// Expect a key of a flow mapping.
    FlowMappingKey,
    /// Expect a value of a flow mapping.
    FlowMappingValue,
    /// Expect an empty value of a flow mapping.
    #[allow(dead_code)] // Reserved for future explicit empty value flows.
    FlowMappingEmptyValue,
    /// Terminal state — stream has ended.
    End,
}

/// A pull parser that converts a token stream into YAML events.
///
/// Implements `Iterator<Item = Result<Event, ParseError>>` for
/// idiomatic consumption. Wraps a [`Scanner`] by default, but can
/// accept any token iterator via [`Parser::from_tokens`].
pub struct Parser<'input> {
    /// Token source.
    tokens: Box<dyn Iterator<Item = Result<Token<'input>, ScanError>> + 'input>,
    /// Current parser state.
    state: ParserState,
    /// Stack of saved states for nested structures.
    state_stack: Vec<ParserState>,
    /// One-token lookahead buffer.
    peeked: Option<Token<'input>>,
    /// Set to `true` once `StreamEnd` has been emitted or an error occurred.
    done: bool,
    /// True when a directive has been seen in the current prologue.
    /// Reset when a document starts. Used to require `---` after directives.
    seen_directive: bool,
}

impl<'input> Parser<'input> {
    /// Create a parser from a YAML input string.
    ///
    /// This constructs a [`Scanner`] internally.
    #[must_use]
    pub fn new(input: &'input str) -> Self {
        Self::from_tokens(Scanner::new(input))
    }

    /// Create a parser from an arbitrary token iterator.
    ///
    /// Useful for testing or for feeding tokens from a non-standard source.
    pub fn from_tokens(
        tokens: impl Iterator<Item = Result<Token<'input>, ScanError>> + 'input,
    ) -> Self {
        Self {
            tokens: Box::new(tokens),
            state: ParserState::StreamStart,
            state_stack: Vec::new(),
            peeked: None,
            done: false,
            seen_directive: false,
        }
    }

    /// Peek at the next token without consuming it.
    fn peek_token(&mut self) -> Result<Option<&Token<'input>>, ParseError> {
        if self.peeked.is_none() {
            self.peeked = self.tokens.next().transpose()?;
        }
        Ok(self.peeked.as_ref())
    }

    /// Consume and return the next token.
    fn next_token(&mut self) -> Result<Option<Token<'input>>, ParseError> {
        if let Some(token) = self.peeked.take() {
            return Ok(Some(token));
        }
        Ok(self.tokens.next().transpose()?)
    }

    /// Push a state onto the state stack.
    // cref: fy-parse.c:5673-5686 (fy_parse_state_push)
    fn push_state(&mut self, state: ParserState) {
        self.state_stack.push(state);
    }

    /// Pop a state from the state stack, defaulting to `End`.
    // cref: fy-parse.c:5688-5702 (fy_parse_state_pop)
    fn pop_state(&mut self) -> ParserState {
        self.state_stack.pop().unwrap_or(ParserState::End)
    }

    /// Create an empty plain scalar event at the given span.
    const fn emit_empty_scalar(span: Span) -> Event<'input> {
        Event::Scalar {
            anchor: None,
            tag: None,
            value: Cow::Borrowed(""),
            style: ScalarStyle::Plain,
            span,
        }
    }

    /// Core dispatch: produce the next event based on the current state.
    // cref: fy-parse.c:6044-7060 (fy_parse_internal)
    fn parse_next(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        if self.done {
            return Ok(None);
        }

        match self.state {
            ParserState::StreamStart => self.parse_stream_start(),
            ParserState::ImplicitDocumentStart => self.parse_implicit_document_start(),
            ParserState::DocumentStart => self.parse_document_start(),
            ParserState::DocumentContent => self.parse_document_content(),
            ParserState::DocumentEnd => self.parse_document_end(),
            ParserState::BlockNode => self.parse_node(),
            ParserState::BlockSequenceFirstEntry => self.parse_block_sequence_first_entry(),
            ParserState::BlockSequenceEntry => self.parse_block_sequence_entry(),
            ParserState::IndentlessSequenceEntry => self.parse_indentless_sequence_entry(),
            ParserState::BlockMappingFirstKey => self.parse_block_mapping_first_key(),
            ParserState::BlockMappingKey => self.parse_block_mapping_key(),
            ParserState::BlockMappingValue => self.parse_block_mapping_value(),
            ParserState::FlowSequenceFirstEntry => self.parse_flow_sequence_first_entry(),
            ParserState::FlowSequenceEntry => self.parse_flow_sequence_entry(),
            ParserState::FlowSequenceEntryMappingKey => {
                self.parse_flow_sequence_entry_mapping_key()
            }
            ParserState::FlowSequenceEntryMappingValue => {
                self.parse_flow_sequence_entry_mapping_value()
            }
            ParserState::FlowSequenceEntryMappingEnd => {
                self.parse_flow_sequence_entry_mapping_end()
            }
            ParserState::FlowMappingFirstKey => self.parse_flow_mapping_first_key(),
            ParserState::FlowMappingKey => self.parse_flow_mapping_key(),
            ParserState::FlowMappingValue => self.parse_flow_mapping_value(),
            ParserState::FlowMappingEmptyValue => self.parse_flow_mapping_empty_value(),
            ParserState::End => {
                self.done = true;
                Ok(None)
            }
        }
    }

    /// Handle `StreamStart` state: consume the `StreamStart` token and
    /// transition to `ImplicitDocumentStart`.
    fn parse_stream_start(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let token = self.next_token()?;
        match token {
            Some(t) if t.kind == TokenKind::StreamStart => {
                self.state = ParserState::ImplicitDocumentStart;
                Ok(Some(Event::StreamStart))
            }
            Some(t) => Err(ParseError::UnexpectedToken {
                expected: "StreamStart",
                got: t.kind,
                span: t.atom.span,
            }),
            None => Err(ParseError::UnexpectedEof {
                expected: "StreamStart",
                span: Span::default(),
            }),
        }
    }

    /// Handle `ImplicitDocumentStart` state: process directives or begin a
    /// document (implicitly or explicitly).
    // cref: fy-parse.c:6156-6340
    fn parse_implicit_document_start(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let kind = self.peek_token()?.map(|t| t.kind);
        match kind {
            Some(TokenKind::VersionDirective) => {
                // Consume the directive token, emit as event, stay in same state.
                self.seen_directive = true;
                let t = self.next_token()?.expect("peeked");
                let (major, minor) = Self::parse_version_string(&t.atom.data, t.atom.span)?;
                Ok(Some(Event::VersionDirective {
                    major,
                    minor,
                    span: t.atom.span,
                }))
            }
            Some(TokenKind::TagDirective) => {
                // Consume the directive token, emit as event, stay in same state.
                self.seen_directive = true;
                let t = self.next_token()?.expect("peeked");
                let (handle, prefix) = Self::parse_tag_directive_data(&t.atom.data);
                Ok(Some(Event::TagDirective {
                    handle: Cow::Owned(handle.to_string()),
                    prefix: Cow::Owned(prefix.to_string()),
                    span: t.atom.span,
                }))
            }
            Some(TokenKind::DocumentStart) => {
                // Explicit `---` — transition to DocumentStart state.
                self.seen_directive = false;
                self.state = ParserState::DocumentStart;
                self.parse_document_start()
            }
            Some(TokenKind::DocumentEnd) if !self.seen_directive => {
                // Bare `...` between documents (e.g., two `...` in a row, or
                // `...` after a comment). Not a document — just consume the
                // marker and stay in ImplicitDocumentStart.
                // cref: fy-parse.c:6186-6201
                //
                // When directives have been seen, `...` without `---` is
                // invalid — directives require an explicit document start.
                // That case falls through to the catch-all and correctly errors.
                let _t = self.next_token()?;
                self.parse_next()
            }
            Some(TokenKind::StreamEnd) => {
                // Empty stream (no documents). Consume and emit StreamEnd.
                let _t = self.next_token()?;
                self.state = ParserState::End;
                Ok(Some(Event::StreamEnd))
            }
            Some(_kind) => {
                // Content token — this is an implicit document start.
                // Don't consume the token; let BlockNode handle it.
                self.seen_directive = false;
                let span = self.peek_token()?.expect("peeked").atom.span;
                self.push_state(ParserState::DocumentEnd);
                self.state = ParserState::BlockNode;
                Ok(Some(Event::DocumentStart {
                    implicit: true,
                    span,
                }))
            }
            None => Err(ParseError::UnexpectedEof {
                expected: "document start or stream end",
                span: Span::default(),
            }),
        }
    }

    /// Handle `DocumentStart` state: consume the `---` token and emit
    /// an explicit `DocumentStart` event.
    // cref: fy-parse.c:6262-6340
    fn parse_document_start(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let token = self.next_token()?;
        match token {
            Some(t) if t.kind == TokenKind::DocumentStart => {
                self.push_state(ParserState::DocumentEnd);
                self.state = ParserState::DocumentContent;
                Ok(Some(Event::DocumentStart {
                    implicit: false,
                    span: t.atom.span,
                }))
            }
            Some(t) => Err(ParseError::UnexpectedToken {
                expected: "DocumentStart (---)",
                got: t.kind,
                span: t.atom.span,
            }),
            None => Err(ParseError::UnexpectedEof {
                expected: "DocumentStart (---)",
                span: Span::default(),
            }),
        }
    }

    /// Handle `DocumentContent` state: check if the document has content
    /// or is empty.
    // cref: fy-parse.c:6429-6455
    fn parse_document_content(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let token = self.peek_token()?;
        match token {
            Some(t)
                if t.kind == TokenKind::DocumentEnd
                    || t.kind == TokenKind::DocumentStart
                    || t.kind == TokenKind::StreamEnd =>
            {
                // Empty document — emit empty scalar, pop to DocumentEnd.
                let span = t.atom.span;
                self.state = self.pop_state();
                Ok(Some(Self::emit_empty_scalar(span)))
            }
            Some(_) => {
                // Content-bearing token — transition to BlockNode.
                self.state = ParserState::BlockNode;
                self.parse_next()
            }
            None => Err(ParseError::UnexpectedEof {
                expected: "document content or document end",
                span: Span::default(),
            }),
        }
    }

    /// Handle `DocumentEnd` state: emit an implicit or explicit document
    /// end event, then transition back to `ImplicitDocumentStart`.
    // cref: fy-parse.c:6342-6428
    fn parse_document_end(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let token = self.peek_token()?;
        match token {
            Some(t) if t.kind == TokenKind::DocumentEnd => {
                // Explicit `...` — consume and emit.
                let t = self.next_token()?.expect("peeked");
                self.state = ParserState::ImplicitDocumentStart;
                Ok(Some(Event::DocumentEnd {
                    implicit: false,
                    span: t.atom.span,
                }))
            }
            Some(t) => {
                // Implicit document end — don't consume the token.
                let span = t.atom.span;
                self.state = ParserState::ImplicitDocumentStart;
                Ok(Some(Event::DocumentEnd {
                    implicit: true,
                    span,
                }))
            }
            None => Err(ParseError::UnexpectedEof {
                expected: "document end or next document",
                span: Span::default(),
            }),
        }
    }

    /// Handle `BlockNode` state: parse a complete YAML node.
    ///
    /// A node may be:
    /// - An alias (`*name`) — never has anchor/tag
    /// - A scalar (plain, quoted, block) with optional anchor and/or tag
    /// - A block/flow collection start with optional anchor and/or tag
    /// - An empty scalar (when anchor/tag present but no content follows)
    // cref: fy-parse.c:5715-5983 (fy_parse_node)
    fn parse_node(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        // 1. Check for alias first — aliases never carry anchor/tag.
        let kind = self.peek_token()?.map(|t| t.kind);
        if kind == Some(TokenKind::Alias) {
            let t = self.next_token()?.expect("peeked");
            self.state = self.pop_state();
            return Ok(Some(Event::Alias {
                name: t.atom.data,
                span: t.atom.span,
            }));
        }

        // 2. Collect optional anchor and/or tag (either order).
        // cref: fy-parse.c:5773-5849
        let mut anchor: Option<Cow<'input, str>> = None;
        let mut tag: Option<Cow<'input, str>> = None;
        let mut anchor_span: Option<Span> = None;

        loop {
            let kind = self.peek_token()?.map(|t| t.kind);
            match kind {
                Some(TokenKind::Anchor) if anchor.is_none() => {
                    let t = self.next_token()?.expect("peeked");
                    anchor_span = Some(t.atom.span);
                    anchor = Some(t.atom.data);
                }
                Some(TokenKind::Tag) if tag.is_none() => {
                    let t = self.next_token()?.expect("peeked");
                    if anchor_span.is_none() {
                        anchor_span = Some(t.atom.span);
                    }
                    tag = Some(t.atom.data);
                }
                _ => break,
            }
        }

        // 3. Dispatch on the next token.
        // cref: fy-parse.c:5851-5983
        let kind = self.peek_token()?.map(|t| t.kind);
        match kind {
            Some(TokenKind::Scalar) => {
                let t = self.next_token()?.expect("peeked");
                self.state = self.pop_state();
                Ok(Some(Event::Scalar {
                    anchor,
                    tag,
                    value: t.atom.data,
                    style: t.atom.style,
                    span: t.atom.span,
                }))
            }

            Some(TokenKind::BlockSequenceStart) => {
                let t = self.next_token()?.expect("peeked");
                self.state = ParserState::BlockSequenceFirstEntry;
                Ok(Some(Event::SequenceStart {
                    anchor,
                    tag,
                    style: CollectionStyle::Block,
                    span: t.atom.span,
                }))
            }

            Some(TokenKind::BlockMappingStart) => {
                let t = self.next_token()?.expect("peeked");
                self.state = ParserState::BlockMappingFirstKey;
                Ok(Some(Event::MappingStart {
                    anchor,
                    tag,
                    style: CollectionStyle::Block,
                    span: t.atom.span,
                }))
            }

            Some(TokenKind::FlowSequenceStart) => {
                let t = self.next_token()?.expect("peeked");
                self.state = ParserState::FlowSequenceFirstEntry;
                Ok(Some(Event::SequenceStart {
                    anchor,
                    tag,
                    style: CollectionStyle::Flow,
                    span: t.atom.span,
                }))
            }

            Some(TokenKind::FlowMappingStart) => {
                let t = self.next_token()?.expect("peeked");
                self.state = ParserState::FlowMappingFirstKey;
                Ok(Some(Event::MappingStart {
                    anchor,
                    tag,
                    style: CollectionStyle::Flow,
                    span: t.atom.span,
                }))
            }

            Some(TokenKind::BlockEntry) => {
                // Indentless sequence: BlockEntry in mapping value/key context.
                // cref: fy-parse.c:5781-5812
                let span = self.peek_token()?.expect("peeked").atom.span;
                let return_state = self.state_stack.last().copied();
                let in_mapping_context = matches!(
                    return_state,
                    Some(ParserState::BlockMappingValue)
                        | Some(ParserState::BlockMappingKey)
                        | Some(ParserState::BlockMappingFirstKey)
                );
                if in_mapping_context {
                    // Don't consume the BlockEntry — let IndentlessSequenceEntry handle it.
                    self.state = ParserState::IndentlessSequenceEntry;
                    Ok(Some(Event::SequenceStart {
                        anchor,
                        tag,
                        style: CollectionStyle::Block,
                        span,
                    }))
                } else if anchor.is_some() || tag.is_some() {
                    // cref: tag/anchor on empty scalar in block sequence —
                    // `- !!str\n- a` → the tag belongs to an empty scalar, the
                    // BlockEntry is the NEXT sequence entry.
                    let span = anchor_span.unwrap_or_default();
                    self.state = self.pop_state();
                    Ok(Some(Event::Scalar {
                        anchor,
                        tag,
                        value: Cow::Borrowed(""),
                        style: ScalarStyle::Plain,
                        span,
                    }))
                } else {
                    Err(ParseError::UnexpectedToken {
                        expected: "node content",
                        got: TokenKind::BlockEntry,
                        span,
                    })
                }
            }

            _ if anchor.is_some() || tag.is_some() => {
                // Anchor/tag present but no content — emit empty scalar.
                let span = anchor_span.unwrap_or_default();
                self.state = self.pop_state();
                Ok(Some(Event::Scalar {
                    anchor,
                    tag,
                    value: Cow::Borrowed(""),
                    style: ScalarStyle::Plain,
                    span,
                }))
            }

            Some(kind) => {
                let span = self.peek_token()?.expect("peeked").atom.span;
                Err(ParseError::UnexpectedToken {
                    expected: "node content (scalar, sequence, mapping, alias, or anchor/tag)",
                    got: kind,
                    span,
                })
            }

            None => Err(ParseError::UnexpectedEof {
                expected: "node content",
                span: Span::default(),
            }),
        }
    }

    /// Handle `BlockSequenceFirstEntry` state: expect the first `BlockEntry`.
    // cref: fy-parse.c:6465-6500
    fn parse_block_sequence_first_entry(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        // First entry MUST be a BlockEntry token.
        let kind = self.peek_token()?.map(|t| t.kind);
        match kind {
            Some(TokenKind::BlockEntry) => {
                let _t = self.next_token()?.expect("peeked");
                // Peek to see if this entry is empty.
                let next_kind = self.peek_token()?.map(|t| t.kind);
                match next_kind {
                    Some(TokenKind::BlockEntry) | Some(TokenKind::BlockEnd) => {
                        // Empty entry — emit empty scalar, stay in BlockSequenceEntry.
                        let span = self.peek_token()?.expect("peeked").atom.span;
                        self.state = ParserState::BlockSequenceEntry;
                        Ok(Some(Self::emit_empty_scalar(span)))
                    }
                    _ => {
                        // Entry has content — recurse into BlockNode.
                        self.push_state(ParserState::BlockSequenceEntry);
                        self.state = ParserState::BlockNode;
                        self.parse_next()
                    }
                }
            }
            Some(kind) => {
                let span = self.peek_token()?.expect("peeked").atom.span;
                Err(ParseError::UnexpectedToken {
                    expected: "BlockEntry (-) in block sequence",
                    got: kind,
                    span,
                })
            }
            None => Err(ParseError::UnexpectedEof {
                expected: "BlockEntry (-) in block sequence",
                span: Span::default(),
            }),
        }
    }

    /// Handle `BlockSequenceEntry` state: expect another `BlockEntry` or `BlockEnd`.
    // cref: fy-parse.c:6502-6550
    fn parse_block_sequence_entry(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let kind = self.peek_token()?.map(|t| t.kind);
        match kind {
            Some(TokenKind::BlockEntry) => {
                let _t = self.next_token()?.expect("peeked");
                // Peek to see if this entry is empty.
                let next_kind = self.peek_token()?.map(|t| t.kind);
                match next_kind {
                    Some(TokenKind::BlockEntry) | Some(TokenKind::BlockEnd) => {
                        // Empty entry — emit empty scalar, stay in BlockSequenceEntry.
                        let span = self.peek_token()?.expect("peeked").atom.span;
                        Ok(Some(Self::emit_empty_scalar(span)))
                    }
                    _ => {
                        // Entry has content — recurse into BlockNode.
                        self.push_state(ParserState::BlockSequenceEntry);
                        self.state = ParserState::BlockNode;
                        self.parse_next()
                    }
                }
            }
            Some(TokenKind::BlockEnd) => {
                let t = self.next_token()?.expect("peeked");
                self.state = self.pop_state();
                Ok(Some(Event::SequenceEnd { span: t.atom.span }))
            }
            Some(kind) => {
                let span = self.peek_token()?.expect("peeked").atom.span;
                Err(ParseError::UnexpectedToken {
                    expected: "BlockEntry (-) or BlockEnd in block sequence",
                    got: kind,
                    span,
                })
            }
            None => Err(ParseError::UnexpectedEof {
                expected: "BlockEntry (-) or BlockEnd in block sequence",
                span: Span::default(),
            }),
        }
    }

    /// Parse a `%YAML` version string like `"1.2"` into `(major, minor)`.
    fn parse_version_string(data: &str, span: Span) -> Result<(u8, u8), ParseError> {
        let parts: Vec<&str> = data.split('.').collect();
        if parts.len() != 2 {
            return Err(ParseError::UnexpectedToken {
                expected: "version string (e.g. 1.2)",
                got: TokenKind::VersionDirective,
                span,
            });
        }
        let major = parts[0]
            .parse::<u8>()
            .map_err(|_| ParseError::UnexpectedToken {
                expected: "numeric major version",
                got: TokenKind::VersionDirective,
                span,
            })?;
        let minor = parts[1]
            .parse::<u8>()
            .map_err(|_| ParseError::UnexpectedToken {
                expected: "numeric minor version",
                got: TokenKind::VersionDirective,
                span,
            })?;
        Ok((major, minor))
    }

    /// Split a `%TAG` directive's atom data into `(handle, prefix)`.
    ///
    /// The scanner emits e.g. `"!e! tag:example.com,2000:"` — split on
    /// the first space.
    fn parse_tag_directive_data(data: &str) -> (&str, &str) {
        match data.split_once(' ') {
            Some((handle, prefix)) => (handle, prefix),
            None => (data, ""),
        }
    }

    // ── Block mapping states ──────────────────────────────────────────

    /// Handle `BlockMappingFirstKey`: expect the first key of a block mapping.
    // cref: fy-parse.c:6551-6664
    fn parse_block_mapping_first_key(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        self.parse_block_mapping_key_impl()
    }

    /// Handle `BlockMappingKey`: expect a key, value-without-key, or BlockEnd.
    // cref: fy-parse.c:6551-6664
    fn parse_block_mapping_key(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        self.parse_block_mapping_key_impl()
    }

    /// Shared implementation for `BlockMappingFirstKey` and `BlockMappingKey`.
    fn parse_block_mapping_key_impl(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let kind = self.peek_token()?.map(|t| t.kind);
        match kind {
            Some(TokenKind::Key) => {
                let _t = self.next_token()?.expect("peeked");
                // Peek at what follows the key indicator.
                let next_kind = self.peek_token()?.map(|t| t.kind);
                match next_kind {
                    Some(TokenKind::Key) | Some(TokenKind::Value) | Some(TokenKind::BlockEnd) => {
                        // Empty key — emit empty scalar, transition to value.
                        let span = self.peek_token()?.expect("peeked").atom.span;
                        self.state = ParserState::BlockMappingValue;
                        Ok(Some(Self::emit_empty_scalar(span)))
                    }
                    _ => {
                        // Key has content — recurse into BlockNode.
                        self.push_state(ParserState::BlockMappingValue);
                        self.state = ParserState::BlockNode;
                        self.parse_next()
                    }
                }
            }
            Some(TokenKind::Value) => {
                // Implicit empty key (`: value` without `?`).
                let span = self.peek_token()?.expect("peeked").atom.span;
                self.state = ParserState::BlockMappingValue;
                Ok(Some(Self::emit_empty_scalar(span)))
            }
            Some(TokenKind::BlockEnd) => {
                let t = self.next_token()?.expect("peeked");
                self.state = self.pop_state();
                Ok(Some(Event::MappingEnd { span: t.atom.span }))
            }
            Some(kind) => {
                let span = self.peek_token()?.expect("peeked").atom.span;
                Err(ParseError::UnexpectedToken {
                    expected: "Key (?), Value (:), or BlockEnd in block mapping",
                    got: kind,
                    span,
                })
            }
            None => Err(ParseError::UnexpectedEof {
                expected: "Key (?), Value (:), or BlockEnd in block mapping",
                span: Span::default(),
            }),
        }
    }

    /// Handle `BlockMappingValue`: expect a value indicator or implicit empty value.
    // cref: fy-parse.c:6620-6664
    fn parse_block_mapping_value(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let kind = self.peek_token()?.map(|t| t.kind);
        match kind {
            Some(TokenKind::Value) => {
                let _t = self.next_token()?.expect("peeked");
                // Peek at what follows the value indicator.
                let next_kind = self.peek_token()?.map(|t| t.kind);
                match next_kind {
                    Some(TokenKind::Key) | Some(TokenKind::Value) | Some(TokenKind::BlockEnd) => {
                        // Empty value — emit empty scalar, transition to next key.
                        let span = self.peek_token()?.expect("peeked").atom.span;
                        self.state = ParserState::BlockMappingKey;
                        Ok(Some(Self::emit_empty_scalar(span)))
                    }
                    _ => {
                        // Value has content — recurse into BlockNode.
                        self.push_state(ParserState::BlockMappingKey);
                        self.state = ParserState::BlockNode;
                        self.parse_next()
                    }
                }
            }
            _ => {
                // No value indicator — implicit empty value.
                let span = self
                    .peek_token()
                    .ok()
                    .flatten()
                    .map_or_else(Span::default, |t| t.atom.span);
                self.state = ParserState::BlockMappingKey;
                Ok(Some(Self::emit_empty_scalar(span)))
            }
        }
    }

    // ── Indentless sequence state ───────────────────────────────────────

    /// Handle `IndentlessSequenceEntry`: expect `BlockEntry` or end of sequence.
    // cref: fy-parse.c:6465-6478
    fn parse_indentless_sequence_entry(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let kind = self.peek_token()?.map(|t| t.kind);
        match kind {
            Some(TokenKind::BlockEntry) => {
                let _t = self.next_token()?.expect("peeked");
                // Peek to see if this entry is empty.
                let next_kind = self.peek_token()?.map(|t| t.kind);
                match next_kind {
                    Some(TokenKind::BlockEntry)
                    | Some(TokenKind::Key)
                    | Some(TokenKind::Value)
                    | Some(TokenKind::BlockEnd) => {
                        // Empty entry — emit empty scalar, stay in IndentlessSequenceEntry.
                        let span = self.peek_token()?.expect("peeked").atom.span;
                        Ok(Some(Self::emit_empty_scalar(span)))
                    }
                    _ => {
                        // Entry has content — recurse into BlockNode.
                        self.push_state(ParserState::IndentlessSequenceEntry);
                        self.state = ParserState::BlockNode;
                        self.parse_next()
                    }
                }
            }
            _ => {
                // No more BlockEntry tokens — sequence is done.
                let span = self
                    .peek_token()
                    .ok()
                    .flatten()
                    .map_or_else(Span::default, |t| t.atom.span);
                self.state = self.pop_state();
                Ok(Some(Event::SequenceEnd { span }))
            }
        }
    }

    // ── Flow sequence states ────────────────────────────────────────────

    /// Handle `FlowSequenceFirstEntry`: expect first entry or `]`.
    // cref: fy-parse.c:6666-6826
    fn parse_flow_sequence_first_entry(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let kind = self.peek_token()?.map(|t| t.kind);
        if kind == Some(TokenKind::FlowSequenceEnd) {
            let t = self.next_token()?.expect("peeked");
            self.state = self.pop_state();
            return Ok(Some(Event::SequenceEnd { span: t.atom.span }));
        }
        // Fall through to parse the first entry (same as after comma).
        self.parse_flow_sequence_entry_content()
    }

    /// Handle `FlowSequenceEntry`: expect `,` + entry or `]`.
    // cref: fy-parse.c:6666-6826
    fn parse_flow_sequence_entry(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let kind = self.peek_token()?.map(|t| t.kind);
        match kind {
            Some(TokenKind::FlowEntry) => {
                let _t = self.next_token()?.expect("peeked");
                // After comma, check what follows.
                // cref: trailing comma — YAML allows `[a, b, ]` with no
                // trailing entry.  Consume `]` directly.
                let next_kind = self.peek_token()?.map(|t| t.kind);
                if next_kind == Some(TokenKind::FlowSequenceEnd) {
                    let t = self.next_token()?.expect("peeked");
                    self.state = self.pop_state();
                    return Ok(Some(Event::SequenceEnd { span: t.atom.span }));
                }
                self.parse_flow_sequence_entry_content()
            }
            Some(TokenKind::FlowSequenceEnd) => {
                let t = self.next_token()?.expect("peeked");
                self.state = self.pop_state();
                Ok(Some(Event::SequenceEnd { span: t.atom.span }))
            }
            Some(kind) => {
                let span = self.peek_token()?.expect("peeked").atom.span;
                Err(ParseError::UnexpectedToken {
                    expected: "FlowEntry (,) or FlowSequenceEnd (]) in flow sequence",
                    got: kind,
                    span,
                })
            }
            None => Err(ParseError::UnexpectedEof {
                expected: "FlowEntry (,) or FlowSequenceEnd (]) in flow sequence",
                span: Span::default(),
            }),
        }
    }

    /// Parse the content of a flow sequence entry (after `[` or `,`).
    ///
    /// Handles implicit mappings (`Key` token) and plain entries.
    fn parse_flow_sequence_entry_content(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let kind = self.peek_token()?.map(|t| t.kind);
        match kind {
            Some(TokenKind::Key) => {
                // Implicit mapping inside flow sequence: `[a: b]`.
                let t = self.next_token()?.expect("peeked");
                // Push FlowSequenceEntryMappingValue as the return state for BlockNode.
                // FlowSequenceEntryMappingValue will handle pushing FlowSequenceEntryMappingEnd.
                self.push_state(ParserState::FlowSequenceEntryMappingValue);
                self.state = ParserState::BlockNode;
                Ok(Some(Event::MappingStart {
                    anchor: None,
                    tag: None,
                    style: CollectionStyle::Flow,
                    span: t.atom.span,
                }))
            }
            Some(TokenKind::Value) => {
                // Implicit mapping with empty key: `[: value]`.
                let span = self.peek_token()?.expect("peeked").atom.span;
                self.state = ParserState::FlowSequenceEntryMappingValue;
                Ok(Some(Event::MappingStart {
                    anchor: None,
                    tag: None,
                    style: CollectionStyle::Flow,
                    span,
                }))
            }
            _ => {
                // Normal entry — parse as a node.
                self.push_state(ParserState::FlowSequenceEntry);
                self.state = ParserState::BlockNode;
                self.parse_next()
            }
        }
    }

    /// Handle `FlowSequenceEntryMappingKey`: parse key of implicit mapping.
    ///
    /// This state is entered when we see `Value` directly (empty key case)
    /// after the MappingStart was already emitted. We need to emit the empty
    /// scalar for the key and then transition to parse the value.
    // cref: fy-parse.c:6780-6826
    fn parse_flow_sequence_entry_mapping_key(
        &mut self,
    ) -> Result<Option<Event<'input>>, ParseError> {
        // This state shouldn't normally be reached because after Key we go
        // directly to BlockNode with FlowSequenceEntryMappingValue pushed.
        // But if we enter here, treat it as empty key.
        let span = self
            .peek_token()
            .ok()
            .flatten()
            .map_or_else(Span::default, |t| t.atom.span);
        self.state = ParserState::FlowSequenceEntryMappingValue;
        Ok(Some(Self::emit_empty_scalar(span)))
    }

    /// Handle `FlowSequenceEntryMappingValue`: expect `:` + value or empty value.
    // cref: fy-parse.c:6800-6826
    fn parse_flow_sequence_entry_mapping_value(
        &mut self,
    ) -> Result<Option<Event<'input>>, ParseError> {
        let kind = self.peek_token()?.map(|t| t.kind);
        match kind {
            Some(TokenKind::Value) => {
                let _t = self.next_token()?.expect("peeked");
                // Peek to see if value is empty.
                let next_kind = self.peek_token()?.map(|t| t.kind);
                match next_kind {
                    Some(TokenKind::FlowEntry)
                    | Some(TokenKind::FlowSequenceEnd)
                    | Some(TokenKind::FlowMappingEnd) => {
                        // Empty value.
                        let span = self.peek_token()?.expect("peeked").atom.span;
                        self.state = ParserState::FlowSequenceEntryMappingEnd;
                        Ok(Some(Self::emit_empty_scalar(span)))
                    }
                    _ => {
                        // Value has content.
                        self.push_state(ParserState::FlowSequenceEntryMappingEnd);
                        self.state = ParserState::BlockNode;
                        self.parse_next()
                    }
                }
            }
            _ => {
                // No `:` — empty value.
                let span = self
                    .peek_token()
                    .ok()
                    .flatten()
                    .map_or_else(Span::default, |t| t.atom.span);
                self.state = ParserState::FlowSequenceEntryMappingEnd;
                Ok(Some(Self::emit_empty_scalar(span)))
            }
        }
    }

    /// Handle `FlowSequenceEntryMappingEnd`: emit `MappingEnd` and return to
    /// `FlowSequenceEntry`.
    // cref: fy-parse.c:6823-6826
    fn parse_flow_sequence_entry_mapping_end(
        &mut self,
    ) -> Result<Option<Event<'input>>, ParseError> {
        let span = self
            .peek_token()
            .ok()
            .flatten()
            .map_or_else(Span::default, |t| t.atom.span);
        self.state = ParserState::FlowSequenceEntry;
        Ok(Some(Event::MappingEnd { span }))
    }

    // ── Flow mapping states ─────────────────────────────────────────────

    /// Handle `FlowMappingFirstKey`: expect first key or `}`.
    // cref: fy-parse.c:6827-6964
    fn parse_flow_mapping_first_key(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let kind = self.peek_token()?.map(|t| t.kind);
        if kind == Some(TokenKind::FlowMappingEnd) {
            let t = self.next_token()?.expect("peeked");
            self.state = self.pop_state();
            return Ok(Some(Event::MappingEnd { span: t.atom.span }));
        }
        // Parse key content.
        self.parse_flow_mapping_key_content()
    }

    /// Handle `FlowMappingKey`: expect `,` + key or `}`.
    // cref: fy-parse.c:6827-6964
    fn parse_flow_mapping_key(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let kind = self.peek_token()?.map(|t| t.kind);
        match kind {
            Some(TokenKind::FlowEntry) => {
                let _t = self.next_token()?.expect("peeked");
                // After comma, check what follows.
                // cref: trailing comma — YAML allows `{a: b, }` with no
                // trailing entry.  Consume `}` directly.
                let next_kind = self.peek_token()?.map(|t| t.kind);
                match next_kind {
                    Some(TokenKind::FlowMappingEnd) => {
                        let t = self.next_token()?.expect("peeked");
                        self.state = self.pop_state();
                        Ok(Some(Event::MappingEnd { span: t.atom.span }))
                    }
                    _ => self.parse_flow_mapping_key_content(),
                }
            }
            Some(TokenKind::FlowMappingEnd) => {
                let t = self.next_token()?.expect("peeked");
                self.state = self.pop_state();
                Ok(Some(Event::MappingEnd { span: t.atom.span }))
            }
            Some(kind) => {
                let span = self.peek_token()?.expect("peeked").atom.span;
                Err(ParseError::UnexpectedToken {
                    expected: "FlowEntry (,) or FlowMappingEnd (}) in flow mapping",
                    got: kind,
                    span,
                })
            }
            None => Err(ParseError::UnexpectedEof {
                expected: "FlowEntry (,) or FlowMappingEnd (}) in flow mapping",
                span: Span::default(),
            }),
        }
    }

    /// Parse the key content of a flow mapping entry (after `{` or `,`).
    fn parse_flow_mapping_key_content(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let kind = self.peek_token()?.map(|t| t.kind);
        match kind {
            Some(TokenKind::Key) => {
                let _t = self.next_token()?.expect("peeked");
                // Peek at what follows the explicit key.
                let next_kind = self.peek_token()?.map(|t| t.kind);
                match next_kind {
                    Some(TokenKind::Value)
                    | Some(TokenKind::FlowEntry)
                    | Some(TokenKind::FlowMappingEnd) => {
                        // Empty key.
                        let span = self.peek_token()?.expect("peeked").atom.span;
                        self.state = ParserState::FlowMappingValue;
                        Ok(Some(Self::emit_empty_scalar(span)))
                    }
                    _ => {
                        // Key has content.
                        self.push_state(ParserState::FlowMappingValue);
                        self.state = ParserState::BlockNode;
                        self.parse_next()
                    }
                }
            }
            Some(TokenKind::Value) => {
                // Implicit empty key (`: value` without `?`).
                let span = self.peek_token()?.expect("peeked").atom.span;
                self.state = ParserState::FlowMappingValue;
                Ok(Some(Self::emit_empty_scalar(span)))
            }
            _ => {
                // Implicit key (no `?`) — parse key node directly.
                self.push_state(ParserState::FlowMappingValue);
                self.state = ParserState::BlockNode;
                self.parse_next()
            }
        }
    }

    /// Handle `FlowMappingValue`: expect `:` + value or empty value.
    // cref: fy-parse.c:6930-6964
    fn parse_flow_mapping_value(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let kind = self.peek_token()?.map(|t| t.kind);
        match kind {
            Some(TokenKind::Value) => {
                let _t = self.next_token()?.expect("peeked");
                // Peek to see if value is empty.
                let next_kind = self.peek_token()?.map(|t| t.kind);
                match next_kind {
                    Some(TokenKind::FlowEntry) | Some(TokenKind::FlowMappingEnd) => {
                        // Empty value.
                        let span = self.peek_token()?.expect("peeked").atom.span;
                        self.state = ParserState::FlowMappingKey;
                        Ok(Some(Self::emit_empty_scalar(span)))
                    }
                    _ => {
                        // Value has content.
                        self.push_state(ParserState::FlowMappingKey);
                        self.state = ParserState::BlockNode;
                        self.parse_next()
                    }
                }
            }
            _ => {
                // No `:` — empty value.
                let span = self
                    .peek_token()
                    .ok()
                    .flatten()
                    .map_or_else(Span::default, |t| t.atom.span);
                self.state = ParserState::FlowMappingKey;
                Ok(Some(Self::emit_empty_scalar(span)))
            }
        }
    }

    /// Handle `FlowMappingEmptyValue`: emit empty scalar, transition to key.
    // cref: fy-parse.c:6960-6964
    fn parse_flow_mapping_empty_value(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let span = self
            .peek_token()
            .ok()
            .flatten()
            .map_or_else(Span::default, |t| t.atom.span);
        self.state = ParserState::FlowMappingKey;
        Ok(Some(Self::emit_empty_scalar(span)))
    }
}

impl<'input> Iterator for Parser<'input> {
    type Item = Result<Event<'input>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
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

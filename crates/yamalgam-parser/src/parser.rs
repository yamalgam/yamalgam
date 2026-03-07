//! YAML pull parser — consumes tokens, emits events.

use std::borrow::Cow;

use yamalgam_core::Span;
use yamalgam_scanner::scanner::{ScanError, Scanner};
use yamalgam_scanner::{ScalarStyle, Token, TokenKind};

use crate::error::ParseError;
use crate::event::Event;

/// Parser states matching libfyaml's `fy_parser_state`.
// cref: fy-parse.h:86-135
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Variants used incrementally as parser states are implemented.
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
            ParserState::BlockNode => self.parse_block_node(),
            ParserState::End => {
                self.done = true;
                Ok(None)
            }
            // Temporary catch-all: check for StreamEnd token.
            _ => self.parse_catchall(),
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
        let token = self.peek_token()?;
        match token {
            Some(t) if t.kind == TokenKind::VersionDirective => {
                // Consume the directive token, emit as event, stay in same state.
                let t = self.next_token()?.expect("peeked");
                let (major, minor) = Self::parse_version_string(&t.atom.data, t.atom.span)?;
                Ok(Some(Event::VersionDirective {
                    major,
                    minor,
                    span: t.atom.span,
                }))
            }
            Some(t) if t.kind == TokenKind::TagDirective => {
                // Consume the directive token, emit as event, stay in same state.
                let t = self.next_token()?.expect("peeked");
                let (handle, prefix) = Self::parse_tag_directive_data(&t.atom.data);
                Ok(Some(Event::TagDirective {
                    handle: Cow::Owned(handle.to_string()),
                    prefix: Cow::Owned(prefix.to_string()),
                    span: t.atom.span,
                }))
            }
            Some(t) if t.kind == TokenKind::DocumentStart => {
                // Explicit `---` — transition to DocumentStart state.
                self.state = ParserState::DocumentStart;
                self.parse_document_start()
            }
            Some(t) if t.kind == TokenKind::StreamEnd => {
                // Empty stream (no documents). Consume and emit StreamEnd.
                let _t = self.next_token()?;
                self.state = ParserState::End;
                Ok(Some(Event::StreamEnd))
            }
            Some(t) => {
                // Content token — this is an implicit document start.
                // Don't consume the token; let BlockNode handle it.
                let span = t.atom.span;
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

    /// Handle `BlockNode` state (temporary): only handles scalars for now.
    /// Will be expanded in later tasks to handle all block-level nodes.
    fn parse_block_node(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let token = self.peek_token()?;
        match token {
            Some(t) if t.kind == TokenKind::Scalar => {
                let t = self.next_token()?.expect("peeked");
                self.state = self.pop_state();
                Ok(Some(Event::Scalar {
                    anchor: None,
                    tag: None,
                    value: t.atom.data,
                    style: t.atom.style,
                    span: t.atom.span,
                }))
            }
            _ => self.parse_catchall(),
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
        let major = parts[0].parse::<u8>().map_err(|_| ParseError::UnexpectedToken {
            expected: "numeric major version",
            got: TokenKind::VersionDirective,
            span,
        })?;
        let minor = parts[1].parse::<u8>().map_err(|_| ParseError::UnexpectedToken {
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

    /// Temporary catch-all for unimplemented states: if the next token is
    /// `StreamEnd`, emit the event and transition to `End`. Otherwise, error.
    fn parse_catchall(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        let token = self.peek_token()?;
        match token {
            Some(t) if t.kind == TokenKind::StreamEnd => {
                // Consume it.
                let _t = self.next_token()?;
                self.state = ParserState::End;
                Ok(Some(Event::StreamEnd))
            }
            Some(t) => {
                let kind = t.kind;
                let span = t.atom.span;
                Err(ParseError::UnexpectedToken {
                    expected: "StreamEnd (state not yet implemented)",
                    got: kind,
                    span,
                })
            }
            None => Err(ParseError::UnexpectedEof {
                expected: "StreamEnd",
                span: Span::default(),
            }),
        }
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

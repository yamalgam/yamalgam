//! YAML pull parser — consumes tokens, emits events.

use yamalgam_core::Span;
use yamalgam_scanner::scanner::{ScanError, Scanner};
use yamalgam_scanner::{Token, TokenKind};

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
    #[allow(dead_code)] // Used once document/collection parsing lands.
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
    #[allow(dead_code)] // Used once document/collection parsing lands.
    fn push_state(&mut self, state: ParserState) {
        self.state_stack.push(state);
    }

    /// Pop a state from the state stack, defaulting to `End`.
    // cref: fy-parse.c:5688-5702 (fy_parse_state_pop)
    #[allow(dead_code)] // Used once document/collection parsing lands.
    fn pop_state(&mut self) -> ParserState {
        self.state_stack.pop().unwrap_or(ParserState::End)
    }

    /// Core dispatch: produce the next event based on the current state.
    // cref: fy-parse.c:6044-7060 (fy_parse_internal)
    fn parse_next(&mut self) -> Result<Option<Event<'input>>, ParseError> {
        if self.done {
            return Ok(None);
        }

        match self.state {
            ParserState::StreamStart => self.parse_stream_start(),
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

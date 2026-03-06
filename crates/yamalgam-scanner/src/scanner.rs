//! YAML scanner state machine.
//!
//! Converts decoded UTF-8 input into a stream of [`Token`]s. The scanner is
//! modeled as an iterator that yields `Result<Token, ScanError>`.
//!
//! Currently only emits stream markers (`StreamStart` / `StreamEnd`).
//! Content token scanning will be added incrementally.

use std::borrow::Cow;

use yamalgam_core::Span;

use crate::Atom;
use crate::reader::Reader;
use crate::style::{AtomFlags, Chomp, ScalarStyle};
use crate::token::{Token, TokenKind};

/// Scanner state machine phases.
// cref: fy_parser.stream_start_produced, fy_parser.stream_end_reached, fy_parser.stream_end_produced
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Initial state — will emit `StreamStart`.
    Start,
    /// Inside the stream — scanning content tokens.
    /// (Currently skips all content and transitions to `End`.)
    Stream,
    /// Will emit `StreamEnd`.
    End,
    /// Past `StreamEnd` — no more tokens.
    Done,
}

/// Error type for scanner failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanError {
    /// Human-readable error message.
    pub message: String,
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "scan error: {}", self.message)
    }
}

impl std::error::Error for ScanError {}

/// YAML token scanner.
///
/// Wraps a [`Reader`] over decoded UTF-8 input and yields [`Token`]s via the
/// [`Iterator`] trait. The scanner manages a state machine that tracks which
/// phase of the YAML stream it's in.
// cref: fy_parser (fy-parse.h)
pub struct Scanner<'input> {
    reader: Reader<'input>,
    state: State,
}

impl<'input> Scanner<'input> {
    /// Create a new scanner over decoded UTF-8 input.
    #[must_use]
    pub const fn new(input: &'input str) -> Self {
        Self {
            reader: Reader::new(input),
            state: State::Start,
        }
    }

    /// Build a token with an empty atom at the current reader position.
    fn empty_token(&self, kind: TokenKind) -> Token<'input> {
        let mark = self.reader.mark();
        Token {
            kind,
            atom: Atom {
                data: Cow::Borrowed(""),
                span: Span {
                    start: mark,
                    end: mark,
                },
                style: ScalarStyle::Plain,
                chomp: Chomp::default(),
                flags: AtomFlags::empty(),
            },
        }
    }

    /// Emit `StreamStart` and transition to `Stream`.
    // cref: fy_fetch_stream_start (fy-parse.c:1921)
    fn fetch_stream_start(&mut self) -> Token<'input> {
        let token = self.empty_token(TokenKind::StreamStart);
        self.state = State::Stream;
        token
    }

    /// Skip remaining input and transition to `End`.
    ///
    /// Placeholder: once content scanning is implemented, `State::Stream`
    /// will drive the main token-fetching loop instead of jumping here.
    fn skip_to_end(&mut self) {
        while self.reader.advance().is_some() {}
        self.state = State::End;
    }

    /// Emit `StreamEnd` and transition to `Done`.
    // cref: fy_fetch_stream_end (fy-parse.c:1939)
    fn fetch_stream_end(&mut self) -> Token<'input> {
        let token = self.empty_token(TokenKind::StreamEnd);
        self.state = State::Done;
        token
    }
}

impl<'input> Iterator for Scanner<'input> {
    type Item = Result<Token<'input>, ScanError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            State::Start => Some(Ok(self.fetch_stream_start())),
            State::Stream => {
                // No content scanning yet — skip to end.
                self.skip_to_end();
                Some(Ok(self.fetch_stream_end()))
            }
            State::End => Some(Ok(self.fetch_stream_end())),
            State::Done => None,
        }
    }
}

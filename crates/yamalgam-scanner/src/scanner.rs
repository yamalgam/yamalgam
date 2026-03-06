//! YAML scanner state machine.
//!
//! Converts decoded UTF-8 input into a stream of [`Token`]s. The scanner is
//! modeled as an iterator that yields `Result<Token, ScanError>`.
//!
//! Currently handles stream markers and document markers
//! (`---`, `...`, `%YAML`, `%TAG`). Content tokens are skipped.

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
    /// Inside the stream — fetching tokens.
    Stream,
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

    /// Build a token with an empty atom spanning from `start` to `end`.
    fn marker_token(
        &self,
        kind: TokenKind,
        start: yamalgam_core::Mark,
        end: yamalgam_core::Mark,
    ) -> Token<'input> {
        Token {
            kind,
            atom: Atom {
                data: Cow::Borrowed(""),
                span: Span { start, end },
                style: ScalarStyle::Plain,
                chomp: Chomp::default(),
                flags: AtomFlags::empty(),
            },
        }
    }

    /// Build a token with borrowed atom data spanning from `start` to `end`.
    fn data_token(
        &self,
        kind: TokenKind,
        data: &'input str,
        start: yamalgam_core::Mark,
        end: yamalgam_core::Mark,
    ) -> Token<'input> {
        Token {
            kind,
            atom: Atom {
                data: Cow::Borrowed(data),
                span: Span { start, end },
                style: ScalarStyle::Plain,
                chomp: Chomp::default(),
                flags: AtomFlags::empty(),
            },
        }
    }

    /// Emit `StreamStart` and transition to `Stream`.
    // cref: fy_fetch_stream_start (fy-parse.c:1921)
    fn fetch_stream_start(&mut self) -> Token<'input> {
        let mark = self.reader.mark();
        self.state = State::Stream;
        self.marker_token(TokenKind::StreamStart, mark, mark)
    }

    /// Emit `StreamEnd` and transition to `Done`.
    // cref: fy_fetch_stream_end (fy-parse.c:1939)
    fn fetch_stream_end(&mut self) -> Token<'input> {
        let mark = self.reader.mark();
        self.state = State::Done;
        self.marker_token(TokenKind::StreamEnd, mark, mark)
    }

    /// Skip whitespace, comments, and newlines between tokens.
    ///
    /// After this returns, the reader is positioned at the first character
    /// that could start a token (or at EOF).
    // cref: fy_scan_to_next_token (fy-parse.c:1260)
    fn scan_to_next_token(&mut self) {
        loop {
            // Skip whitespace (space, tab).
            while let Some(c) = self.reader.peek() {
                if c == ' ' || c == '\t' {
                    self.reader.advance();
                } else {
                    break;
                }
            }

            // Skip comment (# to end of line).
            if self.reader.peek() == Some('#') {
                while let Some(c) = self.reader.peek() {
                    if c == '\n' || c == '\r' {
                        break;
                    }
                    self.reader.advance();
                }
            }

            // Skip newline and loop to handle the next line.
            match self.reader.peek() {
                Some('\n' | '\r') => {
                    self.reader.advance();
                }
                _ => break,
            }
        }
    }

    /// Check if the reader is positioned at `---` followed by blank/EOF.
    // cref: fy_fetch_tokens (fy-parse.c:5326) — "---" check
    fn is_document_start(&self) -> bool {
        self.reader.peek() == Some('-')
            && self.reader.peek_at(1) == Some('-')
            && self.reader.peek_at(2) == Some('-')
            && is_blank_or_end(self.reader.peek_at(3))
    }

    /// Check if the reader is positioned at `...` followed by blank/EOF.
    // cref: fy_fetch_tokens (fy-parse.c:5328) — "..." check
    fn is_document_end(&self) -> bool {
        self.reader.peek() == Some('.')
            && self.reader.peek_at(1) == Some('.')
            && self.reader.peek_at(2) == Some('.')
            && is_blank_or_end(self.reader.peek_at(3))
    }

    /// Consume `---` or `...` and emit a document indicator token.
    // cref: fy_fetch_document_indicator (fy-parse.c:2379)
    fn fetch_document_indicator(&mut self, kind: TokenKind) -> Token<'input> {
        let start = self.reader.mark();
        self.reader.advance_by(3);
        let end = self.reader.mark();
        self.marker_token(kind, start, end)
    }

    /// Scan a `%YAML` or `%TAG` directive.
    ///
    /// Called when the `%` has been detected at column 0.
    /// The `%` has NOT been consumed yet.
    // cref: fy_scan_directive (fy-parse.c:2197)
    fn fetch_directive(&mut self) -> Result<Token<'input>, ScanError> {
        // Skip past '%'
        self.reader.advance();

        if self.check_prefix("YAML") && is_blank(self.reader.peek_at(4)) {
            self.fetch_version_directive()
        } else if self.check_prefix("TAG") && is_blank(self.reader.peek_at(3)) {
            self.fetch_tag_directive()
        } else {
            // Unknown directive — skip to end of line.
            self.skip_to_next_line();
            Err(ScanError {
                message: "unknown directive".to_string(),
            })
        }
    }

    /// Scan `%YAML x.y` — reader is past `%`, positioned at `YAML`.
    // cref: fy_scan_directive (fy-parse.c:2275) — version directive branch
    fn fetch_version_directive(&mut self) -> Result<Token<'input>, ScanError> {
        // Skip "YAML"
        self.reader.advance_by(4);

        // Skip whitespace after YAML
        self.skip_blanks();

        // Parse version: digits.digits
        let ver_start = self.reader.mark();
        while let Some(c) = self.reader.peek() {
            if c.is_ascii_digit() || c == '.' {
                self.reader.advance();
            } else {
                break;
            }
        }
        let ver_end = self.reader.mark();

        if ver_start.offset == ver_end.offset {
            return Err(ScanError {
                message: "expected version after %YAML".to_string(),
            });
        }

        let version_str = self.reader.slice(ver_start.offset, ver_end.offset);

        // Skip rest of line (trailing whitespace, comments).
        self.skip_to_next_line();

        Ok(self.data_token(TokenKind::VersionDirective, version_str, ver_start, ver_end))
    }

    /// Scan `%TAG handle prefix` — reader is past `%`, positioned at `TAG`.
    // cref: fy_scan_directive (fy-parse.c:2296) — tag directive branch
    fn fetch_tag_directive(&mut self) -> Result<Token<'input>, ScanError> {
        // Skip "TAG"
        self.reader.advance_by(3);

        // Skip whitespace after TAG
        self.skip_blanks();

        // Capture the full "handle prefix" portion.
        let data_start = self.reader.mark();

        // Scan tag handle: ! or !! or !name!
        if self.reader.peek() != Some('!') {
            return Err(ScanError {
                message: "expected '!' at start of tag handle".to_string(),
            });
        }
        self.reader.advance(); // skip '!'

        // Read handle body (alphanumeric/-) until '!' or whitespace.
        while let Some(c) = self.reader.peek() {
            if c == '!' {
                self.reader.advance();
                break;
            }
            if is_blank(Some(c)) || is_linebreak(c) {
                break;
            }
            self.reader.advance();
        }

        // Skip whitespace between handle and prefix.
        self.skip_blanks();

        // Read tag prefix (URI) until whitespace/newline/EOF.
        while let Some(c) = self.reader.peek() {
            if is_blank(Some(c)) || is_linebreak(c) {
                break;
            }
            self.reader.advance();
        }
        let data_end = self.reader.mark();

        let tag_data = self.reader.slice(data_start.offset, data_end.offset);

        // Skip rest of line.
        self.skip_to_next_line();

        Ok(self.data_token(TokenKind::TagDirective, tag_data, data_start, data_end))
    }

    /// Check if the next `n` characters match `prefix`.
    fn check_prefix(&self, prefix: &str) -> bool {
        prefix
            .chars()
            .enumerate()
            .all(|(i, expected)| self.reader.peek_at(i) == Some(expected))
    }

    /// Skip whitespace characters (space and tab).
    fn skip_blanks(&mut self) {
        while is_blank(self.reader.peek()) {
            self.reader.advance();
        }
    }

    /// Advance to the next line or EOF.
    fn skip_to_next_line(&mut self) {
        while let Some(c) = self.reader.peek() {
            if c == '\n' || c == '\r' {
                self.reader.advance();
                break;
            }
            self.reader.advance();
        }
    }

    /// Fetch the next token from the stream.
    ///
    /// Skips whitespace/comments, then checks for document indicators,
    /// directives, and EOF. Unrecognized content is skipped line by line.
    // cref: fy_fetch_tokens (fy-parse.c:5250)
    fn fetch_next_token(&mut self) -> Option<Result<Token<'input>, ScanError>> {
        loop {
            self.scan_to_next_token();

            if self.reader.is_eof() {
                return Some(Ok(self.fetch_stream_end()));
            }

            let col = self.reader.mark().column;

            if col == 0 {
                // Document indicators: --- or ...
                if self.is_document_start() {
                    return Some(Ok(self.fetch_document_indicator(TokenKind::DocumentStart)));
                }
                if self.is_document_end() {
                    return Some(Ok(self.fetch_document_indicator(TokenKind::DocumentEnd)));
                }
                // Directives: %YAML or %TAG
                if self.reader.peek() == Some('%') {
                    return Some(self.fetch_directive());
                }
            }

            // Unknown content — skip to next line and try again.
            self.skip_to_next_line();
        }
    }
}

/// Returns `true` if the character is a blank (space or tab) or absent (EOF).
const fn is_blank_or_end(c: Option<char>) -> bool {
    matches!(c, None | Some(' ' | '\t' | '\n' | '\r'))
}

/// Returns `true` if the character is a blank (space or tab).
const fn is_blank(c: Option<char>) -> bool {
    matches!(c, Some(' ' | '\t'))
}

/// Returns `true` if the character is a YAML line break.
const fn is_linebreak(c: char) -> bool {
    c == '\n' || c == '\r'
}

impl<'input> Iterator for Scanner<'input> {
    type Item = Result<Token<'input>, ScanError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            State::Start => Some(Ok(self.fetch_stream_start())),
            State::Stream => self.fetch_next_token(),
            State::Done => None,
        }
    }
}

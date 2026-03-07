//! YAML scanner state machine.
//!
//! Converts decoded UTF-8 input into a stream of [`Token`]s. The scanner is
//! modeled as an iterator that yields `Result<Token, ScanError>`.
//!
//! Currently handles stream markers, document markers
//! (`---`, `...`, `%YAML`, `%TAG`), flow indicators
//! (`[`, `]`, `{`, `}`, `,`), and block indicators
//! (`-`, `?`, `:`, indent-based `BlockSequenceStart`/`BlockMappingStart`/`BlockEnd`).
//! Scalar and other content tokens are skipped.

use std::borrow::Cow;
use std::collections::VecDeque;

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
    /// Nesting depth of flow collections (`[`/`{` increments, `]`/`}` decrements).
    // cref: fy_parser.flow_level
    flow_level: u32,
    /// Current block indentation level. `-1` means "no block context yet".
    // cref: fy_parser.indent
    indent: i32,
    /// Stack of previous indentation levels for unrolling.
    // cref: fy_indent list in fy_parser
    indent_stack: Vec<i32>,
    /// Queued tokens waiting to be yielded (e.g., BlockSequenceStart before BlockEntry).
    queue: VecDeque<Token<'input>>,
    /// Buffered anchor/tag tokens that might precede a simple key.
    ///
    /// When the scanner encounters `&anchor` or `!tag` before a scalar, it
    /// can't know yet whether the scalar is a mapping key. These tokens are
    /// held here until the next scalar resolves the ambiguity: if it's a key,
    /// `BlockMappingStart` + `Key` are inserted before these prefix tokens.
    pending_prefix: Vec<Token<'input>>,
}

impl<'input> Scanner<'input> {
    /// Create a new scanner over decoded UTF-8 input.
    #[must_use]
    pub const fn new(input: &'input str) -> Self {
        Self {
            reader: Reader::new(input),
            state: State::Start,
            flow_level: 0,
            indent: -1,
            indent_stack: Vec::new(),
            queue: VecDeque::new(),
            pending_prefix: Vec::new(),
        }
    }

    // -- Token constructors --

    /// Build a token with an empty atom spanning from `start` to `end`.
    fn marker_token(
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

    /// Build a scalar token with owned or borrowed data and explicit style.
    fn scalar_token(
        data: Cow<'input, str>,
        style: ScalarStyle,
        start: yamalgam_core::Mark,
        end: yamalgam_core::Mark,
    ) -> Token<'input> {
        Token {
            kind: TokenKind::Scalar,
            atom: Atom {
                data,
                span: Span { start, end },
                style,
                chomp: Chomp::default(),
                flags: AtomFlags::empty(),
            },
        }
    }

    // -- Stream markers --

    /// Emit `StreamStart` and transition to `Stream`.
    // cref: fy_fetch_stream_start (fy-parse.c:1921)
    fn fetch_stream_start(&mut self) -> Token<'input> {
        let mark = self.reader.mark();
        self.state = State::Stream;
        Self::marker_token(TokenKind::StreamStart, mark, mark)
    }

    /// Emit `StreamEnd` and transition to `Done`.
    // cref: fy_fetch_stream_end (fy-parse.c:1939)
    fn fetch_stream_end(&mut self) -> Token<'input> {
        let mark = self.reader.mark();
        self.state = State::Done;
        Self::marker_token(TokenKind::StreamEnd, mark, mark)
    }

    // -- Whitespace/comment skipping --

    /// Skip whitespace, comments, and newlines between tokens.
    // cref: fy_scan_to_next_token (fy-parse.c:1260)
    fn scan_to_next_token(&mut self) {
        loop {
            while let Some(c) = self.reader.peek() {
                if c == ' ' || c == '\t' {
                    self.reader.advance();
                } else {
                    break;
                }
            }

            if self.reader.peek() == Some('#') {
                while let Some(c) = self.reader.peek() {
                    if c == '\n' || c == '\r' {
                        break;
                    }
                    self.reader.advance();
                }
            }

            match self.reader.peek() {
                Some('\n' | '\r') => {
                    self.reader.advance();
                }
                _ => break,
            }
        }
    }

    // -- Indent management --

    /// Push the current indent level and emit a block collection start token
    /// if the column is deeper than the current indent.
    ///
    /// Returns `true` if a new indent level was pushed.
    // cref: fy_push_indent (fy-parse.c) + BLOCK_SEQUENCE_START/BLOCK_MAPPING_START emit
    fn roll_indent(&mut self, column: i32, is_mapping: bool) {
        if self.flow_level > 0 || column <= self.indent {
            return;
        }
        self.indent_stack.push(self.indent);
        self.indent = column;
        let mark = self.reader.mark();
        let kind = if is_mapping {
            TokenKind::BlockMappingStart
        } else {
            TokenKind::BlockSequenceStart
        };
        self.queue.push_back(Self::marker_token(kind, mark, mark));
    }

    /// Emit `BlockEnd` tokens for each indent level deeper than `column`.
    // cref: fy_parse_unroll_indent (fy-parse.c:1592)
    fn unroll_indent(&mut self, column: i32) {
        if self.flow_level > 0 {
            return;
        }
        while self.indent > column {
            let mark = self.reader.mark();
            self.queue
                .push_back(Self::marker_token(TokenKind::BlockEnd, mark, mark));
            self.indent = self.indent_stack.pop().unwrap_or(-1);
        }
    }

    // -- Document markers --

    /// Check if the reader is positioned at `---` followed by blank/EOF.
    // cref: fy_fetch_tokens (fy-parse.c:5326)
    fn is_document_start(&self) -> bool {
        self.reader.peek() == Some('-')
            && self.reader.peek_at(1) == Some('-')
            && self.reader.peek_at(2) == Some('-')
            && is_blank_or_end(self.reader.peek_at(3))
    }

    /// Check if the reader is positioned at `...` followed by blank/EOF.
    // cref: fy_fetch_tokens (fy-parse.c:5328)
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
        Self::marker_token(kind, start, end)
    }

    // -- Directives --

    /// Scan a `%YAML` or `%TAG` directive.
    // cref: fy_scan_directive (fy-parse.c:2197)
    fn fetch_directive(&mut self) -> Result<Token<'input>, ScanError> {
        self.reader.advance(); // skip '%'
        if self.check_prefix("YAML") && is_blank(self.reader.peek_at(4)) {
            self.fetch_version_directive()
        } else if self.check_prefix("TAG") && is_blank(self.reader.peek_at(3)) {
            self.fetch_tag_directive()
        } else {
            // YAML 1.2 §6.8.1: unknown directives should be ignored.
            // cref: fy_fetch_directive — libfyaml skips unknown directives with a warning
            self.skip_to_next_line();
            Err(ScanError {
                message: "unknown directive".to_string(),
            })
        }
    }

    /// Scan `%YAML x.y`.
    // cref: fy_scan_directive (fy-parse.c:2275)
    fn fetch_version_directive(&mut self) -> Result<Token<'input>, ScanError> {
        self.reader.advance_by(4); // skip "YAML"
        self.skip_blanks();
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
        self.skip_to_next_line();
        Ok(Self::data_token(
            TokenKind::VersionDirective,
            version_str,
            ver_start,
            ver_end,
        ))
    }

    /// Scan `%TAG handle prefix`.
    // cref: fy_scan_directive (fy-parse.c:2296)
    fn fetch_tag_directive(&mut self) -> Result<Token<'input>, ScanError> {
        self.reader.advance_by(3); // skip "TAG"
        self.skip_blanks();
        let data_start = self.reader.mark();
        if self.reader.peek() != Some('!') {
            return Err(ScanError {
                message: "expected '!' at start of tag handle".to_string(),
            });
        }
        self.reader.advance();
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
        self.skip_blanks();
        while let Some(c) = self.reader.peek() {
            if is_blank(Some(c)) || is_linebreak(c) {
                break;
            }
            self.reader.advance();
        }
        let data_end = self.reader.mark();
        let tag_data = self.reader.slice(data_start.offset, data_end.offset);
        self.skip_to_next_line();
        Ok(Self::data_token(
            TokenKind::TagDirective,
            tag_data,
            data_start,
            data_end,
        ))
    }

    // -- Flow indicators --

    /// Consume `[` or `{` and emit a flow collection start token.
    // cref: fy_fetch_flow_collection_mark_start (fy-parse.c:2432)
    fn fetch_flow_collection_start(&mut self, c: char) -> Token<'input> {
        let kind = if c == '[' {
            TokenKind::FlowSequenceStart
        } else {
            TokenKind::FlowMappingStart
        };
        let start = self.reader.mark();
        self.reader.advance();
        let end = self.reader.mark();
        self.flow_level += 1;
        Self::marker_token(kind, start, end)
    }

    /// Consume `]` or `}` and emit a flow collection end token.
    // cref: fy_fetch_flow_collection_mark_end (fy-parse.c:2518)
    fn fetch_flow_collection_end(&mut self, c: char) -> Token<'input> {
        let kind = if c == ']' {
            TokenKind::FlowSequenceEnd
        } else {
            TokenKind::FlowMappingEnd
        };
        let start = self.reader.mark();
        self.reader.advance();
        let end = self.reader.mark();
        self.flow_level = self.flow_level.saturating_sub(1);
        Self::marker_token(kind, start, end)
    }

    /// Consume `,` and emit a flow entry token.
    // cref: fy_parse_handle_comma (fy-parse.c:1174)
    fn fetch_flow_entry(&mut self) -> Token<'input> {
        let start = self.reader.mark();
        self.reader.advance();
        let end = self.reader.mark();
        Self::marker_token(TokenKind::FlowEntry, start, end)
    }

    // -- Block indicators --

    /// Fetch a block entry (`- `). May push `BlockSequenceStart` into the queue first.
    // cref: fy_fetch_block_entry (fy-parse.c:2703)
    fn fetch_block_entry(&mut self) {
        let col = self.reader.mark().column as i32;
        self.roll_indent(col, false);
        let start = self.reader.mark();
        self.reader.advance(); // skip '-'
        let end = self.reader.mark();
        self.queue
            .push_back(Self::marker_token(TokenKind::BlockEntry, start, end));
    }

    /// Fetch an explicit key (`? `). May push `BlockMappingStart` into the queue first.
    // cref: fy_fetch_key (fy-parse.c:2818)
    fn fetch_key(&mut self) {
        let col = self.reader.mark().column as i32;
        self.roll_indent(col, true);
        let start = self.reader.mark();
        self.reader.advance(); // skip '?'
        let end = self.reader.mark();
        self.queue
            .push_back(Self::marker_token(TokenKind::Key, start, end));
    }

    /// Fetch a value indicator (`: `).
    // cref: fy_fetch_value (fy-parse.c:2931)
    fn fetch_value(&mut self) {
        let start = self.reader.mark();
        self.reader.advance(); // skip ':'
        let end = self.reader.mark();
        self.queue
            .push_back(Self::marker_token(TokenKind::Value, start, end));
    }

    // -- Plain scalars --

    /// Scan one line of plain scalar text.
    ///
    /// Reads characters until a terminator is reached. Trailing whitespace
    /// is trimmed. The reader is left positioned at the terminator.
    ///
    /// Terminators:
    /// - EOF, newline
    /// - `:` followed by blank/EOF (value indicator)
    /// - `#` preceded by whitespace (comment)
    /// - `,` `[` `]` `{` `}` in flow context
    /// - `---` or `...` at column 0 followed by blank/EOF (document indicators)
    // cref: fy_reader_fetch_plain_scalar_handle (fy-reader.c)
    fn scan_plain_scalar_line(&mut self) -> &'input str {
        let start_offset = self.reader.mark().offset;
        let mut prev_was_space = false;

        loop {
            match self.reader.peek() {
                None | Some('\n' | '\r') => break,
                Some(':') if is_blank_or_end(self.reader.peek_at(1)) => break,
                Some('#') if prev_was_space => break,
                Some(',' | '[' | ']' | '{' | '}') if self.flow_level > 0 => break,
                Some(c) => {
                    prev_was_space = c == ' ' || c == '\t';
                    self.reader.advance();
                }
            }
        }

        let end_offset = self.reader.mark().offset;
        self.reader.slice(start_offset, end_offset).trim_end()
    }

    /// Scan a plain scalar, including multi-line continuation.
    ///
    /// After the first line, continuation lines must be indented past the
    /// current block indent level. Line folding follows YAML 1.2 §7.3.3:
    /// - Single newline between content lines → space
    /// - Empty lines (only whitespace) → literal newline
    // cref: fy_reader_fetch_plain_scalar_handle_inline (fy-parse.c:4434)
    fn scan_plain_scalar_text(&mut self) -> Cow<'input, str> {
        let first_line = self.scan_plain_scalar_line();
        if first_line.is_empty() {
            return Cow::Borrowed(first_line);
        }

        // In flow context, min_indent is 0 (any indentation continues).
        // In block context, continuation lines must be indented past the
        // current block indent level.
        let min_indent = if self.flow_level > 0 {
            0
        } else {
            self.indent + 1
        };

        // Peek ahead to see if next line is a continuation.
        if !self.peek_continuation(min_indent) {
            return Cow::Borrowed(first_line);
        }

        // Multi-line: build folded result.
        let mut result = String::from(first_line);
        let mut empty_lines = 0u32;

        while matches!(self.reader.peek(), Some('\n' | '\r')) {
            self.reader.advance(); // consume newline
            // Skip leading whitespace on next line.
            while is_blank(self.reader.peek()) {
                self.reader.advance();
            }
            let col = self.reader.mark().column as i32;

            // Document indicators at column 0 end the scalar.
            if col == 0 && (self.is_document_start() || self.is_document_end()) {
                break;
            }

            // Another newline = empty line in the scalar.
            if matches!(self.reader.peek(), Some('\n' | '\r')) {
                empty_lines += 1;
                continue;
            }

            // EOF or insufficient indent ends the scalar.
            if self.reader.peek().is_none() || col < min_indent {
                break;
            }

            // Comment after space ends the scalar.
            if self.reader.peek() == Some('#') {
                break;
            }

            // We have a continuation line with content.
            if empty_lines > 0 {
                for _ in 0..empty_lines {
                    result.push('\n');
                }
                empty_lines = 0;
            } else {
                result.push(' ');
            }

            let line = self.scan_plain_scalar_line();
            result.push_str(line);
        }

        Cow::Owned(result)
    }

    /// Peek ahead to check if a continuation line follows.
    ///
    /// Returns true if we're at a newline and the next non-empty line
    /// has indentation greater than `min_indent`. Does NOT advance the reader.
    fn peek_continuation(&self, min_indent: i32) -> bool {
        if !matches!(self.reader.peek(), Some('\n' | '\r')) {
            return false;
        }
        let mut i = if self.reader.peek() == Some('\r') && self.reader.peek_at(1) == Some('\n') {
            2
        } else {
            1usize
        };
        // Scan past whitespace/empty lines to find the next content line.
        let mut col = 0i32;
        loop {
            match self.reader.peek_at(i) {
                Some(' ' | '\t') => {
                    col += 1;
                    i += 1;
                }
                Some('\n' | '\r') => {
                    // Empty line — keep looking for content.
                    i += 1;
                    col = 0;
                }
                Some(c) => {
                    // Document indicators at column 0 end the scalar.
                    if col == 0
                        && (c == '-' || c == '.')
                        && self.reader.peek_at(i + 1) == Some(c)
                        && self.reader.peek_at(i + 2) == Some(c)
                    {
                        return false;
                    }
                    return col >= min_indent;
                }
                None => return false,
            }
        }
    }

    /// Fetch a plain scalar token, resolving simple keys eagerly.
    ///
    /// After scanning the scalar text, checks if `:` + blank follows.
    /// If so, this scalar is a mapping key — inserts `BlockMappingStart`
    /// (if indent warrants) and `Key` before the scalar, then advances
    /// past `:` and pushes `Value`.
    // cref: fy_fetch_plain_scalar (fy-parse.c:5151)
    fn fetch_plain_scalar(&mut self) {
        let start_mark = self.reader.mark();
        let text = self.scan_plain_scalar_text();
        let end_mark = self.reader.mark();

        if text.is_empty() {
            // Nothing scanned (e.g., hit a terminator immediately).
            // Skip the character to avoid infinite loop.
            self.flush_pending_prefix();
            self.reader.advance();
            return;
        }

        // Simple key resolution: if `:` + blank follows, this scalar is a key.
        // Works in both block and flow context.
        let is_key = self.reader.peek() == Some(':') && is_blank_or_end(self.reader.peek_at(1));

        if is_key {
            // Use the position of the first pending prefix token (anchor/tag)
            // as the key start, so BlockMappingStart uses the correct column.
            let key_mark = self
                .pending_prefix
                .first()
                .map_or(start_mark, |t| t.atom.span.start);
            let col = key_mark.column as i32;
            self.roll_indent(col, true); // may push BlockMappingStart
            self.queue
                .push_back(Self::marker_token(TokenKind::Key, key_mark, key_mark));
        }

        // Flush pending anchor/tag tokens before the scalar.
        self.flush_pending_prefix();

        let scalar = Self::scalar_token(text, ScalarStyle::Plain, start_mark, end_mark);
        self.queue.push_back(scalar);

        if is_key {
            let v_start = self.reader.mark();
            self.reader.advance(); // skip ':'
            let v_end = self.reader.mark();
            self.queue
                .push_back(Self::marker_token(TokenKind::Value, v_start, v_end));
        }
    }

    // -- Tags --

    /// Fetch a tag token (`!`, `!!suffix`, `!handle!suffix`, `!<uri>`).
    ///
    /// The full tag text (including `!` prefix) is stored as the token value.
    // cref: fy_fetch_tag (fy-parse.c:3342)
    fn fetch_tag(&mut self) {
        let start_mark = self.reader.mark();
        self.reader.advance(); // skip first '!'

        match self.reader.peek() {
            // Verbatim tag: !<uri>
            Some('<') => {
                self.reader.advance(); // skip '<'
                while let Some(c) = self.reader.peek() {
                    if c == '>' {
                        self.reader.advance();
                        break;
                    }
                    self.reader.advance();
                }
            }
            // Secondary handle !! or named handle !name!
            Some('!') => {
                self.reader.advance(); // skip second '!'
                // Read suffix: continues until whitespace/flow indicator
                while let Some(c) = self.reader.peek() {
                    if c.is_ascii_whitespace() || matches!(c, ',' | '[' | ']' | '{' | '}') {
                        break;
                    }
                    self.reader.advance();
                }
            }
            // Primary handle !suffix or non-specific !
            Some(c) if !c.is_ascii_whitespace() && !matches!(c, ',' | '[' | ']' | '{' | '}') => {
                // Could be !suffix or !name!suffix
                while let Some(c) = self.reader.peek() {
                    if c.is_ascii_whitespace() || matches!(c, ',' | '[' | ']' | '{' | '}') {
                        break;
                    }
                    self.reader.advance();
                }
            }
            // Bare ! (non-specific tag) followed by whitespace/EOF
            _ => {}
        }

        let end_mark = self.reader.mark();
        let tag_text = self.reader.slice(start_mark.offset, end_mark.offset);
        let token = Self::data_token(TokenKind::Tag, tag_text, start_mark, end_mark);
        self.queue.push_back(token);
    }

    // -- Anchors and aliases --

    /// Fetch an anchor (`&name`) or alias (`*name`).
    ///
    /// Reads the indicator and the following name. The name extends
    /// until whitespace, a flow indicator, or EOF.
    // cref: fy_fetch_anchor_or_alias (fy-parse.c:5443-5454)
    fn fetch_anchor_or_alias(&mut self, indicator: char) {
        let kind = if indicator == '&' {
            TokenKind::Anchor
        } else {
            TokenKind::Alias
        };
        self.reader.advance(); // skip & or *
        let name_start = self.reader.mark();

        while let Some(c) = self.reader.peek() {
            if c.is_ascii_whitespace() || matches!(c, ',' | '[' | ']' | '{' | '}' | ':') {
                break;
            }
            self.reader.advance();
        }

        let name_end = self.reader.mark();
        let name = self.reader.slice(name_start.offset, name_end.offset);
        let token = Self::data_token(kind, name, name_start, name_end);
        self.queue.push_back(token);
    }

    // -- Block scalars --

    /// Fetch a literal (`|`) or folded (`>`) block scalar.
    ///
    /// Parses the header (indicator, optional chomp/indent), then reads
    /// content lines respecting indentation. Applies chomp rules and
    /// folding (for `>`) to produce the final scalar value.
    // cref: fy_fetch_block_scalar (fy-parse.c), fy_reader_fetch_block_scalar_handle (fy-reader.c)
    fn fetch_block_scalar(&mut self, indicator: char) {
        let start_mark = self.reader.mark();
        let is_literal = indicator == '|';
        self.reader.advance(); // skip | or >

        // Parse header: optional chomp and/or indent digit (either order).
        let mut chomp = Chomp::Clip;
        let mut explicit_indent: Option<usize> = None;
        for _ in 0..2 {
            match self.reader.peek() {
                Some('+') => {
                    chomp = Chomp::Keep;
                    self.reader.advance();
                }
                Some('-') => {
                    chomp = Chomp::Strip;
                    self.reader.advance();
                }
                Some(c) if c.is_ascii_digit() && c != '0' => {
                    explicit_indent = Some((c as u32 - '0' as u32) as usize);
                    self.reader.advance();
                }
                _ => break,
            }
        }

        // Skip rest of header line (whitespace, comment).
        self.skip_to_next_line();

        // Determine content indentation.
        let content_indent =
            explicit_indent.unwrap_or_else(|| self.detect_block_indent_lookahead());

        // Read content lines using lookahead to avoid consuming non-content lines.
        let mut content = String::new();
        let mut trailing_empty: usize = 0;

        loop {
            if self.reader.is_eof() {
                break;
            }

            // Lookahead: count leading spaces without advancing.
            let mut spaces = 0;
            while self.reader.peek_at(spaces) == Some(' ') {
                spaces += 1;
            }

            match self.reader.peek_at(spaces) {
                // Empty/blank line.
                Some('\n' | '\r') => {
                    trailing_empty += 1;
                    self.reader.advance_by(spaces);
                    self.reader.advance(); // consume newline
                }
                // EOF after spaces only.
                None => break,
                // Content line.
                _ => {
                    if spaces < content_indent {
                        // Under-indented → end of block (don't consume).
                        break;
                    }

                    // Flush pending empty lines into content.
                    for _ in 0..trailing_empty {
                        content.push('\n');
                    }
                    trailing_empty = 0;

                    // Skip indent spaces.
                    self.reader.advance_by(content_indent);

                    // Read line content (may include extra indent for folded).
                    let line_start = self.reader.mark().offset;
                    while !self.reader.is_eof() && !matches!(self.reader.peek(), Some('\n' | '\r'))
                    {
                        self.reader.advance();
                    }
                    let line_end = self.reader.mark().offset;
                    content.push_str(self.reader.slice(line_start, line_end));
                    content.push('\n');

                    // Consume the newline.
                    if !self.reader.is_eof() {
                        self.reader.advance();
                    }
                }
            }
        }

        // Apply folding for `>`.
        if !is_literal {
            content = fold_block_scalar(&content);
        }

        // Apply chomp to trailing newlines.
        match chomp {
            Chomp::Strip => {
                let trimmed_len = content.trim_end_matches('\n').len();
                content.truncate(trimmed_len);
            }
            Chomp::Clip => {
                // Exactly one trailing newline (if any content exists).
                let trimmed_len = content.trim_end_matches('\n').len();
                content.truncate(trimmed_len);
                if !content.is_empty() {
                    content.push('\n');
                }
            }
            Chomp::Keep => {
                for _ in 0..trailing_empty {
                    content.push('\n');
                }
            }
        }

        let end_mark = self.reader.mark();
        let style = if is_literal {
            ScalarStyle::Literal
        } else {
            ScalarStyle::Folded
        };
        let scalar = Self::scalar_token(Cow::Owned(content), style, start_mark, end_mark);
        self.queue.push_back(scalar);
    }

    /// Detect block scalar content indentation by peeking ahead
    /// to the first non-empty line.
    fn detect_block_indent_lookahead(&self) -> usize {
        let mut pos = 0;
        loop {
            let mut spaces = 0;
            while self.reader.peek_at(pos) == Some(' ') {
                spaces += 1;
                pos += 1;
            }
            match self.reader.peek_at(pos) {
                None => return spaces.max(1),
                Some('\n') => {
                    pos += 1;
                }
                Some('\r') => {
                    pos += 1;
                    if self.reader.peek_at(pos) == Some('\n') {
                        pos += 1;
                    }
                }
                _ => return spaces.max(1),
            }
        }
    }

    // -- Quoted scalars --

    /// Fetch a single-quoted scalar (`'...'`).
    ///
    /// The only escape in single-quoted scalars is `''` → `'`.
    // cref: fy_reader_fetch_flow_scalar_handle (fy-reader.c) — single-quoted branch
    fn fetch_single_quoted_scalar(&mut self) {
        let start_mark = self.reader.mark();
        self.reader.advance(); // skip opening '

        let mut result = String::new();
        let mut needs_owned = false;

        loop {
            match self.reader.peek() {
                None => break, // unterminated — accept what we have
                Some('\'') => {
                    self.reader.advance();
                    if self.reader.peek() == Some('\'') {
                        // '' → literal '
                        self.reader.advance();
                        result.push('\'');
                        needs_owned = true;
                    } else {
                        // end of string
                        break;
                    }
                }
                // Newline inside single-quoted scalar: fold per YAML 1.2 §6.5.
                // Single newline → space. Empty lines → literal newlines.
                Some('\n' | '\r') => {
                    needs_owned = true;
                    self.reader.advance();
                    let mut empty_lines = 0u32;
                    loop {
                        while matches!(self.reader.peek(), Some(' ' | '\t')) {
                            self.reader.advance();
                        }
                        if matches!(self.reader.peek(), Some('\n' | '\r')) {
                            empty_lines += 1;
                            self.reader.advance();
                        } else {
                            break;
                        }
                    }
                    if empty_lines > 0 {
                        for _ in 0..empty_lines {
                            result.push('\n');
                        }
                    } else {
                        result.push(' ');
                    }
                }
                Some(c) => {
                    result.push(c);
                    self.reader.advance();
                }
            }
        }

        let end_mark = self.reader.mark();
        let data = if needs_owned {
            Cow::Owned(result)
        } else {
            // No escapes — borrow from input (content between quotes).
            let content_start = start_mark.offset + 1; // after opening '
            let content_end = end_mark.offset.saturating_sub(1); // before closing '
            if content_start <= content_end {
                Cow::Borrowed(self.reader.slice(content_start, content_end))
            } else {
                Cow::Borrowed("")
            }
        };

        self.push_quoted_scalar(data, ScalarStyle::SingleQuoted, start_mark, end_mark);
    }

    /// Fetch a double-quoted scalar (`"..."`), processing escape sequences.
    // cref: fy_reader_fetch_flow_scalar_handle (fy-reader.c) — double-quoted branch
    fn fetch_double_quoted_scalar(&mut self) {
        let start_mark = self.reader.mark();
        self.reader.advance(); // skip opening "

        let mut result = String::new();
        let mut needs_owned = false;

        loop {
            match self.reader.peek() {
                None => break, // unterminated
                Some('"') => {
                    self.reader.advance();
                    break;
                }
                Some('\\') => {
                    needs_owned = true;
                    self.reader.advance(); // skip backslash
                    match self.reader.peek() {
                        Some('\\') => {
                            result.push('\\');
                            self.reader.advance();
                        }
                        Some('"') => {
                            result.push('"');
                            self.reader.advance();
                        }
                        Some('n') => {
                            result.push('\n');
                            self.reader.advance();
                        }
                        Some('t') => {
                            result.push('\t');
                            self.reader.advance();
                        }
                        Some('r') => {
                            result.push('\r');
                            self.reader.advance();
                        }
                        Some('0') => {
                            result.push('\0');
                            self.reader.advance();
                        }
                        Some('a') => {
                            result.push('\x07');
                            self.reader.advance();
                        }
                        Some('b') => {
                            result.push('\x08');
                            self.reader.advance();
                        }
                        Some('e') => {
                            result.push('\x1B');
                            self.reader.advance();
                        }
                        Some('f') => {
                            result.push('\x0C');
                            self.reader.advance();
                        }
                        Some('v') => {
                            result.push('\x0B');
                            self.reader.advance();
                        }
                        Some('/') => {
                            result.push('/');
                            self.reader.advance();
                        }
                        Some(' ') => {
                            result.push(' ');
                            self.reader.advance();
                        }
                        Some('_') => {
                            result.push('\u{A0}');
                            self.reader.advance();
                        }
                        Some('N') => {
                            result.push('\u{85}');
                            self.reader.advance();
                        }
                        Some('L') => {
                            result.push('\u{2028}');
                            self.reader.advance();
                        }
                        Some('P') => {
                            result.push('\u{2029}');
                            self.reader.advance();
                        }
                        Some('x') => {
                            self.reader.advance();
                            if let Some(ch) = self.scan_unicode_escape(2) {
                                result.push(ch);
                            }
                        }
                        Some('u') => {
                            self.reader.advance();
                            if let Some(ch) = self.scan_unicode_escape(4) {
                                result.push(ch);
                            }
                        }
                        Some('U') => {
                            self.reader.advance();
                            if let Some(ch) = self.scan_unicode_escape(8) {
                                result.push(ch);
                            }
                        }
                        // Escaped line break: \ + newline joins lines,
                        // consuming the break and any leading whitespace.
                        // cref: YAML 1.2 §8.5 — "escaped line break"
                        Some('\n' | '\r') => {
                            self.reader.advance(); // consume newline
                            while matches!(self.reader.peek(), Some(' ' | '\t')) {
                                self.reader.advance();
                            }
                            // No character pushed — lines are joined seamlessly.
                        }
                        Some(c) => {
                            // Unknown escape — keep as-is.
                            result.push('\\');
                            result.push(c);
                            self.reader.advance();
                        }
                        None => {
                            result.push('\\');
                        }
                    }
                }
                // Newline inside double-quoted scalar: fold per YAML 1.2 §6.5.
                // Single newline between content → space.
                // Empty lines (newline-only) → literal newline.
                // Leading whitespace on continuation is trimmed.
                Some('\n' | '\r') => {
                    needs_owned = true;
                    self.reader.advance(); // consume newline
                    // Count empty lines.
                    let mut empty_lines = 0u32;
                    loop {
                        while matches!(self.reader.peek(), Some(' ' | '\t')) {
                            self.reader.advance();
                        }
                        if matches!(self.reader.peek(), Some('\n' | '\r')) {
                            empty_lines += 1;
                            self.reader.advance();
                        } else {
                            break;
                        }
                    }
                    if empty_lines > 0 {
                        for _ in 0..empty_lines {
                            result.push('\n');
                        }
                    } else {
                        result.push(' ');
                    }
                }
                Some(c) => {
                    result.push(c);
                    self.reader.advance();
                }
            }
        }

        let end_mark = self.reader.mark();
        // Multi-line always needs owned due to folding.
        let data = if needs_owned {
            Cow::Owned(result)
        } else {
            let content_start = start_mark.offset + 1;
            let content_end = end_mark.offset.saturating_sub(1);
            if content_start <= content_end {
                Cow::Borrowed(self.reader.slice(content_start, content_end))
            } else {
                Cow::Borrowed("")
            }
        };

        self.push_quoted_scalar(data, ScalarStyle::DoubleQuoted, start_mark, end_mark);
    }

    /// Read `n` hex digits and return the corresponding Unicode character.
    fn scan_unicode_escape(&mut self, n: usize) -> Option<char> {
        let mut code: u32 = 0;
        for _ in 0..n {
            let c = self.reader.peek()?;
            let digit = c.to_digit(16)?;
            code = code * 16 + digit;
            self.reader.advance();
        }
        char::from_u32(code)
    }

    /// Push a quoted scalar to the queue with simple key resolution.
    fn push_quoted_scalar(
        &mut self,
        data: Cow<'input, str>,
        style: ScalarStyle,
        start_mark: yamalgam_core::Mark,
        end_mark: yamalgam_core::Mark,
    ) {
        // Simple key resolution: if `:` + blank follows, this is a key.
        // Works in both block and flow context.
        let is_key = self.reader.peek() == Some(':') && is_blank_or_end(self.reader.peek_at(1));

        if is_key {
            let key_mark = self
                .pending_prefix
                .first()
                .map_or(start_mark, |t| t.atom.span.start);
            let col = key_mark.column as i32;
            self.roll_indent(col, true);
            self.queue
                .push_back(Self::marker_token(TokenKind::Key, key_mark, key_mark));
        }

        // Flush pending anchor/tag tokens before the scalar.
        self.flush_pending_prefix();

        let scalar = Self::scalar_token(data, style, start_mark, end_mark);
        self.queue.push_back(scalar);

        if is_key {
            let v_start = self.reader.mark();
            self.reader.advance(); // skip ':'
            let v_end = self.reader.mark();
            self.queue
                .push_back(Self::marker_token(TokenKind::Value, v_start, v_end));
        }
    }

    // -- Helpers --

    /// Check if the next characters match `prefix`.
    fn check_prefix(&self, prefix: &str) -> bool {
        prefix
            .chars()
            .enumerate()
            .all(|(i, expected)| self.reader.peek_at(i) == Some(expected))
    }

    /// Move pending anchor/tag tokens to the main queue.
    ///
    /// Called when we determine the pending tokens are NOT part of a
    /// simple key (e.g., followed by a non-scalar, newline, etc.).
    fn flush_pending_prefix(&mut self) {
        for token in self.pending_prefix.drain(..) {
            self.queue.push_back(token);
        }
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

    // -- Main fetch loop --

    /// Fetch the next token from the stream.
    ///
    /// Skips whitespace/comments, manages indent levels, then checks for
    /// document indicators, directives, flow indicators, block indicators,
    /// and EOF. Unrecognized content is skipped.
    // cref: fy_fetch_tokens (fy-parse.c:5250)
    fn fetch_next_token(&mut self) -> Option<Result<Token<'input>, ScanError>> {
        loop {
            // Drain queued tokens first.
            if let Some(token) = self.queue.pop_front() {
                return Some(Ok(token));
            }

            self.scan_to_next_token();

            if self.reader.is_eof() {
                self.flush_pending_prefix();
                self.unroll_indent(-1);
                let end = self.fetch_stream_end();
                self.queue.push_back(end);
                continue;
            }

            let c = self.reader.peek().unwrap();
            let col = self.reader.mark().column;

            // In block mode, unroll indent to current column.
            if self.flow_level == 0 {
                self.unroll_indent(col as i32);
                if !self.queue.is_empty() {
                    continue;
                }
            }

            // Flush pending anchor/tag tokens if they can't be part of a
            // simple key: either we've crossed a newline or the next character
            // is structural (not a scalar-starting character or another tag/anchor).
            if !self.pending_prefix.is_empty() {
                let prefix_line = self.pending_prefix[0].atom.span.start.line;
                let current_line = self.reader.mark().line;
                let is_structural = matches!(c, '[' | ']' | '{' | '}' | ',' | ':' | '%' | '#')
                    || (c == '-' && is_blank_or_end(self.reader.peek_at(1)))
                    || (c == '?' && is_blank_or_end(self.reader.peek_at(1)));

                if current_line != prefix_line || is_structural {
                    self.flush_pending_prefix();
                }
            }

            // Document indicators at column 0 (unroll fully first).
            if col == 0 {
                if self.is_document_start() {
                    self.unroll_indent(-1);
                    let token = self.fetch_document_indicator(TokenKind::DocumentStart);
                    self.queue.push_back(token);
                    continue;
                }
                if self.is_document_end() {
                    self.unroll_indent(-1);
                    let token = self.fetch_document_indicator(TokenKind::DocumentEnd);
                    self.queue.push_back(token);
                    continue;
                }
                if c == '%' {
                    match self.fetch_directive() {
                        Ok(token) => {
                            self.queue.push_back(token);
                            continue;
                        }
                        Err(e) => {
                            // YAML 1.2 §6.8.1: unknown directives are ignored.
                            // cref: fy_fetch_directive — libfyaml skips with a warning
                            if e.message == "unknown directive" {
                                continue;
                            }
                            return Some(Err(e));
                        }
                    }
                }
            }

            // Flow collection indicators.
            // cref: fy_fetch_tokens (fy-parse.c:5364-5394)
            if c == '[' || c == '{' {
                let token = self.fetch_flow_collection_start(c);
                self.queue.push_back(token);
                continue;
            }
            if c == ']' || c == '}' {
                let token = self.fetch_flow_collection_end(c);
                self.queue.push_back(token);
                continue;
            }
            if c == ',' {
                let token = self.fetch_flow_entry();
                self.queue.push_back(token);
                continue;
            }

            // Block indicators (not in flow context).
            // cref: fy_fetch_tokens (fy-parse.c:5396-5441)
            if self.flow_level == 0 {
                if c == '-' && is_blank_or_end(self.reader.peek_at(1)) {
                    self.fetch_block_entry();
                    continue;
                }
                if c == '?' && is_blank_or_end(self.reader.peek_at(1)) {
                    self.fetch_key();
                    continue;
                }
            }

            // Value indicator `:` — in both block and flow context.
            // cref: fy_fetch_tokens (fy-parse.c:5426)
            if c == ':' && is_blank_or_end(self.reader.peek_at(1)) {
                self.fetch_value();
                continue;
            }

            // Tags.
            // cref: fy_fetch_tokens (fy-parse.c:5457)
            if c == '!' {
                self.fetch_tag();
                // Buffer tag as potential simple key prefix.
                if let Some(tag_token) = self.queue.pop_back() {
                    self.pending_prefix.push(tag_token);
                }
                continue;
            }

            // Anchors and aliases.
            // cref: fy_fetch_tokens (fy-parse.c:5443)
            if c == '&' || c == '*' {
                self.fetch_anchor_or_alias(c);
                // Buffer anchors (not aliases) as potential simple key prefix.
                if c == '&'
                    && let Some(anchor_token) = self.queue.pop_back()
                {
                    self.pending_prefix.push(anchor_token);
                }
                continue;
            }

            // Block scalars (not in flow context).
            if self.flow_level == 0 && (c == '|' || c == '>') {
                self.fetch_block_scalar(c);
                continue;
            }

            // Quoted scalars.
            if c == '\'' {
                self.fetch_single_quoted_scalar();
                continue;
            }
            if c == '"' {
                self.fetch_double_quoted_scalar();
                continue;
            }

            // Everything else is a plain scalar (or content we don't
            // handle yet, like anchors, tags, block scalars).
            // cref: fy_fetch_plain_scalar (fy-parse.c:5496)
            self.fetch_plain_scalar();
        }
    }
}

/// Returns `true` if the character is a blank (space or tab) or absent (EOF/newline).
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

/// Fold a block scalar: replace single newlines between same-indent
/// lines with spaces, preserving newlines around more-indented or empty lines.
// cref: fy_atom_format_text_block (fy-atom.c) — folded mode
fn fold_block_scalar(content: &str) -> String {
    let lines: Vec<&str> = content.split('\n').collect();
    let mut result = String::with_capacity(content.len());
    // Last element after split is "" if content ends with \n.
    let line_count = if lines.last() == Some(&"") {
        lines.len() - 1
    } else {
        lines.len()
    };

    for i in 0..line_count {
        let line = lines[i];
        if i > 0 {
            let prev = lines[i - 1];
            // Preserve newline if either line is empty or more-indented.
            if prev.is_empty() || line.is_empty() || prev.starts_with(' ') || line.starts_with(' ')
            {
                result.push('\n');
            } else {
                result.push(' ');
            }
        }
        result.push_str(line);
    }

    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

impl<'input> Iterator for Scanner<'input> {
    type Item = Result<Token<'input>, ScanError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Drain queued tokens first.
        if let Some(token) = self.queue.pop_front() {
            return Some(Ok(token));
        }
        match self.state {
            State::Start => Some(Ok(self.fetch_stream_start())),
            State::Stream => self.fetch_next_token(),
            State::Done => None,
        }
    }
}

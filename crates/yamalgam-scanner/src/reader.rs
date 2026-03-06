//! Character-level reader with position tracking and lookahead.
//!
//! The `Reader` provides the scanner with a uniform view over decoded UTF-8
//! input: single-character peek/advance, arbitrary lookahead, and byte-accurate
//! position tracking with line/column information.

use yamalgam_core::Mark;

/// Character-level reader over UTF-8 input with position tracking.
///
/// Tracks byte offset, line number, and column (character count on the current
/// line). `\r\n` sequences are consumed as a single newline; bare `\r` also
/// counts as a newline (per the YAML spec).
pub struct Reader<'input> {
    input: &'input str,
    /// Current byte offset into `input`.
    offset: usize,
    /// Current zero-indexed line number.
    line: u32,
    /// Current zero-indexed column (character count on this line).
    column: u32,
}

impl<'input> Reader<'input> {
    /// Create a new reader over the given UTF-8 string.
    #[must_use]
    pub const fn new(input: &'input str) -> Self {
        Self {
            input,
            offset: 0,
            line: 0,
            column: 0,
        }
    }

    /// Peek at the current character without advancing.
    ///
    /// Returns `None` at EOF.
    #[must_use]
    pub fn peek(&self) -> Option<char> {
        self.input[self.offset..].chars().next()
    }

    /// Peek at the character `n` positions ahead (0-indexed).
    ///
    /// `peek_at(0)` is equivalent to `peek()`. Returns `None` if the lookahead
    /// extends past the end of input.
    #[must_use]
    pub fn peek_at(&self, n: usize) -> Option<char> {
        self.input[self.offset..].chars().nth(n)
    }

    /// Consume the current character and advance the position.
    ///
    /// Handles `\r\n` as a single newline: when `\r` is encountered and followed
    /// by `\n`, both bytes are consumed in one call and the returned character is
    /// `\r`. Bare `\r` (not followed by `\n`) is also treated as a newline.
    ///
    /// Returns `None` at EOF.
    pub fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.offset += ch.len_utf8();

        if ch == '\r' {
            // Check for \r\n — consume the \n as well.
            if self.input[self.offset..].starts_with('\n') {
                self.offset += 1; // consume \n
            }
            self.line += 1;
            self.column = 0;
        } else if ch == '\n' {
            self.line += 1;
            self.column = 0;
        } else {
            self.column += 1;
        }

        Some(ch)
    }

    /// Return the current position as a [`Mark`].
    #[must_use]
    pub const fn mark(&self) -> Mark {
        Mark {
            line: self.line,
            column: self.column,
            offset: self.offset,
        }
    }

    /// Advance the reader by `n` characters.
    ///
    /// Equivalent to calling [`advance()`](Self::advance) `n` times.
    pub fn advance_by(&mut self, n: usize) {
        for _ in 0..n {
            if self.advance().is_none() {
                break;
            }
        }
    }

    /// Borrow a substring from `start_offset` to `end_offset` (byte offsets).
    ///
    /// # Panics
    ///
    /// Panics if the offsets are out of bounds or not on UTF-8 boundaries.
    #[must_use]
    pub fn slice(&self, start_offset: usize, end_offset: usize) -> &'input str {
        &self.input[start_offset..end_offset]
    }

    /// Returns `true` if the reader has reached the end of input.
    #[must_use]
    pub const fn is_eof(&self) -> bool {
        self.offset >= self.input.len()
    }
}

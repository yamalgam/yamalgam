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

use yamalgam_core::{ResourceLimits, Span};

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

/// A potential simple key saved on the simple key stack.
///
/// When the scanner encounters a scalar, anchor, alias, or tag, it might
/// be a mapping key — but we can't know until we see `:`. This struct
/// records where the potential key started so we can insert
/// `BlockMappingStart` + `Key` retroactively when `:` is found.
// cref: fy_simple_key (fy-parse.h:73)
#[derive(Debug, Clone)]
struct SimpleKey {
    /// Position where the potential key token starts.
    mark: yamalgam_core::Mark,
    /// Line where the potential key token ends.
    /// Used for staleness checks on multiline tokens.
    end_line: u32,
    /// Monotonic ID of the token in the queue (for locating it later).
    token_id: u64,
    /// Flow nesting level when this key was saved.
    flow_level: u32,
    /// If true, this key MUST be resolved (error if purged).
    /// Set when we're at the current block indent level.
    /// Used for error reporting on anchors/tags at wrong indent.
    required: bool,
    /// True when this key was a JSON-like node (quoted scalar or flow
    /// collection end). In flow context, `:` after a JSON-like key is
    /// a value indicator regardless of what follows and can appear on
    /// a subsequent line (YAML §7.4.2, production [153]).
    json_key: bool,
}

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
    /// Stack of potential simple keys (one per flow level).
    // cref: fy_simple_key list in fy_parser
    simple_keys: Vec<SimpleKey>,
    /// Whether a simple key is allowed at the current position.
    // cref: fy_parser.simple_key_allowed
    simple_key_allowed: bool,
    /// Column of a pending explicit key (`?`), or -1 if none.
    /// When `:` is found at or before this column, it's the value
    /// for the explicit key — don't emit another `Key`.
    // cref: fy_parser.pending_complex_key_column
    pending_complex_key_column: i32,
    /// Reader offset where `:` would be "adjacent" to a JSON-like
    /// construct (quoted scalar / flow collection end). Only valid at
    /// exactly this offset — any whitespace consumed invalidates it.
    // cref: fy_parse.c — flow_scalar adjacent value handling
    adjacent_value_offset: Option<usize>,
    /// Monotonic counter — each token pushed to the queue gets a unique ID.
    next_token_id: u64,
    /// Number of tokens consumed (popped) from the queue.
    tokens_consumed: u64,
    /// Deferred error from a void fetch method (e.g., invalid escape).
    /// Checked and drained by the dispatch loop.
    error: Option<ScanError>,
    /// Resource limits (depth caps, size caps, etc.).
    config: ResourceLimits,
    /// Line of the most recent context token (Value, Anchor, Tag).
    /// Used to reject block entries on the same line as a preceding
    /// context token (e.g., `key: - a`, `&anchor - entry`).
    last_block_token_line: Option<u32>,
    /// Block indent at the point where flow context was entered.
    /// Used to reject flow content at or below the block indent.
    /// -1 when no flow context is active or flow started at doc level.
    flow_indent: i32,
    /// Stack of flow context types: `true` = mapping (`{`), `false` = sequence (`[`).
    /// Used to distinguish flow mappings from sequences when validating
    /// multiline implicit keys (`:` in `[...]` vs `{...}`).
    // cref: fy_parser.flow (FYFT_MAP vs FYFT_SEQ)
    flow_is_mapping: Vec<bool>,
    /// Whether we're in the directive prologue (stream start or after `...`).
    /// Directives (`%YAML`, `%TAG`) are only valid here.
    in_directive_prologue: bool,
    /// Whether any directive was seen in the current prologue (for 9MMA check).
    seen_directive: bool,
    /// Whether `%YAML` was seen in the current prologue (for SF5V duplicate check).
    seen_yaml_directive: bool,
    /// Line of most recent `---` for same-line block collection rejection (9KBC, CXX2).
    document_start_line: Option<u32>,
    /// Root-level content already complete — standalone scalar or flow collection
    /// at indent -1. Used to reject extra root content (BS4K, KS4U).
    root_token_emitted: bool,
    /// Named tag handles registered via `%TAG` in the current directive prologue.
    /// Cleared on `...` and on `---` without a preceding prologue.
    tag_handles: Vec<String>,
    /// Whether a tab was consumed in the preceding whitespace by
    /// `scan_to_next_token()`. Used to reject tabs as indentation
    /// before block indicators (`-`, `?`, `:`).
    tab_in_preceding_whitespace: bool,
}

impl<'input> Scanner<'input> {
    /// Create a new scanner over decoded UTF-8 input.
    ///
    /// Uses [`ResourceLimits::none()`] — no depth or size limits.
    #[must_use]
    pub const fn new(input: &'input str) -> Self {
        Self {
            reader: Reader::new(input),
            state: State::Start,
            flow_level: 0,
            indent: -1,
            indent_stack: Vec::new(),
            queue: VecDeque::new(),
            simple_keys: Vec::new(),
            simple_key_allowed: true,
            pending_complex_key_column: -1,
            error: None,
            config: ResourceLimits::none(),
            adjacent_value_offset: None,
            last_block_token_line: None,
            flow_indent: -1,
            flow_is_mapping: Vec::new(),
            in_directive_prologue: true,
            seen_directive: false,
            seen_yaml_directive: false,
            document_start_line: None,
            root_token_emitted: false,
            tag_handles: Vec::new(),
            tab_in_preceding_whitespace: false,
            next_token_id: 0,
            tokens_consumed: 0,
        }
    }

    /// Create a new scanner with resource limits from a [`LoaderConfig`].
    ///
    /// The scanner stores only the [`ResourceLimits`] sub-struct (it doesn't
    /// need resolution policy). Use this when processing untrusted input.
    ///
    /// [`LoaderConfig`]: yamalgam_core::LoaderConfig
    #[must_use]
    pub fn with_config(input: &'input str, config: &yamalgam_core::LoaderConfig) -> Self {
        let mut scanner = Self::new(input);
        scanner.config = config.limits.clone();
        scanner
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
    ///
    /// In block context, crossing a newline re-enables simple keys.
    // cref: fy_scan_to_next_token (fy-parse.c:1260)
    // y[impl struct.s-separate-in-line]
    // y[impl struct.comment.not-content]
    // y[impl struct.comment.separated-by-whitespace]
    // y[impl struct.c-nb-comment-text]
    // y[impl struct.b-comment]
    // y[impl struct.s-b-comment]
    // y[impl struct.l-comment]
    // y[impl struct.s-l-comments]
    // y[impl char.s-white]
    // y[impl char.s-space]
    // y[impl char.s-tab]
    // y[impl char.c-comment] — # starts comment when preceded by whitespace
    // y[impl char.nb-char] — non-break chars consumed in comment text
    // y[impl char.ns-char] — non-space chars consumed in comment text
    // y[impl struct.indent.tab-forbidden] — tabs rejected as indentation in block context
    // y[impl struct.comment.should-terminate-with-break] — comments end at line break
    // y[impl struct.separation.not-content] — whitespace/comments between tokens discarded
    // y[impl struct.s-separate] — separation between tokens
    // y[impl struct.s-separate-lines] — separation including line breaks
    // y[impl struct.separation.indented-after-comments] — content after comments must be indented
    // y[impl struct.comment.json-compat-final-break] — final line may end at EOF without break
    fn scan_to_next_token(&mut self) {
        self.tab_in_preceding_whitespace = false;
        let mut at_line_start = self.reader.mark().column == 0;
        let mut tab_at_flow_line_start = false;
        loop {
            while let Some(c) = self.reader.peek() {
                if c == ' ' {
                    self.reader.advance();
                } else if c == '\t' {
                    // YAML §6.1: tabs must not be used for indentation.
                    // In block context, tabs at the start of a line (before
                    // any non-whitespace) are indentation — reject them.
                    // Only when there's an active block structure (indent >= 0),
                    // so tabs before the first block token (like `\t[`) are fine.
                    // cref: fy_scan_to_next_token (fy-parse.c)
                    // y[impl struct.indent.tab-forbidden]
                    if at_line_start && self.flow_level == 0 && self.indent >= 0 {
                        self.error = Some(ScanError {
                            message: "tab character used for indentation".to_string(),
                        });
                        return;
                    }
                    if at_line_start && self.flow_level > 0 {
                        tab_at_flow_line_start = true;
                    }
                    self.tab_in_preceding_whitespace = true;
                    self.reader.advance();
                } else {
                    break;
                }
            }

            // Y79Y#3: tabs at line start in flow context with content
            // on the same line are indentation — reject them. Tabs on
            // blank lines (Y79Y#2) are fine (next char is newline/EOF).
            if tab_at_flow_line_start && !matches!(self.reader.peek(), Some('\n' | '\r') | None) {
                self.error = Some(ScanError {
                    message: "tab character used for indentation in flow context".to_string(),
                });
                return;
            }

            // Content found — no longer at line start for tab checks.
            // (Set before comment/newline check; overwritten by newline branch.)
            #[allow(unused_assignments)]
            {
                at_line_start = false;
            }

            // In flow context within a block structure, content lines must
            // be indented past the block indent where flow was entered.
            // e.g., `flow: [a,\nb]` — `b` at column 0 <= indent 0 → error.
            // cref: fy_scan_to_next_token (fy-parse.c)
            if self.flow_level > 0
                && self.flow_indent >= 0
                && !matches!(self.reader.peek(), Some('\n' | '\r') | None)
                && (self.reader.mark().column as i32) <= self.flow_indent
            {
                self.error = Some(ScanError {
                    message: "flow content indented at or below block indent".to_string(),
                });
                return;
            }

            // `#` starts a comment only when preceded by whitespace or at
            // the start of a line (YAML §6.4). Check the byte before the
            // current offset — this works regardless of who consumed the
            // whitespace (this function, plain scalar trimming, etc.).
            if self.reader.peek() == Some('#') && self.preceded_by_whitespace() {
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
                    at_line_start = true;
                    // Tabs on blank lines are fine — clear the flag when
                    // a newline is consumed so they don't carry over.
                    self.tab_in_preceding_whitespace = false;
                    tab_at_flow_line_start = false;
                    // In block context, a newline allows a new simple key.
                    // cref: fy_scan_to_next_token (fy-parse.c:1312)
                    if self.flow_level == 0 {
                        self.simple_key_allowed = true;
                    }
                }
                _ => break,
            }
        }
    }

    // -- Indent management --

    // y[impl struct.indent.node-deeper-than-parent]
    // y[impl struct.indent.not-content]
    // y[impl struct.indent.siblings-same-level]
    // y[impl struct.s-indent]
    // y[impl struct.s-indent-less-or-equal]
    // y[impl struct.s-indent-less-than]
    // y[impl block.indent.emit-explicit]
    // y[impl block.indent.leading-empty-error]
    // y[impl block.indent.non-empty-line-error]
    /// Push the current indent level and emit a block collection start token
    /// if the column is deeper than the current indent.
    ///
    /// Returns `true` if a new indent level was pushed.
    // cref: fy_push_indent (fy-parse.c) + BLOCK_SEQUENCE_START/BLOCK_MAPPING_START emit
    fn roll_indent(&mut self, column: i32, is_mapping: bool) {
        if self.flow_level > 0 || column <= self.indent {
            return;
        }
        // 9KBC, CXX2: block collections cannot start on the `---` line.
        // YAML §7.3.2 requires s-l-comments (newline) before block collection entries.
        let mark = self.reader.mark();
        if self.document_start_line == Some(mark.line) {
            self.error = Some(ScanError {
                message: "block collection cannot start on the document start line".to_string(),
            });
            return;
        }
        // Entering block context from root clears root_token_emitted
        // since the root IS this block collection (not yet complete).
        if self.indent == -1 {
            self.root_token_emitted = false;
        }
        self.indent_stack.push(self.indent);
        // Enforce max_depth on combined block + flow nesting.
        if let Err(msg) = self
            .config
            .check_depth(self.indent_stack.len() + self.flow_level as usize)
        {
            self.error = Some(ScanError { message: msg });
            return;
        }
        self.indent = column;
        let kind = if is_mapping {
            TokenKind::BlockMappingStart
        } else {
            TokenKind::BlockSequenceStart
        };
        self.enqueue(Self::marker_token(kind, mark, mark));
    }

    /// Emit `BlockEnd` tokens for each indent level deeper than `column`.
    // cref: fy_parse_unroll_indent (fy-parse.c:1592)
    // y[impl block.s-l-block-collection]
    fn unroll_indent(&mut self, column: i32) {
        if self.flow_level > 0 {
            return;
        }
        while self.indent > column {
            let mark = self.reader.mark();
            self.enqueue(Self::marker_token(TokenKind::BlockEnd, mark, mark));
            self.indent = self.indent_stack.pop().unwrap_or(-1);
        }
    }

    // -- Simple key management --

    /// Push a token to the queue and return its monotonic ID.
    fn enqueue(&mut self, token: Token<'input>) -> u64 {
        let id = self.next_token_id;
        self.next_token_id += 1;
        self.queue.push_back(token);
        id
    }

    /// Save a potential simple key at the current position.
    ///
    /// Called after pushing a token that might turn out to be a mapping key
    /// (scalars, anchors, aliases, tags). The token's queue ID is recorded
    /// so we can insert `Key` (and possibly `BlockMappingStart`) before it
    /// when `:` is encountered later.
    // cref: fy_save_simple_key (fy-parse.c:1698)
    fn save_simple_key(&mut self, token_id: u64, mark: yamalgam_core::Mark) {
        self.save_simple_key_full(token_id, mark, mark.line, false);
    }

    /// Save a simple key with explicit JSON-like flag.
    ///
    /// `json_key` should be `true` for quoted scalars and flow collection
    /// starts — nodes where `:` is a value indicator regardless of what
    /// follows (YAML §7.4.2, production [153]).
    fn save_simple_key_ext(&mut self, token_id: u64, mark: yamalgam_core::Mark, json_key: bool) {
        let end_line = self.reader.mark().line;
        self.save_simple_key_full(token_id, mark, end_line, json_key);
    }

    // y[impl flow.implicit-key.must-single-line]
    fn save_simple_key_full(
        &mut self,
        token_id: u64,
        mark: yamalgam_core::Mark,
        end_line: u32,
        json_key: bool,
    ) {
        if !self.simple_key_allowed {
            return;
        }

        self.purge_stale_simple_keys();

        // A key is "required" if we're in block context at the current indent.
        let required = self.flow_level == 0 && self.indent == mark.column as i32;

        let sk = SimpleKey {
            mark,
            end_line,
            token_id,
            flow_level: self.flow_level,
            required,
            json_key,
        };

        // Replace any existing simple key at the same flow level.
        if let Some(existing) = self
            .simple_keys
            .last()
            .filter(|s| s.flow_level == self.flow_level)
        {
            // If the existing key was required and we're replacing it, that's
            // fine — the new key supersedes it at the same position.
            let _ = existing;
            self.simple_keys.pop();
        }

        self.simple_keys.push(sk);
    }

    /// Remove stale simple keys that can no longer be valid.
    ///
    /// In block context, a simple key is stale if we've moved past its line.
    /// In flow context, a simple key is stale if we've exited its flow level.
    // cref: fy_purge_stale_simple_keys (fy-parse.c:1470)
    fn purge_stale_simple_keys(&mut self) {
        let current_line = self.reader.mark().line;

        // cref: fy_purge_required_simple_key_report (fy-parse.c:1429)
        // When a required simple key (at the current indent level) is about
        // to be purged and its token is an anchor or tag, that means the
        // anchor/tag appeared at the wrong indent for the block structure.
        for sk in &self.simple_keys {
            if !sk.required {
                continue;
            }
            let would_purge = if self.flow_level == 0 {
                current_line > sk.end_line
            } else {
                sk.flow_level > self.flow_level
            };
            if would_purge && let Some(pos) = self.queue_position(sk.token_id) {
                let kind = self.queue[pos].kind;
                if matches!(kind, TokenKind::Anchor | TokenKind::Tag) {
                    let kind_name = if kind == TokenKind::Anchor {
                        "anchor"
                    } else {
                        "tag"
                    };
                    self.error = Some(ScanError {
                        message: format!("invalid {kind_name} indent"),
                    });
                    return;
                }
            }
        }

        // BS4K: collect token IDs of root-level simple keys being purged.
        // After retain, check if any was a Scalar or Alias — those are
        // standalone root values. Tags and anchors are node properties
        // that attach to the following node, so they don't count.
        let indent = self.indent;
        let flow_level = self.flow_level;
        let mut purged_root_ids: Vec<u64> = Vec::new();
        self.simple_keys.retain(|sk| {
            if flow_level == 0 {
                // Block context: keys must end on the current line.
                // Use `end_line` so multiline tokens (quoted scalars,
                // multiline plain scalars) survive until `:` on their
                // last line triggers fetch_value's multiline check.
                let keep = current_line <= sk.end_line;
                if !keep && !sk.required && sk.flow_level == 0 && indent == -1 {
                    purged_root_ids.push(sk.token_id);
                }
                keep
            } else {
                // Flow context: keys must be at <= current flow level.
                sk.flow_level <= flow_level
            }
        });
        for id in purged_root_ids {
            if let Some(pos) = self.queue_position(id) {
                let kind = self.queue[pos].kind;
                if matches!(kind, TokenKind::Scalar | TokenKind::Alias) {
                    self.root_token_emitted = true;
                }
            }
        }
    }

    /// Remove all simple keys at or above the current flow level.
    // cref: fy_remove_simple_key (fy-parse.c:1652)
    // y[impl block.implicit-key-restrictions]
    fn remove_simple_key(&mut self) {
        self.simple_keys
            .retain(|sk| sk.flow_level < self.flow_level);
    }

    /// Find the VecDeque index for a token with the given monotonic ID.
    fn queue_position(&self, token_id: u64) -> Option<usize> {
        if token_id < self.tokens_consumed {
            return None; // Already consumed.
        }
        let pos = (token_id - self.tokens_consumed) as usize;
        if pos < self.queue.len() {
            Some(pos)
        } else {
            None
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
    ///
    /// Manages document-state tracking:
    /// - `---`: enters document, resets directive state, records line for
    ///   same-line block collection rejection (9KBC, CXX2).
    /// - `...`: returns to directive prologue, clears tag handles.
    // y[impl doc.c-directives-end+3]
    // y[impl doc.c-document-end+3]
    // cref: fy_fetch_document_indicator (fy-parse.c:2379)
    fn fetch_document_indicator(&mut self, kind: TokenKind) -> Token<'input> {
        let start = self.reader.mark();
        self.reader.advance_by(3);
        let end = self.reader.mark();

        match kind {
            TokenKind::DocumentStart => {
                // Clear tag handles only if there was no directive prologue
                // (e.g., second `---` without intervening `%TAG`).
                if !self.in_directive_prologue {
                    self.tag_handles.clear();
                }
                self.in_directive_prologue = false;
                self.seen_directive = false;
                self.seen_yaml_directive = false;
                self.document_start_line = Some(start.line);
                self.root_token_emitted = false;
            }
            TokenKind::DocumentEnd => {
                self.in_directive_prologue = true;
                self.seen_directive = false;
                self.seen_yaml_directive = false;
                self.document_start_line = None;
                self.root_token_emitted = false;
                self.tag_handles.clear();
            }
            _ => {}
        }

        Self::marker_token(kind, start, end)
    }

    // -- Directives --

    // y[impl char.c-directive]
    // y[impl struct.l-directive]
    // y[impl struct.ns-directive-name]
    // y[impl struct.ns-directive-parameter]
    // y[impl struct.ns-reserved-directive]
    // y[impl struct.directive.not-content]
    // y[impl struct.directive.ignore-unknown+2] — unknown directives skipped per §6.8.1
    // y[impl struct.yaml-directive.must-accept-prior+2] — prior YAML versions accepted
    // y[impl struct.yaml-directive.should-process-prior-as-current+2] — 1.x versions processed
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

    // y[impl struct.yaml-directive.at-most-once]
    // y[impl struct.yaml-directive.must-accept-current]
    // y[impl struct.ns-yaml-version]
    // y[impl struct.ns-yaml-directive+2]
    // y[impl struct.yaml-directive.should-reject-higher-major] — major version mismatch not accepted
    // y[impl struct.yaml-directive.should-warn-higher-minor] — minor version differences tolerated
    /// Scan `%YAML x.y`.
    // cref: fy_scan_directive (fy-parse.c:2275)
    fn fetch_version_directive(&mut self) -> Result<Token<'input>, ScanError> {
        // SF5V: duplicate %YAML directive in the same prologue.
        if self.seen_yaml_directive {
            return Err(ScanError {
                message: "duplicate %YAML directive".to_string(),
            });
        }
        self.seen_yaml_directive = true;
        self.seen_directive = true;
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

        // y[impl struct.yaml-directive.version-format] — validate MAJOR.MINOR format.
        // Each component must be 1-4 digits with exactly one dot separator.
        // Rejects malformed versions like `1.12345`, `1.`, `.1`, `1.2.3`.
        // cref: fy_scan_directive (fy-parse.c) — "unsupport version number"
        {
            let parts: Vec<&str> = version_str.split('.').collect();
            if parts.len() != 2
                || parts[0].is_empty()
                || parts[1].is_empty()
                || parts[0].len() > 4
                || parts[1].len() > 4
                || !parts[0].chars().all(|c| c.is_ascii_digit())
                || !parts[1].chars().all(|c| c.is_ascii_digit())
            {
                return Err(ScanError {
                    message: format!("unsupported version number '{version_str}'"),
                });
            }
        }

        // After the version, only whitespace + optional comment allowed.
        // `%YAML 1.2 foo` (H7TQ) and `%YAML 1.1#...` (MUS6) are invalid.
        {
            let mut lookahead = 0;
            while self.reader.peek_at(lookahead) == Some(' ')
                || self.reader.peek_at(lookahead) == Some('\t')
            {
                lookahead += 1;
            }
            match self.reader.peek_at(lookahead) {
                None | Some('\n' | '\r') => {}
                Some('#') if lookahead > 0 => {}
                Some(c) => {
                    return Err(ScanError {
                        message: format!("invalid character '{c}' after YAML version"),
                    });
                }
            }
        }

        self.skip_to_next_line();
        Ok(Self::data_token(
            TokenKind::VersionDirective,
            version_str,
            ver_start,
            ver_end,
        ))
    }

    // y[impl struct.ns-tag-directive+2]
    // y[impl struct.tag-directive.at-most-once-per-handle+2]
    // y[impl struct.c-tag-handle+2]
    // y[impl struct.c-primary-tag-handle+2]
    // y[impl struct.c-secondary-tag-handle+2]
    // y[impl struct.c-named-tag-handle+2]
    // y[impl struct.named-tag-handle.must-be-declared+2]
    // y[impl struct.named-tag-handle.not-content+2]
    // y[impl struct.ns-tag-prefix+2]
    // y[impl struct.c-ns-local-tag-prefix+2]
    // y[impl struct.ns-global-tag-prefix+2]
    /// Scan `%TAG handle prefix`.
    // cref: fy_scan_directive (fy-parse.c:2296)
    fn fetch_tag_directive(&mut self) -> Result<Token<'input>, ScanError> {
        self.seen_directive = true;
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
        // QLJ7: register the tag handle for scope validation in fetch_tag.
        let handle_end = self.reader.mark();
        let handle = self.reader.slice(data_start.offset, handle_end.offset);
        self.tag_handles.push(handle.to_string());
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
    ///
    /// The flow collection itself can be a simple key (e.g., `[a, b]: value`).
    /// The simple key is saved at the CURRENT flow level (before incrementing)
    /// so it can be resolved when `:` is found outside the collection.
    // cref: fy_fetch_flow_collection_mark_start (fy-parse.c:2432)
    // y[impl char.c-sequence-start]
    // y[impl char.c-mapping-start]
    // y[impl char.c-flow-indicator]
    // y[impl flow.c-flow-sequence]
    // y[impl flow.c-flow-mapping]
    fn fetch_flow_collection_start(&mut self, c: char) {
        let start_mark = self.reader.mark();
        let kind = if c == '[' {
            TokenKind::FlowSequenceStart
        } else {
            TokenKind::FlowMappingStart
        };
        let start = self.reader.mark();
        self.reader.advance();
        let end = self.reader.mark();
        // Save simple key at current flow level BEFORE incrementing.
        let token = Self::marker_token(kind, start, end);
        let id = self.enqueue(token);
        self.save_simple_key_ext(id, start_mark, true);
        if self.flow_level == 0 {
            // Track the block indent when entering flow context.
            self.flow_indent = self.indent;
        }
        self.flow_level += 1;
        // Enforce max_depth on combined nesting (block indent + flow level).
        if let Err(msg) = self
            .config
            .check_depth(self.indent_stack.len() + self.flow_level as usize)
        {
            self.error = Some(ScanError { message: msg });
            return;
        }
        self.flow_is_mapping.push(c == '{');
        self.simple_key_allowed = true;
    }

    /// Consume `]` or `}` and emit a flow collection end token.
    // cref: fy_fetch_flow_collection_mark_end (fy-parse.c:2518)
    // y[impl char.c-sequence-end]
    // y[impl char.c-mapping-end]
    // y[impl flow.in-flow+3]
    fn fetch_flow_collection_end(&mut self, c: char) {
        if self.flow_level == 0 {
            self.reader.advance();
            self.error = Some(ScanError {
                message: format!("unexpected '{c}' outside of flow collection"),
            });
            return;
        }
        self.remove_simple_key();
        let kind = if c == ']' {
            TokenKind::FlowSequenceEnd
        } else {
            TokenKind::FlowMappingEnd
        };
        let start = self.reader.mark();
        self.reader.advance();
        let end = self.reader.mark();
        self.flow_level -= 1;
        self.flow_is_mapping.pop();
        self.enqueue(Self::marker_token(kind, start, end));
        self.simple_key_allowed = false;
        // In flow context, `:` immediately after `]`/`}` is a value
        // indicator (JSON-compatible, e.g., `{a: b}:value`).
        if self.flow_level > 0 {
            self.adjacent_value_offset = Some(self.reader.mark().offset);
        }
        // When returning to block context, validate trailing content and
        // update simple key tracking.
        // cref: fy_fetch_flow_collection_mark_end (fy-parse.c)
        if self.flow_level == 0 {
            // C2SP: update the outer simple key's end_line to the flow
            // collection's closing line. Without this, the multiline key
            // check in fetch_value can't detect cross-line flow keys.
            if let Some(sk) = self.simple_keys.last_mut().filter(|s| s.flow_level == 0) {
                sk.end_line = end.line;
            }

            // `:` is allowed (e.g., `[a]: value`), as are whitespace and comments.
            // Anything else (e.g., `{ y: z }- invalid`) is an error.
            let mut lookahead = 0;
            while self.reader.peek_at(lookahead) == Some(' ')
                || self.reader.peek_at(lookahead) == Some('\t')
            {
                lookahead += 1;
            }
            match self.reader.peek_at(lookahead) {
                None | Some('\n' | '\r') => {
                    // KS4U: flow collection at root with no `:` following —
                    // root node is complete, reject further content.
                    if self.indent == -1 {
                        self.root_token_emitted = true;
                    }
                }
                Some(':') => {}
                Some('#') if lookahead > 0 => {
                    // Comment after flow close at root — root is complete.
                    if self.indent == -1 {
                        self.root_token_emitted = true;
                    }
                }
                Some(c) => {
                    self.error = Some(ScanError {
                        message: format!("invalid character '{c}' after flow collection close"),
                    });
                }
            }
        }
    }

    /// Consume `,` and emit a flow entry token.
    // cref: fy_parse_handle_comma (fy-parse.c:1174)
    // y[impl char.c-collect-entry]
    fn fetch_flow_entry(&mut self) {
        self.remove_simple_key();
        let start = self.reader.mark();
        self.reader.advance();
        let end = self.reader.mark();
        self.enqueue(Self::marker_token(TokenKind::FlowEntry, start, end));
        self.simple_key_allowed = true;
    }

    // -- Block indicators --

    /// Fetch a block entry (`- `). May push `BlockSequenceStart` into the queue first.
    // cref: fy_fetch_block_entry (fy-parse.c:2703)
    // y[impl char.c-sequence-entry]
    // y[impl block.l-block-sequence]
    // y[impl block.c-l-block-seq-entry]
    // y[impl block.seq.dash-separated]
    fn fetch_block_entry(&mut self) {
        // Y79Y#4-7: tabs in the whitespace before a block entry indicator
        // are indentation — reject them.
        if self.tab_in_preceding_whitespace {
            self.error = Some(ScanError {
                message: "tab character used for indentation of block entry".to_string(),
            });
            return;
        }
        // A block entry that would create a new indent level cannot appear
        // on the same line as a preceding value indicator (e.g., `key: - a`).
        // cref: fy_fetch_block_entry (fy-parse.c:2703)
        let mark = self.reader.mark();
        let col = mark.column as i32;
        if col > self.indent && self.last_block_token_line.is_some_and(|vl| vl == mark.line) {
            self.error = Some(ScanError {
                message: "block sequence entries not allowed in this context".to_string(),
            });
            return;
        }
        self.remove_simple_key();
        self.roll_indent(col, false);
        let start = self.reader.mark();
        self.reader.advance(); // skip '-'
        let end = self.reader.mark();
        self.enqueue(Self::marker_token(TokenKind::BlockEntry, start, end));
        self.simple_key_allowed = true;
    }

    /// Fetch an explicit key (`? `). May push `BlockMappingStart` into the queue first.
    // cref: fy_fetch_key (fy-parse.c:2818)
    // y[impl char.c-mapping-key]
    // y[impl block.c-l-block-map-explicit-key]
    fn fetch_key(&mut self) {
        self.remove_simple_key();
        let col = self.reader.mark().column as i32;
        self.roll_indent(col, true);
        let start = self.reader.mark();
        self.reader.advance(); // skip '?'
        let end = self.reader.mark();
        self.enqueue(Self::marker_token(TokenKind::Key, start, end));
        self.simple_key_allowed = true;
        self.pending_complex_key_column = col;
        // Y79Y#8: tab immediately after `?` is indentation for the key content.
        if self.reader.peek() == Some('\t') {
            self.error = Some(ScanError {
                message: "tab character used for indentation".to_string(),
            });
        }
    }

    /// Fetch a value indicator (`: `).
    ///
    /// Checks the simple key stack: if a pending simple key exists at the
    /// current flow level, inserts `BlockMappingStart` (if needed) + `Key`
    /// before the key token. If no simple key exists, this is an empty key
    /// (`: value`) — emit `BlockMappingStart` + `Key` at the current position.
    // cref: fy_fetch_value (fy-parse.c:2931)
    // y[impl char.c-mapping-value]
    // y[impl block.l-block-mapping]
    // y[impl block.ns-l-block-map-entry]
    // y[impl block.ns-l-block-map-implicit-entry]
    // y[impl block.c-l-block-map-implicit-value]
    // y[impl block.ns-s-block-map-implicit-key]
    // y[impl block.explicit-key-separate-value]
    // y[impl block.value-not-adjacent]
    fn fetch_value(&mut self) {
        self.purge_stale_simple_keys();

        // Check for explicit key resolution first (`? key : value`).
        // The `?` already emitted Key — we just need Value.
        let mark = self.reader.mark();
        let col = mark.column as i32;
        let is_explicit_key_value = self.pending_complex_key_column >= 0
            && (self.flow_level > 0 || col <= self.pending_complex_key_column);

        if is_explicit_key_value {
            self.pending_complex_key_column = -1;
            // Discard any simple key from the explicit key's content.
            if self
                .simple_keys
                .last()
                .is_some_and(|s| s.flow_level == self.flow_level)
            {
                self.simple_keys.pop();
            }
            // In block context, the value content can start a new mapping.
            self.simple_key_allowed = self.flow_level == 0;
        } else {
            // Pop the simple key at the current flow level (if any).
            let sk = self
                .simple_keys
                .last()
                .filter(|s| s.flow_level == self.flow_level)
                .cloned();
            if sk.is_some() {
                self.simple_keys.pop();
            }

            if let Some(sk) = sk {
                // YAML §7.4.2: implicit keys are restricted to a single line.
                // The key token must start and end on the same line. For
                // json_key tokens (quoted scalars, flow collections), `:` may
                // appear on a subsequent line (e.g., `{ "foo" # comment\n :bar }`)
                // as long as the key itself is single-line.
                // cref: fy_fetch_value (fy-parse.c)
                // Multiline simple key restrictions:
                // - Block context: multiline keys are never allowed.
                // - Flow context: plain keys cannot have `:` on a different
                //   line. json_key keys (quoted/flow) CAN span lines.
                let key_is_multiline = sk.end_line > sk.mark.line;
                let colon_on_different_line = sk.end_line < mark.line;
                let reject = if self.flow_level == 0 {
                    // Block: reject any multiline key or cross-line non-json key.
                    key_is_multiline || (sk.mark.line != mark.line && !sk.json_key)
                } else {
                    // Flow: reject when `:` is on a different line than the
                    // key END (not start). Plain keys can fold across lines
                    // but `:` must be on the key's last line.
                    // cref: fy_fetch_value (fy-parse.c:3049) — also reject
                    // multiline implicit keys in flow sequences. Only flow
                    // mappings allow json_keys across lines.
                    if colon_on_different_line
                        && !self.flow_is_mapping.last().copied().unwrap_or(false)
                    {
                        true
                    } else {
                        sk.end_line != mark.line && !sk.json_key
                    }
                };
                if reject {
                    self.error = Some(ScanError {
                        message: "multiline simple key is not allowed".to_string(),
                    });
                    return;
                }

                // We have a pending simple key — insert Key (and BlockMappingStart)
                // before the key token in the queue.
                if let Some(pos) = self.queue_position(sk.token_id) {
                    // Enforce max_key_bytes limit on the key's scalar data.
                    if let Err(msg) = self.config.check_key_size(self.queue[pos].atom.data.len()) {
                        self.error = Some(ScanError { message: msg });
                        return;
                    }

                    // In block context, push BlockMappingStart if indent warrants.
                    if self.flow_level == 0 && sk.mark.column as i32 > self.indent {
                        // 9KBC, CXX2: block collections cannot start on the `---` line.
                        if self.document_start_line == Some(sk.mark.line) {
                            self.error = Some(ScanError {
                                message: "block collection cannot start on the document start line"
                                    .to_string(),
                            });
                            return;
                        }
                        let bms =
                            Self::marker_token(TokenKind::BlockMappingStart, sk.mark, sk.mark);
                        self.queue.insert(pos, bms);
                        self.next_token_id += 1;
                        // Entering block context from root clears root_token_emitted.
                        if self.indent == -1 {
                            self.root_token_emitted = false;
                        }
                        self.indent_stack.push(self.indent);
                        // Enforce max_depth on combined block + flow nesting.
                        if let Err(msg) = self
                            .config
                            .check_depth(self.indent_stack.len() + self.flow_level as usize)
                        {
                            self.error = Some(ScanError { message: msg });
                            return;
                        }
                        self.indent = sk.mark.column as i32;
                        // Key goes after BlockMappingStart, before the key token.
                        let key = Self::marker_token(TokenKind::Key, sk.mark, sk.mark);
                        self.queue.insert(pos + 1, key);
                    } else {
                        let key = Self::marker_token(TokenKind::Key, sk.mark, sk.mark);
                        self.queue.insert(pos, key);
                    }
                    self.next_token_id += 1;
                }
                self.simple_key_allowed = false;
            } else {
                // No simple key — bare empty key (`: value`).
                if self.flow_level == 0 {
                    self.roll_indent(col, true);
                }
                // Emit Key in both block and flow context for empty keys.
                self.enqueue(Self::marker_token(TokenKind::Key, mark, mark));
                self.simple_key_allowed = self.flow_level == 0;
            }
        }

        let start = self.reader.mark();
        self.reader.advance(); // skip ':'
        let end = self.reader.mark();
        self.enqueue(Self::marker_token(TokenKind::Value, start, end));
        // Track last value line for block entry validation, but NOT for
        // explicit key values (? key : - seq) where block entries are valid.
        if !is_explicit_key_value && self.flow_level == 0 {
            self.last_block_token_line = Some(start.line);
        }
        // Y79Y#9: tab immediately after `:` is indentation for the value content.
        if self.flow_level == 0 && self.reader.peek() == Some('\t') {
            self.error = Some(ScanError {
                message: "tab character used for indentation".to_string(),
            });
        }
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
    // y[impl flow.ns-plain-char+4] — plain scalar character set (excluding : # and flow indicators)
    // y[impl flow.ns-plain-safe-in+4] — plain safe chars inside flow collections
    // y[impl flow.ns-plain-safe-out+4] — plain safe chars outside flow collections
    // y[impl flow.plain.must-not-contain-colon-space-space-hash] — `: ` and ` #` terminate plain scalars
    fn scan_plain_scalar_line(&mut self, is_first_line: bool) -> &'input str {
        let start_offset = self.reader.mark().offset;

        // YAML §7.3.3 ns-plain-first: `#` cannot start a plain scalar.
        // Other c-indicators (`&`, `*`, `!`, etc.) are handled by dedicated
        // fetch methods before reaching the plain scalar fallback.
        if self.reader.peek() == Some('#') {
            return self.reader.slice(start_offset, start_offset);
        }

        // YAML §7.3.3 ns-plain-first (first line only): `-`, `?`, `:` can
        // start a plain scalar only when followed by ns-plain-safe. In flow
        // context, flow indicators and blank/EOF are not ns-plain-safe, so
        // bare `-`, `?`, `:` are invalid. On continuation lines, these are
        // just regular ns-plain-char content.
        // cref: YAML 1.2 §7.3.3 [126] ns-plain-first
        if is_first_line
            && matches!(self.reader.peek(), Some('-' | '?' | ':'))
            && (is_blank_or_end(self.reader.peek_at(1))
                || (self.flow_level > 0
                    && matches!(self.reader.peek_at(1), Some(',' | '[' | ']' | '{' | '}'))))
        {
            return self.reader.slice(start_offset, start_offset);
        }

        let mut prev_was_space = false;

        loop {
            match self.reader.peek() {
                None | Some('\n' | '\r') => break,
                Some(':') if is_blank_or_end(self.reader.peek_at(1)) => break,
                // In flow context, `:` followed by a flow indicator is also
                // a value indicator (e.g., `key:,` or `key:}`).
                // cref: fy_fetch_tokens (fy-parse.c:5514)
                Some(':')
                    if self.flow_level > 0
                        && matches!(self.reader.peek_at(1), Some(',' | '[' | ']' | '{' | '}')) =>
                {
                    break;
                }
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
    /// single newline between content lines becomes space, empty lines
    /// (only whitespace) become literal newlines.
    ///
    /// Returns `(text, content_end_line)` where `content_end_line` is the
    /// line number of the last actual content character (not including consumed
    /// whitespace from failed continuation lookahead).
    // cref: fy_reader_fetch_plain_scalar_handle_inline (fy-parse.c:4434)
    // y[impl flow.ns-plain-first+4]
    // y[impl flow.ns-plain-safe+4]
    // y[impl struct.s-line-prefix] — leading whitespace on continuation lines
    // y[impl struct.s-block-line-prefix] — block context line prefix
    // y[impl struct.line-prefix.not-content] — line prefix is not part of scalar content
    fn scan_plain_scalar_text(&mut self) -> (Cow<'input, str>, u32) {
        let first_line = self.scan_plain_scalar_line(true);
        let start_line = self.reader.mark().line;
        if first_line.is_empty() {
            return (Cow::Borrowed(first_line), start_line);
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
            return (Cow::Borrowed(first_line), start_line);
        }

        // Multi-line: build folded result.
        let mut result = String::from(first_line);
        let mut empty_lines = 0u32;
        let mut content_end_line = start_line;

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

            // Scan the continuation line before adding fold separator,
            // in case a terminator stops it immediately (empty result).
            let line = self.scan_plain_scalar_line(false);
            if line.is_empty() {
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

            result.push_str(line);
            content_end_line = self.reader.mark().line;
        }

        (Cow::Owned(result), content_end_line)
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
                    // Note: `%` at column 0 is NOT rejected here — inside a
                    // document, `%YAML` etc. is plain scalar content, not a
                    // directive (YAML §9.2). Directives only appear between
                    // documents.
                    // Flow indicators end the scalar in flow context.
                    if self.flow_level > 0 && matches!(c, ',' | '[' | ']' | '{' | '}') {
                        return false;
                    }
                    return col >= min_indent;
                }
                None => return false,
            }
        }
    }

    /// Fetch a plain scalar token.
    ///
    /// Scans the scalar text and saves a simple key mark so that if `:`
    /// follows later, `fetch_value` can retroactively insert `Key` and
    /// `BlockMappingStart`.
    // cref: fy_fetch_plain_scalar (fy-parse.c:5151)
    // y[impl flow.ns-plain]
    // y[impl flow.ns-plain-one-line]
    // y[impl flow.ns-plain-multi-line]
    // y[impl flow.s-ns-plain-next-line]
    // y[impl flow.nb-ns-plain-in-line]
    // y[impl flow.plain.continuation-must-contain-non-space]
    // y[impl flow.plain.must-not-be-empty]
    // y[impl flow.plain.must-not-begin-with-indicators]
    // y[impl flow.plain.must-not-contain-flow-indicators+3]
    fn fetch_plain_scalar(&mut self) {
        // Don't remove pending simple keys — a preceding tag/anchor's
        // simple key should remain if this scalar is part of the same key.
        let start_mark = self.reader.mark();
        let (text, content_end_line) = self.scan_plain_scalar_text();
        let end_mark = self.reader.mark();

        if text.is_empty() {
            // Nothing scanned — character cannot start a token here.
            let c = self.reader.peek().unwrap_or('\0');
            self.reader.advance();
            self.error = Some(ScanError {
                message: format!("unexpected character '{c}' in this context"),
            });
            return;
        }

        // cref: fy_scan_plain_scalar (fy-parse.c:5207-5228)
        // After a multiline plain scalar in block context, if `:` follows
        // (with blank/EOF after it), this scalar would become a multiline
        // implicit key — which is never allowed. Skip when inside an
        // explicit key (`?`) where `:` is the value indicator.
        if self.flow_level == 0
            && content_end_line > start_mark.line
            && self.pending_complex_key_column < 0
        {
            let mut lookahead = 0;
            while matches!(self.reader.peek_at(lookahead), Some(' ' | '\t')) {
                lookahead += 1;
            }
            if self.reader.peek_at(lookahead) == Some(':')
                && is_blank_or_end(self.reader.peek_at(lookahead + 1))
            {
                self.error = Some(ScanError {
                    message: "invalid multiline plain key".to_string(),
                });
                return;
            }
        }

        // Enforce max_scalar_bytes limit.
        if let Err(msg) = self.config.check_scalar_size(text.len()) {
            self.error = Some(ScanError { message: msg });
            return;
        }

        let scalar = Self::scalar_token(text, ScalarStyle::Plain, start_mark, end_mark);
        let id = self.enqueue(scalar);
        self.save_simple_key_full(id, start_mark, content_end_line, false);
        // Multi-line plain scalars consume newlines internally. In block
        // context, crossing a line allows the NEXT token to be a simple key.
        self.simple_key_allowed = self.flow_level == 0 && end_mark.line > start_mark.line;
    }

    // -- Tags --

    /// Fetch a tag token (`!`, `!!suffix`, `!handle!suffix`, `!<uri>`).
    ///
    /// The full tag text (including `!` prefix) is stored as the token value.
    /// A simple key mark is saved so the tag can be retroactively identified
    /// as part of a mapping key when `:` follows.
    // cref: fy_fetch_tag (fy-parse.c:3342)
    // y[impl char.c-tag+2]
    // y[impl struct.c-ns-tag-property+2]
    // y[impl struct.c-verbatim-tag+2]
    // y[impl struct.verbatim-tag.deliver-as-is+2]
    // y[impl struct.verbatim-tag.must-be-local-or-uri+2]
    // y[impl struct.c-ns-shorthand-tag+2]
    // y[impl struct.shorthand-tag.handle-must-have-prefix+2]
    // y[impl struct.shorthand-tag.result-must-be-local-or-uri+2]
    // y[impl struct.shorthand-tag.handle-not-content+2]
    // y[impl struct.shorthand-tag.suffix-no-bang+2]
    // y[impl struct.shorthand-tag.suffix-escape+2]
    // y[impl struct.shorthand-tag.suffix-no-flow-chars+3]
    // y[impl struct.c-non-specific-tag+2]
    // y[impl char.ns-uri-char]
    // y[impl char.ns-word-char]
    // y[impl char.misc.tag-preserve+2]
    // y[impl char.misc.uri-no-expand] — URI percent-encoding preserved, not expanded during scan
    // y[impl char.misc.tag-shorthand-restrict+3] — shorthand tag handles restricted to word chars
    // y[impl char.ns-tag-char+3] — tag characters: ns-uri-char minus ! and flow indicators
    // y[impl struct.global-tag-prefix.must-be-valid-uri] — global tag prefix validated as URI
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

        // YAML §6.8.2: a tag must be followed by s-separate (whitespace),
        // a newline, EOF, or a flow indicator that properly terminates it.
        // Content like `!invalid{}tag` is an error — `{` is not whitespace.
        // cref: fy_fetch_tag (fy-parse.c:3342)
        match self.reader.peek() {
            None | Some(' ' | '\t' | '\n' | '\r') => {}
            Some(',' | '[' | ']' | '{' | '}') if self.flow_level > 0 => {}
            Some(c) => {
                self.error = Some(ScanError {
                    message: format!("invalid character '{c}' after tag"),
                });
                return;
            }
        }

        let tag_raw = self.reader.slice(start_mark.offset, end_mark.offset);

        // QLJ7: validate named tag handles are declared via %TAG in this document.
        // Primary `!` and secondary `!!` are always valid; named `!name!` must
        // be explicitly declared. Verbatim `!<uri>` bypasses handle resolution.
        if tag_raw.len() > 1 && !tag_raw.starts_with("!<") {
            // Find the handle portion: everything up to and including the
            // second `!` (if present). E.g., `!prefix!A` → handle `!prefix!`.
            if let Some(second_bang) = tag_raw[1..].find('!') {
                let handle = &tag_raw[..second_bang + 2]; // includes both `!`
                if handle != "!!" && !self.tag_handles.iter().any(|h| h == handle) {
                    self.error = Some(ScanError {
                        message: format!("undeclared tag handle '{handle}'"),
                    });
                    return;
                }
            }
        }

        // Decode URI percent-encoding in tag suffix (YAML §6.9.1).
        // E.g., `!e!tag%21` → `!e!tag!`.
        let tag_data: Cow<'input, str> = if tag_raw.contains('%') {
            Cow::Owned(decode_tag_uri(tag_raw))
        } else {
            Cow::Borrowed(tag_raw)
        };
        let token = Token {
            kind: TokenKind::Tag,
            atom: Atom {
                data: tag_data,
                span: Span {
                    start: start_mark,
                    end: end_mark,
                },
                style: ScalarStyle::Plain,
                chomp: Chomp::default(),
                flags: AtomFlags::empty(),
            },
        };
        let id = self.enqueue(token);
        self.save_simple_key(id, start_mark);
        self.simple_key_allowed = false;
    }

    // -- Anchors and aliases --

    /// Fetch an anchor (`&name`) or alias (`*name`).
    ///
    /// Reads the indicator and the following name. Saves a simple key mark
    /// so that anchors and aliases can serve as mapping keys.
    // cref: fy_fetch_anchor_or_alias (fy-parse.c:5443-5454)
    // y[impl char.c-anchor]
    // y[impl char.c-alias]
    // y[impl struct.c-ns-anchor-property]
    // y[impl struct.ns-anchor-name]
    // y[impl struct.anchor.not-content]
    // y[impl struct.anchor.no-flow-chars+3]
    // y[impl struct.ns-anchor-char+3]
    // y[impl flow.c-ns-alias-node]
    // y[impl flow.alias.error-undefined-anchor]
    // y[impl flow.alias.must-anchor-first]
    // y[impl flow.alias.must-not-specify-properties]
    fn fetch_anchor_or_alias(&mut self, indicator: char) {
        let start_mark = self.reader.mark();
        let kind = if indicator == '&' {
            TokenKind::Anchor
        } else {
            TokenKind::Alias
        };
        self.reader.advance(); // skip & or *
        let name_start = self.reader.mark();

        // YAML 1.2 §6.9.2: anchor names end at whitespace or flow indicators.
        // Colons are allowed in anchor names (e.g., `&a:b`, `*a:`).
        while let Some(c) = self.reader.peek() {
            if c.is_ascii_whitespace() || matches!(c, ',' | '[' | ']' | '{' | '}') {
                break;
            }
            self.reader.advance();
        }

        let name_end = self.reader.mark();
        let name = self.reader.slice(name_start.offset, name_end.offset);
        let token = Self::data_token(kind, name, name_start, name_end);
        let id = self.enqueue(token);
        self.save_simple_key(id, start_mark);
        self.simple_key_allowed = false;
        if self.flow_level == 0 {
            self.last_block_token_line = Some(start_mark.line);
        }
    }

    // -- Block scalars --

    /// Fetch a literal (`|`) or folded (`>`) block scalar.
    ///
    /// Parses the header (indicator, optional chomp/indent), then reads
    /// content lines respecting indentation. Applies chomp rules and
    /// folding (for `>`) to produce the final scalar value.
    // cref: fy_fetch_block_scalar (fy-parse.c), fy_reader_fetch_block_scalar_handle (fy-reader.c)
    // y[impl char.c-literal]
    // y[impl char.c-folded]
    // y[impl block.c-l-literal]
    // y[impl block.c-l-folded]
    // y[impl block.c-b-block-header]
    // y[impl block.header.comment-no-follow]
    // y[impl block.c-chomping-indicator]
    // y[impl block.chomping.not-content]
    // y[impl block.c-indentation-indicator]
    // y[impl block.l-literal-content]
    // y[impl block.l-folded-content]
    // y[impl block.l-nb-literal-text]
    // y[impl block.b-nb-literal-next]
    // y[impl block.l-nb-diff-lines]
    // y[impl block.l-nb-folded-lines]
    // y[impl block.s-nb-folded-text]
    // y[impl block.b-l-spaced]
    // y[impl block.l-nb-spaced-lines]
    // y[impl block.s-nb-spaced-text]
    // y[impl block.l-nb-same-lines]
    // y[impl block.l-chomped-empty]
    // y[impl block.l-strip-empty]
    // y[impl block.l-keep-empty]
    // y[impl block.l-trail-comments]
    // y[impl block.trail-comment.indent]
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

        // Validate rest of header: only whitespace and optional comment allowed.
        // cref: YAML 1.2 §8.1 — c-b-block-header = ( (c-indentation-indicator c-chomping-indicator)
        //                      | (c-chomping-indicator c-indentation-indicator) ) s-b-comment
        {
            let mut lookahead = 0;
            while self.reader.peek_at(lookahead) == Some(' ')
                || self.reader.peek_at(lookahead) == Some('\t')
            {
                lookahead += 1;
            }
            match self.reader.peek_at(lookahead) {
                None | Some('\n' | '\r') => {}
                // `#` is only a comment when preceded by whitespace.
                Some('#') if lookahead > 0 => {}
                Some(c) => {
                    self.error = Some(ScanError {
                        message: format!("invalid character '{c}' in block scalar header"),
                    });
                    return;
                }
            }
        }

        // Skip rest of header line (whitespace, comment).
        self.skip_to_next_line();

        // Determine content indentation.
        // cref: fy-parse.c:3653 — current_indent = fyp->indent >= 0 ? fyp->indent : 0
        // Explicit indent indicator is relative to the current indent level
        // (YAML 1.2 §8.1.1.2). Auto-detect uses the first non-empty content line
        // but must respect the minimum indent (current + 1).
        let current_indent = self.indent.max(0) as usize;
        let content_indent = match explicit_indent {
            Some(ei) => current_indent + ei,
            None => {
                let (detected, max_empty_spaces) = self.detect_block_indent_lookahead();
                let min_indent = (self.indent + 1).max(0) as usize;
                let indent = detected.max(min_indent);
                // cref: fy_scan_block_scalar (fy-parse.c) — reject when spaces-only
                // lines before the first content line have more spaces than the
                // detected indent level.
                if max_empty_spaces > indent {
                    self.error = Some(ScanError {
                        message: "block scalar with wrong indented line after spaces only"
                            .to_string(),
                    });
                    return;
                }
                indent
            }
        };

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
                // Blank line: only spaces then newline.
                // If spaces exceed content_indent, the extra spaces are content
                // (e.g., trailing whitespace on an otherwise blank line).
                // Treat those as content lines, not trailing empties.
                Some('\n' | '\r') if spaces <= content_indent => {
                    trailing_empty += 1;
                    self.reader.advance_by(spaces);
                    self.reader.advance(); // consume newline
                }
                // EOF after spaces only.
                None if spaces <= content_indent => break,
                // Content line (or blank line with significant trailing spaces).
                _ => {
                    // cref: fy_scan_block_scalar (fy-parse.c:3700)
                    // Document indicators `---` / `...` at column 0 always
                    // terminate block scalar content, even when content_indent
                    // is 0 (i.e. block scalar at document level).
                    if spaces == 0 && (self.is_document_start() || self.is_document_end()) {
                        break;
                    }

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
        // y[impl block.b-chomped-last+4] — final line break handling per chomp indicator
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

        // Enforce max_scalar_bytes limit.
        if let Err(msg) = self.config.check_scalar_size(content.len()) {
            self.error = Some(ScanError { message: msg });
            return;
        }

        let end_mark = self.reader.mark();
        let style = if is_literal {
            ScalarStyle::Literal
        } else {
            ScalarStyle::Folded
        };
        let scalar = Self::scalar_token(Cow::Owned(content), style, start_mark, end_mark);
        self.enqueue(scalar);
        // Block scalars always end at a line boundary, so the next token
        // on the new line can be a simple key.
        self.simple_key_allowed = self.flow_level == 0;
    }

    /// Detect block scalar content indentation by peeking ahead
    /// to the first non-empty line.
    /// Returns `(indent, max_empty_spaces)`: the raw number of leading spaces
    /// on the first non-empty line, and the maximum spaces seen on any
    /// spaces-only line before it.
    /// The caller is responsible for applying a minimum indent floor.
    // cref: fy_scan_block_scalar (fy-parse.c) — "block scalar with wrong indented line after spaces only"
    fn detect_block_indent_lookahead(&self) -> (usize, usize) {
        let mut pos = 0;
        let mut max_empty_spaces: usize = 0;
        loop {
            let mut spaces = 0;
            while self.reader.peek_at(pos) == Some(' ') {
                spaces += 1;
                pos += 1;
            }
            match self.reader.peek_at(pos) {
                None => return (spaces, max_empty_spaces),
                Some('\n') => {
                    if spaces > max_empty_spaces {
                        max_empty_spaces = spaces;
                    }
                    pos += 1;
                }
                Some('\r') => {
                    if spaces > max_empty_spaces {
                        max_empty_spaces = spaces;
                    }
                    pos += 1;
                    if self.reader.peek_at(pos) == Some('\n') {
                        pos += 1;
                    }
                }
                _ => return (spaces, max_empty_spaces),
            }
        }
    }

    // -- Quoted scalars --

    /// Fetch a single-quoted scalar (`'...'`).
    ///
    /// The only escape in single-quoted scalars is `''` → `'`.
    // cref: fy_reader_fetch_flow_scalar_handle (fy-reader.c) — single-quoted branch
    // y[impl char.c-single-quote]
    // y[impl flow.c-single-quoted]
    // y[impl flow.single-quoted.continuation-must-contain-non-space]
    // y[impl flow.nb-single-char]
    // y[impl flow.ns-single-char]
    // y[impl flow.c-quoted-quote]
    // y[impl flow.nb-single-one-line]
    // y[impl flow.nb-single-text]
    // y[impl flow.nb-single-multi-line]
    // y[impl flow.s-single-next-line]
    // y[impl flow.nb-ns-single-in-line]
    // y[impl struct.s-flow-folded] — flow scalar line folding
    // y[impl struct.s-flow-line-prefix] — leading whitespace on continuation lines
    // y[impl struct.flow-folding.spaces-not-content] — surrounding spaces stripped during fold
    fn fetch_single_quoted_scalar(&mut self) {
        let start_mark = self.reader.mark();
        self.reader.advance(); // skip opening '

        let mut result = String::new();
        let mut needs_owned = false;

        loop {
            match self.reader.peek() {
                None => {
                    self.error = Some(ScanError {
                        message: "unterminated single-quoted scalar".to_string(),
                    });
                    return;
                }
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
                // Trailing whitespace on the current line is excluded (§6.5).
                Some('\n' | '\r') => {
                    needs_owned = true;
                    while result.ends_with(' ') || result.ends_with('\t') {
                        result.pop();
                    }
                    self.reader.advance();
                    // YAML §9.1.4: document markers at column 0 are forbidden
                    // inside flow scalars. Check after consuming the newline.
                    // cref: fy_reader_fetch_flow_scalar_handle (fy-reader.c)
                    {
                        let mut ws = 0;
                        while matches!(self.reader.peek_at(ws), Some(' ' | '\t')) {
                            ws += 1;
                        }
                        if ws == 0
                            && self.reader.mark().column == 0
                            && (self.is_document_start() || self.is_document_end())
                        {
                            self.error = Some(ScanError {
                                message: "document marker inside single-quoted scalar".to_string(),
                            });
                            return;
                        }
                    }
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
                    // In block context, continuation lines of a quoted scalar
                    // must be indented past the current block indent level.
                    if self.flow_level == 0 {
                        let min_indent = (self.indent + 1).max(0) as u32;
                        if self.reader.mark().column < min_indent
                            && !matches!(self.reader.peek(), Some('\''))
                        {
                            self.error = Some(ScanError {
                                message: "single-quoted scalar continuation below block indent"
                                    .to_string(),
                            });
                            return;
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

        // Enforce max_scalar_bytes limit.
        if let Err(msg) = self.config.check_scalar_size(data.len()) {
            self.error = Some(ScanError { message: msg });
            return;
        }

        self.push_quoted_scalar(data, ScalarStyle::SingleQuoted, start_mark, end_mark);
    }

    /// Fetch a double-quoted scalar (`"..."`), processing escape sequences.
    // cref: fy_reader_fetch_flow_scalar_handle (fy-reader.c) — double-quoted branch
    // y[impl char.c-double-quote]
    // y[impl char.c-escape]
    // y[impl char.c-ns-esc-char+3]
    // y[impl char.ns-esc-null]
    // y[impl char.ns-esc-bell]
    // y[impl char.ns-esc-backspace]
    // y[impl char.ns-esc-horizontal-tab+2]
    // y[impl char.ns-esc-line-feed]
    // y[impl char.ns-esc-vertical-tab]
    // y[impl char.ns-esc-form-feed]
    // y[impl char.ns-esc-carriage-return]
    // y[impl char.ns-esc-escape]
    // y[impl char.ns-esc-space]
    // y[impl char.ns-esc-double-quote]
    // y[impl char.ns-esc-backslash]
    // y[impl char.ns-esc-next-line]
    // y[impl char.ns-esc-non-breaking-space]
    // y[impl char.ns-esc-line-separator]
    // y[impl char.ns-esc-paragraph-separator]
    // y[impl char.ns-esc-8-bit]
    // y[impl char.ns-esc-16-bit]
    // y[impl char.ns-esc-32-bit]
    // y[impl char.ns-esc-slash+3]
    // y[impl char.escape.must-escape]
    // y[impl char.escape.not-content]
    // y[impl char.escape.parse-to-unicode]
    // y[impl flow.c-double-quoted]
    // y[impl flow.double-quoted.continuation-must-contain-non-space]
    // y[impl flow.nb-double-char]
    // y[impl flow.ns-double-char]
    // y[impl flow.nb-double-one-line]
    // y[impl flow.nb-double-text]
    // y[impl flow.nb-double-multi-line]
    // y[impl flow.s-double-break]
    // y[impl flow.s-double-escaped]
    // y[impl flow.s-double-next-line]
    // y[impl flow.nb-ns-double-in-line]
    fn fetch_double_quoted_scalar(&mut self) {
        let start_mark = self.reader.mark();
        self.reader.advance(); // skip opening "

        let mut result = String::new();
        let mut needs_owned = false;
        // Byte offset into `result` below which content is protected from
        // trailing-whitespace stripping.  Escape sequences like `\t` produce
        // content characters that must survive line-fold trimming (YAML §6.1).
        let mut escape_fence: usize = 0;

        loop {
            match self.reader.peek() {
                None => {
                    self.error = Some(ScanError {
                        message: "unterminated double-quoted scalar".to_string(),
                    });
                    return;
                }
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
                            // Invalid escape sequence — error per YAML §5.7.
                            self.reader.advance();
                            self.error = Some(ScanError {
                                message: format!(
                                    "invalid escape character '\\{c}' in double-quoted scalar"
                                ),
                            });
                            return;
                        }
                        None => {
                            self.error = Some(ScanError {
                                message: "unterminated escape in double-quoted scalar".to_string(),
                            });
                            return;
                        }
                    }
                    // Protect escape-produced content from trailing-whitespace
                    // stripping on line fold (YAML §6.1 — escapes are content).
                    escape_fence = result.len();
                }
                // Newline inside double-quoted scalar: fold per YAML 1.2 §6.5.
                // Single newline between content → space.
                // Empty lines (newline-only) → literal newline.
                // Leading whitespace on continuation is trimmed.
                // Trailing whitespace on the current line is excluded (§6.5),
                // but escape-produced characters are content and must survive.
                Some('\n' | '\r') => {
                    needs_owned = true;
                    while result.len() > escape_fence
                        && (result.ends_with(' ') || result.ends_with('\t'))
                    {
                        result.pop();
                    }
                    self.reader.advance(); // consume newline
                    // YAML §9.1.4: document markers at column 0 are forbidden
                    // inside flow scalars. Check after consuming the newline.
                    // cref: fy_reader_fetch_flow_scalar_handle (fy-reader.c)
                    {
                        let mut ws = 0;
                        while matches!(self.reader.peek_at(ws), Some(' ' | '\t')) {
                            ws += 1;
                        }
                        if ws == 0
                            && self.reader.mark().column == 0
                            && (self.is_document_start() || self.is_document_end())
                        {
                            self.error = Some(ScanError {
                                message: "document marker inside double-quoted scalar".to_string(),
                            });
                            return;
                        }
                    }
                    // DK95#1: tab as the first character on a continuation
                    // line is indentation — reject it. Spaces before a tab
                    // (DK95#2: `\n  \tbaz`) are fine.
                    if self.reader.peek() == Some('\t') && self.reader.mark().column == 0 {
                        self.error = Some(ScanError {
                            message: "invalid tab used as indentation".to_string(),
                        });
                        return;
                    }
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
                    // In block context, continuation lines of a quoted scalar
                    // must be indented past the current block indent level.
                    // cref: fy_reader_fetch_flow_scalar_handle (fy-reader.c)
                    if self.flow_level == 0 {
                        let min_indent = (self.indent + 1).max(0) as u32;
                        if self.reader.mark().column < min_indent
                            && !matches!(self.reader.peek(), Some('"'))
                        {
                            self.error = Some(ScanError {
                                message: "double-quoted scalar continuation below block indent"
                                    .to_string(),
                            });
                            return;
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

        // Enforce max_scalar_bytes limit.
        if let Err(msg) = self.config.check_scalar_size(data.len()) {
            self.error = Some(ScanError { message: msg });
            return;
        }

        self.push_quoted_scalar(data, ScalarStyle::DoubleQuoted, start_mark, end_mark);
    }

    /// Read `n` hex digits and return the corresponding Unicode character.
    // y[impl char.ns-hex-digit]
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

    /// Push a quoted scalar to the queue and save a simple key mark.
    fn push_quoted_scalar(
        &mut self,
        data: Cow<'input, str>,
        style: ScalarStyle,
        start_mark: yamalgam_core::Mark,
        end_mark: yamalgam_core::Mark,
    ) {
        let scalar = Self::scalar_token(data, style, start_mark, end_mark);
        let id = self.enqueue(scalar);
        self.save_simple_key_ext(id, start_mark, true);
        // Multi-line quoted scalars consume newlines internally.
        self.simple_key_allowed = self.flow_level == 0 && end_mark.line > start_mark.line;
        // In flow context, `:` immediately after a quoted scalar is a
        // value indicator (JSON-compatible adjacent values).
        if self.flow_level > 0 {
            self.adjacent_value_offset = Some(self.reader.mark().offset);
        }
        // In block context, validate trailing content on the same line.
        // Only `:` (value indicator), `#` (comment preceded by whitespace),
        // newline, or EOF may follow a quoted scalar. Anything else is
        // invalid trailing content (e.g., `"quoted" trailing`).
        // cref: fy_reader_fetch_flow_scalar_handle (fy-reader.c)
        if self.flow_level == 0 {
            let mut lookahead = 0;
            while self.reader.peek_at(lookahead) == Some(' ')
                || self.reader.peek_at(lookahead) == Some('\t')
            {
                lookahead += 1;
            }
            match self.reader.peek_at(lookahead) {
                None | Some('\n' | '\r' | ':') => {}
                Some('#') if lookahead > 0 => {}
                Some(c) => {
                    self.error = Some(ScanError {
                        message: format!("invalid trailing content '{c}' after quoted scalar"),
                    });
                }
            }
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

    /// Validate that only whitespace and optional comment remain on the line.
    ///
    /// Used after document markers (`---`/`...`), directives, and block
    /// scalar headers. Sets `self.error` if invalid content is found.
    fn validate_line_tail(&mut self, context: &str) {
        let mut lookahead = 0;
        while self.reader.peek_at(lookahead) == Some(' ')
            || self.reader.peek_at(lookahead) == Some('\t')
        {
            lookahead += 1;
        }
        match self.reader.peek_at(lookahead) {
            None | Some('\n' | '\r') => {}
            Some('#') if lookahead > 0 => {}
            Some(c) => {
                self.error = Some(ScanError {
                    message: format!("invalid character '{c}' after {context}"),
                });
            }
        }
    }

    /// Check if the byte before the current reader position is whitespace.
    ///
    /// Returns `true` at offset 0 (start of input) or at column 0 (start of
    /// line), since both are valid positions for a comment `#`.
    fn preceded_by_whitespace(&self) -> bool {
        let offset = self.reader.mark().offset;
        if offset == 0 || self.reader.mark().column == 0 {
            return true;
        }
        let input = self.reader.input();
        matches!(
            input.as_bytes().get(offset - 1),
            Some(b' ' | b'\t' | b'\n' | b'\r')
        )
    }

    // -- Main fetch loop --

    /// Returns true if we need to fetch more tokens before yielding.
    ///
    /// When there are pending simple keys, we can't yield any tokens because
    /// the next token might be `:`, requiring retroactive insertion of
    /// `Key` + `BlockMappingStart` before the simple key token.
    // cref: fy_scan_token_needs_more (fy-parse.c)
    fn needs_more_tokens(&self) -> bool {
        if self.queue.is_empty() {
            return true;
        }
        !self.simple_keys.is_empty()
    }

    /// Fetch the next token from the stream.
    ///
    /// Skips whitespace/comments, manages indent levels, then checks for
    /// document indicators, directives, flow indicators, block indicators,
    /// and EOF. Unrecognized content is skipped.
    // cref: fy_fetch_tokens (fy-parse.c:5250)
    // y[impl char.c-indicator] — dispatch on all 22 indicator characters
    // y[impl char.c-reserved] — @ and ` fall through to plain scalar → error
    fn fetch_next_token(&mut self) -> Option<Result<Token<'input>, ScanError>> {
        loop {
            // Drain deferred error from a void fetch method.
            if let Some(err) = self.error.take() {
                self.state = State::Done;
                return Some(Err(err));
            }

            // Yield from queue only when all simple keys are resolved.
            if !self.needs_more_tokens()
                && let Some(token) = self.queue.pop_front()
            {
                self.tokens_consumed += 1;
                return Some(Ok(token));
            }

            self.scan_to_next_token();
            self.purge_stale_simple_keys();

            if self.reader.is_eof() {
                // DK95#3: tab consumed as whitespace at document level but
                // no content follows — reject. Tabs on blank lines are fine
                // because the newline clears the flag.
                if self.tab_in_preceding_whitespace
                    && self.indent == -1
                    && self.queue.is_empty()
                    && self.tokens_consumed <= 1
                {
                    self.error = Some(ScanError {
                        message: "tab character cannot be used as content".to_string(),
                    });
                    continue;
                }
                // 9MMA: directive-only stream — directives without a following document.
                if self.seen_directive && self.in_directive_prologue {
                    self.error = Some(ScanError {
                        message: "directives without a document".to_string(),
                    });
                    continue;
                }
                // All pending simple keys are unreachable at EOF.
                self.simple_keys.clear();
                self.unroll_indent(-1);
                let end = self.fetch_stream_end();
                self.enqueue(end);
                continue;
            }

            let c = self.reader.peek().unwrap();
            let col = self.reader.mark().column;

            // In block mode, unroll indent to current column.
            // BlockEnd tokens are enqueued and will be yielded naturally
            // once all simple keys are resolved.
            if self.flow_level == 0 {
                self.unroll_indent(col as i32);
            }

            // Document indicators at column 0 (unroll fully first).
            if col == 0 {
                if self.is_document_start() {
                    self.remove_simple_key();
                    self.unroll_indent(-1);
                    let token = self.fetch_document_indicator(TokenKind::DocumentStart);
                    self.enqueue(token);
                    self.simple_key_allowed = true;
                    // Note: `---` CAN be followed by content on the same line
                    // (e.g., `--- value`, `--- |`). No line-tail validation here.
                    continue;
                }
                if self.is_document_end() {
                    self.remove_simple_key();
                    self.unroll_indent(-1);
                    let token = self.fetch_document_indicator(TokenKind::DocumentEnd);
                    self.enqueue(token);
                    self.simple_key_allowed = true;
                    // Validate: only whitespace/comment after `...`.
                    // cref: YAML 1.2 §9.1.2 — c-document-end = "..."
                    self.validate_line_tail("document end marker '...'");
                    continue;
                }
                if c == '%' {
                    // 9HCY, EB22, RHX7: directives require `...` first if a
                    // document is open (explicit via `---` or implicit content).
                    if !self.in_directive_prologue {
                        self.error = Some(ScanError {
                            message: "directive without document end marker '...'".to_string(),
                        });
                        continue;
                    }
                    // Directives end any open block context.
                    self.remove_simple_key();
                    self.unroll_indent(-1);
                    match self.fetch_directive() {
                        Ok(token) => {
                            self.enqueue(token);
                            self.simple_key_allowed = true;
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

            // Any content beyond this point implicitly starts a document.
            // Clear tag handles for implicit documents (no directive prologue).
            if self.in_directive_prologue {
                self.in_directive_prologue = false;
                // Implicit document has no %TAG directives — clear handles.
                // (Explicit documents keep handles registered before `---`.)
                self.tag_handles.clear();
            }

            // BS4K, KS4U: at root level (indent -1, flow_level 0), only one
            // root node is allowed per document. After a standalone scalar or
            // flow collection, reject further content.
            if self.root_token_emitted && self.indent == -1 && self.flow_level == 0 {
                self.error = Some(ScanError {
                    message: "extra content after document root node".to_string(),
                });
                continue;
            }

            // Flow collection indicators.
            // cref: fy_fetch_tokens (fy-parse.c:5364-5394)
            if c == '[' || c == '{' {
                self.fetch_flow_collection_start(c);
                continue;
            }
            if c == ']' || c == '}' {
                self.fetch_flow_collection_end(c);
                continue;
            }
            if c == ',' {
                self.fetch_flow_entry();
                continue;
            }

            // Block entry (`-`) is only valid in block context.
            // cref: fy_fetch_tokens (fy-parse.c:5396)
            if self.flow_level == 0 && c == '-' && is_blank_or_end(self.reader.peek_at(1)) {
                self.fetch_block_entry();
                continue;
            }

            // Explicit key (`?`) is valid in both block and flow context.
            // cref: fy_fetch_tokens (fy-parse.c:5410)
            if c == '?' && is_blank_or_end(self.reader.peek_at(1)) {
                self.fetch_key();
                continue;
            }

            // Value indicator `:` — in both block and flow context.
            // In block context: `:` must be followed by blank/EOF.
            // In flow context: `:` before a flow indicator doesn't need
            // a trailing blank. After a JSON-like key (quoted scalar or
            // flow collection), `:` is a value indicator regardless of
            // what follows, even across line breaks (YAML §7.4.2 [153]).
            // cref: fy_fetch_tokens (fy-parse.c:5426)
            if c == ':'
                && (is_blank_or_end(self.reader.peek_at(1))
                    || (self.flow_level > 0
                        && (matches!(self.reader.peek_at(1), Some(',' | '[' | ']' | '{' | '}'))
                            || self
                                .adjacent_value_offset
                                .is_some_and(|off| off == self.reader.mark().offset)
                            || self
                                .simple_keys
                                .last()
                                .is_some_and(|s| s.flow_level == self.flow_level && s.json_key))))
            {
                self.adjacent_value_offset = None;
                self.fetch_value();
                continue;
            }

            // Tags.
            // cref: fy_fetch_tokens (fy-parse.c:5457)
            if c == '!' {
                self.fetch_tag();
                continue;
            }

            // Anchors and aliases.
            // cref: fy_fetch_tokens (fy-parse.c:5443)
            if c == '&' || c == '*' {
                self.fetch_anchor_or_alias(c);
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

            // Everything else is a plain scalar.
            // cref: fy_fetch_plain_scalar (fy-parse.c:5496)
            self.fetch_plain_scalar();
        }
    }
}

/// Returns `true` if the character is a blank (space or tab) or absent (EOF/newline).
const fn is_blank_or_end(c: Option<char>) -> bool {
    matches!(c, None | Some(' ' | '\t' | '\n' | '\r'))
}

/// Decode URI percent-encoding in a tag string (YAML §6.9.1).
///
/// `%XX` sequences (two hex digits) are decoded to the corresponding byte.
/// Multi-byte UTF-8 sequences like `%C3%BC` are decoded correctly.
/// Invalid sequences are left as-is.
// cref: fy_tag_scan (fy-parse.c:2210)
// y[impl struct.global-tag-prefix.same-semantics] — URI percent-encoding decoded consistently
fn decode_tag_uri(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded_bytes = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex_digit(bytes[i + 1]), hex_digit(bytes[i + 2]))
        {
            decoded_bytes.push(hi << 4 | lo);
            i += 3;
            continue;
        }
        decoded_bytes.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(decoded_bytes).unwrap_or_else(|_| input.to_string())
}

// y[impl char.ns-dec-digit] — decimal digits 0-9 used in indent indicators and escape sequences
// y[impl char.ns-ascii-letter] — ASCII letters used in directive names and tag handles
const fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Returns `true` if the character is a blank (space or tab).
const fn is_blank(c: Option<char>) -> bool {
    matches!(c, Some(' ' | '\t'))
}

/// Returns `true` if the character is a YAML line break.
const fn is_linebreak(c: char) -> bool {
    c == '\n' || c == '\r'
}

/// Fold a block scalar per YAML 1.2 §6.5 (Line Folding) and §8.2.1.
///
/// Separator rules between consecutive lines:
/// - Empty/MI prev → anything: preserve `\n` (these breaks are always kept)
/// - Content prev → empty cur: check lookahead —
///   if next non-empty line is more-indented, preserve `\n` (§8.2.1);
///   otherwise trim/discard (§6.5: "the first line break is discarded")
/// - Content prev → MI cur: preserve `\n` (folding doesn't apply around MI)
/// - Content prev → content cur: fold to space
// cref: fy_atom_format_text_block (fy-atom.c) — folded mode
// y[impl struct.b-l-folded] — line folding: single break → space, empty lines → newlines
// y[impl struct.b-l-trimmed] — trimming trailing whitespace before folding
// y[impl struct.b-as-space] — single line break folded to space
// y[impl struct.l-empty] — empty lines preserved as literal newlines
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
            if prev.is_empty() || prev.starts_with([' ', '\t']) {
                // Empty/MI line's break is always preserved.
                result.push('\n');
            } else if line.is_empty() {
                // Content → empty: trim unless next non-empty is more-indented.
                // §6.5: "the first line break is discarded and the rest are retained."
                // §8.2.1: "folding does not apply to line breaks surrounding [MI] lines."
                let next_is_mi = lines[i + 1..line_count]
                    .iter()
                    .find(|l| !l.is_empty())
                    .is_some_and(|l| l.starts_with([' ', '\t']));
                if next_is_mi {
                    result.push('\n');
                }
                // else: trim (discard content's line break)
            } else if line.starts_with([' ', '\t']) {
                // Content → more-indented: preserve.
                result.push('\n');
            } else {
                // Content → content: fold to space.
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
            self.tokens_consumed += 1;
            return Some(Ok(token));
        }
        match self.state {
            State::Start => Some(Ok(self.fetch_stream_start())),
            State::Stream => self.fetch_next_token(),
            State::Done => None,
        }
    }
}

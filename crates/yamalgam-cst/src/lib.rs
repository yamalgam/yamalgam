//! Concrete Syntax Tree — lossless representation of YAML source.
//!
//! The CST preserves every byte of the original input: whitespace, comments,
//! quoting style, indicators. Concatenating all leaf token texts in tree order
//! reproduces the source byte-for-byte.
//!
//! Build a CST from events via [`CstBuilder`], or use [`parse_to_cst`] for
//! the one-call path.
//!
//! ```
//! use yamalgam_cst::parse_to_cst;
//!
//! let input = "key: value # comment\n";
//! let cst = parse_to_cst(input);
//! assert_eq!(cst.to_text(), input);
//! ```

use std::borrow::Cow;
use std::fmt;

use yamalgam_core::{LoaderConfig, ResourceLimits, Span};

use yamalgam_parser::Event;
use yamalgam_parser::ParseError;
use yamalgam_parser::Parser;

// ---------------------------------------------------------------------------
// Node kinds
// ---------------------------------------------------------------------------

/// Interior node kinds in the CST.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// Root of the CST (one per input).
    Stream,
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
    /// Error recovery — wraps unparseable content.
    Error,
}

/// Leaf token kinds in the CST.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    // -- Indicators --
    /// `-` block sequence entry indicator.
    BlockEntry,
    /// `?` key indicator.
    Key,
    /// `:` value indicator.
    Value,
    /// `---` document start marker.
    DocumentStart,
    /// `...` document end marker.
    DocumentEnd,
    /// `[` flow sequence start.
    FlowSeqStart,
    /// `]` flow sequence end.
    FlowSeqEnd,
    /// `{` flow mapping start.
    FlowMapStart,
    /// `}` flow mapping end.
    FlowMapEnd,
    /// `,` flow entry separator.
    FlowEntry,

    // -- Content --
    /// Scalar text (plain, quoted, or block).
    Scalar,
    /// `&name` anchor.
    Anchor,
    /// `*name` alias reference.
    Alias,
    /// Tag (`!tag`, `!!tag`, `!prefix!suffix`).
    Tag,

    // -- Directives --
    /// `%YAML x.y` version directive.
    VersionDirective,
    /// `%TAG !handle! prefix` tag directive.
    TagDirective,

    // -- Trivia --
    /// `# text` comment.
    Comment,
    /// Whitespace (spaces, tabs, newlines) between tokens.
    Whitespace,
}

// ---------------------------------------------------------------------------
// CST nodes
// ---------------------------------------------------------------------------

/// An interior node in the concrete syntax tree.
#[derive(Clone, Debug)]
pub struct CstNode<'input> {
    /// What kind of syntax construct this node represents.
    pub kind: NodeKind,
    /// Source span (union of all children spans).
    pub span: Span,
    /// Ordered children (nodes and tokens in source order).
    pub children: Vec<CstElement<'input>>,
}

/// A leaf token in the concrete syntax tree.
#[derive(Clone, Debug)]
pub struct CstToken<'input> {
    /// What kind of token this is.
    pub kind: TokenKind,
    /// The original source text for this token.
    pub text: Cow<'input, str>,
    /// Exact source span.
    pub span: Span,
}

/// A child element in the CST — either an interior node or a leaf token.
#[derive(Clone, Debug)]
pub enum CstElement<'input> {
    /// An interior node with children.
    Node(CstNode<'input>),
    /// A leaf token with text.
    Token(CstToken<'input>),
}

// ---------------------------------------------------------------------------
// CstNode API
// ---------------------------------------------------------------------------

impl<'input> CstNode<'input> {
    /// Concatenate all leaf token texts in tree order.
    ///
    /// This reproduces the original source byte-for-byte (round-trip property).
    pub fn to_text(&self) -> String {
        let mut buf = String::new();
        let mut pending: Vec<_> = self.children.iter().rev().collect();
        while let Some(element) = pending.pop() {
            match element {
                CstElement::Node(node) => pending.extend(node.children.iter().rev()),
                CstElement::Token(token) => buf.push_str(&token.text),
            }
        }
        buf
    }

    /// Iterate all direct children.
    pub fn children(&self) -> &[CstElement<'input>] {
        &self.children
    }

    /// Find first child node of a given kind.
    pub fn child_node(&self, kind: NodeKind) -> Option<&Self> {
        self.children.iter().find_map(|c| match c {
            CstElement::Node(n) if n.kind == kind => Some(n),
            _ => None,
        })
    }

    /// Iterate all child tokens of a given kind.
    pub fn child_tokens(&self, kind: TokenKind) -> impl Iterator<Item = &CstToken<'input>> {
        self.children.iter().filter_map(move |c| match c {
            CstElement::Token(t) if t.kind == kind => Some(t),
            _ => None,
        })
    }

    /// Find the deepest node containing a given byte offset.
    pub fn node_at_offset(&self, offset: usize) -> Option<&Self> {
        if offset < self.span.start.offset || offset >= self.span.end.offset {
            return None;
        }

        let mut current = self;
        loop {
            let next = current.children.iter().find_map(|child| match child {
                CstElement::Node(node)
                    if offset >= node.span.start.offset && offset < node.span.end.offset =>
                {
                    Some(node)
                }
                _ => None,
            });
            match next {
                Some(node) => current = node,
                None => return Some(current),
            }
        }
    }

    /// Count all leaf tokens in the tree (for testing).
    pub fn leaf_count(&self) -> usize {
        let mut count = 0;
        let mut pending: Vec<_> = self.children.iter().rev().collect();
        while let Some(element) = pending.pop() {
            match element {
                CstElement::Node(node) => pending.extend(node.children.iter().rev()),
                CstElement::Token(_) => count += 1,
            }
        }
        count
    }
}

impl Drop for CstNode<'_> {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        pending.append(&mut self.children);
        while let Some(element) = pending.pop() {
            if let CstElement::Node(mut node) = element {
                pending.append(&mut node.children);
            }
        }
    }
}

impl fmt::Display for CstNode<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_text())
    }
}

// ---------------------------------------------------------------------------
// CstBuilder
// ---------------------------------------------------------------------------

/// Builds a [`CstNode`] tree from a parser event stream.
///
/// The builder holds a reference to the original source text so it can
/// insert whitespace tokens for the gaps between event spans.
pub struct CstBuilder<'input> {
    source: &'input str,
    stack: Vec<CstNode<'input>>,
    last_offset: usize,
    limits: ResourceLimits,
    collection_depth: usize,
}

impl<'input> CstBuilder<'input> {
    /// Create a new builder for the given source text with moderate resource
    /// limits.
    #[must_use]
    pub const fn new(source: &'input str) -> Self {
        Self {
            source,
            stack: Vec::new(),
            last_offset: 0,
            limits: ResourceLimits::moderate(),
            collection_depth: 0,
        }
    }

    /// Create a new builder without resource limits.
    ///
    /// Use only for trusted input. Prefer [`Self::new`] for ordinary input.
    #[must_use]
    pub const fn new_unchecked(source: &'input str) -> Self {
        Self {
            source,
            stack: Vec::new(),
            last_offset: 0,
            limits: ResourceLimits::none(),
            collection_depth: 0,
        }
    }

    /// Create a new builder with caller-selected resource limits.
    #[must_use]
    pub fn new_with_config(source: &'input str, config: &LoaderConfig) -> Self {
        Self {
            source,
            stack: Vec::new(),
            last_offset: 0,
            limits: config.limits.clone(),
            collection_depth: 0,
        }
    }

    /// Consume an event stream and build the CST.
    ///
    /// On parse errors, wraps remaining source in an error node.
    pub fn build(
        mut self,
        events: impl Iterator<Item = Result<Event<'input>, ParseError>>,
    ) -> CstNode<'input> {
        for result in events {
            match result {
                Ok(event) => {
                    if !self.process_event(event) {
                        break;
                    }
                }
                Err(e) => {
                    self.handle_error(e);
                    break;
                }
            }
        }

        // Insert trailing whitespace.
        if self.last_offset < self.source.len() {
            let ws = &self.source[self.last_offset..];
            if !ws.is_empty() {
                self.push_token(
                    TokenKind::Whitespace,
                    Cow::Borrowed(ws),
                    Span {
                        start: yamalgam_core::Mark {
                            offset: self.last_offset,
                            ..Default::default()
                        },
                        end: yamalgam_core::Mark {
                            offset: self.source.len(),
                            ..Default::default()
                        },
                    },
                );
            }
        }

        // Close remaining open nodes.
        while self.stack.len() > 1 {
            self.close_node();
        }

        self.stack.pop().unwrap_or_else(|| CstNode {
            kind: NodeKind::Stream,
            span: Span::default(),
            children: Vec::new(),
        })
    }

    fn process_event(&mut self, event: Event<'input>) -> bool {
        match event {
            Event::StreamStart => {
                self.open_node(NodeKind::Stream, Span::default());
            }
            Event::StreamEnd => {
                // StreamEnd has no span text — just close Stream.
                self.close_to(NodeKind::Stream);
            }
            Event::VersionDirective { span, .. } => {
                self.insert_whitespace_before(span);
                let text = self.source_text(span);
                self.push_token(TokenKind::VersionDirective, text, span);
            }
            Event::TagDirective { span, .. } => {
                self.insert_whitespace_before(span);
                let text = self.source_text(span);
                self.push_token(TokenKind::TagDirective, text, span);
            }
            Event::DocumentStart { implicit, span } => {
                self.open_node(NodeKind::Document, span);
                if !implicit {
                    self.insert_whitespace_before(span);
                    self.push_source_token(TokenKind::DocumentStart, span);
                }
            }
            Event::DocumentEnd { implicit, span } => {
                // Close any open entries before document end.
                self.maybe_close(NodeKind::MappingEntry);
                self.maybe_close(NodeKind::SequenceEntry);
                if !implicit {
                    self.insert_whitespace_before(span);
                    self.push_source_token(TokenKind::DocumentEnd, span);
                }
                self.close_to(NodeKind::Document);
            }
            Event::SequenceStart { span, .. } => {
                if !self.open_collection(NodeKind::Sequence, span) {
                    return false;
                }
                self.push_source_token(TokenKind::FlowSeqStart, span);
            }
            Event::SequenceEnd { span } => {
                self.maybe_close(NodeKind::SequenceEntry);
                self.insert_whitespace_before(span);
                self.push_source_token(TokenKind::FlowSeqEnd, span);
                self.close_to(NodeKind::Sequence);
            }
            Event::MappingStart { span, .. } => {
                if !self.open_collection(NodeKind::Mapping, span) {
                    return false;
                }
                self.push_source_token(TokenKind::FlowMapStart, span);
            }
            Event::MappingEnd { span } => {
                self.maybe_close(NodeKind::MappingEntry);
                self.insert_whitespace_before(span);
                self.push_source_token(TokenKind::FlowMapEnd, span);
                self.close_to(NodeKind::Mapping);
            }
            Event::BlockEntry { span } => {
                // Close previous SequenceEntry if open.
                self.maybe_close(NodeKind::SequenceEntry);
                self.insert_whitespace_before(span);
                self.open_node(NodeKind::SequenceEntry, span);
                self.push_source_token(TokenKind::BlockEntry, span);
            }
            Event::KeyIndicator { span } => {
                // Close previous MappingEntry if open.
                self.maybe_close(NodeKind::MappingEntry);
                self.insert_whitespace_before(span);
                self.open_node(NodeKind::MappingEntry, span);
                self.push_source_token(TokenKind::Key, span);
            }
            Event::ValueIndicator { span } => {
                self.insert_whitespace_before(span);
                self.push_source_token(TokenKind::Value, span);
            }
            Event::Scalar {
                span, value, style, ..
            } => {
                // Implicit empty scalars (plain style, empty value) occupy no source
                // bytes. Their span borrows the next token's position — skip to avoid
                // extracting the wrong source text.
                if value.is_empty() && style == yamalgam_scanner::ScalarStyle::Plain {
                    return true;
                }
                self.insert_whitespace_before(span);
                // Use source text for round-trip fidelity, not the processed value.
                let text = self.source_text(span);
                self.push_token(TokenKind::Scalar, text, span);
            }
            Event::Alias { span, .. } => {
                self.insert_whitespace_before(span);
                let text = self.source_text(span);
                self.push_token(TokenKind::Alias, text, span);
            }
            Event::Comment { text, span } => {
                // Comments may arrive out of source order (after MappingEnd/SequenceEnd).
                // If the source region is already covered, skip — it's in a whitespace gap.
                if span.start.offset < self.last_offset {
                    return true;
                }
                self.insert_whitespace_before(span);
                self.push_token(TokenKind::Comment, text, span);
            }
        }
        true
    }

    // -- Helpers --

    /// Open a collection if doing so stays within the configured depth.
    fn open_collection(&mut self, kind: NodeKind, span: Span) -> bool {
        let Some(depth) = self.collection_depth.checked_add(1) else {
            self.handle_error(ParseError::LimitExceeded("depth overflow".into()));
            return false;
        };
        if let Err(message) = self.limits.check_depth(depth) {
            self.handle_error(ParseError::LimitExceeded(message));
            return false;
        }
        self.insert_whitespace_before(span);
        self.open_node(kind, span);
        self.collection_depth = depth;
        true
    }

    /// Open a new interior node on the stack.
    fn open_node(&mut self, kind: NodeKind, span: Span) {
        self.stack.push(CstNode {
            kind,
            span,
            children: Vec::new(),
        });
    }

    /// Pop the top node and add it as a child of the new top.
    fn close_node(&mut self) {
        if let Some(mut node) = self.stack.pop() {
            let is_collection = matches!(node.kind, NodeKind::Mapping | NodeKind::Sequence);
            // Update span to cover all children.
            if let Some(last) = node.children.last() {
                node.span.end = match last {
                    CstElement::Token(t) => t.span.end,
                    CstElement::Node(n) => n.span.end,
                };
            }
            let attached = if let Some(parent) = self.stack.last_mut() {
                parent.children.push(CstElement::Node(node));
                true
            } else {
                // No parent — push back (root node).
                self.stack.push(node);
                false
            };
            if attached && is_collection {
                self.collection_depth = self.collection_depth.saturating_sub(1);
            }
        }
    }

    /// Close nodes until we reach one of the given kind, then close it too.
    fn close_to(&mut self, kind: NodeKind) {
        // Close any children first (e.g., open SequenceEntry inside Sequence).
        while self.stack.len() > 1 {
            if self.stack.last().map(|n| n.kind) == Some(kind) {
                break;
            }
            self.close_node();
        }
        // Close the target node itself.
        if self.stack.last().map(|n| n.kind) == Some(kind) {
            self.close_node();
        }
    }

    /// Close the top node if it matches the given kind.
    fn maybe_close(&mut self, kind: NodeKind) {
        if self.stack.last().map(|n| n.kind) == Some(kind) {
            self.close_node();
        }
    }

    /// Add a leaf token to the current top-of-stack node.
    fn push_token(&mut self, kind: TokenKind, text: Cow<'input, str>, span: Span) {
        // Only advance offset for real (non-virtual) spans.
        if span.start.offset != span.end.offset && span.end.offset > self.last_offset {
            self.last_offset = span.end.offset;
        }
        let token = CstToken { kind, text, span };
        if let Some(parent) = self.stack.last_mut() {
            parent.children.push(CstElement::Token(token));
        }
    }

    /// Add a token whose text comes directly from the source at the given span.
    ///
    /// Skips zero-length spans (virtual tokens like implicit Key indicators).
    fn push_source_token(&mut self, kind: TokenKind, span: Span) {
        if span.start.offset == span.end.offset {
            return;
        }
        let text = self.source_text(span);
        self.push_token(kind, text, span);
    }

    /// Extract source text for a span.
    fn source_text(&self, span: Span) -> Cow<'input, str> {
        let start = span.start.offset;
        let end = span.end.offset.min(self.source.len());
        if start < end {
            Cow::Borrowed(&self.source[start..end])
        } else {
            Cow::Borrowed("")
        }
    }

    /// Insert a whitespace token for the gap between last_offset and span start.
    ///
    /// Skips virtual events (zero-length spans like BlockEnd, StreamEnd)
    /// which don't correspond to source text.
    fn insert_whitespace_before(&mut self, span: Span) {
        // Zero-length spans are virtual (e.g., BlockEnd, implicit DocumentStart).
        if span.start.offset == span.end.offset {
            return;
        }
        if span.start.offset > self.last_offset {
            let ws_start = self.last_offset;
            let ws_end = span.start.offset.min(self.source.len());
            if ws_start >= ws_end {
                return;
            }
            let ws_text = &self.source[ws_start..ws_end];
            if !ws_text.is_empty() {
                let ws_span = Span {
                    start: yamalgam_core::Mark {
                        offset: ws_start,
                        ..Default::default()
                    },
                    end: yamalgam_core::Mark {
                        offset: ws_end,
                        ..Default::default()
                    },
                };
                // Don't update last_offset through push_token — do it manually.
                let token = CstToken {
                    kind: TokenKind::Whitespace,
                    text: Cow::Borrowed(ws_text),
                    span: ws_span,
                };
                if let Some(parent) = self.stack.last_mut() {
                    parent.children.push(CstElement::Token(token));
                }
                self.last_offset = ws_end;
            }
        }
    }

    /// Handle a parse error by wrapping remaining source in an error node.
    fn handle_error(&mut self, error: ParseError) {
        if self.last_offset < self.source.len() {
            let remaining = &self.source[self.last_offset..];
            let err_span = Span {
                start: yamalgam_core::Mark {
                    offset: self.last_offset,
                    ..Default::default()
                },
                end: yamalgam_core::Mark {
                    offset: self.source.len(),
                    ..Default::default()
                },
            };
            let err_node = CstNode {
                kind: NodeKind::Error,
                span: err_span,
                children: vec![CstElement::Token(CstToken {
                    kind: TokenKind::Whitespace, // best-effort — error content
                    text: Cow::Borrowed(remaining),
                    span: err_span,
                })],
            };
            if let Some(parent) = self.stack.last_mut() {
                parent.children.push(CstElement::Node(err_node));
            }
            self.last_offset = self.source.len();
        }
        let _ = error; // consumed
    }
}

// ---------------------------------------------------------------------------
// Convenience function
// ---------------------------------------------------------------------------

/// Parse a YAML string to a CST in one call.
///
/// This is the simplest entry point. The returned tree preserves every byte
/// of the input — `cst.to_text() == input` (round-trip property). Applies
/// [`LoaderConfig::default`] resource limits.
pub fn parse_to_cst(input: &str) -> CstNode<'_> {
    parse_to_cst_with_config(input, &LoaderConfig::default())
}

/// Parse trusted YAML to a CST without resource limits.
///
/// Prefer [`parse_to_cst`] for ordinary input.
pub fn parse_to_cst_unchecked(input: &str) -> CstNode<'_> {
    parse_to_cst_with_config(input, &LoaderConfig::unchecked())
}

/// Parse a YAML string to a CST with caller-selected loader configuration.
///
/// The returned tree preserves every byte of the input, including when a
/// resource limit stops parsing.
pub fn parse_to_cst_with_config<'input>(
    input: &'input str,
    config: &LoaderConfig,
) -> CstNode<'input> {
    let parser = Parser::with_config(input, config);
    CstBuilder::new_with_config(input, config).build(parser)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const DEEP_CST_DROP_WORKER: &str = "YAMALGAM_DEEP_CST_DROP_WORKER";
    const DEEP_CST_TRAVERSAL_WORKER: &str = "YAMALGAM_DEEP_CST_TRAVERSAL_WORKER";

    fn nested_flow_sequence(depth: usize) -> String {
        format!("{}0{}", "[".repeat(depth), "]".repeat(depth))
    }

    fn contains_node_kind(root: &CstNode<'_>, kind: NodeKind) -> bool {
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            if node.kind == kind {
                return true;
            }
            pending.extend(node.children.iter().rev().filter_map(|child| match child {
                CstElement::Node(child) => Some(child),
                CstElement::Token(_) => None,
            }));
        }
        false
    }

    fn depth_config(max_depth: usize) -> LoaderConfig {
        LoaderConfig {
            limits: ResourceLimits {
                max_depth: Some(max_depth),
                ..ResourceLimits::none()
            },
            ..LoaderConfig::unchecked()
        }
    }

    fn deeply_nested_public_cst(depth: usize) -> CstNode<'static> {
        let span = Span {
            start: yamalgam_core::Mark::default(),
            end: yamalgam_core::Mark {
                offset: 1,
                ..Default::default()
            },
        };
        let mut node = CstNode {
            kind: NodeKind::Document,
            span,
            children: vec![CstElement::Token(CstToken {
                kind: TokenKind::Scalar,
                text: Cow::Borrowed("x"),
                span,
            })],
        };
        for _ in 0..depth {
            node = CstNode {
                kind: NodeKind::Sequence,
                span,
                children: vec![CstElement::Node(node)],
            };
        }
        node
    }

    fn assert_deep_worker_succeeds(environment: &str, test_name: &str) {
        let output = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .env(environment, "1")
            .args(["--exact", test_name, "--nocapture"])
            .output()
            .expect("run deep CST worker");
        assert!(
            output.status.success(),
            "deep CST worker failed: status={} stdout={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    #[test]
    fn deep_public_cst_traversals_are_stack_safe() {
        if std::env::var_os(DEEP_CST_TRAVERSAL_WORKER).is_some() {
            let node = deeply_nested_public_cst(100_000);
            assert_eq!(node.to_text(), "x");
            assert_eq!(node.to_string(), "x");
            assert_eq!(node.leaf_count(), 1);
            let deepest = node.node_at_offset(0).expect("offset in tree");
            assert!(matches!(
                deepest.children.as_slice(),
                [CstElement::Token(_)]
            ));
            std::mem::forget(node);
            return;
        }

        assert_deep_worker_succeeds(
            DEEP_CST_TRAVERSAL_WORKER,
            "tests::deep_public_cst_traversals_are_stack_safe",
        );
    }

    #[test]
    fn deep_public_cst_drop_is_stack_safe() {
        if std::env::var_os(DEEP_CST_DROP_WORKER).is_some() {
            drop(deeply_nested_public_cst(100_000));
            return;
        }

        assert_deep_worker_succeeds(
            DEEP_CST_DROP_WORKER,
            "tests::deep_public_cst_drop_is_stack_safe",
        );
    }

    #[test]
    fn ordinary_cst_entry_point_uses_moderate_limits() {
        let input = nested_flow_sequence(257);
        let cst = parse_to_cst(&input);

        assert_eq!(cst.to_text(), input);
        assert!(contains_node_kind(&cst, NodeKind::Error));
    }

    #[test]
    fn unchecked_cst_entry_points_preserve_unlimited_behavior() {
        let input = nested_flow_sequence(257);

        let parsed = parse_to_cst_unchecked(&input);
        assert_eq!(parsed.to_text(), input);
        assert!(!contains_node_kind(&parsed, NodeKind::Error));

        let built = CstBuilder::new_unchecked(&input).build(Parser::new(&input));
        assert_eq!(built.to_text(), input);
        assert!(!contains_node_kind(&built, NodeKind::Error));
    }

    #[test]
    fn cst_builder_enforces_collection_depth_at_the_exact_boundary() {
        let config = depth_config(1);

        let exact_input = "[0]";
        let exact =
            CstBuilder::new_with_config(exact_input, &config).build(Parser::new(exact_input));
        assert_eq!(exact.to_text(), exact_input);
        assert!(!contains_node_kind(&exact, NodeKind::Error));

        let over_input = "[[0]]";
        let over = CstBuilder::new_with_config(over_input, &config).build(Parser::new(over_input));
        assert_eq!(over.to_text(), over_input);
        assert!(contains_node_kind(&over, NodeKind::Error));
    }

    #[test]
    fn empty_input_round_trip() {
        let input = "";
        let cst = parse_to_cst(input);
        assert_eq!(cst.to_text(), input);
        assert_eq!(cst.kind, NodeKind::Stream);
    }

    #[test]
    fn plain_scalar_round_trip() {
        let input = "hello";
        let cst = parse_to_cst(input);
        assert_eq!(cst.to_text(), input);
    }

    #[test]
    fn block_sequence_round_trip() {
        let input = "- a\n- b\n- c\n";
        let cst = parse_to_cst(input);
        assert_eq!(cst.to_text(), input);
    }

    #[test]
    fn block_mapping_round_trip() {
        let input = "key: value\nother: stuff\n";
        let cst = parse_to_cst(input);
        assert_eq!(cst.to_text(), input);
    }

    #[test]
    fn flow_sequence_round_trip() {
        let input = "[1, 2, 3]";
        let cst = parse_to_cst(input);
        assert_eq!(cst.to_text(), input);
    }

    #[test]
    fn flow_mapping_round_trip() {
        let input = "{a: 1, b: 2}";
        let cst = parse_to_cst(input);
        assert_eq!(cst.to_text(), input);
    }

    #[test]
    fn comment_preserved() {
        let input = "# comment\nkey: value\n";
        let cst = parse_to_cst(input);
        assert_eq!(cst.to_text(), input);
    }

    #[test]
    fn inline_comment_preserved() {
        let input = "key: value # inline\n";
        let cst = parse_to_cst(input);
        assert_eq!(cst.to_text(), input);
    }

    #[test]
    fn explicit_document_round_trip() {
        let input = "---\nfoo: bar\n...\n";
        let cst = parse_to_cst(input);
        assert_eq!(cst.to_text(), input);
    }

    #[test]
    fn nested_structure_round_trip() {
        let input = "outer:\n  inner:\n    - a\n    - b\n";
        let cst = parse_to_cst(input);
        assert_eq!(cst.to_text(), input);
    }

    #[test]
    fn quoted_scalar_round_trip() {
        let input = "'single quoted'\n";
        let cst = parse_to_cst(input);
        assert_eq!(cst.to_text(), input);
    }

    #[test]
    fn double_quoted_scalar_round_trip() {
        let input = "\"double quoted\"\n";
        let cst = parse_to_cst(input);
        assert_eq!(cst.to_text(), input);
    }

    #[test]
    fn stream_node_structure() {
        let cst = parse_to_cst("key: val");
        assert_eq!(cst.kind, NodeKind::Stream);
        // Stream should contain a Document.
        assert!(cst.child_node(NodeKind::Document).is_some());
    }

    #[test]
    fn document_contains_mapping() {
        let cst = parse_to_cst("a: 1\nb: 2");
        let doc = cst.child_node(NodeKind::Document).expect("no document");
        assert!(doc.child_node(NodeKind::Mapping).is_some());
    }

    #[test]
    fn mapping_contains_entries() {
        let cst = parse_to_cst("a: 1\nb: 2");
        let doc = cst.child_node(NodeKind::Document).expect("no document");
        let mapping = doc.child_node(NodeKind::Mapping).expect("no mapping");
        let entry_count = mapping
            .children()
            .iter()
            .filter(|c| matches!(c, CstElement::Node(n) if n.kind == NodeKind::MappingEntry))
            .count();
        assert_eq!(entry_count, 2, "expected 2 mapping entries");
    }

    #[test]
    fn sequence_contains_entries() {
        let cst = parse_to_cst("- a\n- b\n- c");
        let doc = cst.child_node(NodeKind::Document).expect("no document");
        let seq = doc.child_node(NodeKind::Sequence).expect("no sequence");
        let entry_count = seq
            .children()
            .iter()
            .filter(|c| matches!(c, CstElement::Node(n) if n.kind == NodeKind::SequenceEntry))
            .count();
        assert_eq!(entry_count, 3, "expected 3 sequence entries");
    }

    #[test]
    fn node_at_offset_finds_scalar() {
        let input = "key: value";
        let cst = parse_to_cst(input);
        // "value" starts at offset 5.
        let node = cst.node_at_offset(5);
        assert!(node.is_some());
    }

    #[test]
    fn error_recovery_produces_partial_cst() {
        let input = "'unterminated";
        let cst = parse_to_cst(input);
        // Should still produce a tree (with error node).
        assert_eq!(cst.kind, NodeKind::Stream);
        assert_eq!(cst.to_text(), input);
    }

    #[test]
    fn anchor_and_alias_round_trip() {
        let input = "a: &ref hello\nb: *ref\n";
        let cst = parse_to_cst(input);
        assert_eq!(cst.to_text(), input);
    }

    #[test]
    fn multiline_block_scalar_round_trip() {
        let input = "text: |\n  line1\n  line2\n";
        let cst = parse_to_cst(input);
        assert_eq!(cst.to_text(), input);
    }

    #[test]
    fn leaf_count_nonzero() {
        let cst = parse_to_cst("a: 1");
        assert!(cst.leaf_count() > 0);
    }
}

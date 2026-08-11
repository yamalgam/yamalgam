//! Owned Rowan syntax trees projected from Yamalgam's lossless CST.

use rowan::{GreenNode, GreenNodeBuilder, Language, TextSize, TokenAtOffset};
use yamalgam_core::LoaderConfig;
use yamalgam_parser::ParseError;

use crate::{
    CstElement, CstNode, NodeKind, RecoveryOptions, RecoveryReport, TokenKind,
    parse_to_cst_recovering_with_config,
};

/// Unified node and token kinds stored in the Rowan tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    /// Complete YAML stream.
    Stream,
    /// One YAML document.
    Document,
    /// Block or flow mapping.
    Mapping,
    /// Mapping key-value entry.
    MappingEntry,
    /// Block or flow sequence.
    Sequence,
    /// Sequence entry.
    SequenceEntry,
    /// Unparseable but preserved source.
    Error,
    /// Scalar token.
    Scalar,
    /// Anchor token.
    Anchor,
    /// Alias token.
    Alias,
    /// Tag token.
    Tag,
    /// Comment token.
    Comment,
    /// Whitespace token.
    Whitespace,
    /// YAML punctuation or directive token.
    Indicator,
}

/// Rowan language marker for YAML syntax.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum YamlLanguage {}

impl Language for YamlLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        match raw.0 {
            0 => SyntaxKind::Stream,
            1 => SyntaxKind::Document,
            2 => SyntaxKind::Mapping,
            3 => SyntaxKind::MappingEntry,
            4 => SyntaxKind::Sequence,
            5 => SyntaxKind::SequenceEntry,
            6 => SyntaxKind::Error,
            7 => SyntaxKind::Scalar,
            8 => SyntaxKind::Anchor,
            9 => SyntaxKind::Alias,
            10 => SyntaxKind::Tag,
            11 => SyntaxKind::Comment,
            12 => SyntaxKind::Whitespace,
            13 => SyntaxKind::Indicator,
            value => panic!("invalid YAML syntax kind {value}"),
        }
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}

/// Owned Rowan syntax node.
pub type SyntaxNode = rowan::SyntaxNode<YamlLanguage>;
/// Owned Rowan syntax token.
pub type SyntaxToken = rowan::SyntaxToken<YamlLanguage>;
/// Owned Rowan syntax element.
pub type SyntaxElement = rowan::SyntaxElement<YamlLanguage>;

/// Immutable green syntax tree and its parser diagnostics.
#[derive(Clone, Debug)]
pub struct SyntaxTree {
    green: GreenNode,
    diagnostics: Vec<ParseError>,
}

impl SyntaxTree {
    /// Parse a YAML stream with default limits and bounded recovery.
    #[must_use]
    pub fn parse(input: &str) -> Self {
        Self::parse_with_config(input, &LoaderConfig::default())
    }

    /// Parse a YAML stream with caller-selected limits and bounded recovery.
    #[must_use]
    pub fn parse_with_config(input: &str, config: &LoaderConfig) -> Self {
        Self::from_recovery(parse_to_cst_recovering_with_config(
            input,
            config,
            RecoveryOptions::default(),
        ))
    }

    pub(crate) fn from_recovery(report: RecoveryReport<'_>) -> Self {
        let mut builder = GreenNodeBuilder::new();
        build_node(&mut builder, &report.cst);
        Self {
            green: builder.finish(),
            diagnostics: report.diagnostics,
        }
    }

    pub(crate) const fn from_parts(green: GreenNode, diagnostics: Vec<ParseError>) -> Self {
        Self { green, diagnostics }
    }

    /// Return the immutable green root.
    #[must_use]
    pub const fn green(&self) -> &GreenNode {
        &self.green
    }

    /// Return an owned red root for navigation.
    #[must_use]
    pub fn root(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }

    /// Reconstruct the original source text.
    #[must_use]
    pub fn text(&self) -> String {
        self.root().text().to_string()
    }

    /// Structured diagnostics retained by recovery.
    #[must_use]
    pub fn diagnostics(&self) -> &[ParseError] {
        &self.diagnostics
    }

    /// Return the deepest syntax node owning the token at a byte offset.
    #[must_use]
    pub fn node_at_offset(&self, offset: usize) -> Option<SyntaxNode> {
        if offset > usize::from(self.green.text_len()) {
            return None;
        }
        let offset = TextSize::try_from(offset).ok()?;
        let token = match self.root().token_at_offset(offset) {
            TokenAtOffset::None => return None,
            TokenAtOffset::Single(token) => token,
            TokenAtOffset::Between(_, right) => right,
        };
        token.parent()
    }
}

fn build_node(builder: &mut GreenNodeBuilder<'_>, node: &CstNode<'_>) {
    builder.start_node(YamlLanguage::kind_to_raw(node_kind(node.kind)));
    for child in &node.children {
        match child {
            CstElement::Node(child) => build_node(builder, child),
            CstElement::Token(token) => builder.token(
                YamlLanguage::kind_to_raw(token_kind(token.kind)),
                &token.text,
            ),
        }
    }
    builder.finish_node();
}

const fn node_kind(kind: NodeKind) -> SyntaxKind {
    match kind {
        NodeKind::Stream => SyntaxKind::Stream,
        NodeKind::Document => SyntaxKind::Document,
        NodeKind::Mapping => SyntaxKind::Mapping,
        NodeKind::MappingEntry => SyntaxKind::MappingEntry,
        NodeKind::Sequence => SyntaxKind::Sequence,
        NodeKind::SequenceEntry => SyntaxKind::SequenceEntry,
        NodeKind::Error => SyntaxKind::Error,
    }
}

const fn token_kind(kind: TokenKind) -> SyntaxKind {
    match kind {
        TokenKind::Scalar => SyntaxKind::Scalar,
        TokenKind::Anchor => SyntaxKind::Anchor,
        TokenKind::Alias => SyntaxKind::Alias,
        TokenKind::Tag => SyntaxKind::Tag,
        TokenKind::Comment => SyntaxKind::Comment,
        TokenKind::Whitespace => SyntaxKind::Whitespace,
        TokenKind::BlockEntry
        | TokenKind::Key
        | TokenKind::Value
        | TokenKind::DocumentStart
        | TokenKind::DocumentEnd
        | TokenKind::FlowSeqStart
        | TokenKind::FlowSeqEnd
        | TokenKind::FlowMapStart
        | TokenKind::FlowMapEnd
        | TokenKind::FlowEntry
        | TokenKind::VersionDirective
        | TokenKind::TagDirective => SyntaxKind::Indicator,
    }
}

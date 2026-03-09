//! Token types for the YAML scanner.

use serde::{Deserialize, Serialize};

use crate::atom::Atom;

// cref: fy_token_type (libfyaml.h)
/// The type of a YAML token.
///
/// Covers all structural and content tokens emitted by the scanner. Path
/// expression tokens from libfyaml are intentionally omitted — yamalgam
/// handles path queries at a higher layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TokenKind {
    // -- Non-content tokens --
    /// Beginning of a YAML stream.
    StreamStart,
    /// End of a YAML stream.
    StreamEnd,
    /// `%YAML x.y` version directive.
    VersionDirective,
    /// `%TAG !prefix! uri` tag directive.
    TagDirective,
    /// `---` document start marker.
    DocumentStart,
    /// `...` document end marker.
    DocumentEnd,

    // -- Content tokens --
    /// Start of a block-style sequence (indentation-based).
    BlockSequenceStart,
    /// Start of a block-style mapping (indentation-based).
    BlockMappingStart,
    /// Implicit end of a block collection (dedent).
    BlockEnd,
    /// `[` — start of a flow sequence.
    FlowSequenceStart,
    /// `]` — end of a flow sequence.
    FlowSequenceEnd,
    /// `{` — start of a flow mapping.
    FlowMappingStart,
    /// `}` — end of a flow mapping.
    FlowMappingEnd,
    /// `- ` block sequence entry indicator.
    BlockEntry,
    /// `,` flow collection entry separator.
    FlowEntry,
    /// `?` explicit key indicator.
    Key,
    /// `:` value indicator.
    Value,
    /// `&name` anchor definition.
    Anchor,
    /// `*name` alias reference.
    Alias,
    /// `!tag` or `!!tag` tag handle.
    Tag,
    /// Scalar content (plain, quoted, or block).
    Scalar,
    /// `# ...` comment (text includes the `#` prefix).
    Comment,
}

impl TokenKind {
    /// Returns `true` if this is a YAML content token (block/flow structure or data).
    ///
    /// Content tokens are everything from `BlockSequenceStart` through `Scalar`.
    /// Non-content tokens are stream markers, directives, and document markers.
    // cref: fy_token_type_is_content
    #[must_use]
    pub const fn is_content(self) -> bool {
        matches!(
            self,
            Self::BlockSequenceStart
                | Self::BlockMappingStart
                | Self::BlockEnd
                | Self::FlowSequenceStart
                | Self::FlowSequenceEnd
                | Self::FlowMappingStart
                | Self::FlowMappingEnd
                | Self::BlockEntry
                | Self::FlowEntry
                | Self::Key
                | Self::Value
                | Self::Anchor
                | Self::Alias
                | Self::Tag
                | Self::Scalar
        )
    }
}

/// A single YAML token with source tracking and content metadata.
// cref: fy_token
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'input> {
    /// The type of this token.
    pub kind: TokenKind,
    /// Content and metadata for this token.
    pub atom: Atom<'input>,
}

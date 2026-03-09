//! YAML event types emitted by the pull parser.

use std::borrow::Cow;

use yamalgam_core::Span;
use yamalgam_scanner::ScalarStyle;

/// Collection presentation style (block vs flow).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionStyle {
    /// Block-style collection (indentation-based).
    Block,
    /// Flow-style collection (JSON-like `[]` / `{}`).
    Flow,
}

/// A YAML parse event.
///
/// Events form a flat stream that encodes the YAML data model:
/// `StreamStart`, zero or more documents (each bracketed by
/// `DocumentStart`/`DocumentEnd`), then `StreamEnd`.
// cref: libfyaml.h:501-513 (fy_event_type)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event<'input> {
    /// Beginning of the YAML stream.
    StreamStart,
    /// End of the YAML stream.
    StreamEnd,
    /// `%YAML x.y` version directive.
    // cref: fy-parse.c:6199-6227 (directive processing)
    VersionDirective {
        /// Major version number.
        major: u8,
        /// Minor version number.
        minor: u8,
        /// Source span of the directive.
        span: Span,
    },
    /// `%TAG !handle! prefix` tag directive.
    TagDirective {
        /// Tag handle (e.g. `!`, `!!`, `!prefix!`).
        handle: Cow<'input, str>,
        /// Tag prefix URI.
        prefix: Cow<'input, str>,
        /// Source span of the directive.
        span: Span,
    },
    /// Start of a YAML document.
    DocumentStart {
        /// True if the document start is implicit (no `---`).
        implicit: bool,
        /// Source span of the document start marker (or inferred position).
        span: Span,
    },
    /// End of a YAML document.
    DocumentEnd {
        /// True if the document end is implicit (no `...`).
        implicit: bool,
        /// Source span of the document end marker (or inferred position).
        span: Span,
    },
    /// Start of a sequence (list).
    SequenceStart {
        /// Optional anchor name (without `&`).
        anchor: Option<Cow<'input, str>>,
        /// Optional resolved tag.
        tag: Option<Cow<'input, str>>,
        /// Block or flow style.
        style: CollectionStyle,
        /// Source span.
        span: Span,
    },
    /// End of a sequence.
    SequenceEnd {
        /// Source span.
        span: Span,
    },
    /// Start of a mapping (dictionary).
    MappingStart {
        /// Optional anchor name (without `&`).
        anchor: Option<Cow<'input, str>>,
        /// Optional resolved tag.
        tag: Option<Cow<'input, str>>,
        /// Block or flow style.
        style: CollectionStyle,
        /// Source span.
        span: Span,
    },
    /// End of a mapping.
    MappingEnd {
        /// Source span.
        span: Span,
    },
    /// A scalar value.
    Scalar {
        /// Optional anchor name (without `&`).
        anchor: Option<Cow<'input, str>>,
        /// Optional resolved tag.
        tag: Option<Cow<'input, str>>,
        /// The scalar text content.
        value: Cow<'input, str>,
        /// Presentation style.
        style: ScalarStyle,
        /// Source span.
        span: Span,
    },
    /// An alias reference (e.g. `*foo`).
    Alias {
        /// The alias name (without `*`).
        name: Cow<'input, str>,
        /// Source span.
        span: Span,
    },

    // -- Full-fidelity structural events (yamalgam extension) --
    // These events are emitted for CST, SAX, and streaming consumers.
    // Semantic consumers (Value, serde) skip them.
    /// A YAML comment (text includes the `#` prefix).
    Comment {
        /// The comment text, including `#`.
        text: Cow<'input, str>,
        /// Source span.
        span: Span,
    },

    /// `-` block sequence entry indicator.
    BlockEntry {
        /// Source span of the `-`.
        span: Span,
    },

    /// `?` or implicit key indicator.
    KeyIndicator {
        /// Source span of the `?` (or inferred position for implicit keys).
        span: Span,
    },

    /// `:` value indicator.
    ValueIndicator {
        /// Source span of the `:`.
        span: Span,
    },
}

impl Event<'_> {
    /// Returns `true` for yamalgam-specific structural events that semantic
    /// consumers (Value, serde) should skip.
    #[must_use]
    pub const fn is_structural(&self) -> bool {
        matches!(
            self,
            Event::Comment { .. }
                | Event::BlockEntry { .. }
                | Event::KeyIndicator { .. }
                | Event::ValueIndicator { .. }
        )
    }
}

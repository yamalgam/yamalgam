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
}

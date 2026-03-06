//! Scalar presentation styles, chomping modes, and atom content flags.

use serde::{Deserialize, Serialize};

// cref: fy_scalar_style (libfyaml.h)
/// Presentation style for YAML scalar values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScalarStyle {
    /// Unquoted scalar — no quoting indicators.
    Plain,
    /// Single-quoted scalar (`'...'`).
    SingleQuoted,
    /// Double-quoted scalar (`"..."`), supports escape sequences.
    DoubleQuoted,
    /// Literal block scalar (`|`) — preserves newlines.
    Literal,
    /// Folded block scalar (`>`) — folds newlines to spaces.
    Folded,
}

// cref: fy_atom_chomp (fy-atom.h: FYAC_STRIP, FYAC_CLIP, FYAC_KEEP)
/// Block scalar chomping mode — controls trailing newline handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Chomp {
    /// Strip (`-`): remove all trailing newlines.
    Strip,
    /// Clip (default): keep a single trailing newline.
    #[default]
    Clip,
    /// Keep (`+`): preserve all trailing newlines.
    Keep,
}

bitflags::bitflags! {
    // cref: fy_atom (fy-atom.h boolean fields)
    /// Content property flags for an [`Atom`](crate::Atom).
    ///
    /// Tracks characteristics of the atom's text content, derived from
    /// the boolean fields on libfyaml's `struct fy_atom`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct AtomFlags: u32 {
        /// Atom contains at least one line break.
        const HAS_LB          = 1 << 0;
        /// Atom contains at least one whitespace character.
        const HAS_WS          = 1 << 1;
        /// Atom starts with a whitespace character.
        const STARTS_WITH_WS  = 1 << 2;
        /// Atom starts with a line break.
        const STARTS_WITH_LB  = 1 << 3;
        /// Atom ends with a whitespace character.
        const ENDS_WITH_WS    = 1 << 4;
        /// Atom ends with a line break.
        const ENDS_WITH_LB    = 1 << 5;
        /// Atom has trailing line breaks (more than one at the end).
        const TRAILING_LB     = 1 << 6;
        /// Atom contains only whitespace and line breaks (if length > 0).
        const EMPTY           = 1 << 7;
        /// Atom contains absolutely nothing (zero size).
        const SIZE0           = 1 << 8;
        /// Atom contains escape sequences (double-quoted scalars).
        const HAS_ESC         = 1 << 9;
        /// Atom can be emitted verbatim without re-encoding.
        const DIRECT_OUTPUT   = 1 << 10;
        /// Atom spans multiple lines.
        const IS_MULTILINE    = 1 << 11;
        /// Atom is a valid anchor name (without `&` prefix).
        const VALID_ANCHOR    = 1 << 12;
        /// Atom was read in JSON compatibility mode.
        const JSON_MODE       = 1 << 13;
        /// Atom ends at EOF of input.
        const ENDS_WITH_EOF   = 1 << 14;
        /// Atom is the merge key `<<`.
        const IS_MERGE_KEY    = 1 << 15;
        /// Atom allows a simple key.
        const SIMPLE_KEY_ALLOWED = 1 << 16;
        /// Atom contains high ASCII (UTF-8 code points >= 0x80).
        const HIGH_ASCII      = 1 << 17;
        /// Block scalar chomp indicator was explicit in the source.
        const CHOMP_EXPLICIT  = 1 << 18;
    }
}

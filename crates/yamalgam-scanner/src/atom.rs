//! Zero-copy text atoms with presentation metadata.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use yamalgam_core::Span;

use crate::style::{AtomFlags, Chomp, ScalarStyle};

/// Zero-copy text chunk with presentation metadata.
///
/// An atom represents a unit of YAML content along with its source location
/// and presentation information. For zero-copy parsing, `data` borrows from
/// the input buffer when possible ([`Cow::Borrowed`]) and owns the text when
/// processing was required (escape handling, folding, etc.).
// cref: fy_atom
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Atom<'input> {
    /// The text content of this atom.
    pub data: Cow<'input, str>,
    /// Source location of this atom in the input.
    pub span: Span,
    /// Presentation style (only meaningful for scalar tokens).
    pub style: ScalarStyle,
    /// Block scalar chomping mode (only meaningful for literal/folded scalars).
    pub chomp: Chomp,
    /// Content property flags.
    pub flags: AtomFlags,
}

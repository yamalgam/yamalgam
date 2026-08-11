//! Rope-backed editable YAML documents with bounded syntax reparsing.

use std::fmt;
use std::ops::Range;

use ropey::Rope;
use yamalgam_core::LoaderConfig;

use crate::{SyntaxKind, SyntaxNode, SyntaxTree};

/// A byte-based source edit. Ranges refer to the document before the batch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    /// Byte range replaced by this edit.
    pub range: Range<usize>,
    /// Replacement UTF-8 text.
    pub replacement: String,
}

/// Scope used for the most recent syntax reparse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReparseOutcome {
    /// One enclosing YAML document subtree was replaced.
    Document,
    /// The complete stream was reparsed.
    Full,
}

/// Rejected edit batch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    /// A range is reversed, outside the document, or splits a UTF-8 scalar.
    InvalidRange(Range<usize>),
    /// Two edits address overlapping bytes in the pre-edit document.
    OverlappingEdits,
}

impl fmt::Display for EditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRange(range) => write!(
                formatter,
                "invalid edit range {}..{}",
                range.start, range.end
            ),
            Self::OverlappingEdits => formatter.write_str("edit ranges overlap"),
        }
    }
}

impl std::error::Error for EditError {}

/// Editable source text and its current immutable syntax tree.
#[derive(Clone, Debug)]
pub struct IncrementalDocument {
    text: Rope,
    syntax: SyntaxTree,
}

impl IncrementalDocument {
    /// Create an editable document from UTF-8 source.
    #[must_use]
    pub fn new(input: &str, config: &LoaderConfig) -> Self {
        Self {
            text: Rope::from_str(input),
            syntax: SyntaxTree::parse_with_config(input, config),
        }
    }

    /// Return the current source text.
    #[must_use]
    pub fn text(&self) -> String {
        self.text.to_string()
    }

    /// Return the current immutable syntax tree.
    #[must_use]
    pub const fn syntax(&self) -> &SyntaxTree {
        &self.syntax
    }

    /// Apply a batch of byte edits and reparse the smallest safe scope.
    pub fn apply(
        &mut self,
        edits: &[TextEdit],
        config: &LoaderConfig,
    ) -> Result<ReparseOutcome, EditError> {
        let original = self.text.to_string();
        let ordered = validate_edits(edits, &original)?;
        let document = self.document_reparse_candidate(&ordered, &original);

        for edit in ordered.iter().rev() {
            let start = self.text.byte_to_char(edit.range.start);
            let end = self.text.byte_to_char(edit.range.end);
            self.text.remove(start..end);
            self.text.insert(start, &edit.replacement);
        }

        let updated = self.text.to_string();
        if let Some((old_document, old_range)) = document {
            let delta = ordered.iter().fold(0isize, |total, edit| {
                total + edit.replacement.len() as isize
                    - (edit.range.end - edit.range.start) as isize
            });
            if let Some(new_end) = old_range.end.checked_add_signed(delta)
                && new_end <= updated.len()
                && updated.is_char_boundary(old_range.start)
                && updated.is_char_boundary(new_end)
            {
                let reparsed =
                    SyntaxTree::parse_with_config(&updated[old_range.start..new_end], config);
                let mut documents = reparsed
                    .root()
                    .children()
                    .filter(|node| node.kind() == SyntaxKind::Document);
                if reparsed.diagnostics().is_empty()
                    && let Some(replacement) = documents.next()
                    && documents.next().is_none()
                    && !replacement
                        .descendants()
                        .any(|node| node.kind() == SyntaxKind::Error)
                {
                    let green = old_document.replace_with(replacement.green().into_owned());
                    let syntax = SyntaxTree::from_parts(green, Vec::new());
                    if syntax.text() == updated {
                        self.syntax = syntax;
                        return Ok(ReparseOutcome::Document);
                    }
                }
            }
        }

        self.syntax = SyntaxTree::parse_with_config(&updated, config);
        Ok(ReparseOutcome::Full)
    }

    /// Convert a UTF-8 byte offset to a zero-indexed UTF-16 line and column.
    #[must_use]
    pub fn byte_offset_to_utf16(&self, offset: usize) -> Option<(u32, u32)> {
        if offset > self.text.len_bytes() || self.text.try_byte_to_char(offset).is_err() {
            return None;
        }
        let line = self.text.try_byte_to_line(offset).ok()?;
        let line_start = self.text.try_line_to_byte(line).ok()?;
        let prefix = self.text.byte_slice(line_start..offset).to_string();
        Some((
            u32::try_from(line).ok()?,
            u32::try_from(prefix.encode_utf16().count()).ok()?,
        ))
    }

    /// Convert a zero-indexed UTF-16 line and column to a UTF-8 byte offset.
    #[must_use]
    pub fn utf16_to_byte_offset(&self, line: u32, character: u32) -> Option<usize> {
        let line = usize::try_from(line).ok()?;
        if line >= self.text.len_lines() {
            return None;
        }
        let line_start = self.text.try_line_to_byte(line).ok()?;
        let mut content = self.text.line(line).to_string();
        if content.ends_with("\r\n") {
            content.truncate(content.len() - 2);
        } else if content.ends_with(['\r', '\n']) {
            content.pop();
        }

        let target = usize::try_from(character).ok()?;
        let mut utf16 = 0;
        let mut bytes = 0;
        for scalar in content.chars() {
            if utf16 == target {
                return Some(line_start + bytes);
            }
            let units = scalar.len_utf16();
            if utf16 + units > target {
                return None;
            }
            utf16 += units;
            bytes += scalar.len_utf8();
        }
        (utf16 == target).then_some(line_start + bytes)
    }

    fn document_reparse_candidate(
        &self,
        edits: &[&TextEdit],
        original: &str,
    ) -> Option<(SyntaxNode, Range<usize>)> {
        if edits.is_empty() || !self.syntax.diagnostics().is_empty() {
            return None;
        }
        if edits.iter().any(|edit| {
            contains_document_marker(&original[edit.range.clone()])
                || contains_document_marker(&edit.replacement)
        }) {
            return None;
        }

        let mut matches = self.syntax.root().children().filter(|node| {
            if node.kind() != SyntaxKind::Document
                || node
                    .descendants()
                    .any(|child| child.kind() == SyntaxKind::Error)
            {
                return false;
            }
            let range = syntax_range(node);
            edits
                .iter()
                .all(|edit| edit.range.start >= range.start && edit.range.end <= range.end)
        });
        let document = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        let range = syntax_range(&document);
        Some((document, range))
    }
}

fn validate_edits<'a>(edits: &'a [TextEdit], text: &str) -> Result<Vec<&'a TextEdit>, EditError> {
    let mut ordered: Vec<_> = edits.iter().collect();
    ordered.sort_by_key(|edit| (edit.range.start, edit.range.end));
    for edit in &ordered {
        if edit.range.start > edit.range.end
            || edit.range.end > text.len()
            || !text.is_char_boundary(edit.range.start)
            || !text.is_char_boundary(edit.range.end)
        {
            return Err(EditError::InvalidRange(edit.range.clone()));
        }
    }
    if ordered
        .windows(2)
        .any(|pair| pair[0].range.end > pair[1].range.start)
    {
        return Err(EditError::OverlappingEdits);
    }
    Ok(ordered)
}

fn syntax_range(node: &SyntaxNode) -> Range<usize> {
    usize::from(node.text_range().start())..usize::from(node.text_range().end())
}

fn contains_document_marker(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim_start();
        line == "---" || line.starts_with("--- ") || line == "..." || line.starts_with("... ")
    })
}

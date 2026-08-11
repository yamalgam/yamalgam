//! Versioned language-server document snapshots.

use std::ops::Range as ByteRange;
use std::sync::Arc;

use lsp_types::{Position, TextDocumentContentChangeEvent, Uri};
use thiserror::Error;
use yamalgam_compose::from_str_located_with_config;
use yamalgam_core::{LoaderConfig, LocatedValue};
use yamalgam_cst::{EditError, IncrementalDocument, SyntaxKind};
use yamalgam_schema::SelectedSchema;

use crate::ServerConfig;

/// One open text document and its derived syntax and semantic state.
#[derive(Clone, Debug)]
pub struct DocumentState {
    /// Canonical document URI supplied by the client.
    pub uri: Uri,
    /// Latest accepted monotonically increasing client version.
    pub version: i32,
    /// Rope-backed recovering syntax snapshot.
    pub syntax: IncrementalDocument,
    source: Arc<str>,
    /// Located composed documents; empty while source is not composable.
    pub located: Vec<LocatedValue>,
    /// Schemas selected for the documents in this stream.
    pub selected_schemas: Vec<SelectedSchema>,
    schema_assignments: Vec<Option<SelectedSchema>>,
    schema_selection_error: Option<String>,
    composition_error: Option<String>,
}

impl DocumentState {
    /// Open a document and build its first syntax and semantic snapshots.
    #[must_use]
    pub fn new(uri: Uri, version: i32, text: &str, config: &LoaderConfig) -> Self {
        let (located, composition_error) = compose(text, config);
        Self {
            uri,
            version,
            syntax: IncrementalDocument::new(text, config),
            source: Arc::from(text),
            located,
            selected_schemas: Vec::new(),
            schema_assignments: Vec::new(),
            schema_selection_error: None,
            composition_error,
        }
    }

    /// Apply sequential LSP content changes when `version` is newer.
    ///
    /// Returns `Ok(false)` for stale or duplicate versions without changing
    /// the snapshot.
    pub fn apply_changes(
        &mut self,
        version: i32,
        changes: &[TextDocumentContentChangeEvent],
        config: &LoaderConfig,
    ) -> Result<bool, DocumentError> {
        if version <= self.version {
            return Ok(false);
        }

        let mut text = self.source.to_string();
        for change in changes {
            let range = match change.range {
                Some(range) => {
                    position_offset(&text, range.start)?..position_offset(&text, range.end)?
                }
                None => 0..text.len(),
            };
            text.replace_range(range, &change.text);
        }

        let mut syntax = self.syntax.clone();
        syntax.replace_text(&text, config);
        let (located, composition_error) = compose(&text, config);
        self.syntax = syntax;
        self.source = Arc::from(text);
        self.located = located;
        self.composition_error = composition_error;
        self.version = version;
        Ok(true)
    }

    /// Re-select schemas for the current text and configured associations.
    ///
    /// # Errors
    ///
    /// Returns a typed selection error for malformed directives, URIs, or globs.
    pub fn select_schemas(
        &mut self,
        config: &ServerConfig,
    ) -> Result<(), yamalgam_schema::SchemaSelectionError> {
        let containing = yamalgam_resolve::ResourceId::new(self.uri.as_str()).ok();
        let selector = yamalgam_schema::SchemaSelector {
            associations: config.schema_associations.clone(),
            schema_store: None,
        };
        let selection = if let Some(message) = self.composition_error.as_ref() {
            Err(yamalgam_schema::SchemaSelectionError::InvalidDocument {
                message: message.clone(),
            })
        } else if self.source.trim().is_empty() {
            selector.select("null\n", containing.as_ref(), None)
        } else {
            selector.select_composed(&self.source, &self.located, containing.as_ref(), None)
        };
        match selection {
            Ok(assignments) => {
                self.selected_schemas = assignments.iter().flatten().cloned().collect();
                self.schema_assignments = assignments;
                self.schema_selection_error = None;
                Ok(())
            }
            Err(error) => {
                self.selected_schemas.clear();
                self.schema_assignments.clear();
                self.schema_selection_error = Some(error.to_string());
                Err(error)
            }
        }
    }

    pub(crate) fn schema_for_document(&self, index: usize) -> Option<&SelectedSchema> {
        self.schema_assignments.get(index).and_then(Option::as_ref)
    }

    pub(crate) fn document_index_at_offset(&self, offset: usize) -> usize {
        let mut index = 0;
        for (candidate, document) in self
            .syntax
            .syntax()
            .root()
            .children()
            .filter(|node| node.kind() == SyntaxKind::Document)
            .enumerate()
        {
            if usize::from(document.text_range().start()) > offset {
                break;
            }
            index = candidate;
        }
        index
    }

    pub(crate) fn schema_selection_error(&self) -> Option<&str> {
        self.schema_selection_error.as_deref()
    }

    /// Convert one negotiated UTF-16 position to an internal byte offset.
    #[must_use]
    pub fn offset_at(&self, position: Position) -> Option<usize> {
        self.syntax
            .utf16_to_byte_offset(position.line, position.character)
    }
}

/// Invalid document update.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum DocumentError {
    /// An LSP UTF-16 position does not identify a UTF-8 boundary.
    #[error("invalid UTF-16 document position {0:?}")]
    InvalidPosition(Position),
    /// The resulting byte edit was invalid.
    #[error(transparent)]
    Edit(#[from] EditError),
}

fn compose(text: &str, config: &LoaderConfig) -> (Vec<LocatedValue>, Option<String>) {
    match from_str_located_with_config(text, config) {
        Ok(located) => (located, None),
        Err(error) => (Vec::new(), Some(error.to_string())),
    }
}

fn position_offset(input: &str, position: Position) -> Result<usize, DocumentError> {
    let range =
        line_content_range(input, position.line).ok_or(DocumentError::InvalidPosition(position))?;
    let target = usize::try_from(position.character)
        .map_err(|_| DocumentError::InvalidPosition(position))?;
    let mut utf16 = 0;
    for (relative, scalar) in input[range.clone()].char_indices() {
        if utf16 == target {
            return Ok(range.start + relative);
        }
        let units = scalar.len_utf16();
        if utf16 + units > target {
            return Err(DocumentError::InvalidPosition(position));
        }
        utf16 += units;
    }
    if utf16 == target {
        Ok(range.end)
    } else {
        Err(DocumentError::InvalidPosition(position))
    }
}

fn line_content_range(input: &str, target: u32) -> Option<ByteRange<usize>> {
    let bytes = input.as_bytes();
    let mut line = 0u32;
    let mut start = 0;
    let mut offset = 0;
    while offset < bytes.len() {
        let width = match bytes[offset] {
            b'\r' if bytes.get(offset + 1) == Some(&b'\n') => 2,
            b'\r' | b'\n' => 1,
            _ => {
                offset += 1;
                continue;
            }
        };
        if line == target {
            return Some(start..offset);
        }
        offset += width;
        line = line.checked_add(1)?;
        start = offset;
    }
    (line == target).then_some(start..input.len())
}

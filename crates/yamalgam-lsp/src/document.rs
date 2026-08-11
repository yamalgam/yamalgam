//! Versioned language-server document snapshots.

use lsp_types::{Position, TextDocumentContentChangeEvent, Uri};
use thiserror::Error;
use yamalgam_compose::from_str_located_with_config;
use yamalgam_core::{LoaderConfig, LocatedValue};
use yamalgam_cst::{EditError, IncrementalDocument, TextEdit};
use yamalgam_schema::SelectedSchema;

/// One open text document and its derived syntax and semantic state.
#[derive(Clone, Debug)]
pub struct DocumentState {
    /// Canonical document URI supplied by the client.
    pub uri: Uri,
    /// Latest accepted monotonically increasing client version.
    pub version: i32,
    /// Rope-backed recovering syntax snapshot.
    pub syntax: IncrementalDocument,
    /// Located composed documents; empty while source is not composable.
    pub located: Vec<LocatedValue>,
    /// Schemas selected for the documents in this stream.
    pub selected_schemas: Vec<SelectedSchema>,
}

impl DocumentState {
    /// Open a document and build its first syntax and semantic snapshots.
    #[must_use]
    pub fn new(uri: Uri, version: i32, text: &str, config: &LoaderConfig) -> Self {
        Self {
            uri,
            version,
            syntax: IncrementalDocument::new(text, config),
            located: from_str_located_with_config(text, config).unwrap_or_default(),
            selected_schemas: Vec::new(),
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

        let mut syntax = self.syntax.clone();
        for change in changes {
            let edit = match change.range {
                Some(range) => TextEdit {
                    range: position_offset(&syntax, range.start)?
                        ..position_offset(&syntax, range.end)?,
                    replacement: change.text.clone(),
                },
                None => TextEdit {
                    range: 0..syntax.text().len(),
                    replacement: change.text.clone(),
                },
            };
            syntax.apply(&[edit], config)?;
        }

        let text = syntax.text();
        self.syntax = syntax;
        self.located = from_str_located_with_config(&text, config).unwrap_or_default();
        self.version = version;
        Ok(true)
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

fn position_offset(
    document: &IncrementalDocument,
    position: Position,
) -> Result<usize, DocumentError> {
    document
        .utf16_to_byte_offset(position.line, position.character)
        .ok_or(DocumentError::InvalidPosition(position))
}

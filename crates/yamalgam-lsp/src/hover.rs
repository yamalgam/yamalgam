//! Schema annotation hover rendering.

use lsp_types::{Hover, HoverContents, MarkedString, Position};

use crate::completion::editor_context;
use crate::diagnostics::redacted_schema_uri;
use crate::{DocumentState, SchemaWorkspace};

/// Describe the schema applying at one UTF-16 document position.
#[must_use]
pub fn hover(
    document: &DocumentState,
    position: Position,
    schemas: &SchemaWorkspace,
) -> Option<Hover> {
    let offset = document.offset_at(position)?;
    if !hoverable(&document.syntax.text(), offset) {
        return None;
    }
    let cursor = editor_context(document, position)?;
    let selected = document.schema_for_document(cursor.document_index)?;
    let validator = schemas.validator(selected, 64).ok()?;
    let context = validator.context_at(&cursor.path);
    if context.title.is_none()
        && context.description.is_none()
        && context.types.is_empty()
        && context.default.is_none()
        && context.enum_values.is_empty()
        && context.examples.is_empty()
        && !context.deprecated
    {
        return None;
    }

    let mut lines = Vec::new();
    if let Some(title) = context.title {
        lines.push(title);
    }
    if let Some(description) = context.description {
        lines.push(description);
    }
    if !context.types.is_empty() {
        lines.push(format!("Type: {}", context.types.join(" | ")));
    }
    if let Some(default) = context.default {
        lines.push(format!("Default: {default}"));
    }
    if !context.examples.is_empty() {
        lines.push(format!("Examples: {}", context.examples.join(", ")));
    }
    if !context.enum_values.is_empty() {
        lines.push(format!("Allowed: {}", context.enum_values.join(", ")));
    }
    if context.deprecated {
        lines.push("Deprecated".to_owned());
    }
    lines.push(format!("Schema: {}", redacted_schema_uri(&selected.uri)));
    Some(Hover {
        contents: HoverContents::Scalar(MarkedString::String(lines.join("\n\n"))),
        range: None,
    })
}

fn hoverable(text: &str, offset: usize) -> bool {
    let Some(character) = text[offset..].chars().next() else {
        return false;
    };
    if character.is_whitespace() {
        return false;
    }
    let line_start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    !text[line_start..offset].contains('#') && character != '#'
}

//! Schema-backed YAML completion.

use std::collections::HashSet;

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, Documentation, InsertTextFormat,
    Position,
};
use yamalgam_core::Value;
use yamalgam_cst::SyntaxKind;

use crate::{DocumentState, SchemaWorkspace};

/// Return schema candidates at one UTF-16 document position.
#[must_use]
pub fn complete(
    document: &DocumentState,
    position: Position,
    schemas: &SchemaWorkspace,
) -> CompletionResponse {
    let Some(cursor) = editor_context(document, position) else {
        return CompletionResponse::Array(Vec::new());
    };
    let Some(selected) = document.schema_for_document(cursor.document_index) else {
        return CompletionResponse::Array(Vec::new());
    };
    let Ok(validator) = schemas.validator(selected, 64) else {
        return CompletionResponse::Array(Vec::new());
    };
    let context = validator.context_at(&cursor.path);
    let mut items = Vec::new();

    if cursor.value_position {
        for value in context.enum_values {
            items.push(CompletionItem {
                label: value.clone(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                insert_text: Some(value),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..CompletionItem::default()
            });
        }
        if items.is_empty()
            && let Some(default) = context.default
        {
            items.push(CompletionItem {
                label: default.clone(),
                kind: Some(CompletionItemKind::VALUE),
                insert_text: Some(default),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..CompletionItem::default()
            });
        }
        return CompletionResponse::Array(items);
    }

    for candidate in context.properties {
        if cursor.existing_keys.contains(&candidate.name) {
            continue;
        }
        let insert_text = candidate.default.as_ref().map_or_else(
            || format!("{}: ", candidate.name),
            |default| format!("{}: {default}", candidate.name),
        );
        items.push(CompletionItem {
            label: candidate.name.clone(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: candidate.required.then(|| "required".to_owned()),
            documentation: candidate.description.map(Documentation::String),
            sort_text: Some(format!(
                "{}-{}",
                usize::from(!candidate.required),
                candidate.name
            )),
            insert_text: Some(insert_text),
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
            ..CompletionItem::default()
        });
    }
    CompletionResponse::Array(items)
}

pub(super) struct EditorContext {
    pub document_index: usize,
    pub path: String,
    pub value_position: bool,
    pub existing_keys: HashSet<String>,
}

pub(super) fn editor_context(
    document: &DocumentState,
    position: Position,
) -> Option<EditorContext> {
    let offset = document.offset_at(position)?;
    let text = document.syntax.text();
    let line_start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    let prefix = &text[line_start..offset];
    let document_index = document.document_index_at_offset(offset);
    let mut path = syntax_path(document, offset, line_start);
    let content = prefix.trim_start();
    let value_position = mapping_entry(content).is_some();
    if let Some((key, _)) = mapping_entry(content)
        && !path.ends_with(&format!("/{}", escape_pointer(&key)))
    {
        path = append_pointer(&path, &key);
    }
    let existing_keys = mapping_at(document.located.get(document_index), &path)
        .map(|mapping| {
            mapping
                .iter()
                .filter_map(|(key, _)| key.canonical_scalar_text().map(|key| key.into_owned()))
                .collect()
        })
        .unwrap_or_default();

    Some(EditorContext {
        document_index,
        path,
        value_position,
        existing_keys,
    })
}

fn syntax_path(document: &DocumentState, offset: usize, line_start: usize) -> String {
    let syntax = document.syntax.syntax();
    let mut probe = offset;
    let node = loop {
        if let Some(node) = syntax.node_at_offset(probe)
            && node.kind() != SyntaxKind::Stream
        {
            break Some(node);
        }
        if probe == line_start {
            break None;
        }
        probe -= 1;
    };
    let Some(node) = node else {
        return String::new();
    };
    let mut ancestors = node.ancestors().collect::<Vec<_>>();
    ancestors.reverse();
    let mut path = String::new();

    for (position, node) in ancestors.iter().enumerate() {
        match node.kind() {
            SyntaxKind::MappingEntry => {
                let text = node.text().to_string();
                if let Some((key, _)) = mapping_entry(text.trim_start()) {
                    path = append_pointer(&path, &key);
                }
            }
            SyntaxKind::Sequence => {
                let child = ancestors.get(position + 1);
                let index = child.map_or_else(
                    || flow_sequence_index(node, offset),
                    |child| sequence_index(node, child, offset),
                );
                path = append_pointer(&path, &index.to_string());
            }
            _ => {}
        }
    }
    path
}

fn sequence_index(
    sequence: &yamalgam_cst::SyntaxNode,
    child: &yamalgam_cst::SyntaxNode,
    offset: usize,
) -> usize {
    if child.kind() == SyntaxKind::SequenceEntry {
        return sequence
            .children()
            .filter(|candidate| candidate.kind() == SyntaxKind::SequenceEntry)
            .take_while(|candidate| candidate.text_range().start() < child.text_range().start())
            .count();
    }
    flow_sequence_index(sequence, offset)
}

fn flow_sequence_index(sequence: &yamalgam_cst::SyntaxNode, offset: usize) -> usize {
    sequence
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.text() == "," && usize::from(token.text_range().start()) < offset)
        .count()
}

fn mapping_entry(line: &str) -> Option<(String, &str)> {
    let line = line.strip_prefix("- ").unwrap_or(line);
    let (key, value) = line.split_once(':')?;
    let key = key.trim();
    if key.is_empty() || key.starts_with(['[', '{']) {
        return None;
    }
    Some((
        key.trim_matches(|character| character == '\'' || character == '"')
            .to_owned(),
        value.trim(),
    ))
}

fn mapping_at<'value>(
    located: Option<&'value yamalgam_core::LocatedValue>,
    path: &str,
) -> Option<&'value yamalgam_core::Mapping> {
    let mut value = &located?.value;
    if !path.is_empty() {
        for segment in path.strip_prefix('/')?.split('/') {
            let segment = segment.replace("~1", "/").replace("~0", "~");
            value = match value {
                Value::Mapping(mapping) => mapping.get_str(&segment)?,
                Value::Sequence(values) => values.get(segment.parse::<usize>().ok()?)?,
                _ => return None,
            };
        }
    }
    match value {
        Value::Mapping(mapping) => Some(mapping),
        _ => None,
    }
}

fn append_pointer(path: &str, segment: &str) -> String {
    format!("{path}/{}", escape_pointer(segment))
}

fn escape_pointer(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

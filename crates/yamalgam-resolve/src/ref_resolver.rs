//! Sole-key `$ref` recognition and RFC 6901 node selection.

// r[impl resolution.ref]
// r[impl resolution.ref.fragment]
// r[impl resolution.ref.document]

use yamalgam_core::{Span, TagResolution, TagResolver};
use yamalgam_parser::ScalarStyle;

use crate::Fragment;
use crate::document::{OwnedKind, OwnedNode};
use crate::error::SessionError;
use crate::pointer::sequence_index;

pub struct RefDirective {
    pub requested: String,
    pub span: Span,
}

pub fn recognize(
    node: &OwnedNode,
    tag_resolution: TagResolution,
) -> Result<Option<RefDirective>, SessionError> {
    let OwnedKind::Mapping { entries, .. } = &node.kind else {
        return Ok(None);
    };
    let ref_entries = entries
        .iter()
        .filter(|(key, _)| string_value(key, tag_resolution) == Some("$ref"))
        .collect::<Vec<_>>();
    if ref_entries.is_empty() {
        return Ok(None);
    }
    if entries.len() != 1 || ref_entries.len() != 1 {
        return Err(SessionError::RefSiblings);
    }
    let (_, value) = ref_entries[0];
    let requested = string_value(value, tag_resolution)
        .ok_or(SessionError::RefValueNotString)?
        .to_owned();
    Ok(Some(RefDirective {
        requested,
        span: value.span,
    }))
}

pub fn select(
    root: &OwnedNode,
    fragment: &Fragment,
    tag_resolution: TagResolution,
) -> Result<OwnedNode, SessionError> {
    let mut selected = root;
    for token in fragment.pointer_tokens() {
        selected = match &selected.kind {
            OwnedKind::Mapping { entries, .. } => {
                let mut has_non_string_key = false;
                let found = entries.iter().find_map(|(key, value)| {
                    let Some(key) = string_value(key, tag_resolution) else {
                        has_non_string_key = true;
                        return None;
                    };
                    (key == token).then_some(value)
                });
                match found {
                    Some(value) => value,
                    None if has_non_string_key => {
                        return Err(SessionError::NonStringMappingKey);
                    }
                    None => return Err(SessionError::MissingFragment),
                }
            }
            OwnedKind::Sequence { items, .. } => {
                let index =
                    sequence_index(token).map_err(|_| SessionError::InvalidSequenceIndex)?;
                items.get(index).ok_or(SessionError::MissingFragment)?
            }
            _ => return Err(SessionError::MissingFragment),
        };
    }
    Ok(selected.clone())
}

fn string_value(node: &OwnedNode, tag_resolution: TagResolution) -> Option<&str> {
    let OwnedKind::Scalar(scalar) = &node.kind else {
        return None;
    };
    match node.tag.as_deref() {
        Some("tag:yaml.org,2002:str" | "!!str") => Some(&scalar.value),
        Some(_) => None,
        None if scalar.style != ScalarStyle::Plain => Some(&scalar.value),
        None if tag_resolution.resolves_as_string(&scalar.value) => Some(&scalar.value),
        None => None,
    }
}

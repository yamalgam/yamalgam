//! Imported-subtree anchor validation and deterministic namespacing.

use std::collections::HashSet;

use crate::SessionError;
use crate::document::{OwnedKind, OwnedNode};

pub fn reserve(node: &OwnedNode, reserved: &mut HashSet<String>) {
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        if let Some(anchor) = &node.anchor {
            reserved.insert(anchor.clone());
        }
        match &node.kind {
            OwnedKind::Sequence { items, .. } => pending.extend(items.iter().rev()),
            OwnedKind::Mapping { entries, .. } => {
                for (key, value) in entries.iter().rev() {
                    pending.push(value);
                    pending.push(key);
                }
            }
            OwnedKind::Alias { .. } | OwnedKind::Scalar(_) => {}
        }
    }
}

pub fn namespace(
    node: &mut OwnedNode,
    base_prefix: &str,
    reserved: &mut HashSet<String>,
) -> Result<(), SessionError> {
    let mut visible = HashSet::new();
    let mut source_anchors = HashSet::new();
    let mut pending = vec![&*node];
    while let Some(node) = pending.pop() {
        if let Some(anchor) = &node.anchor {
            visible.insert(anchor.clone());
            source_anchors.insert(anchor.clone());
        }
        match &node.kind {
            OwnedKind::Alias { name } => {
                if !visible.contains(name) {
                    return Err(SessionError::NonSelfContainedAlias(name.clone()));
                }
            }
            OwnedKind::Sequence { items, .. } => pending.extend(items.iter().rev()),
            OwnedKind::Mapping { entries, .. } => {
                for (key, value) in entries.iter().rev() {
                    pending.push(value);
                    pending.push(key);
                }
            }
            OwnedKind::Scalar(_) => {}
        }
    }

    let mut attempt = 0_usize;
    let prefix = loop {
        let candidate = if attempt == 0 {
            base_prefix.to_owned()
        } else {
            format!("{base_prefix}n{attempt}_")
        };
        if source_anchors
            .iter()
            .all(|anchor| !reserved.contains(&format!("{candidate}{anchor}")))
        {
            break candidate;
        }
        attempt = attempt.saturating_add(1);
    };
    for anchor in &source_anchors {
        reserved.insert(format!("{prefix}{anchor}"));
    }

    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        if let Some(anchor) = &mut node.anchor {
            anchor.insert_str(0, &prefix);
        }
        match &mut node.kind {
            OwnedKind::Alias { name } => name.insert_str(0, &prefix),
            OwnedKind::Sequence { items, .. } => pending.extend(items.iter_mut().rev()),
            OwnedKind::Mapping { entries, .. } => {
                for (key, value) in entries.iter_mut().rev() {
                    pending.push(value);
                    pending.push(key);
                }
            }
            OwnedKind::Scalar(_) => {}
        }
    }
    Ok(())
}

//! Imported-subtree anchor validation and deterministic namespacing.

use std::collections::HashSet;

use crate::SessionError;
use crate::document::{OwnedKind, OwnedNode};

pub fn namespace(node: &mut OwnedNode, prefix: &str) -> Result<(), SessionError> {
    let mut defined = HashSet::new();
    let mut used = HashSet::new();
    let mut pending = vec![&*node];
    while let Some(node) = pending.pop() {
        if let Some(anchor) = &node.anchor {
            defined.insert(anchor.clone());
        }
        match &node.kind {
            OwnedKind::Alias { name } => {
                used.insert(name.clone());
            }
            OwnedKind::Sequence { items, .. } => pending.extend(items),
            OwnedKind::Mapping { entries, .. } => {
                for (key, value) in entries {
                    pending.push(key);
                    pending.push(value);
                }
            }
            OwnedKind::Scalar(_) => {}
        }
    }

    if let Some(external) = used.iter().filter(|name| !defined.contains(*name)).min() {
        return Err(SessionError::NonSelfContainedAlias(external.clone()));
    }

    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        if let Some(anchor) = &mut node.anchor {
            anchor.insert_str(0, prefix);
        }
        match &mut node.kind {
            OwnedKind::Alias { name } => name.insert_str(0, prefix),
            OwnedKind::Sequence { items, .. } => pending.extend(items),
            OwnedKind::Mapping { entries, .. } => {
                for (key, value) in entries {
                    pending.push(key);
                    pending.push(value);
                }
            }
            OwnedKind::Scalar(_) => {}
        }
    }
    Ok(())
}

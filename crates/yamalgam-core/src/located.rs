//! Source locations parallel to the semantic [`Value`](crate::Value) tree.

use crate::{Span, Value};

/// A semantic YAML value and its value-shaped source-location tree.
#[derive(Debug, Clone, PartialEq)]
pub struct LocatedValue {
    /// Composed semantic value.
    pub value: Value,
    /// Source locations with the same shape and child order as `value`.
    pub locations: ValueLocations,
}

/// Source location for one semantic value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueLocations {
    /// Source span for this value.
    pub span: Span,
    /// Child locations matching the value's semantic kind.
    pub kind: LocationKind,
}

/// Child-location shape for a semantic value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocationKind {
    /// A scalar has no child locations.
    Scalar,
    /// Sequence item locations in semantic order.
    Sequence(Vec<ValueLocations>),
    /// Mapping entry locations in semantic order.
    Mapping(Vec<MappingLocations>),
}

/// Source locations for one semantic mapping entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingLocations {
    /// Location of the mapping key.
    pub key: ValueLocations,
    /// Location of the mapping value.
    pub value: ValueLocations,
}

impl LocatedValue {
    /// Returns the value span addressed by an RFC 6901 JSON Pointer.
    ///
    /// Scalar YAML mapping keys use their canonical property-name text.
    /// Complex keys, invalid pointer escapes, and ambiguous projected names do
    /// not resolve.
    #[must_use]
    pub fn span_at_pointer(&self, pointer: &str) -> Option<Span> {
        if pointer.is_empty() {
            return Some(self.locations.span);
        }
        let remainder = pointer.strip_prefix('/')?;
        let mut value = &self.value;
        let mut locations = &self.locations;

        for raw_segment in remainder.split('/') {
            let segment = decode_pointer_segment(raw_segment)?;
            match (value, &locations.kind) {
                (Value::Sequence(items), LocationKind::Sequence(children)) => {
                    let index = parse_sequence_index(&segment)?;
                    value = items.get(index)?;
                    locations = children.get(index)?;
                }
                (Value::Mapping(mapping), LocationKind::Mapping(entries)) => {
                    let mut matched = None;
                    for ((key, child), entry) in mapping.iter().zip(entries) {
                        if key.canonical_scalar_text().as_deref() != Some(segment.as_str()) {
                            continue;
                        }
                        if matched.is_some() {
                            return None;
                        }
                        matched = Some((child, &entry.value));
                    }
                    let (child, child_locations) = matched?;
                    value = child;
                    locations = child_locations;
                }
                _ => return None,
            }
        }

        Some(locations.span)
    }
}

fn decode_pointer_segment(segment: &str) -> Option<String> {
    let mut decoded = String::with_capacity(segment.len());
    let mut chars = segment.chars();
    while let Some(character) = chars.next() {
        if character != '~' {
            decoded.push(character);
            continue;
        }
        match chars.next()? {
            '0' => decoded.push('~'),
            '1' => decoded.push('/'),
            _ => return None,
        }
    }
    Some(decoded)
}

fn parse_sequence_index(segment: &str) -> Option<usize> {
    if segment.len() > 1 && segment.starts_with('0') {
        return None;
    }
    segment.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mapping;

    fn scalar(span: Span) -> ValueLocations {
        ValueLocations {
            span,
            kind: LocationKind::Scalar,
        }
    }

    #[test]
    fn ambiguous_projected_keys_do_not_resolve() {
        let mut mapping = Mapping::new();
        mapping.insert(Value::Integer(1), Value::Null);
        mapping.insert(Value::from("1"), Value::Null);
        let located = LocatedValue {
            value: Value::Mapping(mapping),
            locations: ValueLocations {
                span: Span::default(),
                kind: LocationKind::Mapping(vec![
                    MappingLocations {
                        key: scalar(Span::default()),
                        value: scalar(Span::default()),
                    },
                    MappingLocations {
                        key: scalar(Span::default()),
                        value: scalar(Span::default()),
                    },
                ]),
            },
        };

        assert_eq!(located.span_at_pointer("/1"), None);
    }

    #[test]
    fn invalid_pointer_escapes_and_array_indices_do_not_resolve() {
        let located = LocatedValue {
            value: Value::Sequence(vec![Value::Null]),
            locations: ValueLocations {
                span: Span::default(),
                kind: LocationKind::Sequence(vec![scalar(Span::default())]),
            },
        };

        assert_eq!(located.span_at_pointer("/00"), None);
        assert_eq!(located.span_at_pointer("/~2"), None);
    }
}

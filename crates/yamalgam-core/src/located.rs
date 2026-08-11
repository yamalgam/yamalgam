//! Source locations parallel to the semantic [`Value`](crate::Value) tree.

use crate::{Span, Value};

/// A semantic YAML value and its value-shaped source-location tree.
#[derive(Debug, Clone, PartialEq)]
pub struct LocatedValue {
    /// Composed semantic value.
    value: Value,
    /// Source locations with the same shape and child order as `value`.
    locations: ValueLocations,
}

/// Why semantic and source-location trees could not be paired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationShapeError {
    /// A semantic node and its location node have different kinds.
    KindMismatch,
    /// A collection and its location node have different child counts.
    ChildCountMismatch {
        /// Number of semantic children or entries.
        values: usize,
        /// Number of source-location children or entries.
        locations: usize,
    },
}

impl std::fmt::Display for LocationShapeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KindMismatch => formatter.write_str("value and location kinds do not match"),
            Self::ChildCountMismatch { values, locations } => write!(
                formatter,
                "value and location child counts do not match ({values} != {locations})"
            ),
        }
    }
}

impl std::error::Error for LocationShapeError {}

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
    /// Pair a semantic value with a checked, value-shaped location tree.
    pub fn new(value: Value, locations: ValueLocations) -> Result<Self, LocationShapeError> {
        validate_shape(&value, &locations)?;
        Ok(Self { value, locations })
    }

    /// Build a located scalar.
    pub fn scalar(value: Value, span: Span) -> Result<Self, LocationShapeError> {
        Self::new(
            value,
            ValueLocations {
                span,
                kind: LocationKind::Scalar,
            },
        )
    }

    /// Build a located sequence from already-checked children.
    #[must_use]
    pub fn sequence(items: Vec<Self>, span: Span) -> Self {
        let (values, locations) = items.into_iter().map(Self::into_parts).unzip();
        Self {
            value: Value::Sequence(values),
            locations: ValueLocations {
                span,
                kind: LocationKind::Sequence(locations),
            },
        }
    }

    /// Build a located mapping from already-checked key-value pairs.
    #[must_use]
    pub fn mapping(entries: Vec<(Self, Self)>, span: Span) -> Self {
        let mut mapping = crate::Mapping::new();
        let mut locations = Vec::with_capacity(entries.len());
        for (key, value) in entries {
            let (key, key_locations) = key.into_parts();
            let (value, value_locations) = value.into_parts();
            let existing = mapping.iter().position(|(candidate, _)| candidate == &key);
            mapping.insert(key, value);
            let entry = MappingLocations {
                key: key_locations,
                value: value_locations,
            };
            if let Some(index) = existing {
                locations[index] = entry;
            } else {
                locations.push(entry);
            }
        }
        Self {
            value: Value::Mapping(mapping),
            locations: ValueLocations {
                span,
                kind: LocationKind::Mapping(locations),
            },
        }
    }

    /// Borrow the semantic value.
    #[must_use]
    pub const fn value(&self) -> &Value {
        &self.value
    }

    /// Borrow the value-shaped location tree.
    #[must_use]
    pub const fn locations(&self) -> &ValueLocations {
        &self.locations
    }

    /// Consume the located value and return its semantic value.
    #[must_use]
    pub fn into_value(self) -> Value {
        self.value
    }

    /// Consume the located value and return both checked trees.
    #[must_use]
    pub fn into_parts(self) -> (Value, ValueLocations) {
        (self.value, self.locations)
    }

    /// Replace only the root span while preserving the checked tree shape.
    #[must_use]
    pub const fn with_root_span(mut self, span: Span) -> Self {
        self.locations.span = span;
        self
    }

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

fn validate_shape(value: &Value, locations: &ValueLocations) -> Result<(), LocationShapeError> {
    let mut pending = vec![(value, locations)];
    while let Some((value, locations)) = pending.pop() {
        match (value, &locations.kind) {
            (
                Value::Null
                | Value::Bool(_)
                | Value::Integer(_)
                | Value::Float(_)
                | Value::String(_),
                LocationKind::Scalar,
            ) => {}
            (Value::Sequence(values), LocationKind::Sequence(child_locations)) => {
                if values.len() != child_locations.len() {
                    return Err(LocationShapeError::ChildCountMismatch {
                        values: values.len(),
                        locations: child_locations.len(),
                    });
                }
                pending.extend(values.iter().zip(child_locations));
            }
            (Value::Mapping(values), LocationKind::Mapping(entry_locations)) => {
                if values.len() != entry_locations.len() {
                    return Err(LocationShapeError::ChildCountMismatch {
                        values: values.len(),
                        locations: entry_locations.len(),
                    });
                }
                for ((key, value), locations) in values.iter().zip(entry_locations) {
                    pending.push((key, &locations.key));
                    pending.push((value, &locations.value));
                }
            }
            _ => return Err(LocationShapeError::KindMismatch),
        }
    }
    Ok(())
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
    fn checked_construction_rejects_mismatched_shapes() {
        let wrong_kind =
            LocatedValue::new(Value::Sequence(vec![Value::Null]), scalar(Span::default()));
        assert_eq!(wrong_kind, Err(LocationShapeError::KindMismatch));

        let wrong_length = LocatedValue::new(
            Value::Sequence(vec![Value::Null]),
            ValueLocations {
                span: Span::default(),
                kind: LocationKind::Sequence(Vec::new()),
            },
        );
        assert_eq!(
            wrong_length,
            Err(LocationShapeError::ChildCountMismatch {
                values: 1,
                locations: 0,
            })
        );

        let mut mapping = Mapping::new();
        mapping.insert(Value::from("key"), Value::Null);
        let nested_mismatch = LocatedValue::new(
            Value::Mapping(mapping),
            ValueLocations {
                span: Span::default(),
                kind: LocationKind::Mapping(vec![MappingLocations {
                    key: scalar(Span::default()),
                    value: ValueLocations {
                        span: Span::default(),
                        kind: LocationKind::Sequence(Vec::new()),
                    },
                }]),
            },
        );
        assert_eq!(nested_mismatch, Err(LocationShapeError::KindMismatch));
    }

    #[test]
    fn ambiguous_projected_keys_do_not_resolve() {
        let mut mapping = Mapping::new();
        mapping.insert(Value::Integer(1), Value::Null);
        mapping.insert(Value::from("1"), Value::Null);
        let located = LocatedValue::new(
            Value::Mapping(mapping),
            ValueLocations {
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
        )
        .unwrap();

        assert_eq!(located.span_at_pointer("/1"), None);
    }

    #[test]
    fn invalid_pointer_escapes_and_array_indices_do_not_resolve() {
        let located = LocatedValue::new(
            Value::Sequence(vec![Value::Null]),
            ValueLocations {
                span: Span::default(),
                kind: LocationKind::Sequence(vec![scalar(Span::default())]),
            },
        )
        .unwrap();

        assert_eq!(located.span_at_pointer("/00"), None);
        assert_eq!(located.span_at_pointer("/~2"), None);
    }
}

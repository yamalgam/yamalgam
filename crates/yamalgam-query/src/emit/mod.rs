//! YAML and JSON output serialization.

// r[impl output]

mod json;
mod yaml;

pub use json::to_json;
pub use yaml::to_yaml;

use std::collections::HashSet;
use std::rc::Rc;

use crate::value::{Data, YamlVal, YgMap};

/// Serialization failure caused by an unsafe or lossy output request.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EmitError {
    /// A configured resource limit was exceeded.
    #[error("{0}")]
    Limit(#[source] yamalgam_core::LimitExceeded),
    /// Resource-accounting arithmetic overflowed before a configured limit
    /// could be checked.
    #[error("{0} overflow")]
    ResourceOverflow(&'static str),
    /// Distinct YAML mapping keys would collapse to one JSON object name.
    #[error("JSON object key {0:?} would be duplicated after string conversion")]
    DuplicateJsonKey(String),
}

/// Output format selector for the CLI.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EmitFormat {
    /// YAML output.
    #[default]
    Yaml,
    /// JSON output with aliases materialized.
    Json,
}

/// Options shared by the YAML and JSON emitters.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct EmitOpts {
    /// Number of spaces in each indentation level.
    pub indent: usize,
    /// Emit a single-line representation.
    pub compact: bool,
    /// Sort mapping entries by resolved key value.
    pub sort_keys: bool,
    /// Enable ANSI color in supported emitters.
    pub color: bool,
    /// Schema under which emitted plain scalars must remain strings.
    pub schema: yamalgam_core::TagResolution,
    /// Resource limits applied while validating and rendering output.
    pub limits: yamalgam_core::ResourceLimits,
}

impl Default for EmitOpts {
    fn default() -> Self {
        Self {
            indent: 2,
            compact: false,
            sort_keys: false,
            color: false,
            schema: yamalgam_core::TagResolution::Yaml12,
            limits: yamalgam_core::LoaderConfig::moderate().limits,
        }
    }
}

/// Render a value using the selected output format.
pub fn emit(value: &YamlVal, format: EmitFormat, options: &EmitOpts) -> Result<String, EmitError> {
    match format {
        EmitFormat::Yaml => to_yaml(value, options),
        EmitFormat::Json => to_json(value, options),
    }
}

enum MapEntries<'mapping> {
    Direct(indexmap::map::Iter<'mapping, YamlVal, YamlVal>),
    Sorted(std::vec::IntoIter<(&'mapping YamlVal, &'mapping YamlVal)>),
}

fn ordered_entries(mapping: &YgMap, sort_keys: bool) -> MapEntries<'_> {
    if sort_keys {
        let mut entries = mapping.0.iter().collect::<Vec<_>>();
        entries.sort_by(|left, right| left.0.cmp(right.0));
        MapEntries::Sorted(entries.into_iter())
    } else {
        MapEntries::Direct(mapping.0.iter())
    }
}

impl<'mapping> Iterator for MapEntries<'mapping> {
    type Item = (&'mapping YamlVal, &'mapping YamlVal);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Direct(entries) => entries.next(),
            Self::Sorted(entries) => entries.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Direct(entries) => entries.size_hint(),
            Self::Sorted(entries) => entries.size_hint(),
        }
    }
}

impl DoubleEndedIterator for MapEntries<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self {
            Self::Direct(entries) => entries.next_back(),
            Self::Sorted(entries) => entries.next_back(),
        }
    }
}

impl ExactSizeIterator for MapEntries<'_> {}

fn validate_value(
    value: &YamlVal,
    options: &EmitOpts,
    format: EmitFormat,
) -> Result<(), EmitError> {
    let mut stack = vec![(value, 0_usize)];
    let mut anchors = HashSet::new();
    let mut expanded_aliases = 0_usize;

    while let Some((value, depth)) = stack.pop() {
        options
            .limits
            .check_depth(depth)
            .map_err(EmitError::Limit)?;

        if let Some(metadata) = value.meta() {
            if let Some(alias) = metadata.alias_of.as_deref() {
                let emits_reference = format == EmitFormat::Yaml && anchors.contains(alias);
                if emits_reference {
                    continue;
                }
                expanded_aliases = expanded_aliases
                    .checked_add(1)
                    .ok_or(EmitError::ResourceOverflow("alias expansion count"))?;
                options
                    .limits
                    .check_alias_expansions(expanded_aliases)
                    .map_err(EmitError::Limit)?;
            } else if let Some(anchor) = metadata.anchor.as_ref() {
                anchors.insert(Rc::clone(anchor));
            }
        }

        match &value.data {
            Data::Str(string) => options
                .limits
                .check_scalar_size(string.len())
                .map_err(EmitError::Limit)?,
            Data::Seq(items) => {
                options
                    .limits
                    .check_collection_size(items.len())
                    .map_err(EmitError::Limit)?;
                let child_depth = depth
                    .checked_add(1)
                    .ok_or(EmitError::ResourceOverflow("value depth"))?;
                stack.extend(items.iter().rev().map(|item| (item, child_depth)));
            }
            Data::Map(mapping) => {
                options
                    .limits
                    .check_collection_size(mapping.0.len())
                    .map_err(EmitError::Limit)?;
                let child_depth = depth
                    .checked_add(1)
                    .ok_or(EmitError::ResourceOverflow("value depth"))?;
                for (key, value) in ordered_entries(mapping, options.sort_keys).rev() {
                    stack.push((value, child_depth));
                    stack.push((key, child_depth));
                }
            }
            Data::Null | Data::Bool(_) | Data::Int(_) | Data::Float(_) => {}
        }
    }

    Ok(())
}

impl EmitOpts {
    /// Construct compact single-line output options.
    #[must_use]
    pub fn compact() -> Self {
        Self {
            compact: true,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MapEntries, ordered_entries};
    use crate::value::{Data, YamlVal};

    #[test]
    fn ordered_entries_borrows_unsorted_mappings() {
        let value = YamlVal::map(vec![
            (YamlVal::str("b"), YamlVal::int(1)),
            (YamlVal::str("a"), YamlVal::int(2)),
        ]);
        let Data::Map(mapping) = &value.data else {
            panic!("fixture must be a mapping");
        };

        assert!(matches!(
            ordered_entries(mapping, false),
            MapEntries::Direct(_)
        ));
        assert_eq!(
            ordered_entries(mapping, true)
                .map(|(key, _)| key.clone())
                .collect::<Vec<_>>(),
            vec![YamlVal::str("a"), YamlVal::str("b")]
        );
    }
}

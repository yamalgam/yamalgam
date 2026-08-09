//! Typed `!include` directive parsing and aggregation.

use std::collections::HashSet;
use std::mem;
use std::path::{Path, PathBuf};

use yamalgam_core::{TagResolution, TagResolver, Value};
use yamalgam_parser::{AtomFlags, Chomp, CollectionStyle, ScalarStyle};

use crate::ResourceId;
use crate::document::{OwnedKind, OwnedNode, OwnedScalar};
use crate::error::SessionError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IncludeSelector {
    Path(String),
    Glob(String),
    Dir(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IncludeFormat {
    Yaml,
    Text,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IncludeMode {
    Node,
    List,
    Named,
    Merge,
    Splice,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IncludeKey {
    Stem,
    Relative,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConflictPolicy {
    Error,
    First,
    Last,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncludeSpec {
    pub selector: IncludeSelector,
    pub format: IncludeFormat,
    pub mode: IncludeMode,
    pub recursive: bool,
    pub matches: Vec<String>,
    pub key: IncludeKey,
    pub on_conflict: ConflictPolicy,
    pub on_conflict_explicit: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IncludeContext {
    Node,
    SequenceItem,
    MergeValue,
}

pub enum IncludeExpansion {
    Node {
        node: OwnedNode,
        additional_weight: usize,
    },
    Splice {
        items: Vec<OwnedNode>,
        additional_weight: usize,
    },
    MappingMerge {
        entries: Vec<(OwnedNode, OwnedNode)>,
        additional_weight: usize,
    },
}

pub fn recognize(
    node: &OwnedNode,
    tag_resolution: TagResolution,
) -> Result<Option<IncludeSpec>, SessionError> {
    if node.tag.as_deref() != Some("!include") {
        return Ok(None);
    }
    match &node.kind {
        OwnedKind::Scalar(scalar) => {
            if !payload_is_string(node, scalar, tag_resolution) {
                return Err(SessionError::MalformedInclude(
                    "scalar shorthand requires a string path",
                ));
            }
            Ok(Some(IncludeSpec {
                selector: IncludeSelector::Path(scalar.value.clone()),
                format: IncludeFormat::Yaml,
                mode: IncludeMode::Node,
                recursive: false,
                matches: Vec::new(),
                key: IncludeKey::Stem,
                on_conflict: ConflictPolicy::Error,
                on_conflict_explicit: false,
            }))
        }
        OwnedKind::Mapping { entries, .. } => parse_mapping(entries, tag_resolution).map(Some),
        _ => Err(SessionError::MalformedInclude(
            "directive must tag a scalar or mapping",
        )),
    }
}

fn parse_mapping(
    entries: &[(OwnedNode, OwnedNode)],
    tag_resolution: TagResolution,
) -> Result<IncludeSpec, SessionError> {
    let mut seen = HashSet::new();
    let mut selector = None;
    let mut format = IncludeFormat::Yaml;
    let mut mode = IncludeMode::Node;
    let mut recursive = false;
    let mut recursive_set = false;
    let mut matches = Vec::new();
    let mut matches_set = false;
    let mut key = IncludeKey::Stem;
    let mut key_set = false;
    let mut on_conflict = ConflictPolicy::Error;
    let mut on_conflict_explicit = false;

    for (key_node, value) in entries {
        let key_name = semantic_string(key_node, tag_resolution).ok_or(
            SessionError::MalformedInclude("option name must be a string"),
        )?;
        if !seen.insert(key_name.to_owned()) {
            return Err(SessionError::MalformedInclude("duplicate option"));
        }
        match key_name {
            "path" => set_selector(
                &mut selector,
                IncludeSelector::Path(required_string(value, tag_resolution)?),
            )?,
            "glob" => set_selector(
                &mut selector,
                IncludeSelector::Glob(required_string(value, tag_resolution)?),
            )?,
            "dir" => set_selector(
                &mut selector,
                IncludeSelector::Dir(required_string(value, tag_resolution)?),
            )?,
            "format" => {
                format = match required_string(value, tag_resolution)?.as_str() {
                    "yaml" => IncludeFormat::Yaml,
                    "text" => IncludeFormat::Text,
                    _ => return Err(SessionError::MalformedInclude("invalid format")),
                };
            }
            "mode" => {
                mode = match required_string(value, tag_resolution)?.as_str() {
                    "node" => IncludeMode::Node,
                    "list" => IncludeMode::List,
                    "named" => IncludeMode::Named,
                    "merge" => IncludeMode::Merge,
                    "splice" => IncludeMode::Splice,
                    _ => return Err(SessionError::MalformedInclude("invalid mode")),
                };
            }
            "recursive" => {
                recursive = required_bool(value)?;
                recursive_set = true;
            }
            "match" => {
                matches = required_strings(value, tag_resolution)?;
                matches_set = true;
            }
            "key" => {
                key = match required_string(value, tag_resolution)?.as_str() {
                    "stem" => IncludeKey::Stem,
                    "relative" => IncludeKey::Relative,
                    _ => return Err(SessionError::MalformedInclude("invalid key mode")),
                };
                key_set = true;
            }
            "on_conflict" => {
                on_conflict = match required_string(value, tag_resolution)?.as_str() {
                    "error" => ConflictPolicy::Error,
                    "first" => ConflictPolicy::First,
                    "last" => ConflictPolicy::Last,
                    _ => {
                        return Err(SessionError::MalformedInclude("invalid conflict policy"));
                    }
                };
                on_conflict_explicit = true;
            }
            _ => return Err(SessionError::MalformedInclude("unknown option")),
        }
    }

    let selector = selector.ok_or(SessionError::MalformedInclude("missing selector"))?;
    let is_dir = matches!(selector, IncludeSelector::Dir(_));
    if (recursive_set || matches_set) && !is_dir {
        return Err(SessionError::MalformedInclude(
            "recursive and match require dir",
        ));
    }
    if key_set && mode != IncludeMode::Named {
        return Err(SessionError::MalformedInclude("key requires named mode"));
    }
    if on_conflict_explicit && mode != IncludeMode::Merge {
        return Err(SessionError::MalformedInclude(
            "on_conflict requires merge mode",
        ));
    }
    if format == IncludeFormat::Text
        && !matches!(
            mode,
            IncludeMode::Node | IncludeMode::List | IncludeMode::Named
        )
    {
        return Err(SessionError::MalformedInclude(
            "text supports node, list, or named mode",
        ));
    }

    Ok(IncludeSpec {
        selector,
        format,
        mode,
        recursive,
        matches,
        key,
        on_conflict,
        on_conflict_explicit,
    })
}

fn set_selector(
    selector: &mut Option<IncludeSelector>,
    value: IncludeSelector,
) -> Result<(), SessionError> {
    if selector.replace(value).is_some() {
        return Err(SessionError::MalformedInclude(
            "exactly one selector is required",
        ));
    }
    Ok(())
}

fn required_string(
    node: &OwnedNode,
    tag_resolution: TagResolution,
) -> Result<String, SessionError> {
    semantic_string(node, tag_resolution)
        .map(str::to_owned)
        .ok_or(SessionError::MalformedInclude("option requires a string"))
}

fn required_strings(
    node: &OwnedNode,
    tag_resolution: TagResolution,
) -> Result<Vec<String>, SessionError> {
    match &node.kind {
        OwnedKind::Scalar(_) => Ok(vec![required_string(node, tag_resolution)?]),
        OwnedKind::Sequence { items, .. } => items
            .iter()
            .map(|item| required_string(item, tag_resolution))
            .collect(),
        _ => Err(SessionError::MalformedInclude(
            "match requires a string or string sequence",
        )),
    }
}

fn required_bool(node: &OwnedNode) -> Result<bool, SessionError> {
    let OwnedKind::Scalar(scalar) = &node.kind else {
        return Err(SessionError::MalformedInclude(
            "recursive requires a boolean",
        ));
    };
    if scalar.style != ScalarStyle::Plain {
        return Err(SessionError::MalformedInclude(
            "recursive requires a boolean",
        ));
    }
    match (node.tag.as_deref(), scalar.value.as_str()) {
        (None | Some("tag:yaml.org,2002:bool" | "!!bool"), "true") => Ok(true),
        (None | Some("tag:yaml.org,2002:bool" | "!!bool"), "false") => Ok(false),
        _ => Err(SessionError::MalformedInclude(
            "recursive requires a boolean",
        )),
    }
}

fn payload_is_string(
    node: &OwnedNode,
    scalar: &OwnedScalar,
    tag_resolution: TagResolution,
) -> bool {
    scalar.style != ScalarStyle::Plain
        || tag_resolution.resolves_as_string(&scalar.value)
        || node.tag.as_deref() == Some("tag:yaml.org,2002:str")
}

fn semantic_string(node: &OwnedNode, tag_resolution: TagResolution) -> Option<&str> {
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

pub fn aggregate(
    spec: &IncludeSpec,
    context: IncludeContext,
    directive: &OwnedNode,
    resources: Vec<(ResourceId, OwnedNode)>,
    tag_resolution: TagResolution,
) -> Result<IncludeExpansion, SessionError> {
    if spec.mode == IncludeMode::Node && resources.len() != 1 {
        return Err(SessionError::IncludeMatchCount {
            expected: 1,
            observed: resources.len(),
        });
    }
    if resources.is_empty() {
        return Err(SessionError::IncludeMatchCount {
            expected: 1,
            observed: 0,
        });
    }

    let expansion = match spec.mode {
        IncludeMode::Node => IncludeExpansion::Node {
            node: resources.into_iter().next().expect("length checked").1,
            additional_weight: 0,
        },
        IncludeMode::List => {
            let items = resources
                .into_iter()
                .map(|(_, node)| node)
                .collect::<Vec<_>>();
            let additional_weight = sequence_wrapper_weight(items.len())?;
            IncludeExpansion::Node {
                node: synthetic_sequence(directive, items),
                additional_weight,
            }
        }
        IncludeMode::Named => aggregate_named(spec, directive, resources)?,
        IncludeMode::Merge => aggregate_merge(spec, directive, resources, tag_resolution)?,
        IncludeMode::Splice => {
            if context != IncludeContext::SequenceItem {
                return Err(SessionError::IncludeContext);
            }
            let mut items = Vec::new();
            for (_, node) in resources {
                let OwnedKind::Sequence {
                    items: included, ..
                } = node.kind
                else {
                    return Err(SessionError::IncludeAggregateType);
                };
                items.extend(included);
            }
            let additional_weight = items
                .len()
                .checked_mul(mem::size_of::<OwnedNode>())
                .ok_or(SessionError::ResourceOverflow("include splice storage"))?;
            IncludeExpansion::Splice {
                items,
                additional_weight,
            }
        }
    };

    if context == IncludeContext::MergeValue {
        let IncludeExpansion::Node {
            node,
            additional_weight,
        } = expansion
        else {
            return Err(SessionError::IncludeAggregateType);
        };
        let OwnedKind::Mapping { entries, .. } = node.kind else {
            return Err(SessionError::IncludeAggregateType);
        };
        return Ok(IncludeExpansion::MappingMerge {
            entries,
            additional_weight,
        });
    }
    Ok(expansion)
}

pub fn validate_names(
    spec: &IncludeSpec,
    directive: &OwnedNode,
    resources: &[ResourceId],
) -> Result<(), SessionError> {
    if spec.mode != IncludeMode::Named {
        return Ok(());
    }
    let base = selector_base(&directive.origin, &spec.selector)?;
    let mut names = HashSet::new();
    for resource in resources {
        if !names.insert(derive_name(resource, &base, spec.key)?) {
            return Err(SessionError::IncludeConflict);
        }
    }
    Ok(())
}

fn aggregate_named(
    spec: &IncludeSpec,
    directive: &OwnedNode,
    resources: Vec<(ResourceId, OwnedNode)>,
) -> Result<IncludeExpansion, SessionError> {
    let base = selector_base(&directive.origin, &spec.selector)?;
    let mut names = HashSet::new();
    let mut entries = Vec::with_capacity(resources.len());
    let mut key_weight = 0_usize;
    for (resource, node) in resources {
        let name = derive_name(&resource, &base, spec.key)?;
        if !names.insert(name.clone()) {
            return Err(SessionError::IncludeConflict);
        }
        let key = synthetic_scalar(directive, name);
        key_weight = key_weight
            .checked_add(
                key.materialized_weight()
                    .map_err(|_| SessionError::ResourceOverflow("include named key weight"))?,
            )
            .ok_or(SessionError::ResourceOverflow("include named key weight"))?;
        entries.push((key, node));
    }
    let storage = mapping_wrapper_weight(entries.len())?;
    let additional_weight = storage
        .checked_add(key_weight)
        .ok_or(SessionError::ResourceOverflow("include named storage"))?;
    Ok(IncludeExpansion::Node {
        node: synthetic_mapping(directive, entries),
        additional_weight,
    })
}

fn aggregate_merge(
    spec: &IncludeSpec,
    directive: &OwnedNode,
    resources: Vec<(ResourceId, OwnedNode)>,
    tag_resolution: TagResolution,
) -> Result<IncludeExpansion, SessionError> {
    let mapping = resources
        .first()
        .is_some_and(|(_, node)| matches!(node.kind, OwnedKind::Mapping { .. }));
    let sequence = resources
        .first()
        .is_some_and(|(_, node)| matches!(node.kind, OwnedKind::Sequence { .. }));
    if !mapping && !sequence {
        return Err(SessionError::IncludeAggregateType);
    }
    if mapping {
        let mut merged = Vec::new();
        for (_, node) in resources {
            let OwnedKind::Mapping { entries, .. } = node.kind else {
                return Err(SessionError::IncludeAggregateType);
            };
            for entry in entries {
                if let Some(position) = merged
                    .iter()
                    .position(|(key, _)| semantic_key_eq(key, &entry.0, tag_resolution))
                {
                    match spec.on_conflict {
                        ConflictPolicy::Error => return Err(SessionError::IncludeConflict),
                        ConflictPolicy::First => {}
                        ConflictPolicy::Last => merged[position] = entry,
                    }
                } else {
                    merged.push(entry);
                }
            }
        }
        let additional_weight = mapping_wrapper_weight(merged.len())?;
        return Ok(IncludeExpansion::Node {
            node: synthetic_mapping(directive, merged),
            additional_weight,
        });
    }
    if spec.on_conflict_explicit {
        return Err(SessionError::MalformedInclude(
            "on_conflict applies only to mapping merge",
        ));
    }
    let mut items = Vec::new();
    for (_, node) in resources {
        let OwnedKind::Sequence {
            items: included, ..
        } = node.kind
        else {
            return Err(SessionError::IncludeAggregateType);
        };
        items.extend(included);
    }
    let additional_weight = sequence_wrapper_weight(items.len())?;
    Ok(IncludeExpansion::Node {
        node: synthetic_sequence(directive, items),
        additional_weight,
    })
}

fn synthetic_scalar(directive: &OwnedNode, value: String) -> OwnedNode {
    OwnedNode {
        origin: directive.origin.clone(),
        span: directive.span,
        anchor: None,
        tag: None,
        kind: OwnedKind::Scalar(OwnedScalar {
            value,
            style: ScalarStyle::DoubleQuoted,
            chomp: Chomp::Clip,
            flags: AtomFlags::empty(),
        }),
    }
}

fn synthetic_sequence(directive: &OwnedNode, items: Vec<OwnedNode>) -> OwnedNode {
    OwnedNode {
        origin: directive.origin.clone(),
        span: directive.span,
        anchor: None,
        tag: None,
        kind: OwnedKind::Sequence {
            style: CollectionStyle::Block,
            items,
        },
    }
}

fn synthetic_mapping(directive: &OwnedNode, entries: Vec<(OwnedNode, OwnedNode)>) -> OwnedNode {
    OwnedNode {
        origin: directive.origin.clone(),
        span: directive.span,
        anchor: None,
        tag: None,
        kind: OwnedKind::Mapping {
            style: CollectionStyle::Block,
            entries,
        },
    }
}

fn sequence_wrapper_weight(len: usize) -> Result<usize, SessionError> {
    mem::size_of::<OwnedNode>()
        .checked_add(
            len.checked_mul(mem::size_of::<OwnedNode>())
                .ok_or(SessionError::ResourceOverflow("include sequence storage"))?,
        )
        .ok_or(SessionError::ResourceOverflow("include sequence storage"))
}

fn mapping_wrapper_weight(len: usize) -> Result<usize, SessionError> {
    mem::size_of::<OwnedNode>()
        .checked_add(
            len.checked_mul(mem::size_of::<(OwnedNode, OwnedNode)>())
                .ok_or(SessionError::ResourceOverflow("include mapping storage"))?,
        )
        .ok_or(SessionError::ResourceOverflow("include mapping storage"))
}

fn selector_base(origin: &ResourceId, selector: &IncludeSelector) -> Result<PathBuf, SessionError> {
    let origin = origin.as_path().ok_or(SessionError::IncludeName)?;
    let parent = origin.parent().unwrap_or_else(|| Path::new(""));
    match selector {
        IncludeSelector::Path(_) => Ok(parent.to_path_buf()),
        IncludeSelector::Dir(path) => Ok(parent.join(path)),
        IncludeSelector::Glob(pattern) => {
            let pattern_path = Path::new(pattern);
            let mut saw_pattern = false;
            let mut static_prefix = pattern_path
                .components()
                .take_while(|component| {
                    let value = component.as_os_str().to_string_lossy();
                    let is_pattern = value.contains(['*', '?', '[', ']']);
                    saw_pattern |= is_pattern;
                    !is_pattern
                })
                .collect::<PathBuf>();
            if !saw_pattern {
                static_prefix.pop();
            }
            Ok(parent.join(static_prefix))
        }
    }
}

fn derive_name(
    resource: &ResourceId,
    base: &Path,
    key: IncludeKey,
) -> Result<String, SessionError> {
    let path = resource.as_path().ok_or(SessionError::IncludeName)?;
    let selected = match key {
        IncludeKey::Stem => PathBuf::from(path.file_stem().ok_or(SessionError::IncludeName)?),
        IncludeKey::Relative => {
            let relative = path
                .strip_prefix(base)
                .map_err(|_| SessionError::IncludeName)?;
            let mut relative = relative.to_path_buf();
            relative.set_extension("");
            relative
        }
    };
    let components = selected
        .components()
        .map(|component| {
            component
                .as_os_str()
                .to_str()
                .ok_or(SessionError::IncludeName)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if components.is_empty() {
        return Err(SessionError::IncludeName);
    }
    Ok(components.join("/"))
}

pub fn semantic_key_eq(left: &OwnedNode, right: &OwnedNode, tag_resolution: TagResolution) -> bool {
    match (&left.kind, &right.kind) {
        (OwnedKind::Scalar(left_scalar), OwnedKind::Scalar(right_scalar)) => {
            match (
                scalar_semantic_value(left, left_scalar, tag_resolution),
                scalar_semantic_value(right, right_scalar, tag_resolution),
            ) {
                (Some(left), Some(right)) => left == right,
                _ => left_scalar.value == right_scalar.value && left.tag == right.tag,
            }
        }
        (OwnedKind::Alias { name: left }, OwnedKind::Alias { name: right }) => left == right,
        (OwnedKind::Sequence { items: left, .. }, OwnedKind::Sequence { items: right, .. }) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| semantic_key_eq(left, right, tag_resolution))
        }
        (OwnedKind::Mapping { entries: left, .. }, OwnedKind::Mapping { entries: right, .. }) => {
            left.len() == right.len()
                && left.iter().zip(right).all(|(left, right)| {
                    semantic_key_eq(&left.0, &right.0, tag_resolution)
                        && semantic_key_eq(&left.1, &right.1, tag_resolution)
                })
        }
        _ => false,
    }
}

fn scalar_semantic_value(
    node: &OwnedNode,
    scalar: &OwnedScalar,
    tag_resolution: TagResolution,
) -> Option<Value> {
    match node.tag.as_deref() {
        Some("tag:yaml.org,2002:str" | "!!str") => Some(Value::String(scalar.value.clone())),
        Some(_) => None,
        None if scalar.style == ScalarStyle::Plain => {
            Some(tag_resolution.resolve_scalar(&scalar.value))
        }
        None => Some(Value::String(scalar.value.clone())),
    }
}

//! Indexed JSON Schema navigation for editor-facing features.

use std::collections::{HashMap, HashSet, VecDeque};

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use url::Url;
use yamalgam_core::{LocatedValue, Span};

/// One property completion candidate.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PropertyCandidate {
    /// Property name as declared by the schema.
    pub name: String,
    /// First applicable property description.
    pub description: Option<String>,
    /// Whether every applicable constraint requires this property.
    pub required: bool,
    /// Compact JSON representation of the default value.
    pub default: Option<String>,
}

/// Schema annotations applicable at one instance path.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchemaContext {
    /// First applicable title.
    pub title: Option<String>,
    /// First applicable description.
    pub description: Option<String>,
    /// De-duplicated JSON Schema types in source order.
    pub types: Vec<String>,
    /// Compact JSON representation of the first applicable default.
    pub default: Option<String>,
    /// Compact JSON representations of allowed enum values.
    pub enum_values: Vec<String>,
    /// Whether an applicable schema marks this value as deprecated.
    pub deprecated: bool,
    /// Property candidates sorted required-first, then by source order.
    pub properties: Vec<PropertyCandidate>,
    /// Canonical schema locations contributing to this context.
    pub schema_locations: Vec<String>,
}

/// Indexed location of a schema node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaLocation {
    /// Canonical fragmentless resource URI.
    pub uri: String,
    /// RFC 6901 pointer within the resource.
    pub pointer: String,
    /// Source span when the schema was loaded from located YAML.
    pub span: Option<Span>,
}

#[derive(Clone, Debug)]
pub struct NavigationResource {
    pub uri: String,
    pub schema: JsonValue,
    pub located: Option<LocatedValue>,
}

#[derive(Clone, Debug)]
struct IndexedResource {
    schema: JsonValue,
    located: Option<LocatedValue>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Cursor {
    uri: String,
    pointer: String,
    alternative: bool,
}

#[derive(Debug, Default)]
pub struct NavigationIndex {
    root_uri: Option<String>,
    resources: HashMap<String, IndexedResource>,
    definitions: HashMap<String, SchemaLocation>,
    scopes: HashMap<(String, String), String>,
    aliases: HashMap<String, String>,
}

impl NavigationIndex {
    pub(super) fn single(
        schema: JsonValue,
        located: Option<LocatedValue>,
        base_uri: Option<&str>,
    ) -> Self {
        let uri = base_uri
            .and_then(fragmentless_uri)
            .unwrap_or_else(|| "urn:yamalgam:schema:root".to_owned());
        Self::from_resources(
            Some(&uri),
            vec![NavigationResource {
                uri: uri.clone(),
                schema,
                located,
            }],
            HashMap::new(),
        )
    }

    pub(super) fn from_resources(
        root_uri: Option<&str>,
        resources: Vec<NavigationResource>,
        aliases: HashMap<String, String>,
    ) -> Self {
        let mut index = Self {
            root_uri: root_uri.and_then(fragmentless_uri),
            aliases,
            ..Self::default()
        };
        for resource in resources {
            let uri = fragmentless_uri(&resource.uri).unwrap_or(resource.uri);
            index.resources.insert(
                uri,
                IndexedResource {
                    schema: resource.schema,
                    located: resource.located,
                },
            );
        }
        let uris = index.resources.keys().cloned().collect::<Vec<_>>();
        for uri in uris {
            index.index_resource(&uri);
        }
        index
    }

    pub(super) fn context_at(&self, instance_path: &str) -> SchemaContext {
        let Some(root_uri) = self.root_uri.as_ref() else {
            return SchemaContext::default();
        };
        let Some(segments) = decode_pointer(instance_path) else {
            return SchemaContext::default();
        };
        let mut cursors = vec![Cursor {
            uri: root_uri.clone(),
            pointer: String::new(),
            alternative: false,
        }];

        let mut traversed = String::new();
        for segment in segments {
            cursors = self.expand(cursors, &traversed);
            traversed = append_pointer(&traversed, &segment);
            cursors = self.children_for_segment(&cursors, &segment);
            if cursors.is_empty() {
                return SchemaContext::default();
            }
        }
        cursors = self.expand(cursors, instance_path);
        self.context_from_cursors(&cursors)
    }

    pub(super) fn definition_for_ref(
        &self,
        schema_uri: &str,
        reference: &str,
    ) -> Option<SchemaLocation> {
        let base = Url::parse(schema_uri).ok()?;
        let target = base.join(reference).ok()?;
        self.location_for_url(&target)
    }

    fn index_resource(&mut self, uri: &str) {
        let Some(resource) = self.resources.get(uri) else {
            return;
        };
        let mut pending = vec![(String::new(), uri.to_owned(), String::new())];
        while let Some((pointer, inherited_scope, inherited_scope_root)) = pending.pop() {
            let Some(node) = self
                .resources
                .get(uri)
                .and_then(|resource| resource.schema.pointer(&pointer))
            else {
                continue;
            };
            let declared_id = node.get("$id").and_then(JsonValue::as_str);
            let scope = declared_id
                .and_then(|id| resolve_url(&inherited_scope, id))
                .unwrap_or(inherited_scope);
            let scope_root = if declared_id.is_some() {
                pointer.clone()
            } else {
                inherited_scope_root
            };
            self.scopes
                .insert((uri.to_owned(), pointer.clone()), scope.clone());

            let location = SchemaLocation {
                uri: uri.to_owned(),
                pointer: pointer.clone(),
                span: resource
                    .located
                    .as_ref()
                    .and_then(|located| located.span_at_pointer(&pointer)),
            };
            if let Some(pointer_uri) = uri_with_fragment(uri, &pointer) {
                self.definitions.insert(pointer_uri, location.clone());
            }
            if let Some(relative_pointer) = pointer_from_scope_root(&pointer, &scope_root)
                && let Some(pointer_uri) = uri_with_fragment(&scope, relative_pointer)
            {
                self.definitions.insert(pointer_uri, location.clone());
            }
            if declared_id.is_some() {
                self.definitions.insert(scope.clone(), location.clone());
            }
            for keyword in ["$anchor", "$dynamicAnchor"] {
                if let Some(anchor) = node.get(keyword).and_then(JsonValue::as_str)
                    && let Some(anchor_uri) = uri_with_fragment(&scope, anchor)
                {
                    self.definitions.insert(anchor_uri, location.clone());
                }
            }

            match node {
                JsonValue::Array(items) => {
                    for index in (0..items.len()).rev() {
                        pending.push((
                            append_pointer(&pointer, &index.to_string()),
                            scope.clone(),
                            scope_root.clone(),
                        ));
                    }
                }
                JsonValue::Object(object) => {
                    for key in object.keys().rev() {
                        pending.push((
                            append_pointer(&pointer, key),
                            scope.clone(),
                            scope_root.clone(),
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    fn expand(&self, cursors: Vec<Cursor>, instance_path: &str) -> Vec<Cursor> {
        let mut pending = VecDeque::from(cursors);
        let mut expanded = Vec::new();
        let mut visited = HashSet::new();
        while let Some(cursor) = pending.pop_front() {
            let key = (
                cursor.uri.clone(),
                cursor.pointer.clone(),
                instance_path.to_owned(),
            );
            if !visited.insert(key) {
                continue;
            }
            let Some(node) = self.node(&cursor) else {
                continue;
            };
            expanded.push(cursor.clone());

            for keyword in ["$ref", "$dynamicRef"] {
                if let Some(reference) = node.get(keyword).and_then(JsonValue::as_str)
                    && let Some(mut target) = self.resolve_cursor(&cursor, reference)
                {
                    target.alternative = cursor.alternative;
                    pending.push_back(target);
                }
            }
            for keyword in ["allOf", "anyOf", "oneOf"] {
                if let Some(items) = node.get(keyword).and_then(JsonValue::as_array) {
                    for index in 0..items.len() {
                        pending.push_back(Cursor {
                            uri: cursor.uri.clone(),
                            pointer: append_pointer(
                                &append_pointer(&cursor.pointer, keyword),
                                &index.to_string(),
                            ),
                            alternative: cursor.alternative || keyword != "allOf",
                        });
                    }
                }
            }
            for keyword in ["if", "then", "else"] {
                if node.get(keyword).is_some_and(is_schema) {
                    pending.push_back(Cursor {
                        uri: cursor.uri.clone(),
                        pointer: append_pointer(&cursor.pointer, keyword),
                        alternative: true,
                    });
                }
            }
        }
        expanded
    }

    fn children_for_segment(&self, cursors: &[Cursor], segment: &str) -> Vec<Cursor> {
        let mut children = Vec::new();
        let mut seen = HashSet::new();
        for cursor in cursors {
            let Some(node) = self.node(cursor) else {
                continue;
            };
            if let Some(index) = parse_index(segment) {
                if let Some(prefix_items) = node.get("prefixItems").and_then(JsonValue::as_array)
                    && index < prefix_items.len()
                {
                    push_cursor(
                        &mut children,
                        &mut seen,
                        Cursor {
                            uri: cursor.uri.clone(),
                            pointer: append_pointer(
                                &append_pointer(&cursor.pointer, "prefixItems"),
                                &index.to_string(),
                            ),
                            alternative: cursor.alternative,
                        },
                    );
                } else if node.get("items").is_some_and(is_schema) {
                    push_cursor(
                        &mut children,
                        &mut seen,
                        Cursor {
                            uri: cursor.uri.clone(),
                            pointer: append_pointer(&cursor.pointer, "items"),
                            alternative: cursor.alternative,
                        },
                    );
                }
            }

            let mut matched = false;
            if let Some(properties) = node.get("properties").and_then(JsonValue::as_object)
                && properties.contains_key(segment)
            {
                matched = true;
                push_cursor(
                    &mut children,
                    &mut seen,
                    Cursor {
                        uri: cursor.uri.clone(),
                        pointer: append_pointer(
                            &append_pointer(&cursor.pointer, "properties"),
                            segment,
                        ),
                        alternative: cursor.alternative,
                    },
                );
            }
            if let Some(patterns) = node.get("patternProperties").and_then(JsonValue::as_object) {
                for pattern in patterns.keys() {
                    if Regex::new(pattern).is_ok_and(|regex| regex.is_match(segment)) {
                        matched = true;
                        push_cursor(
                            &mut children,
                            &mut seen,
                            Cursor {
                                uri: cursor.uri.clone(),
                                pointer: append_pointer(
                                    &append_pointer(&cursor.pointer, "patternProperties"),
                                    pattern,
                                ),
                                alternative: cursor.alternative,
                            },
                        );
                    }
                }
            }
            if !matched && node.get("additionalProperties").is_some_and(is_schema) {
                push_cursor(
                    &mut children,
                    &mut seen,
                    Cursor {
                        uri: cursor.uri.clone(),
                        pointer: append_pointer(&cursor.pointer, "additionalProperties"),
                        alternative: cursor.alternative,
                    },
                );
            }
        }
        children
    }

    fn context_from_cursors(&self, cursors: &[Cursor]) -> SchemaContext {
        struct Candidate {
            value: PropertyCandidate,
            order: usize,
        }

        let mut context = SchemaContext::default();
        let mut candidates = Vec::<Candidate>::new();
        let mut candidate_indices = HashMap::<String, usize>::new();
        let mut seen_types = HashSet::new();
        let mut seen_enums = HashSet::new();
        let mut seen_locations = HashSet::new();

        for cursor in cursors {
            let Some(node) = self.node(cursor) else {
                continue;
            };
            if context.title.is_none() {
                context.title = annotation(node, "title");
            }
            if context.description.is_none() {
                context.description = annotation(node, "description");
            }
            if context.default.is_none() {
                context.default = node.get("default").map(compact);
            }
            context.deprecated |= node
                .get("deprecated")
                .and_then(JsonValue::as_bool)
                .unwrap_or(false);
            collect_types(node, &mut context.types, &mut seen_types);
            if let Some(values) = node.get("enum").and_then(JsonValue::as_array) {
                for value in values {
                    let rendered = compact(value);
                    if seen_enums.insert(rendered.clone()) {
                        context.enum_values.push(rendered);
                    }
                }
            }
            let location = render_location(&cursor.uri, &cursor.pointer);
            if seen_locations.insert(location.clone()) {
                context.schema_locations.push(location);
            }

            let required = node
                .get("required")
                .and_then(JsonValue::as_array)
                .into_iter()
                .flatten()
                .filter_map(JsonValue::as_str)
                .collect::<HashSet<_>>();
            if let Some(properties) = node.get("properties").and_then(JsonValue::as_object) {
                for (name, property) in properties {
                    let is_required = !cursor.alternative && required.contains(name.as_str());
                    if let Some(index) = candidate_indices.get(name).copied() {
                        let candidate = &mut candidates[index].value;
                        candidate.required |= is_required;
                        if candidate.description.is_none() {
                            candidate.description = annotation(property, "description");
                        }
                        if candidate.default.is_none() {
                            candidate.default = property.get("default").map(compact);
                        }
                        continue;
                    }
                    let index = candidates.len();
                    candidate_indices.insert(name.clone(), index);
                    candidates.push(Candidate {
                        order: index,
                        value: PropertyCandidate {
                            name: name.clone(),
                            description: annotation(property, "description"),
                            required: is_required,
                            default: property.get("default").map(compact),
                        },
                    });
                }
            }
        }

        candidates.sort_by(|left, right| {
            right
                .value
                .required
                .cmp(&left.value.required)
                .then_with(|| left.order.cmp(&right.order))
                .then_with(|| left.value.name.cmp(&right.value.name))
        });
        context.properties = candidates
            .into_iter()
            .map(|candidate| candidate.value)
            .collect();
        context
    }

    fn node(&self, cursor: &Cursor) -> Option<&JsonValue> {
        self.resources
            .get(&cursor.uri)?
            .schema
            .pointer(&cursor.pointer)
    }

    fn resolve_cursor(&self, cursor: &Cursor, reference: &str) -> Option<Cursor> {
        let scope = self
            .scopes
            .get(&(cursor.uri.clone(), cursor.pointer.clone()))
            .map_or(cursor.uri.as_str(), String::as_str);
        let target = Url::parse(scope).ok()?.join(reference).ok()?;
        let location = self.location_for_url(&target)?;
        Some(Cursor {
            uri: location.uri,
            pointer: location.pointer,
            alternative: cursor.alternative,
        })
    }

    fn location_for_url(&self, target: &Url) -> Option<SchemaLocation> {
        if let Some(location) = self.definitions.get(target.as_str()) {
            return Some(location.clone());
        }
        let mut resource = target.clone();
        let fragment = resource.fragment().map(str::to_owned);
        resource.set_fragment(None);
        let canonical = self
            .aliases
            .get(resource.as_str())
            .map_or(resource.as_str(), String::as_str);
        let canonical_target = uri_with_fragment(canonical, fragment.as_deref().unwrap_or(""))?;
        self.definitions.get(&canonical_target).cloned()
    }
}

fn push_cursor(output: &mut Vec<Cursor>, seen: &mut HashSet<Cursor>, cursor: Cursor) {
    if seen.insert(cursor.clone()) {
        output.push(cursor);
    }
}

fn collect_types(node: &JsonValue, output: &mut Vec<String>, seen: &mut HashSet<String>) {
    let values: Box<dyn Iterator<Item = &JsonValue> + '_> = match node.get("type") {
        Some(JsonValue::Array(values)) => Box::new(values.iter()),
        Some(value) => Box::new(std::iter::once(value)),
        None => return,
    };
    for value in values.filter_map(JsonValue::as_str) {
        if seen.insert(value.to_owned()) {
            output.push(value.to_owned());
        }
    }
}

fn annotation(node: &JsonValue, keyword: &str) -> Option<String> {
    node.get(keyword)
        .and_then(JsonValue::as_str)
        .map(str::to_owned)
}

fn compact(value: &JsonValue) -> String {
    serde_json::to_string(value).expect("JSON values always serialize")
}

const fn is_schema(value: &JsonValue) -> bool {
    matches!(value, JsonValue::Bool(_) | JsonValue::Object(_))
}

fn parse_index(segment: &str) -> Option<usize> {
    if segment.len() > 1 && segment.starts_with('0') {
        return None;
    }
    segment.parse().ok()
}

fn decode_pointer(pointer: &str) -> Option<Vec<String>> {
    if pointer.is_empty() {
        return Some(Vec::new());
    }
    pointer
        .strip_prefix('/')?
        .split('/')
        .map(|segment| {
            let mut decoded = String::new();
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
        })
        .collect()
}

fn append_pointer(pointer: &str, segment: &str) -> String {
    let escaped = segment.replace('~', "~0").replace('/', "~1");
    format!("{pointer}/{escaped}")
}

fn pointer_from_scope_root<'pointer>(pointer: &'pointer str, root: &str) -> Option<&'pointer str> {
    if root.is_empty() {
        return Some(pointer);
    }
    if pointer == root {
        return Some("");
    }
    pointer
        .strip_prefix(root)
        .filter(|suffix| suffix.starts_with('/'))
}

fn fragmentless_uri(uri: &str) -> Option<String> {
    let mut url = Url::parse(uri).ok()?;
    url.set_fragment(None);
    Some(url.into())
}

fn resolve_url(base: &str, reference: &str) -> Option<String> {
    Url::parse(base).ok()?.join(reference).ok().map(Into::into)
}

fn uri_with_fragment(uri: &str, fragment: &str) -> Option<String> {
    let mut url = Url::parse(uri).ok()?;
    url.set_fragment(Some(fragment));
    Some(url.into())
}

fn render_location(uri: &str, pointer: &str) -> String {
    if pointer.is_empty() {
        uri.to_owned()
    } else {
        format!("{uri}#{pointer}")
    }
}

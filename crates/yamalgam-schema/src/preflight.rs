use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};

use yamalgam_core::Value;

use crate::value::YamlNode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceStep {
    Sequence(usize),
    MappingKey(usize),
    MappingValue(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepresentationErrorKind {
    ComplexMappingKey,
    NonFiniteNumber,
    ProjectedKeyCollision { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentationError {
    pub kind: RepresentationErrorKind,
    pub instance_path: String,
    pub route: Vec<InstanceStep>,
    pub conflicting_route: Option<Vec<InstanceStep>>,
}

#[derive(Debug)]
pub struct PreparedInstance<'value> {
    pub value: &'value Value,
    pub projected_keys: ProjectedKeys,
}

#[derive(Debug, Default)]
pub struct ProjectedKeys {
    mappings: HashMap<usize, Vec<ProjectedKey>>,
}

#[derive(Debug)]
pub struct ProjectedKey {
    mapping_index: usize,
    pub name: ProjectedName,
}

#[derive(Debug)]
pub enum ProjectedName {
    Original,
    Canonical(String),
}

impl ProjectedKey {
    pub fn as_str<'value>(&'value self, key: &'value Value) -> &'value str {
        match &self.name {
            ProjectedName::Original => key
                .as_str()
                .expect("original projected names must be string keys"),
            ProjectedName::Canonical(name) => name,
        }
    }
}

impl ProjectedKeys {
    pub fn for_mapping(&self, value: &Value) -> Option<&[ProjectedKey]> {
        self.mappings
            .get(&(std::ptr::from_ref(value) as usize))
            .map(Vec::as_slice)
    }
}

impl<'value> PreparedInstance<'value> {
    pub fn new(value: &'value Value) -> Result<Self, Vec<RepresentationError>> {
        let mut projected_keys = ProjectedKeys::default();
        let mut errors = Vec::new();
        let mut pending = vec![(value, String::new(), Vec::new())];

        while let Some((node, path, route)) = pending.pop() {
            match node {
                Value::Float(number) if !number.is_finite() => {
                    errors.push(RepresentationError {
                        kind: RepresentationErrorKind::NonFiniteNumber,
                        instance_path: path,
                        route,
                        conflicting_route: None,
                    });
                }
                Value::Sequence(items) => {
                    for (index, item) in items.iter().enumerate().rev() {
                        let mut child_route = route.clone();
                        child_route.push(InstanceStep::Sequence(index));
                        pending.push((item, append_path(&path, &index.to_string()), child_route));
                    }
                }
                Value::Mapping(mapping) => {
                    let mut projection: Vec<ProjectedKey> = Vec::with_capacity(mapping.len());
                    let mut projections_by_hash = HashMap::<u64, Vec<usize>>::new();
                    let mut child_nodes = Vec::with_capacity(mapping.len());

                    for (index, (key, child)) in mapping.iter().enumerate() {
                        let mut key_route = route.clone();
                        key_route.push(InstanceStep::MappingKey(index));
                        let projected_name = match key {
                            Value::Float(number) if !number.is_finite() => {
                                errors.push(RepresentationError {
                                    kind: RepresentationErrorKind::NonFiniteNumber,
                                    instance_path: path.clone(),
                                    route: key_route,
                                    conflicting_route: None,
                                });
                                None
                            }
                            Value::Sequence(_) | Value::Mapping(_) => {
                                errors.push(RepresentationError {
                                    kind: RepresentationErrorKind::ComplexMappingKey,
                                    instance_path: path.clone(),
                                    route: key_route,
                                    conflicting_route: None,
                                });
                                None
                            }
                            Value::String(_) => Some(ProjectedName::Original),
                            _ => key
                                .canonical_scalar_text()
                                .map(|name| ProjectedName::Canonical(name.into_owned())),
                        };

                        let mut child_route = route.clone();
                        child_route.push(InstanceStep::MappingValue(index));
                        let Some(projected_name) = projected_name else {
                            child_nodes.push((child, path.clone(), child_route));
                            continue;
                        };
                        let name = match &projected_name {
                            ProjectedName::Original => key
                                .as_str()
                                .expect("original projected names must be string keys"),
                            ProjectedName::Canonical(name) => name.as_str(),
                        };
                        child_nodes.push((child, append_path(&path, name), child_route));

                        let name_hash = hash_name(name);
                        let first_index = projections_by_hash.get(&name_hash).and_then(|indices| {
                            indices.iter().find_map(|projection_index| {
                                let prior = &projection[*projection_index];
                                let prior_key = mapping
                                    .iter()
                                    .nth(prior.mapping_index)
                                    .expect("projected key must retain its mapping index")
                                    .0;
                                (prior.as_str(prior_key) == name).then_some(prior.mapping_index)
                            })
                        });
                        if let Some(first_index) = first_index {
                            let mut conflicting_route = route.clone();
                            conflicting_route.push(InstanceStep::MappingKey(first_index));
                            let mut collision_route = route.clone();
                            collision_route.push(InstanceStep::MappingKey(index));
                            errors.push(RepresentationError {
                                kind: RepresentationErrorKind::ProjectedKeyCollision {
                                    name: name.to_owned(),
                                },
                                instance_path: append_path(&path, name),
                                route: collision_route,
                                conflicting_route: Some(conflicting_route),
                            });
                        }
                        projection.push(ProjectedKey {
                            mapping_index: index,
                            name: projected_name,
                        });
                        projections_by_hash
                            .entry(name_hash)
                            .or_default()
                            .push(projection.len() - 1);
                    }

                    pending.extend(child_nodes.into_iter().rev());

                    projected_keys
                        .mappings
                        .insert(std::ptr::from_ref(node) as usize, projection);
                }
                Value::Null
                | Value::Bool(_)
                | Value::Integer(_)
                | Value::Float(_)
                | Value::String(_) => {}
            }
        }

        if errors.is_empty() {
            Ok(Self {
                value,
                projected_keys,
            })
        } else {
            Err(errors)
        }
    }

    pub const fn root(&'value self) -> YamlNode<'value> {
        YamlNode {
            value: self.value,
            projected_keys: &self.projected_keys,
        }
    }
}

fn append_path(path: &str, segment: &str) -> String {
    let escaped = segment.replace('~', "~0").replace('/', "~1");
    format!("{path}/{escaped}")
}

fn hash_name(name: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod yaml_schema_compat {
    use std::borrow::Cow;

    use jsonschema::json::{JsonNumber, Node, Object};
    use yamalgam_core::{Mapping, Value};

    use super::{InstanceStep, PreparedInstance, ProjectedName, RepresentationErrorKind};

    fn mapping(entries: impl IntoIterator<Item = (Value, Value)>) -> Value {
        let mut mapping = Mapping::new();
        for (key, value) in entries {
            mapping.insert(key, value);
        }
        Value::Mapping(mapping)
    }

    #[test]
    fn scalar_mapping_keys_project_to_canonical_property_names() {
        let value = mapping([
            (Value::Null, Value::Integer(0)),
            (Value::Bool(true), Value::Integer(1)),
            (Value::Integer(42), Value::Integer(2)),
            (Value::Float(3.5), Value::Integer(3)),
            (Value::from("plain"), Value::Integer(4)),
        ]);

        let prepared = PreparedInstance::new(&value).expect("scalar keys are representable");
        let object = prepared.root().as_object().expect("mapping is an object");
        let members = object.members().collect::<Vec<_>>();
        let names = members
            .iter()
            .map(|(name, _)| name.as_ref())
            .collect::<Vec<_>>();

        assert_eq!(names, ["null", "true", "42", "3.5", "plain"]);
        assert_eq!(
            object
                .get(&"42".to_owned())
                .expect("projected integer key")
                .as_number()
                .expect("integer value")
                .as_i64(),
            Some(2)
        );

        let projection = prepared
            .projected_keys
            .for_mapping(object.0.value)
            .expect("root projection");
        assert!(matches!(projection[0].name, ProjectedName::Canonical(_)));
        assert!(matches!(projection[4].name, ProjectedName::Original));
        assert!(matches!(members[4].0, Cow::Borrowed("plain")));
    }

    #[test]
    fn complex_mapping_keys_are_rejected_with_their_route() {
        let value = mapping([(Value::Sequence(vec![Value::from("complex")]), Value::Null)]);

        let errors = PreparedInstance::new(&value).expect_err("complex key must fail");

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, RepresentationErrorKind::ComplexMappingKey);
        assert_eq!(errors[0].instance_path, "");
        assert_eq!(errors[0].route, [InstanceStep::MappingKey(0)]);
        assert_eq!(errors[0].conflicting_route, None);
    }

    #[test]
    fn non_finite_mapping_keys_are_rejected() {
        let value = mapping([
            (Value::Float(f64::NAN), Value::Null),
            (Value::Float(f64::INFINITY), Value::Null),
        ]);

        let errors = PreparedInstance::new(&value).expect_err("non-finite keys must fail");

        assert_eq!(errors.len(), 2);
        assert!(errors.iter().all(|error| {
            error.kind == RepresentationErrorKind::NonFiniteNumber && error.instance_path.is_empty()
        }));
    }

    #[test]
    fn projected_name_collisions_never_overwrite_members() {
        let value = mapping([
            (Value::Integer(1), Value::from("integer")),
            (Value::Float(1.0), Value::from("float")),
            (Value::from("1"), Value::from("string")),
        ]);

        let errors = PreparedInstance::new(&value).expect_err("collisions must fail");

        assert_eq!(errors.len(), 2);
        for (offset, error) in errors.iter().enumerate() {
            assert_eq!(
                error.kind,
                RepresentationErrorKind::ProjectedKeyCollision {
                    name: "1".to_owned(),
                }
            );
            assert_eq!(error.instance_path, "/1");
            assert_eq!(error.route, [InstanceStep::MappingKey(offset + 1)]);
            assert_eq!(
                error.conflicting_route,
                Some(vec![InstanceStep::MappingKey(0)])
            );
        }
    }
}

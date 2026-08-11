use std::borrow::Cow;

use jsonschema::json::{Array, Json, JsonNumber, Node, NodeIdentity, Object, cmp};
use jsonschema::types::JsonType;
use serde_json::{Map as JsonMap, Number as JsonNumberValue, Value as JsonValue};
use yamalgam_core::{Mapping, Value};

use crate::preflight::{ProjectedKey, ProjectedKeys};

pub struct YamlJson;

#[derive(Debug, Clone, Copy)]
pub struct YamlNode<'value> {
    pub value: &'value Value,
    pub projected_keys: &'value ProjectedKeys,
}

#[derive(Debug, Clone, Copy)]
pub struct YamlObject<'value>(pub YamlNode<'value>);

#[derive(Debug, Clone, Copy)]
pub struct YamlArray<'value>(YamlNode<'value>);

#[derive(Debug, Clone, Copy)]
pub enum YamlNumber {
    Integer(i64),
    Float(f64),
}

impl Json for YamlJson {
    type Node<'value> = YamlNode<'value>;
    type PreparedKey = String;
    type StringBuffer = Value;

    fn prepare_key(key: &str) -> Self::PreparedKey {
        key.to_owned()
    }

    fn with_string_node<T>(
        buffer: &mut Self::StringBuffer,
        string: &str,
        f: impl FnOnce(Self::Node<'_>) -> T,
    ) -> T {
        if let Value::String(existing) = buffer {
            existing.clear();
            existing.push_str(string);
        } else {
            *buffer = Value::String(string.to_owned());
        }
        let projected_keys = ProjectedKeys::default();
        f(YamlNode {
            value: buffer,
            projected_keys: &projected_keys,
        })
    }
}

impl JsonNumber for YamlNumber {
    fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Integer(value) => u64::try_from(*value).ok(),
            Self::Float(_) => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            Self::Float(_) => None,
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Integer(value) => Some(*value as f64),
            Self::Float(value) => Some(*value),
        }
    }

    fn as_str(&self) -> Cow<'_, str> {
        match self {
            Self::Integer(value) => Cow::Owned(value.to_string()),
            Self::Float(value) => Cow::Owned(value.to_string()),
        }
    }

    fn to_number(&self) -> Cow<'_, JsonNumberValue> {
        let number = match self {
            Self::Integer(value) => JsonNumberValue::from(*value),
            Self::Float(value) => {
                JsonNumberValue::from_f64(*value).expect("prepared YAML numbers must be finite")
            }
        };
        Cow::Owned(number)
    }

    fn is_integer(&self) -> bool {
        match self {
            Self::Integer(_) => true,
            Self::Float(value) => value.fract() == 0.0,
        }
    }
}

impl<'value> Node<'value, YamlJson> for YamlNode<'value> {
    type Object = YamlObject<'value>;
    type Array = YamlArray<'value>;
    type Number = YamlNumber;

    fn as_object(&self) -> Option<Self::Object> {
        matches!(self.value, Value::Mapping(_)).then_some(YamlObject(*self))
    }

    fn as_array(&self) -> Option<Self::Array> {
        matches!(self.value, Value::Sequence(_)).then_some(YamlArray(*self))
    }

    fn as_string(&self) -> Option<Cow<'value, str>> {
        self.value.as_str().map(Cow::Borrowed)
    }

    fn as_number(&self) -> Option<Self::Number> {
        match self.value {
            Value::Integer(value) => Some(YamlNumber::Integer(*value)),
            Value::Float(value) => Some(YamlNumber::Float(*value)),
            _ => None,
        }
    }

    fn as_boolean(&self) -> Option<bool> {
        self.value.as_bool()
    }

    fn is_null(&self) -> bool {
        self.value.is_null()
    }

    fn json_type(&self) -> JsonType {
        match self.value {
            Value::Null => JsonType::Null,
            Value::Bool(_) => JsonType::Boolean,
            Value::Integer(_) | Value::Float(_) => JsonType::Number,
            Value::String(_) => JsonType::String,
            Value::Sequence(_) => JsonType::Array,
            Value::Mapping(_) => JsonType::Object,
        }
    }

    fn equals_value(&self, expected: &JsonValue) -> bool {
        equals_json(*self, expected)
    }

    fn to_value(&self) -> Cow<'value, JsonValue> {
        #[cfg(test)]
        TO_VALUE_CALLS.with(|calls| calls.set(calls.get() + 1));
        Cow::Owned(materialize(*self))
    }

    fn identity(&self) -> Option<NodeIdentity> {
        Some(NodeIdentity::new(std::ptr::from_ref(self.value) as usize))
    }
}

impl<'value> Object<'value, YamlJson> for YamlObject<'value> {
    type Node = YamlNode<'value>;
    type MemberName = Cow<'value, str>;
    type MembersIter = YamlMembersIter<'value>;

    fn len(&self) -> usize {
        self.mapping().len()
    }

    fn get(&self, key: &String) -> Option<Self::Node> {
        let mapping = self.mapping();
        if let Some(value) = mapping.get_str(key) {
            return Some(self.node(value));
        }

        let projection = self.projection();
        mapping
            .iter()
            .zip(projection)
            .find(|((key_value, _), projected)| projected.as_str(key_value) == key)
            .map(|((_, value), _)| self.node(value))
    }

    fn members(&self) -> Self::MembersIter {
        YamlMembersIter {
            entries: self.mapping().into_iter().enumerate(),
            projection: self.projection(),
            projected_keys: self.0.projected_keys,
        }
    }
}

impl<'value> YamlObject<'value> {
    fn mapping(self) -> &'value Mapping {
        let Value::Mapping(mapping) = self.0.value else {
            unreachable!("YamlObject must wrap a mapping")
        };
        mapping
    }

    fn projection(self) -> &'value [ProjectedKey] {
        self.0
            .projected_keys
            .for_mapping(self.0.value)
            .expect("prepared mappings must have projected keys")
    }

    const fn node(self, value: &'value Value) -> YamlNode<'value> {
        YamlNode {
            value,
            projected_keys: self.0.projected_keys,
        }
    }
}

pub struct YamlMembersIter<'value> {
    entries: std::iter::Enumerate<<&'value Mapping as IntoIterator>::IntoIter>,
    projection: &'value [ProjectedKey],
    projected_keys: &'value ProjectedKeys,
}

impl<'value> Iterator for YamlMembersIter<'value> {
    type Item = (Cow<'value, str>, YamlNode<'value>);

    fn next(&mut self) -> Option<Self::Item> {
        let (index, (key, value)) = self.entries.next()?;
        let name = self.projection[index].as_str(key);
        Some((
            Cow::Borrowed(name),
            YamlNode {
                value,
                projected_keys: self.projected_keys,
            },
        ))
    }
}

impl<'value> Array<'value, YamlJson> for YamlArray<'value> {
    type Node = YamlNode<'value>;
    type ElementsIter = YamlElementsIter<'value>;

    fn len(&self) -> usize {
        self.items().len()
    }

    fn elements(&self) -> Self::ElementsIter {
        YamlElementsIter {
            items: self.items().iter(),
            projected_keys: self.0.projected_keys,
        }
    }
}

impl<'value> YamlArray<'value> {
    fn items(self) -> &'value [Value] {
        let Value::Sequence(items) = self.0.value else {
            unreachable!("YamlArray must wrap a sequence")
        };
        items
    }
}

pub struct YamlElementsIter<'value> {
    items: std::slice::Iter<'value, Value>,
    projected_keys: &'value ProjectedKeys,
}

impl<'value> Iterator for YamlElementsIter<'value> {
    type Item = YamlNode<'value>;

    fn next(&mut self) -> Option<Self::Item> {
        self.items.next().map(|value| YamlNode {
            value,
            projected_keys: self.projected_keys,
        })
    }
}

fn equals_json(node: YamlNode<'_>, expected: &JsonValue) -> bool {
    let mut pending = vec![(node, expected)];
    while let Some((node, expected)) = pending.pop() {
        match (node.value, expected) {
            (Value::Null, JsonValue::Null) => {}
            (Value::Bool(left), JsonValue::Bool(right)) if left == right => {}
            (Value::Integer(value), JsonValue::Number(number))
                if cmp::equal_numbers(&YamlNumber::Integer(*value), number) => {}
            (Value::Float(value), JsonValue::Number(number))
                if cmp::equal_numbers(&YamlNumber::Float(*value), number) => {}
            (Value::String(left), JsonValue::String(right)) if left == right => {}
            (Value::Sequence(left), JsonValue::Array(right)) if left.len() == right.len() => {
                pending.extend(left.iter().zip(right).map(|(child, expected)| {
                    (
                        YamlNode {
                            value: child,
                            projected_keys: node.projected_keys,
                        },
                        expected,
                    )
                }));
            }
            (Value::Mapping(_), JsonValue::Object(right)) => {
                let object = YamlObject(node);
                if object.len() != right.len() {
                    return false;
                }
                for (name, expected) in right {
                    let Some(child) = object.get(name) else {
                        return false;
                    };
                    pending.push((child, expected));
                }
            }
            _ => return false,
        }
    }
    true
}

fn materialize(node: YamlNode<'_>) -> JsonValue {
    enum ArenaNode {
        Scalar(JsonValue),
        Sequence(Vec<usize>),
        Mapping(Vec<(String, usize)>),
    }

    let mut arena = vec![None];
    let mut pending = vec![(node, 0_usize)];
    while let Some((node, index)) = pending.pop() {
        let materialized = match node.value {
            Value::Null => ArenaNode::Scalar(JsonValue::Null),
            Value::Bool(value) => ArenaNode::Scalar(JsonValue::Bool(*value)),
            Value::Integer(value) => {
                ArenaNode::Scalar(JsonValue::Number(JsonNumberValue::from(*value)))
            }
            Value::Float(value) => ArenaNode::Scalar(JsonValue::Number(
                JsonNumberValue::from_f64(*value).expect("prepared YAML numbers must be finite"),
            )),
            Value::String(value) => ArenaNode::Scalar(JsonValue::String(value.clone())),
            Value::Sequence(items) => {
                let first = arena.len();
                let children = (first..first + items.len()).collect::<Vec<_>>();
                arena.resize_with(first + items.len(), || None);
                for (value, child) in items.iter().zip(children.iter().copied()).rev() {
                    pending.push((
                        YamlNode {
                            value,
                            projected_keys: node.projected_keys,
                        },
                        child,
                    ));
                }
                ArenaNode::Sequence(children)
            }
            Value::Mapping(_) => {
                let members = YamlObject(node).members().collect::<Vec<_>>();
                let first = arena.len();
                let children = (first..first + members.len()).collect::<Vec<_>>();
                arena.resize_with(first + members.len(), || None);
                let mut entries = Vec::with_capacity(members.len());
                for ((name, value), child) in members.into_iter().zip(children).rev() {
                    entries.push((name.into_owned(), child));
                    pending.push((value, child));
                }
                entries.reverse();
                ArenaNode::Mapping(entries)
            }
        };
        arena[index] = Some(materialized);
    }

    let mut values = vec![None; arena.len()];
    for index in (0..arena.len()).rev() {
        let value = match arena[index]
            .take()
            .expect("materialization arena node must be built")
        {
            ArenaNode::Scalar(value) => value,
            ArenaNode::Sequence(children) => JsonValue::Array(
                children
                    .into_iter()
                    .map(|child| {
                        values[child]
                            .take()
                            .expect("sequence child must be materialized")
                    })
                    .collect(),
            ),
            ArenaNode::Mapping(entries) => JsonValue::Object(
                entries
                    .into_iter()
                    .map(|(name, child)| {
                        (
                            name,
                            values[child]
                                .take()
                                .expect("mapping child must be materialized"),
                        )
                    })
                    .collect::<JsonMap<_, _>>(),
            ),
        };
        values[index] = Some(value);
    }
    values[0]
        .take()
        .expect("materialization arena root must be built")
}

#[cfg(test)]
std::thread_local! {
    static TO_VALUE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn reset_to_value_calls() {
    TO_VALUE_CALLS.set(0);
}

#[cfg(test)]
fn to_value_calls() -> usize {
    TO_VALUE_CALLS.get()
}

#[cfg(test)]
mod value_adapter {
    use jsonschema::json::{Array, Json, JsonNumber, Node, Object};
    use serde_json::Value as JsonValue;
    use yamalgam_core::{Mapping, Value};

    use super::{YamlJson, reset_to_value_calls, to_value_calls};
    use crate::preflight::PreparedInstance;

    fn from_json(value: JsonValue) -> Value {
        match value {
            JsonValue::Null => Value::Null,
            JsonValue::Bool(value) => Value::Bool(value),
            JsonValue::Number(value) => value.as_i64().map_or_else(
                || Value::Float(value.as_f64().expect("finite conformance number")),
                Value::Integer,
            ),
            JsonValue::String(value) => Value::String(value),
            JsonValue::Array(values) => {
                Value::Sequence(values.into_iter().map(from_json).collect())
            }
            JsonValue::Object(values) => {
                let mut mapping = Mapping::new();
                for (key, value) in values {
                    mapping.insert(Value::String(key), from_json(value));
                }
                Value::Mapping(mapping)
            }
        }
    }

    #[test]
    fn custom_representation_satisfies_jsonschema_conformance() {
        let value = from_json(jsonschema::json::conformance::document());
        let prepared =
            PreparedInstance::new(&value, usize::MAX, usize::MAX).expect("conformance value");

        jsonschema::json::conformance::assert_conformance::<YamlJson>(&prepared.root());
    }

    #[test]
    fn mapping_lookup_and_iteration_reuse_native_nodes() {
        let value = from_json(serde_json::json!({"items": [1, 2], "name": "héllo"}));
        let prepared =
            PreparedInstance::new(&value, usize::MAX, usize::MAX).expect("representable value");
        let root = prepared.root();
        let object = root.as_object().expect("object");
        let items = object
            .get(&YamlJson::prepare_key("items"))
            .expect("items member");
        let iterated_items = object
            .members()
            .find(|(name, _)| name.as_ref() == "items")
            .expect("iterated items")
            .1;

        assert_eq!(items.identity(), iterated_items.identity());
        assert_ne!(root.identity(), items.identity());
        assert_eq!(
            items
                .as_array()
                .expect("array")
                .elements()
                .next()
                .expect("first item")
                .as_number()
                .expect("number")
                .as_i64(),
            Some(1)
        );
        assert_eq!(
            object
                .get(&YamlJson::prepare_key("name"))
                .expect("name member")
                .string_length(),
            Some(5)
        );
    }

    #[test]
    fn hot_accessors_do_not_materialize_json_values() {
        let value = from_json(serde_json::json!({"items": [1, 2], "name": "value"}));
        let prepared =
            PreparedInstance::new(&value, usize::MAX, usize::MAX).expect("representable value");
        let root = prepared.root();

        reset_to_value_calls();
        let object = root.as_object().expect("object");
        let _ = object.get(&YamlJson::prepare_key("items"));
        let _ = object.members().count();
        let _ = object
            .get(&YamlJson::prepare_key("items"))
            .expect("items")
            .as_array()
            .expect("array")
            .elements()
            .count();
        assert_eq!(to_value_calls(), 0);

        let materialized = root.to_value();
        assert_eq!(to_value_calls(), 1);
        assert_eq!(
            materialized.as_ref(),
            &serde_json::json!({"items": [1, 2], "name": "value"})
        );
    }

    #[test]
    fn deep_native_values_use_iterative_fallbacks() {
        const DEPTH: usize = 1_024;

        let mut value = Value::Integer(1);
        let mut expected = JsonValue::from(1);
        for _ in 0..DEPTH {
            value = Value::Sequence(vec![value]);
            expected = JsonValue::Array(vec![expected]);
        }
        let prepared =
            PreparedInstance::new(&value, DEPTH, usize::MAX).expect("deep value is representable");

        assert!(prepared.root().equals_value(&expected));

        let materialized = prepared.root().to_value().into_owned();
        let mut cursor = &materialized;
        for _ in 0..DEPTH {
            cursor = &cursor.as_array().expect("nested array")[0];
        }
        assert_eq!(cursor.as_i64(), Some(1));
    }
}

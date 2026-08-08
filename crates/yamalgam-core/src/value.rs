//! YAML Value DOM types.
//!
//! Provides a lossy tree representation of a YAML document. "Lossy" because
//! comments, formatting, and tag information are discarded — only semantic
//! content is preserved.
//!
//! The primary types are [`Value`] (a single YAML node) and [`Mapping`]
//! (an insertion-order-preserving key/value map backed by a `Vec`).

// y[impl model.repr.node-definition]
// y[impl overview.collections.block-indent]
// y[impl overview.collections.block-seq-indicator]
// y[impl overview.collections.comment-indicator]
// y[impl overview.scalars.double-quoted-escapes]
// y[impl overview.scalars.flow-multiline-fold]
// y[impl overview.scalars.flow-plain]
use std::collections::{HashMap, hash_map::RandomState};
use std::fmt;
use std::hash::{BuildHasher, Hash, Hasher};

/// A YAML value.
///
/// Represents any node in a YAML document tree. Scalars are resolved to their
/// native Rust types; collections hold child `Value`s recursively.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Value {
    /// A YAML null (`~`, `null`, or absent value).
    #[default]
    Null,
    /// A boolean (`true` / `false`).
    Bool(bool),
    /// A signed 64-bit integer.
    Integer(i64),
    /// A 64-bit float.
    Float(f64),
    /// A UTF-8 string.
    // y[impl model.repr.node.scalar]
    String(String),
    /// An ordered sequence of values.
    // y[impl model.repr.node.sequence]
    Sequence(Vec<Self>),
    /// An ordered mapping of key/value pairs.
    Mapping(Mapping),
}

/// An insertion-order-preserving map of YAML key/value pairs.
///
/// Backed by a `Vec<(Value, Value)>`. Insert replaces an existing key rather
/// than appending a duplicate. A private side index accelerates equality-safe
/// scalar keys while preserving ordered entries as the semantic state.
#[derive(Clone)]
// y[impl model.repr.node.mapping]
// y[impl model.repr.uniqueness]
// y[impl model.serial.key-order.serialization-detail]
// y[impl model.serial.key-order.no-representation-order]
pub struct Mapping {
    entries: Vec<(Value, Value)>,
    scalar_index: Option<ScalarIndex>,
}

#[derive(Clone)]
struct ScalarIndex {
    hash_builder: RandomState,
    positions: HashMap<u64, Vec<usize>>,
}

impl ScalarIndex {
    fn new() -> Self {
        Self {
            hash_builder: RandomState::new(),
            positions: HashMap::new(),
        }
    }

    const fn is_indexable(key: &Value) -> bool {
        matches!(
            key,
            Value::Null | Value::Bool(_) | Value::Integer(_) | Value::String(_)
        )
    }

    fn fingerprint_value(&self, key: &Value) -> Option<u64> {
        let mut hasher = self.hash_builder.build_hasher();
        match key {
            Value::Null => 0_u8.hash(&mut hasher),
            Value::Bool(value) => {
                1_u8.hash(&mut hasher);
                value.hash(&mut hasher);
            }
            Value::Integer(value) => {
                2_u8.hash(&mut hasher);
                value.hash(&mut hasher);
            }
            Value::String(value) => {
                3_u8.hash(&mut hasher);
                value.hash(&mut hasher);
            }
            Value::Float(_) | Value::Sequence(_) | Value::Mapping(_) => return None,
        }
        Some(hasher.finish())
    }

    fn fingerprint_str(&self, key: &str) -> u64 {
        let mut hasher = self.hash_builder.build_hasher();
        3_u8.hash(&mut hasher);
        key.hash(&mut hasher);
        hasher.finish()
    }
}

impl fmt::Debug for Mapping {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Mapping")
            .field("entries", &self.entries)
            .finish()
    }
}

impl PartialEq for Mapping {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl Mapping {
    /// Creates an empty mapping.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            scalar_index: None,
        }
    }

    /// Inserts a key/value pair. If the key already exists, its value is
    /// replaced and the old value is returned.
    pub fn insert(&mut self, key: Value, value: Value) -> Option<Value> {
        if ScalarIndex::is_indexable(&key) {
            if self.scalar_index.is_none() {
                self.scalar_index = Some(ScalarIndex::new());
            }

            let fingerprint = self
                .scalar_index
                .as_ref()
                .and_then(|index| index.fingerprint_value(&key))
                .expect("indexable scalar must have a fingerprint");
            if let Some(position) = self.indexed_position(fingerprint, &key) {
                return Some(std::mem::replace(&mut self.entries[position].1, value));
            }

            let position = self.entries.len();
            self.entries.push((key, value));
            self.scalar_index
                .as_mut()
                .expect("scalar index must be initialized")
                .positions
                .entry(fingerprint)
                .or_default()
                .push(position);
            return None;
        }

        if let Some(position) = self.linear_position(&key) {
            return Some(std::mem::replace(&mut self.entries[position].1, value));
        }

        self.entries.push((key, value));
        None
    }

    /// Looks up a value by key.
    #[must_use]
    pub fn get(&self, key: &Value) -> Option<&Value> {
        if ScalarIndex::is_indexable(key) {
            let index = self.scalar_index.as_ref()?;
            let fingerprint = index
                .fingerprint_value(key)
                .expect("indexable scalar must have a fingerprint");
            let position = self.indexed_position(fingerprint, key)?;
            return Some(&self.entries[position].1);
        }

        self.linear_position(key)
            .map(|position| &self.entries[position].1)
    }

    /// Looks up a value by a borrowed string key without allocating.
    #[must_use]
    pub fn get_str(&self, key: &str) -> Option<&Value> {
        let index = self.scalar_index.as_ref()?;
        let fingerprint = index.fingerprint_str(key);
        index
            .positions
            .get(&fingerprint)?
            .iter()
            .copied()
            .find_map(|position| match &self.entries[position] {
                (Value::String(candidate), value) if candidate == key => Some(value),
                _ => None,
            })
    }

    fn indexed_position(&self, fingerprint: u64, key: &Value) -> Option<usize> {
        self.scalar_index
            .as_ref()?
            .positions
            .get(&fingerprint)?
            .iter()
            .copied()
            .find(|&position| &self.entries[position].0 == key)
    }

    fn linear_position(&self, key: &Value) -> Option<usize> {
        self.entries
            .iter()
            .position(|(candidate, _)| candidate == key)
    }

    /// Returns an iterator over keys.
    pub fn keys(&self) -> impl Iterator<Item = &Value> {
        self.entries.iter().map(|(k, _)| k)
    }

    /// Returns an iterator over values.
    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.entries.iter().map(|(_, v)| v)
    }

    /// Returns an iterator over `(key, value)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&Value, &Value)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    /// Returns the number of entries.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the mapping contains no entries.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for Mapping {
    fn default() -> Self {
        Self::new()
    }
}

const fn entry_refs((key, value): &(Value, Value)) -> (&Value, &Value) {
    (key, value)
}

const fn entry_refs_mut((key, value): &mut (Value, Value)) -> (&Value, &mut Value) {
    (key, value)
}

impl IntoIterator for Mapping {
    type Item = (Value, Value);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl<'mapping> IntoIterator for &'mapping Mapping {
    type Item = (&'mapping Value, &'mapping Value);
    type IntoIter = std::iter::Map<
        std::slice::Iter<'mapping, (Value, Value)>,
        fn(&'mapping (Value, Value)) -> Self::Item,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter().map(entry_refs)
    }
}

impl<'mapping> IntoIterator for &'mapping mut Mapping {
    type Item = (&'mapping Value, &'mapping mut Value);
    type IntoIter = std::iter::Map<
        std::slice::IterMut<'mapping, (Value, Value)>,
        fn(&'mapping mut (Value, Value)) -> Self::Item,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter_mut().map(entry_refs_mut)
    }
}

// ---------------------------------------------------------------------------
// Value accessors
// ---------------------------------------------------------------------------

impl Value {
    /// Looks up a value inside a [`Mapping`] by string key.
    ///
    /// Returns `None` if `self` is not a mapping or if the key is absent.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Mapping(m) => m.get_str(key),
            _ => None,
        }
    }

    /// Indexes into a [`Sequence`](Value::Sequence) by position.
    ///
    /// Returns `None` if `self` is not a sequence or if the index is
    /// out of bounds.
    #[must_use]
    pub fn get_index(&self, index: usize) -> Option<&Self> {
        match self {
            Self::Sequence(seq) => seq.get(index),
            _ => None,
        }
    }

    /// Returns the contained string slice, if this is a [`Value::String`].
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the contained bool, if this is a [`Value::Bool`].
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the contained integer, if this is a [`Value::Integer`].
    #[must_use]
    pub const fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(n) => Some(*n),
            _ => None,
        }
    }

    /// Returns the contained float, if this is a [`Value::Float`].
    #[must_use]
    pub const fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Returns `true` if this value is [`Value::Null`].
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

// ---------------------------------------------------------------------------
// From impls
// ---------------------------------------------------------------------------

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::String(s.to_owned())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Self::Integer(n)
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Self::Integer(i64::from(n))
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Self::Float(f)
    }
}

// ---------------------------------------------------------------------------
// serde Deserialize
// ---------------------------------------------------------------------------

impl<'de> serde::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> serde::de::Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any YAML value")
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        serde::Deserialize::deserialize(deserializer)
    }

    fn visit_bool<E>(self, b: bool) -> Result<Value, E> {
        Ok(Value::Bool(b))
    }

    fn visit_i64<E>(self, i: i64) -> Result<Value, E> {
        Ok(Value::Integer(i))
    }

    fn visit_u64<E>(self, u: u64) -> Result<Value, E>
    where
        E: serde::de::Error,
    {
        // Value::Integer is i64; refuse to wrap or silently lose precision.
        i64::try_from(u)
            .map(Value::Integer)
            .map_err(|_| E::custom(format!("integer {u} out of range for i64")))
    }

    fn visit_f64<E>(self, f: f64) -> Result<Value, E> {
        Ok(Value::Float(f))
    }

    fn visit_str<E>(self, s: &str) -> Result<Value, E> {
        Ok(Value::String(s.to_owned()))
    }

    fn visit_string<E>(self, s: String) -> Result<Value, E> {
        Ok(Value::String(s))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut items = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(item) = seq.next_element()? {
            items.push(item);
        }
        Ok(Value::Sequence(items))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        // Mapping::insert replaces duplicates — same semantics as the
        // Composer, so both paths agree on duplicate-key documents.
        let mut mapping = Mapping::new();
        while let Some((key, value)) = map.next_entry()? {
            mapping.insert(key, value);
        }
        Ok(Value::Mapping(mapping))
    }
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Integer(n) => write!(f, "{n}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::String(s) => write!(f, "{s}"),
            Self::Sequence(_) => write!(f, "[sequence]"),
            Self::Mapping(_) => write!(f, "{{mapping}}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_default() {
        assert_eq!(Value::default(), Value::Null);
    }

    #[test]
    fn mapping_get_by_str() {
        let mut m = Mapping::new();
        m.insert(Value::from("key"), Value::from("value"));
        let v = Value::Mapping(m);
        assert_eq!(v.get("key"), Some(&Value::from("value")));
    }

    #[test]
    fn scalar_keys_use_a_complete_side_index() {
        let mut mapping = Mapping::new();
        for index in 0_i64..10_000 {
            mapping.insert(Value::Integer(index), Value::Integer(index));
        }

        let scalar_index = mapping.scalar_index.as_ref().expect("scalar index");
        assert_eq!(
            scalar_index.positions.values().map(Vec::len).sum::<usize>(),
            mapping.len()
        );
        assert_eq!(
            mapping.get(&Value::Integer(9_999)),
            Some(&Value::Integer(9_999))
        );
    }

    #[test]
    fn borrowed_string_lookup_uses_the_scalar_index() {
        let mut mapping = Mapping::new();
        mapping.insert(Value::from("alpha"), Value::Integer(1));
        mapping.insert(Value::from("beta"), Value::Integer(2));

        assert_eq!(mapping.get_str("beta"), Some(&Value::Integer(2)));
        assert_eq!(
            Value::Mapping(mapping).get("alpha"),
            Some(&Value::Integer(1))
        );
    }

    #[test]
    fn absent_scalar_lookup_on_fallback_only_mapping_is_none() {
        let mut mapping = Mapping::new();
        for index in 0..1_000 {
            mapping.insert(Value::Float(f64::from(index) + 0.5), Value::Null);
        }

        assert!(mapping.scalar_index.is_none());
        assert_eq!(mapping.get(&Value::from("missing")), None);
        assert_eq!(mapping.get_str("missing"), None);
    }

    #[test]
    fn scalar_and_fallback_keys_preserve_replacement_semantics() {
        let keys = [
            Value::Null,
            Value::Bool(true),
            Value::Integer(7),
            Value::from("seven"),
            Value::Sequence(vec![Value::Integer(7)]),
        ];
        let mut mapping = Mapping::new();
        for key in &keys {
            assert_eq!(mapping.insert(key.clone(), Value::Integer(1)), None);
            assert_eq!(
                mapping.insert(key.clone(), Value::Integer(2)),
                Some(Value::Integer(1))
            );
        }
        assert_eq!(mapping.len(), keys.len());
        for key in &keys {
            assert_eq!(mapping.get(key), Some(&Value::Integer(2)));
        }
    }

    #[test]
    fn fallback_keys_preserve_float_and_nested_mapping_equality() {
        let mut signed_zero = Mapping::new();
        assert_eq!(
            signed_zero.insert(Value::Float(-0.0), Value::Integer(1)),
            None
        );
        assert_eq!(
            signed_zero.insert(Value::Float(0.0), Value::Integer(2)),
            Some(Value::Integer(1))
        );
        assert_eq!(signed_zero.len(), 1);

        let mut nan = Mapping::new();
        assert_eq!(nan.insert(Value::Float(f64::NAN), Value::Integer(1)), None);
        assert_eq!(nan.insert(Value::Float(f64::NAN), Value::Integer(2)), None);
        assert_eq!(nan.len(), 2);

        let mut nested_key = Mapping::new();
        nested_key.insert(Value::from("nested"), Value::Bool(true));
        let nested_key = Value::Mapping(nested_key);
        let mut outer = Mapping::new();
        assert_eq!(outer.insert(nested_key.clone(), Value::Integer(1)), None);
        assert_eq!(
            outer.insert(nested_key, Value::Integer(2)),
            Some(Value::Integer(1))
        );
        assert_eq!(outer.len(), 1);
    }

    #[test]
    fn fingerprint_collisions_compare_actual_string_keys() {
        let mut mapping = Mapping::new();
        mapping.insert(Value::from("left"), Value::Integer(1));
        mapping.insert(Value::from("right"), Value::Integer(2));
        let scalar_index = mapping.scalar_index.as_mut().expect("scalar index");
        let fingerprint = scalar_index.fingerprint_str("right");
        scalar_index
            .positions
            .get_mut(&fingerprint)
            .unwrap()
            .insert(0, 0);

        assert_eq!(mapping.get_str("right"), Some(&Value::Integer(2)));
        assert_eq!(
            mapping.insert(Value::from("right"), Value::Integer(3)),
            Some(Value::Integer(2))
        );
        assert_eq!(mapping.len(), 2);
    }

    #[test]
    fn clone_equality_and_debug_ignore_index_identity() {
        let mut mapping = Mapping::new();
        mapping.insert(Value::from("key"), Value::Integer(1));
        let mut same_entries = mapping.clone();
        same_entries
            .scalar_index
            .as_mut()
            .expect("scalar index")
            .positions
            .clear();

        assert_eq!(mapping, same_entries);
        assert_eq!(
            format!("{mapping:?}"),
            "Mapping { entries: [(String(\"key\"), Integer(1))] }"
        );
    }

    #[test]
    fn sequence_index() {
        let seq = Value::Sequence(vec![Value::from("a"), Value::from("b"), Value::from("c")]);
        assert_eq!(seq.get_index(1), Some(&Value::from("b")));
        assert_eq!(seq.get_index(5), None);
    }

    #[test]
    fn as_str_returns_string_content() {
        assert_eq!(Value::from("hello").as_str(), Some("hello"));
        assert_eq!(Value::Null.as_str(), None);
    }

    #[test]
    fn as_bool_coerces() {
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::from("true").as_bool(), None);
    }

    #[test]
    fn as_i64_returns_integer() {
        assert_eq!(Value::Integer(42).as_i64(), Some(42));
    }

    #[test]
    fn as_f64_returns_float() {
        assert_eq!(Value::Float(2.5).as_f64(), Some(2.5));
    }

    #[test]
    fn from_str_creates_string() {
        let v: Value = "hello".into();
        assert!(matches!(v, Value::String(s) if s == "hello"));
    }

    #[test]
    fn display_null() {
        assert_eq!(format!("{}", Value::Null), "null");
    }

    #[test]
    fn mapping_preserves_insertion_order() {
        let mut m = Mapping::new();
        m.insert(Value::from("z"), Value::from(1_i64));
        m.insert(Value::from("a"), Value::from(2_i64));

        let keys: Vec<&Value> = m.keys().collect();
        assert_eq!(keys, vec![&Value::from("z"), &Value::from("a")]);
    }

    #[test]
    fn mapping_supports_standard_borrowed_mutable_and_owned_iteration() {
        let mut mapping = Mapping::new();
        mapping.insert(Value::from("first"), Value::Integer(1));
        mapping.insert(Value::from("second"), Value::Integer(2));

        let borrowed = (&mapping)
            .into_iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            borrowed,
            vec![
                (Value::from("first"), Value::Integer(1)),
                (Value::from("second"), Value::Integer(2)),
            ]
        );

        for (_, value) in &mut mapping {
            let Value::Integer(integer) = value else {
                panic!("test mapping values must be integers");
            };
            *integer += 10;
        }
        assert_eq!(mapping.get_str("first"), Some(&Value::Integer(11)));

        assert_eq!(
            mapping.into_iter().collect::<Vec<_>>(),
            vec![
                (Value::from("first"), Value::Integer(11)),
                (Value::from("second"), Value::Integer(12)),
            ]
        );
    }

    // -- serde Deserialize ---------------------------------------------------
    //
    // Driven through serde_json: it exercises visitor paths the yamalgam
    // deserializer never hits (visit_u64 for positive integers).

    #[test]
    fn deserialize_value_via_serde_json() {
        let v: Value = serde_json::from_str(
            r#"{"name":"test","count":3,"ratio":0.5,"ok":true,"nothing":null,"items":[1,2]}"#,
        )
        .unwrap();
        assert_eq!(v.get("name"), Some(&Value::from("test")));
        assert_eq!(v.get("count"), Some(&Value::Integer(3)));
        assert_eq!(v.get("ratio"), Some(&Value::Float(0.5)));
        assert_eq!(v.get("ok"), Some(&Value::Bool(true)));
        assert_eq!(v.get("nothing"), Some(&Value::Null));
        assert_eq!(
            v.get("items"),
            Some(&Value::Sequence(vec![Value::Integer(1), Value::Integer(2)]))
        );
    }

    #[test]
    fn deserialize_value_negative_integer() {
        let v: Value = serde_json::from_str("-7").unwrap();
        assert_eq!(v, Value::Integer(-7));
    }

    #[test]
    fn deserialize_value_u64_in_i64_range() {
        // serde_json visits positive integers as u64.
        let v: Value = serde_json::from_str("42").unwrap();
        assert_eq!(v, Value::Integer(42));
    }

    #[test]
    fn deserialize_value_u64_overflow_errors() {
        // u64::MAX does not fit Value::Integer (i64) — must error, not wrap.
        let result: Result<Value, _> = serde_json::from_str("18446744073709551615");
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_value_duplicate_keys_last_wins() {
        // Mapping::insert replaces — consistent with the Composer.
        let v: Value = serde_json::from_str(r#"{"k":1,"k":2}"#).unwrap();
        let Value::Mapping(m) = &v else {
            panic!("expected mapping");
        };
        assert_eq!(m.len(), 1);
        assert_eq!(v.get("k"), Some(&Value::Integer(2)));
    }
}

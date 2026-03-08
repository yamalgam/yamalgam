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
use std::fmt;

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
/// Backed by a `Vec<(Value, Value)>`. Insert replaces an existing key (linear
/// scan) rather than appending a duplicate. This keeps the API predictable at
/// the cost of O(n) lookups — fine for typical YAML document sizes.
#[derive(Debug, Clone, PartialEq)]
// y[impl model.repr.node.mapping]
// y[impl model.repr.uniqueness]
// y[impl model.serial.key-order.serialization-detail]
// y[impl model.serial.key-order.no-representation-order]
pub struct Mapping {
    entries: Vec<(Value, Value)>,
}

impl Mapping {
    /// Creates an empty mapping.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Inserts a key/value pair. If the key already exists, its value is
    /// replaced and the old value is returned.
    pub fn insert(&mut self, key: Value, value: Value) -> Option<Value> {
        for entry in &mut self.entries {
            if entry.0 == key {
                let old = std::mem::replace(&mut entry.1, value);
                return Some(old);
            }
        }
        self.entries.push((key, value));
        None
    }

    /// Looks up a value by key.
    #[must_use]
    pub fn get(&self, key: &Value) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
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
            Self::Mapping(m) => m.get(&Self::String(key.to_owned())),
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
}

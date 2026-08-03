//! The metadata-carrying YAML value type.

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use std::rc::Rc;

use indexmap::IndexMap;
use yamalgam_parser::CollectionStyle;
use yamalgam_scanner::ScalarStyle;

/// Presentation style of a YAML node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeStyle {
    /// Scalar presentation such as plain, quoted, literal, or folded.
    Scalar(ScalarStyle),
    /// Collection presentation as block or flow.
    Collection(CollectionStyle),
}

/// YAML node metadata, excluded from resolved equality, ordering, and hashing.
// r[datamodel.metadata]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Meta {
    /// Explicit tag as carried by the parser.
    pub tag: Option<Rc<str>>,
    /// Anchor name without the `&` prefix.
    pub anchor: Option<Rc<str>>,
    /// Referenced anchor name when this node came from an alias.
    pub alias_of: Option<Rc<str>>,
    /// Presentation style from the source document.
    pub style: Option<NodeStyle>,
    /// Raw scalar text before schema resolution.
    pub raw: Option<Rc<str>>,
}

/// Order-preserving YAML mapping with order-sensitive equality.
// r[comparison.resolved.collection]
#[derive(Clone, Debug, Default)]
pub struct YgMap(pub(crate) IndexMap<YamlVal, YamlVal>);

/// Resolved YAML value data.
#[derive(Clone, Debug, Default)]
pub enum Data {
    /// Null value. r[datamodel.types.null]
    #[default]
    Null,
    /// Boolean value. r[datamodel.types.bool]
    Bool(bool),
    /// Signed 64-bit integer. r[datamodel.types.int]
    Int(i64),
    /// IEEE 754 double-precision float. r[datamodel.types.float]
    Float(f64),
    /// Unicode string. r[datamodel.types.str]
    Str(Rc<str>),
    /// Sequence. r[datamodel.types.seq]
    Seq(Rc<Vec<YamlVal>>),
    /// Mapping with arbitrary resolved-value keys. r[datamodel.types.mapping]
    Map(Rc<YgMap>),
}

/// Resolved YAML data plus optional source presentation metadata.
// r[datamodel]
#[derive(Clone, Debug, Default)]
pub struct YamlVal {
    pub(crate) data: Data,
    pub(crate) meta: Option<Rc<Meta>>,
}

impl YamlVal {
    /// Construct null without source metadata.
    #[must_use]
    pub const fn null() -> Self {
        Self {
            data: Data::Null,
            meta: None,
        }
    }

    /// Construct an integer without source metadata.
    #[must_use]
    pub const fn int(value: i64) -> Self {
        Self {
            data: Data::Int(value),
            meta: None,
        }
    }

    /// Construct a float without source metadata.
    #[must_use]
    pub const fn float(value: f64) -> Self {
        Self {
            data: Data::Float(value),
            meta: None,
        }
    }

    /// Construct a string without source metadata.
    #[must_use]
    pub fn str(value: impl Into<Rc<str>>) -> Self {
        Self {
            data: Data::Str(value.into()),
            meta: None,
        }
    }

    /// Construct a sequence without source metadata.
    #[must_use]
    pub fn seq(items: Vec<Self>) -> Self {
        Self {
            data: Data::Seq(Rc::new(items)),
            meta: None,
        }
    }

    /// Construct an insertion-ordered mapping without source metadata.
    #[must_use]
    pub fn map(pairs: Vec<(Self, Self)>) -> Self {
        Self {
            data: Data::Map(Rc::new(pairs.into_iter().collect())),
            meta: None,
        }
    }

    /// Attach metadata, replacing any existing metadata.
    #[must_use]
    pub fn with_meta(mut self, meta: Meta) -> Self {
        self.meta = Some(Rc::new(meta));
        self
    }

    /// Return the node metadata, when present.
    #[must_use]
    pub fn meta(&self) -> Option<&Meta> {
        self.meta.as_deref()
    }

    /// Return mutable metadata, creating an empty metadata record on demand.
    pub(crate) fn meta_mut(&mut self) -> &mut Meta {
        Rc::make_mut(self.meta.get_or_insert_with(|| Rc::new(Meta::default())))
    }

    /// Return the YAML data-model type name.
    // r[diverge.type]
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match &self.data {
            Data::Null => "null",
            Data::Bool(_) => "boolean",
            Data::Int(_) => "integer",
            Data::Float(_) => "float",
            Data::Str(_) => "string",
            Data::Seq(_) => "sequence",
            Data::Map(_) => "mapping",
        }
    }

    /// Return the source node kind without schema resolution.
    // r[ext.kind]
    #[must_use]
    pub fn kind_name(&self) -> &'static str {
        if self.meta().is_some_and(|meta| meta.alias_of.is_some()) {
            return "alias";
        }
        match &self.data {
            Data::Seq(_) => "sequence",
            Data::Map(_) => "mapping",
            _ => "scalar",
        }
    }

    /// Return a core tag in `!!name` shorthand, preserving other tag forms.
    // r[ext.tag]
    #[must_use]
    pub fn tag_shorthand(&self) -> Option<String> {
        let tag = self.meta().and_then(|meta| meta.tag.as_deref())?;
        if tag.starts_with("!!") {
            return Some(tag.to_owned());
        }
        Some(
            tag.strip_prefix("tag:yaml.org,2002:")
                .map_or_else(|| tag.to_owned(), |suffix| format!("!!{suffix}")),
        )
    }

    /// Look up a mapping value by a string key using resolved comparison.
    #[must_use]
    pub fn get_str(&self, key: &str) -> Option<&Self> {
        match &self.data {
            Data::Map(map) => map.0.get(&Self::str(key)),
            _ => None,
        }
    }

    /// Iterate mapping keys in insertion order, or return an empty iterator.
    pub fn keys(&self) -> Box<dyn Iterator<Item = &Self> + '_> {
        match &self.data {
            Data::Map(map) => Box::new(map.0.keys()),
            _ => Box::new(core::iter::empty()),
        }
    }
}

impl From<bool> for YamlVal {
    fn from(value: bool) -> Self {
        Self {
            data: Data::Bool(value),
            meta: None,
        }
    }
}

impl From<String> for YamlVal {
    fn from(value: String) -> Self {
        Self::str(value)
    }
}

impl From<isize> for YamlVal {
    fn from(value: isize) -> Self {
        Self::int(value as i64)
    }
}

impl From<usize> for YamlVal {
    fn from(value: usize) -> Self {
        i64::try_from(value).map_or_else(|_| Self::float(value as f64), Self::int)
    }
}

impl From<f64> for YamlVal {
    fn from(value: f64) -> Self {
        Self::float(value)
    }
}

impl FromIterator<Self> for YamlVal {
    fn from_iter<T: IntoIterator<Item = Self>>(iter: T) -> Self {
        Self::seq(iter.into_iter().collect())
    }
}

// r[comparison.resolved]
impl PartialEq for YamlVal {
    fn eq(&self, other: &Self) -> bool {
        match (&self.data, &other.data) {
            (Data::Null, Data::Null) => true,
            (Data::Bool(left), Data::Bool(right)) => left == right,
            (Data::Int(left), Data::Int(right)) => left == right,
            (Data::Float(left), Data::Float(right)) => float_cmp(*left, *right) == Ordering::Equal,
            (Data::Int(left), Data::Float(right)) | (Data::Float(right), Data::Int(left)) => {
                right.is_finite() && float_cmp(*left as f64, *right) == Ordering::Equal
            }
            (Data::Str(left), Data::Str(right)) => left == right,
            (Data::Seq(left), Data::Seq(right)) => left == right,
            (Data::Map(left), Data::Map(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for YamlVal {}

impl PartialEq for YgMap {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && self
                .0
                .iter()
                .zip(other.0.iter())
                .all(|(left, right)| left == right)
    }
}

impl Eq for YgMap {}

impl PartialOrd for YamlVal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// r[comparison.ordering]
impl Ord for YamlVal {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.data, &other.data) {
            (Data::Null, Data::Null) => Ordering::Equal,
            (Data::Bool(left), Data::Bool(right)) => left.cmp(right),
            (Data::Int(left), Data::Int(right)) => left.cmp(right),
            (Data::Float(left), Data::Float(right)) => float_cmp(*left, *right),
            (Data::Int(left), Data::Float(right)) => float_cmp(*left as f64, *right),
            (Data::Float(left), Data::Int(right)) => float_cmp(*left, *right as f64),
            (Data::Str(left), Data::Str(right)) => left.cmp(right),
            (Data::Seq(left), Data::Seq(right)) => left.cmp(right),
            (Data::Map(left), Data::Map(right)) => left.0.iter().cmp(right.0.iter()),
            (left, right) => data_rank(left).cmp(&data_rank(right)),
        }
    }
}

impl Hash for YamlVal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match &self.data {
            Data::Null => state.write_u8(0),
            Data::Bool(value) => {
                state.write_u8(1);
                value.hash(state);
            }
            Data::Int(value) => hash_number(*value as f64, state),
            Data::Float(value) => hash_number(*value, state),
            Data::Str(value) => {
                state.write_u8(3);
                value.hash(state);
            }
            Data::Seq(items) => {
                state.write_u8(4);
                items.len().hash(state);
                items.iter().for_each(|item| item.hash(state));
            }
            Data::Map(map) => {
                state.write_u8(5);
                map.0.len().hash(state);
                map.0.iter().for_each(|entry| entry.hash(state));
            }
        }
    }
}

impl FromIterator<(YamlVal, YamlVal)> for YgMap {
    fn from_iter<T: IntoIterator<Item = (YamlVal, YamlVal)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

const fn data_rank(data: &Data) -> u8 {
    match data {
        Data::Null => 0,
        Data::Bool(_) => 1,
        Data::Int(_) | Data::Float(_) => 2,
        Data::Str(_) => 3,
        Data::Seq(_) => 4,
        Data::Map(_) => 5,
    }
}

fn float_cmp(left: f64, right: f64) -> Ordering {
    match (left.is_nan(), right.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) if left == 0.0 && right == 0.0 => Ordering::Equal,
        (false, false) => left.total_cmp(&right),
    }
}

fn hash_number<H: Hasher>(value: f64, state: &mut H) {
    state.write_u8(2);
    let bits = if value.is_nan() {
        f64::NAN.to_bits()
    } else if value == 0.0 {
        0.0_f64.to_bits()
    } else {
        value.to_bits()
    };
    bits.hash(state);
}

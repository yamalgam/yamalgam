//! The metadata-carrying YAML value type.

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use std::rc::Rc;

use indexmap::{IndexMap, IndexSet};
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
// r[impl datamodel.metadata]
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
// r[impl comparison.resolved.collection]
#[derive(Clone, Debug, Default)]
pub struct YgMap(pub(crate) IndexMap<YamlVal, YamlVal>);

/// YAML sequence storage with iterative destruction for deeply nested values.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YgSeq(pub(crate) Vec<YamlVal>);

/// Resolved YAML value data.
// r[impl datamodel.types]
#[derive(Clone, Debug, Default)]
pub enum Data {
    /// Null value.
    // r[impl datamodel.types.null]
    #[default]
    Null,
    /// Boolean value.
    // r[impl datamodel.types.bool]
    Bool(bool),
    /// Signed 64-bit integer.
    // r[impl datamodel.types.int]
    Int(i64),
    /// IEEE 754 double-precision float.
    // r[impl datamodel.types.float]
    Float(f64),
    /// Unicode string.
    // r[impl datamodel.types.str]
    Str(Rc<str>),
    /// Sequence.
    // r[impl datamodel.types.seq]
    Seq(Rc<YgSeq>),
    /// Mapping with arbitrary resolved-value keys.
    // r[impl datamodel.types.mapping]
    Map(Rc<YgMap>),
}

/// Resolved YAML data plus optional source presentation metadata.
///
/// Values use [`Rc`] for cheap cloning during jaq evaluation and are therefore
/// intentionally neither `Send` nor `Sync`. Parse and evaluate them on the
/// thread where they will be consumed.
// r[impl datamodel]
#[derive(Clone, Debug, Default)]
pub struct YamlVal {
    pub(crate) data: Data,
    pub(crate) meta: Option<Rc<Meta>>,
}

#[derive(Default)]
struct ExpansionBudget {
    aliases: usize,
    entries: usize,
}

impl ExpansionBudget {
    fn count_alias(
        &mut self,
        limits: &yamalgam_core::ResourceLimits,
    ) -> jaq_core::ValR<(), YamlVal> {
        self.aliases = self
            .aliases
            .checked_add(1)
            .ok_or_else(|| jaq_core::Error::str("alias expansion count overflow"))?;
        limits
            .check_alias_expansions(self.aliases)
            .map_err(jaq_core::Error::str)
    }

    fn count_entries(
        &mut self,
        count: usize,
        limits: &yamalgam_core::ResourceLimits,
    ) -> jaq_core::ValR<(), YamlVal> {
        self.entries = self
            .entries
            .checked_add(count)
            .ok_or_else(|| jaq_core::Error::str("materialized entry count overflow"))?;
        limits
            .check_collection_size(self.entries)
            .map_err(jaq_core::Error::str)
    }
}

impl YgSeq {
    pub(crate) fn into_values(mut self) -> Vec<YamlVal> {
        core::mem::take(&mut self.0)
    }
}

impl core::ops::Deref for YgSeq {
    type Target = Vec<YamlVal>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::DerefMut for YgSeq {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl YgMap {
    pub(crate) fn into_entries(mut self) -> IndexMap<YamlVal, YamlVal> {
        core::mem::take(&mut self.0)
    }
}

impl Drop for YgSeq {
    fn drop(&mut self) {
        drop_values(core::mem::take(&mut self.0));
    }
}

impl Drop for YgMap {
    fn drop(&mut self) {
        let entries = core::mem::take(&mut self.0);
        drop_values(entries.into_iter().flat_map(<[_; 2]>::from));
    }
}

fn drop_values(values: impl IntoIterator<Item = YamlVal>) {
    let mut stack = values.into_iter().collect::<Vec<_>>();
    while let Some(mut value) = stack.pop() {
        match core::mem::take(&mut value.data) {
            Data::Seq(items) => {
                if let Ok(items) = Rc::try_unwrap(items) {
                    stack.extend(items.into_values());
                }
            }
            Data::Map(mapping) => {
                if let Ok(mapping) = Rc::try_unwrap(mapping) {
                    stack.extend(mapping.into_entries().into_iter().flat_map(<[_; 2]>::from));
                }
            }
            Data::Null | Data::Bool(_) | Data::Int(_) | Data::Float(_) | Data::Str(_) => {}
        }
    }
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
            data: Data::Seq(Rc::new(YgSeq(items))),
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

    /// Return the resolved boolean value.
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self.data {
            Data::Bool(value) => Some(value),
            _ => None,
        }
    }

    /// Return the resolved signed integer value.
    #[must_use]
    pub const fn as_i64(&self) -> Option<i64> {
        match self.data {
            Data::Int(value) => Some(value),
            _ => None,
        }
    }

    /// Return the resolved numeric value as an `f64`.
    #[must_use]
    pub const fn as_f64(&self) -> Option<f64> {
        match self.data {
            Data::Int(value) => Some(value as f64),
            Data::Float(value) => Some(value),
            _ => None,
        }
    }

    /// Return the resolved sequence contents.
    #[must_use]
    pub fn as_sequence(&self) -> Option<&[Self]> {
        match &self.data {
            Data::Seq(values) => Some(values.as_slice()),
            _ => None,
        }
    }

    /// Iterate over resolved mapping entries in insertion order.
    pub fn entries(&self) -> Option<impl ExactSizeIterator<Item = (&Self, &Self)>> {
        match &self.data {
            Data::Map(mapping) => Some(mapping.0.iter()),
            _ => None,
        }
    }

    /// Return mutable metadata, creating an empty metadata record on demand.
    pub(crate) fn meta_mut(&mut self) -> &mut Meta {
        Rc::make_mut(self.meta.get_or_insert_with(|| Rc::new(Meta::default())))
    }

    /// Return mutable value data after invalidating source identity and raw text.
    pub(crate) fn data_mut(&mut self) -> &mut Data {
        if let Some(metadata) = &mut self.meta {
            let metadata = Rc::make_mut(metadata);
            metadata.anchor = None;
            metadata.alias_of = None;
            metadata.raw = None;
        }
        &mut self.data
    }

    /// Return the YAML data-model type name.
    // r[impl diverge.type]
    // r[impl diverge.type.rationale]
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
    // r[impl ext.kind]
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
    // r[impl ext.tag]
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

    /// Return the source presentation style name.
    // r[impl ext.style]
    #[must_use]
    pub fn style_name(&self) -> Option<&'static str> {
        self.meta()
            .and_then(|metadata| metadata.style)
            .map(|style| match style {
                NodeStyle::Scalar(ScalarStyle::Plain) => "plain",
                NodeStyle::Scalar(ScalarStyle::SingleQuoted) => "single-quoted",
                NodeStyle::Scalar(ScalarStyle::DoubleQuoted) => "double-quoted",
                NodeStyle::Scalar(ScalarStyle::Literal) => "literal",
                NodeStyle::Scalar(ScalarStyle::Folded) => "folded",
                NodeStyle::Collection(CollectionStyle::Block) => "block",
                NodeStyle::Collection(CollectionStyle::Flow) => "flow",
            })
    }

    // r[impl ext.raw]
    pub(crate) fn raw_text(&self) -> jaq_core::ValR<Self> {
        match &self.data {
            Data::Seq(_) | Data::Map(_) => Err(jaq_core::Error::str(format_args!(
                "{} has no raw form",
                self.type_name()
            ))),
            Data::Str(string) => Ok(Self::str(
                self.meta()
                    .and_then(|metadata| metadata.raw.clone())
                    .unwrap_or_else(|| string.clone()),
            )),
            _ => Ok(Self::str(
                self.meta()
                    .and_then(|metadata| metadata.raw.as_deref().map(str::to_owned))
                    .unwrap_or_else(|| self.to_string()),
            )),
        }
    }

    // r[impl ext.resolve]
    // r[impl ext.resolve-as]
    pub(crate) fn resolve_with(
        &self,
        schema: yamalgam_core::TagResolution,
    ) -> jaq_core::ValR<Self> {
        use yamalgam_core::TagResolver as _;

        let text = match (
            &self.data,
            self.meta().and_then(|metadata| metadata.raw.as_deref()),
        ) {
            (Data::Seq(_) | Data::Map(_), _) => {
                return Err(jaq_core::Error::typ(self.clone(), "scalar"));
            }
            (_, Some(raw)) => raw.to_owned(),
            (Data::Str(string), None) => string.to_string(),
            _ => self.to_string(),
        };
        Ok(Self {
            data: crate::build::core_value_to_data(schema.resolve_scalar(&text)),
            meta: None,
        })
    }

    // r[impl ext.set-tag]
    pub(crate) fn set_tag(mut self, tag: Self) -> jaq_core::ValR<Self> {
        let tag = match tag.data {
            Data::Null => None,
            Data::Str(tag) if is_valid_tag(&tag) => Some(tag.strip_prefix("!!").map_or_else(
                || tag.clone(),
                |suffix| Rc::from(format!("tag:yaml.org,2002:{suffix}")),
            )),
            Data::Str(tag) => {
                return Err(jaq_core::Error::str(format_args!(
                    "invalid YAML tag {tag:?}"
                )));
            }
            data => {
                return Err(jaq_core::Error::typ(
                    Self {
                        data,
                        meta: tag.meta,
                    },
                    "string or null",
                ));
            }
        };
        self.meta_mut().tag = tag;
        Ok(self)
    }

    // r[impl ext.set-anchor]
    pub(crate) fn set_anchor(mut self, anchor: Self) -> jaq_core::ValR<Self> {
        let anchor = match anchor.data {
            Data::Null => None,
            Data::Str(anchor)
                if !anchor.is_empty()
                    && anchor.chars().all(|character| {
                        character.is_ascii_alphanumeric() || "_-".contains(character)
                    }) =>
            {
                Some(anchor)
            }
            Data::Str(anchor) => {
                return Err(jaq_core::Error::str(format_args!(
                    "invalid anchor name {anchor:?}; expected [A-Za-z0-9_-]+"
                )));
            }
            data => {
                return Err(jaq_core::Error::typ(
                    Self {
                        data,
                        meta: anchor.meta,
                    },
                    "string or null",
                ));
            }
        };
        let metadata = self.meta_mut();
        metadata.anchor = anchor;
        if metadata.anchor.is_some() {
            metadata.alias_of = None;
        }
        Ok(self)
    }

    // r[impl ext.set-style]
    pub(crate) fn set_style(mut self, style: Self) -> jaq_core::ValR<Self> {
        let Data::Str(style_name) = style.data else {
            return Err(jaq_core::Error::typ(style, "style name string"));
        };
        let scalar = !matches!(&self.data, Data::Seq(_) | Data::Map(_));
        let style = match (style_name.as_ref(), scalar) {
            ("plain", true) => NodeStyle::Scalar(ScalarStyle::Plain),
            ("single-quoted", true) => NodeStyle::Scalar(ScalarStyle::SingleQuoted),
            ("double-quoted", true) => NodeStyle::Scalar(ScalarStyle::DoubleQuoted),
            ("literal", true) => NodeStyle::Scalar(ScalarStyle::Literal),
            ("folded", true) => NodeStyle::Scalar(ScalarStyle::Folded),
            ("block", false) => NodeStyle::Collection(CollectionStyle::Block),
            ("flow", false) => NodeStyle::Collection(CollectionStyle::Flow),
            (name, _) => {
                return Err(jaq_core::Error::str(format_args!(
                    "style {name:?} does not apply to {} nodes",
                    if scalar { "scalar" } else { "collection" }
                )));
            }
        };
        self.meta_mut().style = Some(style);
        Ok(self)
    }

    // r[impl ext.merge]
    pub(crate) fn merge_keys(&self) -> jaq_core::ValR<Self> {
        let Data::Map(mapping) = &self.data else {
            return Err(jaq_core::Error::typ(self.clone(), "mapping"));
        };
        let mut output = IndexMap::new();

        for (key, value) in &mapping.0 {
            if key.tag_shorthand().as_deref() != Some("!!merge") {
                output.insert(key.clone(), value.clone());
                continue;
            }

            let sources = match &value.data {
                Data::Map(_) => vec![value],
                Data::Seq(values) => values.iter().collect(),
                _ => {
                    return Err(jaq_core::Error::typ(
                        value.clone(),
                        "mapping or sequence of mappings",
                    ));
                }
            };
            let mut defaults = IndexMap::new();
            for source in sources {
                let Data::Map(source) = &source.data else {
                    return Err(jaq_core::Error::typ(source.clone(), "mapping"));
                };
                for (key, value) in &source.0 {
                    defaults.entry(key.clone()).or_insert_with(|| value.clone());
                }
            }
            for (key, value) in defaults {
                output.entry(key).or_insert(value);
            }
        }

        Ok(Self {
            data: Data::Map(Rc::new(YgMap(output))),
            meta: self.meta.clone(),
        })
    }

    // r[impl ext.explode]
    pub(crate) fn explode_aliases(
        &self,
        limits: &yamalgam_core::ResourceLimits,
    ) -> jaq_core::ValR<Self> {
        self.explode_aliases_bounded(limits, &mut ExpansionBudget::default(), 0)
    }

    fn explode_aliases_bounded(
        &self,
        limits: &yamalgam_core::ResourceLimits,
        budget: &mut ExpansionBudget,
        depth: usize,
    ) -> jaq_core::ValR<Self> {
        limits.check_depth(depth).map_err(jaq_core::Error::str)?;
        if self
            .meta()
            .is_some_and(|metadata| metadata.alias_of.is_some())
        {
            budget.count_alias(limits)?;
        }

        let data = match &self.data {
            Data::Seq(items) => {
                budget.count_entries(items.len(), limits)?;
                let child_depth = depth
                    .checked_add(1)
                    .ok_or_else(|| jaq_core::Error::str("value depth overflow"))?;
                let items = items
                    .iter()
                    .map(|item| item.explode_aliases_bounded(limits, budget, child_depth))
                    .collect::<jaq_core::ValR<Vec<_>, Self>>()?;
                Data::Seq(Rc::new(YgSeq(items)))
            }
            Data::Map(mapping) => {
                budget.count_entries(mapping.0.len(), limits)?;
                let child_depth = depth
                    .checked_add(1)
                    .ok_or_else(|| jaq_core::Error::str("value depth overflow"))?;
                let mapping = mapping
                    .0
                    .iter()
                    .map(|(key, value)| {
                        Ok::<_, jaq_core::Error<Self>>((
                            key.explode_aliases_bounded(limits, budget, child_depth)?,
                            value.explode_aliases_bounded(limits, budget, child_depth)?,
                        ))
                    })
                    .collect::<jaq_core::ValR<YgMap, Self>>()?;
                Data::Map(Rc::new(mapping))
            }
            data => data.clone(),
        };
        let mut meta = self.meta.clone();
        if let Some(metadata) = &mut meta {
            let metadata = Rc::make_mut(metadata);
            metadata.anchor = None;
            metadata.alias_of = None;
        }
        Ok(Self { data, meta })
    }

    #[must_use]
    // r[impl ext.aliases]
    pub(crate) fn alias_report(&self) -> Self {
        let inventory = IdentityInventory::collect(self);
        Self::map(
            inventory
                .aliases
                .into_iter()
                .map(|(anchor, paths)| {
                    let paths = paths.into_iter().map(Self::seq).collect();
                    (Self::str(anchor), Self::seq(paths))
                })
                .collect(),
        )
    }

    #[must_use]
    // r[impl ext.orphan-anchors]
    pub(crate) fn orphan_anchors(&self) -> Self {
        let inventory = IdentityInventory::collect(self);
        Self::seq(
            inventory
                .anchors
                .into_iter()
                .filter(|anchor| !inventory.aliases.contains_key(anchor.as_ref()))
                .map(Self::str)
                .collect(),
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

    /// Return whether this value is a string.
    #[must_use]
    pub const fn is_string(&self) -> bool {
        matches!(&self.data, Data::Str(_))
    }

    /// Borrow this value's string content.
    #[must_use]
    pub fn as_str_content(&self) -> Option<&str> {
        match &self.data {
            Data::Str(string) => Some(string),
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

    /// Consume a sequence into its elements, returning other values unchanged.
    pub(crate) fn into_seq_vec(self) -> Result<Vec<Self>, Self> {
        match self.data {
            Data::Seq(items) => Ok(Rc::unwrap_or_clone(items).into_values()),
            _ => Err(self),
        }
    }
}

struct IdentityInventory {
    anchors: IndexSet<Rc<str>>,
    aliases: IndexMap<String, Vec<Vec<YamlVal>>>,
}

impl IdentityInventory {
    fn collect(value: &YamlVal) -> Self {
        let mut inventory = Self {
            anchors: IndexSet::new(),
            aliases: IndexMap::new(),
        };
        inventory.visit(value, &mut Vec::new());
        inventory
    }

    fn visit(&mut self, value: &YamlVal, path: &mut Vec<YamlVal>) {
        if let Some(alias) = value
            .meta()
            .and_then(|metadata| metadata.alias_of.as_deref())
        {
            self.aliases
                .entry(alias.to_owned())
                .or_default()
                .push(path.clone());
            return;
        }
        if let Some(anchor) = value.meta().and_then(|metadata| metadata.anchor.as_ref()) {
            self.anchors.insert(Rc::clone(anchor));
        }

        match &value.data {
            Data::Seq(items) => {
                for (index, item) in items.iter().enumerate() {
                    path.push(YamlVal::from(index));
                    self.visit(item, path);
                    path.pop();
                }
            }
            Data::Map(mapping) => {
                for (key, value) in &mapping.0 {
                    path.push(YamlVal::map(vec![(YamlVal::str("$key"), key.clone())]));
                    self.visit(key, path);
                    path.pop();
                    path.push(key.clone());
                    self.visit(value, path);
                    path.pop();
                }
            }
            _ => {}
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
    // r[impl filters.construct.seq]
    fn from_iter<T: IntoIterator<Item = Self>>(iter: T) -> Self {
        Self::seq(iter.into_iter().collect())
    }
}

// r[impl comparison]
// r[impl comparison.resolved]
// r[impl comparison.resolved.type-coercion]
// r[impl comparison.resolved.numeric]
// r[impl comparison.resolved.string]
// r[impl comparison.resolved.null]
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

// r[impl comparison.ordering]
// r[impl comparison.ordering.numeric]
// r[impl comparison.ordering.string]
// r[impl comparison.ordering.cross-type]
// r[impl diverge.sort]
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

impl core::fmt::Display for YamlVal {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self.data {
            Data::Null => formatter.write_str("null"),
            Data::Bool(value) => write!(formatter, "{value}"),
            Data::Int(value) => write!(formatter, "{value}"),
            Data::Float(value) => formatter.write_str(&fmt_float(*value)),
            Data::Str(value)
                if needs_quotes(value, yamalgam_core::TagResolution::Yaml12, false) =>
            {
                formatter.write_str(&quote_double(value))
            }
            Data::Str(value) => formatter.write_str(value),
            Data::Seq(items) => {
                formatter.write_str("[")?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    write!(formatter, "{item}")?;
                }
                formatter.write_str("]")
            }
            Data::Map(map) => {
                formatter.write_str("{")?;
                for (index, (key, value)) in map.0.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    write!(formatter, "{key}: {value}")?;
                }
                formatter.write_str("}")
            }
        }
    }
}

/// Return whether a tag can be emitted without changing YAML structure.
pub(crate) fn is_valid_tag(tag: &str) -> bool {
    fn is_uri_char(character: char) -> bool {
        character.is_ascii_alphanumeric()
            || "-#;/?:@&=+$,_.!~*'()[]%".contains(character)
            || (!character.is_ascii() && !character.is_control() && !character.is_whitespace())
    }

    fn is_tag_char(character: char) -> bool {
        is_uri_char(character) && !"!,[]{}".contains(character)
    }

    if let Some(verbatim) = tag.strip_prefix("!<") {
        let Some(uri) = verbatim.strip_suffix('>') else {
            return false;
        };
        return !uri.is_empty() && uri.chars().all(is_uri_char);
    }

    if let Some(core) = tag.strip_prefix("!!") {
        return !core.is_empty() && core.chars().all(is_tag_char);
    }

    if let Some(local) = tag.strip_prefix('!') {
        return local.chars().all(is_tag_char);
    }

    !tag.is_empty() && tag.chars().all(is_uri_char)
}

/// Return whether plain YAML emission would change a string's value or syntax.
// r[impl output.yaml.quoting]
pub(crate) fn needs_quotes(value: &str, schema: yamalgam_core::TagResolution, flow: bool) -> bool {
    use yamalgam_core::TagResolver as _;

    if value.is_empty() || !schema.resolves_as_string(value) {
        return true;
    }

    let first = value.chars().next().unwrap_or(' ');
    let syntactic_first = "!&*-?:,[]{}#|>@`\"'%".contains(first);
    value.starts_with(' ')
        || value.ends_with(' ')
        || syntactic_first
        || value.contains(": ")
        || value.ends_with(':')
        || value.contains(" #")
        || value.chars().any(char::is_control)
        || (flow && value.contains([',', '[', ']', '{', '}']))
}

/// Format a float using YAML 1.2 spellings and a round-trippable finite form.
pub(crate) fn fmt_float(value: f64) -> String {
    if value.is_nan() {
        ".nan".into()
    } else if value == f64::INFINITY {
        ".inf".into()
    } else if value == f64::NEG_INFINITY {
        "-.inf".into()
    } else {
        format!("{value:?}")
    }
}

/// Double-quote a string using YAML-compatible escapes.
pub(crate) fn quote_double(value: &str) -> String {
    use core::fmt::Write as _;

    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\t' => output.push_str("\\t"),
            '\r' => output.push_str("\\r"),
            '\0' => output.push_str("\\0"),
            control if (control as u32) < 0x20 => {
                write!(output, "\\x{:02x}", control as u32)
                    .expect("writing to a String cannot fail");
            }
            other => output.push(other),
        }
    }
    output.push('"');
    output
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

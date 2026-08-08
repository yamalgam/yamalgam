# M10: yg CLI + Query Engine — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `yg <filter> [files...]` — a YAML-native query/transformation CLI whose expression language is jq (via jaq 3.0), covering spec §11 Phases 1–2.

**Architecture:** New crate `yamalgam-query` holds a metadata-carrying `YamlVal` (implements jaq's `ValT` traits), a builder that constructs it directly from parser events (compose's `Value` is lossy — tags/anchors/styles don't survive it), YAML/JSON emitters (none exist in the workspace yet), and the jaq engine wiring. The existing `crates/yamalgam` CLI crate gains the `yg` binary name and the query front-end on its librebar foundation.

**Tech Stack:** jaq-core 3.0 + jaq-std 3.0 (expression engine; default features include regex/time/math), indexmap 2, thiserror 2, yamalgam-parser (events), yamalgam-core (TagResolver, LoaderConfig), clap 4 derive + librebar (existing CLI shell).

**Spec:** `docs/spec/yg-query-language-spec-draft-01.md` — annotate code with `// r[impl <rule>]` (tracey prefix `r`, see `.config/tracey/config.styx`).

**Reference checkout:** jaq source lives at `ref/jaq` (gitignored). If absent:
`git clone --depth 1 https://github.com/01mf02/jaq ref/jaq`. Files cited below (jaq-json's `Val`, `jaq-all/src/data.rs`) are the canonical templates — read them when a task says so.

---

## Spec deviations (decided here, flagged for draft-02)

The spec is a draft; these are points where M10 deliberately deviates or defers. Each gets a note in the final docs task. **Clay: these are the calls I made — veto any before execution.**

1. **jaq crate layout** — spec says "jaq-syn and jaq-core" (1.x naming). jaq 3.0 merged lexer/parser/compiler/interpreter into `jaq-core`; `jaq-syn` is dead. Engine = jaq-core 3.0 + jaq-std 3.0. We do NOT depend on `jaq-json` (its `Val` is JSON-only) — we implement `ValT` for our own `YamlVal`.
2. **`===` / `!==` (spec §3.2) are not implementable without forking jaq's grammar**, which r[overview.jaq-foundation] forbids ("MUST NOT reimplement the jq expression grammar... not through parser modifications"). M10 ships **no strict-comparison operator**. The metadata filters (`anchor`, `tag`, `raw`) cover most use cases. Recommendation for draft-02: either a `strict_eq(f; g)` native filter or an upstream jaq grammar contribution.
3. **`explode` name collision** — jq's standard `explode` converts a string to codepoints, and Phase 1's test strategy requires jq-manual triples to pass. The alias-dereference operation (r[ext.explode]) ships as **`explode_aliases`**; `explode` keeps jq semantics. `-j` uses the internal deref. Draft-02 should rename r[ext.explode].
4. **`chomping` deferred to Phase 3** — `Event::Scalar` carries `ScalarStyle` but not `Chomp` (that lives at scanner/CST level). Phase 1's filter list (r[phases.core]) doesn't include `chomping`, so this is consistent with the spec's own phasing.
5. **Integers are `i64`** (not arbitrary-precision, r[datamodel.types.int] aspiration). Matches `yamalgam_core::Value`. Arithmetic overflow falls back to `f64` (classic jq behavior).
6. **Mapping `Ord` is insertion-order-sensitive**, consistent with the order-sensitive `==` (r[comparison.resolved.collection]). jq compares objects by sorted keys; we can't have order-sensitive Eq and order-insensitive Ord without breaking `sort`/`unique` invariants (Eq must agree with Ord==Equal).
7. **`tag` filter normalization** — events carry resolved tags. `tag` returns `!!x` shorthand for `tag:yaml.org,2002:x`, and the as-written form for everything else (`!include`, full URIs). r[ext.tag]'s examples mix both forms; we pick deterministic output.
8. **`# yg-schema:` document pragma (r[datamodel.schema.document-declared]) deferred** — not listed in Phases 1–2. `--schema` flag covers CLI control.
9. **Exit code 4** — spec defines 0/1/2/3/5. jq also uses 4 ("no output produced" with `-e`). We adopt 4 for that case; it conflicts with nothing in the spec.
10. **`--allow-includes` / `--allow-refs` flags deferred to M11** — they gate filters that don't exist until Phase 4 (r[phases.resolution]). Shipping dead flags helps no one. `--trusted` / `--strict` (LoaderConfig presets exist today) DO ship.
11. **`-L` library path / `--arg` / `--argjson` are out of scope** — spec §8 doesn't list them. The engine API accepts named vars so they can be added later without redesign.

---

## File structure

```
crates/yamalgam-query/
├── Cargo.toml
├── src/
│   ├── lib.rs          # crate root, re-exports
│   ├── value.rs        # YamlVal, Data, Meta, NodeStyle, YgMap + Eq/Ord/Hash/Display
│   ├── build.rs        # parser events → YamlVal (anchors, aliases, limits, tags)
│   ├── ops.rs          # Add/Sub/Mul/Div/Rem/Neg
│   ├── valt.rs         # impl jaq_core::ValT + jaq_std::ValT
│   ├── emit/
│   │   ├── mod.rs      # EmitOpts, shared scalar formatting (ints/floats)
│   │   ├── json.rs     # JSON emitter (drops metadata, stringifies keys)
│   │   └── yaml.rs     # YAML emitter (quoting engine, styles, anchors)
│   ├── engine.rs       # YgKind/YgData/RunCtx, compile_filter, run_filter
│   ├── funs.rs         # native filters (core ports + YAML extensions)
│   ├── defs.jq         # yg definitions: type/keys/keys_sorted + jaq-json def ports
│   └── error.rs        # BuildError, YgError (exit-code-aware)
└── tests/
    ├── value.rs        # YamlVal semantics (eq/ord/hash/builder)
    ├── emit.rs         # emitter behavior
    ├── engine.rs       # filter execution end-to-end (in-process)
    └── fixtures/
        ├── jq-manual.yml          # vendored jq manual (test corpus)
        └── manual-allowlist.yml   # documented divergences

crates/yamalgam/            # existing CLI crate
├── Cargo.toml              # [[bin]] name = "yg", + yamalgam-query dep
├── src/main.rs             # dispatch: query path vs subcommands
├── src/lib.rs              # Cli grows query args
├── src/query.rs            # NEW: CLI query pipeline (inputs → engine → output)
└── tests/yg.rs             # NEW: assert_cmd end-to-end tests
```

Responsibility boundaries: `yamalgam-query` knows nothing about argv/stdin/files — it takes `&str` in and pushes `YamlVal` out through callbacks. The CLI crate owns IO, flag parsing, exit codes. Emitters live in the query crate because `tojson`/`tostring` filters need them.

---

## Task 1: Scaffold yamalgam-query

**Files:**
- Create: `crates/yamalgam-query/Cargo.toml`
- Create: `crates/yamalgam-query/src/lib.rs`
- Create: `crates/yamalgam-query/src/error.rs`

- [ ] **Step 1: Cargo.toml**

```toml
[package]
name = "yamalgam-query"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "YAML-native jq-style query engine for yamalgam (powers the yg CLI)"
publish = false

[dependencies]
indexmap = "2"
jaq-core = "3.0"
jaq-std = "3.0"
thiserror = "2.0"
yamalgam-core = { path = "../yamalgam-core" }
yamalgam-parser = { path = "../yamalgam-parser" }
yamalgam-scanner = { path = "../yamalgam-scanner" }

[dev-dependencies]
pretty_assertions = "1"
serde = { version = "1", features = ["derive"] }
yamalgam-serde = { path = "../yamalgam-serde" }

[lints]
workspace = true
```

(`serde`/`yamalgam-serde` dev-deps are for the manual.yml harness in Task 19.)

- [ ] **Step 2: lib.rs skeleton**

```rust
//! YAML-native jq-style query engine.
//!
//! Implements the yg query language (`docs/spec/yg-query-language-spec-draft-01.md`)
//! on top of jaq 3.0. The value type [`YamlVal`] carries YAML metadata
//! (tag, anchor, style, raw scalar text) that survives filter pipelines.

pub mod build;
mod emit;
pub mod engine;
mod error;
mod funs;
mod ops;
mod valt;
pub mod value;

pub use build::{parse_str, parse_str_with_config};
pub use emit::{EmitFormat, EmitOpts};
pub use error::{BuildError, YgError};
pub use value::{Meta, NodeStyle, YamlVal};
```

Modules don't exist yet — create empty `value.rs`, `build.rs`, `ops.rs`, `valt.rs`, `engine.rs`, `funs.rs`, `error.rs`, `emit/mod.rs`, `emit/json.rs`, `emit/yaml.rs` with `//! module doc` lines so the crate compiles as each task fills them in. Comment out the `pub use` lines until their items exist (uncomment per-task).

- [ ] **Step 3: error.rs initial shape**

```rust
//! Error types for the query engine.

use yamalgam_core::Span;

/// Error building a [`crate::YamlVal`] tree from parser events.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// Underlying YAML parse error.
    #[error(transparent)]
    Parse(#[from] yamalgam_parser::ParseError),
    /// Alias references an anchor that has not been defined.
    #[error("unknown alias *{name} (line {}, column {})", .span.start.line + 1, .span.start.column + 1)]
    UnknownAlias {
        /// The alias name.
        name: String,
        /// Location of the alias.
        span: Span,
    },
    /// A `LoaderConfig` resource limit was exceeded.
    #[error("{0}")]
    Limit(String),
}
```

Check `yamalgam_core::Span`'s field shape before relying on `.start.line` (the workspace convention renders 1-indexed "(line L, column C)"; mirror how `ComposeError` does it in `crates/yamalgam-compose/src/lib.rs` and reuse its exact formatting helpers if they're public).

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p yamalgam-query`
Expected: clean (warnings about unused modules are fine at this stage; silence with `#![allow(dead_code)]` ONLY if clippy blocks — remove it by Task 13).

- [ ] **Step 5: Commit**

```bash
git add crates/yamalgam-query
git commit -m "feat(query): scaffold yamalgam-query crate"
```

---

## Task 2: YamlVal core type

**Files:**
- Modify: `crates/yamalgam-query/src/value.rs`
- Test: `crates/yamalgam-query/tests/value.rs`

The value type is the heart of the milestone. Design rules (from spec §2–3):
- Resolved data and presentation metadata are SEPARATE: `==`, `sort`, map-key lookup see only data.
- Metadata is `Option<Rc<Meta>>` — `None` for constructed values, cheap to clone, structural sharing via `Rc` everywhere (jaq clones values constantly; jaq-json uses the same trick).

- [ ] **Step 1: Write failing tests**

```rust
// tests/value.rs
use yamalgam_query::value::{Data, Meta, NodeStyle, YamlVal};
use yamalgam_scanner::ScalarStyle;

#[test]
fn kind_names_follow_yaml_terms() {
    // r[diverge.type]
    assert_eq!(YamlVal::null().type_name(), "null");
    assert_eq!(YamlVal::from(true).type_name(), "boolean");
    assert_eq!(YamlVal::int(3).type_name(), "integer");
    assert_eq!(YamlVal::float(3.5).type_name(), "float");
    assert_eq!(YamlVal::str("hi").type_name(), "string");
    assert_eq!(YamlVal::seq(vec![]).type_name(), "sequence");
    assert_eq!(YamlVal::map(vec![]).type_name(), "mapping");
}

#[test]
fn kind_is_structure_not_resolution() {
    // r[ext.kind]
    assert_eq!(YamlVal::int(3).kind_name(), "scalar");
    assert_eq!(YamlVal::seq(vec![]).kind_name(), "sequence");
    assert_eq!(YamlVal::map(vec![]).kind_name(), "mapping");
    let aliased = YamlVal::int(3).with_meta(Meta {
        alias_of: Some("a".into()),
        ..Meta::default()
    });
    assert_eq!(aliased.kind_name(), "alias");
}

#[test]
fn meta_accessors() {
    let v = YamlVal::str("x").with_meta(Meta {
        tag: Some("tag:yaml.org,2002:str".into()),
        anchor: Some("base".into()),
        style: Some(NodeStyle::Scalar(ScalarStyle::DoubleQuoted)),
        raw: Some("x".into()),
        alias_of: None,
    });
    assert_eq!(v.meta().and_then(|m| m.anchor.as_deref()), Some("base"));
    assert_eq!(v.tag_shorthand().as_deref(), Some("!!str")); // r[ext.tag] deviation #7
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo nextest run -p yamalgam-query`
Expected: FAIL — types don't exist.

- [ ] **Step 3: Implement value.rs**

```rust
//! The metadata-carrying YAML value type.

use std::rc::Rc;

use indexmap::IndexMap;
use yamalgam_parser::CollectionStyle;
use yamalgam_scanner::ScalarStyle;

/// Presentation style of a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeStyle {
    /// Scalar presentation (plain, quoted, literal, folded).
    Scalar(ScalarStyle),
    /// Collection presentation (block or flow).
    Collection(CollectionStyle),
}

/// YAML node metadata. Never participates in equality, ordering, or hashing.
// r[datamodel.metadata]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Meta {
    /// Explicit tag, as resolved by the parser (e.g. `tag:yaml.org,2002:int`, `!include`).
    pub tag: Option<Rc<str>>,
    /// Anchor name (without `&`).
    pub anchor: Option<Rc<str>>,
    /// If this node came from an alias, the referenced anchor name.
    pub alias_of: Option<Rc<str>>,
    /// Presentation style from the source document.
    pub style: Option<NodeStyle>,
    /// Raw scalar text before schema resolution (input scalars only).
    pub raw: Option<Rc<str>>,
}

/// Order-preserving YAML mapping with order-SENSITIVE equality.
// r[comparison.resolved.collection]
#[derive(Clone, Debug, Default)]
pub struct YgMap(pub(crate) IndexMap<YamlVal, YamlVal>);

/// Resolved value data. Equality/ordering live here.
#[derive(Clone, Debug, Default)]
pub enum Data {
    /// Null. r[datamodel.types.null]
    #[default]
    Null,
    /// Boolean. r[datamodel.types.bool]
    Bool(bool),
    /// 64-bit integer (deviation #5: not arbitrary precision yet).
    Int(i64),
    /// IEEE 754 double. r[datamodel.types.float]
    Float(f64),
    /// Unicode string. r[datamodel.types.str]
    Str(Rc<str>),
    /// Sequence. r[datamodel.types.seq]
    Seq(Rc<Vec<YamlVal>>),
    /// Mapping (any key type). r[datamodel.types.mapping]
    Map(Rc<YgMap>),
}

/// A YAML value as processed by yg filters: resolved data + presentation metadata.
// r[datamodel]
#[derive(Clone, Debug, Default)]
pub struct YamlVal {
    pub(crate) data: Data,
    pub(crate) meta: Option<Rc<Meta>>,
}

impl YamlVal {
    /// Null value with no metadata.
    #[must_use]
    pub const fn null() -> Self {
        Self { data: Data::Null, meta: None }
    }

    /// Integer value.
    #[must_use]
    pub const fn int(i: i64) -> Self {
        Self { data: Data::Int(i), meta: None }
    }

    /// Float value.
    #[must_use]
    pub const fn float(f: f64) -> Self {
        Self { data: Data::Float(f), meta: None }
    }

    /// String value.
    #[must_use]
    pub fn str(s: impl Into<Rc<str>>) -> Self {
        Self { data: Data::Str(s.into()), meta: None }
    }

    /// Sequence value.
    #[must_use]
    pub fn seq(items: Vec<Self>) -> Self {
        Self { data: Data::Seq(Rc::new(items)), meta: None }
    }

    /// Mapping value from key/value pairs (insertion order preserved).
    #[must_use]
    pub fn map(pairs: Vec<(Self, Self)>) -> Self {
        Self {
            data: Data::Map(Rc::new(YgMap(pairs.into_iter().collect()))),
            meta: None,
        }
    }

    /// Attach metadata, replacing any existing metadata.
    #[must_use]
    pub fn with_meta(mut self, meta: Meta) -> Self {
        self.meta = Some(Rc::new(meta));
        self
    }

    /// The node's metadata, if any.
    #[must_use]
    pub fn meta(&self) -> Option<&Meta> {
        self.meta.as_deref()
    }

    /// Mutable metadata access, creating empty metadata on demand.
    pub(crate) fn meta_mut(&mut self) -> &mut Meta {
        Rc::make_mut(self.meta.get_or_insert_with(|| Rc::new(Meta::default())))
    }

    /// YAML data-model type name. r[diverge.type]
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self.data {
            Data::Null => "null",
            Data::Bool(_) => "boolean",
            Data::Int(_) => "integer",
            Data::Float(_) => "float",
            Data::Str(_) => "string",
            Data::Seq(_) => "sequence",
            Data::Map(_) => "mapping",
        }
    }

    /// YAML node kind without resolution. r[ext.kind]
    #[must_use]
    pub fn kind_name(&self) -> &'static str {
        if self.meta().is_some_and(|m| m.alias_of.is_some()) {
            return "alias";
        }
        match self.data {
            Data::Seq(_) => "sequence",
            Data::Map(_) => "mapping",
            _ => "scalar",
        }
    }

    /// Tag in `!!x` shorthand for the core namespace, as-written otherwise.
    /// Deviation #7. r[ext.tag]
    #[must_use]
    pub fn tag_shorthand(&self) -> Option<String> {
        let tag = self.meta().and_then(|m| m.tag.as_deref())?;
        Some(tag.strip_prefix("tag:yaml.org,2002:").map_or_else(
            || tag.to_owned(),
            |suffix| format!("!!{suffix}"),
        ))
    }
}

impl From<bool> for YamlVal {
    fn from(b: bool) -> Self {
        Self { data: Data::Bool(b), meta: None }
    }
}

impl From<String> for YamlVal {
    fn from(s: String) -> Self {
        Self::str(s)
    }
}

impl From<isize> for YamlVal {
    fn from(i: isize) -> Self {
        Self::int(i as i64)
    }
}

impl From<usize> for YamlVal {
    fn from(i: usize) -> Self {
        i64::try_from(i).map_or_else(|_| Self::float(i as f64), Self::int)
    }
}

impl From<f64> for YamlVal {
    fn from(f: f64) -> Self {
        Self::float(f)
    }
}

impl FromIterator<Self> for YamlVal {
    fn from_iter<T: IntoIterator<Item = Self>>(iter: T) -> Self {
        Self::seq(iter.into_iter().collect())
    }
}
```

Note `Data` and `YgMap` are `pub` (in a `pub mod value`) because tests and sibling modules pattern-match them; keep `YamlVal.data`/`.meta` fields `pub(crate)`.

- [ ] **Step 4: Run tests**

Run: `cargo nextest run -p yamalgam-query`
Expected: PASS (the `FromIterator` collect for `YgMap` in `Self::map` needs `impl FromIterator<(YamlVal, YamlVal)> for YgMap` — add it; `IndexMap::from_iter` requires `Hash + Eq` on keys, which lands in Task 3. Until then make `YgMap::from_iter` push pairs via `insert`… simplest: implement Task 3's Eq/Hash FIRST if the compiler blocks you, or use a `Vec`-backed temporary. If blocked, fold Step 3 of Task 3 in here — the commit boundary below still stands.)

- [ ] **Step 5: Commit**

```bash
git add crates/yamalgam-query
git commit -m "feat(query): YamlVal core type with metadata"
```

---

## Task 3: Equality, ordering, hashing

**Files:**
- Modify: `crates/yamalgam-query/src/value.rs`
- Test: `crates/yamalgam-query/tests/value.rs`

These three impls ARE the spec's comparison semantics (§3.1, §3.3). jaq calls them for `==`, `<`, `sort`, `unique`, `group_by`, and map-key lookup. Invariants:
- metadata-blind (quoted `"1"` string ≠ plain `1` int is a DATA difference, not meta),
- `1 == 1.0` (r[comparison.resolved.numeric]) → Int/Float compare numerically → Hash must agree,
- mappings order-sensitive (r[comparison.resolved.collection]),
- cross-type order: null < false < true < numbers < strings < sequences < mappings (r[comparison.ordering.cross-type]) — jaq-std's `def type` detects types via this ordering, so it must be exact,
- total order (`Ord`) with NaN pinned below all other numbers, NaN cmp NaN == Equal.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn resolved_eq_is_meta_blind_and_numeric() {
    // r[comparison.resolved]
    assert_eq!(YamlVal::int(1), YamlVal::float(1.0)); // r[comparison.resolved.numeric]
    assert_ne!(YamlVal::int(1), YamlVal::str("1"));   // r[comparison.resolved.type-coercion]
    assert_ne!(YamlVal::null(), YamlVal::from(false)); // r[comparison.resolved.null]
    let plain = YamlVal::str("hello");
    let quoted = YamlVal::str("hello").with_meta(Meta {
        style: Some(NodeStyle::Scalar(ScalarStyle::DoubleQuoted)),
        ..Meta::default()
    });
    assert_eq!(plain, quoted); // r[comparison.resolved.string]
}

#[test]
fn mapping_eq_is_order_sensitive() {
    // r[comparison.resolved.collection]
    let ab = YamlVal::map(vec![
        (YamlVal::str("a"), YamlVal::int(1)),
        (YamlVal::str("b"), YamlVal::int(2)),
    ]);
    let ba = YamlVal::map(vec![
        (YamlVal::str("b"), YamlVal::int(2)),
        (YamlVal::str("a"), YamlVal::int(1)),
    ]);
    assert_ne!(ab, ba);
    assert_eq!(ab, ab.clone());
}

#[test]
fn cross_type_ordering() {
    // r[comparison.ordering.cross-type]
    let order = [
        YamlVal::null(),
        YamlVal::from(false),
        YamlVal::from(true),
        YamlVal::int(1),
        YamlVal::str("a"),
        YamlVal::seq(vec![]),
        YamlVal::map(vec![]),
    ];
    for w in order.windows(2) {
        assert!(w[0] < w[1], "{} should sort before {}", w[0].type_name(), w[1].type_name());
    }
}

#[test]
fn int_float_hash_consistency() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let h = |v: &YamlVal| {
        let mut s = DefaultHasher::new();
        v.hash(&mut s);
        s.finish()
    };
    assert_eq!(h(&YamlVal::int(1)), h(&YamlVal::float(1.0)));
    assert_eq!(h(&YamlVal::float(0.0)), h(&YamlVal::float(-0.0)));
}

#[test]
fn nan_sorts_first_among_numbers() {
    let mut v = vec![YamlVal::float(1.0), YamlVal::float(f64::NAN), YamlVal::int(-5)];
    v.sort();
    assert!(matches!(v[0].data_for_tests(), d if format!("{d:?}").contains("NaN")));
}
```

(Add `#[doc(hidden)] pub fn data_for_tests(&self) -> &Data` or just make the test compare via `as_f64().unwrap().is_nan()` once Task 8 lands — executor's choice, keep it simple.)

- [ ] **Step 2: Run to verify failure**

Run: `cargo nextest run -p yamalgam-query`
Expected: FAIL — no PartialEq/Ord/Hash impls.

- [ ] **Step 3: Implement**

```rust
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

// r[comparison.resolved] — equality on resolved data, metadata-blind.
impl PartialEq for YamlVal {
    fn eq(&self, other: &Self) -> bool {
        match (&self.data, &other.data) {
            (Data::Null, Data::Null) => true,
            (Data::Bool(a), Data::Bool(b)) => a == b,
            (Data::Int(a), Data::Int(b)) => a == b,
            (Data::Float(a), Data::Float(b)) => a == b,
            (Data::Int(a), Data::Float(b)) | (Data::Float(b), Data::Int(a)) => {
                (*a as f64) == *b // r[comparison.resolved.numeric]
            }
            (Data::Str(a), Data::Str(b)) => a == b,
            (Data::Seq(a), Data::Seq(b)) => a == b,
            (Data::Map(a), Data::Map(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for YamlVal {}

// Order-sensitive mapping equality. r[comparison.resolved.collection]
impl PartialEq for YgMap {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && self.0.iter().zip(other.0.iter()).all(|(a, b)| a == b)
    }
}

impl Eq for YgMap {}

impl PartialOrd for YamlVal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// r[comparison.ordering] — total order, jq cross-type convention.
impl Ord for YamlVal {
    fn cmp(&self, other: &Self) -> Ordering {
        use Data::{Bool, Float, Int, Map, Null, Seq, Str};
        fn num_cmp(a: f64, b: f64) -> Ordering {
            // NaN below everything, NaN == NaN (total order for sort()).
            match (a.is_nan(), b.is_nan()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                (false, false) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
            }
        }
        fn rank(d: &Data) -> u8 {
            match d {
                Null => 0,
                Bool(_) => 1,
                Int(_) | Float(_) => 2,
                Str(_) => 3,
                Seq(_) => 4,
                Map(_) => 5,
            }
        }
        match (&self.data, &other.data) {
            (Null, Null) => Ordering::Equal,
            (Bool(a), Bool(b)) => a.cmp(b),
            (Int(a), Int(b)) => a.cmp(b),
            (Float(a), Float(b)) => num_cmp(*a, *b),
            (Int(a), Float(b)) => num_cmp(*a as f64, *b),
            (Float(a), Int(b)) => num_cmp(*a, *b as f64),
            (Str(a), Str(b)) => a.cmp(b), // r[comparison.ordering.string]
            (Seq(a), Seq(b)) => a.cmp(b),
            // Deviation #6: entry-order-sensitive, consistent with Eq.
            (Map(a), Map(b)) => {
                let entries = |m: &YgMap| m.0.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>();
                entries(a).cmp(&entries(b))
            }
            (a, b) => rank(a).cmp(&rank(b)),
        }
    }
}

impl Hash for YamlVal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match &self.data {
            Data::Null => state.write_u8(0),
            Data::Bool(b) => state.write_u8(if *b { 1 } else { 2 }),
            // Int and integral Float hash identically (Eq says they're equal).
            Data::Int(i) => {
                state.write_u8(3);
                i.hash(state);
            }
            Data::Float(f) => {
                // -0.0 normalizes to 0.0; integral floats hash as their Int form.
                let f = if *f == 0.0 { 0.0 } else { *f };
                if f.fract() == 0.0 && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
                    state.write_u8(3);
                    (f as i64).hash(state);
                } else {
                    state.write_u8(4);
                    f.to_bits().hash(state);
                }
            }
            Data::Str(s) => {
                state.write_u8(5);
                s.hash(state);
            }
            Data::Seq(items) => {
                state.write_u8(6);
                items.iter().for_each(|v| v.hash(state));
            }
            Data::Map(m) => {
                state.write_u8(7);
                m.0.iter().for_each(|kv| kv.hash(state));
            }
        }
    }
}

impl FromIterator<(YamlVal, YamlVal)> for YgMap {
    fn from_iter<T: IntoIterator<Item = (YamlVal, YamlVal)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}
```

`Seq.cmp` works because `Vec<YamlVal>: Ord` once `YamlVal: Ord`. The Map branch's `Vec` collect is O(n) per comparison — fine for M10 (jaq-json also collects for object cmp).

- [ ] **Step 4: Run tests**

Run: `cargo nextest run -p yamalgam-query`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/yamalgam-query
git commit -m "feat(query): resolved-value equality, jq cross-type ordering, hashing"
```

---

## Task 4: Builder — parser events → YamlVal

**Files:**
- Modify: `crates/yamalgam-query/src/build.rs`
- Modify: `crates/yamalgam-query/src/error.rs` (if span helpers needed)
- Test: `crates/yamalgam-query/tests/value.rs` (append)

Read `crates/yamalgam-compose/src/lib.rs` first — the Composer is the structural template (event loop, anchor map, limit checks). Differences: we KEEP tag/anchor/style/raw in `Meta`, we do NOT expand merge keys (r[ext.merge.preserve]), and alias splices get `alias_of` markers (r[comparison.strict.anchor-identity] groundwork).

- [ ] **Step 1: Write failing tests**

```rust
use yamalgam_query::{parse_str, parse_str_with_config};

#[test]
fn builder_preserves_metadata() {
    let docs = parse_str("base: &b !custom 'hi'\n").unwrap();
    let map = &docs[0];
    let v = map.get_str("base").unwrap();
    let m = v.meta().unwrap();
    assert_eq!(m.anchor.as_deref(), Some("b"));
    assert_eq!(m.tag.as_deref(), Some("!custom"));
    assert!(matches!(m.style, Some(NodeStyle::Scalar(ScalarStyle::SingleQuoted))));
    assert_eq!(m.raw.as_deref(), Some("hi"));
}

#[test]
fn plain_scalars_resolve_via_core_schema() {
    // r[datamodel.schema.default]
    let docs = parse_str("a: 1\nb: yes\nc: 1.5\nd: null\n").unwrap();
    let m = &docs[0];
    assert_eq!(m.get_str("a"), Some(&YamlVal::int(1)));
    assert_eq!(m.get_str("b"), Some(&YamlVal::str("yes"))); // r[datamodel.types.bool]
    assert_eq!(m.get_str("c"), Some(&YamlVal::float(1.5)));
    assert_eq!(m.get_str("d"), Some(&YamlVal::null()));
}

#[test]
fn aliases_splice_with_identity_marker() {
    let docs = parse_str("a: &x [1, 2]\nb: *x\n").unwrap();
    let m = &docs[0];
    assert_eq!(m.get_str("a"), m.get_str("b")); // same resolved value
    let b = m.get_str("b").unwrap();
    assert_eq!(b.meta().unwrap().alias_of.as_deref(), Some("x"));
    assert!(b.meta().unwrap().anchor.is_none()); // copy must not re-anchor
}

#[test]
fn merge_keys_are_preserved_not_expanded() {
    // r[ext.merge.preserve]
    let docs = parse_str("base: &b {x: 1}\nchild:\n  <<: *b\n  y: 2\n").unwrap();
    let child = docs[0].get_str("child").unwrap();
    assert!(child.get_str("x").is_none(), "merge must not auto-expand");
    let merge_key = child.keys().next().unwrap();
    assert_eq!(merge_key.tag_shorthand().as_deref(), Some("!!merge"));
}

#[test]
fn unknown_alias_is_an_error() {
    assert!(parse_str("a: *nope\n").is_err());
}

#[test]
fn multi_document_streams() {
    let docs = parse_str("---\n1\n---\n2\n").unwrap();
    assert_eq!(docs, vec![YamlVal::int(1), YamlVal::int(2)]);
}

#[test]
fn explicit_core_tags_force_resolution() {
    let docs = parse_str("a: !!str 5\nb: !!int '5'\n").unwrap();
    assert_eq!(docs[0].get_str("a"), Some(&YamlVal::str("5")));
    assert_eq!(docs[0].get_str("b"), Some(&YamlVal::int(5)));
}
```

Add the helpers these tests use to `value.rs` (public, documented):

```rust
impl YamlVal {
    /// Mapping lookup by string key (resolved comparison).
    #[must_use]
    pub fn get_str(&self, key: &str) -> Option<&Self> {
        match &self.data {
            Data::Map(m) => m.0.get(&Self::str(key)),
            _ => None,
        }
    }

    /// Mapping keys in insertion order (empty iterator for non-mappings).
    pub fn keys(&self) -> Box<dyn Iterator<Item = &Self> + '_> {
        match &self.data {
            Data::Map(m) => Box::new(m.0.keys()),
            _ => Box::new(core::iter::empty()),
        }
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo nextest run -p yamalgam-query`
Expected: FAIL — `parse_str` unimplemented.

- [ ] **Step 3: Implement build.rs**

```rust
//! Build [`YamlVal`] trees from the parser event stream.
//!
//! Parallel consumer to yamalgam-compose's Composer, but metadata-preserving:
//! tags, anchors, styles, and raw scalar text survive into [`Meta`].

use std::collections::HashMap;
use std::rc::Rc;

use yamalgam_core::{LoaderConfig, TagResolver, Value};
use yamalgam_parser::{CollectionStyle, Event, Parser};
use yamalgam_scanner::ScalarStyle;

use crate::error::BuildError;
use crate::value::{Data, Meta, NodeStyle, YamlVal, YgMap};

/// Parse all documents with the default (`moderate`) config.
// r[cli.usage]
pub fn parse_str(input: &str) -> Result<Vec<YamlVal>, BuildError> {
    parse_str_with_config(input, &LoaderConfig::moderate())
}

/// Parse all documents under an explicit [`LoaderConfig`].
pub fn parse_str_with_config(
    input: &str,
    config: &LoaderConfig,
) -> Result<Vec<YamlVal>, BuildError> {
    config
        .limits
        .check_input_size(input.len())
        .map_err(BuildError::Limit)?;
    let mut events = Parser::new(input).filter(|e| {
        e.as_ref().map_or(true, |ev| !ev.is_structural())
    });
    let mut builder = Builder::new(config);
    let mut docs = Vec::new();
    while let Some(event) = events.next() {
        match event? {
            Event::StreamStart | Event::StreamEnd => {}
            Event::VersionDirective { .. } | Event::TagDirective { .. } => {}
            Event::DocumentStart { .. } => {
                builder.anchors.clear();
                builder.alias_expansions = 0;
            }
            Event::DocumentEnd { .. } => {}
            other => docs.push(builder.node(other, &mut events, 0)?),
        }
    }
    Ok(docs)
}

struct Builder<'cfg> {
    config: &'cfg LoaderConfig,
    anchors: HashMap<String, YamlVal>,
    alias_expansions: usize,
}

impl<'cfg> Builder<'cfg> {
    fn new(config: &'cfg LoaderConfig) -> Self {
        Self { config, anchors: HashMap::new(), alias_expansions: 0 }
    }

    /// Build one node from `first` plus following events.
    fn node<'i>(
        &mut self,
        first: Event<'i>,
        events: &mut impl Iterator<Item = Result<Event<'i>, yamalgam_parser::ParseError>>,
        depth: usize,
    ) -> Result<YamlVal, BuildError> {
        self.config.limits.check_depth(depth).map_err(BuildError::Limit)?;
        match first {
            // r[datamodel.metadata.tag] r[datamodel.metadata.anchor] r[datamodel.metadata.style]
            Event::Scalar { anchor, tag, value, style, .. } => {
                let resolved = self.resolve_scalar(&value, style, tag.as_deref());
                let node = self.finish(resolved, anchor, tag, NodeStyle::Scalar(style), Some(&value));
                Ok(node)
            }
            Event::SequenceStart { anchor, tag, style, .. } => {
                let mut items = Vec::new();
                loop {
                    let ev = next_event(events)?;
                    if matches!(ev, Event::SequenceEnd { .. }) {
                        break;
                    }
                    items.push(self.node(ev, events, depth + 1)?);
                }
                let data = Data::Seq(Rc::new(items));
                Ok(self.finish(data, anchor, tag, NodeStyle::Collection(style), None))
            }
            Event::MappingStart { anchor, tag, style, .. } => {
                let mut map = YgMap::default();
                loop {
                    let ev = next_event(events)?;
                    if matches!(ev, Event::MappingEnd { .. }) {
                        break;
                    }
                    let mut key = self.node(ev, events, depth + 1)?;
                    self.tag_merge_key(&mut key);
                    let val_ev = next_event(events)?;
                    let val = self.node(val_ev, events, depth + 1)?;
                    map.0.insert(key, val);
                }
                let data = Data::Map(Rc::new(map));
                Ok(self.finish(data, anchor, tag, NodeStyle::Collection(style), None))
            }
            // r[comparison.strict.anchor-identity] groundwork: alias splice + marker
            Event::Alias { name, span } => {
                self.alias_expansions += 1;
                self.config
                    .limits
                    .check_alias_expansions(self.alias_expansions)
                    .map_err(BuildError::Limit)?;
                let target = self.anchors.get(name.as_ref()).cloned().ok_or_else(|| {
                    BuildError::UnknownAlias { name: name.to_string(), span }
                })?;
                let mut copy = target;
                let meta = copy.meta_mut();
                meta.anchor = None;
                meta.alias_of = Some(Rc::from(name.as_ref()));
                Ok(copy)
            }
            unexpected => unreachable!("non-node event reached Builder::node: {unexpected:?}"),
        }
    }

    /// Apply schema resolution to a scalar. r[datamodel.schema]
    fn resolve_scalar(&self, text: &str, style: ScalarStyle, tag: Option<&str>) -> Data {
        if let Some(tag) = tag {
            return resolve_tagged(text, tag);
        }
        if style != ScalarStyle::Plain {
            return Data::Str(Rc::from(text)); // quoted/block scalars are strings
        }
        core_value_to_data(self.config.tag_resolution.resolve_scalar(text))
    }

    /// Attach metadata + register anchor. Anchor registration AFTER the node
    /// completes (no self-referential aliases — matches Composer).
    fn finish(
        &mut self,
        data: Data,
        anchor: Option<std::borrow::Cow<'_, str>>,
        tag: Option<std::borrow::Cow<'_, str>>,
        style: NodeStyle,
        raw: Option<&str>,
    ) -> YamlVal {
        let meta = Meta {
            tag: tag.as_deref().map(Rc::from),
            anchor: anchor.as_deref().map(Rc::from),
            alias_of: None,
            style: Some(style),
            raw: raw.map(Rc::from),
        };
        let mut node = YamlVal { data, meta: Some(Rc::new(meta)) };
        if let Some(name) = anchor {
            // limit: anchor count
            let _ = self.config.limits.check_anchor_count(self.anchors.len() + 1)
                .map_err(BuildError::Limit); // propagate — see step note
            self.anchors.insert(name.into_owned(), node.clone());
        }
        node
    }

    /// Tag plain `<<` keys with !!merge. r[ext.merge.preserve]
    fn tag_merge_key(&self, key: &mut YamlVal) {
        let is_merge = matches!(&key.data, Data::Str(s) if &**s == "<<")
            && key.meta().is_none_or(|m| {
                m.tag.is_none() && matches!(m.style, Some(NodeStyle::Scalar(ScalarStyle::Plain)))
            });
        if is_merge {
            key.meta_mut().tag = Some(Rc::from("tag:yaml.org,2002:merge"));
        }
    }
}

fn next_event<'i>(
    events: &mut impl Iterator<Item = Result<Event<'i>, yamalgam_parser::ParseError>>,
) -> Result<Event<'i>, BuildError> {
    events
        .next()
        .transpose()?
        .ok_or_else(|| BuildError::Limit("unexpected end of event stream".into()))
}

/// Convert a core-schema resolved Value scalar into Data.
fn core_value_to_data(v: Value) -> Data {
    match v {
        Value::Null => Data::Null,
        Value::Bool(b) => Data::Bool(b),
        Value::Integer(i) => Data::Int(i),
        Value::Float(f) => Data::Float(f),
        Value::String(s) => Data::Str(Rc::from(s.as_str())),
        // resolve_scalar never returns collections
        Value::Sequence(_) | Value::Mapping(_) => unreachable!("scalar resolver returned collection"),
    }
}

/// Explicit-tag resolution. Core-namespace tags force typing; mismatched
/// text falls back to string (lenient — documented in Task 23).
fn resolve_tagged(text: &str, tag: &str) -> Data {
    use yamalgam_core::TagResolution;
    let suffix = tag.strip_prefix("tag:yaml.org,2002:");
    match suffix {
        Some("str") | None => Data::Str(Rc::from(text)),
        Some("null") => Data::Null,
        Some("bool" | "int" | "float") => {
            match core_value_to_data(TagResolution::Yaml12.resolve_scalar(text)) {
                d @ (Data::Bool(_) | Data::Int(_) | Data::Float(_)) => d,
                _ => Data::Str(Rc::from(text)),
            }
        }
        Some(_) => Data::Str(Rc::from(text)), // !!binary, !!timestamp, custom: keep text
    }
}
```

Adjustments the executor WILL need (verify against real signatures, don't trust this sketch blindly):
- `Parser::new(input)` — check the actual constructor in `crates/yamalgam-parser/src/parser.rs` (it may take `LoaderConfig` or want `Parser::new(input).with_config(...)`; the Composer shows the canonical instantiation, including how limits are threaded to the SCANNER level).
- The `finish` anchor-limit check swallows the error in the sketch (`let _`) — restructure so `finish` returns `Result<YamlVal, BuildError>` and propagate properly. The `?` belongs there; do not ship a swallowed error.
- The tagged-`bool|int|float` match arm above mis-types `!!bool yes` under 1.2 rules (resolves "yes" to string → falls back to Str). That is CORRECT per core schema (only true/false are bools) — add a test asserting `!!bool yes` stays a string, documenting the choice.
- Verify the exact `tag` form events carry: write a quick assertion test `parse_str("a: !!int 5")` and inspect `m.tag` — if the parser emits `!!int` unresolved instead of `tag:yaml.org,2002:int`, normalize in `resolve_tagged`/`tag_shorthand` accordingly (handle BOTH forms defensively).

- [ ] **Step 4: Run tests**

Run: `cargo nextest run -p yamalgam-query`
Expected: PASS, including the metadata, merge-preserve, alias-marker, and multi-doc tests.

- [ ] **Step 5: Commit**

```bash
git add crates/yamalgam-query
git commit -m "feat(query): metadata-preserving event-to-YamlVal builder"
```

---

## Task 5: Display and string conversion

**Files:**
- Modify: `crates/yamalgam-query/src/value.rs`
- Test: `crates/yamalgam-query/tests/value.rs` (append)

jaq requires `Display` (error messages, `"\(v)"` interpolation via `into_string`). Render compact single-line flow YAML. Strings render with quotes in Display (collection context) — but `into_string` on a string yields the bare content (jq parity).

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn display_is_compact_flow() {
    let v = parse_str("a: [1, two]\nb: null\n").unwrap().remove(0);
    assert_eq!(v.to_string(), r#"{a: [1, two], b: null}"#);
    assert_eq!(YamlVal::str("needs: quoting").to_string(), r#""needs: quoting""#);
    assert_eq!(YamlVal::float(1.0).to_string(), "1.0");
    assert_eq!(YamlVal::float(f64::INFINITY).to_string(), ".inf");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo nextest run -p yamalgam-query`
Expected: FAIL.

- [ ] **Step 3: Implement Display**

The quoting decision ("does this string need quotes?") is shared with the YAML emitter (Task 10). Put it in `value.rs` now, move/`pub(crate)`-share when Task 10 needs it:

```rust
/// Would this string re-resolve as a non-string (or break YAML syntax) if
/// emitted plain? Conservative: quote on doubt. r[output.yaml.quoting]
pub(crate) fn needs_quotes(s: &str) -> bool {
    use yamalgam_core::{TagResolution, TagResolver as _, Value};
    if s.is_empty() {
        return true;
    }
    // Re-resolution check: plain emission must round-trip as a string.
    if !matches!(TagResolution::Yaml12.resolve_scalar(s), Value::String(_)) {
        return true;
    }
    let first = s.chars().next().unwrap_or(' ');
    let syntactic_first = "!&*-?:,[]{}#|>@`\"'%".contains(first);
    s.starts_with(' ')
        || s.ends_with(' ')
        || syntactic_first
        || s.contains(": ")
        || s.ends_with(':')
        || s.contains(" #")
        || s.contains('\n')
        || s.contains('\t')
}

impl core::fmt::Display for YamlVal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self.data {
            Data::Null => write!(f, "null"),
            Data::Bool(b) => write!(f, "{b}"),
            Data::Int(i) => write!(f, "{i}"),
            Data::Float(x) => write!(f, "{}", fmt_float(*x)),
            Data::Str(s) if needs_quotes(s) => write!(f, "{}", quote_double(s)),
            Data::Str(s) => write!(f, "{s}"),
            Data::Seq(items) => {
                write!(f, "[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            Data::Map(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.0.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            }
        }
    }
}

/// YAML 1.2 float formatting: shortest round-trip repr, always float-typed
/// on re-parse (Rust's {:?} keeps ".0"), specials as .inf/-.inf/.nan.
pub(crate) fn fmt_float(f: f64) -> String {
    if f.is_nan() {
        ".nan".into()
    } else if f.is_infinite() {
        if f > 0.0 { ".inf".into() } else { "-.inf".into() }
    } else {
        format!("{f:?}")
    }
}

/// Double-quote with YAML escapes (\\, \", \n, \t, \r, \0, \x.., \u....).
pub(crate) fn quote_double(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
```

- [ ] **Step 4: Run tests, then commit**

Run: `cargo nextest run -p yamalgam-query`
Expected: PASS.

```bash
git add crates/yamalgam-query
git commit -m "feat(query): compact flow Display, float formatting, quote engine"
```

---

## Task 6: Arithmetic operators

**Files:**
- Modify: `crates/yamalgam-query/src/ops.rs`
- Test: unit tests in `ops.rs` (`#[cfg(test)]`)

jaq requires `Add/Sub/Mul/Div/Rem/Neg` with `Output = ValR<Self>` (`jaq_core::ValR<YamlVal>`). Semantics = jq's (spec §5.5–5.6 says "identical to jq"): null is `+`-neutral, strings concat on `+` and split on `/`, seqs concat/subtract, maps shallow-merge on `+` and DEEP-merge on `*` (r[diverge.multiply]). Read `ref/jaq/jaq-json/src/lib.rs:562-696` — port each `impl`, replacing `Num` with our Int/Float pair and `Bytes` strings with `Rc<str>`.

Integer overflow: use `checked_*`; on overflow redo in f64 (deviation #5).

- [ ] **Step 1: Write failing unit tests** (in `ops.rs` `#[cfg(test)] mod tests`)

```rust
use jaq_core::ValT as _; // not needed yet; remove if unused
use crate::value::YamlVal;

#[test]
fn add_semantics() {
    let add = |a: YamlVal, b: YamlVal| (a + b).unwrap();
    assert_eq!(add(YamlVal::null(), YamlVal::int(2)), YamlVal::int(2));
    assert_eq!(add(YamlVal::int(1), YamlVal::float(0.5)), YamlVal::float(1.5));
    assert_eq!(add(YamlVal::str("a"), YamlVal::str("b")), YamlVal::str("ab"));
    assert_eq!(
        add(YamlVal::seq(vec![YamlVal::int(1)]), YamlVal::seq(vec![YamlVal::int(2)])),
        YamlVal::seq(vec![YamlVal::int(1), YamlVal::int(2)])
    );
    assert!((YamlVal::from(true) + YamlVal::int(1)).is_err());
}

#[test]
fn int_overflow_promotes_to_float() {
    let r = (YamlVal::int(i64::MAX) + YamlVal::int(1)).unwrap();
    assert_eq!(r, YamlVal::float(i64::MAX as f64 + 1.0));
}

#[test]
fn mul_deep_merges_mappings() {
    // r[diverge.multiply]
    let a = crate::build::parse_str("x: {y: 1, z: 2}\n").unwrap().remove(0);
    let b = crate::build::parse_str("x: {y: 9}\n").unwrap().remove(0);
    let m = (a * b).unwrap();
    assert_eq!(m.get_str("x").unwrap().get_str("y"), Some(&YamlVal::int(9)));
    assert_eq!(m.get_str("x").unwrap().get_str("z"), Some(&YamlVal::int(2)));
}

#[test]
fn div_splits_strings() {
    let r = (YamlVal::str("a,b") / YamlVal::str(",")).unwrap();
    assert_eq!(r, YamlVal::seq(vec![YamlVal::str("a"), YamlVal::str("b")]));
}
```

- [ ] **Step 2: Verify failure, implement, verify pass**

Implementation skeleton (numeric core; collection arms port jaq-json line-for-line with our types — `Rc::make_mut` for in-place when uniquely owned):

```rust
//! Arithmetic operators with jq semantics on resolved values.

use std::rc::Rc;

use jaq_core::{ops::Math, Error, ValR};

use crate::value::{Data, YamlVal, YgMap};

type R = ValR<YamlVal>;

/// Apply an integer op with float fallback on overflow. Deviation #5.
fn int_op(a: i64, b: i64, i: impl Fn(i64, i64) -> Option<i64>, f: impl Fn(f64, f64) -> f64) -> Data {
    i(a, b).map_or_else(|| Data::Float(f(a as f64, b as f64)), Data::Int)
}

fn num_pair(l: &Data, r: &Data) -> Option<(f64, f64)> {
    let as_f = |d: &Data| match d {
        Data::Int(i) => Some(*i as f64),
        Data::Float(f) => Some(*f),
        _ => None,
    };
    Some((as_f(l)?, as_f(r)?))
}

impl core::ops::Add for YamlVal {
    type Output = R;
    fn add(self, rhs: Self) -> R {
        use Data::{Float, Int, Map, Null, Seq, Str};
        let data = match (self.data, rhs.data) {
            (Null, x) | (x, Null) => x, // null is neutral
            (Int(a), Int(b)) => int_op(a, b, i64::checked_add, |x, y| x + y),
            (l, r) if num_pair(&l, &r).is_some() => {
                let (x, y) = num_pair(&l, &r).unwrap();
                Float(x + y)
            }
            (Str(a), Str(b)) => Str(Rc::from(format!("{a}{b}"))),
            (Seq(mut a), Seq(b)) => {
                Rc::make_mut(&mut a).extend(b.iter().cloned());
                Seq(a)
            }
            (Map(mut a), Map(b)) => {
                let m = Rc::make_mut(&mut a);
                b.0.iter().for_each(|(k, v)| {
                    m.0.insert(k.clone(), v.clone());
                });
                Map(a)
            }
            (l, r) => {
                return Err(Error::math(
                    Self { data: l, meta: None },
                    Math::Add,
                    Self { data: r, meta: None },
                ))
            }
        };
        Ok(Self { data, meta: None })
    }
}
```

Port the remaining five ops the same way (consult `ref/jaq/jaq-json/src/lib.rs`):
- `Sub`: numbers; seq difference (retain those not in rhs, set-style via a `Vec` contains check — our YamlVal is Ord so a `BTreeSet<&YamlVal>` works like jaq's).
- `Mul`: numbers; string×int repeat (0 or negative → null); map deep-merge (recursive helper `fn deep_merge(l: &mut Rc<YgMap>, r: &YgMap)` — port `obj_merge` from jaq-json:606).
- `Div`: numbers (int/int → Float ALWAYS when not exact? NO — jq: `10/3` = 3.333... division always float UNLESS both int and exact? jq returns 3.3333. jaq's Num div yields float. Do: both-int with `a % b == 0` → Int(a/b), else Float — then CHECK against manual triples in Task 19; if jq says `6/2 == 3` exactly, keep; the manual has `10/5` → `2`. This matches.); string/string split (empty rhs special-case per jaq `split` helper, empty lhs → empty seq).
- `Rem`: ints (b==0 → error), floats via `%`.
- `Neg`: numbers only.

Arithmetic results drop metadata (`meta: None`) — constructed values have no provenance.

Run: `cargo nextest run -p yamalgam-query`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/yamalgam-query
git commit -m "feat(query): jq arithmetic semantics for YamlVal"
```

---

## Task 7: jaq_core::ValT implementation

**Files:**
- Modify: `crates/yamalgam-query/src/valt.rs`
- Test: unit tests in `valt.rs`

THE integration trait. Reference: trait definition `ref/jaq/jaq-core/src/val.rs:55-155`, reference impl `ref/jaq/jaq-json/src/lib.rs:133-299`. Port method-by-method, with these YamlVal-specific decisions:
- `index` on mappings looks up ANY key type (r[filters.field.nonstring-keys]) — IndexMap.get already does this since our Eq/Hash are resolved-value based.
- `map_values`/`map_index`/`map_range` mutate `data` via `Rc::make_mut` and PRESERVE `self.meta` — this is what makes container styles survive `.a |= f` (Phase 2's best-effort preservation falls out of this design).
- Strings are `Rc<str>` (no byte strings): `range`/`map_range` on strings slice by CHARS (port `skip_take_chars` logic using `char_indices`).

- [ ] **Step 1: Write failing tests**

```rust
use jaq_core::ValT as _;

use crate::build::parse_str;
use crate::value::YamlVal;

#[test]
fn from_num_parses_ints_and_floats() {
    assert_eq!(YamlVal::from_num("3").unwrap(), YamlVal::int(3));
    assert_eq!(YamlVal::from_num("3.5").unwrap(), YamlVal::float(3.5));
    assert_eq!(YamlVal::from_num("1e3").unwrap(), YamlVal::float(1000.0));
}

#[test]
fn index_any_key_type() {
    // r[filters.field.nonstring-keys]
    let m = parse_str("1: int-key\ntrue: bool-key\n").unwrap().remove(0);
    assert_eq!(m.clone().index(&YamlVal::int(1)).unwrap(), YamlVal::str("int-key"));
    assert_eq!(m.index(&YamlVal::from(true)).unwrap(), YamlVal::str("bool-key"));
}

#[test]
fn index_missing_key_is_null() {
    // r[filters.field]
    let m = parse_str("a: 1\n").unwrap().remove(0);
    assert_eq!(m.index(&YamlVal::str("zzz")).unwrap(), YamlVal::null());
}

#[test]
fn negative_seq_index_wraps() {
    // r[filters.index]
    let s = parse_str("[1, 2, 3]\n").unwrap().remove(0);
    assert_eq!(s.index(&YamlVal::int(-1)).unwrap(), YamlVal::int(3));
}

#[test]
fn values_iterates_seq_and_map_values() {
    // r[filters.iterator]
    let m = parse_str("a: 1\nb: 2\n").unwrap().remove(0);
    let vals: Vec<_> = m.values().map(Result::unwrap).collect();
    assert_eq!(vals, vec![YamlVal::int(1), YamlVal::int(2)]);
}

#[test]
fn map_values_preserves_container_meta() {
    let m = parse_str("{a: 1}\n").unwrap().remove(0); // flow style
    let style_before = m.meta().unwrap().style;
    let out = m
        .map_values(jaq_core::path::Opt::Essential, |v| {
            Box::new(core::iter::once(Ok::<_, jaq_core::Exn<YamlVal>>(v))) as Box<dyn Iterator<Item = _>>
        })
        .unwrap();
    assert_eq!(out.meta().unwrap().style, style_before);
}

#[test]
fn string_range_slices_by_chars() {
    // r[filters.slice]
    let s = YamlVal::str("héllo");
    let r = s.range(Some(&YamlVal::int(1))..Some(&YamlVal::int(3))).unwrap();
    assert_eq!(r, YamlVal::str("él"));
}
```

(The `map_values` closure signature is fiddly — if the generic bounds fight you, test through the engine in Task 12 instead and keep a simpler direct test here. Don't burn an hour on test-harness generics.)

- [ ] **Step 2: Verify failure, then implement**

```rust
//! jaq trait implementations for [`YamlVal`].

use std::rc::Rc;

use jaq_core::box_iter::{box_once, BoxIter};
use jaq_core::path::Opt;
use jaq_core::val::{Range, ValR, ValX};
use jaq_core::Error;

use crate::value::{Data, YamlVal, YgMap};

impl From<Range<Self>> for YamlVal {
    fn from(r: Range<Self>) -> Self {
        let kv = |(k, v): (&str, Option<Self>)| v.map(|v| (Self::str(k), v));
        let kvs = [("start", r.start), ("end", r.end)];
        Self::map(kvs.into_iter().filter_map(kv).collect())
    }
}

impl jaq_core::ValT for YamlVal {
    fn from_num(n: &str) -> ValR<Self> {
        n.parse::<i64>().map(Self::int).or_else(|_| {
            n.parse::<f64>()
                .map(Self::float)
                .map_err(|_| Error::str(format_args!("cannot parse {n} as number")))
        })
    }

    fn from_map<I: IntoIterator<Item = (Self, Self)>>(iter: I) -> ValR<Self> {
        Ok(Self::map(iter.into_iter().collect()))
    }

    fn key_values(self) -> BoxIter<'static, ValR<(Self, Self), Self>> {
        match self.data {
            Data::Seq(a) => {
                let items = unwrap_rc(a);
                Box::new(items.into_iter().enumerate().map(|(i, x)| Ok((Self::from(i), x))))
            }
            Data::Map(m) => {
                let m = unwrap_rc_map(m);
                Box::new(m.0.into_iter().map(Ok))
            }
            _ => box_once(Err(Error::typ(self, "iterable (sequence or mapping)"))),
        }
    }

    fn values(self) -> Box<dyn Iterator<Item = ValR<Self>>> {
        match self.data {
            Data::Seq(a) => Box::new(unwrap_rc(a).into_iter().map(Ok)),
            Data::Map(m) => Box::new(unwrap_rc_map(m).0.into_iter().map(|(_k, v)| Ok(v))),
            _ => box_once(Err(Error::typ(self, "iterable (sequence or mapping)"))),
        }
    }

    fn index(self, index: &Self) -> ValR<Self> {
        match (&self.data, &index.data) {
            (Data::Null, _) => Ok(Self::null()),
            (Data::Seq(a), Data::Int(i)) => Ok(abs_index(*i, a.len())
                .and_then(|i| a.get(i).cloned())
                .unwrap_or_default()),
            (Data::Map(m), _) => Ok(m.0.get(index).cloned().unwrap_or_default()),
            // {start, end} object-slice form — port from jaq-json index_opt if
            // manual triples need it (Task 19 will tell you).
            _ => Err(Error::index(self.clone(), index.clone())),
        }
    }

    fn range(self, range: Range<&Self>) -> ValR<Self> {
        let as_idx = |v: Option<&Self>| -> Result<Option<i64>, Error<Self>> {
            v.filter(|v| !matches!(v.data, Data::Null))
                .map(|v| match v.data {
                    Data::Int(i) => Ok(i),
                    Data::Float(f) if f.is_finite() => Ok(f as i64),
                    _ => Err(Error::typ(v.clone(), "integer")),
                })
                .transpose()
        };
        let (start, end) = (as_idx(range.start)?, as_idx(range.end)?);
        match &self.data {
            Data::Seq(a) => {
                let (skip, take) = skip_take(start, end, a.len());
                Ok(Self::seq(a.iter().skip(skip).take(take).cloned().collect()))
            }
            Data::Str(s) => {
                let n = s.chars().count();
                let (skip, take) = skip_take(start, end, n);
                Ok(Self::str(s.chars().skip(skip).take(take).collect::<String>()))
            }
            _ => Err(Error::typ(self.clone(), "rangeable (sequence or string)")),
        }
    }

    fn map_values<'a, I: Iterator<Item = ValX<'a, Self>>>(
        mut self,
        opt: Opt,
        f: impl Fn(Self) -> I,
    ) -> ValX<'a, Self> {
        match &mut self.data {
            Data::Seq(a) => {
                let items = std::mem::take(Rc::make_mut(a));
                let mapped: Result<Vec<_>, _> = items.into_iter().flat_map(&f).collect();
                *Rc::make_mut(a) = mapped?;
                Ok(self)
            }
            Data::Map(m) => {
                let entries = std::mem::take(&mut Rc::make_mut(m).0);
                let mapped = entries
                    .into_iter()
                    .filter_map(|(k, v)| f(v).next().map(|v| Ok((k, v?))))
                    .collect::<Result<_, jaq_core::Exn<_>>>()?;
                Rc::make_mut(m).0 = mapped;
                Ok(self)
            }
            _ => opt.fail(self, |v| {
                jaq_core::Exn::from(Error::typ(v, "iterable (sequence or mapping)"))
            }),
        }
    }

    fn map_index<'a, I: Iterator<Item = ValX<'a, Self>>>(
        mut self,
        index: &Self,
        opt: Opt,
        f: impl Fn(Self) -> I,
    ) -> ValX<'a, Self> {
        match &mut self.data {
            Data::Map(m) => {
                let m = Rc::make_mut(m);
                match m.0.entry(index.clone()) {
                    indexmap::map::Entry::Occupied(mut e) => {
                        let v = std::mem::take(e.get_mut());
                        match f(v).next().transpose()? {
                            Some(y) => {
                                e.insert(y);
                            }
                            None => {
                                e.shift_remove(); // preserve order (NOT swap_remove)
                            }
                        }
                    }
                    indexmap::map::Entry::Vacant(e) => {
                        if let Some(y) = f(Self::null()).next().transpose()? {
                            e.insert(y);
                        }
                    }
                }
                Ok(self)
            }
            Data::Seq(a) => {
                let len = a.len();
                let i = match &index.data {
                    Data::Int(i) => abs_index(*i, len),
                    _ => None,
                };
                let Some(i) = i else {
                    let e = Error::str(format_args!("index {index} out of bounds"));
                    return opt.fail(self, |_| jaq_core::Exn::from(e));
                };
                let a = Rc::make_mut(a);
                let x = std::mem::take(&mut a[i]);
                match f(x).next().transpose()? {
                    Some(y) => a[i] = y,
                    None => {
                        a.remove(i);
                    }
                }
                Ok(self)
            }
            _ => opt.fail(self, |v| {
                jaq_core::Exn::from(Error::typ(v, "iterable (sequence or mapping)"))
            }),
        }
    }

    fn map_range<'a, I: Iterator<Item = ValX<'a, Self>>>(
        mut self,
        range: Range<&Self>,
        opt: Opt,
        f: impl Fn(Self) -> I,
    ) -> ValX<'a, Self> {
        // Sequences only for M10 (string-range assignment is exotic; jq supports
        // it, manual triples in Task 19 will flag if needed).
        match &mut self.data {
            Data::Seq(a) => {
                let as_idx = |v: Option<&Self>| match v.map(|v| &v.data) {
                    None | Some(Data::Null) => Ok(None),
                    Some(Data::Int(i)) => Ok(Some(*i)),
                    Some(_) => Err(Error::typ(v.unwrap().clone(), "integer")),
                };
                let (start, end) = match (as_idx(range.start), as_idx(range.end)) {
                    (Ok(s), Ok(e)) => (s, e),
                    (Err(e), _) | (_, Err(e)) => return opt.fail(self, |_| jaq_core::Exn::from(e)),
                };
                let (skip, take) = skip_take(start, end, a.len());
                let slice = Self::seq(a.iter().skip(skip).take(take).cloned().collect());
                let y = f(slice).next().transpose()?;
                let y = match y.map(|y| y.into_seq_vec()) {
                    None => Vec::new(),
                    Some(Ok(v)) => v,
                    Some(Err(v)) => {
                        let e = Error::typ(v, "sequence");
                        return opt.fail(self, |_| jaq_core::Exn::from(e));
                    }
                };
                Rc::make_mut(a).splice(skip..skip + take, y);
                Ok(self)
            }
            _ => opt.fail(self, |v| jaq_core::Exn::from(Error::typ(v, "sequence"))),
        }
    }

    fn as_bool(&self) -> bool {
        // r[filters.cond.and-or-not] — jq truthiness.
        !matches!(self.data, Data::Null | Data::Bool(false))
    }

    fn into_string(self) -> Self {
        match self.data {
            Data::Str(_) => Self { data: self.data, meta: None },
            _ => Self::str(self.to_string()),
        }
    }
}

/// `Rc` unwrap-or-clone helpers (MSRV-safe; Rc::unwrap_or_clone exists since 1.76,
/// fine to use directly — workspace MSRV is 1.89).
fn unwrap_rc<T: Clone>(rc: Rc<T>) -> T {
    Rc::unwrap_or_clone(rc)
}

fn unwrap_rc_map(rc: Rc<YgMap>) -> YgMap {
    Rc::unwrap_or_clone(rc)
}

/// Absolutize a possibly-negative index; None if out of bounds.
fn abs_index(i: i64, len: usize) -> Option<usize> {
    let i = if i < 0 { i + len as i64 } else { i };
    usize::try_from(i).ok().filter(|i| *i < len)
}

/// Clamp [start, end) bounds (negative wraps) to (skip, take).
fn skip_take(start: Option<i64>, end: Option<i64>, len: usize) -> (usize, usize) {
    let abs = |i: i64| -> usize {
        let i = if i < 0 { i + len as i64 } else { i };
        i.clamp(0, len as i64) as usize
    };
    let from = start.map_or(0, abs);
    let upto = end.map_or(len, abs);
    (from, upto.saturating_sub(from))
}
```

Add to `value.rs` the helper `map_range` uses:

```rust
impl YamlVal {
    /// Into a plain Vec if this is a sequence, else Err(self).
    pub(crate) fn into_seq_vec(self) -> Result<Vec<Self>, Self> {
        match self.data {
            Data::Seq(a) => Ok(std::rc::Rc::unwrap_or_clone(a)),
            _ => Err(self),
        }
    }
}
```

Compile-driven adjustments expected: exact `Exn`/`Error` constructor paths, `Opt::fail` signature, `entry` API. Lean on `ref/jaq/jaq-json/src/lib.rs` whenever the compiler disagrees with this sketch — jaq-json compiles against the same jaq-core 3.0.

- [ ] **Step 3: Run tests, commit**

Run: `cargo nextest run -p yamalgam-query`
Expected: PASS.

```bash
git add crates/yamalgam-query
git commit -m "feat(query): jaq_core::ValT for YamlVal"
```

---

## Task 8: jaq_std::ValT implementation

**Files:**
- Modify: `crates/yamalgam-query/src/valt.rs`
- Test: unit tests in `valt.rs` (append)

The extension trait jaq-std's generic natives need (sort_by, regex, implode, trim, base64...). Reference: `ref/jaq/jaq-json/src/lib.rs:301-346`.

- [ ] **Step 1: Tests**

```rust
#[test]
fn std_valt_surface() {
    use jaq_std::ValT as _;
    assert!(YamlVal::int(3).is_int());
    assert!(!YamlVal::float(3.0).is_int());
    assert_eq!(YamlVal::int(3).as_isize(), Some(3));
    assert_eq!(YamlVal::float(3.0).as_isize(), Some(3));
    assert_eq!(YamlVal::float(3.5).as_f64(), Some(3.5));
    assert!(YamlVal::str("x").is_utf8_str());
    assert_eq!(YamlVal::str("abc").as_bytes(), Some(b"abc".as_ref()));
    assert_eq!(
        YamlVal::seq(vec![YamlVal::int(1)]).into_seq::<Vec<_>>().unwrap(),
        vec![YamlVal::int(1)]
    );
}
```

- [ ] **Step 2: Implement**

```rust
impl jaq_std::ValT for YamlVal {
    fn into_seq<S: FromIterator<Self>>(self) -> Result<S, Self> {
        self.into_seq_vec().map(|v| v.into_iter().collect())
    }

    fn is_int(&self) -> bool {
        matches!(self.data, Data::Int(_))
    }

    fn as_isize(&self) -> Option<isize> {
        match self.data {
            Data::Int(i) => isize::try_from(i).ok(),
            Data::Float(f) if f.fract() == 0.0 && f.is_finite() => Some(f as isize),
            _ => None,
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self.data {
            Data::Int(i) => Some(i as f64),
            Data::Float(f) => Some(f),
            _ => None,
        }
    }

    fn is_utf8_str(&self) -> bool {
        matches!(self.data, Data::Str(_))
    }

    fn as_bytes(&self) -> Option<&[u8]> {
        match &self.data {
            Data::Str(s) => Some(s.as_bytes()),
            _ => None,
        }
    }

    fn as_sub_str(&self, sub: &[u8]) -> Self {
        // Our strings are Rc<str>; no zero-copy subslice. Allocate.
        Self::str(String::from_utf8_lossy(sub).into_owned())
    }

    fn from_utf8_bytes(b: impl AsRef<[u8]> + Send + 'static) -> Self {
        Self::str(String::from_utf8_lossy(b.as_ref()).into_owned())
    }
}
```

(Check the trait's exact method list against the installed jaq-std 3.0 — if it grew methods beyond the eight shown in `ref/jaq`, port those too from jaq-json's impl.)

- [ ] **Step 3: Run tests, commit**

Run: `cargo nextest run -p yamalgam-query`
Expected: PASS.

```bash
git add crates/yamalgam-query
git commit -m "feat(query): jaq_std::ValT for YamlVal"
```

---

## Task 9: JSON emitter

**Files:**
- Modify: `crates/yamalgam-query/src/emit/mod.rs`, `crates/yamalgam-query/src/emit/json.rs`
- Test: `crates/yamalgam-query/tests/emit.rs`

r[output.json]: valid JSON, metadata discarded, aliases expanded (our splice already materialized content — alias expansion is free), non-string keys stringified, tags/comments dropped. NaN/±Inf → `null` (JSON has no representation; matches jq's `tojson`).

- [ ] **Step 1: Tests**

```rust
// tests/emit.rs
use yamalgam_query::{parse_str, EmitOpts};

#[test]
fn json_output_is_valid_json() {
    // r[output.json]
    let v = parse_str("a: &x [1, .inf]\nb: *x\n1: int-key\n").unwrap().remove(0);
    let json = yamalgam_query::emit::to_json(&v, &EmitOpts::compact());
    assert_eq!(json, r#"{"a":[1,null],"b":[1,null],"1":[1,null]}"#);
}

#[test]
fn json_pretty_indents_two_spaces() {
    let v = parse_str("a: [1]\n").unwrap().remove(0);
    let json = yamalgam_query::emit::to_json(&v, &EmitOpts::default());
    assert_eq!(json, "{\n  \"a\": [\n    1\n  ]\n}");
}
```

(Make `emit` module `pub` in lib.rs and export `to_json`/`to_yaml` — adjust the Task 1 skeleton.)

- [ ] **Step 2: Implement**

`emit/mod.rs`:

```rust
//! Output serialization for query results.

mod json;
mod yaml;

pub use json::to_json;
pub use yaml::to_yaml;

/// Output format selector for the CLI.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EmitFormat {
    /// YAML (default). r[output.yaml]
    #[default]
    Yaml,
    /// JSON with implicit alias expansion. r[cli.json-output]
    Json,
}

/// Emitter options shared by both formats.
#[derive(Clone, Debug)]
pub struct EmitOpts {
    /// Indent width (spaces).
    pub indent: usize,
    /// Single-line output. r[cli.compact]
    pub compact: bool,
    /// Sort mapping keys. r[cli.sort-keys]
    pub sort_keys: bool,
    /// ANSI colors.
    pub color: bool,
}

impl Default for EmitOpts {
    fn default() -> Self {
        Self { indent: 2, compact: false, sort_keys: false, color: false }
    }
}

impl EmitOpts {
    /// Compact single-line output.
    #[must_use]
    pub fn compact() -> Self {
        Self { compact: true, ..Self::default() }
    }
}
```

`emit/json.rs`:

```rust
//! JSON output. r[output.json]

use crate::value::{Data, YamlVal};
use crate::EmitOpts;

/// Render a value as JSON text. Metadata dropped, non-string keys stringified,
/// non-finite floats become null.
pub fn to_json(v: &YamlVal, opts: &EmitOpts) -> String {
    let mut out = String::new();
    write_json(&mut out, v, opts, 0);
    out
}

fn write_json(out: &mut String, v: &YamlVal, opts: &EmitOpts, depth: usize) {
    let (nl, pad, pad_in, sep) = layout(opts, depth);
    match &v.data {
        Data::Null => out.push_str("null"),
        Data::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Data::Int(i) => out.push_str(&i.to_string()),
        Data::Float(f) if !f.is_finite() => out.push_str("null"),
        Data::Float(f) => out.push_str(&format!("{f:?}")),
        Data::Str(s) => out.push_str(&json_quote(s)),
        Data::Seq(items) if items.is_empty() => out.push_str("[]"),
        Data::Seq(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&nl);
                out.push_str(&pad_in);
                write_json(out, item, opts, depth + 1);
            }
            out.push_str(&nl);
            out.push_str(&pad);
            out.push(']');
        }
        Data::Map(m) if m.0.is_empty() => out.push_str("{}"),
        Data::Map(m) => {
            let mut entries: Vec<_> = m.0.iter().collect();
            if opts.sort_keys {
                entries.sort_by(|a, b| a.0.cmp(b.0));
            }
            out.push('{');
            for (i, (k, val)) in entries.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&nl);
                out.push_str(&pad_in);
                out.push_str(&json_quote(&key_string(k)));
                out.push(':');
                out.push_str(sep);
                write_json(out, val, opts, depth + 1);
            }
            out.push_str(&nl);
            out.push_str(&pad);
            out.push('}');
        }
    }
}

fn layout(opts: &EmitOpts, depth: usize) -> (String, String, String, &'static str) {
    if opts.compact {
        (String::new(), String::new(), String::new(), "")
    } else {
        let unit = " ".repeat(opts.indent);
        ("\n".into(), unit.repeat(depth), unit.repeat(depth + 1), " ")
    }
}

/// Stringify a mapping key for JSON. Strings pass through; everything else
/// uses its resolved string representation. r[output.json]
fn key_string(k: &YamlVal) -> String {
    match &k.data {
        Data::Str(s) => s.to_string(),
        _ => k.to_string(),
    }
}

fn json_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
```

- [ ] **Step 3: Run tests, commit**

Run: `cargo nextest run -p yamalgam-query`
Expected: PASS.

```bash
git add crates/yamalgam-query
git commit -m "feat(query): JSON emitter"
```

---

## Task 10: YAML emitter — scalars and quoting

**Files:**
- Modify: `crates/yamalgam-query/src/emit/yaml.rs`
- Modify: `crates/yamalgam-query/src/value.rs` (share `needs_quotes`/`fmt_float`/`quote_double` via `pub(crate)`)
- Test: `crates/yamalgam-query/tests/emit.rs` (append)

This is the workspace's FIRST YAML emitter (M13 builds the real one; this one is yg-scoped). Scalar rules:
- r[output.yaml.quoting]: strings that would re-resolve as another type MUST be quoted (`needs_quotes` from Task 5).
- r[output.yaml.style-default]: multi-line strings → literal block (`|`), short safe strings → plain, troublesome strings → double-quoted.
- Style honoring: a scalar whose `Meta.style` is `Literal` and whose content still has a trailing newline re-emits as literal; single-quoted stays single-quoted IF content has no `'` complications; otherwise fall back to the default decision. Best-effort, never invalid YAML.

- [ ] **Step 1: Tests**

```rust
use yamalgam_query::emit::to_yaml;

#[test]
fn scalars_quote_when_ambiguous() {
    // r[output.yaml.quoting]
    let v = parse_str(r#"[a, "1", "true", "null", "x: y"]"#).unwrap().remove(0);
    let out = to_yaml(&v, &EmitOpts::default());
    assert_eq!(out, "- a\n- \"1\"\n- \"true\"\n- \"null\"\n- \"x: y\"\n");
}

#[test]
fn multiline_strings_use_literal_block() {
    // r[output.yaml.style-default]
    let v = parse_str("key: |\n  line1\n  line2\n").unwrap().remove(0);
    let out = to_yaml(&v, &EmitOpts::default());
    assert_eq!(out, "key: |\n  line1\n  line2\n");
}

#[test]
fn round_trip_types_survive() {
    // emit → reparse → same resolved values
    let src = "int: 1\nfloat: 1.0\nbool: true\nnully: null\nstr: '1'\n";
    let v = parse_str(src).unwrap().remove(0);
    let emitted = to_yaml(&v, &EmitOpts::default());
    let re = parse_str(&emitted).unwrap().remove(0);
    assert_eq!(v, re);
}
```

- [ ] **Step 2: Implement scalar layer of emit/yaml.rs**

```rust
//! YAML output. r[output.yaml]

use yamalgam_scanner::ScalarStyle;

use crate::value::{fmt_float, needs_quotes, quote_double, Data, NodeStyle, YamlVal};
use crate::EmitOpts;

/// Render one value as a YAML document body (no `---` separator; the CLI
/// layer owns multi-doc separators). Always ends with a newline.
pub fn to_yaml(v: &YamlVal, opts: &EmitOpts) -> String {
    let mut out = String::new();
    let mut ctx = Ctx { opts, anchors_emitted: Vec::new() };
    ctx.node(&mut out, v, 0, Pos::Root);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Where a node sits — drives indentation/inline decisions.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pos {
    Root,
    SeqItem,
    MapValue,
    MapKey,
    Flow,
}

struct Ctx<'o> {
    opts: &'o EmitOpts,
    anchors_emitted: Vec<String>,
}

impl Ctx<'_> {
    fn scalar_text(&self, v: &YamlVal, pos: Pos) -> String {
        match &v.data {
            Data::Null => "null".into(),
            Data::Bool(b) => b.to_string(),
            Data::Int(i) => i.to_string(),
            Data::Float(f) => fmt_float(*f),
            Data::Str(s) => self.string_text(s, v, pos),
            _ => unreachable!("scalar_text on collection"),
        }
    }

    /// Pick a presentation for a string scalar. r[output.yaml.style-default]
    fn string_text(&self, s: &str, v: &YamlVal, pos: Pos) -> String {
        let from_style = v.meta().and_then(|m| m.style);
        let multiline = s.contains('\n');
        // Block scalars are illegal in flow context and as mapping keys.
        let block_ok = matches!(pos, Pos::Root | Pos::SeqItem | Pos::MapValue);
        if multiline && block_ok && s.ends_with('\n') && !self.opts.compact {
            return literal_block(s, self.opts.indent); // emitted by caller w/ indent
        }
        if multiline || needs_quotes(s) {
            return quote_double(s);
        }
        match from_style {
            Some(NodeStyle::Scalar(ScalarStyle::SingleQuoted)) if !s.contains('\'') => {
                format!("'{s}'")
            }
            Some(NodeStyle::Scalar(ScalarStyle::DoubleQuoted)) => quote_double(s),
            _ => s.to_owned(), // plain
        }
    }
}

/// Render `|` literal block — returns a marker string the collection layer
/// expands with correct indentation (see Task 11 `emit_block_scalar`).
fn literal_block(s: &str, _indent: usize) -> String {
    // Encoded as "\x00LIT\x00<content>" sentinel consumed by the caller; if
    // that feels gross (it is), restructure: scalar_text returns an enum
    // { Inline(String), Literal(String) } — DO THAT instead. Plan shows the
    // enum approach in Task 11; implement it there directly.
    format!("\u{0}LIT\u{0}{s}")
}
```

**Do NOT ship the sentinel hack.** Task 11 defines `enum Rendered { Inline(String), Literal(String) }` — implement `scalar_text -> Rendered` from the start; the sentinel note exists to explain WHY the enum exists. Literal blocks indent content one level deeper than the parent node and never appear in flow/compact mode or key position (fall back to `quote_double`).

- [ ] **Step 3: Run scalar tests (collection tests still fail until Task 11), commit**

Run: `cargo nextest run -p yamalgam-query -E 'test(scalars_quote)'`
Expected: PASS for scalar-only paths (the multiline + round-trip tests complete in Task 11).

```bash
git add crates/yamalgam-query
git commit -m "feat(query): YAML emitter scalar layer with quoting engine"
```

---

## Task 11: YAML emitter — collections, anchors, documents

**Files:**
- Modify: `crates/yamalgam-query/src/emit/yaml.rs`
- Test: `crates/yamalgam-query/tests/emit.rs` (append)

Block collections by default; flow when `compact` or when the node's `Meta.style` says flow (style honoring); `sort_keys` sorts mapping entries; anchors/tags re-emit; aliases re-emit as `*name` when their anchor was already emitted in this document, else inline expansion (best-effort, documented).

- [ ] **Step 1: Tests**

```rust
#[test]
fn block_collections_default() {
    let v = parse_str("a:\n  - 1\n  - x: 2\n").unwrap().remove(0);
    assert_eq!(to_yaml(&v, &EmitOpts::default()), "a:\n  - 1\n  - x: 2\n");
}

#[test]
fn flow_style_honored_and_compact_forces_flow() {
    let v = parse_str("a: {b: 1}\n").unwrap().remove(0);
    assert_eq!(to_yaml(&v, &EmitOpts::default()), "a: {b: 1}\n"); // meta says flow
    let block = parse_str("a:\n  b: 1\n").unwrap().remove(0);
    assert_eq!(to_yaml(&block, &EmitOpts::compact()), "{a: {b: 1}}\n"); // r[cli.compact]
}

#[test]
fn anchors_and_aliases_reemit() {
    let v = parse_str("base: &b\n  x: 1\nchild: *b\n").unwrap().remove(0);
    let out = to_yaml(&v, &EmitOpts::default());
    assert_eq!(out, "base: &b\n  x: 1\nchild: *b\n");
}

#[test]
fn tags_reemit_shorthand() {
    let v = parse_str("a: !custom hi\nb: !!binary aGk=\n").unwrap().remove(0);
    let out = to_yaml(&v, &EmitOpts::default());
    assert_eq!(out, "a: !custom hi\nb: !!binary aGk=\n");
}

#[test]
fn sort_keys_sorts_output_only() {
    // r[cli.sort-keys]
    let v = parse_str("b: 1\na: 2\n").unwrap().remove(0);
    let opts = EmitOpts { sort_keys: true, ..EmitOpts::default() };
    assert_eq!(to_yaml(&v, &opts), "a: 2\nb: 1\n");
}

#[test]
fn non_string_keys_emit() {
    let v = parse_str("1: a\ntrue: b\n? [1, 2]\n: c\n").unwrap().remove(0);
    let out = to_yaml(&v, &EmitOpts::default());
    assert_eq!(out, "1: a\ntrue: b\n? [1, 2]\n: c\n");
}
```

(If the `? [1, 2]` complex-key fixture trips the known `? key`-without-value parser gap, simplify to a flow-context complex key or drop the case with a `// blocked on parser gap` note — the gap is queued post-M10, don't fix the parser in this milestone.)

- [ ] **Step 2: Implement the collection layer**

Core shape (replace the Task 10 sentinel with this enum):

```rust
enum Rendered {
    Inline(String),
    /// Literal block content; caller writes `|`+chomp header and indents.
    Literal(String),
}

impl Ctx<'_> {
    fn node(&mut self, out: &mut String, v: &YamlVal, depth: usize, pos: Pos) {
        let prefix = self.props_prefix(v); // "&anchor !tag " as applicable
        // Alias re-emission: r[ext.explode] inverse — keep doc structure tight.
        if let Some(alias) = v.meta().and_then(|m| m.alias_of.as_deref()) {
            if self.anchors_emitted.iter().any(|a| a == alias) {
                out.push_str(&format!("*{alias}"));
                return;
            }
            // anchor not yet emitted (filter reordered things): inline expand
        }
        if let Some(anchor) = v.meta().and_then(|m| m.anchor.as_deref()) {
            self.anchors_emitted.push(anchor.to_owned());
        }
        let flow = self.opts.compact
            || matches!(
                v.meta().and_then(|m| m.style),
                Some(NodeStyle::Collection(yamalgam_parser::CollectionStyle::Flow))
            )
            || pos == Pos::Flow
            || pos == Pos::MapKey;
        match &v.data {
            Data::Seq(items) => { /* block: "- " entries at depth; flow: [a, b] */ }
            Data::Map(m) => { /* block: "k: v" lines, complex keys via "? k\n: v";
                                 flow: {k: v}; sort_keys via sorted entry Vec */ }
            _ => { /* prefix + scalar via scalar_text/Rendered */ }
        }
        let _ = (prefix, flow, depth, out);
    }

    /// "&anchor " / "!tag " prefix (anchor first, then tag — YAML allows either
    /// order; pick one, test it).
    fn props_prefix(&self, v: &YamlVal) -> String {
        let mut p = String::new();
        if let Some(m) = v.meta() {
            if m.alias_of.is_none() {
                if let Some(a) = m.anchor.as_deref() {
                    p.push_str(&format!("&{a} "));
                }
                if let Some(t) = v.tag_shorthand() {
                    // suppress !!merge (it's implied by the << key text itself)
                    if t != "!!merge" {
                        p.push_str(&format!("{t} "));
                    }
                }
            }
        }
        p
    }
}
```

The block-layout rules to encode (these ARE the test expectations above):
- Map entry, scalar value: `key: value\n`.
- Map entry, collection value: `key:\n` then child at `depth+1` — UNLESS child renders flow (then `key: {..}\n` inline).
- Map entry with anchored/tagged collection value: props go on the key line (`base: &b\n  x: 1`).
- Seq item: `- ` then inline scalar, or `- ` then nested block content with continuation indent (`- x: 2` for single-pair maps reads naturally if you inline the FIRST entry after `- `, which is the conventional style and what the test expects — alternatively emit `-\n    x: 2`; PICK the inline-first-entry form, it matches every YAML tool's output).
- Complex keys (non-scalar or multiline): `? key\n: value`.
- Empty seq → `[]`, empty map → `{}` (inline, even in block context).
- Literal blocks: `key: |\n<indented content>`; strip the trailing `\n` from content before splitting lines, re-add via block semantics; if content has trailing blank lines use `|+`? NO — keep clip-only for M10; strings whose content would need `|+`/`|-` fidelity fall back to double-quoted. Document.

Write it straightforwardly; ~150 lines. Test-drive each layout rule.

- [ ] **Step 3: Multi-doc + full test pass**

`to_yaml` stays single-value; CLI joins with `---\n` separators (Task 18). Run the FULL emit suite now including Task 10's multiline/round-trip tests:

Run: `cargo nextest run -p yamalgam-query`
Expected: PASS.

- [ ] **Step 4: Property check vs corpus (cheap confidence)**

Add one test that walks `crates/yamalgam-serde/tests/fixtures/**/*.{yml,yaml}` (the 408-file M9 corpus), parses each with `parse_str`, emits, re-parses, and asserts resolved-value equality (`==`) for every document. Allowlist file for any failures (same pattern as the compliance harness — a `const ALLOWLIST: &[&str]` of relative paths with reasons). Target: empty or near-empty allowlist; investigate before allowlisting.

```rust
#[test]
fn emit_reparse_round_trips_fixture_corpus() { /* walkdir or std fs::read_dir recursion */ }
```

(std-only recursion; don't add a walkdir dep for one test.)

- [ ] **Step 5: Commit**

```bash
git add crates/yamalgam-query
git commit -m "feat(query): YAML emitter — block/flow collections, anchors, tags"
```

---

## Task 12: Engine wiring — compile and run

**Files:**
- Modify: `crates/yamalgam-query/src/engine.rs`
- Create: `crates/yamalgam-query/src/defs.jq`
- Modify: `crates/yamalgam-query/src/error.rs` (YgError)
- Test: `crates/yamalgam-query/tests/engine.rs`

Template: `ref/jaq/jaq-all/src/data.rs` (DataKind/Data/base_funs/run) + `ref/jaq/jaq/src/filter.rs` (parse_compile). Ours adds `RunCtx` (schema + LoaderConfig + per-input metadata) and splits load errors from compile errors for exit codes 2 vs 3 (r[cli.exit.usage], r[cli.exit.compile]).

- [ ] **Step 1: Tests (in-process end-to-end)**

```rust
// tests/engine.rs
use yamalgam_core::LoaderConfig;
use yamalgam_query::engine::{self, RunCtx};

/// Helper: run `filter` against `input` YAML, render outputs as compact flow.
fn eval(filter: &str, input: &str) -> Vec<String> {
    let ctx = RunCtx::new(LoaderConfig::moderate());
    let compiled = engine::compile_filter(filter, &[]).unwrap();
    let docs = yamalgam_query::parse_str(input).unwrap();
    let mut out = Vec::new();
    engine::run_filter(&compiled, &ctx, docs.into_iter().map(Ok), |v| {
        out.push(v.to_string());
        Ok::<(), yamalgam_query::YgError>(())
    })
    .unwrap();
    out
}

#[test]
fn identity_and_field_access() {
    // r[filters.identity] r[filters.field]
    assert_eq!(eval(".", "a: 1\n"), vec!["{a: 1}"]);
    assert_eq!(eval(".a", "a: 1\n"), vec!["1"]);
    assert_eq!(eval(".a.b", "a: {b: 2}\n"), vec!["2"]);
    assert_eq!(eval(".missing", "a: 1\n"), vec!["null"]);
}

#[test]
fn pipes_commas_iteration() {
    // r[filters.pipe] r[filters.comma] r[filters.iterator]
    assert_eq!(eval(".[] | . + 1", "[1, 2]\n"), vec!["2", "3"]);
    assert_eq!(eval(".a, .b", "a: 1\nb: 2\n"), vec!["1", "2"]);
}

#[test]
fn keys_is_insertion_ordered() {
    // r[diverge.keys]
    assert_eq!(eval("keys", "b: 1\na: 2\n"), vec!["[b, a]"]);
    // r[diverge.keys-sorted]
    assert_eq!(eval("keys_sorted", "b: 1\na: 2\n"), vec!["[a, b]"]);
    // r[diverge.keys.seq]
    assert_eq!(eval("keys", "[9, 8]\n"), vec!["[0, 1]"]);
}

#[test]
fn type_returns_yaml_names() {
    // r[diverge.type]
    assert_eq!(
        eval("map(type)", r#"[null, true, 1, 1.5, "s", [], {}]"#),
        vec!["[null, boolean, integer, float, string, sequence, mapping]"]
    );
}

#[test]
fn defs_reduce_conditionals_work() {
    // r[filters.def] r[filters.reduce] r[filters.cond.if]
    assert_eq!(
        eval("def double: . * 2; [.[] | if . > 1 then double else . end]", "[1, 2]\n"),
        vec!["[1, 4]"]
    );
    // r[filters.reduce]
    assert_eq!(eval("reduce .[] as $x (0; . + $x)", "[1, 2, 3]\n"), vec!["6"]);
}

#[test]
fn string_interpolation() {
    // r[filters.string-interp]
    assert_eq!(eval(r#""v=\(.a)""#, "a: 7\n"), vec!["v=7"]);
}

#[test]
fn load_vs_compile_errors() {
    // r[cli.exit.usage]: syntax error
    assert!(matches!(
        engine::compile_filter(".a |", &[]).unwrap_err(),
        yamalgam_query::YgError::FilterSyntax(_)
    ));
    // r[cli.exit.compile]: undefined name
    assert!(matches!(
        engine::compile_filter("nosuchfilter", &[]).unwrap_err(),
        yamalgam_query::YgError::FilterCompile(_)
    ));
}
```

- [ ] **Step 2: defs.jq**

```jq
# yg definitions — loaded AFTER jaq-core/jaq-std defs, so these shadow.

# r[impl diverge.type] — YAML data-model type names via the _yaml_type native.
def type: _yaml_type;

# r[impl diverge.keys] — insertion order (keys_unsorted native preserves
# IndexMap order through key_values).
def keys: keys_unsorted;

# r[impl diverge.keys-sorted]
def keys_sorted: keys_unsorted | sort;

# Ports of jaq-json defs (we don't depend on jaq-json) — see
# ref/jaq/jaq-json/src/defs.jq. Copy VERBATIM:
def totype(p; e): if p then . else fromjson | if p then . else e end end;
def tonumber : totype(isnumber ; error("cannot parse as number" ));
def toboolean: totype(isboolean; error("cannot parse as boolean"));
def transpose: [range([.[] | length] | max) as $i | [.[][$i]]];
def in(xs)    : . as $x | xs | has     ($x);
def inside(xs): . as $x | xs | contains($x);
def  index($i): indices($i)[ 0];
def rindex($i): indices($i)[-1];
def @json: tojson;
```

- [ ] **Step 3: Implement engine.rs**

```rust
//! jaq engine wiring: data kind, compilation, execution.

use std::cell::RefCell;
use std::rc::Rc;

use jaq_core::{compile, load, Ctx, DataT, Filter, Lut, Vars};
use jaq_std::input::{Inputs, RcIter};
use yamalgam_core::{LoaderConfig, TagResolution};

use crate::error::YgError;
use crate::value::YamlVal;

/// yg's jaq data kind.
pub struct YgKind;

impl DataT for YgKind {
    type V<'a> = YamlVal;
    type Data<'a> = &'a YgData<'a>;
}

/// Per-run global data visible to native filters.
pub struct YgData<'a> {
    /// Compiled filter lookup table.
    pub lut: &'a Lut<YgKind>,
    /// Remaining inputs (for `input`/`inputs`).
    pub inputs: Inputs<'a, YamlVal>,
    /// yg-specific run context.
    pub ctx: &'a RunCtx,
}

impl<'a> jaq_core::data::HasLut<'a, YgKind> for &'a YgData<'a> {
    fn lut(&self) -> &'a Lut<YgKind> {
        self.lut
    }
}

impl<'a> jaq_std::input::HasInputs<'a, YamlVal> for &'a YgData<'a> {
    fn inputs(&self) -> Inputs<'a, YamlVal> {
        self.inputs
    }
}

/// Metadata about the input currently being processed. Updated by the input
/// iterator as it yields documents; read by document_index/file_index/filename.
#[derive(Clone, Debug, Default)]
pub struct InputMeta {
    /// Current file name (None for stdin). r[ext.filename]
    pub filename: Option<Rc<str>>,
    /// Zero-based file index. r[ext.file-index]
    pub file_index: usize,
    /// Zero-based document index within the file. r[ext.document-index]
    pub doc_index: usize,
}

/// Run-wide context: schema, loader config, current-input metadata.
pub struct RunCtx {
    /// Loader config (limits + tag resolution). r[cli.schema-flag]
    pub config: LoaderConfig,
    /// Current input metadata (interior-mutable; the input iterator writes it).
    pub input_meta: RefCell<InputMeta>,
}

impl RunCtx {
    /// New context from a loader config.
    #[must_use]
    pub fn new(config: LoaderConfig) -> Self {
        Self { config, input_meta: RefCell::new(InputMeta::default()) }
    }

    /// Active schema name. r[ext.schema]
    #[must_use]
    pub const fn schema_name(&self) -> &'static str {
        match self.config.tag_resolution {
            TagResolution::Yaml12 => "core",
            TagResolution::Failsafe => "failsafe",
            TagResolution::Json => "json",
            TagResolution::Yaml11 => "1.1",
        }
    }
}

/// yg definitions (shadow jaq-std's type/keys).
fn yg_defs() -> impl Iterator<Item = load::parse::Def<&'static str>> {
    load::parse(include_str!("defs.jq"), |p| p.defs())
        .expect("defs.jq must parse")
        .into_iter()
}

/// Compile a filter. Splits syntax (load) errors from compile errors so the
/// CLI maps them to exit 2 vs 3. r[cli.exit.usage] r[cli.exit.compile]
pub fn compile_filter(code: &str, vars: &[String]) -> Result<Filter<YgKind>, YgError> {
    let arena = load::Arena::default();
    let defs = jaq_core::defs().chain(jaq_std::defs()).chain(yg_defs());
    let loader = load::Loader::new(defs);
    let path: load::File<&str, &str> = load::File { path: "<filter>", code };
    let modules = loader
        .load(&arena, path)
        .map_err(|e| YgError::FilterSyntax(render_load_errors(e)))?;
    let vars: Vec<String> = vars.iter().map(|v| format!("${v}")).collect();
    let compiler = compile::Compiler::default()
        .with_funs(crate::funs::all_funs())
        .with_global_vars(vars.iter().map(String::as_str));
    compiler
        .compile(modules)
        .map_err(|e| YgError::FilterCompile(render_compile_errors(e)))
}

/// Run a compiled filter over inputs, calling `f` per output value.
/// Returns the last output's truthiness (for `-e`). r[cli.exit.falsy]
pub fn run_filter<E: From<YgError>>(
    filter: &Filter<YgKind>,
    ctx: &RunCtx,
    inputs: impl Iterator<Item = Result<YamlVal, String>>,
    mut f: impl FnMut(YamlVal) -> Result<(), E>,
) -> Result<Option<bool>, E> {
    use jaq_core::ValT as _;
    let inputs: Box<dyn Iterator<Item = Result<YamlVal, String>>> = Box::new(inputs);
    let inputs = RcIter::new(inputs);
    let data = YgData { lut: &filter.lut, inputs: &inputs, ctx };
    let jctx = Ctx::new(&data, Vars::new(Vec::new()));
    let mut last = None;
    let run_one = |x: YamlVal, f: &mut dyn FnMut(YamlVal) -> Result<(), E>, last: &mut Option<bool>| {
        filter.id.run((jctx.clone(), x)).try_for_each(|y| {
            let y = y.map_err(|e| E::from(YgError::runtime(e)))?;
            *last = Some(y.as_bool());
            f(y)
        })
    };
    while let Some(x) = (&inputs).next() {
        match x {
            Ok(x) => run_one(x, &mut f, &mut last)?,
            Err(e) => return Err(E::from(YgError::InputParse(e))),
        }
    }
    Ok(last)
}
```

Things the executor must reconcile against real jaq-core 3.0 signatures (compile errors will point the way; jaq's own `filter.rs`/`data.rs` are the ground truth):
- `load::File` field/types and `Loader::load` error shape — render via jaq's `FileReports` Display (see `jaq/src/main.rs:271-276`); store rendered `String`s in `YgError`.
- `Vars::new` + null-input handling — jaq feeds a separate `null` RcIter for `-n`; our CLI passes `once(Ok(YamlVal::null()))` as inputs instead, KEEPING `data.inputs` pointed at the real input iterator so `input`/`inputs` still work under `-n` (jq semantics). Mirror `jaq-all/src/data.rs run()` exactly on this point.
- `filter.id.run((ctx, x))` — exact accessor may differ; check `data.rs`.
- `halt`/`halt_error` produce `Exn` with exit payloads — `YgError::runtime` must preserve `e.get_halt()` so the CLI can `exit(n)` (see Task 18; store `Halt(i32)` as its own variant).

`YgError` (error.rs):

```rust
/// Engine errors, partitioned by CLI exit code. r[cli.exit]
#[derive(Debug, thiserror::Error)]
pub enum YgError {
    /// Filter lex/parse failure → exit 2.
    #[error("{0}")]
    FilterSyntax(String),
    /// Filter compile failure (undefined names) → exit 3.
    #[error("{0}")]
    FilterCompile(String),
    /// Input document failed to parse → exit 5.
    #[error("{0}")]
    InputParse(String),
    /// Filter runtime error → exit 5.
    #[error("{0}")]
    Runtime(String),
    /// `halt(n)` → exit n.
    #[error("halt")]
    Halt(i32),
}

impl YgError {
    /// Classify a runtime exception: `halt(n)` becomes [`Self::Halt`],
    /// everything else renders into [`Self::Runtime`].
    pub(crate) fn runtime(e: jaq_core::Exn<crate::YamlVal>) -> Self {
        match e.get_halt() {
            Ok(code) => Self::Halt(code),
            Err(e) => Self::Runtime(
                e.get_err().map_or_else(|_| "unknown error".into(), |err| err.to_string()),
            ),
        }
    }
}
```

(`Exn::get_halt`/`get_err` shapes: see `ref/jaq/jaq-core/src/exn.rs` and how `jaq/src/filter.rs:54-55` unwraps them; adjust to the real API.)

- [ ] **Step 4: Stub funs::all_funs() minimally**

Task 13 fills natives; to make THIS task compile and its tests pass, `all_funs` needs at least the std chain plus `_yaml_type`, `length`, `has`, `contains`, `indices`, `tojson`, `fromjson`, `isnumber`-family support (check which of those are defs vs natives — `isboolean` etc. are jaq-std defs already). Minimum for Step 1's tests: `_yaml_type` + whatever `defs.jq` references that isn't yet defined — TRIM defs.jq to only `type`/`keys`/`keys_sorted` in this task if needed, restoring the ports in Task 13.

```rust
//! Native filter registration.

use jaq_core::native::{bome, v, Fun, RunPtr};
use jaq_core::{DataT, Filter};

use crate::engine::YgKind;
use crate::value::YamlVal;

/// All native filters for the yg engine.
pub fn all_funs() -> impl Iterator<Item = Fun<YgKind>> {
    let run = jaq_core::native::run::<YgKind>;
    jaq_core::funs::<YgKind>()
        .chain(jaq_std::funs::<YgKind>())
        .chain(jaq_std::input::funs::<YgKind>().into_vec().into_iter().map(run))
        .chain(yg_natives().into_vec().into_iter().map(run))
}

fn yg_natives() -> Box<[Filter<RunPtr<YgKind>>]> {
    Box::new([
        // r[impl diverge.type]
        ("_yaml_type", v(0), |cv| {
            bome(Ok(YamlVal::str(cv.1.type_name())))
        }),
    ])
}
```

- [ ] **Step 5: Run tests, commit**

Run: `cargo nextest run -p yamalgam-query`
Expected: PASS (engine tests green; this is the milestone's first end-to-end filter execution — celebrate accordingly).

```bash
git add crates/yamalgam-query
git commit -m "feat(query): jaq engine wiring — compile, run, yg defs"
```

---

## Task 13: Core native filters

**Files:**
- Modify: `crates/yamalgam-query/src/funs.rs`
- Modify: `crates/yamalgam-query/src/defs.jq` (restore full ports if trimmed)
- Test: `crates/yamalgam-query/tests/engine.rs` (append)

Port the jaq-json natives we lack (reference `ref/jaq/jaq-json/src/funs.rs`): `length`, `has`, `contains`, `indices`, `bsearch`, `tojson`, `fromjson`. Skip `tobytes` (no byte strings in YAML's data model — M10 treats `!!binary` as a tagged string).

- [ ] **Step 1: Tests**

```rust
#[test]
fn core_native_filters() {
    assert_eq!(eval("length", "[1, 2, 3]\n"), vec!["3"]);
    assert_eq!(eval("length", "\"héllo\"\n"), vec!["5"]); // codepoints
    assert_eq!(eval("length", "a: 1\n"), vec!["1"]);
    assert_eq!(eval("length", "null\n"), vec!["0"]);
    assert_eq!(eval("has(\"a\")", "a: 1\n"), vec!["true"]);
    assert_eq!(eval("has(1)", "[9, 8]\n"), vec!["true"]);
    assert_eq!(eval("contains(\"ell\")", "\"hello\"\n"), vec!["true"]);
    assert_eq!(eval("indices(\",\")", "\"a,b,c\"\n"), vec!["[1, 3]"]);
    // tojson yields a STRING value; the eval helper renders via Display, which
    // double-quotes strings containing `{`/`"`. Assert through -r-style content:
    assert_eq!(eval("tojson | fromjson | .a", "a: [1, null]\n"), vec!["[1, null]"]);
    assert_eq!(eval("tojson | length", "a: [1, null]\n"), vec!["14"]); // {"a":[1,null]}
    assert_eq!(eval("fromjson", "'{\"a\": 1}'\n"), vec!["{a: 1}"]);
    assert_eq!(eval("add", "[{a: 1}, {b: 2}]\n"), vec!["{a: 1, b: 2}"]); // r[diverge.add]
    assert_eq!(eval("to_entries", "a: 1\n"), vec!["[{key: a, value: 1}]"]); // r[diverge.entries]
    assert_eq!(eval("explode", "\"ab\"\n"), vec!["[97, 98]"]); // deviation #3: jq semantics kept
}
```

(`add`/`to_entries`/`with_entries`/`from_entries` are jaq-std defs built on operators we already have — the tests prove they work through OUR ValT, no porting needed.)

- [ ] **Step 2: Implement natives**

```rust
fn yg_natives() -> Box<[Filter<RunPtr<YgKind>>]> {
    Box::new([
        ("_yaml_type", v(0), |cv| bome(Ok(YamlVal::str(cv.1.type_name())))),
        // r[impl diverge] length: 0/null, |n|/numbers, codepoints/strings, len/collections
        ("length", v(0), |cv| bome(cv.1.length())),
        ("has", v(1), |cv| {
            jaq_core::native::unary(cv, |v, k| v.has(&k).map(YamlVal::from))
        }),
        ("contains", v(1), |cv| {
            jaq_core::native::unary(cv, |v, c| Ok(YamlVal::from(v.contains(&c))))
        }),
        ("indices", v(1), |cv| {
            jaq_core::native::unary(cv, |v, y| v.indices(&y))
        }),
        ("bsearch", v(1), |cv| {
            jaq_core::native::unary(cv, |v, t| v.bsearch(&t))
        }),
        ("tojson", v(0), |cv| {
            bome(Ok(YamlVal::str(crate::emit::to_json(&cv.1, &crate::EmitOpts::compact()))))
        }),
        ("fromjson", v(0), |cv| bome(cv.1.fromjson())),
    ])
}
```

With `impl YamlVal` helpers in funs.rs (port bodies from jaq-json, adapting types):

```rust
impl YamlVal {
    fn length(&self) -> jaq_core::ValR<Self> { /* port jaq-json funs.rs:16-26 */ }
    fn has(&self, key: &Self) -> Result<bool, jaq_core::Error<Self>> {
        // seq: integer index in bounds; map: key present (any key type)
        match (&self.data, &key.data) {
            (Data::Seq(a), Data::Int(i)) => Ok(*i >= 0 && (*i as usize) < a.len()),
            (Data::Map(m), _) => Ok(m.0.contains_key(key)),
            _ => Err(jaq_core::Error::index(self.clone(), key.clone())),
        }
    }
    fn contains(&self, other: &Self) -> bool { /* port jaq-json recursive contains */ }
    fn indices(&self, y: &Self) -> jaq_core::ValR<Self> { /* port; chars for strings */ }
    fn bsearch(&self, target: &Self) -> jaq_core::ValR<Self> { /* port: Ok(idx) | Err(-(ins+1)) */ }
    fn fromjson(&self) -> jaq_core::ValR<Self> {
        // JSON is YAML: parse with the JSON resolver. r[datamodel.schema.json]
        let Data::Str(s) = &self.data else {
            return Err(jaq_core::Error::typ(self.clone(), "string"));
        };
        let config = yamalgam_core::LoaderConfig::moderate()
            .with_tag_resolution(yamalgam_core::TagResolution::Json);
        let mut docs = crate::build::parse_str_with_config(s, &config)
            .map_err(|e| jaq_core::Error::str(format_args!("fromjson: {e}")))?;
        match docs.len() {
            1 => Ok(docs.remove(0)),
            n => Err(jaq_core::Error::str(format_args!("fromjson: expected 1 document, got {n}"))),
        }
    }
}
```

(`unary`'s exact signature: check `ref/jaq/jaq-core/src/native.rs`. If it doesn't exist in the published 3.0 with that shape, write the closure long-hand like jaq-json does: pop the arg via `cv.0.pop_var()` or run the param filter — copy whatever jaq-json's `has` does VERBATIM.)

- [ ] **Step 3: Run tests, commit**

Run: `cargo nextest run -p yamalgam-query`
Expected: PASS.

```bash
git add crates/yamalgam-query
git commit -m "feat(query): core native filters (length, has, contains, tojson, fromjson)"
```

---

## Task 14: YAML metadata filters

**Files:**
- Modify: `crates/yamalgam-query/src/funs.rs`
- Test: `crates/yamalgam-query/tests/engine.rs` (append)

The Phase 1 extension set: `tag`, `anchor`, `style`, `kind`, `raw`, `resolved_type`, `is_alias`.

- [ ] **Step 1: Tests**

```rust
#[test]
fn metadata_filters() {
    let doc = "plain: 1\nquoted: \"1\"\nlit: |\n  x\nanchored: &a [1]\nref: *a\ntagged: !custom v\n";
    // r[ext.tag]
    assert_eq!(eval(".tagged | tag", doc), vec!["!custom"]);
    assert_eq!(eval(".plain | tag", doc), vec!["null"]); // implicit → null
    // r[ext.anchor]
    assert_eq!(eval(".anchored | anchor", doc), vec!["a"]);
    assert_eq!(eval(".plain | anchor", doc), vec!["null"]);
    // r[ext.style]
    assert_eq!(eval(".quoted | style", doc), vec!["double-quoted"]);
    assert_eq!(eval(".lit | style", doc), vec!["literal"]);
    assert_eq!(eval(".anchored | style", doc), vec!["flow"]);
    // r[ext.kind]
    assert_eq!(eval(".ref | kind", doc), vec!["alias"]);
    assert_eq!(eval(".plain | kind", doc), vec!["scalar"]);
    // r[ext.is-alias]
    assert_eq!(eval(".ref | is_alias", doc), vec!["true"]);
    // r[ext.raw]
    assert_eq!(eval(".plain | raw", doc), vec!["1"]);
    assert_eq!(eval("\"1.0\" | raw", "null\n"), vec!["1.0"]); // hmm — constructed
    // ^ constructed strings: raw falls back to the string content. For
    //   constructed NON-strings raw falls back to canonical scalar text.
    // r[ext.resolved-type]
    assert_eq!(eval(".plain | resolved_type", doc), vec!["integer"]);
}

#[test]
fn raw_errors_on_collections() {
    // r[ext.raw]
    let ctx = RunCtx::new(LoaderConfig::moderate());
    let f = engine::compile_filter("raw", &[]).unwrap();
    let docs = yamalgam_query::parse_str("[1]\n").unwrap();
    let r = engine::run_filter(&f, &ctx, docs.into_iter().map(Ok), |_| Ok::<_, yamalgam_query::YgError>(()));
    assert!(r.is_err());
}
```

- [ ] **Step 2: Implement** (append to `yg_natives()`)

```rust
// r[impl ext.tag] — explicit tag in shorthand, null for implicit.
("tag", v(0), |cv| {
    bome(Ok(cv.1.tag_shorthand().map_or_else(YamlVal::null, YamlVal::from)))
}),
// r[impl ext.anchor]
("anchor", v(0), |cv| {
    bome(Ok(cv.1.meta()
        .and_then(|m| m.anchor.as_deref().or(m.alias_of.as_deref()))
        .map_or_else(YamlVal::null, |a| YamlVal::str(a.to_owned()))))
}),
// r[impl ext.style] — "plain"|"single-quoted"|"double-quoted"|"literal"|"folded"|"block"|"flow"
("style", v(0), |cv| bome(Ok(cv.1.style_name().map_or_else(YamlVal::null, YamlVal::str)))),
// r[impl ext.kind]
("kind", v(0), |cv| bome(Ok(YamlVal::str(cv.1.kind_name())))),
// r[impl ext.is-alias]
("is_alias", v(0), |cv| {
    bome(Ok(YamlVal::from(cv.1.meta().is_some_and(|m| m.alias_of.is_some()))))
}),
// r[impl ext.raw]
("raw", v(0), |cv| bome(cv.1.raw_text())),
// r[impl ext.resolved-type]
("resolved_type", v(0), |cv| bome(Ok(YamlVal::str(cv.1.type_name())))),
```

With value.rs helpers:

```rust
impl YamlVal {
    /// Style name per r[ext.style], None when no style metadata.
    #[must_use]
    pub fn style_name(&self) -> Option<&'static str> {
        use yamalgam_parser::CollectionStyle;
        use yamalgam_scanner::ScalarStyle;
        self.meta().and_then(|m| m.style).map(|s| match s {
            NodeStyle::Scalar(ScalarStyle::Plain) => "plain",
            NodeStyle::Scalar(ScalarStyle::SingleQuoted) => "single-quoted",
            NodeStyle::Scalar(ScalarStyle::DoubleQuoted) => "double-quoted",
            NodeStyle::Scalar(ScalarStyle::Literal) => "literal",
            NodeStyle::Scalar(ScalarStyle::Folded) => "folded",
            NodeStyle::Collection(CollectionStyle::Block) => "block",
            NodeStyle::Collection(CollectionStyle::Flow) => "flow",
        })
    }

    /// Raw scalar text. r[ext.raw] — stored raw for input scalars, canonical
    /// text for constructed scalars, error for collections.
    pub(crate) fn raw_text(&self) -> jaq_core::ValR<Self> {
        match &self.data {
            Data::Seq(_) | Data::Map(_) => {
                Err(jaq_core::Error::str(format_args!("{} has no raw form", self.type_name())))
            }
            Data::Str(s) => Ok(Self::str(
                self.meta()
                    .and_then(|m| m.raw.clone())
                    .unwrap_or_else(|| s.clone()),
            )),
            _ => Ok(Self::str(
                self.meta()
                    .and_then(|m| m.raw.as_deref().map(str::to_owned))
                    .unwrap_or_else(|| self.to_string()),
            )),
        }
    }
}
```

Note `resolved_type` == `type_name()` today because the builder resolves at parse time under the active schema; the distinction (r[ext.resolved-type]) becomes meaningful with `resolve_as` (Task 15) — the doc comment should say so.

Decision encoded above: `anchor` on an ALIAS node returns the referenced anchor name (`.or(m.alias_of...)`) — that's what makes `.anchored | anchor == (.ref | anchor)` express "same identity", the useful reading of r[comparison.strict.anchor-identity] without `===`. Test both.

- [ ] **Step 3: Run tests, commit**

Run: `cargo nextest run -p yamalgam-query`
Expected: PASS.

```bash
git add crates/yamalgam-query
git commit -m "feat(query): YAML metadata filters (tag, anchor, style, kind, raw)"
```

---

## Task 15: Schema, context, and alias-audit filters

**Files:**
- Modify: `crates/yamalgam-query/src/funs.rs`
- Test: `crates/yamalgam-query/tests/engine.rs` (append)

`schema`/`resolve`/`resolve_as` (§6.3), `document_index`/`file_index`/`filename` (§6.4), `explode_aliases` (deviation #3), `aliases`/`orphan_anchors` (§6.7).

These natives need `RunCtx` — they read `cv.0.data().ctx`. This is why YgData carries it.

- [ ] **Step 1: Tests**

```rust
#[test]
fn schema_filters() {
    // r[ext.schema]
    assert_eq!(eval("schema", "null\n"), vec!["core"]);
    // r[ext.resolve-as] — "yes" is a string under core, boolean under 1.1
    assert_eq!(eval("\"yes\" | resolve_as(\"1.1\")", "null\n"), vec!["true"]);
    assert_eq!(eval("\"yes\" | resolve_as(\"1.1\") | type", "null\n"), vec!["boolean"]);
    // r[ext.resolve] — re-resolve raw text under the active schema
    assert_eq!(eval(".a | resolve | type", "a: '123'\n"), vec!["string"]);
    // quoted stays string under core: quoting forces str. resolve honors raw+style?
    // DECISION: resolve/resolve_as operate on the scalar's TEXT (raw or content)
    // as if it were a plain scalar. So '123' | resolve → 123 integer? NO —
    // see step note below; pin behavior in this test after deciding.
}

#[test]
fn input_context_filters() {
    // document_index increments per document. r[ext.document-index]
    // (filename/file_index need the CLI; unit-test via RunCtx mutation.)
    let ctx = RunCtx::new(LoaderConfig::moderate());
    ctx.input_meta.borrow_mut().filename = Some("test.yml".into());
    ctx.input_meta.borrow_mut().doc_index = 2;
    let f = engine::compile_filter("[filename, document_index, file_index]", &[]).unwrap();
    let mut out = Vec::new();
    engine::run_filter(&f, &ctx, std::iter::once(Ok(YamlVal::null())), |v| {
        out.push(v.to_string());
        Ok::<_, yamalgam_query::YgError>(())
    })
    .unwrap();
    assert_eq!(out, vec!["[test.yml, 2, 0]"]);
}

#[test]
fn alias_audit_filters() {
    let doc = "a: &x 1\nb: *x\nc: &unused 2\n";
    // r[ext.aliases] — anchor name → paths of alias USES
    assert_eq!(eval("aliases", doc), vec!["{x: [[b]]}"]);
    // r[ext.orphan-anchors]
    assert_eq!(eval("orphan_anchors", doc), vec!["[unused]"]);
    // deviation #3: explode_aliases strips identity markers
    assert_eq!(eval("explode_aliases | .b | is_alias", doc), vec!["false"]);
    assert_eq!(eval("explode_aliases | .a | anchor", doc), vec!["null"]);
}
```

**`resolve` semantics decision (write the test to match):** `resolve` re-runs schema resolution on the scalar's RAW TEXT as a plain scalar — that's its purpose after `set_tag`/`resolve_as` chains (r[ext.resolve] "Useful after set_tag or when operating on raw scalars"). So `'123' | resolve` → integer `123` (quoting was presentation; resolve is explicit). If that feels too surprising, the alternative is honoring quoted style as string-forcing — but then `resolve` is a no-op on everything from input, which makes it useless. Go with raw-text resolution; document loudly.

- [ ] **Step 2: Implement** (append to `yg_natives()`)

```rust
// r[impl ext.schema]
("schema", v(0), |cv| bome(Ok(YamlVal::str(cv.0.data().ctx.schema_name())))),
// r[impl ext.resolve]
("resolve", v(0), |cv| {
    let schema = cv.0.data().ctx.config.tag_resolution;
    bome(cv.1.resolve_with(schema))
}),
// r[impl ext.resolve-as]
("resolve_as", v(1), |cv| { /* unary: parse schema name, resolve_with */ }),
// r[impl ext.document-index]
("document_index", v(0), |cv| {
    bome(Ok(YamlVal::from(cv.0.data().ctx.input_meta.borrow().doc_index)))
}),
// r[impl ext.file-index]
("file_index", v(0), |cv| {
    bome(Ok(YamlVal::from(cv.0.data().ctx.input_meta.borrow().file_index)))
}),
// r[impl ext.filename]
("filename", v(0), |cv| {
    bome(Ok(cv.0.data().ctx.input_meta.borrow().filename.as_deref()
        .map_or_else(YamlVal::null, |f| YamlVal::str(f.to_owned()))))
}),
// deviation #3 (spec r[ext.explode] renamed)
("explode_aliases", v(0), |cv| bome(Ok(cv.1.explode_aliases()))),
// r[impl ext.aliases]
("aliases", v(0), |cv| bome(Ok(cv.1.alias_report()))),
// r[impl ext.orphan-anchors]
("orphan_anchors", v(0), |cv| bome(Ok(cv.1.orphan_anchors()))),
```

value.rs / funs.rs helpers:

```rust
impl YamlVal {
    /// Re-resolve this scalar's text under `schema`. Errors on collections.
    pub(crate) fn resolve_with(&self, schema: yamalgam_core::TagResolution) -> jaq_core::ValR<Self> {
        use yamalgam_core::TagResolver as _;
        match &self.data {
            Data::Seq(_) | Data::Map(_) => Err(jaq_core::Error::typ(self.clone(), "scalar")),
            _ => {
                let text = match (&self.data, self.meta().and_then(|m| m.raw.as_deref())) {
                    (_, Some(raw)) => raw.to_owned(),
                    (Data::Str(s), None) => s.to_string(),
                    _ => self.to_string(),
                };
                Ok(Self {
                    data: crate::build::core_value_to_data(schema.resolve_scalar(&text)),
                    meta: None,
                })
            }
        }
    }

    /// Deep-copy with alias/anchor identity stripped. Deviation #3.
    #[must_use]
    pub(crate) fn explode_aliases(&self) -> Self { /* walk, clear anchor+alias_of via meta_mut, recurse via map_values-style traversal on data */ }

    /// {anchor: [path, ...]} of alias uses. r[ext.aliases]
    #[must_use]
    pub(crate) fn alias_report(&self) -> Self { /* DFS with a path Vec<YamlVal>, collect alias_of hits */ }

    /// Anchors never referenced. r[ext.orphan-anchors]
    #[must_use]
    pub(crate) fn orphan_anchors(&self) -> Self { /* DFS twice or once with two sets */ }
}
```

(`core_value_to_data` moves from `build.rs` private fn to `pub(crate)`.)

`resolve_as` schema-name parsing: `"core" | "failsafe" | "json" | "1.1"` → TagResolution; unknown → runtime error listing valid names. r[datamodel.schema.document-declared.pragma] names are the same — reuse this parser in M11+ when the pragma lands.

- [ ] **Step 3: Run tests, commit**

Run: `cargo nextest run -p yamalgam-query`
Expected: PASS.

```bash
git add crates/yamalgam-query
git commit -m "feat(query): schema, input-context, and alias-audit filters"
```

---

## Task 16: CLI — yg rename and clap surface

**Files:**
- Modify: `crates/yamalgam/Cargo.toml`
- Modify: `crates/yamalgam/src/lib.rs`
- Modify: `crates/yamalgam/src/main.rs`
- Test: existing `crates/yamalgam/tests/cli.rs` updated

- [ ] **Step 1: Rename the binary**

In `crates/yamalgam/Cargo.toml`:

```toml
[[bin]]
name = "yg"
path = "src/main.rs"
```

Add the dependency:

```toml
yamalgam-query = { path = "../yamalgam-query" }
```

Then sweep for hardcoded binary-name references:

Run: `/usr/bin/grep -rn 'cargo_bin("yamalgam")\|target/release/yamalgam\b\|bin/yamalgam\b' crates xtask .github .justfile dist scripts 2>/dev/null`

Update every hit (assert_cmd `cargo_bin("yg")`, xtask hyperfine bench path, CI workflows, completion/manpage generation if librebar wires it to the bin name). Run the sweep AGAIN after fixing; expect zero hits.

- [ ] **Step 2: Extend Cli with query args**

In `crates/yamalgam/src/lib.rs` (alongside existing `Cli`):

```rust
/// Query-mode arguments. r[cli]
#[derive(clap::Args, Debug, Default)]
pub struct QueryArgs {
    /// The yg filter expression (defaults to `.` — identity/pretty-print).
    pub filter: Option<String>,
    /// Input files (stdin if none). r[cli.usage]
    pub files: Vec<camino::Utf8PathBuf>,

    /// Run filter once with null input. r[cli.null-input]
    #[arg(short = 'n', long)]
    pub null_input: bool,
    /// Read all documents into one sequence, run filter once. r[cli.slurp]
    #[arg(short = 's', long)]
    pub slurp: bool,
    /// Read each input line as a string. r[cli.raw-input]
    #[arg(short = 'R', long)]
    pub raw_input: bool,
    /// Read the filter from a file. r[cli.from-file]
    #[arg(short = 'f', long, value_name = "FILE")]
    pub from_file: Option<camino::Utf8PathBuf>,

    /// Output JSON (implies alias expansion). r[cli.json-output]
    #[arg(short = 'j', long)]
    pub json_output: bool,
    /// Output raw strings without quotes. r[cli.raw-output]
    #[arg(short = 'r', long)]
    pub raw_output: bool,
    /// Compact (single-line) output. r[cli.compact]
    #[arg(short = 'c', long)]
    pub compact_output: bool,
    /// Edit files in place. r[cli.inplace]
    #[arg(short = 'i', long, requires = "files")]
    pub in_place: bool,
    /// Sort mapping keys in output. r[cli.sort-keys]
    #[arg(short = 'S', long)]
    pub sort_keys: bool,
    /// Force colored output. r[cli.color]
    #[arg(short = 'C', long, conflicts_with = "monochrome_output")]
    pub color_output: bool,
    /// Disable colored output. r[cli.color]
    #[arg(short = 'M', long)]
    pub monochrome_output: bool,
    /// Suppress `---` document separators. r[output.multidoc.suppress]
    #[arg(short = 'N', long)]
    pub no_doc: bool,

    /// Schema for plain-scalar resolution. r[cli.schema-flag]
    #[arg(long, value_enum, default_value_t = SchemaArg::Core)]
    pub schema: SchemaArg,
    /// Generous resource limits (trusted local input). r[cli.trusted]
    #[arg(long, conflicts_with = "strict")]
    pub trusted: bool,
    /// Tight resource limits (untrusted input). r[cli.strict]
    #[arg(long)]
    pub strict: bool,

    /// Exit 1 when the last output is false/null, 4 on no output. r[cli.exit.falsy]
    #[arg(short = 'e', long)]
    pub exit_status: bool,
}

/// `--schema` values. r[datamodel.schema]
#[derive(clap::ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SchemaArg {
    /// YAML 1.2 core (default). r[datamodel.schema.default]
    #[default]
    Core,
    /// All plain scalars are strings. r[datamodel.schema.failsafe]
    Failsafe,
    /// JSON typing rules. r[datamodel.schema.json]
    Json,
    /// Legacy YAML 1.1 booleans — emits a warning. r[datamodel.schema.yaml11]
    #[value(name = "1.1")]
    Yaml11,
}
```

Wire into `Cli`: flatten `QueryArgs` so `yg '.a' file.yml` works WITHOUT a subcommand while `yg doctor`/`yg info` still dispatch. Clap resolves a leading positional that matches a subcommand name AS the subcommand — a filter literally named `doctor` needs `yg '.doctor'` or `yg -- doctor`; acceptable, document in --help epilog. Check `librebar`'s `command()` helper still wraps cleanly (the manual-composition landmine from the handoff: do NOT collapse main.rs into librebar's builder).

- [ ] **Step 3: main.rs dispatch**

```rust
match cli.command {
    Some(Commands::Doctor(args)) => commands::doctor::cmd_doctor(&config, &args),
    Some(Commands::Info(args)) => commands::info::cmd_info(&config, &args),
    None => commands::query::cmd_query(&config, &cli.query),
}
```

`cmd_query` returns a `std::process::ExitCode` (Task 18 defines the mapping); reconcile with main's current return type (librebar diagnostics may want `anyhow::Result` — return `Result<ExitCode>` and let main convert).

- [ ] **Step 4: Verify help + existing tests**

Run: `cargo run -p yamalgam --bin yg -- --help`
Expected: usage shows `yg [OPTIONS] [FILTER] [FILES]...` plus subcommands.

Run: `cargo nextest run -p yamalgam`
Expected: existing CLI tests pass with `cargo_bin("yg")`.

- [ ] **Step 5: Commit**

```bash
git add crates/yamalgam xtask .github .justfile
git commit -m "feat(cli): rename binary to yg, add query argument surface"
```

---

## Task 17: CLI input pipeline

**Files:**
- Create: `crates/yamalgam/src/commands/query.rs`
- Modify: `crates/yamalgam/src/commands/mod.rs`
- Test: `crates/yamalgam/tests/yg.rs` (new, assert_cmd)

- [ ] **Step 1: Tests (subprocess level)**

```rust
// tests/yg.rs
use assert_cmd::Command;

fn yg() -> Command {
    Command::cargo_bin("yg").unwrap()
}

#[test]
fn stdin_identity() {
    yg().arg(".").write_stdin("a: 1\n").assert().success().stdout("a: 1\n");
}

#[test]
fn default_filter_is_identity() {
    // r[cli.usage]
    yg().write_stdin("a: 1\n").assert().success().stdout("a: 1\n");
}

#[test]
fn files_process_as_unified_stream() {
    // r[cli.multi-file]
    let dir = tempfile::tempdir().unwrap();
    let f1 = dir.path().join("1.yml");
    let f2 = dir.path().join("2.yml");
    std::fs::write(&f1, "1\n").unwrap();
    std::fs::write(&f2, "2\n").unwrap();
    yg().arg(".").arg(&f1).arg(&f2).assert().success().stdout("1\n---\n2\n");
}

#[test]
fn null_input() {
    // r[cli.null-input]
    yg().args(["-n", "1 + 1"]).assert().success().stdout("2\n");
}

#[test]
fn slurp_collects_all_documents() {
    // r[cli.slurp]
    yg().args(["-s", "length"]).write_stdin("---\n1\n---\n2\n").assert().success().stdout("2\n");
}

#[test]
fn raw_input_reads_lines() {
    // r[cli.raw-input]
    yg().args(["-R", "."]).write_stdin("plain text\n").assert().success().stdout("plain text\n");
}

#[test]
fn filter_from_file() {
    // r[cli.from-file]
    let dir = tempfile::tempdir().unwrap();
    let prog = dir.path().join("prog.jq");
    std::fs::write(&prog, ".a\n").unwrap();
    yg().arg("-f").arg(&prog).write_stdin("a: 5\n").assert().success().stdout("5\n");
}

#[test]
fn schema_flag_changes_resolution() {
    // r[cli.schema-flag] r[datamodel.schema.yaml11]
    yg().args(["--schema", "1.1", ".a | type"])
        .write_stdin("a: yes\n")
        .assert()
        .success()
        .stdout("boolean\n")
        .stderr(predicates::str::contains("YAML 1.1"));
    yg().args(["--schema", "failsafe", ".a | type"])
        .write_stdin("a: 5\n")
        .assert()
        .success()
        .stdout("string\n");
}
```

- [ ] **Step 2: Implement cmd_query input side**

```rust
//! The yg query pipeline: inputs → engine → output.

use std::io::Read as _;
use std::process::ExitCode;
use std::rc::Rc;

use yamalgam_core::{LoaderConfig, TagResolution};
use yamalgam_query::engine::{self, RunCtx};
use yamalgam_query::YamlVal;

use crate::{QueryArgs, SchemaArg};

pub fn cmd_query(args: &QueryArgs) -> anyhow::Result<ExitCode> {
    let config = loader_config(args);
    if args.schema == SchemaArg::Yaml11 {
        eprintln!("warning: YAML 1.1 schema is a compatibility mode (yes/no/on/off are booleans)");
    }
    let (filter_src, files) = filter_and_files(args)?;
    let filter = match engine::compile_filter(&filter_src, &[]) {
        Ok(f) => f,
        Err(e) => return report(e),
    };
    let ctx = RunCtx::new(config);
    // ... assemble inputs (below), then Task 18's output loop
    todo!("Task 18 completes the output side")
}

/// `-f FILE` makes the filter positional an input file (jq parity).
fn filter_and_files(args: &QueryArgs) -> anyhow::Result<(String, Vec<camino::Utf8PathBuf>)> {
    match &args.from_file {
        Some(path) => {
            let src = std::fs::read_to_string(path)?;
            let mut files = Vec::new();
            files.extend(args.filter.clone().map(camino::Utf8PathBuf::from));
            files.extend(args.files.iter().cloned());
            Ok((src, files))
        }
        None => Ok((
            args.filter.clone().unwrap_or_else(|| ".".into()),
            args.files.clone(),
        )),
    }
}

fn loader_config(args: &QueryArgs) -> LoaderConfig {
    let base = if args.strict {
        LoaderConfig::strict() // r[cli.strict]
    } else if args.trusted {
        LoaderConfig::trusted() // r[cli.trusted]
    } else {
        LoaderConfig::moderate()
    };
    base.with_tag_resolution(match args.schema {
        SchemaArg::Core => TagResolution::Yaml12,
        SchemaArg::Failsafe => TagResolution::Failsafe,
        SchemaArg::Json => TagResolution::Json,
        SchemaArg::Yaml11 => TagResolution::Yaml11,
    })
}
```

Input assembly (the interesting part — one lazy iterator over all files/docs that UPDATES `ctx.input_meta` as it yields, so `filename`/`document_index` track):

```rust
/// All inputs as engine-ready values. Eagerly parses each FILE on first touch
/// (parse_str returns Vec per file — M10 accepts whole-file parsing; the
/// event stream stays lazy inside the parser, and files are bounded by
/// LoaderConfig limits anyway).
fn collect_inputs(
    files: &[camino::Utf8PathBuf],
    args: &QueryArgs,
    ctx: &RunCtx,
    config: &LoaderConfig,
) -> anyhow::Result<Vec<Result<YamlVal, String>>> { ... }
```

Implementation rules:
- stdin (no files): read to string; `filename = None`, `file_index = 0`.
- `-R`: each LINE is a `YamlVal::str` input (with `-s`: the whole text as ONE string).
- normal: `parse_str_with_config` per file; doc_index per document, reset per file; parse errors become `Err(rendered)` entries so the engine reports them at consumption time with exit 5 (matches jq's lazy input failure).
- `-s`: wrap everything in one `YamlVal::seq` (after `-R` line handling if both).
- `-n`: inputs become `once(Ok(YamlVal::null()))` BUT the real inputs stay reachable via the `inputs` filter — pass the real iterator to `YgData.inputs` and the null as the driver (mirror `jaq-all/src/data.rs::run`). For M10 simplicity: `run_filter` gains a `null_input: bool` parameter implementing exactly jaq's split (`data.inputs` vs driver iterator).

Because values keep `input_meta` in sync only while LAZILY consumed, don't pre-collect into a Vec if you can avoid it — make `collect_inputs` return an ITERATOR that sets `ctx.input_meta` in its `next()` (a `std::iter::from_fn` over a `VecDeque` of (meta, value) pairs is the simple honest version: parse eagerly, meta-stamp lazily). Eager parse + lazy meta-stamp is the M10 contract; document it.

- [ ] **Step 3: Run tests** (they'll pass once Task 18's output loop lands — run what compiles, commit the input layer)

```bash
git add crates/yamalgam
git commit -m "feat(cli): yg input pipeline (stdin, files, -n/-s/-R/-f, schema flags)"
```

---

## Task 18: CLI output pipeline and exit codes

**Files:**
- Modify: `crates/yamalgam/src/commands/query.rs`
- Test: `crates/yamalgam/tests/yg.rs` (append)

- [ ] **Step 1: Tests**

```rust
#[test]
fn json_output_explodes_and_is_json() {
    // r[cli.json-output] r[output.json]
    yg().args(["-j", "."])
        .write_stdin("a: &x [1]\nb: *x\n")
        .assert()
        .success()
        .stdout("{\n  \"a\": [\n    1\n  ],\n  \"b\": [\n    1\n  ]\n}\n");
}

#[test]
fn raw_output_unquotes_strings() {
    // r[cli.raw-output]
    yg().args(["-r", ".a"]).write_stdin("a: 'hello: world'\n").assert().success().stdout("hello: world\n");
}

#[test]
fn compact_output_single_line() {
    // r[cli.compact]
    yg().args(["-c", "."]).write_stdin("a:\n  - 1\n").assert().success().stdout("{a: [1]}\n");
}

#[test]
fn multidoc_separators_and_suppression() {
    // r[output.multidoc] r[output.multidoc.suppress]
    yg().args([".[]"]).write_stdin("[1, 2]\n").assert().success().stdout("1\n---\n2\n");
    yg().args(["-N", ".[]"]).write_stdin("[1, 2]\n").assert().success().stdout("1\n2\n");
}

#[test]
fn exit_codes() {
    // r[cli.exit.success]
    yg().arg(".").write_stdin("1\n").assert().code(0);
    // r[cli.exit.falsy]
    yg().args(["-e", ".a"]).write_stdin("a: false\n").assert().code(1);
    yg().args(["-e", ".missing"]).write_stdin("a: 1\n").assert().code(1);
    // r[cli.exit.usage] — filter syntax error
    yg().arg(".a |").write_stdin("1\n").assert().code(2);
    // r[cli.exit.compile]
    yg().arg("nosuchfilter").write_stdin("1\n").assert().code(3);
    // deviation #9: no output with -e
    yg().args(["-e", "empty"]).write_stdin("1\n").assert().code(4);
    // r[cli.exit.runtime] — type error and input parse error
    yg().arg(".a + true").write_stdin("a: 1\n").assert().code(5);
    yg().arg(".").write_stdin("a: [unclosed\n").assert().code(5);
}

#[test]
fn color_flags() {
    // r[cli.color] — -C forces ANSI even when piped; -M strips
    let out = yg().args(["-C", "."]).write_stdin("a: 1\n").assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("\x1b["), "expected ANSI escapes with -C");
    yg().args(["-M", "."]).write_stdin("a: 1\n").assert().success().stdout("a: 1\n");
}
```

- [ ] **Step 2: Implement the output loop + exit mapping**

```rust
/// Map results to spec exit codes. r[cli.exit]
fn exit_for(e: &yamalgam_query::YgError) -> ExitCode {
    use yamalgam_query::YgError as E;
    match e {
        E::FilterSyntax(_) => ExitCode::from(2),
        E::FilterCompile(_) => ExitCode::from(3),
        E::InputParse(_) | E::Runtime(_) => ExitCode::from(5),
        E::Halt(n) => ExitCode::from((*n).clamp(0, 255) as u8),
    }
}
```

Output writer:

```rust
let opts = EmitOpts {
    compact: args.compact_output,
    sort_keys: args.sort_keys,
    color: use_color(args), // -C → true, -M → false, else stdout.is_terminal()
    ..EmitOpts::default()
};
let mut first = true;
let mut write_val = |v: YamlVal| -> std::io::Result<()> {
    if args.json_output {
        // r[cli.json-output]: JSON per value, newline-separated
        writeln!(out, "{}", to_json(&v, &opts))
    } else if args.raw_output && v.is_string() {
        // r[cli.raw-output]
        writeln!(out, "{}", v.as_str_content().unwrap())
    } else {
        // r[output.multidoc]: `---` between values
        if !first && !args.no_doc {
            writeln!(out, "---")?;
        }
        first = false;
        write!(out, "{}", to_yaml(&v, &opts))
    }
};
```

(Add `is_string()` / `as_str_content()` accessors to YamlVal — trivial, documented, public.)

`-e` mapping after the run: `Some(true)` → 0, `Some(false)` → 1, `None` → 4 (deviation #9). Without `-e`: always 0 on success.

Color: implement in the YAML emitter as `Style` wrappers when `opts.color` (keys cyan, strings green, numbers+bools yellow, null dim, anchors/tags magenta — owo-colors is already a CLI dep; emitter shouldn't depend on it → use raw ANSI constants in `emit/yaml.rs` behind `opts.color`, they're 5 escape strings, no dep). JSON output: skip colors in M10 (jq colors JSON; defer — note in --help).

`-i` is Task 22. Until then `-i` errors "not yet implemented" (exit 2) with a test asserting that, replaced in Task 22.

- [ ] **Step 3: Run full CLI suite, commit**

Run: `cargo nextest run -p yamalgam`
Expected: PASS, including all Task 17 tests now that the loop exists.

```bash
git add crates/yamalgam
git commit -m "feat(cli): yg output pipeline, exit codes, color"
```

---

## Task 19: jq-manual compliance harness

**Files:**
- Create: `crates/yamalgam-query/tests/fixtures/jq-manual.yml` (vendored)
- Create: `crates/yamalgam-query/tests/fixtures/manual-allowlist.yml`
- Create: `crates/yamalgam-query/tests/jq_manual.rs`

Phase 1's test strategy (r[phases.core]): every program/input/output triple from the jq manual that operates on JSON-compatible values MUST pass, with documented divergences. Same allowlist discipline as the YAML Test Suite compliance harness: failures are either fixed or allowlisted WITH A REASON; unexpected failures break the build; unexpected PASSES of allowlisted entries break the build too (forces allowlist hygiene).

- [ ] **Step 1: Vendor the manual**

```bash
curl -fsSL https://raw.githubusercontent.com/jqlang/jq/master/docs/content/manual/manual.yml \
  -o crates/yamalgam-query/tests/fixtures/jq-manual.yml
```

Prepend an attribution header (jq docs are CC-BY-3.0; test-fixture vendoring with attribution):

```yaml
# Vendored from https://github.com/jqlang/jq (docs/content/manual/manual.yml)
# License: CC-BY-3.0 — used as a test corpus for jq-compatibility tracking.
# Commit: <fill with the fetched commit sha — curl the API or note the date>
```

Dogfood note: this file is parsed BY YAMALGAM (serde derive structs below). If it trips a parser bug, that's a finding — reduce and file it, don't swap in serde_yaml.

- [ ] **Step 2: The harness**

```rust
// tests/jq_manual.rs
//! jq-manual example triples run through the yg engine.
//! r[phases.core] test strategy.

use serde::Deserialize;
use yamalgam_core::LoaderConfig;
use yamalgam_query::engine::{self, RunCtx};

#[derive(Deserialize)]
struct Manual {
    sections: Vec<Section>,
}

#[derive(Deserialize)]
struct Section {
    #[serde(default)]
    entries: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    #[serde(default)]
    examples: Vec<Example>,
    // entries nest: subsections carry their own entries in some manual versions;
    // add #[serde(default)] entries: Vec<Entry> if the vendored file has them.
}

#[derive(Deserialize)]
struct Example {
    program: String,
    input: String,
    output: Vec<String>,
}

#[derive(Deserialize)]
struct Allowlist {
    /// program → reason. Failures listed here are EXPECTED.
    expected_failures: std::collections::BTreeMap<String, String>,
    /// programs we don't attempt (nondeterministic, out-of-scope features).
    skip: std::collections::BTreeMap<String, String>,
}

fn eval_triple(program: &str, input: &str) -> Result<Vec<yamalgam_query::YamlVal>, String> {
    let filter = engine::compile_filter(program, &[]).map_err(|e| e.to_string())?;
    let ctx = RunCtx::new(LoaderConfig::moderate());
    let docs = yamalgam_query::parse_str(input).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    engine::run_filter(&filter, &ctx, docs.into_iter().map(Ok), |v| {
        out.push(v);
        Ok::<_, yamalgam_query::YgError>(())
    })
    .map_err(|e| e.to_string())?;
    Ok(out)
}

#[test]
fn jq_manual_triples() {
    let manual: Manual = yamalgam_serde::from_str(include_str!("fixtures/jq-manual.yml"))
        .expect("manual.yml parses (dogfood)");
    let allow: Allowlist = yamalgam_serde::from_str(include_str!("fixtures/manual-allowlist.yml"))
        .expect("allowlist parses");

    let mut unexpected_fail = Vec::new();
    let mut unexpected_pass = Vec::new();
    let (mut pass, mut fail, mut skipped) = (0u32, 0u32, 0u32);

    for ex in manual.sections.iter().flat_map(|s| &s.entries).flat_map(|e| &e.examples) {
        if allow.skip.contains_key(&ex.program) {
            skipped += 1;
            continue;
        }
        let expected: Result<Vec<_>, String> = ex
            .output
            .iter()
            .map(|o| {
                yamalgam_query::parse_str(o)
                    .map_err(|e| e.to_string())
                    .map(|mut d| d.remove(0))
            })
            .collect();
        let ok = match (eval_triple(&ex.program, &ex.input), expected) {
            (Ok(actual), Ok(expected)) => actual == expected, // resolved == (r[comparison.resolved])
            _ => false,
        };
        let allowlisted = allow.expected_failures.contains_key(&ex.program);
        match (ok, allowlisted) {
            (true, false) => pass += 1,
            (false, true) => fail += 1, // expected failure
            (false, false) => unexpected_fail.push(ex.program.clone()),
            (true, true) => unexpected_pass.push(ex.program.clone()),
        }
    }

    eprintln!("jq-manual: {pass} pass, {fail} expected-fail, {skipped} skipped");
    assert!(
        unexpected_fail.is_empty(),
        "unexpected failures (fix or allowlist with reason):\n{unexpected_fail:#?}"
    );
    assert!(
        unexpected_pass.is_empty(),
        "allowlisted entries now pass — remove from allowlist:\n{unexpected_pass:#?}"
    );
}
```

- [ ] **Step 3: Triage loop**

Run: `cargo nextest run -p yamalgam-query -E 'test(jq_manual)' --no-capture`

First run will dump failures. Triage every one into exactly one bucket:
1. **yg bug** → fix it (expect a batch: `from_num` edge cases, `range` semantics, `tostring` formatting, `limit/first/until` defs needing a native we missed). Special case: a jaq-std def that internally binds the SORTED `keys` (lexical scoping means our shadow doesn't retro-apply inside std). If one shows up (candidates: `pick`, `paths`-adjacent defs), copy that def into yg's defs.jq AFTER our `keys` so it rebinds — and note it in the implementation-notes doc.
2. **Documented divergence** → `expected_failures` with the spec rule as reason: `keys` order (r[diverge.keys]), `type` names (r[diverge.type]), float rendering (`1.0` stays `1.0` in YAML — expected-output parses as float, compare may still PASS via `==`; only string-typed expectations like `tostring` outputs diverge), mapping-order equality cases.
3. **Out of scope** → `skip` with reason: `$__loc__`, `input_line_number`, `now`/`localtime` (nondeterministic), `env`/`$ENV` (environment-dependent), `getpath/setpath` if jaq lacks them (it has them), SQL-style filters jq itself marks experimental, anything needing `--args`.

Seed `manual-allowlist.yml`:

```yaml
# jq-manual divergence tracking. Every entry needs a reason.
# unexpected-pass enforcement keeps this list honest.
expected_failures:
  "keys": "r[diverge.keys] — yg keys is insertion-ordered; jq sorts"
skip:
  "$__loc__": "no source-location vars in M10"
  "now": "nondeterministic"
```

(Exact entries emerge from the run; the two assert blocks make drift impossible.)

Acceptance bar for this task: **zero unexpected failures, every allowlist entry carries a spec-rule or scope reason.** Raw pass count goes in the handoff.

- [ ] **Step 4: Commit**

```bash
git add crates/yamalgam-query/tests
git commit -m "test(query): jq-manual compliance harness with divergence allowlist"
```

---

## Task 20: Assignment-path verification (spec Phase 2 entry)

**Files:**
- Test: `crates/yamalgam-query/tests/engine.rs` (append)

Assignment operators (r[filters.assign.*]) are jaq grammar + our `map_index`/`map_values`/`map_range` — already implemented. This task PROVES them through the engine and pins YAML-specific behavior (meta preservation on containers).

- [ ] **Step 1: Write the tests, fix what falls out**

```rust
#[test]
fn assignment_operators() {
    // r[filters.assign.plain]
    assert_eq!(eval(".a = 9", "a: 1\nb: 2\n"), vec!["{a: 9, b: 2}"]);
    // r[filters.assign.update]
    assert_eq!(eval(".a |= . + 1", "a: 1\n"), vec!["{a: 2}"]);
    // r[filters.assign.arithmetic]
    assert_eq!(eval(".a += 5", "a: 1\n"), vec!["{a: 6}"]);
    assert_eq!(eval(".a //= 9", "a: null\n"), vec!["{a: 9}"]);
    // nested paths
    assert_eq!(eval(".a.b.c = 1", "a: {}\n"), vec!["{a: {b: {c: 1}}}"]);
    // iteration assignment
    assert_eq!(eval(".[] |= . * 2", "[1, 2]\n"), vec!["[2, 4]"]);
    // del()
    assert_eq!(eval("del(.a)", "a: 1\nb: 2\n"), vec!["{b: 2}"]);
    // delete preserves order of remaining keys (shift_remove in map_index!)
    assert_eq!(eval("del(.b)", "a: 1\nb: 2\nc: 3\n"), vec!["{a: 1, c: 3}"]);
    // slice assignment
    assert_eq!(eval(".[1:2] = [9, 9]", "[1, 2, 3]\n"), vec!["[1, 9, 9, 3]"]);
}

#[test]
fn update_preserves_untouched_sibling_meta() {
    // Phase 2 goal (r[output.yaml.style-preservation] pre-CST): a filter that
    // touches .a must not destroy .b's style or the container's style.
    let src = "a: 1\nb: \"quoted\"\nflow: {x: 1}\n";
    let f = engine::compile_filter(".a = 2", &[]).unwrap();
    let ctx = RunCtx::new(LoaderConfig::moderate());
    let docs = yamalgam_query::parse_str(src).unwrap();
    let mut out = Vec::new();
    engine::run_filter(&f, &ctx, docs.into_iter().map(Ok), |v| {
        out.push(v);
        Ok::<_, yamalgam_query::YgError>(())
    })
    .unwrap();
    let emitted = yamalgam_query::emit::to_yaml(&out[0], &Default::default());
    assert_eq!(emitted, "a: 2\nb: \"quoted\"\nflow: {x: 1}\n");
}
```

That second test is the Phase 2 acceptance test in miniature. If it fails, the leak is in `map_index` (replacing the PAIR instead of the value, or `Rc::make_mut` cloning away meta) — fix there, not in the emitter.

- [ ] **Step 2: Commit**

```bash
git add crates/yamalgam-query
git commit -m "test(query): assignment operators preserve untouched metadata"
```

---

## Task 21: Mutation natives — set_tag, set_anchor, set_style, merge

**Files:**
- Modify: `crates/yamalgam-query/src/funs.rs`
- Test: `crates/yamalgam-query/tests/engine.rs` (append)

r[phases.mutation]: `set_tag`, `set_anchor`, `set_style`, `merge`. (`explode` shipped as `explode_aliases` in Task 15 — deviation #3.)

- [ ] **Step 1: Tests**

```rust
#[test]
fn set_metadata_filters() {
    // r[ext.set-tag]
    assert_eq!(eval(".a |= (set_tag(\"!special\")) | .a | tag", "a: 1\n"), vec!["!special"]);
    assert_eq!(eval("set_tag(null) | tag", "!x v\n"), vec!["null"]); // clear
    // r[ext.set-anchor]
    let out = eval(".a |= set_anchor(\"base\")", "a: {x: 1}\nb: 2\n");
    assert_eq!(out, vec!["{a: &base {x: 1}, b: 2}"]);
    // r[ext.set-style]
    assert_eq!(
        eval(".a |= set_style(\"double-quoted\")", "a: hi\n"),
        vec!["{a: \"hi\"}"]
    );
    assert_eq!(eval(".a |= set_style(\"flow\")", "a:\n  - 1\n"), vec!["{a: [1]}"]);
}

#[test]
fn set_style_rejects_kind_mismatch() {
    // r[ext.set-style] — collection styles on scalars are errors
    let f = engine::compile_filter("set_style(\"flow\")", &[]).unwrap();
    let ctx = RunCtx::new(LoaderConfig::moderate());
    let docs = yamalgam_query::parse_str("scalar\n").unwrap();
    let r = engine::run_filter(&f, &ctx, docs.into_iter().map(Ok), |_| {
        Ok::<_, yamalgam_query::YgError>(())
    });
    assert!(r.is_err());
}

#[test]
fn merge_expands_merge_keys() {
    // r[ext.merge]
    let doc = "base: &b {x: 1, y: 1}\nchild:\n  <<: *b\n  y: 2\n";
    assert_eq!(eval(".child | merge", doc), vec!["{y: 2, x: 1}"]);
    // ^ ORDER DECISION: explicit keys keep their position; merged-in keys
    // append at the merge-key's position in first-seen order. The expected
    // string above assumes explicit-first because y precedes << in... it
    // does NOT — << comes first in the doc. Pin the real rule:
    // merged entries insert AT the << position (x goes first), explicit
    // keys stay where written (y after), explicit WINS on conflict (y: 2).
    // Correct expectation: vec!["{x: 1, y: 2}"]. Fix the assert to that
    // and delete this comment block in the implementation.
}

#[test]
fn merge_sequence_earlier_wins() {
    // YAML merge spec: earlier mappings in the sequence take precedence.
    let doc = "a: &a {x: 1}\nb: &b {x: 2, y: 9}\nc:\n  <<: [*a, *b]\n";
    assert_eq!(eval(".c | merge", doc), vec!["{x: 1, y: 9}"]);
}
```

- [ ] **Step 2: Implement** (append natives; helpers on YamlVal)

```rust
// r[impl ext.set-tag]
("set_tag", v(1), |cv| { /* unary: null clears, string sets (accept "!!x" → expand to tag:yaml.org,2002:x, keep others verbatim) */ }),
// r[impl ext.set-anchor]
("set_anchor", v(1), |cv| { /* unary: null clears; validates anchor charset [A-Za-z0-9_-]+ */ }),
// r[impl ext.set-style]
("set_style", v(1), |cv| { /* unary: style_from_name + kind check */ }),
// r[impl ext.merge]
("merge", v(0), |cv| bome(cv.1.merge_keys())),
```

```rust
impl YamlVal {
    /// Expand `<<` merge keys one level. r[ext.merge]
    pub(crate) fn merge_keys(&self) -> jaq_core::ValR<Self> {
        let Data::Map(m) = &self.data else {
            return Err(jaq_core::Error::typ(self.clone(), "mapping"));
        };
        let mut out: indexmap::IndexMap<Self, Self> = indexmap::IndexMap::default();
        let mut merged: Vec<(Self, Self)> = Vec::new(); // defaults, first-seen order
        for (k, v) in &m.0 {
            let is_merge = k.tag_shorthand().as_deref() == Some("!!merge");
            if !is_merge {
                out.insert(k.clone(), v.clone());
                continue;
            }
            // value: mapping, or sequence of mappings (earlier wins)
            let sources: Vec<&Self> = match &v.data {
                Data::Map(_) => vec![v],
                Data::Seq(s) => s.iter().collect(),
                _ => return Err(jaq_core::Error::typ(v.clone(), "mapping or sequence of mappings")),
            };
            for src in sources {
                let Data::Map(sm) = &src.data else {
                    return Err(jaq_core::Error::typ(src.clone(), "mapping"));
                };
                for (mk, mv) in &sm.0 {
                    if !merged.iter().any(|(k, _)| k == mk) {
                        merged.push((mk.clone(), mv.clone()));
                    }
                }
            }
            // splice defaults at the << position, explicit keys override later
            for (mk, mv) in merged.drain(..) {
                out.entry(mk).or_insert(mv);
            }
        }
        // explicit keys AFTER << must override merged defaults:
        // (the loop above inserts in document order, and IndexMap::insert on an
        // existing key REPLACES value but KEEPS original position — so an
        // explicit key written after << overrides the merged value while the
        // merged position wins. That matches "<<: provides defaults".)
        Ok(Self {
            data: Data::Map(std::rc::Rc::new(crate::value::YgMap(out))),
            meta: self.meta.clone(),
        })
    }
}
```

(The position-vs-override subtlety in the comment is REAL — write the two tests first, let them drive the exact insert/entry calls. The invariants: explicit wins on value; merged entries appear at the `<<` position; earlier sequence elements win among themselves. r[ext.merge])

`set_style` style names: reuse `style_name()`'s vocabulary in reverse (`"plain"|"single-quoted"|"double-quoted"|"literal"|"folded"` for scalars, `"block"|"flow"` for collections); mismatched kind → runtime error.

- [ ] **Step 3: Run tests, commit**

Run: `cargo nextest run -p yamalgam-query`
Expected: PASS.

```bash
git add crates/yamalgam-query
git commit -m "feat(query): mutation natives — set_tag/set_anchor/set_style/merge"
```

---

## Task 22: In-place editing (-i)

**Files:**
- Modify: `crates/yamalgam/src/commands/query.rs`
- Modify: `crates/yamalgam/Cargo.toml` (tempfile moves from dev-deps to deps)
- Test: `crates/yamalgam/tests/yg.rs` (append)

r[cli.inplace] with r[output.yaml.style-preservation]'s pre-CST caveat: best-effort re-serialization (metadata-honoring emitter = Tasks 10/11/20 already carry most styles through; comments are LOST until Phase 3 — document that loudly in --help and README).

- [ ] **Step 1: Tests**

```rust
#[test]
fn in_place_edits_file() {
    // r[cli.inplace]
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("cfg.yml");
    std::fs::write(&f, "a: 1\nb: \"keep\"\n").unwrap();
    yg().args(["-i", ".a = 2"]).arg(&f).assert().success().stdout("");
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "a: 2\nb: \"keep\"\n");
}

#[test]
fn in_place_multi_document_file() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("multi.yml");
    std::fs::write(&f, "a: 1\n---\na: 2\n").unwrap();
    yg().args(["-i", ".a |= . + 10"]).arg(&f).assert().success();
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "a: 11\n---\na: 12\n");
}

#[test]
fn in_place_requires_files() {
    yg().args(["-i", "."]).write_stdin("a: 1\n").assert().code(2);
}

#[test]
fn in_place_preserves_permissions() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("x.yml");
    std::fs::write(&f, "a: 1\n").unwrap();
    std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o600)).unwrap();
    yg().args(["-i", "."]).arg(&f).assert().success();
    assert_eq!(std::fs::metadata(&f).unwrap().permissions().mode() & 0o777, 0o600);
}
```

- [ ] **Step 2: Implement**

Port jaq's approach (`ref/jaq/jaq/src/main.rs:154-169`): per file — run the filter over that file's documents, write outputs to a `tempfile::Builder` tempfile IN THE SAME DIRECTORY (atomic rename safety), then `tmp.persist(path)` and restore the original permissions. `-i` forces per-file processing (no cross-file stream) and suppresses stdout. Combine with `-j` freely (jq allows it; the file becomes JSON — the user asked).

`clap`'s `requires = "files"` (Task 16) handles the no-files case at parse time → exit 2 automatically. Verify the test agrees with clap's actual error code (clap errors exit 2 via our mapping of usage errors — wire `clap::Error` → exit 2 explicitly in main).

- [ ] **Step 3: Run tests, commit**

Run: `cargo nextest run -p yamalgam`
Expected: PASS.

```bash
git add crates/yamalgam
git commit -m "feat(cli): in-place editing with best-effort style preservation"
```

---

## Task 23: Docs, annotation sweep, final check

**Files:**
- Modify: `README.md` (yg usage section replaces the pre-release stub's CLI mention)
- Modify: `record/superpowers/plans/README.md` (M10 status → Complete, with deviations summary)
- Modify: `docs/spec/yg-query-language-spec-draft-01.md` — NO. Spec edits are Clay's; instead:
- Create: `docs/spec/yg-spec-draft-01-implementation-notes.md` (the 11 deviations, fleshed out, for the draft-02 revision)
- Test: full gate

- [ ] **Step 1: Implementation-notes doc**

Move the "Spec deviations" section of this plan into `docs/spec/yg-spec-draft-01-implementation-notes.md`, expanded with what actually happened (final jq-manual pass counts, allowlist size, any decisions that changed during execution). One section per deviation, each citing the spec rule it touches.

- [ ] **Step 2: README**

Add a `## yg` section: one-paragraph positioning (straight from r[overview.positioning]: "yg is NOT jq for YAML — it's a YAML processor whose expression language is jq"), a 6-line example block (identity, field access, keys insertion-order, -j escape hatch, --schema, -i), and the comment-loss caveat for -i pre-CST.

- [ ] **Step 3: Annotation sweep**

Run: `/usr/bin/grep -rn 'r\[impl' crates/yamalgam-query/src crates/yamalgam/src | wc -l`

Walk spec sections 4, 5, 6.1–6.4, 6.6–6.7, 8, 9 and confirm each shipped rule has an `r[impl ...]` annotation at its implementation site (the task code blocks above seeded most). Add missing ones. Rules NOT shipped (comments filters, include/ref, validate, ===) get NO annotation — tracey's coverage report is the truth, not a goal of 100%.

- [ ] **Step 4: Full gate**

Run: `just check`
Expected: fmt clean, clippy (nursery!) clean, deny clean, all tests pass, doc-tests pass. Budget real time for nursery lints on the new crates (`option_if_let_else`, `missing_const_for_fn`, `or_fun_call` WILL fire on first drafts).

Run: `cargo nextest run --workspace --all-features 2>&1 | tail -5`
Record the new total test count for the handoff.

- [ ] **Step 5: Handoff + commit.txt, then STOP**

Write `.handoffs/<TZ=America/New_York date +%Y-%m-%d-%H%M>-m10-yg-cli-complete.md` (state, decisions, jq-manual numbers, what M11 picks up). Write `commit.txt` (conventional, `feat(cli): ...` with Release-Note + Release-Impact trailers per scrat config). **HARD STOP after commit.txt** — Clay runs `gtxt && git pm && gsm && git up`.

---

## Execution notes

- **Branch:** `m10-yg-cli` (already created). Commits land per-task on the branch; `git pm` squash-merges the lot — one PR, one logical change.
- **Order matters:** Tasks 1→8 are strictly sequential (each builds on the last). 9–11 (emitters) and 12–15 (engine) could interleave, but tojson (Task 13) needs the JSON emitter (Task 9) — keep plan order unless blocked.
- **When the plan's code disagrees with jaq-core 3.0's real API, jaq wins.** The sketches were written against `ref/jaq` (master ≈ 3.0); `ref/jaq/jaq-json` and `ref/jaq/jaq-all` are compiling reference implementations of everything we're adapting — read them before fighting the borrow checker.
- **Scanner/parser internals are OFF LIMITS.** If a builder test exposes a parser bug, reduce it, note it, allowlist around it (the `? key` gap is already known). Read `memory/scanner-parser-gotchas.md` before even thinking about touching those crates.
- **Scope discipline:** no comment filters, no include/ref, no validate, no `===`, no pragma, no `--arg`. They have milestones. The allowlist + implementation-notes doc are the pressure valve — divergence documented beats scope creep shipped.

## Self-review checklist (run after writing, before execution)

- [x] Spec coverage: §2 (data model → Tasks 2–4), §3.1/3.3 (Task 3), §3.2 (deviation #2, documented not shipped), §4 (jaq + Tasks 7/12/20), §5 (Tasks 12/13 + defs.jq), §6.1 (Tasks 14/21), §6.2 (Task 14), §6.3 (Task 15), §6.4 (Task 15), §6.5 (Phase 3 — out), §6.6 (Tasks 4/21), §6.7 (Task 15), §7 (M11 — out), §8 (Tasks 16–18, 22; §8.4 include/ref flags → deviation #10), §9 (Tasks 9–11, 18), §10 (M12 — out), §11 Phases 1–2 (whole plan).
- [x] No TBD/TODO placeholders that hide work (the two `todo!`/sketch markers in Tasks 17/15 are explicitly completed by named later steps or carry full porting instructions with line-number references).
- [x] Type consistency: `YamlVal::{null,int,float,str,seq,map}`, `Data`, `Meta`, `YgMap`, `RunCtx`, `YgData`, `YgKind`, `EmitOpts`, `compile_filter`, `run_filter`, `parse_str`, `parse_str_with_config` used uniformly across tasks.


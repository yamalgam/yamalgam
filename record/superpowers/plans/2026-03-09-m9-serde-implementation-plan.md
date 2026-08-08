# M9: Streaming serde Deserializer — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `yamalgam::from_str::<T>()` as a streaming serde Deserializer that consumes the parser event stream without materializing a Value DOM.

**Architecture:** Three-phase delivery: (1) extract Composer and CST into standalone crates; (2) implement streaming serde Deserializer with erased-serde, anchor buffering, multi-document support; (3) integration tests against real-world YAML and yamllint test corpora.

**Tech Stack:** serde 1.0, erased-serde 0.4, thiserror 2.0, yamalgam-parser (event stream), yamalgam-core (Value, TagResolver, LoaderConfig)

**Design doc:** `record/superpowers/plans/2026-03-09-streaming-serde-deserializer-design.md`

---

## Phase 1: Crate Extraction

### Task 1: Create yamalgam-compose crate

Extract the Composer (events → Value DOM) from yamalgam-parser into its own crate.

**Files:**
- Create: `crates/yamalgam-compose/Cargo.toml`
- Create: `crates/yamalgam-compose/src/lib.rs` (moved from `crates/yamalgam-parser/src/compose.rs`)
- Modify: `crates/yamalgam-parser/src/lib.rs` (remove compose module + convenience fns)
- Move: `crates/yamalgam-parser/tests/compose.rs` → `crates/yamalgam-compose/tests/compose.rs`

**Step 1: Scaffold the crate**

Create `crates/yamalgam-compose/Cargo.toml`:

```toml
[package]
name = "yamalgam-compose"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "YAML event stream to Value DOM composer"
publish = false

[dependencies]
thiserror = "2.0"
yamalgam-core = { path = "../yamalgam-core" }
yamalgam-parser = { path = "../yamalgam-parser" }
yamalgam-scanner = { path = "../yamalgam-scanner" }

[dev-dependencies]
pretty_assertions = "1"

[lints]
workspace = true
```

**Step 2: Move compose.rs → yamalgam-compose/src/lib.rs**

Copy the full contents of `crates/yamalgam-parser/src/compose.rs` into `crates/yamalgam-compose/src/lib.rs`. Update imports:

- `use crate::error::ParseError;` → `use yamalgam_parser::ParseError;`
- `use crate::event::Event;` → `use yamalgam_parser::Event;`
- `use crate::parser::Parser;` → `use yamalgam_parser::Parser;`
- `use crate::resolve::{...};` → `use yamalgam_parser::{NoopResolver, ResolveError, ResolvedEvents, Resolver};`

Add crate-level doc comment and `#![deny(unsafe_code)]`.

**Step 3: Move compose tests**

Move `crates/yamalgam-parser/tests/compose.rs` → `crates/yamalgam-compose/tests/compose.rs`. Update imports:

- `use yamalgam_parser::compose::Composer;` → `use yamalgam_compose::Composer;`
- `use yamalgam_parser::{...};` — split: keep `Event, ResolveError, Resolver` from yamalgam_parser, get `from_str, from_str_single` from yamalgam_compose
- Check each import and fix. The convenience functions `from_str`, `from_str_single` now live in yamalgam-compose.

**Step 4: Slim yamalgam-parser/src/lib.rs**

Remove from `crates/yamalgam-parser/src/lib.rs`:
- `pub mod compose;`
- `pub use compose::{ComposeError, Composer};`
- `use yamalgam_core::Value;`
- All four `from_str*` convenience functions (lines 37-75)

The parser crate now exports only: `Event`, `CollectionStyle`, `Parser`, `ParseError`, `Resolver`, `NoopResolver`, `ResolvedEvents`, `ResolveError`, `ScalarStyle`.

**Step 5: Verify**

Run: `just check`
Expected: All tests pass. The parser crate compiles without compose. The compose crate compiles with its own tests.

**Step 6: Commit**

```
feat: extract yamalgam-compose from yamalgam-parser

Move Composer, ComposeError, and convenience from_str functions into
a standalone yamalgam-compose crate. Parser crate now focused on
event production only.
```

---

### Task 2: Create yamalgam-cst crate

Extract the CST builder from yamalgam-parser into its own crate.

**Files:**
- Create: `crates/yamalgam-cst/Cargo.toml`
- Create: `crates/yamalgam-cst/src/lib.rs` (moved from `crates/yamalgam-parser/src/cst.rs`)
- Modify: `crates/yamalgam-parser/src/lib.rs` (remove cst module)
- Modify: `crates/yamalgam-compare/Cargo.toml` (add yamalgam-cst dep)
- Modify: `crates/yamalgam-compare/tests/cst_round_trip.rs` (update import)

**Step 1: Scaffold the crate**

Create `crates/yamalgam-cst/Cargo.toml`:

```toml
[package]
name = "yamalgam-cst"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Lossless concrete syntax tree for YAML"
publish = false

[dependencies]
yamalgam-core = { path = "../yamalgam-core" }
yamalgam-parser = { path = "../yamalgam-parser" }

[lints]
workspace = true
```

**Step 2: Move cst.rs → yamalgam-cst/src/lib.rs**

Copy the full contents of `crates/yamalgam-parser/src/cst.rs` into `crates/yamalgam-cst/src/lib.rs`. Update imports:

- `use crate::error::ParseError;` → `use yamalgam_parser::ParseError;`
- `use crate::event::Event;` → `use yamalgam_parser::Event;`
- `use crate::parser::Parser;` → `use yamalgam_parser::Parser;`

Add `#![deny(unsafe_code)]`.

**Step 3: Update yamalgam-parser/src/lib.rs**

Remove:
- `pub mod cst;`

**Step 4: Update yamalgam-compare**

In `crates/yamalgam-compare/Cargo.toml`, add:
```toml
yamalgam-cst = { path = "../yamalgam-cst" }
```

In `crates/yamalgam-compare/tests/cst_round_trip.rs`, update:
- `use yamalgam_parser::cst::parse_to_cst;` → `use yamalgam_cst::parse_to_cst;`

**Step 5: Verify**

Run: `just check`
Expected: All tests pass including cst_round_trip compliance tests.

**Step 6: Commit**

```
feat: extract yamalgam-cst from yamalgam-parser

Move CstNode, CstBuilder, parse_to_cst, and all CST types into a
standalone yamalgam-cst crate. Parser crate is now purely event
production: Event, Parser, Resolver.
```

---

### Task 3: Update downstream crate dependencies

Ensure all workspace consumers compile correctly after extraction.

**Files:**
- Modify: `crates/yamalgam/Cargo.toml` (no changes expected — CLI doesn't use compose/cst)
- Modify: `crates/yamalgam-bench/Cargo.toml` (no changes expected — bench only uses Parser)
- Check: `crates/yamalgam-compare/tests/compliance.rs` (uses Parser/Event — should be unchanged)

**Step 1: Verify all crates compile**

Run: `cargo check --workspace`

**Step 2: Run full check**

Run: `just check`
Expected: All 1202+ tests pass. No regressions.

**Step 3: Create PR**

PR title: `refactor: extract consumer crates (yamalgam-compose, yamalgam-cst)`

---

## Phase 2: Streaming serde Deserializer

### Task 4: Scaffold yamalgam-serde + Error type

**Files:**
- Create: `crates/yamalgam-serde/Cargo.toml`
- Create: `crates/yamalgam-serde/src/lib.rs`
- Create: `crates/yamalgam-serde/src/error.rs`
- Test: `crates/yamalgam-serde/tests/error.rs`

**Step 1: Write the failing test**

Create `crates/yamalgam-serde/tests/error.rs`:

```rust
use yamalgam_serde::Error;

#[test]
fn error_implements_serde_de_error() {
    // serde::de::Error requires custom() constructor
    let err = <Error as serde::de::Error>::custom("test error");
    assert_eq!(err.to_string(), "test error");
}

#[test]
fn error_implements_std_error() {
    let err = <Error as serde::de::Error>::custom("something went wrong");
    let _: &dyn std::error::Error = &err;
}

#[test]
fn error_from_parse_error() {
    // Verify From<ParseError> conversion exists
    let parse_err = yamalgam_parser::ParseError::UnexpectedEof {
        expected: "value".to_string(),
        span: yamalgam_core::Span::default(),
    };
    let err: Error = Error::Parse(parse_err);
    assert!(err.to_string().contains("unexpected end"));
}

#[test]
fn error_more_than_one_document() {
    let err = Error::MoreThanOneDocument;
    assert!(err.to_string().contains("more than one document"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo nextest run -p yamalgam-serde`
Expected: FAIL — crate doesn't exist yet.

**Step 3: Scaffold the crate**

Create `crates/yamalgam-serde/Cargo.toml`:

```toml
[package]
name = "yamalgam-serde"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Streaming serde Deserializer for YAML"
publish = false

[dependencies]
erased-serde = "0.4"
serde = "1.0"
thiserror = "2.0"
yamalgam-core = { path = "../yamalgam-core" }
yamalgam-parser = { path = "../yamalgam-parser" }

[dev-dependencies]
pretty_assertions = "1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[lints]
workspace = true
```

Create `crates/yamalgam-serde/src/error.rs`:

```rust
//! Deserialization error type.

use std::fmt;

use yamalgam_core::Span;
use yamalgam_parser::ParseError;
use yamalgam_parser::resolve::ResolveError;

/// Errors that can occur during YAML deserialization.
#[derive(Debug)]
pub enum Error {
    /// YAML parsing failed (scanner/parser level).
    Parse(ParseError),
    /// Resolver middleware error (!include, $ref).
    Resolve(ResolveError),
    /// Unexpected event during deserialization.
    Unexpected {
        /// What was expected.
        expected: &'static str,
        /// What was found.
        found: String,
        /// Source location, if available.
        span: Option<Span>,
    },
    /// Undefined alias reference.
    UndefinedAlias {
        /// The alias name.
        name: String,
        /// Source location, if available.
        span: Option<Span>,
    },
    /// Resource limit exceeded.
    LimitExceeded(String),
    /// Multiple documents found in single-document API.
    MoreThanOneDocument,
    /// Custom error from serde Deserialize impls.
    Custom(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "{e}"),
            Self::Resolve(e) => write!(f, "{e}"),
            Self::Unexpected { expected, found, .. } => {
                write!(f, "expected {expected}, found {found}")
            }
            Self::UndefinedAlias { name, .. } => write!(f, "undefined alias: *{name}"),
            Self::LimitExceeded(msg) => write!(f, "limit exceeded: {msg}"),
            Self::MoreThanOneDocument => write!(f, "more than one document in input"),
            Self::Custom(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(e) => Some(e),
            Self::Resolve(e) => Some(e),
            _ => None,
        }
    }
}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Custom(msg.to_string())
    }
}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Error::Parse(e)
    }
}

impl From<ResolveError> for Error {
    fn from(e: ResolveError) -> Self {
        Error::Resolve(e)
    }
}
```

Create `crates/yamalgam-serde/src/lib.rs`:

```rust
//! Streaming serde Deserializer for YAML.
//!
//! Consumes the yamalgam parser event stream directly — no intermediate DOM
//! materialization. Uses erased-serde internally so the parser never
//! monomorphizes.
#![deny(unsafe_code)]

mod error;

pub use error::Error;
```

**Step 4: Run test to verify it passes**

Run: `cargo nextest run -p yamalgam-serde`
Expected: All 4 error tests pass.

**Step 5: Commit**

```
feat(serde): scaffold yamalgam-serde crate with Error type
```

---

### Task 5: Scalar deserialization

**Files:**
- Create: `crates/yamalgam-serde/src/de.rs`
- Modify: `crates/yamalgam-serde/src/lib.rs`
- Test: `crates/yamalgam-serde/tests/scalars.rs`

**Step 1: Write the failing tests**

Create `crates/yamalgam-serde/tests/scalars.rs`:

```rust
use yamalgam_serde::from_str;

#[test]
fn deserialize_bool_true() {
    assert_eq!(from_str::<bool>("true").unwrap(), true);
}

#[test]
fn deserialize_bool_false() {
    assert_eq!(from_str::<bool>("false").unwrap(), false);
}

#[test]
fn deserialize_integer() {
    assert_eq!(from_str::<i64>("42").unwrap(), 42);
    assert_eq!(from_str::<i32>("-7").unwrap(), -7);
    assert_eq!(from_str::<u64>("0").unwrap(), 0);
}

#[test]
fn deserialize_float() {
    assert_eq!(from_str::<f64>("3.14").unwrap(), 3.14);
    assert!(from_str::<f64>(".nan").unwrap().is_nan());
    assert_eq!(from_str::<f64>(".inf").unwrap(), f64::INFINITY);
    assert_eq!(from_str::<f64>("-.inf").unwrap(), f64::NEG_INFINITY);
}

#[test]
fn deserialize_string() {
    assert_eq!(from_str::<String>("hello").unwrap(), "hello");
    assert_eq!(from_str::<String>("\"quoted\"").unwrap(), "quoted");
    assert_eq!(from_str::<String>("'single'").unwrap(), "single");
}

#[test]
fn deserialize_borrowed_str() {
    // Plain scalars can be borrowed
    let input = "hello";
    let result: &str = from_str(input).unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn deserialize_null() {
    assert_eq!(from_str::<()>("~").unwrap(), ());
    assert_eq!(from_str::<()>("null").unwrap(), ());
    assert_eq!(from_str::<()>("").unwrap(), ());
}

#[test]
fn deserialize_option_some() {
    assert_eq!(from_str::<Option<i64>>("42").unwrap(), Some(42));
}

#[test]
fn deserialize_option_none() {
    assert_eq!(from_str::<Option<i64>>("~").unwrap(), None);
    assert_eq!(from_str::<Option<i64>>("null").unwrap(), None);
}

#[test]
fn deserialize_quoted_string_not_bool() {
    // Quoted scalars are always strings — "true" is not a bool
    assert_eq!(from_str::<String>("\"true\"").unwrap(), "true");
    assert_eq!(from_str::<String>("\"42\"").unwrap(), "42");
}

#[test]
fn deserialize_hex_int() {
    assert_eq!(from_str::<i64>("0xFF").unwrap(), 255);
}

#[test]
fn deserialize_octal_int() {
    assert_eq!(from_str::<i64>("0o17").unwrap(), 15);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo nextest run -p yamalgam-serde --test scalars`
Expected: FAIL — `from_str` doesn't exist yet.

**Step 3: Implement the Deserializer core**

Create `crates/yamalgam-serde/src/de.rs`. This file contains:

1. `Deserializer<'input>` struct wrapping a peekable event iterator
2. `next_event()` / `peek_event()` that skip structural events
3. `impl<'de> serde::Deserializer<'de> for &mut Deserializer<'de>` with:
   - `deserialize_any` — dispatches based on scalar value (tag resolution) or event kind (seq/map)
   - `deserialize_bool`, `deserialize_i8..i64`, `deserialize_u8..u64`, `deserialize_f32..f64` — consume scalar, parse
   - `deserialize_str`, `deserialize_string` — consume scalar, return text
   - `deserialize_option` — peek for null/empty, then forward to `some`
   - `deserialize_unit`, `deserialize_unit_struct` — consume null scalar
   - Forward methods for remaining types (filled in Task 6/7)
4. Event consumption helpers: consume StreamStart/DocumentStart on first access, skip structural events

Key implementation detail — `deserialize_any` must use tag resolution to dispatch plain scalars to the correct visitor method. Use `yamalgam_core::resolve_plain_scalar()` to determine if a plain scalar is bool/int/float/null/string, then call the matching `visitor.visit_*()`.

For quoted scalars, always call `visitor.visit_borrowed_str()` (or `visit_str` if the value is owned due to escape processing).

Update `crates/yamalgam-serde/src/lib.rs` to expose the public API:

```rust
mod de;
mod error;

pub use de::Deserializer;
pub use error::Error;

/// Deserialize a single YAML document.
///
/// Errors if the input contains multiple documents. For multi-document
/// streams, use [`Deserializer::from_str`] with [`Deserializer::documents`].
pub fn from_str<'de, T: serde::Deserialize<'de>>(input: &'de str) -> Result<T, Error> {
    let mut de = Deserializer::from_str(input);
    let value = T::deserialize(&mut de)?;
    de.check_end()?;
    Ok(value)
}
```

Note: `from_str` does NOT use erased-serde yet. That comes in Task 8. Get it working first, optimize for compile time later.

**Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p yamalgam-serde --test scalars`
Expected: All scalar tests pass.

**Step 5: Commit**

```
feat(serde): scalar deserialization with tag resolution
```

---

### Task 6: Sequence and mapping deserialization

**Files:**
- Modify: `crates/yamalgam-serde/src/de.rs`
- Test: `crates/yamalgam-serde/tests/collections.rs`

**Step 1: Write the failing tests**

Create `crates/yamalgam-serde/tests/collections.rs`:

```rust
use std::collections::HashMap;
use yamalgam_serde::from_str;

#[test]
fn deserialize_sequence() {
    let v: Vec<i64> = from_str("- 1\n- 2\n- 3").unwrap();
    assert_eq!(v, vec![1, 2, 3]);
}

#[test]
fn deserialize_flow_sequence() {
    let v: Vec<String> = from_str("[a, b, c]").unwrap();
    assert_eq!(v, vec!["a", "b", "c"]);
}

#[test]
fn deserialize_mapping() {
    let v: HashMap<String, i64> = from_str("x: 1\ny: 2").unwrap();
    assert_eq!(v["x"], 1);
    assert_eq!(v["y"], 2);
}

#[test]
fn deserialize_flow_mapping() {
    let v: HashMap<String, String> = from_str("{a: b, c: d}").unwrap();
    assert_eq!(v["a"], "b");
    assert_eq!(v["c"], "d");
}

#[test]
fn deserialize_nested_mapping() {
    let v: HashMap<String, HashMap<String, i64>> =
        from_str("outer:\n  inner: 42").unwrap();
    assert_eq!(v["outer"]["inner"], 42);
}

#[test]
fn deserialize_sequence_of_mappings() {
    let v: Vec<HashMap<String, String>> =
        from_str("- name: alice\n- name: bob").unwrap();
    assert_eq!(v[0]["name"], "alice");
    assert_eq!(v[1]["name"], "bob");
}

#[test]
fn deserialize_mapping_of_sequences() {
    let v: HashMap<String, Vec<i64>> = from_str("nums:\n  - 1\n  - 2").unwrap();
    assert_eq!(v["nums"], vec![1, 2]);
}

#[test]
fn deserialize_empty_sequence() {
    let v: Vec<i64> = from_str("[]").unwrap();
    assert!(v.is_empty());
}

#[test]
fn deserialize_empty_mapping() {
    let v: HashMap<String, String> = from_str("{}").unwrap();
    assert!(v.is_empty());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo nextest run -p yamalgam-serde --test collections`
Expected: FAIL — SeqAccess/MapAccess not implemented.

**Step 3: Implement SeqAccess and MapAccess**

Add to `crates/yamalgam-serde/src/de.rs`:

- `struct SeqAccess<'a, 'de>` — holds `&'a mut Deserializer<'de>`, tracks whether SequenceEnd has been consumed
- `impl serde::de::SeqAccess<'de>` — `next_element_seed()` peeks for SequenceEnd, if not found deserializes next element
- `struct MapAccess<'a, 'de>` — holds `&'a mut Deserializer<'de>`, tracks whether MappingEnd has been consumed
- `impl serde::de::MapAccess<'de>` — `next_key_seed()` peeks for MappingEnd, deserializes key; `next_value_seed()` deserializes value

Wire into `deserialize_seq`, `deserialize_map`:
- Consume `SequenceStart` / `MappingStart` event
- Create the access struct
- Call `visitor.visit_seq(access)` / `visitor.visit_map(access)`

`deserialize_any` — when peeking `SequenceStart`, forward to `deserialize_seq`; when peeking `MappingStart`, forward to `deserialize_map`.

**Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p yamalgam-serde --test collections`
Expected: All collection tests pass.

**Step 5: Commit**

```
feat(serde): sequence and mapping deserialization
```

---

### Task 7: Struct and enum deserialization

**Files:**
- Modify: `crates/yamalgam-serde/src/de.rs`
- Test: `crates/yamalgam-serde/tests/structs.rs`

**Step 1: Write the failing tests**

Create `crates/yamalgam-serde/tests/structs.rs`:

```rust
use serde::Deserialize;
use yamalgam_serde::from_str;

#[derive(Debug, Deserialize, PartialEq)]
struct Server {
    host: String,
    port: u16,
}

#[test]
fn deserialize_struct() {
    let s: Server = from_str("host: localhost\nport: 8080").unwrap();
    assert_eq!(s, Server { host: "localhost".into(), port: 8080 });
}

#[derive(Debug, Deserialize, PartialEq)]
struct Config {
    server: Server,
    debug: bool,
}

#[test]
fn deserialize_nested_struct() {
    let c: Config = from_str("server:\n  host: 0.0.0.0\n  port: 443\ndebug: false").unwrap();
    assert_eq!(c.server.host, "0.0.0.0");
    assert_eq!(c.server.port, 443);
    assert!(!c.debug);
}

#[derive(Debug, Deserialize, PartialEq)]
struct WithDefaults {
    name: String,
    #[serde(default)]
    count: u32,
    #[serde(default)]
    tags: Vec<String>,
}

#[test]
fn deserialize_with_defaults() {
    let w: WithDefaults = from_str("name: test").unwrap();
    assert_eq!(w, WithDefaults { name: "test".into(), count: 0, tags: vec![] });
}

#[derive(Debug, Deserialize, PartialEq)]
struct WithOption {
    name: String,
    label: Option<String>,
}

#[test]
fn deserialize_optional_present() {
    let w: WithOption = from_str("name: foo\nlabel: bar").unwrap();
    assert_eq!(w.label, Some("bar".into()));
}

#[test]
fn deserialize_optional_null() {
    let w: WithOption = from_str("name: foo\nlabel: ~").unwrap();
    assert_eq!(w.label, None);
}

#[test]
fn deserialize_optional_missing() {
    let w: WithOption = from_str("name: foo").unwrap();
    assert_eq!(w.label, None);
}

#[derive(Debug, Deserialize, PartialEq)]
enum Color {
    Red,
    Green,
    Blue,
}

#[test]
fn deserialize_unit_enum() {
    assert_eq!(from_str::<Color>("Red").unwrap(), Color::Red);
    assert_eq!(from_str::<Color>("Blue").unwrap(), Color::Blue);
}

#[derive(Debug, Deserialize, PartialEq)]
enum Shape {
    Circle(f64),
    Rectangle { width: f64, height: f64 },
    Point,
}

#[test]
fn deserialize_newtype_enum() {
    let s: Shape = from_str("Circle: 5.0").unwrap();
    assert_eq!(s, Shape::Circle(5.0));
}

#[test]
fn deserialize_struct_enum() {
    let s: Shape = from_str("Rectangle:\n  width: 10.0\n  height: 20.0").unwrap();
    assert_eq!(s, Shape::Rectangle { width: 10.0, height: 20.0 });
}

#[test]
fn deserialize_unit_enum_variant() {
    let s: Shape = from_str("Point").unwrap();
    assert_eq!(s, Shape::Point);
}

#[derive(Debug, Deserialize, PartialEq)]
struct WithVec {
    items: Vec<Server>,
}

#[test]
fn deserialize_vec_of_structs() {
    let input = "items:\n  - host: a\n    port: 1\n  - host: b\n    port: 2";
    let w: WithVec = from_str(input).unwrap();
    assert_eq!(w.items.len(), 2);
    assert_eq!(w.items[0].host, "a");
    assert_eq!(w.items[1].port, 2);
}

#[test]
fn deserialize_tuple() {
    let v: (i64, String, bool) = from_str("- 1\n- hello\n- true").unwrap();
    assert_eq!(v, (1, "hello".into(), true));
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum StringOrInt {
    Int(i64),
    Str(String),
}

#[test]
fn deserialize_untagged_enum() {
    assert_eq!(from_str::<StringOrInt>("42").unwrap(), StringOrInt::Int(42));
    assert_eq!(from_str::<StringOrInt>("hello").unwrap(), StringOrInt::Str("hello".into()));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo nextest run -p yamalgam-serde --test structs`
Expected: FAIL — struct/enum deserialization not implemented.

**Step 3: Implement struct and enum deserialization**

Add to `crates/yamalgam-serde/src/de.rs`:

- `deserialize_struct` — same as `deserialize_map` (YAML represents structs as mappings). The `fields` parameter is available but YAML is self-describing so it's used only for `#[serde(default)]` / missing field handling.
- `deserialize_enum` — peek the next event:
  - If `Scalar` (plain string) → unit variant via `visitor.visit_enum(UnitVariantAccess)`
  - If `MappingStart` → tagged variant. Consume start, read key as variant name, dispatch value via `VariantAccess`
- `struct EnumAccess<'a, 'de>` — implements `serde::de::EnumAccess`
- `struct VariantAccess<'a, 'de>` — implements `serde::de::VariantAccess` with `unit_variant`, `newtype_variant_seed`, `tuple_variant`, `struct_variant`
- `deserialize_tuple`, `deserialize_tuple_struct` — consume SequenceStart, yield elements, consume SequenceEnd
- `deserialize_newtype_struct` — forward to inner type deserialization
- `deserialize_ignored_any` — skip the current value (scalar: consume; collection: track depth until matching end)

**Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p yamalgam-serde --test structs`
Expected: All struct/enum tests pass.

**Step 5: Commit**

```
feat(serde): struct, enum, and tuple deserialization
```

---

### Task 8: Anchor and alias support

**Files:**
- Modify: `crates/yamalgam-serde/src/de.rs`
- Test: `crates/yamalgam-serde/tests/anchors.rs`

**Step 1: Write the failing tests**

Create `crates/yamalgam-serde/tests/anchors.rs`:

```rust
use std::collections::HashMap;
use serde::Deserialize;
use yamalgam_serde::from_str;

#[test]
fn scalar_anchor_alias() {
    let v: HashMap<String, String> =
        from_str("a: &val hello\nb: *val").unwrap();
    assert_eq!(v["a"], "hello");
    assert_eq!(v["b"], "hello");
}

#[derive(Debug, Deserialize, PartialEq)]
struct Entry {
    host: String,
    port: u16,
}

#[test]
fn mapping_anchor_alias() {
    let input = "primary: &srv\n  host: db.local\n  port: 5432\nreplica: *srv";
    let v: HashMap<String, Entry> = from_str(input).unwrap();
    assert_eq!(v["primary"], v["replica"]);
    assert_eq!(v["replica"].host, "db.local");
}

#[test]
fn sequence_anchor_alias() {
    let input = "a: &nums\n  - 1\n  - 2\nb: *nums";
    let v: HashMap<String, Vec<i64>> = from_str(input).unwrap();
    assert_eq!(v["a"], vec![1, 2]);
    assert_eq!(v["b"], vec![1, 2]);
}

#[test]
fn undefined_alias_errors() {
    let result = from_str::<HashMap<String, String>>("a: *missing");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("undefined alias"));
}

#[test]
fn alias_expansion_limit() {
    use yamalgam_core::LoaderConfig;
    use yamalgam_serde::from_str_with_config;

    let config = LoaderConfig::strict(); // max_alias_expansions: 100
    // Create input with many alias references
    let mut input = "base: &b x\n".to_string();
    for i in 0..200 {
        input.push_str(&format!("k{i}: *b\n"));
    }
    let result = from_str_with_config::<HashMap<String, String>>(&input, &config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("limit exceeded"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo nextest run -p yamalgam-serde --test anchors`
Expected: FAIL — anchor/alias handling not implemented.

**Step 3: Implement anchor buffering + alias replay**

Add to `crates/yamalgam-serde/src/de.rs`:

1. Add `anchors: HashMap<String, Vec<Event<'input>>>` field to `Deserializer`
2. Add `anchor_count: usize` and `alias_expansions: usize` for limit tracking
3. When processing any event with `anchor: Some(name)`:
   - If scalar: clone the event, store `anchors[name] = vec![event.clone()]`
   - If SequenceStart/MappingStart: buffer all events (tracking depth) until the matching End, store in `anchors[name]`. Then replay the buffered events for the current deserialization.
4. When encountering `Event::Alias { name, .. }`:
   - Look up `anchors[name]`, error if not found
   - Increment `alias_expansions`, check `limits.max_alias_expansions`
   - Create a `ReplayDeserializer` that wraps `VecDeque<Event>` (cloned from the buffer)
   - Deserialize from the replay deserializer
5. `ReplayDeserializer` — minimal Deserializer that reads from a `VecDeque<Event>` instead of the main event stream. Shares the same `serde::Deserializer` implementation via a trait or internal enum dispatch.

Design note: To avoid duplicating the entire `serde::Deserializer` impl, consider making the event source an enum:
```rust
enum EventSource<'input> {
    Stream(Peekable<Box<dyn Iterator<...>>>),
    Replay(VecDeque<Event<'input>>),
}
```

This way one `Deserializer` impl handles both live and replayed events.

**Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p yamalgam-serde --test anchors`
Expected: All anchor/alias tests pass.

**Step 5: Commit**

```
feat(serde): anchor buffering and alias replay
```

---

### Task 9: erased-serde integration

**Files:**
- Modify: `crates/yamalgam-serde/src/lib.rs`
- Modify: `crates/yamalgam-serde/src/de.rs`
- Test: `crates/yamalgam-serde/tests/erased.rs`

**Step 1: Write the failing test**

Create `crates/yamalgam-serde/tests/erased.rs`:

```rust
//! Verify that from_str uses erased-serde (parser code compiled once).

use serde::Deserialize;
use yamalgam_serde::from_str;

#[derive(Debug, Deserialize, PartialEq)]
struct A { x: i64 }

#[derive(Debug, Deserialize, PartialEq)]
struct B { y: String }

#[test]
fn from_str_works_with_different_types() {
    // Both types go through the same erased deserializer internally
    let a: A = from_str("x: 42").unwrap();
    assert_eq!(a, A { x: 42 });

    let b: B = from_str("y: hello").unwrap();
    assert_eq!(b, B { y: "hello".into() });
}
```

**Step 2: Integrate erased-serde into from_str**

Update `crates/yamalgam-serde/src/lib.rs` — change `from_str` to use erased-serde:

```rust
pub fn from_str<'de, T: serde::Deserialize<'de>>(input: &'de str) -> Result<T, Error> {
    let mut de = Deserializer::from_str(input);
    let value = erased_serde::deserialize::<T>(&mut de)?;
    de.check_end()?;
    Ok(value)
}
```

This requires `Deserializer` to implement `erased_serde::Deserializer` — which it gets for free if it implements `serde::Deserializer`. The `erased_serde::deserialize` function handles the type erasure.

Handle the error conversion: `erased_serde::Error` → `Error`. Implement `From<erased_serde::Error> for Error` via the Custom variant, or use the `erased_serde::Deserializer` trait object approach.

Note: Check `erased-serde` 0.4 API — the exact integration pattern may differ from 0.3. Read the docs.

**Step 3: Run tests to verify they pass**

Run: `cargo nextest run -p yamalgam-serde`
Expected: All existing tests + erased test pass.

**Step 4: Commit**

```
feat(serde): erased-serde integration for zero-monomorphization
```

---

### Task 10: Multi-document streaming iterator

**Files:**
- Create: `crates/yamalgam-serde/src/documents.rs`
- Modify: `crates/yamalgam-serde/src/lib.rs`
- Test: `crates/yamalgam-serde/tests/multi_doc.rs`

**Step 1: Write the failing tests**

Create `crates/yamalgam-serde/tests/multi_doc.rs`:

```rust
use serde::Deserialize;
use yamalgam_serde::{from_str, Deserializer};

#[test]
fn single_document_via_from_str() {
    let v: i64 = from_str("42").unwrap();
    assert_eq!(v, 42);
}

#[test]
fn from_str_errors_on_multiple_documents() {
    let result = from_str::<i64>("42\n---\n99");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("more than one document"));
}

#[derive(Debug, Deserialize, PartialEq)]
struct Item { name: String }

#[test]
fn documents_iterator() {
    let input = "---\nname: first\n---\nname: second\n---\nname: third";
    let docs: Vec<Item> = Deserializer::from_str(input)
        .documents::<Item>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(docs.len(), 3);
    assert_eq!(docs[0].name, "first");
    assert_eq!(docs[2].name, "third");
}

#[test]
fn documents_iterator_mixed_types() {
    // All documents must be the same type T in documents::<T>()
    let input = "---\n42\n---\n99";
    let docs: Vec<i64> = Deserializer::from_str(input)
        .documents::<i64>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(docs, vec![42, 99]);
}

#[test]
fn documents_empty_stream() {
    let docs: Vec<i64> = Deserializer::from_str("")
        .documents::<i64>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(docs.is_empty());
}

#[test]
fn documents_single_implicit() {
    let docs: Vec<i64> = Deserializer::from_str("42")
        .documents::<i64>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(docs, vec![42]);
}

#[test]
fn documents_with_doc_end_markers() {
    let input = "---\n42\n...\n---\n99\n...";
    let docs: Vec<i64> = Deserializer::from_str(input)
        .documents::<i64>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(docs, vec![42, 99]);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo nextest run -p yamalgam-serde --test multi_doc`
Expected: FAIL — `documents()` method doesn't exist.

**Step 3: Implement Documents iterator**

Create `crates/yamalgam-serde/src/documents.rs`:

```rust
//! Streaming multi-document iterator.

use serde::Deserialize;
use crate::de::Deserializer;
use crate::Error;

/// Iterator over YAML documents in a stream.
///
/// Created by [`Deserializer::documents`]. Analogous to
/// `serde_json::StreamDeserializer`.
pub struct Documents<'input, T> {
    de: Deserializer<'input>,
    done: bool,
    _marker: std::marker::PhantomData<T>,
}

impl<'input, T> Documents<'input, T> {
    pub(crate) fn new(de: Deserializer<'input>) -> Self {
        Self { de, done: false, _marker: std::marker::PhantomData }
    }
}

impl<'input, T: Deserialize<'input>> Iterator for Documents<'input, T> {
    type Item = Result<T, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        // Check if we've reached StreamEnd
        // Consume DocumentStart if present, deserialize one document,
        // consume DocumentEnd, check for next DocumentStart or StreamEnd
        // Return None when StreamEnd is reached
        todo!()
    }
}
```

Add `documents()` method to `Deserializer`:
```rust
pub fn documents<T: Deserialize<'input>>(self) -> Documents<'input, T> {
    Documents::new(self)
}
```

The iterator:
1. On first call, consume `StreamStart` (if not already consumed)
2. Peek: if `StreamEnd`, consume it, set `done = true`, return `None`
3. Consume `DocumentStart` (explicit or implicit)
4. Deserialize one `T` from the event stream
5. Consume `DocumentEnd` (explicit or implicit)
6. Return `Some(Ok(value))`
7. On error, set `done = true`, return `Some(Err(e))`

Also update `check_end()` in `from_str` to consume `DocumentEnd` + check for additional `DocumentStart` + consume `StreamEnd`.

**Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p yamalgam-serde --test multi_doc`
Expected: All multi-document tests pass.

**Step 5: Commit**

```
feat(serde): multi-document streaming iterator
```

---

### Task 11: Config-aware APIs

**Files:**
- Modify: `crates/yamalgam-serde/src/lib.rs`
- Modify: `crates/yamalgam-serde/src/de.rs`
- Test: `crates/yamalgam-serde/tests/config.rs`

**Step 1: Write the failing tests**

Create `crates/yamalgam-serde/tests/config.rs`:

```rust
use std::collections::HashMap;
use yamalgam_core::{LoaderConfig, TagResolution};
use yamalgam_serde::{from_str_with_config, Deserializer};

#[test]
fn yaml11_tag_resolution() {
    let config = LoaderConfig::moderate().with_tag_resolution(TagResolution::Yaml11);
    // YAML 1.1: "yes" → true (bool)
    let v: bool = from_str_with_config("yes", &config).unwrap();
    assert!(v);
}

#[test]
fn failsafe_tag_resolution() {
    let config = LoaderConfig::moderate().with_tag_resolution(TagResolution::Failsafe);
    // Failsafe: "true" stays a string
    let v: String = from_str_with_config("true", &config).unwrap();
    assert_eq!(v, "true");
}

#[test]
fn depth_limit_enforced() {
    let config = LoaderConfig::strict(); // max_depth: 64
    // Create deeply nested YAML that exceeds depth limit
    let mut input = String::new();
    for i in 0..100 {
        input.push_str(&"  ".repeat(i));
        input.push_str(&format!("k{i}:\n"));
    }
    let result = from_str_with_config::<serde_json::Value>(&input, &config);
    assert!(result.is_err());
}

#[test]
fn config_with_documents_iterator() {
    let config = LoaderConfig::strict();
    let input = "---\n42\n---\n99";
    let docs: Vec<i64> = Deserializer::from_str_with_config(input, &config)
        .documents::<i64>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(docs, vec![42, 99]);
}
```

**Step 2: Implement config-aware constructors**

Add to `Deserializer`:
```rust
pub fn from_str_with_config(input: &'input str, config: &LoaderConfig) -> Self
```

Add to `lib.rs`:
```rust
pub fn from_str_with_config<'de, T: serde::Deserialize<'de>>(
    input: &'de str,
    config: &yamalgam_core::LoaderConfig,
) -> Result<T, Error>
```

The Deserializer stores `tag_resolver: Box<dyn TagResolver>` (constructed from `config.tag_resolution`) and `limits: ResourceLimits` (from `config.limits`). Limits are checked during:
- Anchor registration (anchor count)
- Alias expansion (expansion count)
- Nesting depth (seq/map recursion depth tracking)

**Step 3: Run tests**

Run: `cargo nextest run -p yamalgam-serde --test config`
Expected: All config tests pass.

**Step 4: Commit**

```
feat(serde): config-aware deserialization with limits and tag resolution
```

---

### Task 12: from_reader API

**Files:**
- Modify: `crates/yamalgam-serde/src/lib.rs`
- Modify: `crates/yamalgam-serde/src/de.rs`
- Test: `crates/yamalgam-serde/tests/reader.rs`

**Step 1: Write the failing test**

Create `crates/yamalgam-serde/tests/reader.rs`:

```rust
use std::io::Cursor;
use serde::Deserialize;
use yamalgam_serde::from_reader;

#[derive(Debug, Deserialize, PartialEq)]
struct Item { name: String }

#[test]
fn deserialize_from_reader() {
    let data = b"name: yamalgam";
    let item: Item = from_reader(Cursor::new(data)).unwrap();
    assert_eq!(item, Item { name: "yamalgam".into() });
}
```

**Step 2: Implement from_reader**

```rust
pub fn from_reader<R: std::io::Read, T: serde::de::DeserializeOwned>(
    mut reader: R,
) -> Result<T, Error> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf).map_err(/* IO error → Error */)?;
    from_str(&buf)  // Note: can't borrow, so T: DeserializeOwned
}
```

Note: `from_reader` requires `DeserializeOwned` because the buffer is local. This matches serde_yaml's behavior.

**Step 3: Run tests, commit**

```
feat(serde): from_reader API
```

---

## Phase 3: Integration Tests

### Task 13: Re-export from yamalgam CLI crate

**Files:**
- Modify: `crates/yamalgam/Cargo.toml` (add yamalgam-serde dep)
- Modify: `crates/yamalgam/src/lib.rs` (re-export)

**Step 1: Add dependency and re-export**

In `crates/yamalgam/Cargo.toml`:
```toml
yamalgam-serde = { path = "../yamalgam-serde" }
```

In `crates/yamalgam/src/lib.rs`, add:
```rust
// Re-export serde deserialization for `yamalgam::from_str::<T>()`
pub use yamalgam_serde::{from_str, from_str_with_config, from_reader, Deserializer, Error as DeserializeError};
```

**Step 2: Verify**

Run: `just check`

**Step 3: Commit**

```
feat: re-export serde API from yamalgam crate
```

---

### Task 14: Real-world YAML integration tests

**Files:**
- Create: `crates/yamalgam-serde/tests/integration.rs`
- Create: `crates/yamalgam-serde/tests/fixtures/` (YAML test files)

**Step 1: Crawl yamllint test repos**

Identify and collect real-world YAML test fixtures from:
- **yamllint** (Python): `https://github.com/adrienverge/yamllint` — `tests/` directory has extensive YAML fixtures
- **yaml-lint** (Ruby): `https://github.com/Pryz/yaml-lint` — test fixtures
- **yamlfmt** (Go): `https://github.com/google/yamlfmt` — test data
- **prettier YAML plugin**: `https://github.com/prettier/prettier` — YAML test fixtures
- **yq** (Go): `https://github.com/mikefarah/yq` — test YAML files

Collect a diverse set of YAML files covering:
- Kubernetes manifests (Deployment, Service, ConfigMap)
- Docker Compose files
- GitHub Actions workflows
- Ansible playbooks
- Helm charts
- CI/CD configs (GitLab CI, CircleCI, Travis CI)

Store in `crates/yamalgam-serde/tests/fixtures/` with subdirectories by source.

**Step 2: Write integration tests**

```rust
use serde_json::Value;
use yamalgam_serde::from_str;

/// Deserialize every fixture into serde_json::Value.
/// If the YAML is valid, deserialization must succeed.
#[test]
fn fixtures_round_trip() {
    let fixtures_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures");

    for entry in walkdir::WalkDir::new(&fixtures_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| {
            e.path().extension().is_some_and(|ext| ext == "yaml" || ext == "yml")
        })
    {
        let content = std::fs::read_to_string(entry.path()).unwrap();
        let path = entry.path().strip_prefix(&fixtures_dir).unwrap();

        // Try deserializing each document
        let docs: Result<Vec<Value>, _> = yamalgam_serde::Deserializer::from_str(&content)
            .documents::<Value>()
            .collect();

        assert!(docs.is_ok(), "Failed to parse {}: {}", path.display(), docs.unwrap_err());
    }
}
```

Note: Add `walkdir` as a dev-dependency, or use `std::fs::read_dir` with manual recursion.

Alternatively, if fixture collection is too large, curate a smaller set of representative files and inline them or keep them in a vendor directory.

**Step 3: serde_yaml compatibility tests**

Create `crates/yamalgam-serde/tests/compat.rs` — tests ported from serde_yaml's test suite that verify behavioral parity:

- Mapping key types (string, integer, bool keys)
- Nested collections
- Special float values (NaN, Inf)
- Empty documents
- Complex anchors/aliases
- Tagged values
- Multi-line strings (literal, folded)

**Step 4: Commit**

```
test(serde): integration tests with real-world YAML fixtures
```

---

### Task 15: Compliance harness extension

**Files:**
- Modify: `crates/yamalgam-compare/Cargo.toml` (add yamalgam-serde + yamalgam-compose deps)
- Modify: `crates/yamalgam-compare/tests/compliance.rs` (add serde round-trip check)

**Step 1: Add serde round-trip to compliance**

For each non-failing YAML Test Suite case, add a check:
1. Parse with Composer → `Vec<Value>`
2. Parse with serde → `from_str::<yamalgam_core::Value>()`
3. Compare the results — they should match

This verifies that the serde deserializer produces the same semantic output as the Composer for all test suite cases.

Note: This requires `impl Deserialize for Value` — either implement it in yamalgam-core, or use `serde_json::Value` as the comparison type (both Composer and serde produce JSON-compatible values).

Decision: Implement `impl<'de> Deserialize<'de> for Value` in yamalgam-core (it already has serde as a dependency). This also makes `Value` useful as a serde target for consumers.

**Step 2: Commit**

```
test: serde round-trip in YAML Test Suite compliance harness
```

---

### Task 16: Final verification + PR

**Step 1: Run full check**

Run: `just check`
Expected: All tests pass — scanner, parser, compose, CST, serde, compliance.

**Step 2: Run benchmarks**

Run: `just bench`
Verify no performance regression in scanner/parser benchmarks.

**Step 3: Update record/superpowers/plans/README.md**

Mark M9 as complete, note test count.

**Step 4: Create PR**

PR title: `feat(serde): streaming serde Deserializer (#M9)`

Include in PR description:
- Summary of all new crates and APIs
- Test count increase
- Drop-in compatibility notes
- Link to design doc

---

## PR Grouping

The tasks above can be grouped into PRs:

| PR | Tasks | Title |
|----|-------|-------|
| 1 | 1-3 | `refactor: extract yamalgam-compose and yamalgam-cst crates` |
| 2 | 4-6 | `feat(serde): scalar, collection, and struct deserialization` |
| 3 | 7-8 | `feat(serde): anchor/alias support with event buffering` |
| 4 | 9-10 | `feat(serde): erased-serde integration and multi-document iterator` |
| 5 | 11-13 | `feat(serde): config-aware APIs and CLI re-export` |
| 6 | 14-16 | `test(serde): integration tests, compliance, and fixtures` |

Each PR should pass `just check` independently.

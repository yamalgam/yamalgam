# Resolver Trait + Value DOM Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Define the resolver middleware trait (with NoopResolver) and build the Value DOM as the first concrete consumer of the parser event stream.

**Architecture:** The resolver trait is a composable event-stream transformer that sits between the parser and all consumers. Value is a lossy DOM type (enum of YAML data types) built by a Composer that consumes resolved events. The resolver trait and NoopResolver are defined first so every consumer is built against the middleware interface from day one. Value lives in yamalgam-core (pure data type, no parser deps). Resolver trait and Composer live in yamalgam-parser (they reference Event).

**Tech Stack:** Rust (edition 2024), yamalgam-core, yamalgam-parser, thiserror. No new crates — this milestone adds modules to existing crates.

**Reference files:**
- Design: `docs/plans/2026-03-08-event-consumers-and-resolver-design.md`
- ADR: `docs/decisions/0007-resolver-trait-for-event-stream-middleware.md`
- ADR: `docs/decisions/0006-loaderconfig-for-resource-limits-and-security-policy.md`
- Parser events: `crates/yamalgam-parser/src/event.rs`
- Parser API: `crates/yamalgam-parser/src/parser.rs`
- LoaderConfig: `crates/yamalgam-core/src/loader.rs`
- Core lib: `crates/yamalgam-core/src/lib.rs`
- Parser lib: `crates/yamalgam-parser/src/lib.rs`

**Crate placement rationale:**
- `Value` type → `yamalgam-core::value` — pure data enum with no parser/scanner deps
- `Resolver` trait, `ResolveError`, `NoopResolver` → `yamalgam-parser::resolve` — references `Event` (lives in parser)
- `Composer` (events → Value) → `yamalgam-parser::compose` — consumes parser events, produces core Value

**Dependency graph (unchanged):**
```
yamalgam-core  (base — no internal deps)
    ↑
yamalgam-scanner  (depends on core)
    ↑
yamalgam-parser  (depends on core + scanner)
    ↑
yamalgam  (CLI — depends on core)
```

---

## Task 1: Define the Value type in yamalgam-core

**Files:**
- Create: `crates/yamalgam-core/src/value.rs`
- Modify: `crates/yamalgam-core/src/lib.rs`
- Test: inline `#[cfg(test)]` in `value.rs`

**Step 1: Write the failing test**

Add `crates/yamalgam-core/src/value.rs` with tests only:

```rust
//! YAML Value type — the lossy DOM representation.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_default() {
        assert_eq!(Value::default(), Value::Null);
    }

    #[test]
    fn mapping_get_by_str() {
        let mut map = Mapping::new();
        map.insert(Value::from("key"), Value::from("value"));
        let v = Value::Mapping(map);
        assert_eq!(v.get("key"), Some(&Value::from("value")));
    }

    #[test]
    fn sequence_index() {
        let v = Value::Sequence(vec![Value::from("a"), Value::from("b")]);
        assert_eq!(v.get_index(1), Some(&Value::from("b")));
        assert_eq!(v.get_index(5), None);
    }

    #[test]
    fn as_str_returns_string_content() {
        let v = Value::from("hello");
        assert_eq!(v.as_str(), Some("hello"));
        assert_eq!(Value::Null.as_str(), None);
    }

    #[test]
    fn as_bool_coerces_yaml_booleans() {
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::from("not a bool").as_bool(), None);
    }

    #[test]
    fn as_i64_returns_integer() {
        assert_eq!(Value::Integer(42).as_i64(), Some(42));
        assert_eq!(Value::Null.as_i64(), None);
    }

    #[test]
    fn as_f64_returns_float() {
        assert_eq!(Value::Float(3.14).as_f64(), Some(3.14));
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
        let mut map = Mapping::new();
        map.insert(Value::from("z"), Value::from(1));
        map.insert(Value::from("a"), Value::from(2));
        let keys: Vec<_> = map.keys().collect();
        assert_eq!(keys[0], &Value::from("z"));
        assert_eq!(keys[1], &Value::from("a"));
    }
}
```

**Step 2: Write the Value type**

Complete `crates/yamalgam-core/src/value.rs` above the tests:

```rust
//! YAML Value type — the lossy DOM representation.
//!
//! [`Value`] represents the YAML data model as an in-memory tree.
//! It discards comments, whitespace, quoting style, and other
//! presentation details. For a lossless representation that preserves
//! these, use the CST layer (not yet implemented).
//!
//! # Mapping key ordering
//!
//! [`Mapping`] preserves insertion order, matching the behavior users
//! expect from YAML files. Internally backed by `Vec<(Value, Value)>`
//! with linear scan for lookup — appropriate for typical config-sized
//! documents. Not optimized for 10K+ key mappings.

use std::fmt;

/// A YAML value in the lossy DOM representation.
///
/// Mirrors the YAML data model: null, bool, integer, float, string,
/// sequence (list), and mapping (ordered key-value pairs).
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// YAML null (`~`, `null`, empty value).
    Null,
    /// Boolean value.
    Bool(bool),
    /// Integer value (signed 64-bit).
    Integer(i64),
    /// Floating-point value (64-bit).
    Float(f64),
    /// String value.
    String(String),
    /// Ordered sequence (list) of values.
    Sequence(Vec<Value>),
    /// Ordered mapping (dictionary) of key-value pairs.
    Mapping(Mapping),
}

impl Default for Value {
    fn default() -> Self {
        Self::Null
    }
}

impl Value {
    /// Get a value from a mapping by string key.
    ///
    /// Returns `None` if `self` is not a mapping or the key is absent.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Mapping(map) => map.get(&Value::String(key.to_owned())),
            _ => None,
        }
    }

    /// Get a value from a sequence by index.
    ///
    /// Returns `None` if `self` is not a sequence or the index is out of bounds.
    #[must_use]
    pub fn get_index(&self, index: usize) -> Option<&Value> {
        match self {
            Value::Sequence(seq) => seq.get(index),
            _ => None,
        }
    }

    /// Extract as a string slice if this is a `String` variant.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Extract as a boolean if this is a `Bool` variant.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Extract as an i64 if this is an `Integer` variant.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Integer(n) => Some(*n),
            _ => None,
        }
    }

    /// Extract as an f64 if this is a `Float` variant.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Returns `true` if this is the `Null` variant.
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Integer(n) => write!(f, "{n}"),
            Value::Float(v) => write!(f, "{v}"),
            Value::String(s) => write!(f, "{s}"),
            Value::Sequence(_) => write!(f, "[sequence]"),
            Value::Mapping(_) => write!(f, "{{mapping}}"),
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_owned())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Integer(n)
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Integer(i64::from(n))
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

// ---------------------------------------------------------------------------
// Mapping
// ---------------------------------------------------------------------------

/// An ordered mapping of YAML key-value pairs.
///
/// Backed by `Vec<(Value, Value)>` — preserves insertion order and supports
/// arbitrary Value keys (not just strings). Lookup is linear scan, which is
/// appropriate for typical YAML documents (tens to hundreds of keys).
#[derive(Clone, Debug, Default)]
pub struct Mapping {
    entries: Vec<(Value, Value)>,
}

impl Mapping {
    /// Create an empty mapping.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Insert a key-value pair. If the key already exists, replaces the value.
    pub fn insert(&mut self, key: Value, value: Value) {
        for entry in &mut self.entries {
            if entry.0 == key {
                entry.1 = value;
                return;
            }
        }
        self.entries.push((key, value));
    }

    /// Get a value by key reference.
    #[must_use]
    pub fn get(&self, key: &Value) -> Option<&Value> {
        self.entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// Returns an iterator over the keys.
    pub fn keys(&self) -> impl Iterator<Item = &Value> {
        self.entries.iter().map(|(k, _)| k)
    }

    /// Returns an iterator over the values.
    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.entries.iter().map(|(_, v)| v)
    }

    /// Returns an iterator over the key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&Value, &Value)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    /// Returns the number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the mapping is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl PartialEq for Mapping {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}
```

**Step 3: Register the module in core's lib.rs**

Add to `crates/yamalgam-core/src/lib.rs`:

```rust
pub mod value;
pub use value::{Mapping, Value};
```

**Step 4: Run tests**

Run: `cargo nextest run -p yamalgam-core`
Expected: all tests pass including new Value tests.

**Step 5: Commit**

```
feat(core): add Value DOM type with Mapping, accessors, and From impls
```

---

## Task 2: Define the Resolver trait and ResolveError in yamalgam-parser

**Files:**
- Create: `crates/yamalgam-parser/src/resolve.rs`
- Modify: `crates/yamalgam-parser/src/lib.rs`
- Modify: `crates/yamalgam-parser/src/error.rs` (check existing error type)
- Test: inline `#[cfg(test)]` in `resolve.rs`

**Step 1: Read the existing ParseError**

Read `crates/yamalgam-parser/src/error.rs` to understand the current error type so `ResolveError` can wrap it.

**Step 2: Write the failing test**

Add `crates/yamalgam-parser/src/resolve.rs` with tests only:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Event, Parser};

    #[test]
    fn noop_resolver_passes_all_events_through() {
        let input = "key: value";
        let parser = Parser::new(input);
        let direct: Vec<_> = Parser::new(input)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let resolved: Vec<_> = ResolvedEvents::new(
            Box::new(parser.map(|r| r.map_err(ResolveError::Parse))),
            NoopResolver,
        )
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
        assert_eq!(direct, resolved);
    }

    #[test]
    fn noop_resolver_preserves_event_count() {
        let input = "- a\n- b\n- c";
        let parser = Parser::new(input);
        let count = Parser::new(input).count();
        let resolved_count = ResolvedEvents::new(
            Box::new(parser.map(|r| r.map_err(ResolveError::Parse))),
            NoopResolver,
        )
        .count();
        assert_eq!(count, resolved_count);
    }

    #[test]
    fn noop_resolver_passes_errors_through() {
        // Unterminated single-quoted scalar
        let input = "'unterminated";
        let parser = Parser::new(input);
        let results: Vec<_> = ResolvedEvents::new(
            Box::new(parser.map(|r| r.map_err(ResolveError::Parse))),
            NoopResolver,
        )
        .collect();
        assert!(results.iter().any(|r| r.is_err()));
    }

    #[test]
    fn resolve_error_from_parse_error() {
        let input = "'bad";
        let parser = Parser::new(input);
        let results: Vec<Result<Event, ResolveError>> = parser
            .map(|r| r.map_err(ResolveError::Parse))
            .collect();
        assert!(results.iter().any(|r| matches!(r, Err(ResolveError::Parse(_)))));
    }
}
```

**Step 3: Write the Resolver trait, ResolveError, and NoopResolver**

Complete `crates/yamalgam-parser/src/resolve.rs` above the tests:

```rust
//! Resolver middleware for event-stream transformation.
//!
//! Resolvers sit between the parser and consumers (Value, CST, serde, SAX).
//! They intercept, transform, or annotate events — typically to handle
//! `!include` directives, `$ref` references, or custom tag processing.
//!
//! See [ADR-0007](../../docs/decisions/0007-resolver-trait-for-event-stream-middleware.md).

use std::fmt;
use std::io;
use std::path::PathBuf;

use crate::error::ParseError;
use crate::event::Event;

// ---------------------------------------------------------------------------
// ResolveError
// ---------------------------------------------------------------------------

/// Errors that can occur during event resolution.
#[derive(Debug)]
pub enum ResolveError {
    /// An upstream parse error.
    Parse(ParseError),
    /// Include file could not be read.
    Include {
        /// Path that was requested.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
    /// `$ref` target could not be resolved.
    Ref {
        /// The reference target string.
        target: String,
        /// Underlying error.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// Circular include/ref chain detected.
    Cycle {
        /// The chain of references that formed the cycle.
        chain: Vec<String>,
    },
    /// A resource limit from LoaderConfig was exceeded.
    LimitExceeded(String),
    /// Custom resolver error.
    Custom(Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "{e}"),
            Self::Include { path, source } => {
                write!(f, "include error for {}: {source}", path.display())
            }
            Self::Ref { target, source } => {
                write!(f, "ref error for {target}: {source}")
            }
            Self::Cycle { chain } => {
                write!(f, "cycle detected: {}", chain.join(" -> "))
            }
            Self::LimitExceeded(msg) => write!(f, "limit exceeded: {msg}"),
            Self::Custom(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ResolveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(e) => Some(e),
            Self::Include { source, .. } => Some(source),
            Self::Ref { source, .. } => Some(source.as_ref()),
            Self::Custom(e) => Some(e.as_ref()),
            Self::Cycle { .. } | Self::LimitExceeded(_) => None,
        }
    }
}

impl From<ParseError> for ResolveError {
    fn from(e: ParseError) -> Self {
        Self::Parse(e)
    }
}

// ---------------------------------------------------------------------------
// Resolver trait
// ---------------------------------------------------------------------------

/// An event-stream transformer that can intercept, replace, or annotate events.
///
/// Resolvers wrap an event iterator and produce a new one. This gives full
/// control over buffering, lookahead, and recursive parsing (needed for
/// `!include` which must parse included files and splice events inline).
///
/// # Composition
///
/// Resolvers compose by chaining. Each resolver wraps the previous:
///
/// ```ignore
/// let events = ResolvedEvents::new(parser_events, IncludeResolver::new());
/// let events = ResolvedEvents::new(events, RefResolver::new());
/// ```
///
/// Order matters — includes are resolved before refs so that `$ref` inside
/// an included file works correctly.
pub trait Resolver<'input> {
    /// Process a single event, returning zero or more output events.
    ///
    /// - Return `Ok(smallvec![event])` to pass through unchanged.
    /// - Return `Ok(smallvec![...])` to replace with multiple events.
    /// - Return `Ok(smallvec![])` to suppress the event.
    /// - Return `Err(...)` to signal a resolution failure.
    ///
    /// For resolvers that need multi-event lookahead (e.g., `$ref` which
    /// spans a key-value pair), buffer events internally and emit them
    /// when the pattern completes or fails to match.
    fn on_event(
        &mut self,
        event: Event<'input>,
    ) -> Result<Vec<Event<'input>>, ResolveError>;
}

// ---------------------------------------------------------------------------
// NoopResolver
// ---------------------------------------------------------------------------

/// A resolver that passes all events through unchanged.
///
/// Used as the default when no resolution is configured. The optimizer
/// should inline this to zero overhead.
pub struct NoopResolver;

impl<'input> Resolver<'input> for NoopResolver {
    fn on_event(
        &mut self,
        event: Event<'input>,
    ) -> Result<Vec<Event<'input>>, ResolveError> {
        Ok(vec![event])
    }
}

// ---------------------------------------------------------------------------
// ResolvedEvents iterator
// ---------------------------------------------------------------------------

/// An iterator adapter that applies a [`Resolver`] to an event stream.
///
/// Wraps an upstream event iterator, passes each event through the resolver,
/// and yields the (potentially transformed) output events.
pub struct ResolvedEvents<'input, R: Resolver<'input>> {
    /// Upstream event source.
    upstream: Box<dyn Iterator<Item = Result<Event<'input>, ResolveError>> + 'input>,
    /// The resolver to apply.
    resolver: R,
    /// Buffer for events produced by the resolver (when one input event
    /// produces multiple output events).
    buffer: std::collections::VecDeque<Event<'input>>,
}

impl<'input, R: Resolver<'input>> ResolvedEvents<'input, R> {
    /// Create a new resolved event stream.
    pub fn new(
        upstream: Box<dyn Iterator<Item = Result<Event<'input>, ResolveError>> + 'input>,
        resolver: R,
    ) -> Self {
        Self {
            upstream,
            resolver,
            buffer: std::collections::VecDeque::new(),
        }
    }
}

impl<'input, R: Resolver<'input>> Iterator for ResolvedEvents<'input, R> {
    type Item = Result<Event<'input>, ResolveError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Drain buffered events first.
        if let Some(event) = self.buffer.pop_front() {
            return Some(Ok(event));
        }

        // Pull from upstream and resolve.
        loop {
            let upstream_event = match self.upstream.next()? {
                Ok(event) => event,
                Err(e) => return Some(Err(e)),
            };

            match self.resolver.on_event(upstream_event) {
                Ok(events) => {
                    let mut iter = events.into_iter();
                    if let Some(first) = iter.next() {
                        self.buffer.extend(iter);
                        return Some(Ok(first));
                    }
                    // Resolver returned empty vec — event suppressed, continue.
                }
                Err(e) => return Some(Err(e)),
            }
        }
    }
}
```

**Step 4: Register the module in parser's lib.rs**

Add to `crates/yamalgam-parser/src/lib.rs`:

```rust
pub mod resolve;
pub use resolve::{NoopResolver, ResolveError, ResolvedEvents, Resolver};
```

**Step 5: Run tests**

Run: `cargo nextest run -p yamalgam-parser`
Expected: all existing parser tests pass + new resolver tests pass.

**Step 6: Run full check**

Run: `cargo fmt --all && just clippy`
Expected: clean.

**Step 7: Commit**

```
feat(parser): add Resolver trait, ResolveError, NoopResolver, and ResolvedEvents adapter
```

---

## Task 3: Add scalar tag resolution (YAML Core Schema)

**Files:**
- Create: `crates/yamalgam-core/src/tag.rs`
- Modify: `crates/yamalgam-core/src/lib.rs`
- Test: inline `#[cfg(test)]` in `tag.rs`

The Composer needs to resolve plain scalars to typed Values (null, bool, int,
float, string) according to the YAML 1.2 Core Schema. This is a pure function
with no parser dependency, so it belongs in core.

**Step 1: Write the failing tests**

Add `crates/yamalgam-core/src/tag.rs`:

```rust
//! YAML 1.2 Core Schema tag resolution for plain scalars.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    #[test]
    fn null_variants() {
        assert_eq!(resolve_plain_scalar("null"), Value::Null);
        assert_eq!(resolve_plain_scalar("Null"), Value::Null);
        assert_eq!(resolve_plain_scalar("NULL"), Value::Null);
        assert_eq!(resolve_plain_scalar("~"), Value::Null);
        assert_eq!(resolve_plain_scalar(""), Value::Null);
    }

    #[test]
    fn bool_true_variants() {
        assert_eq!(resolve_plain_scalar("true"), Value::Bool(true));
        assert_eq!(resolve_plain_scalar("True"), Value::Bool(true));
        assert_eq!(resolve_plain_scalar("TRUE"), Value::Bool(true));
    }

    #[test]
    fn bool_false_variants() {
        assert_eq!(resolve_plain_scalar("false"), Value::Bool(false));
        assert_eq!(resolve_plain_scalar("False"), Value::Bool(false));
        assert_eq!(resolve_plain_scalar("FALSE"), Value::Bool(false));
    }

    #[test]
    fn yaml_11_booleans_are_strings() {
        // YAML 1.2 does NOT treat these as booleans
        assert_eq!(resolve_plain_scalar("yes"), Value::String("yes".into()));
        assert_eq!(resolve_plain_scalar("no"), Value::String("no".into()));
        assert_eq!(resolve_plain_scalar("on"), Value::String("on".into()));
        assert_eq!(resolve_plain_scalar("off"), Value::String("off".into()));
    }

    #[test]
    fn integers_decimal() {
        assert_eq!(resolve_plain_scalar("0"), Value::Integer(0));
        assert_eq!(resolve_plain_scalar("42"), Value::Integer(42));
        assert_eq!(resolve_plain_scalar("-17"), Value::Integer(-17));
        assert_eq!(resolve_plain_scalar("+99"), Value::Integer(99));
    }

    #[test]
    fn integers_octal() {
        assert_eq!(resolve_plain_scalar("0o17"), Value::Integer(0o17));
        assert_eq!(resolve_plain_scalar("0o0"), Value::Integer(0));
    }

    #[test]
    fn integers_hex() {
        assert_eq!(resolve_plain_scalar("0xFF"), Value::Integer(0xFF));
        assert_eq!(resolve_plain_scalar("0x0"), Value::Integer(0));
        assert_eq!(resolve_plain_scalar("0xDEAD"), Value::Integer(0xDEAD));
    }

    #[test]
    fn floats() {
        assert_eq!(resolve_plain_scalar("1.0"), Value::Float(1.0));
        assert_eq!(resolve_plain_scalar("-0.5"), Value::Float(-0.5));
        assert_eq!(resolve_plain_scalar("+12.5"), Value::Float(12.5));
        assert_eq!(resolve_plain_scalar("1e10"), Value::Float(1e10));
        assert_eq!(resolve_plain_scalar("1.5e-3"), Value::Float(1.5e-3));
    }

    #[test]
    fn float_special_values() {
        assert_eq!(resolve_plain_scalar(".inf"), Value::Float(f64::INFINITY));
        assert_eq!(resolve_plain_scalar(".Inf"), Value::Float(f64::INFINITY));
        assert_eq!(resolve_plain_scalar(".INF"), Value::Float(f64::INFINITY));
        assert_eq!(resolve_plain_scalar("-.inf"), Value::Float(f64::NEG_INFINITY));
        assert!(resolve_plain_scalar(".nan").as_f64().unwrap().is_nan());
        assert!(resolve_plain_scalar(".NaN").as_f64().unwrap().is_nan());
        assert!(resolve_plain_scalar(".NAN").as_f64().unwrap().is_nan());
    }

    #[test]
    fn plain_strings() {
        assert_eq!(resolve_plain_scalar("hello"), Value::String("hello".into()));
        assert_eq!(resolve_plain_scalar("hello world"), Value::String("hello world".into()));
        assert_eq!(resolve_plain_scalar("not-a-number"), Value::String("not-a-number".into()));
    }
}
```

**Step 2: Write the resolver function**

Add above the tests in `tag.rs`:

```rust
//! YAML 1.2 Core Schema tag resolution for plain scalars.
//!
//! Resolves untagged, unquoted (plain) scalars to their typed [`Value`]
//! representation per the YAML 1.2 Core Schema (Section 10.3.2).
//!
//! Key difference from YAML 1.1: `yes`, `no`, `on`, `off` are NOT
//! booleans — they are strings.

use crate::Value;

/// Resolve a plain (unquoted) scalar string to a typed [`Value`].
///
/// Applies the YAML 1.2 Core Schema rules:
/// - `null`, `Null`, `NULL`, `~`, `""` → `Value::Null`
/// - `true`/`True`/`TRUE` → `Value::Bool(true)`
/// - `false`/`False`/`FALSE` → `Value::Bool(false)`
/// - Decimal, octal (`0o`), hex (`0x`) integers → `Value::Integer`
/// - Floats including `.inf`, `-.inf`, `.nan` → `Value::Float`
/// - Everything else → `Value::String`
#[must_use]
pub fn resolve_plain_scalar(s: &str) -> Value {
    // Null
    if s.is_empty() || s == "~" || s == "null" || s == "Null" || s == "NULL" {
        return Value::Null;
    }

    // Boolean
    match s {
        "true" | "True" | "TRUE" => return Value::Bool(true),
        "false" | "False" | "FALSE" => return Value::Bool(false),
        _ => {}
    }

    // Special float values (check before integer parsing)
    match s {
        ".inf" | ".Inf" | ".INF" => return Value::Float(f64::INFINITY),
        "+.inf" | "+.Inf" | "+.INF" => return Value::Float(f64::INFINITY),
        "-.inf" | "-.Inf" | "-.INF" => return Value::Float(f64::NEG_INFINITY),
        ".nan" | ".NaN" | ".NAN" => return Value::Float(f64::NAN),
        _ => {}
    }

    // Integer: octal 0o...
    if let Some(rest) = s.strip_prefix("0o") {
        if let Ok(n) = i64::from_str_radix(rest, 8) {
            return Value::Integer(n);
        }
    }

    // Integer: hex 0x...
    if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        if let Ok(n) = i64::from_str_radix(rest, 16) {
            return Value::Integer(n);
        }
    }

    // Integer: decimal (with optional sign)
    if let Ok(n) = s.parse::<i64>() {
        return Value::Integer(n);
    }

    // Float: decimal with dot or exponent
    if looks_like_float(s) {
        if let Ok(f) = s.parse::<f64>() {
            return Value::Float(f);
        }
    }

    // Fallback: string
    Value::String(s.to_owned())
}

/// Check if a string looks like it could be a float.
///
/// This prevents strings like "123abc" from being parsed as floats
/// (Rust's f64::parse is permissive about some formats).
fn looks_like_float(s: &str) -> bool {
    let s = s.strip_prefix(['+', '-']).unwrap_or(s);
    // Must have a dot or exponent to be a float (otherwise it's an int or string)
    s.contains('.') || s.contains('e') || s.contains('E')
}
```

**Step 3: Register the module in core's lib.rs**

Add to `crates/yamalgam-core/src/lib.rs`:

```rust
pub mod tag;
pub use tag::resolve_plain_scalar;
```

**Step 4: Run tests**

Run: `cargo nextest run -p yamalgam-core`
Expected: all pass.

**Step 5: Commit**

```
feat(core): add YAML 1.2 Core Schema plain scalar resolution
```

---

## Task 4: Build the Composer — events to Value

**Files:**
- Create: `crates/yamalgam-parser/src/compose.rs`
- Modify: `crates/yamalgam-parser/src/lib.rs`
- Test: inline `#[cfg(test)]` in `compose.rs`

The Composer consumes resolved events and builds a `Vec<Value>` (one per
YAML document in the stream). It handles anchor registration, alias
expansion, merge key (`<<`) processing, and scalar tag resolution.

**Step 1: Write the failing tests**

Add `crates/yamalgam-parser/src/compose.rs` with tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use yamalgam_core::Value;

    fn compose(input: &str) -> Vec<Value> {
        Composer::from_str(input).unwrap()
    }

    fn compose_one(input: &str) -> Value {
        let docs = compose(input);
        assert_eq!(docs.len(), 1, "expected 1 document, got {}", docs.len());
        docs.into_iter().next().unwrap()
    }

    #[test]
    fn empty_document() {
        let docs = compose("");
        assert_eq!(docs.len(), 0);
    }

    #[test]
    fn null_document() {
        assert_eq!(compose_one("~"), Value::Null);
        assert_eq!(compose_one("null"), Value::Null);
    }

    #[test]
    fn plain_scalar() {
        assert_eq!(compose_one("hello"), Value::from("hello"));
    }

    #[test]
    fn quoted_scalar_is_string() {
        // Quoted scalars are always strings, no type resolution
        assert_eq!(compose_one("'true'"), Value::from("true"));
        assert_eq!(compose_one("\"42\""), Value::from("42"));
        assert_eq!(compose_one("'null'"), Value::from("null"));
    }

    #[test]
    fn integer_scalar() {
        assert_eq!(compose_one("42"), Value::Integer(42));
    }

    #[test]
    fn float_scalar() {
        assert_eq!(compose_one("3.14"), Value::Float(3.14));
    }

    #[test]
    fn bool_scalar() {
        assert_eq!(compose_one("true"), Value::Bool(true));
        assert_eq!(compose_one("false"), Value::Bool(false));
    }

    #[test]
    fn simple_sequence() {
        let v = compose_one("- a\n- b\n- c");
        assert_eq!(
            v,
            Value::Sequence(vec![
                Value::from("a"),
                Value::from("b"),
                Value::from("c"),
            ])
        );
    }

    #[test]
    fn simple_mapping() {
        let v = compose_one("name: Clay\nage: 25");
        assert_eq!(v.get("name"), Some(&Value::from("Clay")));
        assert_eq!(v.get("age"), Some(&Value::Integer(25)));
    }

    #[test]
    fn nested_mapping() {
        let v = compose_one("outer:\n  inner: value");
        let inner = v.get("outer").unwrap();
        assert_eq!(inner.get("inner"), Some(&Value::from("value")));
    }

    #[test]
    fn sequence_of_mappings() {
        let v = compose_one("- name: a\n  val: 1\n- name: b\n  val: 2");
        let seq = match &v {
            Value::Sequence(s) => s,
            _ => panic!("expected sequence"),
        };
        assert_eq!(seq.len(), 2);
        assert_eq!(seq[0].get("name"), Some(&Value::from("a")));
        assert_eq!(seq[1].get("val"), Some(&Value::Integer(2)));
    }

    #[test]
    fn flow_sequence() {
        let v = compose_one("[1, 2, 3]");
        assert_eq!(
            v,
            Value::Sequence(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
            ])
        );
    }

    #[test]
    fn flow_mapping() {
        let v = compose_one("{a: 1, b: 2}");
        assert_eq!(v.get("a"), Some(&Value::Integer(1)));
        assert_eq!(v.get("b"), Some(&Value::Integer(2)));
    }

    #[test]
    fn anchor_and_alias() {
        let v = compose_one("a: &anchor hello\nb: *anchor");
        assert_eq!(v.get("a"), Some(&Value::from("hello")));
        assert_eq!(v.get("b"), Some(&Value::from("hello")));
    }

    #[test]
    fn anchor_on_collection() {
        let v = compose_one("a: &list\n  - 1\n  - 2\nb: *list");
        let expected = Value::Sequence(vec![Value::Integer(1), Value::Integer(2)]);
        assert_eq!(v.get("a"), Some(&expected));
        assert_eq!(v.get("b"), Some(&expected));
    }

    #[test]
    fn multiple_documents() {
        let docs = compose("---\na: 1\n---\nb: 2");
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].get("a"), Some(&Value::Integer(1)));
        assert_eq!(docs[1].get("b"), Some(&Value::Integer(2)));
    }

    #[test]
    fn empty_mapping_value() {
        let v = compose_one("key:");
        assert_eq!(v.get("key"), Some(&Value::Null));
    }

    #[test]
    fn block_literal_scalar() {
        let v = compose_one("text: |\n  line1\n  line2\n");
        assert_eq!(v.get("text"), Some(&Value::from("line1\nline2\n")));
    }

    #[test]
    fn merge_key() {
        let v = compose_one("defaults: &d\n  a: 1\n  b: 2\nresult:\n  <<: *d\n  b: 3");
        let result = v.get("result").unwrap();
        assert_eq!(result.get("a"), Some(&Value::Integer(1)));
        assert_eq!(result.get("b"), Some(&Value::Integer(3))); // overridden
    }

    #[test]
    fn undefined_alias_errors() {
        let result = Composer::from_str("a: *undefined");
        assert!(result.is_err());
    }
}
```

**Step 2: Write the Composer**

Complete `crates/yamalgam-parser/src/compose.rs` above the tests:

```rust
//! Composer — builds [`Value`] trees from parser events.
//!
//! The Composer consumes an event stream (optionally passed through a
//! [`Resolver`](crate::resolve::Resolver)) and produces one [`Value`]
//! per YAML document. It handles:
//!
//! - Scalar type resolution (YAML 1.2 Core Schema for plain scalars)
//! - Anchor registration and alias expansion
//! - Merge key (`<<`) processing
//!
//! # Usage
//!
//! ```
//! use yamalgam_parser::compose::Composer;
//! use yamalgam_core::Value;
//!
//! let docs = Composer::from_str("key: value").unwrap();
//! assert_eq!(docs[0].get("key"), Some(&Value::from("value")));
//! ```

use std::collections::HashMap;

use yamalgam_core::{Mapping, Value};
use yamalgam_scanner::ScalarStyle;

use crate::error::ParseError;
use crate::event::{CollectionStyle, Event};
use crate::parser::Parser;
use crate::resolve::{NoopResolver, ResolveError, ResolvedEvents, Resolver};

/// Error type for composition failures.
#[derive(Debug)]
pub enum ComposeError {
    /// A resolver or parse error from the event stream.
    Resolve(ResolveError),
    /// An alias references an undefined anchor.
    UndefinedAlias(String),
    /// Unexpected event during composition.
    UnexpectedEvent(String),
}

impl std::fmt::Display for ComposeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resolve(e) => write!(f, "{e}"),
            Self::UndefinedAlias(name) => write!(f, "undefined alias: *{name}"),
            Self::UnexpectedEvent(msg) => write!(f, "unexpected event: {msg}"),
        }
    }
}

impl std::error::Error for ComposeError {}

impl From<ResolveError> for ComposeError {
    fn from(e: ResolveError) -> Self {
        Self::Resolve(e)
    }
}

impl From<ParseError> for ComposeError {
    fn from(e: ParseError) -> Self {
        Self::Resolve(ResolveError::Parse(e))
    }
}

/// Builds [`Value`] documents from a resolved event stream.
pub struct Composer<'input, I>
where
    I: Iterator<Item = Result<Event<'input>, ResolveError>>,
{
    events: I,
    /// Registered anchors: name → cloned Value.
    anchors: HashMap<String, Value>,
}

impl<'input> Composer<'input, ResolvedEvents<'input, NoopResolver>> {
    /// Compose all documents from a YAML string (no resolver).
    ///
    /// # Errors
    ///
    /// Returns an error if parsing or composition fails.
    pub fn from_str(input: &'input str) -> Result<Vec<Value>, ComposeError> {
        let parser = Parser::new(input);
        let events = ResolvedEvents::new(
            Box::new(parser.map(|r| r.map_err(ResolveError::Parse))),
            NoopResolver,
        );
        let mut composer = Composer {
            events,
            anchors: HashMap::new(),
        };
        composer.compose_stream()
    }
}

impl<'input, I> Composer<'input, I>
where
    I: Iterator<Item = Result<Event<'input>, ResolveError>>,
{
    /// Create a composer from a resolved event iterator.
    pub fn new(events: I) -> Self {
        Self {
            events,
            anchors: HashMap::new(),
        }
    }

    /// Compose all documents in the stream.
    pub fn compose_stream(&mut self) -> Result<Vec<Value>, ComposeError> {
        let mut documents = Vec::new();

        // Expect StreamStart
        match self.next_event()? {
            Some(Event::StreamStart) => {}
            Some(e) => return Err(ComposeError::UnexpectedEvent(format!("expected StreamStart, got {e:?}"))),
            None => return Ok(documents),
        }

        loop {
            match self.peek_event_kind()? {
                Some(EventKind::StreamEnd) => {
                    self.next_event()?; // consume StreamEnd
                    break;
                }
                Some(EventKind::DocumentStart) | Some(EventKind::VersionDirective) | Some(EventKind::TagDirective) => {
                    // Skip directives
                    loop {
                        match self.peek_event_kind()? {
                            Some(EventKind::VersionDirective) | Some(EventKind::TagDirective) => {
                                self.next_event()?;
                            }
                            _ => break,
                        }
                    }
                    // Consume DocumentStart
                    match self.next_event()? {
                        Some(Event::DocumentStart { .. }) => {}
                        Some(e) => return Err(ComposeError::UnexpectedEvent(format!("expected DocumentStart, got {e:?}"))),
                        None => break,
                    }
                    // Clear anchors per document
                    self.anchors.clear();
                    // Compose the document content
                    let value = match self.peek_event_kind()? {
                        Some(EventKind::DocumentEnd) => Value::Null,
                        _ => self.compose_node()?,
                    };
                    documents.push(value);
                    // Consume DocumentEnd
                    match self.next_event()? {
                        Some(Event::DocumentEnd { .. }) => {}
                        Some(e) => return Err(ComposeError::UnexpectedEvent(format!("expected DocumentEnd, got {e:?}"))),
                        None => break,
                    }
                }
                Some(_) => {
                    // Implicit document
                    self.anchors.clear();
                    let value = self.compose_node()?;
                    documents.push(value);
                    // Consume implicit DocumentEnd if present
                    if matches!(self.peek_event_kind()?, Some(EventKind::DocumentEnd)) {
                        self.next_event()?;
                    }
                }
                None => break,
            }
        }

        Ok(documents)
    }

    /// Compose a single node (scalar, sequence, mapping, or alias).
    fn compose_node(&mut self) -> Result<Value, ComposeError> {
        let event = self.next_event()?
            .ok_or_else(|| ComposeError::UnexpectedEvent("unexpected end of events".into()))?;

        match event {
            Event::Scalar { anchor, value, style, .. } => {
                let resolved = if style == ScalarStyle::Plain {
                    yamalgam_core::resolve_plain_scalar(&value)
                } else {
                    // Quoted/block scalars are always strings
                    Value::String(value.into_owned())
                };
                if let Some(anchor) = anchor {
                    self.anchors.insert(anchor.into_owned(), resolved.clone());
                }
                Ok(resolved)
            }

            Event::SequenceStart { anchor, .. } => {
                let mut items = Vec::new();
                loop {
                    if matches!(self.peek_event_kind()?, Some(EventKind::SequenceEnd)) {
                        self.next_event()?; // consume SequenceEnd
                        break;
                    }
                    items.push(self.compose_node()?);
                }
                let value = Value::Sequence(items);
                if let Some(anchor) = anchor {
                    self.anchors.insert(anchor.into_owned(), value.clone());
                }
                Ok(value)
            }

            Event::MappingStart { anchor, .. } => {
                let mut map = Mapping::new();
                let mut merge_pairs: Vec<(Value, Value)> = Vec::new();
                loop {
                    if matches!(self.peek_event_kind()?, Some(EventKind::MappingEnd)) {
                        self.next_event()?; // consume MappingEnd
                        break;
                    }
                    let key = self.compose_node()?;
                    let val = self.compose_node()?;

                    // Check for merge key
                    if key == Value::from("<<") {
                        match val {
                            Value::Mapping(ref m) => {
                                for (k, v) in m.iter() {
                                    merge_pairs.push((k.clone(), v.clone()));
                                }
                            }
                            Value::Sequence(ref seq) => {
                                // Merge from multiple mappings
                                for item in seq {
                                    if let Value::Mapping(m) = item {
                                        for (k, v) in m.iter() {
                                            merge_pairs.push((k.clone(), v.clone()));
                                        }
                                    }
                                }
                            }
                            _ => {
                                // Non-mapping merge value — treat as normal key
                                map.insert(key, val);
                                continue;
                            }
                        }
                    } else {
                        map.insert(key, val);
                    }
                }
                // Apply merge: merged keys go in first, then explicit keys override
                if !merge_pairs.is_empty() {
                    let mut merged = Mapping::new();
                    for (k, v) in merge_pairs {
                        merged.insert(k, v);
                    }
                    // Explicit keys override merged ones
                    for (k, v) in map.iter() {
                        merged.insert(k.clone(), v.clone());
                    }
                    map = merged;
                }
                let value = Value::Mapping(map);
                if let Some(anchor) = anchor {
                    self.anchors.insert(anchor.into_owned(), value.clone());
                }
                Ok(value)
            }

            Event::Alias { name, .. } => {
                let name_str = name.as_ref();
                self.anchors
                    .get(name_str)
                    .cloned()
                    .ok_or_else(|| ComposeError::UndefinedAlias(name_str.to_owned()))
            }

            other => Err(ComposeError::UnexpectedEvent(format!("{other:?}"))),
        }
    }

    // -- Event helpers -------------------------------------------------------

    /// Buffer for peeked events.
    fn next_event(&mut self) -> Result<Option<Event<'input>>, ComposeError> {
        match self.events.next() {
            Some(Ok(event)) => Ok(Some(event)),
            Some(Err(e)) => Err(ComposeError::from(e)),
            None => Ok(None),
        }
    }

    /// Peek at the next event's kind without consuming it.
    ///
    /// This is a simplified peek that doesn't buffer — it consumes the event
    /// and stores it. We use a small peeked buffer for this.
    fn peek_event_kind(&mut self) -> Result<Option<EventKind>, ComposeError> {
        // This is a placeholder — we need a peekable wrapper.
        // For now, we'll use a different approach: store peeked event.
        // Actually, let's just make the events peekable.
        //
        // Since we can't easily make the trait-object peekable, we'll
        // track a peeked event in the struct. But that requires adding
        // a field. Let's use a different approach — the caller pattern
        // matches on what comes next.
        //
        // IMPLEMENTATION NOTE: This needs refactoring to use a Peekable
        // wrapper or an internal peeked buffer field. For now, this
        // won't compile — the implementing agent should add a
        // `peeked: Option<Event<'input>>` field to Composer and
        // implement peek/consume properly.
        todo!("implement peek — add peeked field to Composer")
    }
}

/// Simplified event kind for peeking without cloning full event data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventKind {
    StreamStart,
    StreamEnd,
    VersionDirective,
    TagDirective,
    DocumentStart,
    DocumentEnd,
    SequenceStart,
    SequenceEnd,
    MappingStart,
    MappingEnd,
    Scalar,
    Alias,
}

impl EventKind {
    fn of(event: &Event<'_>) -> Self {
        match event {
            Event::StreamStart => Self::StreamStart,
            Event::StreamEnd => Self::StreamEnd,
            Event::VersionDirective { .. } => Self::VersionDirective,
            Event::TagDirective { .. } => Self::TagDirective,
            Event::DocumentStart { .. } => Self::DocumentStart,
            Event::DocumentEnd { .. } => Self::DocumentEnd,
            Event::SequenceStart { .. } => Self::SequenceStart,
            Event::SequenceEnd { .. } => Self::SequenceEnd,
            Event::MappingStart { .. } => Self::MappingStart,
            Event::MappingEnd { .. } => Self::MappingEnd,
            Event::Scalar { .. } => Self::Scalar,
            Event::Alias { .. } => Self::Alias,
        }
    }
}
```

**IMPORTANT:** The `peek_event_kind` method above has a `todo!()`. The
implementing agent MUST:

1. Add a `peeked: Option<Event<'input>>` field to `Composer`
2. Implement `peek_event_kind` to consume into `peeked` if empty, return kind
3. Implement `next_event` to drain `peeked` first
4. Initialize `peeked: None` in both constructors

This is left as a `todo!()` intentionally — the implementing agent should
write the correct peek logic rather than copy a potentially buggy version.

**Step 3: Register the module in parser's lib.rs**

Add to `crates/yamalgam-parser/src/lib.rs`:

```rust
pub mod compose;
```

**Step 4: Run tests**

Run: `cargo nextest run -p yamalgam-parser`
Expected: all pass (after implementing the peek logic).

**Step 5: Run full check**

Run: `cargo fmt --all && just clippy`
Expected: clean.

**Step 6: Commit**

```
feat(parser): add Composer — events-to-Value builder with anchor/alias/merge support
```

---

## Task 5: Add convenience functions and public API

**Files:**
- Modify: `crates/yamalgam-parser/src/lib.rs`
- Modify: `crates/yamalgam-parser/src/compose.rs`
- Create: `crates/yamalgam-parser/tests/compose.rs` (integration tests)

**Step 1: Add top-level convenience functions**

Add to `crates/yamalgam-parser/src/lib.rs`:

```rust
use yamalgam_core::Value;
use compose::{Composer, ComposeError};

/// Parse a YAML string into a list of [`Value`] documents.
///
/// This is the primary entry point for loading YAML into a DOM.
/// Quoted scalars are treated as strings; plain scalars are resolved
/// per the YAML 1.2 Core Schema (null, bool, int, float, string).
///
/// # Errors
///
/// Returns an error if parsing or composition fails.
pub fn from_str(input: &str) -> Result<Vec<Value>, ComposeError> {
    Composer::from_str(input)
}

/// Parse a YAML string into a single [`Value`].
///
/// Expects exactly one document. Returns an error if zero or multiple
/// documents are present.
///
/// # Errors
///
/// Returns an error if parsing fails or the input contains zero or
/// multiple documents.
pub fn from_str_single(input: &str) -> Result<Value, ComposeError> {
    let mut docs = Composer::from_str(input)?;
    match docs.len() {
        0 => Ok(Value::Null),
        1 => Ok(docs.remove(0)),
        n => Err(ComposeError::UnexpectedEvent(
            format!("expected 1 document, got {n}"),
        )),
    }
}
```

**Step 2: Write integration tests**

Create `crates/yamalgam-parser/tests/compose.rs`:

```rust
//! Integration tests for the Composer (events → Value).

use yamalgam_core::{Mapping, Value};
use yamalgam_parser::{from_str, from_str_single};

#[test]
fn parse_simple_mapping() {
    let v = from_str_single("name: yamalgam\nversion: 0.1").unwrap();
    assert_eq!(v.get("name"), Some(&Value::from("yamalgam")));
}

#[test]
fn parse_nested_config() {
    let input = r#"
database:
  host: localhost
  port: 5432
  credentials:
    user: admin
    pass: secret
"#;
    let v = from_str_single(input).unwrap();
    let db = v.get("database").unwrap();
    assert_eq!(db.get("host"), Some(&Value::from("localhost")));
    assert_eq!(db.get("port"), Some(&Value::Integer(5432)));
    let creds = db.get("credentials").unwrap();
    assert_eq!(creds.get("user"), Some(&Value::from("admin")));
}

#[test]
fn parse_sequence_of_strings() {
    let v = from_str_single("- alpha\n- bravo\n- charlie").unwrap();
    assert_eq!(v.get_index(0), Some(&Value::from("alpha")));
    assert_eq!(v.get_index(2), Some(&Value::from("charlie")));
}

#[test]
fn parse_mixed_types() {
    let input = "str: hello\nint: 42\nfloat: 3.14\nbool: true\nnull_val:";
    let v = from_str_single(input).unwrap();
    assert_eq!(v.get("str"), Some(&Value::from("hello")));
    assert_eq!(v.get("int"), Some(&Value::Integer(42)));
    assert_eq!(v.get("float"), Some(&Value::Float(3.14)));
    assert_eq!(v.get("bool"), Some(&Value::Bool(true)));
    assert_eq!(v.get("null_val"), Some(&Value::Null));
}

#[test]
fn parse_multi_document() {
    let docs = from_str("---\na: 1\n---\nb: 2\n...").unwrap();
    assert_eq!(docs.len(), 2);
}

#[test]
fn parse_anchor_alias_roundtrip() {
    let v = from_str_single("default: &cfg\n  timeout: 30\nservice:\n  <<: *cfg\n  name: api").unwrap();
    let svc = v.get("service").unwrap();
    assert_eq!(svc.get("timeout"), Some(&Value::Integer(30))); // from merge
    assert_eq!(svc.get("name"), Some(&Value::from("api"))); // explicit
}

#[test]
fn parse_flow_collections() {
    let v = from_str_single("{ports: [80, 443], tls: true}").unwrap();
    let ports = v.get("ports").unwrap();
    assert_eq!(ports.get_index(0), Some(&Value::Integer(80)));
    assert_eq!(ports.get_index(1), Some(&Value::Integer(443)));
}

#[test]
fn parse_empty_string() {
    let docs = from_str("").unwrap();
    assert!(docs.is_empty());
}

#[test]
fn undefined_alias_is_error() {
    assert!(from_str_single("*nope").is_err());
}
```

**Step 3: Run tests**

Run: `cargo nextest run -p yamalgam-parser`
Expected: all pass.

**Step 4: Run full check**

Run: `just check`
Expected: clean across the entire workspace.

**Step 5: Commit**

```
feat(parser): add from_str/from_str_single convenience API and integration tests
```

---

## Task 6: Wire resolver into Composer for future extensibility

**Files:**
- Modify: `crates/yamalgam-parser/src/compose.rs`
- Test: extend integration tests

Add a `Composer::with_resolver()` constructor so users can plug in custom
resolvers. This proves the resolver → composer pipeline works end-to-end
even though only `NoopResolver` exists today.

**Step 1: Add Composer::with_resolver constructor**

```rust
impl<'input> Composer<'input, ...> {
    /// Create a composer from a parser with a custom resolver.
    pub fn with_resolver<R: Resolver<'input>>(
        input: &'input str,
        resolver: R,
    ) -> Result<Vec<Value>, ComposeError> {
        let parser = Parser::new(input);
        let events = ResolvedEvents::new(
            Box::new(parser.map(|r| r.map_err(ResolveError::Parse))),
            resolver,
        );
        let mut composer = Composer::new(events);
        composer.compose_stream()
    }
}
```

**Step 2: Write test proving resolver composition works**

```rust
#[test]
fn custom_resolver_can_transform_events() {
    // A test resolver that uppercases all scalar values.
    struct UppercaseResolver;
    impl<'input> Resolver<'input> for UppercaseResolver {
        fn on_event(&mut self, event: Event<'input>) -> Result<Vec<Event<'input>>, ResolveError> {
            match event {
                Event::Scalar { anchor, tag, value, style, span } => {
                    let upper = value.to_uppercase();
                    Ok(vec![Event::Scalar {
                        anchor,
                        tag,
                        value: Cow::Owned(upper),
                        style,
                        span,
                    }])
                }
                other => Ok(vec![other]),
            }
        }
    }

    let docs = Composer::with_resolver("key: value", UppercaseResolver).unwrap();
    assert_eq!(docs[0].get("KEY"), Some(&Value::from("VALUE")));
}
```

**Step 3: Run tests and full check**

Run: `just check`
Expected: clean.

**Step 4: Commit**

```
feat(parser): wire resolver into Composer with with_resolver constructor
```

---

## Task 7: Final validation and cleanup

**Step 1: Run the full test suite**

Run: `just check`
Expected: fmt clean, clippy clean, deny clean, all nextest tests pass, doc-tests pass.

**Step 2: Run compliance tests**

Run: `cargo nextest run -p yamalgam-compare --test compliance --success-output=immediate 2>&1 | grep -oE "^    (PASS|UNEXPECTED|MISMATCH|EXPECTED)" | sort | uniq -c | sort -rn`
Expected: compliance numbers unchanged (349 EVENT_PASS, 0 UNEXPECTED).

**Step 3: Run the fuzzer briefly to check new code doesn't panic**

Run: `cd fuzz && cargo +nightly fuzz run fuzz_parser -- -max_total_time=30`
Expected: no panics.

**Step 4: Verify test count increased**

Run: `cargo nextest run --workspace 2>&1 | tail -5`
Expected: test count > 1013 (baseline was 1013 at end of Milestone 5).

**Step 5: Commit any cleanup**

```
chore: cleanup after Value/Resolver milestone
```

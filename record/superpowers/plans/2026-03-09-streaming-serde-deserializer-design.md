# Streaming serde Deserializer (M9)

**Date:** 2026-03-09
**Status:** Draft
**Depends on:** M8 (CST), M7 (tag resolution), M6 (Value DOM + Composer)

## Context

yamalgam has a four-consumer event-stream architecture. Three consumers
exist: Composer (Value DOM), CST (lossless tree), and SAX (future). M9
adds the fourth: a streaming serde Deserializer.

The parser produces `Event<'input>` via
`Iterator<Item = Result<Event, ParseError>>`. Consumers are peers — each
consumes the same event stream independently.

## Goals

1. `yamalgam::from_str::<T>()` as a drop-in replacement for `serde_yaml::from_str`
2. True streaming — no intermediate DOM materialization
3. Parser internals compile once (erased-serde pattern, no monomorphization of event-walking code)
4. First-class multi-document support via `documents()` iterator
5. Resource limit enforcement (LoaderConfig integration)
6. Tag resolution support (Yaml12, Yaml11, Json, Failsafe)

## Non-Goals

- serde Serializer (M13)
- Schema validation during deserialization (M12)
- Async deserialization

## Crate Extraction

M9 extracts existing event consumers into their own crates. The parser
crate keeps its focus on event production.

### Before

| Crate | Contents |
|-------|----------|
| yamalgam-parser | Parser, Composer, CST builder, Resolver |

### After

| Crate | Contents | Depends on |
|-------|----------|------------|
| yamalgam-parser (slimmed) | Event, Parser, Resolver, ResolvedEvents, ParseError | scanner, core |
| yamalgam-compose (extracted) | Composer, ComposeError | parser, core |
| yamalgam-cst (extracted) | CstNode, CstBuilder, parse_to_cst | parser, core |
| yamalgam-serde (new) | Deserializer, Documents, SeqAccess, MapAccess, Error | parser, core, serde, erased-serde |

What stays in core: Value, Mapping, TagResolver, LoaderConfig,
ResourceLimits, diagnostics, spans.

What stays in parser: Event, Parser, Resolver, ResolvedEvents,
ParseError, ScalarStyle, CollectionStyle.

Re-exports: The `yamalgam` CLI crate re-exports
`yamalgam_serde::{from_str, Deserializer}` so users write
`yamalgam::from_str::<T>()`.

## Deserializer Architecture

### Core Struct

```rust
pub struct Deserializer<'input> {
    events: Peekable<Box<dyn Iterator<Item = Result<Event<'input>, DeserializeError>> + 'input>>,
    anchors: HashMap<String, Vec<Event<'input>>>,
    tag_resolver: Box<dyn TagResolver>,
    limits: ResourceLimits,
    anchor_count: usize,
    alias_expansions: usize,
}
```

The event source is `Box<dyn Iterator>` — the same type-erasure pattern
the Composer uses. Accepts Parser directly or ResolvedEvents (for
!include/$ref in M11).

### Event Consumption

Mirrors the Composer:
- `next_event()` / `peek_event()` skip structural events (Comment, BlockEntry, KeyIndicator, ValueIndicator)
- DocumentStart/DocumentEnd consumed at document boundaries
- StreamStart/StreamEnd consumed at stream boundaries

### Anchor Buffering

When the deserializer encounters an anchored node:
- **Scalar with anchor:** clone the event into `anchors[name] = vec![event]`
- **SequenceStart/MappingStart with anchor:** buffer all sub-events (tracking depth) until the matching End event, store in `anchors[name]`
- **On Alias:** create a sub-deserializer that replays from `anchors[name]` (cloned events via VecDeque). Enforce `ResourceLimits::max_alias_expansions`.

This keeps the deserializer fully streaming. Most YAML documents have
zero or few anchors, so the buffer stays empty or small. When anchors
exist, buffering events (small flat structs with Cow references) costs
less than building Values.

### Tag Resolution

- Plain scalars: `TagResolver::resolve_scalar()` determines the type, then calls the appropriate visitor method (`visit_bool`, `visit_i64`, `visit_f64`, etc.)
- Quoted scalars: always `visitor.visit_borrowed_str()` (or `visit_str` for escaped content requiring an owned String)
- Tagged scalars (`!!int`, `!!bool`, etc.): respect the explicit tag

### Erased-Serde Integration

The Deserializer implements `serde::Deserializer<'input>` with all
`deserialize_*` methods. The public `from_str` function:

1. Creates a Deserializer
2. Converts to `&mut dyn erased_serde::Deserializer`
3. Calls `T::deserialize()` through the erased boundary

The entire event-walking implementation compiles once. Only the thin
erased boundary monomorphizes per T.

## Public API

```rust
// === Single-document convenience (drop-in for serde_yaml) ===

/// Deserialize a single YAML document. Errors if input contains multiple documents.
pub fn from_str<'de, T: Deserialize<'de>>(input: &'de str) -> Result<T, Error>

/// Deserialize from a reader. Requires DeserializeOwned (can't borrow).
pub fn from_reader<R: Read, T: DeserializeOwned>(reader: R) -> Result<T, Error>

/// Deserialize with config (limits + tag resolution).
pub fn from_str_with_config<'de, T: Deserialize<'de>>(
    input: &'de str,
    config: &LoaderConfig,
) -> Result<T, Error>

// === Streaming multi-document API (primary interface) ===

impl<'input> Deserializer<'input> {
    pub fn from_str(input: &'input str) -> Self
    pub fn from_str_with_config(input: &'input str, config: &LoaderConfig) -> Self
    pub fn from_reader<R: Read>(reader: R) -> Self
    pub fn from_events<I>(events: I) -> Self
        where I: Iterator<Item = Result<Event<'input>, _>> + 'input

    /// Returns a streaming iterator over documents.
    pub fn documents<T: Deserialize<'input>>(self) -> Documents<'input, T>
}

/// Streaming document iterator (analogous to serde_json's StreamDeserializer).
impl<'input, T: Deserialize<'input>> Iterator for Documents<'input, T> {
    type Item = Result<T, Error>;
}
```

### Drop-in Compatibility with serde_yaml

**Same:** `from_str::<T>()` signature, `from_reader::<R, T>()` signature,
`Deserializer::from_str()` constructor, single-doc-or-error behavior.

**Different:** `documents()` returns `Iterator<Item=Result<T>>` (serde_json
style) instead of `Iterator<Item=Deserializer>` (serde_yaml style).
Config-aware variants via LoaderConfig. `from_events()` for custom
pipelines (resolver middleware).

### Ecosystem Comparison

| Crate | from_str on multi-doc | Multi-doc API | Streaming? |
|-------|----------------------|---------------|------------|
| serde_yaml | Err(MoreThanOneDocument) | Iterator\<Item=Deserializer\> | Lazy |
| serde-saphyr | Err with guidance | from_multiple() -> Vec\<T\> | Eager |
| serde_json | Err(TrailingCharacters) | StreamDeserializer\<Item=Result\<T\>\> | Lazy |
| **yamalgam** | **Err(MoreThanOneDocument)** | **documents() -> Iterator\<Item=Result\<T\>\>** | **Lazy** |

yamalgam follows serde_json's more ergonomic pattern while maintaining
serde_yaml's single-doc error convention.

## Error Type

```rust
#[derive(Debug)]
pub enum Error {
    /// YAML parsing failed (scanner/parser level).
    Parse(ParseError),
    /// Resolver middleware error (!include, $ref).
    Resolve(ResolveError),
    /// Unexpected event during deserialization.
    Unexpected { expected: &'static str, found: String, span: Option<Span> },
    /// Undefined alias reference.
    UndefinedAlias { name: String, span: Option<Span> },
    /// Resource limit exceeded.
    LimitExceeded(String),
    /// Multiple documents in single-document API.
    MoreThanOneDocument,
    /// Custom serde error (from Deserialize impls via serde::de::Error::custom).
    Custom(String),
}
```

Implements `serde::de::Error` (required for serde integration). Carries
`Span` where available for diagnostic tooling. Uses `thiserror` for
Display/Error derives.

## Testing Strategy

1. **Unit tests** — each `deserialize_*` method: scalars (bool, i64, f64, string, null), sequences, mappings, nested structures, enums (unit/newtype/tuple/struct variants), Option, `#[serde(flatten)]`, `#[serde(default)]`
2. **Anchor/alias tests** — scalar anchors, mapping/sequence anchors, alias replay, limit enforcement, undefined alias errors
3. **Multi-document tests** — `documents()` iterator, empty documents, `from_str` error on multi-doc
4. **Drop-in compatibility tests** — port a subset of serde_yaml's test suite to verify behavioral parity
5. **Tag resolution tests** — plain scalar typing under each TagResolution variant (Yaml12, Yaml11, Json, Failsafe)
6. **Integration tests** — real-world YAML (k8s manifests, GitHub Actions, docker-compose) through `from_str::<Value>()` and typed structs
7. **Compliance harness extension** — serde round-trip in YAML Test Suite runner (`from_str::<Value>()` vs Composer output)

## Dependencies

New dependencies for yamalgam-serde:
- `serde = "1.0"` (already in workspace)
- `erased-serde = "0.4"` (new — ~100M downloads, well-maintained, MIT)

No new dependencies for yamalgam-parser (serde removed during extraction).

## Risks

- **Anchor event cloning** — events contain `Cow<'input, str>`, so cloning borrowed events is cheap (pointer copy). Owned events (from escape processing) duplicate the string. Acceptable tradeoff.
- **erased-serde version churn** — 0.4 is stable. Pin in Cargo.toml.
- **Crate extraction scope** — extracting Composer and CST is mechanical but touches many files. Do this as a separate PR before the serde implementation to minimize blast radius.
- **serde enum deserialization** — YAML enum representation (tagged mappings, plain strings for unit variants) has edge cases. Use serde_yaml's conventions for compatibility.

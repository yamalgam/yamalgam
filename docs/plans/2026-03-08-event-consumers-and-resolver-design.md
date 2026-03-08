# Event consumers and resolver middleware — design

**Date:** 2026-03-08
**Status:** Draft
**Depends on:** ADR-0005 (parser events), ADR-0006 (LoaderConfig), ADR-0007 (resolver trait)

## Problem

yamalgam has a scanner and a parser that produce a streaming event sequence.
Nothing consumes those events yet. We need to decide:

1. What consumers exist (the API surface users see).
2. Where `!include` and `$ref` resolution fits in the pipeline.
3. What order to build these layers in.

## Architecture

The parser's event stream is the universal core. All downstream APIs are
**peer consumers** — none sits in front of the others.

```
YAML bytes
    |
yamalgam-scanner (tokens)
    |
yamalgam-parser (events)
    |
resolver middleware (trait-based, composable)
    |
    +-- Streaming serde Deserializer   (zero materialization)
    +-- Value / DOM                    (lossy materialization)
    +-- CST                            (lossless materialization)
    +-- SAX / Callbacks                (zero materialization)
```

See `docs/diagrams/event-stream-architecture.svg` for the visual.

### Four consumer APIs

| API | Materializes? | Preserves comments? | Primary use case |
|-----|:---:|:---:|---|
| Streaming serde | No | No | `from_str::<T>()`, large files, library consumers |
| Value (DOM) | Yes (lossy) | No | Data extraction, `yg get`, serde two-phase |
| CST (lossless) | Yes (lossless) | Yes | Round-trip editing, `yg set`, LSP, linter, emitter |
| SAX / Callbacks | No | No | FFI, WASM, large-file streaming |

Key property: **the user's choice of API determines behavior**. No mode flags,
no ambient state. You call `from_str()` or `parse_value()` or `parse_cst()`,
and the right thing happens.

### Resolver middleware

Resolution (`!include`, `$ref`) sits between the parser and the consumers as
an event-stream transformer. It is implemented as a **trait** so users can
layer custom resolvers (e.g., `!vault`, `!env`, `!sops`) alongside the
built-in ones.

The resolver's behavior is determined by the downstream consumer:

| Consumer | Resolver behavior | Why |
|---|---|---|
| Streaming serde | **Resolve** — replace `!include` with resolved events | Caller wants data, not references |
| Value | **Resolve** | Same |
| CST | **Preserve** — keep original node, attach resolved content as metadata | Caller needs to edit the `!include` line itself |
| SAX | **Resolve** | Same as streaming |

This is implicit in the API, not a flag. `parse_value()` resolves.
`parse_cst()` preserves. The user never thinks about it.

### Resolver trait design (Approach C: hybrid)

```rust
/// An event-stream transformer that can intercept, replace, or annotate events.
///
/// Implementations wrap an event iterator and produce a new one. This gives
/// full control over buffering, lookahead, and recursive parsing (needed for
/// `!include` which must parse the included file's events inline).
pub trait Resolver<'input> {
    /// Wrap the upstream event iterator, producing a transformed one.
    ///
    /// The returned iterator may:
    /// - Pass events through unchanged
    /// - Replace events with resolved content (for `!include`)
    /// - Buffer key-value pairs for lookahead (for `$ref`)
    /// - Emit annotated events with resolved metadata (for CST/LSP)
    fn resolve(
        self,
        events: Box<dyn Iterator<Item = Result<Event<'input>, ResolveError>> + 'input>,
        config: &'input LoaderConfig,
    ) -> Box<dyn Iterator<Item = Result<Event<'input>, ResolveError>> + 'input>;
}
```

Composition via chaining:

```rust
let events = parser.resolve(IncludeResolver::new()).resolve(RefResolver::new());
```

A `SimpleResolver` convenience adapter handles the common case where a
resolver reacts to a single event (tag-based dispatch) without needing
lookahead:

```rust
/// Simplified resolver for tag-based single-event transforms.
pub trait SimpleResolver<'input> {
    /// Check whether this resolver handles the given event.
    fn matches(&self, event: &Event<'input>) -> bool;

    /// Transform the matched event into zero or more replacement events.
    fn transform(
        &mut self,
        event: Event<'input>,
        config: &LoaderConfig,
    ) -> Result<Vec<Event<'input>>, ResolveError>;
}

/// Adapter: wraps a SimpleResolver into a full Resolver.
impl<'input, S: SimpleResolver<'input>> Resolver<'input> for SimpleResolverAdapter<S> { ... }
```

### Error type

```rust
pub enum ResolveError {
    /// Upstream parse error.
    Parse(ParseError),
    /// Include file not found, permission denied, etc.
    Include { path: PathBuf, source: io::Error },
    /// $ref target not found or fetch failed.
    Ref { target: String, source: Box<dyn Error> },
    /// Cycle detected in include/ref chain.
    Cycle { chain: Vec<String> },
    /// Resource budget exhausted.
    LimitExceeded(String),
    /// Custom resolver error.
    Custom(Box<dyn Error + Send + Sync>),
}
```

### Interaction with LoaderConfig

`ResolutionPolicy` (already defined in `yamalgam-core::loader`) governs
resolver behavior:

- `IncludePolicy.enabled` — resolver skips `!include` tags when false
- `IncludePolicy.root` — base path for relative includes
- `IncludePolicy.allow`/`deny` — glob-based path filtering
- `IncludePolicy.max_depth` — recursion limit
- `IncludePolicy.max_total_bytes` — budget across all includes
- `RefPolicy` — analogous for `$ref`

The resolver receives `&LoaderConfig` and enforces policy. The consumer
never sees policy — it just gets events.

### Crate placement

The `Resolver` trait and `ResolveError` live in `yamalgam-core` (shared
infrastructure). Built-in resolver implementations (`IncludeResolver`,
`RefResolver`) can live in a `yamalgam-resolve` crate or in `yamalgam-core`
depending on dependency weight — TBD when we implement them.

## Build order

1. **Resolver trait + pass-through impl** — define the trait, ship a `NoopResolver`
   that passes events through unchanged. Every consumer uses the trait interface
   from day one, so `!include`/`$ref` slots in later without touching consumer code.
2. **Value (DOM)** — first concrete consumer behind the resolver. Proves the
   event-to-tree pipeline. Unlocks `yg get` (read-only extraction).
3. **Streaming serde Deserializer** — second consumer. Unlocks
   `yamalgam::from_str::<T>()` for library adoption.
4. **CST** — third consumer. Unlocks round-trip editing, `yg set`, LSP, linter.
5. **IncludeResolver / RefResolver** — the built-in resolver implementations.
   These can land at any point after step 1 without changing consumer code.

## What this does NOT cover

- CST node representation (arena vs Box, trivia model, error nodes) — separate design doc
- Serde Deserializer architecture (streaming vs two-phase, erased pattern) — separate design doc
- `yg` CLI command design — separate design doc
- Pretty emitter / serializer — separate design doc

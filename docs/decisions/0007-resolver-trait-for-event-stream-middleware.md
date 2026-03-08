---
status: accepted
date: 2026-03-08
decision-makers: [Clay Loveless]
---

# 0007: Resolver trait for composable event-stream middleware

## Context and Problem Statement

yamalgam's parser produces a streaming event sequence (`Iterator<Item = Result<Event, ParseError>>`). The planned `!include` and `$ref` features need to intercept and transform events before consumers (Value, CST, serde, SAX) see them. Users will also want custom transforms for their own tags (`!vault`, `!env`, `!sops`). Where in the pipeline does resolution belong, and what interface does it expose?

This is the pipeline-shape decision: should resolution be baked into each consumer, baked into the parser, or expressed as composable middleware between them?

## Decision Drivers

- **`!include` requires recursive parsing.** An include resolver must open a file, run it through scanner and parser, and splice the resulting events into the stream. This is too complex to inline into every consumer.
- **`$ref` requires lookahead.** A JSON Schema `$ref` is a key-value pair (two events). The resolver must buffer events to detect the pattern. A simple per-event callback is insufficient.
- **Users need custom resolvers.** HashiCorp Vault secrets, environment variable interpolation, SOPS decryption — these are real use cases that yamalgam should enable without forking the parser.
- **CST and Value need different resolver behavior.** Value consumers want includes resolved (flattened into data). CST consumers want includes preserved (the `!include` node stays in the tree, with resolved content attached as metadata). This is implicit in the consumer choice, not a flag.
- **`LoaderConfig::ResolutionPolicy` already defines the security model** (ADR-0006). The resolver must enforce it — sandboxed paths, recursion limits, byte budgets — without each consumer reimplementing policy checks.

## Considered Options

### Option A: Per-event handler

```rust
trait Resolver {
    fn on_event(&mut self, event: Event) -> Result<SmallVec<[Event; 1]>>;
}
```

Each event passes through the resolver, which returns zero or more replacement events.

- Good, because it is simple to implement for tag-based transforms
- Good, because composition is trivial (chain handlers)
- Bad, because it cannot handle `$ref` — that requires buffering a key-value pair across two events
- Bad, because there is no way to express recursive parsing (`!include` needs to spawn a scanner/parser and splice events)

### Option B: Stream transformer

```rust
trait Resolver<'input> {
    fn resolve(
        self,
        events: Box<dyn Iterator<Item = Result<Event<'input>, ResolveError>> + 'input>,
        config: &'input LoaderConfig,
    ) -> Box<dyn Iterator<Item = Result<Event<'input>, ResolveError>> + 'input>;
}
```

The resolver wraps an event iterator and produces a new one. Full control over buffering, lookahead, and recursive sub-parsing.

- Good, because it handles both `!include` (recursive parse + splice) and `$ref` (key-value lookahead)
- Good, because composition is natural — `events.resolve(a).resolve(b)`
- Good, because each resolver owns its iteration state (no shared mutable state between resolvers)
- Bad, because implementing a full `Iterator` wrapper is heavyweight for simple tag-based transforms

### Option C (chosen): Hybrid — stream transformer trait + convenience adapter

Same trait as Option B for the full interface. Additionally, a `SimpleResolver` trait covers the common case (react to one event based on its tag) and an adapter wraps it into the full `Resolver` trait.

- Good, because the full trait handles every use case (includes, refs, lookahead)
- Good, because the simple trait keeps easy things easy (`!env` is 10 lines)
- Good, because composition works the same way regardless of which trait was used
- Bad, because two traits is more API surface to learn (mitigated: `SimpleResolver` is optional, not required)

## Decision Outcome

**Chosen option: Option C** — hybrid with stream transformer as the primary trait and `SimpleResolver` as a convenience adapter.

The resolver trait sits between the parser and all consumers. It is the single extension point for event-stream transformation. Built-in resolvers (`IncludeResolver`, `RefResolver`) and user-defined resolvers use the same interface.

### Key design properties

**Resolver behavior is determined by the consumer, not by a flag.** `parse_value()` runs resolvers in resolve mode (replace `!include` with resolved content). `parse_cst()` runs resolvers in preserve mode (keep the `!include` node, attach resolved content). The user never sets a "mode" — their choice of API implies it.

**Composition is chaining.** Multiple resolvers layer naturally:

```rust
let events = parser
    .resolve(IncludeResolver::new())
    .resolve(RefResolver::new())
    .resolve(VaultResolver::new(client));
```

Each resolver wraps the previous iterator. Order matters — includes are resolved before refs, so `$ref` inside an included file works correctly.

**The `NoopResolver` is the default.** When no resolution is configured, events pass through unchanged with zero overhead (the optimizer can inline the pass-through).

**`LoaderConfig` flows through the resolver.** Policy enforcement (path sandboxing, recursion limits, byte budgets) happens inside the resolver, not in the consumer. Consumers never see `ResolutionPolicy`.

### Crate placement

The `Resolver` trait and `ResolveError` type live in `yamalgam-core` alongside `LoaderConfig`. Built-in resolver implementations ship in a separate crate (`yamalgam-resolve` or similar) to isolate filesystem and network dependencies from the core.

## Consequences

### Positive

- `!include` and `$ref` are implementable without modifying the parser or any consumer
- Custom resolvers use the same trait as built-in ones — no second-class extension API
- The resolve-vs-preserve behavior is implicit in the consumer choice, eliminating a class of configuration errors (cf. yq's mode confusion)
- The trait interface can be defined and integrated now (with `NoopResolver`) so consumers are built against the right abstraction from day one

### Negative

- Two traits (`Resolver` + `SimpleResolver`) is more API surface than a single trait. We accept this because the simple trait is genuinely simpler for the common case and the adapter is mechanical.
- `Box<dyn Iterator>` in the trait signature adds one vtable indirection per resolver in the chain. For YAML parsing where I/O and tokenization dominate, this is noise. If profiling shows otherwise, we can add a generic (non-erased) fast path later.
- The resolve-vs-preserve split means resolvers must be aware of which mode they're operating in. This adds a parameter or associated type to the trait. Exact API TBD during implementation.

### Neutral

- Built-in resolvers (`IncludeResolver`, `RefResolver`) are not implemented yet. This ADR covers the trait design and pipeline shape, not the resolver implementations.
- The `ResolveError` type wraps `ParseError` and adds resolver-specific variants (file not found, cycle detected, budget exceeded, custom). Error conversion from `ParseError` is mechanical.

## More Information

- Design document: [docs/plans/2026-03-08-event-consumers-and-resolver-design.md](../plans/2026-03-08-event-consumers-and-resolver-design.md)
- Architecture diagram: [docs/diagrams/event-stream-architecture.svg](../diagrams/event-stream-architecture.svg)
- Related: [ADR-0005](0005-parser-event-model-with-inline-directives.md) (parser event model), [ADR-0006](0006-loaderconfig-for-resource-limits-and-security-policy.md) (LoaderConfig and ResolutionPolicy)
- Upstream discussion: `ref/architecture-discussion.md` (CST, serde/facet, resolver security surface)

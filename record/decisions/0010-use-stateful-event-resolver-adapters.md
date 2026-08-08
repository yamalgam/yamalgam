---
status: accepted
date: 2026-08-08
decision-makers: [Clay Loveless]
consulted: [Codex post-remediation audit]
supersedes: [ADR-0007]
---

# 0010: Use stateful event resolver adapters across consumers

## Context and Problem Statement

ADR-0007 chose a resolver that owns a full event iterator. It also chose a
separate adapter for simple event changes. It put the trait in
`yamalgam-core`. It promised that CST resolution would keep the source node and
attach the resolved data.

The code did not follow that decision. `yamalgam-parser` now exposes a stateful
`Resolver::on_event` callback. Its `ResolvedEvents` adapter drives that
callback. Composer uses the adapter, but serde, CST, and query still use the
parser directly. The current event type has no field for resolved data. The
shipped trait therefore cannot provide ADR-0007's CST behavior.

Should we replace the working API with ADR-0007's design? Or should we define
the smaller stateful adapter as the contract for every consumer?

## Decision Drivers

- Existing composer users and tests implement `Resolver::on_event` directly.
- A stateful callback can buffer lookahead and suppress events. It can then
  emit zero or more new events.
- An include resolver can parse owned source again. It can emit owned events
  without owning the outer iterator.
- Resolver failures need one typed contract across all consumers.
- Resource and resolution policy must reach resolver code selected by a
  config-aware consumer entry point.
- `Resolver` depends on parser `Event` and `ParseError`; placing it in core
  would invert the existing crate dependency.
- CST must remain byte-lossless even when a resolver transforms event
  semantics or fails.
- CST nodes need a new data model before they can attach resolved data. The
  current API must not imply that support.

## Considered Options

- Implement ADR-0007 literally: move the trait to core, replace the public
  callback with an iterator-owning transformer, add `SimpleResolver`, and add
  resolved-content metadata to CST nodes.
- Keep separate resolver integrations in each consumer.
- Keep the stateful callback and shared adapter, add config-aware dispatch,
  and route every consumer through the same resolved-event contract.
- Leave resolver middleware composer-only until built-in resolvers ship.

## Decision Outcome

Chosen option: "Keep the stateful callback and shared adapter." This
supersedes ADR-0007's trait shape, crate placement, and implicit CST metadata
promise.

`Resolver` remains in `yamalgam-parser`. `ResolvedEvents` remains the only
iterator adapter. It accepts a `LoaderConfig`. The trait gains a callback that
can read that config. Its default calls the existing `on_event` method. Existing
resolver code therefore keeps compiling. A resolver that needs include or ref
policy can override the new callback.

The adapter passes each event and the selected config to the resolver. A
resolver may pass, suppress, replace, or expand events. It may keep state for
lookahead. Nested `ResolvedEvents` adapters provide chaining.

`ResolveError` is the shared error contract. Serde, CST, query, and composer
map its `Parse` variant to their existing parse error. They keep every other
variant as a resolver failure.

Every string consumer uses `ResolvedEvents<NoopResolver>` internally and
offers a public resolver-aware, config-aware path. The no-op path retains the
existing behavior and limits.

CST resolution preserves source bytes. It does not attach resolved data. Tree
classification follows the resolver's output events. Leaf text still comes
from the original source spans.

The CST builder owns source coverage. It emits each source byte at most once,
even if a resolver repeats or reorders visible spans. Suppressed spans return as
gap tokens. A resolver should give synthetic expansion events zero-width spans.
The builder ignores any visible event that starts inside source it has already
covered. Comment text also comes from its source span, not a resolver payload.
A resolver failure produces a typed diagnostic. Recovery keeps all source
bytes. A future include or ref feature may propose a separate CST data model in
another ADR.

### Consequences

- Good, because every current consumer gains middleware without replacing the
  working composer API.
- Good, because existing resolver implementations continue to compile.
- Good, because lookahead and expansion remain possible through resolver state
  and multi-event output.
- Good, because loader policy reaches resolvers through one adapter contract.
- Good, because CST losslessness remains independent of resolver success.
- Bad, because advanced resolvers manage their own buffering state instead of
  owning the upstream iterator.
- Bad, because chained resolvers expose nested adapter types.
- Bad, because CST cannot expose resolved data without a new data model.

### Confirmation

- Existing `Resolver::on_event` implementations compile unchanged.
- A resolver can observe the selected `LoaderConfig` through the adapter.
- Composer passes its default, unchecked, or selected config to the adapter.
- Resolver rewrites reach serde, CST, and query public APIs.
- Resolver failures retain typed variants in every consumer.
- Existing parser errors retain their established consumer variants.
- CST output remains equal to its source after suppression, expansion,
  repeated spans, reordering, and failures.

## Pros and Cons of the Options

### Implement ADR-0007 literally

- Good, because code would match the original design verbatim.
- Good, because iterator ownership makes complex buffering natural.
- Bad, because it breaks the existing resolver trait and composer users.
- Bad, because it adds a new CST data model to this remediation.
- Bad, because core cannot own parser events without changing crate links.

### Integrate resolvers separately in every consumer

- Good, because each consumer could choose a locally convenient type.
- Bad, because errors, config flow, and chaining would diverge immediately.
- Bad, because built-in resolvers would need consumer-specific adapters.

### Use the stateful callback and shared adapter

- Good, because it extends the implemented architecture with minimal API
  disruption.
- Good, because one config-aware adapter covers simple and stateful resolvers.
- Bad, because the resolver cannot directly pull arbitrary upstream events.

### Keep middleware composer-only

- Good, because it requires no code changes.
- Bad, because it leaves the public architecture and documented consumers
  false.

## More Information

- [Superseded ADR-0007](../../docs/decisions/0007-resolver-trait-for-event-stream-middleware.md)
- [Original consumer and resolver design](../../docs/plans/2026-03-08-event-consumers-and-resolver-design.md)
- [Post-remediation audit finding](../audits/2026-08-04-16-m10-post-remediation/README.md#resolver-middleware-cannot-reach-most-documented-consumers)

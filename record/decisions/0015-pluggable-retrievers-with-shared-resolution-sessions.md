---
status: accepted
date: 2026-08-08
decision-makers: [Clay Loveless]
consulted: [Codex M11 design]
---

# 0015: Use pluggable retrievers within shared resolution sessions

## Context and Problem Statement

M11 must retrieve resources from memory, local files, and HTTP without putting
I/O into the parser. Yamalgam parses every retrieved YAML document. It uses its
own scanner, parser, events, and limits. Nested directives also need one cache,
cycle stack, budget, and source graph.

How should Yamalgam separate resource transport from recursive YAML
resolution?

## Decision Drivers

- Keep Yamalgam's parser and consumers independent of filesystem and network
  dependencies.
- Let applications supply resources without forking the built-in resolver.
- Keep current synchronous parser and consumer APIs.
- Share caches, limits, cycle state, and reporting across nested resolution.
- Support memory, file, and HTTP resources through one contract.

## Considered Options

- Add file and HTTP loading directly to `yamalgam-parser`.
- Implement retrieval separately in query, composer, serde, and CST.
- Expose an async retriever and make resolver consumers async.
- Put a synchronous retriever trait behind one shared resolution session.

## Decision Outcome

Chosen option: put a synchronous retriever trait behind one shared resolution
session.

The new `yamalgam-resolve` crate owns `ResourceId`, `ResourceRetriever`,
`RetrievedResource`, `ResolutionSession<R>`, and `BuiltinResolver<R>`. It
depends on core policy and parser events. Core and parser do not depend on it.

`ResourceRetriever` retrieves approved bytes and provenance. It does not parse
YAML or choose a fragment. The first implementations are `MemoryRetriever`,
`FileRetriever`, and `LibrebarRetriever`.

`ResolutionSession<R>` owns one retriever. It also owns the fragmentless fetch
and parse cache, active cycle stack, budgets, source registry, and resolution
report. Cache hits avoid transport and parsing work. Each substitution still
charges its logical edge and materialized bytes.

`BuiltinResolver<R>` composes include and reference behavior and implements the
existing stateful parser `Resolver`. Retrieved bytes always return through
Yamalgam's scanner and parser.

The trait remains synchronous. `LibrebarRetriever` adapts Librebar 0.6.0's
private-runtime and channel pattern behind that boundary. Async runtime types do
not spread through the workspace.

### Consequences

- Good, because callers can replace transport without replacing resolution.
- Good, because all nested directives share one security and accounting state.
- Good, because parser, composer, serde, CST, and query keep synchronous APIs.
- Good, because Yamalgam remains the only YAML implementation in the pipeline.
- Bad, because the HTTP retriever must own runtime and worker coordination.
- Bad, because imported documents require owned bytes and events.
- Bad, because one more crate and public interface increase API surface.

### Confirmation

- Memory, file, and Librebar retrievers pass the same contract tests.
- One session reuses fragmentless fetches while charging every substitution.
- Nested includes and references share cycle, limit, and report state.
- Disabled resolution retains the current no-op path and performs no retrieval.

## Pros and Cons of the Options

### Put transport in the parser

- Good, because recursive parsing would have direct access to transport.
- Bad, because syntax parsing would gain filesystem, HTTP, and policy concerns.
- Bad, because applications could not replace transport cleanly.

### Implement transport in each consumer

- Good, because each consumer could select local behavior.
- Bad, because security, errors, cycles, and caching would diverge.

### Use an async retriever

- Good, because HTTP transport is naturally async.
- Bad, because every synchronous consumer would need an async boundary.

### Use a synchronous retriever and shared session

- Good, because it extends the existing stateful resolver architecture.
- Good, because backends remain pluggable and independently testable.
- Bad, because async transports need an internal adapter.

## More Information

- [M11 external resource resolution design](../superpowers/specs/2026-08-08-m11-external-resource-resolution-design.md).
- [ADR-0010: Use stateful event resolver adapters across consumers](0010-use-stateful-event-resolver-adapters.md).
- [ADR-0012: Specialize parser-backed resolver upstreams](0012-specialize-parser-backed-resolver-upstreams.md).

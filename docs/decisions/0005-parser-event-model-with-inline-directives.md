---
status: accepted
date: 2026-03-07
decision-makers: [Clay Loveless]
---

# 0005: Parser event model — flat events with inline directives

## Context and Problem Statement

The parser consumes scanner tokens and emits semantic events. Two key design choices intersect: how anchors/tags attach to events, and whether directives (`%YAML`, `%TAG`) are consumed by the parser or emitted as events.

These choices determine what information is available to downstream consumers (serde, DOM, CST) and whether round-trip fidelity is achievable without multi-pass reconstruction.

## Decision Drivers

- CST round-trip fidelity is a core differentiator — no other Rust YAML library preserves comments, directives, and style through mutations
- ADR-0002 established "no information loss" at the atom level; this must extend through the event layer
- Comparison harness needs 1:1 event mapping to libfyaml for compliance validation
- Serde and DOM consumers should not be burdened by information they don't need, but must not be the reason information is destroyed

## Considered Options

### Option A: Flat events, anchors/tags on node events, directives consumed

The libfyaml / libyaml / saphyr model. Anchors and tags are fields on `MappingStart`, `SequenceStart`, `Scalar` events. Directives are processed during `DocumentStart` and attached as metadata. Consumers never see raw directive events.

**Pro:** Simple consumer code. Matches libfyaml exactly for comparison.
**Con:** Directive information is destroyed. CST layer cannot reconstruct original directives without a second pass over raw input.

### Option B: Granular event stream, everything separate

Separate `Anchor`, `Tag`, `VersionDirective`, `TagDirective` events emitted inline. Node events carry no anchor/tag metadata — consumers correlate by position.

**Pro:** Maximum information preservation. Uniform event structure.
**Con:** Consumers must buffer and correlate. Serde `Deserializer` needs tag information when encountering a node, not one event earlier. Departs significantly from libfyaml's model, complicating comparison.

### Option C (chosen): Flat events, anchors/tags on nodes, directives as separate events

Hybrid. Anchors and tags attach to node events (libfyaml model) for easy consumption. But directives are separate events in the stream — `VersionDirective` and `TagDirective` appear before `DocumentStart`, not consumed by it.

## Decision Outcome

**Chosen option: Option C** — flat events with inline directives.

Anchors and tags on node events is the pragmatic choice: it matches libfyaml's proven model, simplifies serde and DOM consumers, and keeps comparison testing straightforward.

Directives as separate events is the principled choice: it preserves information that every other Rust YAML library destroys. Serde and DOM consumers skip directive events trivially. CST gets full fidelity without a second pass.

### Monitoring: Option B as future alternative

The XMLReader / SAX model (Option B) where everything is a separate event has value for streaming transformation use cases where consumers need to see children before deciding what to do with a parent node. We may offer a dual API in the future.

**Decision point to watch:** if we find that consumers frequently need to decouple anchors from nodes (e.g., for anchor table construction, or for streaming transforms that rewrite anchors), Option B becomes more attractive. Monitor this before the API stabilizes.

## Consequences

### Positive

- No information loss from scanner through parser — directives, anchors, tags, styles all preserved
- Serde and DOM consumers have simple, familiar event structure
- Comparison harness maps cleanly to libfyaml (filter directive events for 1:1 comparison)
- CST builder gets everything it needs in a single pass

### Negative

- Slight departure from libfyaml's event model (directive events are extra) — comparison must account for this
- Consumers that want directive metadata on `DocumentStart` must correlate manually

### Neutral

- Comment handling deferred — scanner will emit `TokenKind::Comment` inline when CST milestone begins; parser will skip them

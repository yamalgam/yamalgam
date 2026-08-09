---
status: accepted
date: 2026-08-08
decision-makers: [Clay Loveless]
consulted: [Codex M11 design]
---

# 0019: Record resolution reports and defer integrity lockfiles

## Context and Problem Statement

Loading external data creates hidden sources in the final event stream. Users
need to see which files and URLs supplied data. M12 needs the same links for
source navigation. Remote sources will also need hashes and locked loading.

What source record should M11 keep without choosing a lockfile format?

## Decision Drivers

- Expose stable source edges on success and failure.
- Keep source spans and reference chains outside synthetic events.
- Support M12 navigation without another resolver redesign.
- Leave room for strong hashes and source validators.
- Defer update, frozen, and atomic-write behavior.
- Keep credentials and resource bodies out of diagnostics.

## Considered Options

- Emit only tracing and human-readable error messages.
- Persist a complete lockfile in M11.
- Return a flat list of fetched resource names.
- Build a report and leave room for hash data.

## Decision Outcome

Chosen option: every `ResolutionSession` builds a stable `ResolutionReport`.
The fetched-resource result also leaves room for hash data.

The report records each resource ID. It records parent-to-child edges and
directive spans. Each edge keeps the request and selected fragment. Collection
edges keep their matches. Counters track fetched bytes, added bytes, and cache
hits. Failed edges keep their outcome. Callers can read the report after
success or failure. High-level input conversion errors retain reports from all
completed sessions plus the current session. Include edges are created before
fallible resource preflight.

The report is structured data, not a log transcript. Public output sanitizes
resource IDs. It never includes response bodies or credentials. It also omits
cookies and authorization data.

`RetrievedResource` and report entries can carry source data, validators, and a
strong content hash. M11 does not require a hash. It does not enforce a pin.
It writes no report or lockfile.

A later lockfile can map fragmentless resource IDs to hashes and source data.
Pins stay outside `$ref` and `!include` syntax. Librebar's advisory `Lockfile`
can guard atomic writes. Yamalgam will define the file schema later. It will
also define frozen behavior later.

### Consequences

- Good, because source and failure data stays machine-readable.
- Good, because M12 navigation can use the same source graph.
- Good, because future lockfiles have resource IDs and hash seams.
- Good, because M11 does not freeze a file format before real use.
- Bad, because every session retains report state and source links.
- Bad, because reports add public types that need compatibility discipline.
- Bad, because M11 cannot yet guarantee repeatable remote loading.

### Confirmation

- Reports are stable across file enumeration and cache state.
- Success and failure tests keep the same resource and edge context.
- Post-resolution conversion failures keep completed and current reports.
- Preflight failures keep an attempted edge with a terminal failure outcome.
- Public rendering contains no body or credential material.
- Optional hash data survives retriever, session, and report layers.
- M11 performs no report or lockfile writes.

## Pros and Cons of the Options

### Use tracing and display errors

- Good, because it adds no public data model.
- Bad, because callers must parse prose and cannot navigate source edges.

### Implement lockfiles now

- Good, because remote resources become repeatable at once.
- Bad, because M11 would also need update, frozen, merge, and write contracts.

### Return a flat resource list

- Good, because it exposes fetched sources cheaply.
- Bad, because it loses parent edges, directive spans, and failed attempts.

### Build a report and hash seam

- Good, because one model serves errors, navigation, and future locks.
- Bad, because the session must retain more data during loading.

## More Information

- [M11 external resource resolution design](../superpowers/specs/2026-08-08-m11-external-resource-resolution-design.md).
- [ADR-0015: Pluggable retrievers and resolution sessions](0015-pluggable-retrievers-with-shared-resolution-sessions.md).
- [ADR-0017: Sole-key URI references](0017-use-sole-key-uri-references.md).

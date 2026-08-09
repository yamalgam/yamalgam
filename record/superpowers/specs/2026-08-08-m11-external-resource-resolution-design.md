# M11 External Resource Resolution

**Date:** 2026-08-08
**Status:** Accepted

## Overview

Milestone 11 adds secure external resource resolution to Yamalgam. It resolves
local `!include` directives and local or HTTP `$ref` references through a
pluggable retrieval interface while keeping parsing, composition, YAML values,
and event production inside Yamalgam's own stack.

Resolution remains disabled by default. Callers must opt into explicit query
filters or automatic resolution, and every enabled path enforces filesystem,
network, cycle, and resource limits.

## Context

Yamalgam already has the lower-level architecture for resolution:

- `LoaderConfig` separates parser resource limits from external resolution
  policy.
- `yamalgam-parser` exposes a stateful `Resolver` callback and the shared
  `ResolvedEvents` adapter.
- Composer, serde, CST, and query consumers route resolver-aware paths through
  that adapter.
- `NoopResolver` rejects enabled resolution policy instead of silently
  ignoring it.
- The query language reserves `include` and `ref` filters plus automatic CLI
  resolution for M11.

What is missing is the built-in resolver: resource identity, retrieval,
security enforcement, recursive parsing, include aggregation, reference
selection, and usable provenance.

Yamalgam owns its YAML scanner, parser, event model, composer, CST, serde layer,
and query values. M11 keeps that property. Other configuration systems provide
useful behavioral prior art, but no external YAML loader becomes part of the
resolution path.

### Prior art we retain

We borrow deterministic directory ordering and explicit naming and conflict
controls from Ansible. We borrow immutable resource identity and future
content-pinning ideas from Chef Policyfiles. We borrow insertion-context type
validation and dependency visibility from CloudFormation.

We do not borrow Ansible variable precedence, Chef's code-execution or
include-once behavior, or CloudFormation's macro syntax, S3 restriction, and
recursion ban. A resource may be fetched and parsed once, but Yamalgam
materializes it at every reference site.

## Approach

### Crate and component boundaries

We add a `yamalgam-resolve` crate. It depends on `yamalgam-core` for policy and
shared types and on `yamalgam-parser` for events and the `Resolver` contract.
The parser and core crates do not depend on the new crate.

The crate contains five main abstractions:

- `ResourceId` is the canonical identity of a fragmentless local or remote
  resource. A reference fragment belongs to a resolution edge, not the fetch
  identity.
- `ResourceRetriever` is a synchronous trait that retrieves bytes and
  provenance for one approved resource request.
- `RetrievedResource` carries the canonical identity, bytes, and optional
  provenance or integrity metadata supplied by the retriever.
- `ResolutionSession<R>` owns one retriever, the fetch and parse cache, active
  cycle stack, budgets, source registry, and `ResolutionReport`.
- `BuiltinResolver<R>` composes `IncludeResolver` and `RefResolver` and
  implements `yamalgam_parser::Resolver`.

The initial retrievers are:

- `MemoryRetriever` for library callers, deterministic tests, and resources
  that never touch external systems.
- `FileRetriever` for sandboxed local files.
- `LibrebarRetriever` for HTTP and HTTPS resources.

The workspace upgrades Librebar from 0.4 to 0.6.0 and enables its `http`
feature. Yamalgam keeps the existing Librebar CLI, config, logging, and
diagnostics integration.

The trait stays synchronous because every current loader and resolver consumer
is synchronous. `LibrebarRetriever` adapts Librebar 0.6.0's private-runtime and
channel pattern behind that trait. Async runtimes do not leak into parser,
composer, serde, CST, or query APIs.

The CLI crate remains a thin shell. It translates arguments into
`LoaderConfig`, selects retrievers, and invokes resolver-aware core/query APIs.
It does not implement retrieval or resolution behavior.

### Resource identity and bases

The initial input receives a base identity:

- A file input uses its canonical file identity.
- Standard input and constructed in-memory values have no relative base unless
  the caller supplies `--base-uri` or `--resolution-root`.
- A retrieved file resolves relative references from its containing directory.
- A retrieved HTTP resource resolves relative URI references against its URL.

Relative `$ref` targets inherit the containing resource's scheme. Local inputs
therefore produce file references; HTTP inputs produce HTTP references. A
remote resource cannot resolve a local `!include`, because it has no containing
filesystem directory. The caller must use `$ref` for URL composition.

The session caches retrieval and parsing by canonical fragmentless
`ResourceId`. A reference fragment selects from the cached document after
parsing. Cache hits avoid repeated I/O and parsing but do not bypass logical
resolution counts or materialized-byte budgets. Unique retrievals also charge
one cumulative fetched-byte budget before parsing, so selecting small
fragments cannot grow the session cache without bound.

### Resolution policy

`ResolutionPolicy` replaces each `enabled: bool` with a separate mode for
includes and references:

- `Disabled` rejects the capability.
- `Explicit` permits resolution only when the corresponding query filter is
  invoked.
- `Automatic` resolves matching directives during normal loading.

The policy also owns shared filesystem rules, network rules, and session
limits. Existing `LoaderConfig` presets select the following resolution limits
while leaving both capabilities disabled:

| Preset | Maximum depth | Resolution edges | Materialized bytes |
| --- | ---: | ---: | ---: |
| Moderate/default | 10 | 256 | 256 MiB |
| Strict | 5 | 32 | 10 MiB |
| Trusted | 64 | 4,096 | Unlimited |
| Unchecked | Unlimited | Unlimited | Unlimited |

The edge limit charges every attempted substitution, including cache hits.
The byte limit charges content at every materialization site. Fetch caching is
an optimization, not a way to evade resource controls. Checked arithmetic
turns counter overflow into a structured limit error.

The CLI maps policy modes as follows:

- `--allow-includes` selects explicit include resolution.
- `--resolve-includes` selects automatic include resolution.
- `--allow-refs=file,https` selects explicit reference resolution for the
  listed schemes.
- `--resolve-refs=file,https` selects automatic reference resolution for the
  listed schemes.
- Repeated `--allow-ref-host=HOST` entries form the network host allowlist.
- `--resolution-root=DIR` selects the filesystem sandbox root.
- `--base-uri=URI` supplies a base for standard input or anonymous values.

`--allow-*` and `--resolve-*` conflict when supplied for the same capability.
Enabling HTTP or HTTPS requires at least one allowed host. Top-level CLI inputs
remain files or standard input; URLs enter the system through `$ref`.

The `include` filter requires a scalar or structured node tagged `!include`.
The `ref` filter requires a mapping whose only entry is `$ref`. An explicit
filter resolves its selected node and recursively resolves the same capability
within the result. The other capability runs only when its policy mode is
`Automatic`.

### Filesystem boundary

Includes accept relative paths only. File references accept relative paths or
`file:` URI references. Both use the same filesystem policy:

- Resolution starts from the containing file and stays within one canonical
  sandbox root.
- `FileRetriever` holds an opened root-directory capability. It opens relative
  targets through that capability instead of trusting an ambient absolute path
  that was checked earlier.
- The initial file's directory is the default root. Standard input requires an
  explicit root for relative filesystem access.
- Deny patterns run before allow patterns.
- Targets must be regular files. We impose no file-extension restriction.
- Symbolic links are rejected by default.
- When symbolic links are enabled, the resolved target must still remain under
  the canonical sandbox root.
- Missing bases, traversal outside the root, denied targets, and non-regular
  files are hard errors.

Glob and directory discovery follows the same boundary before reading any
matched file. Sorting uses normalized root-relative path bytes, not locale or
filesystem enumeration order. Discovery charges every directory and entry it
visits, including non-matches, against one session budget. Glob traversal
starts at the pattern's static directory prefix and descends only to its
required depth unless the pattern contains `**`. Canonical resources count
once toward the match ceiling even when multiple approved symlink paths reach
them; every alias path still passes policy independently.

### Network boundary

HTTP and HTTPS `$ref` resources use Librebar 0.6.0's HTTP implementation.
Yamalgam applies policy before the request:

- The scheme and hostname must be explicitly allowed.
- URLs containing user information are rejected.
- Redirects are disabled for M11 because Librebar does not yet expose every
  redirect hop for Yamalgam's host validation.
- Requests do not use cookies, authentication, or a persistent HTTP cache.
- Timeouts and body limits come from resolution policy.
- Non-success responses, oversized bodies, and invalid text are hard errors.

The retriever operates on bytes and metadata. Yamalgam parses the response with
its own scanner and parser. Librebar supplies transport, not YAML semantics.

### `$ref` contract

`$ref` uses URI references with RFC 6901 fragments. The directive is valid only
when a mapping contains exactly one `$ref` entry and its value is a string.
Sibling entries are rejected instead of merged, ignored, or silently
discarded.

The fragmentless resource is parsed as exactly one valid YAML document:

- With no fragment, the complete document root replaces the `$ref` mapping.
- An empty fragment also selects the document root.
- A JSON Pointer fragment selects a mapping value by string key or a sequence
  item by index. Non-string YAML mapping keys are not addressable through RFC
  6901.
- Missing targets, malformed pointers, invalid YAML, empty streams, and
  multiple documents are hard errors.

The selected node keeps its native YAML type. Nested references resolve in the
same session. Relative references use the containing resource as their base.
HTTP and HTTPS targets are first-class when their scheme and host pass policy.

The active cycle key is the canonical resource plus fragment. This permits
multiple independent fragments from one cached document while rejecting a
recursive chain that returns to the same selected target.

### `!include` contract

The scalar shorthand includes one YAML node:

```yaml
defaults: !include shared.yaml
```

Mapping merge uses valid YAML merge-key syntax:

```yaml
service:
  <<: !include defaults.yaml
  port: 8080
```

Yamalgam does not accept a tag as a standalone pseudo-entry between mapping
entries. That form is invalid YAML and would make resolution responsible for a
second grammar.

Structured includes use this shape:

```yaml
!include
  path: shared.yaml
  format: yaml
  mode: node
```

Exactly one source selector is required:

- `path` selects one relative file.
- `glob` selects files matching one relative pattern.
- `dir` selects files from one relative directory.

The remaining options are:

| Option | Values | Default | Scope |
| --- | --- | --- | --- |
| `format` | `yaml`, `text` | `yaml` | All selectors |
| `mode` | `node`, `list`, `named`, `merge`, `splice` | `node` | See rules below |
| `recursive` | Boolean | `false` | `dir` only |
| `match` | Filename glob or globs | `*.yaml`, `*.yml` | `dir` only |
| `key` | `stem`, `relative` | `stem` | `named` only |
| `on_conflict` | `error`, `first`, `last` | `error` | Mapping `merge` only |

`match` accepts one filename glob or a sequence of globs. `relative` names use
the path below the selected directory or the static prefix of a glob. They
remove the filename extension and use `/` on every platform. A single `path`
selector uses its filename stem.

Each mode has an explicit type contract:

- `node` requires exactly one matched resource and substitutes its root node.
- `list` produces a sequence containing one result per matched resource.
- `named` produces a mapping keyed by filename stem or extensionless relative
  path. Relative keys use `/` on every platform.
- `merge` accepts homogeneous mapping roots or homogeneous sequence roots. It
  merges mappings under `on_conflict` and concatenates sequences in match
  order.
- `splice` is valid only at a sequence item and requires every YAML root to be
  a sequence. Its items replace the directive item.

Text format reads valid UTF-8 and produces YAML strings. It supports only
`node`, `list`, and `named`. Every YAML resource must contain exactly one valid
document. Zero matches, duplicate derived names, mixed root types, invalid
option combinations, and context mismatches are hard errors.

For `<<: !include`, explicit entries in the containing mapping override
included entries, which preserves YAML merge-key precedence. `on_conflict`
controls collisions among multiple included mapping roots. It never changes
the precedence of an explicit containing entry.

No mode performs implicit flattening, silent overwrite, filename munging, or
special empty-result behavior. Glob and directory results use deterministic
root-relative ordering. Every matched resource participates in the same
filesystem policy, cycle checks, edge counts, and byte budgets as scalar
includes.

### Parsing, anchors, and events

Retrieved YAML bytes run through Yamalgam's scanner and parser with the active
`LoaderConfig`. The session requires exactly one document before selection or
aggregation. Nested directives return to the same session so caches, cycle
state, limits, and reporting remain global to the load.

Imported anchors receive deterministic resource namespaces before their events
enter the outer stream. An imported alias may refer to an anchor inside the
same selected subtree. A fragment whose aliases depend on anchors outside that
subtree is not self-contained and fails resolution.

Injected events use synthetic zero-width spans at the substitution site, as
required by the existing CST coverage contract. The session separately retains
the resource identity, original source span, directive text, selected fragment,
and reference chain for diagnostics and navigation.

Query values record their originating `ResourceId` in metadata. This allows an
explicit `include` or `ref` filter to resolve relative to the node's actual
resource. Constructed values without source metadata require an explicit base
when they contain a relative directive.

### Resolution report

Every session builds a `ResolutionReport` on success or failure. The report is
a deterministic dependency and provenance graph, not a log transcript. It
contains:

- each canonical resource identity and resource kind.
- each parent-to-child resolution edge.
- the directive source span, requested reference, and selected fragment.
- glob or directory matches in materialization order.
- fetched and materialized byte counts.
- cache-hit state.
- optional retriever provenance or integrity metadata.
- the terminal outcome for failed edges.

The session exposes the current report by reference and can return the finished
report when resolution ends. High-level integrations can attach the report to
diagnostics without changing the event representation. M11 does not add a
`yg deps` command or persist reports to disk.

Reports never contain response bodies, credentials, cookies, or authorization
data. Public rendering uses a sanitized resource identity; the session keeps
the full internal identity required for retrieval and cache correctness.

The report serves three later features without redesign:

- M12 source navigation and go-to-definition.
- a dependency inspection command.
- a reproducible resolution lockfile.

### Integrity seam and deferred lockfiles

`RetrievedResource` and report entries can carry optional resource provenance,
validators, and a strong content digest. M11 preserves these fields and their
ownership boundary but does not require a digest, enforce a pin, or define a
lockfile format.

A later lockfile can map canonical fragmentless resource identities to content
digests and retriever provenance. `$ref` remains a sole-key mapping and
`!include` syntax remains unchanged; pins live in the external policy or lock
record. Librebar's advisory `Lockfile` can coordinate atomic lockfile writes,
but that feature is process locking rather than the dependency-lock schema
itself. M11 performs no lockfile I/O.

### Errors and diagnostics

Resolver failures remain typed through `ResolveError` and consumer-specific
wrappers. M11 adds structured categories for:

- malformed directives and incompatible insertion contexts.
- disabled capabilities and policy denials.
- missing bases and invalid resource identifiers.
- file and HTTP retrieval failures.
- invalid UTF-8, invalid YAML, and document-count failures.
- missing or non-self-contained fragments.
- mapping conflicts and aggregation type mismatches.
- cycles and resource-limit failures.

Every error identifies the directive site and includes the resource/reference
chain when one exists. The report records the same failure without reducing it
to display text. A resolver stops after its first terminal error, matching the
current `ResolvedEvents` contract.

## Data flow

For each root input, resolution follows one pipeline:

1. The caller assigns the root resource identity and creates one
   `ResolutionSession`.
2. Yamalgam parses the root through `ResolvedEvents<BuiltinResolver<_>>`.
3. The resolver recognizes an enabled include or sole-key `$ref` directive.
4. The session resolves its base, canonicalizes the target, and enforces
   filesystem or network policy.
5. The retriever returns bytes and provenance, or the session reuses its cache.
6. Yamalgam parses exactly one document and recursively resolves enabled
   directives through the same session.
7. The resolver selects a fragment or aggregates include matches, validates
   the insertion context, namespaces anchors, and emits owned replacement
   events.
8. The session charges logical edges, fetched bytes, discovery visits, and
   materialized bytes and records the outcome in `ResolutionReport`.
9. The existing consumer builds values, serde output, CST classification, or
   query values from the resolved event stream.

Disabled resolution retains the current `NoopResolver` path and performs no
filesystem or network work.

## Testing

Implementation follows red-green-refactor and adds behavior tests with every
change. The test matrix covers:

- canonical `ResourceId` and base resolution for file, memory, and HTTP
  resources.
- memory and file retrievers, canonical sandboxing, deny-before-allow,
  symlinks, and regular-file enforcement.
- Librebar HTTP success, host/scheme denial, redirects, timeouts, status
  failures, invalid text, and body limits against a local test server.
- whole-document and RFC 6901 reference selection, same-document references,
  sibling rejection, fragments, native types, and cycles.
- scalar and structured include modes, deterministic aggregation, conflicts,
  text loading, zero matches, and context/type errors.
- recursive cross-capability mode behavior for explicit and automatic policy.
- cache reuse with per-edge, fetched-byte, and discovery-work budget charging.
- anchor namespaces and non-self-contained fragment rejection.
- deterministic success and failure reports, sanitized rendering, and optional
  integrity metadata.
- composer, serde, CST, query, and CLI integration.
- proof that disabled policy performs no retrieval.

Focused crate suites run during implementation. `just test` and `just check`
are the final repository gates. The existing YAML Test Suite remains unchanged
because resolution is disabled by default.

## Alternatives considered

### Use an existing YAML library's include implementation

This would reduce initial code. It would also create a second parser and value
model inside the one feature that most needs Yamalgam's events, tags, anchors,
spans, limits, and diagnostics. We keep external systems as semantic prior art
and build on Yamalgam's stack.

### Put retrieval directly in the parser or query crate

Parser integration would couple syntax to I/O and invert the existing resolver
boundary. Query-only integration would leave composer, serde, and CST with
different behavior. A separate resolver crate extends the shipped middleware
without contaminating lower layers.

### Expose an async retrieval trait

HTTP clients are naturally async. Every current Yamalgam consumer is
synchronous, so an async trait would spread runtime requirements through the
workspace. A synchronous interface with Librebar's adapter pattern contains
that cost in the HTTP retriever.

### Ship local resolution first and defer HTTP

This would reduce M11's network surface. Librebar 0.6.0 already supplies the
transport and synchronous adapter shape, so omitting URLs would create a
short-lived API and leave relative remote references incomplete. We include
HTTP now with redirects and ambient credentials disabled.

### Merge `$ref` siblings

Several ecosystems treat sibling keys as overlays or annotations. That
behavior is order-sensitive and easy to misread. Yamalgam replaces only an
unambiguous sole-key `$ref` mapping and rejects siblings.

### Add standalone include syntax outside YAML

Copier-like or preprocessor syntax can place an include between mapping lines.
It requires a second grammar and makes scanner, CST, and diagnostics disagree
about the source. Mapping merge, node substitution, and sequence splicing cover
the useful cases with valid YAML.

### Implement lockfiles in M11

Remote resources make reproducibility important, but a lockfile introduces
update, frozen, conflict, serialization, and atomic-write contracts. The
resolution report and integrity seam preserve the required data now. We defer
the persistence contract until resolution behavior has real-world use.

### Parameterize included resources

CloudFormation-style macro parameters and nested-stack inputs are powerful.
They turn loading into templating and code evaluation. M11 composes data only.
Parameterized components belong to a later design.

## Consequences

We gain one resolution architecture across library and CLI consumers, with
local, in-memory, and remote resources behind a replaceable interface. Includes
cover single nodes, mapping composition, sequence composition, deterministic
collections, and text. References support complete resources and RFC 6901
fragments without collapsing YAML types. Reports make otherwise invisible
dependencies observable and prepare M12 navigation.

We also accept real costs. Resolution requires owned buffering for imported
documents. A synchronous HTTP adapter owns a private runtime and worker
coordination. Anchor names change internally when documents are combined.
Policy configuration becomes larger, and deterministic aggregation rejects
several permissive behaviors users may know from other tools.

The security boundary is intentionally restrictive. Network access requires
scheme and host opt-in. Redirects, ambient credentials, cookies, and persistent
HTTP caching are unavailable. Includes cannot fetch URLs. Symlinks are denied
unless explicitly enabled and contained.

We defer dependency lockfiles, frozen resolution, redirect-hop validation,
authenticated retrieval, HTTP includes, persistent HTTP caching, parameterized
includes, and a dependency-inspection CLI. The interfaces preserve room for
those features without promising their eventual syntax.

## Non-goals

- Replacing Yamalgam's scanner, parser, composer, CST, serde, or value model.
- Adding a general template or macro language.
- Resolving directives when policy remains disabled.
- Treating top-level URLs as ordinary CLI input files.
- Supporting binary raw includes.
- Silently merging `$ref` siblings, include conflicts, or mixed aggregates.
- Persisting resolution reports or lockfiles in M11.
- Enabling redirects before every hop can be validated.

## Related decisions

- [ADR-0006: LoaderConfig for resource limits, resolution policy, and security boundaries](../../../record/decisions/0006-loaderconfig-for-resource-limits-and-security-policy.md)
- [ADR-0010: Use stateful event resolver adapters across consumers](../../../record/decisions/0010-use-stateful-event-resolver-adapters.md)
- [ADR-0011: Reject unhandled resolution policy on no-op paths](../../../record/decisions/0011-reject-unhandled-resolution-policy.md)
- [ADR-0015: Use pluggable retrievers within shared resolution sessions](../../../record/decisions/0015-pluggable-retrievers-with-shared-resolution-sessions.md)
- [ADR-0016: Use three resolution modes with explicit security boundaries](../../../record/decisions/0016-use-three-mode-resolution-policy.md)
- [ADR-0017: Replace sole-key URI references with RFC 6901 targets](../../../record/decisions/0017-use-sole-key-uri-references.md)
- [ADR-0018: Use structured includes for deterministic typed aggregation](../../../record/decisions/0018-use-structured-include-aggregation.md)
- [ADR-0019: Record resolution reports and defer integrity lockfiles](../../../record/decisions/0019-record-resolution-reports-and-integrity-metadata.md)
- [Draft-01 query language: external resolution](../../../docs/spec/yg-query-language-spec-draft-01.md#7-external-resolution)

---
audit: 2026-08-11-07-m12-full-repo
last_updated: 2026-08-11
status:
  fixed: 17
  mitigated: 0
  accepted: 0
  disputed: 0
  deferred: 0
  open: 3
---

# Actions Taken: M12 full-repository audit

Remediation status for the [2026-08-11 M12 full-repository audit](README.md).

---

## 2026-08-11 — Preserve leading document-marker semantics

**Disposition:** fixed  
**Addresses:** [leading-document-marker-shifts-lsp-document-indexes](README.md#leading-document-marker-shifts-lsp-document-indexes)  
**Commit:** 1b3b14b726f37941c203c8847c026475aac38a12  
**Author:** Codex

`DocumentState` now derives a byte offset's semantic document index from the
recovering CST's `Document` nodes. Completion, hover, definition, and syntax
diagnostics all use that shared boundary, so an initial `---` remains part of
document zero while later markers advance normally. Four consumer-level
regressions cover the boundary; `just test-lsp` passes 18 tests.

---

## 2026-08-11 — Resolve sequence-item schema context

**Disposition:** fixed  
**Addresses:** [sequence-item-schema-context-is-unreachable](README.md#sequence-item-schema-context-is-unreachable)  
**Commit:** 69ea147b3c89544a8153d034c03abfb01e906fe8  
**Author:** Codex

Completion and hover now derive JSON Pointer paths from recovering CST
ancestry. Block sequence entries use their semantic sibling index; flow
sequences count their direct comma separators. The fallback probe stays inside
the current line, preserving partial `- ` recovery without inheriting the
previous mapping entry. Block completion, flow completion, and sequence-value
hover regressions pass with the full 21-test LSP suite and workspace Clippy.

---

## 2026-08-11 — Align Ropey line boundaries with LSP

**Disposition:** fixed

**Addresses:** [ropey-unicode-lines-desynchronize-lsp-positions](README.md#ropey-unicode-lines-desynchronize-lsp-positions)

**Commit:** 53da91348fd819edbf437d1287d35048a24db1ec

**Author:** Codex

The CST now disables Ropey's Unicode-line feature while retaining CR-line and
SIMD support. LSP UTF-16 conversions therefore treat U+2028 and U+2029 as
content while honoring CR, LF, and CRLF as line boundaries. A bidirectional
regression passes with all 54 CST tests, all 21 LSP tests, and workspace Clippy.

---

## 2026-08-11 — Render schema examples in hover

**Disposition:** fixed

**Addresses:** [documented-example-hover-is-not-implemented](README.md#documented-example-hover-is-not-implemented)

**Commit:** ef26ca4b44d7b9515ff444c421a4aa626ee4a1a1

**Author:** Codex

Schema navigation now collects the first eight unique JSON Schema examples in
deterministic traversal order, including examples contributed by combinators.
Hover renders those compact JSON values as plain text, so an example-only
schema produces useful output. The regression passes with all 41 schema tests,
all 22 LSP tests, and workspace Clippy.

---

## 2026-08-11 — Make LSP request cancellation responsive

**Disposition:** fixed

**Addresses:** [lsp-cancellation-cannot-interrupt-work-and-retains-request-ids](README.md#lsp-cancellation-cannot-interrupt-work-and-retains-request-ids)

**Commit:** 8f5e87ff44ed9d539dd5ca3f29a2fd982a06897c

**Author:** Codex

The receiver loop now dispatches request work to a bounded four-worker queue
over immutable document snapshots. Active request generations carry
cooperative cancellation flags; cancellation responds immediately, suppresses
stale completion, and removes the exact generation so reused IDs remain clean.
Unknown and late cancellation IDs are ignored. The lifecycle and deterministic
held-work regressions pass with all 23 LSP tests and workspace Clippy.

---

## 2026-08-11 — Preserve stdio failures without blocking on input

**Disposition:** fixed

**Addresses:** [stdio-error-path-can-hang-joining-reader](README.md#stdio-error-path-can-hang-joining-reader)

**Commit:** f4d4ba7a4000c12f65b5dd5e304dfdf02557dce6

**Author:** Codex

The stdio entry point now returns a server failure before attempting to join
the transport threads, avoiding a wait on a reader blocked on still-open
standard input. Clean protocol exits retain the existing join behavior. Both
lifecycle branches pass with all 25 LSP tests and workspace Clippy.

---

## 2026-08-11 — Surface definition resource failures

**Disposition:** fixed

**Addresses:** [lsp-definition-resource-errors-erased](README.md#lsp-definition-resource-errors-erased)

**Commit:** 56ff7a46da10d80e6f7c58036e5f46d81623c08c

**Author:** Codex

Definition lookup now distinguishes `Ok(None)` from typed workspace policy,
retrieval, UTF-8, and URI failures. The LSP server maps those failures to
`RequestFailed` with a fixed path-safe message while retaining full typed
causes inside the library. Library and protocol regressions pass with all 27
LSP tests and workspace Clippy.

---

## 2026-08-11 — Preserve schema validator cache identity

**Disposition:** fixed

**Addresses:** [schema-validator-cache-key-erases-identity-and-options](README.md#schema-validator-cache-key-erases-identity-and-options)

**Commit:** 4911f5714c297e3d65a296356013a2e332fb9c5f

**Author:** Codex

The validator cache now uses a typed key containing the canonical
`ResourceId`, semantic schema digest, and diagnostic limit. Query-bearing
schema identities no longer collide through redacted display strings, and a
validator compiled for one report cap cannot be reused for another. Both
regressions pass with all 29 LSP tests and workspace Clippy.

---

## 2026-08-11 — Bound native schema instance depth

**Disposition:** fixed

**Addresses:** [schema-native-adapter-unbounded-recursion](README.md#schema-native-adapter-unbounded-recursion)

**Commit:** 8e3425f2751b101cc9ddd9799b8669b4c47032a8

**Author:** Codex

Schema validation now rejects instances deeper than
`SchemaOptions::max_instance_depth`, which defaults to 256, before entering
`jsonschema`. The typed `yaml/depth-limit` diagnostic retains the first
excessive instance path. Native equality and JSON materialization use explicit
work stacks; a 1,024-level regression covers both fallbacks. All 43 schema
tests, all 29 LSP tests, and workspace Clippy pass.

---

## 2026-08-11 — Bound schema diagnostic work

**Disposition:** fixed

**Addresses:** [schema-diagnostic-cap-does-not-bound-work](README.md#schema-diagnostic-cap-does-not-bound-work)

**Commit:** ad9668ea3c5fb949f0b7aea4eb9b464aed9ea9e3

**Author:** Codex

Representation scanning and schema error iteration now stop after
`max_diagnostics` plus one sentinel. Only the retained errors incur diagnostic
conversion and span lookup. Limit warnings intentionally report “more than N”
instead of claiming an exact suppressed count, and a zero limit still reports
an invalid result through the sentinel warning. All 46 schema tests, all 29 LSP
tests, and workspace Clippy pass.

---

## 2026-08-11 — Release the schema session lock during loads

**Disposition:** fixed

**Addresses:** [schema-session-holds-global-lock-across-retrieval](README.md#schema-session-holds-global-lock-across-retrieval)

**Commit:** 60c143d8a6df70f1720e1a173ec0263d2ee2e2d3

**Author:** Codex

Schema loads now reserve per-key ownership and aggregate byte capacity under
the session mutex, then release it for backend retrieval, UTF-8 decoding, and
composition. Concurrent same-key callers share one published document;
unrelated keys can retrieve concurrently within disjoint byte reservations.
RAII cleanup releases failed or panicked work, and same-thread re-entry returns
a typed error instead of deadlocking. All 49 schema tests, all 29 LSP tests,
and workspace Clippy pass.

---

## 2026-08-11 — Share cached schema document handles

**Disposition:** fixed

**Addresses:** [schema-session-cache-clones-located-documents](README.md#schema-session-cache-clones-located-documents)

**Commit:** d07192d8a82e4f0de53dab6e5cbba32ae0c2e4e2

**Author:** Codex

The schema session now stores and returns `Arc<SchemaDocument>` handles, so a
cache hit no longer clones the semantic value and location trees. Each
document retains its semantic digest when parsed, and the LSP validator cache
uses that retained value without another tree walk. The shared-handle
regression passes with all 50 schema tests, all 29 LSP tests, and workspace
Clippy.

---

## 2026-08-11 — Precompile schema navigation patterns

**Disposition:** fixed

**Addresses:** [schema-navigation-recompiles-patterns-per-request](README.md#schema-navigation-recompiles-patterns-per-request)

**Commit:** bdded7d00e8af192932a5d4ef15a0b14537a0af0

**Author:** Codex

Navigation resources now compile valid `patternProperties` expressions once
when the index is built and retain them beside their schema-node pointers.
Completion and hover path traversal reuse those automata instead of recompiling
patterns on every lookup. A structural regression covers retained valid
patterns and ignored invalid patterns; all 51 schema tests, all 29 LSP tests,
and workspace Clippy pass.

---

## 2026-08-11 — Normalize recovery spans in one pass

**Disposition:** fixed

**Addresses:** [recovering-cst-span-normalization-is-quadratic](README.md#recovering-cst-span-normalization-is-quadratic)

**Commit:** fec2f80fdf4bc32d6c0ef67627dd714f39e9fa60

**Author:** Codex

Recovery assembly now retains a running source mark and shifts each parsed
chunk's relative spans once. The completed tree no longer recomputes every
node and token endpoint by rescanning from byte zero. The scaling regression
drops the 4,096-line fixture from 3.37 seconds to 15 milliseconds; all 55 CST
tests, all 29 LSP tests, and workspace Clippy pass.

---

## 2026-08-11 — Batch editor snapshot rebuilds

**Disposition:** fixed

**Addresses:** [lsp-didchange-rebuilds-full-document-stages](README.md#lsp-didchange-rebuilds-full-document-stages)

**Commit:** 660a8a4a838d0df8222dca83e16d3e28c94fe75d

**Author:** Codex

`didChange` now applies sequential UTF-16 edits to one shared source snapshot,
updates the incremental CST once, and composes located values once. Minimal
prefix/suffix differencing retains document-local CST reparsing. Schema
selection consumes those precomposed values and retained composition failures
instead of copying and composing the source again. All 56 CST tests, all 52
schema tests, all 30 LSP tests, and workspace Clippy pass.

---

## 2026-08-11 — Index diagnostic positions per document version

**Disposition:** fixed

**Addresses:** [lsp-diagnostic-positioning-recopies-source](README.md#lsp-diagnostic-positioning-recopies-source)

**Commit:** 75b67d2e87ac1745b5f365cf0ba29a228abf334c

**Author:** Codex

Each accepted document snapshot now retains a CR/LF-aware line index used by
diagnostic byte-to-UTF-16 conversion. Endpoint lookup no longer materializes
the Rope, and the CST conversion APIs expose byte length directly while
counting UTF-16 units over Rope slices instead of temporary strings. All 56
CST tests, all 30 LSP tests, and workspace Clippy pass.

---

## 2026-08-11 — Enforce located-value tree shape

**Disposition:** fixed

**Addresses:** [located-value-allows-invalid-parallel-shapes](README.md#located-value-allows-invalid-parallel-shapes)

**Commit:** 528edd0be8e61007163482788e4f75506cb63bf5

**Author:** Codex

`LocatedValue` now seals its semantic and location fields behind checked
construction and read-only accessors. The checked constructor validates kind,
cardinality, and every nested mapping key/value pair with an explicit work
stack; bottom-up scalar, sequence, and mapping constructors preserve the same
invariant without repeatedly walking completed subtrees. Composer merge,
alias, schema, LSP, and CLI consumers now use that API. The invariant
regression passes with all 77 core tests, all 66 composer tests, all 52 schema
tests, all 30 LSP tests, and workspace Clippy.

---

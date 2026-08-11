# yamalgam Roadmap

Overall project plan, organized by milestone. Each milestone has a design doc and implementation plan in this directory. ADRs in `record/decisions/`.

## Pipeline Architecture

```
YAML bytes → Scanner (tokens) → Parser (events) → Resolver middleware
                                                        │
                    ┌───────────┬───────────────┬───────┴───────┐
                    │           │               │               │
              Streaming     Value (DOM)       CST          SAX/Callbacks
              serde Deser   (lossy)        (lossless)      (zero alloc)
```

See `record/diagrams/event-stream-architecture.svg`.

---

## Milestone 1 — Scanner Foundation
**Status:** Complete (7 PRs merged)

Scanner crate scaffolding, core types (Token, Span, Mark), input layer, comparison infrastructure against libfyaml.

- Design: `2026-03-06-scanner-foundation-design.md`
- Plan: `2026-03-06-scanner-foundation-plan.md`
- ADRs: 0001 (state machine port), 0002 (atom/CST fidelity), 0003 (comparison — deprecated), 0004 (UTF encoding)

## Milestone 2 — Scanner State Machine
**Status:** Complete (10 PRs, #9-#18)

Full YAML token type coverage: block/flow scalars, collections, anchors, aliases, tags, directives, document markers. Token queue, indent stack.

## Milestone 3 — Scanner Compliance
**Status:** Complete (PRs #20-#25, #28-#46)

97.7% YAML Test Suite compliance (343/351). Simple key stack, error validations, fail:true rejections. 143 scanner unit tests.

- Plan: `2026-03-07-scanner-fail-true-rejections.md`

## Milestone 4 — Parser Layer
**Status:** Complete (PRs #48-#53)

StAX-style pull parser, 22 states, 13 event types. Iterator-based streaming interface. 349/351 event compliance, 0 UNEXPECTED.

- Design: `2026-03-07-parser-layer-design.md`
- Plan: `2026-03-07-parser-layer-plan.md`
- ADR: 0005 (parser event model)

## Milestone 5 — Security, Fuzzing, Benchmarks
**Status:** Complete (PRs #58-#67)

LoaderConfig with ResourceLimits + ResolutionPolicy. 6 cargo-fuzz targets, comparative benchmarks against 7 peers. 1013 tests.

- Design: `2026-03-07-fuzzing-benchmarks-loaderconfig-design.md`
- Plan: `2026-03-07-fuzzing-benchmarks-loaderconfig-implementation.md`
- ADR: 0006 (LoaderConfig)

## Milestone 6 — Resolver Trait + Value DOM
**Status:** Complete

Resolver trait (composable event-stream middleware), NoopResolver, Value type, YAML 1.2 Core Schema scalar resolution, Composer (events → Value with anchor/alias/merge support), convenience API. 1069 tests.

- Design: `2026-03-08-event-consumers-and-resolver-design.md`
- Plan: `2026-03-08-resolver-and-value-plan.md`
- ADR: 0007 (resolver trait)

---

## Milestone 7 — Tag Resolution Trait
**Status:** Complete

Pluggable tag resolution for plain scalar typing via `TagResolver` trait. Four built-in implementations: `Yaml12TagResolver` (default), `FailsafeTagResolver`, `JsonTagResolver`, `Yaml11TagResolver`. `TagResolution` enum in `LoaderConfig`, `Box<dyn TagResolver>` in Composer.

- Design: `2026-03-09-tag-resolution-trait-design.md`
- Plan: `2026-03-09-tag-resolution-trait-plan.md`

Prerequisite for yg CLI's `--schema` flag and `resolve_as()` filter.

## Milestone 8 — CST (Concrete Syntax Tree)
**Status:** Complete (PRs #80-#81)

Lossless tree preserving comments, whitespace, quoting style. Full-fidelity
event stream (Comment + structural indicator events emitted inline), Box
allocation, whitespace recovered from span gaps, error recovery nodes.
Round-trip fidelity `cst.to_text() == input`: 343/351 YAML Test Suite
(8 flow-mapping close-token cases allowlisted). Unlocks round-trip editing,
`yg -i`, linter, pretty emitter.

- Design: `2026-03-09-cst-design.md`, `2026-03-09-full-fidelity-event-stream-design.md`

## Milestone 9 — Streaming serde Deserializer
**Status:** Complete

`yamalgam::from_str::<T>()` drop-in for serde_yaml, plus
`Deserializer::documents::<T>()` for multi-doc streaming. Erased-serde
pattern internally — parser event-walking never monomorphizes. Anchor
buffering via event replay (aliases expand at buffer time — compose-time
semantics, immune to redefinition cycles). Config-aware
(`from_str_with_config`). Consumer crates extracted along the way:
`yamalgam-compose`, `yamalgam-cst`.

Integration: `Deserialize for Value`, serde round-trip in the compliance
harness — Composer and serde agree on all 351 YAML Test Suite files with
an empty allowlist. 408-file real-world fixture corpus (yq, yamlfmt,
prettier, hand-written k8s/compose/CI/ansible/helm), serde_yaml parity
tests with documented divergences. 1699 tests.

Single-pair flow-sequence entries (`[a: b]`, `[ : ]`, spec example 7.21)
parse and CST-round-trip as of the post-M9 fixes; the remaining
`KNOWN_FAILING` fixture gap is explicit `? key` entries without values.

- Design: `2026-03-09-streaming-serde-deserializer-design.md`
- Plan: `2026-03-09-m9-serde-implementation-plan.md`

## Milestone 10 — yg CLI + Query Engine

**Status:** Complete

jaq 3 supplies the jq grammar and engine. `YamlVal` keeps YAML data and
presentation metadata available to core jq filters and YAML-native extensions.

- Spec: `docs/spec/yg-query-language-spec-draft-01.md`.
- Plan: `record/superpowers/plans/2026-06-10-m10-yg-cli-implementation-plan.md`.
- Scope: Phases 1-2 from spec Section 11.

Key deliverables:

- `yg <filter> [files...]` queries and transforms YAML.
- jaq provides the jq-familiar expression language.
- YAML 1.2 Core Schema is the default; `--schema` selects another resolver.
- `type` returns YAML names such as `"mapping"` and `"sequence"`.
- `keys` returns insertion order; `keys_sorted` opts in to sorting.
- `-j` emits JSON after alias materialization.
- `-r` emits raw string output.
- Exit codes follow jq conventions.

The jq 1.8 manual harness passes 230 examples. It tracks 13 expected failures
and 8 scope exclusions by exact case. Stale entries and unexpected passes fail
the suite.

Draft-01 deviations are recorded in
`docs/spec/yg-spec-draft-01-implementation-notes.md`:

- `explode_aliases` avoids jq's `explode` name collision.
- Strict `===` waits for grammar support instead of forking jaq.
- Numbers use an `i64`/`f64` backend.
- M11 owns external resolution.
- Librebar's `--color` interface replaces the draft's `-C`/`-M` pair.
- Pre-CST `-i` edits lose comments and exact formatting.

## Milestone 11 — Built-in Resolvers

**Status:** Complete (PR #113)

`IncludeResolver` and `RefResolver` implementations. Filesystem sandboxing,
URL policy, cycle detection, byte budgets, source navigation, and redacted
resolution reports wired through `LoaderConfig`.

- Design: `../specs/2026-08-08-m11-external-resource-resolution-design.md`
- Plan: `2026-08-09-m11-external-resource-resolution.md`

## Milestone 12 — Schema Validation + LSP

**Status:** In progress — implementation and documentation complete; final
integration, release, and audit gates remain

- Plan: `2026-08-09-m12-schema-validation-lsp.md`

Native YAML adaptation over the `jsonschema` Draft 2020-12 engine, operating
directly on Yamalgam's `Value`, plus a YAML language server that uses the same
schema session for completions, hover, diagnostics, and navigation.

Schema validation:

- `SchemaValidator` in `yamalgam-schema` validates `Value` against a schema `Value`
- JSON Schema Draft 2020-12 through `jsonschema`'s custom-value API
- `$ref` within schemas for schema composition
- Structured diagnostics: path to failing node, rule violated, expected vs actual
- `validate(schema)` filter for yg (spec Section 10)
- `yg validate` / `yg lint` subcommands
- `# yg-schema-validate:` pragma support
- SchemaStore integration for auto-detection (Kubernetes, GitHub Actions, etc.)
- Scalar YAML mapping keys use collision-checked canonical property names while retaining original type and span
- Adapted MIT-licensed boundary cases from `yaml-schema`; no Saphyr or duplicate keyword engine

LSP (powered by CST + schema validator):

- Schema-aware completions (property names, enum values, type hints)
- Hover info from schema descriptions
- Real-time validation diagnostics
- `!include` navigation (click-through to included file)
- `$ref` resolution and go-to-definition
- Error-recovering parser mode for in-progress typing

Improvements over existing YAML language servers:

- **Real parser, not heuristics.** Existing servers rely on JS YAML parsers that give up on partial/invalid documents (which is most of the time while typing). CST with error-recovery nodes produces a usable tree even for broken input.
- **Comment-preserving edits.** Quickfixes and formatting in existing servers serialize/deserialize through a lossy path that strips comments. CST round-trip keeps unmodified nodes intact.
- **`!include` and `$ref` awareness.** Existing servers flag custom tags as errors. yamalgam's resolver trait lets the LSP resolve includes, validate included content, and provide click-through navigation.
- **Native schema adaptation.** Existing servers convert whole YAML documents to JSON before validation. Yamalgam adapts `Value` directly, preserves anchors/merges after composition, projects scalar keys without overwrites, and retains original source locations.
- **Multi-document support.** Existing servers treat each file as one document. yamalgam handles `---`-separated streams natively.

## Milestone 13 — serde Serializer + Pretty Emitter

**Status:** Not started

YAML serialization with configurable style preferences. Round-trip editing via CST (parse → mutate → emit with comments intact).

---

## Cross-Cutting Concerns

| Concern | Status | Notes |
|---------|--------|-------|
| YAML Test Suite compliance | 350/351 events | 1 EXPECTED remaining (DK95) |
| Fuzzing | 6 targets, CI integrated | OSS-Fuzz submission after public launch |
| Benchmarks | 7 peers | yamalgam fastest on small/medium |
| WASM build | Not started | `wasm32-wasip2` target, wit-bindgen |
| facet integration | Not started | Season 2 — after serde |

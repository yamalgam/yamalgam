# yamalgam Roadmap

Overall project plan, organized by milestone. Each milestone has a design doc and implementation plan in this directory. ADRs in `docs/decisions/`.

## Pipeline Architecture

```
YAML bytes → Scanner (tokens) → Parser (events) → Resolver middleware
                                                        │
                    ┌───────────┬───────────────┬───────┴───────┐
                    │           │               │               │
              Streaming     Value (DOM)       CST          SAX/Callbacks
              serde Deser   (lossy)        (lossless)      (zero alloc)
```

See `docs/diagrams/event-stream-architecture.svg`.

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
Round-trip fidelity `cst.to_text() == input`: 339/351 YAML Test Suite
(12 flow close-token cases allowlisted). Unlocks round-trip editing,
`yg -i`, linter, pretty emitter.

- Design: `2026-03-09-cst-design.md`, `2026-03-09-full-fidelity-event-stream-design.md`

## Milestone 9 — Streaming serde Deserializer
**Status:** Core complete (PRs #82-#83) — integration tests remaining

`yamalgam::from_str::<T>()` drop-in for serde_yaml, plus
`Deserializer::documents::<T>()` for multi-doc streaming. Erased-serde
pattern internally — parser event-walking never monomorphizes. Anchor
buffering via event replay. Config-aware (`from_str_with_config`).
Consumer crates extracted along the way: `yamalgam-compose`, `yamalgam-cst`.

Remaining: real-world fixture corpus, serde_yaml behavioral parity tests,
serde round-trip in the compliance harness (needs `Deserialize for Value`).

- Design: `2026-03-09-streaming-serde-deserializer-design.md`
- Plan: `2026-03-09-m9-serde-implementation-plan.md`

## Milestone 10 — yg CLI + Query Engine
**Status:** Not started

Integrate jaq-syn/jaq-core as expression engine. Implement `YamlVal` with jaq's `ValT` trait. Build `yg` CLI with core jq-compatible filters, YAML-native extensions (tag, anchor, style, kind, raw), and schema flags.

- Spec: `docs/spec/yg-query-language-spec-draft-01.md`
- Phases 1-2 from spec Section 11

Key deliverables:
- `yg <filter> [files...]` — YAML query and transformation
- jq-familiar expression language via jaq (not reimplemented)
- YAML 1.2 Core Schema by default, configurable (`--schema`) via TagResolver trait
- `type` returns YAML names (`"mapping"`, `"sequence"`)
- `keys` returns insertion order (not sorted — use `keys_sorted`)
- `-j` JSON output with implicit `explode`
- `-r` raw string output
- Exit codes matching jq convention

## Milestone 11 — Built-in Resolvers
**Status:** Not started

`IncludeResolver` and `RefResolver` implementations. Filesystem sandboxing, URL policy, cycle detection, byte budgets. Wires into LoaderConfig's ResolutionPolicy.

## Milestone 12 — Schema Validation + LSP
**Status:** Not started

Native schema validator operating directly on yamalgam's Value type, and a YAML Language Server that uses it for completions, hover, and diagnostics. These are the same thing — the LSP is the schema validator with a cursor position.

Schema validation:
- `SchemaValidator` in yamalgam-core — validates `Value` against a schema `Value`
- JSON Schema Draft 2020-12 keyword support (the subset that matters for YAML)
- `$ref` within schemas for schema composition
- Structured diagnostics: path to failing node, rule violated, expected vs actual
- `validate(schema)` filter for yg (spec Section 10)
- `yg validate` / `yg lint` subcommands
- `# yg-schema-validate:` pragma support
- SchemaStore integration for auto-detection (Kubernetes, GitHub Actions, etc.)
- Not using `yaml-schema` crate (saphyr dep) or `jsonschema` crate (lossy JSON conversion)

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
- **Native schema validation.** Existing servers convert YAML to JSON for validation, losing YAML-specific features (tags, anchors, non-string keys, merge keys). Native validation on `Value` skips the conversion.
- **Multi-document support.** Existing servers treat each file as one document. yamalgam handles `---`-separated streams natively.

## Milestone 13 — serde Serializer + Pretty Emitter
**Status:** Not started

YAML serialization with configurable style preferences. Round-trip editing via CST (parse → mutate → emit with comments intact).

---

## Cross-Cutting Concerns

| Concern | Status | Notes |
|---------|--------|-------|
| YAML Test Suite compliance | 349/351 events | 2 EXPECTED remaining (CFD4, DK95) |
| Fuzzing | 6 targets, CI integrated | OSS-Fuzz submission after public launch |
| Benchmarks | 8 peers (yamlstar feature-gated) | yamalgam fastest on small/medium |
| WASM build | Not started | `wasm32-wasip2` target, wit-bindgen |
| facet integration | Not started | Season 2 — after serde |

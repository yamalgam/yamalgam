# Handoff: Resolver Trait, Value DOM, and Tracey Spec Traceability

**Date:** 2026-03-08
**Branch:** `main` (two PRs merged this session)
**State:** Green — 1069 tests pass, compliance unchanged (349 EVENT_PASS, 0 UNEXPECTED).

## Where things stand

Milestone 6 (Resolver Trait + Value DOM) is complete and merged. The event-stream architecture is established: parser events are the core, with four peer consumers (streaming serde, Value, CST, SAX). The resolver trait provides composable middleware for `!include`/`$ref` and custom tag processing.

Tracey spec traceability is set up and merged. The full YAML 1.2.2 specification is marked up with 459 requirement markers (prefix `y`). YAML 1.1 spec is downloaded and converted to markdown but not yet marked up. No code annotations yet — that's the next step to light up coverage.

## Decisions made

- **ADR-0007: Resolver trait** — composable event-stream middleware between parser and all consumers. Resolve vs preserve behavior is implicit in consumer type, not a flag.
- **Four peer consumers** of the event stream (not CST-in-front-of-everything). See `docs/diagrams/event-stream-architecture.svg`.
- **Tracey prefix is `y`** — single prefix across all YAML spec versions. Spec evolution tracked via tracey versioning (`y[char.c-printable+2]`), not separate prefixes per spec version.
- **Schema Resolver is a trait** — extensible for custom schemas without forking.
- **Schema Validation + LSP merged** into one milestone — the LSP is the schema validator with a cursor position.
- **ADR-0003 deprecated** — comparison MCP server was never the path; comparison harness uses subprocess/cache.

## What's next

1. **Mark up YAML 1.1 spec** with `y[...]` markers at version 1. Then bump versions in 1.2.2 markup where requirements changed. This validates the tracey versioning workflow before YAML 1.3 arrives.
2. **Annotate scanner/parser code** with `y[impl ...]` markers to light up tracey coverage. Start with Chapter 5 (character productions) → scanner, since that's the most direct mapping.
3. **Milestone 7: Schema Resolver trait** — extract `resolve_plain_scalar()` into a `SchemaResolver` trait with `CoreSchema`, `FailsafeSchema`, `JsonSchema`, `Yaml11Schema` implementations.
4. **Milestone 8: yg CLI** — integrate jaq-syn/jaq-core, implement `YamlVal` with `ValT` trait. Spec: `docs/spec/yg-query-language-spec-draft-01.md` (168 requirements).

## Landmines

- **`docs/test/1.1/` still exists** alongside `docs/spec/yaml-1.1/`. The `docs/test/` version is the source; `docs/spec/` is the copy. Clean up or gitignore the test directory.
- **`vendor/yaml-spec-1.1/clean-spec.pl`** is an abandoned cleanup script from an agent that couldn't run it. Delete it — Clay's manual conversion was better.
- **`vendor/yaml-spec-1.1/spec-1.1.md`** is the rough pandoc conversion (HTML artifacts). The clean version is `docs/spec/yaml-1.1/spec-1.1.md`. The vendor copy can be deleted.
- **yamlstar benchmark requires `libyamlstar.dylib`** which needs GraalVM to build. GraalVM dropped Intel macOS — only works on ARM Mac or Linux. Feature-gated behind `--features yamlstar`.
- **Tracey daemon auto-starts** on `tracey query` — first run takes a few seconds. Subsequent queries are fast.
- **The yg query spec** (`docs/spec/yg-query-language-spec-draft-01.md`) is tracked by tracey but not yet committed to the repo. It's in Clay's working tree.

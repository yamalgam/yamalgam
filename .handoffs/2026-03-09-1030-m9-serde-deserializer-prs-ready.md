# Handoff: M9 Streaming serde Deserializer — PRs Ready for Review

**Date:** 2026-03-09
**Branch:** `feat/m9-serde-deserializer` (stacked on `feat/m9-extract-consumer-crates`)
**State:** Green — 1271 tests, `just check` passes, no uncommitted work.

## Where things stand

M9 core implementation is complete across two PRs. PR #82 extracts Composer and CST into standalone crates. PR #83 adds `yamalgam-serde` with a streaming Deserializer, erased-serde integration, multi-document iterator, and config-aware APIs. Both pass `just check`. Integration tests (Tasks 14-16) remain.

## Decisions made

- **Each event consumer gets its own crate.** Parser → yamalgam-compose, yamalgam-cst, yamalgam-serde. Parser crate is pure event production.
- **erased-serde 0.4** for zero-monomorphization. `from_str` routes through `<dyn erased_serde::Deserializer>::erase()`.
- **serde_json-style multi-doc iterator.** `Documents<T>` yields `Iterator<Item=Result<T>>`, not serde_yaml's `Iterator<Item=Deserializer>`.
- **Event buffering for anchors** (not Value fallback). Two-layer event architecture: `next_raw_event()` (mechanical) and `next_event()` (semantic anchor/alias processing). Replay buffer via `VecDeque`.
- **`from_str` errors on multi-doc** (drop-in compatible with serde_yaml). `documents()` is the streaming entry point.
- Design doc: `docs/plans/2026-03-09-streaming-serde-deserializer-design.md`
- Implementation plan: `docs/plans/2026-03-09-m9-serde-implementation-plan.md`

## What's next

### Tasks 14-16: Integration tests (separate PR, after review)

1. **Crawl yamllint test repos** for real-world YAML fixtures. Sources: adrienverge/yamllint, google/yamlfmt, mikefarah/yq, prettier YAML plugin. Store in `crates/yamalgam-serde/tests/fixtures/`.
2. **serde_yaml compatibility tests** — port a subset to verify behavioral parity.
3. **Compliance harness extension** — add serde round-trip to YAML Test Suite runner (`from_str::<Value>()` vs Composer output). Requires `impl Deserialize for Value` in yamalgam-core.
4. **Update `docs/plans/README.md`** — mark M9 complete with test count.

### After M9

- M10: yg CLI + Query Engine
- Fix 12 CST flow close token allowlist cases
- Performance tuning (scanner allocation, bulk whitespace skip)

## Landmines

- **PR #83 is stacked on #82.** Merge #82 first (crate extraction), then #83 (serde). The serde branch includes #82's commits.
- **`parse_integer`/`parse_float` were fixed** to use `self.tag_resolver` instead of the `resolve_plain_scalar` free function. This means YAML 1.1 octal (`017`), binary (`0b1010`), and extended booleans (`yes`/`no`) now work with `from_str_with_config`. The original scalar tests used the default Yaml12 resolver and still pass.
- **`Deserialize for Value` doesn't exist yet.** Task 15 (compliance harness round-trip) needs it. Implement in yamalgam-core since it already depends on serde.
- **`from_str_with_limits` exists alongside `from_str_with_config`.** The limits variant takes bare `ResourceLimits`, the config variant takes full `LoaderConfig`. Consider deprecating the limits variant after the config API stabilizes.
- **68 serde tests use synthetic YAML.** Real-world edge cases (k8s manifests, docker-compose, GitHub Actions) aren't tested yet — that's Tasks 14-16.
- **`just check` is the gate.** Run it before every PR.

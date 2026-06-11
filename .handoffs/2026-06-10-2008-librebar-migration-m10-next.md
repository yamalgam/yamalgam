# Handoff: librebar Migration Complete — M10 (yg CLI) Next

**Date:** 2026-06-10
**Branch:** `librebar-migration` (staged, commit.txt written, awaiting merge)
**State:** Green — 1689 tests, `just check` clean.

## Where things stand

Big day: three PRs merged (#108 M9 integration tests, #109 error stash,
#110 flow-pair parser fix), and the librebar migration is staged as the
fourth. M1-M9 are complete. M10 (yg CLI + jaq query engine) is next and
should start in a fresh session.

- **#108** — M9 done: `Deserialize for Value`, serde round-trip compliance
  harness (Composer vs serde agree 351/351, empty allowlist), 408-fixture
  corpus, serde_yaml parity tests. Fixed along the way: nested-anchor
  registration during buffering, buffer-time alias splicing (self-ref and
  redefinition-cycle safe), peek raw-Alias leak, directive skipping.
- **#109** — structured errors survive erased-serde: clone stashed at
  origin, restored by rendered-text match in `from_str*`/`Documents::next`.
  Callers can match `Error` variants and read spans from `from_str`.
- **#110** — empty-key flow pairs (`[: v]`, `[ : ]`) parse; synthetic
  single-pair MappingStart/End carry zero-width spans, which fixed the CST
  close-token duplication family. Event compliance 350/351 (DK95 last
  expected), CST round-trip 343/351 (8 flow-MAPPING close cases remain).
- **Staged (this branch)** — claylo-rs template scaffolding replaced with
  librebar 0.1 (crates.io). See below.

## The librebar migration

- **yamalgam-core went from 11 runtime deps to 1 (serde).** Deleted
  `config.rs` (figment), `observability.rs`, `error.rs` (ConfigError),
  and the template config benchmarks. Core is now a lean library crate.
- **CLI compose pattern:** librebar primitives, not the App builder —
  `config::ConfigLoader` + `logging::init`/`env_filter` + `diagnostics::
  DoctorRunner`. Reason: the builder hardcodes "info" as the filter
  baseline; manual composition keeps config-driven `log_level` and
  `log_dir` (template parity). librebar's own README blesses this.
- **doctor** now runs librebar `DoctorCheck`s (config-discovered,
  log-dir-resolvable) plus the directory/env report; `--json` preserved.
- **Template bench machinery removed:** xtask gen-benchmarks, core
  divan/gungraun harnesses + benchmarks.toml, the two CI jobs in
  benchmarks.yml, `just bench-divan`. `cargo xtask bench` now runs
  scanner/parser self-benches + comparative + hyperfine CLI.
- **Deleted `.repo.yml`** (Copier answers). README rewritten from template
  boilerplate to honest pre-release content. Workspace description/
  keywords/categories fixed (`["", ""]` would have failed publish).

## Decisions made

- **YAML config files now parse** (librebar's loader, serde-saphyr
  backed). This intentionally supersedes #103's "no YAML config until
  self-hosted" — serde-saphyr is pure Rust and already in the workspace
  as a bench peer. The three rejection tests flipped to acceptance tests.
  **Follow-up intent:** once yamalgam publishes, librebar should grow a
  pluggable YAML backend so yamalgam parses its own config (and everyone
  else's). Clay owns both projects; timing works post-publish.
- **librebar from crates.io** (`version = "0.1"`), not a path dep. Local
  checkout at `~/source/claylo/librebar` matched the published API
  exactly (workspace compiled first try).
- Test count moved 1712 → 1689: template config/observability tests left
  with their modules; CLI tests were rewritten, not dropped (46/46).

## What's next: M10 — yg CLI + query engine (FRESH SESSION)

Spec: `docs/spec/yg-query-language-spec-draft-01.md` (phases 1-2 of
section 11 are M10 scope). Key shape:

- jaq-syn/jaq-core as the expression engine; implement jaq's `ValT` trait
  for a YamlVal wrapper — do NOT reimplement jq
- `yg <filter> [files...]`; YAML-native extensions (tag, anchor, style,
  kind, raw); `-j` JSON output with implicit explode; `-r` raw strings;
  jq exit-code conventions
- YAML 1.2 core schema default, `--schema` via TagResolver
- `keys` insertion-ordered (use `keys_sorted` for sorted)
- Write the implementation plan first (superpowers:writing-plans), like
  M9's — the M9 plan's task granularity worked well
- The CLI crate is now a clean librebar foundation to build on; `yg`
  rename/restructure happens here

Queued behind M10: crates.io publish (publish = false flags come off,
release workflow dry-run), librebar-yamalgam config backend, 8 remaining
CST flow-mapping close-token cases, `? key`-without-value parser gap
(root/example.yml fixture), rust-yaml bench anomaly.

## Landmines

- **librebar's builder ≠ this CLI's wiring.** If someone "simplifies"
  main.rs to the `librebar::init().config().logging().start()` chain,
  config-driven log_level/log_dir silently stop working. The manual
  composition is deliberate — see comment in main.rs.
- **CommonArgs has no `--config`** — that's the app's own flag, layered
  via `ConfigLoader::with_file` on top of discovery (template precedence
  preserved: explicit file wins).
- **benchmarks.yml changed** — verify the workflow is green after merge
  (divan/gungraun jobs removed; comparative + self-benchmarks remain).
- **Network was down at session end** — `just check` ran clean offline
  (cargo cache + cached advisory DB), but `git pm` and CI need the
  network back. Cargo.lock includes the new librebar dependency tree;
  if `cargo deny` complains about anything librebar pulled once the
  advisory DB refreshes, that's new-info, not a regression.
- **MEMORY.md updated** for the librebar state; milestone-history has
  M1-M9 detail. The scanner-parser-gotchas memory is still the required
  read before touching scanner/parser internals (M10 mostly won't).

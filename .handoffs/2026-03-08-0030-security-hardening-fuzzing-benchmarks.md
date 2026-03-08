# Handoff: Security Hardening, Fuzzing Infrastructure, and Comparative Benchmarks

**Date:** 2026-03-08
**Branch:** `main` (one uncommitted PR on `feat/bench-self-and-ci`)
**State:** Green — 1013 tests pass, compliance unchanged (349 EVENT_PASS, 0 UNEXPECTED).

## Where things stand

Security hardening, fuzzing, and comparative benchmarks are implemented across 10 PRs (#58-#67). `LoaderConfig` enforces resource limits (depth, scalar size, key size, input size) at every pipeline layer. Six cargo-fuzz targets with an Arbitrary-based YAML generator are ready for continuous fuzzing. Comparative benchmarks show yamalgam is fastest or tied for fastest against 7 Rust YAML peers on small and medium inputs.

## Decisions made

- **ADR-0006: `LoaderConfig` taxonomy** — top-level config with `ResourceLimits` + `ResolutionPolicy` sub-structs, named after YAML spec's "load" operation. Four presets: moderate (default), strict, trusted, unchecked.
- **Scanner `new()` stays `const fn`** — `ResourceLimits::none()` is const-constructible. `with_config()` is the opt-in path for limits.
- **Fuzz crate excluded from workspace** — cargo-fuzz requires standalone `fuzz/Cargo.toml`. Added `exclude = ["fuzz"]` to root.
- **Depth = combined** — `indent_stack.len() + flow_level` at all enforcement points (flow start, roll_indent, fetch_value inline path).
- **`push_state` made fallible** — returns `Result<(), ParseError>` with new `LimitExceeded` variant. 13 call sites updated.
- **Comparative benchmarks in separate crate** — `yamalgam-bench` pulls 7 peer deps that don't belong in library crates.

## What's next

1. **Plan the pull parser → CST architecture** — the next milestone. Clay flagged this as the session opener. CST is the differentiator no other Rust YAML library has. Key design questions: arena allocation, trivia (comments/whitespace) representation, error nodes for LSP, incremental reparsing. See `ref/architecture-discussion.md` for the full context (CST discussion starts around line 389).
2. **Investigate `rust-yaml` large-input anomaly** — it benchmarked faster than yamalgam on 10K records (40ms vs 70ms). May be skipping work or using a different strategy. Worth understanding before publishing comparisons.
3. **`bench.yamalgam.com` / `fuzz.yamalgam.com`** — Tufte-inspired reporting sites. Data pipeline exists (timestamped reports in `bench-reports/`). Visualization layer is next.
4. **OSS-Fuzz submission** — requires Dockerfile + project.yaml. Do after the project goes public.

## Landmines

- **`fuzz/Cargo.lock` must be committed** — it's a standalone package, not a workspace member. Without the lockfile, CI fuzz builds are non-reproducible.
- **`fetch_value` inline indent-rolling bypasses `roll_indent()`** — any limit check added to `roll_indent` must also be added to `fetch_value`'s inline path (~line 1015). This was caught during review and fixed, but future scanner changes must respect this invariant.
- **Bench reports are gitignored** — `bench-reports/*.txt` files are local-only. CI uploads them as artifacts. Don't expect to find them in the repo.
- **`serde-saphyr` and `yaml_serde` lack native Value types** — comparative benchmarks deserialize into `serde_json::Value`, which adds serde overhead. Not a fair apples-to-apples comparison for those two; the others use their native DOM APIs.
- **Fuzz CI skips `fuzz_differential`** — requires the C harness binary which isn't built in CI. Enable separately when C harness is added to CI build matrix.

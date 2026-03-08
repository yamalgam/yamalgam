# Fuzzing, Benchmarking, and LoaderConfig Design

**Date:** 2026-03-07
**Status:** Proposed
**Relates to:** [ADR-0006](../decisions/0006-loaderconfig-for-resource-limits-and-security-policy.md)

## Problem

yamalgam's scanner and parser pass 917 tests and achieve 99.4% event compliance. But three gaps remain before the project can credibly claim production readiness:

1. **No fuzzing.** Hand-written tests catch known edge cases. Fuzzing catches the unknown ones — panics on malformed UTF-8, infinite loops on pathological nesting, memory exhaustion from adversarial scalars. Every serious Rust parser project fuzzes continuously.

2. **No comparative benchmarks.** Two config-loading benchmarks exist in yamalgam-core. Scanner and parser throughput are unmeasured. Performance relative to peers (yaml-serde, libyaml-safer, yaml-rust2, saphyr-parser, serde-saphyr, rust-yaml) is unknown.

3. **No resource limits.** ADR-0006 defines `LoaderConfig` with `ResourceLimits` and `ResolutionPolicy`. None of it is implemented. A malicious input can exhaust memory today.

This document designs all three.

---

## 1. Fuzzing Architecture

### Location

```
fuzz/
├── Cargo.toml                    # workspace member, depends on scanner + parser + compare
├── fuzz_targets/
│   ├── fuzz_scanner_bytes.rs     # raw bytes → Scanner::new().collect()
│   ├── fuzz_parser_bytes.rs      # raw bytes → Parser::new().collect()
│   ├── fuzz_scanner_structured.rs # Arbitrary-generated YAML → scan
│   ├── fuzz_parser_structured.rs  # Arbitrary-generated YAML → parse
│   ├── fuzz_limits.rs            # adversarial inputs vs LoaderConfig::strict()
│   └── fuzz_differential.rs      # yamalgam vs libfyaml, disagreement = bug
├── corpus/                       # committed regression inputs
│   └── seed/                     # seeded from YAML Test Suite
└── arbitrary_yaml.rs             # shared structured generator
```

`fuzz/` lives at the workspace root. `Cargo.toml` declares it as a workspace member with `cargo-fuzz` configuration.

### Fuzz targets

**Byte-level targets** feed arbitrary bytes to the scanner and parser. These catch panics, infinite loops, and assertion failures on malformed input. Most random bytes fail early in the scanner, but coverage-guided mutation evolves toward deeper code paths over time.

```rust
// fuzz_scanner_bytes.rs
fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = Scanner::new(input).collect::<Result<Vec<_>, _>>();
    }
});

// fuzz_parser_bytes.rs
fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = Parser::new(input).collect::<Result<Vec<_>, _>>();
    }
});
```

**Structured targets** use `arbitrary::Arbitrary` to generate syntactically plausible YAML. This reaches deep parser logic that random bytes cannot.

```rust
// arbitrary_yaml.rs — shared generator
#[derive(Arbitrary)]
struct YamlDoc {
    documents: Vec<YamlNode>,
    use_directives: bool,
}

#[derive(Arbitrary)]
enum YamlNode {
    Scalar { value: String, style: ScalarStyle },
    Mapping { entries: Vec<(YamlNode, YamlNode)> },
    Sequence { items: Vec<YamlNode>, flow: bool },
    Alias { anchor_ref: u8 },
    Anchored { anchor_id: u8, node: Box<YamlNode> },
}

impl YamlDoc {
    fn render(&self) -> String { /* emit valid-ish YAML text */ }
}
```

The generator produces random nesting depths, mixed flow/block styles, anchor/alias patterns, and varied scalar content. Each `YamlNode` variant maps to a YAML construct. `anchor_ref` and `anchor_id` use `u8` to keep the anchor namespace small enough that aliases frequently resolve.

**Limits target** creates adversarial inputs and verifies that `LoaderConfig::strict()` rejects them. Crafted patterns: deeply nested `[[[[...]]]]`, megabyte-sized scalars, thousands of anchors, recursive alias references.

```rust
// fuzz_limits.rs
fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let config = LoaderConfig::strict();
        // Must not panic. Must not allocate unboundedly.
        // Errors are expected and acceptable.
        let _ = Scanner::with_config(input, &config)
            .collect::<Result<Vec<_>, _>>();
        let _ = Parser::with_config(input, &config)
            .collect::<Result<Vec<_>, _>>();
    }
});
```

**Differential target** feeds the same input to yamalgam and libfyaml (via the existing `yamalgam-compare` infrastructure). Any disagreement — one accepts and the other rejects, or both accept but produce different tokens — is a bug in one or the other.

```rust
// fuzz_differential.rs
fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let rust_result = Scanner::new(input).collect::<Result<Vec<_>, _>>();
        let c_result = c_baseline::scan_with_c_harness(input);
        match (&rust_result, &c_result) {
            (Ok(rust_tokens), Ok(c_tokens)) => {
                // Compare token streams (kind + value, ignore spans)
                assert_tokens_equivalent(rust_tokens, c_tokens);
            }
            (Ok(_), Err(_)) => {
                // yamalgam accepts, C rejects — we may be too permissive
                // Log but don't panic (some cases are known, e.g. fail:true inputs)
            }
            (Err(_), Ok(_)) => {
                // yamalgam rejects, C accepts — we may be too strict
                // Log but don't panic
            }
            (Err(_), Err(_)) => {
                // Both reject — acceptable
            }
        }
    }
});
```

### Multi-engine strategy

| Engine | Purpose | When |
|--------|---------|------|
| **cargo-fuzz** (libFuzzer) | Coverage-guided mutation. Finds panics and crashes fast. Primary engine. | PR: 60s/target. Nightly: 1hr/target. |
| **cargo-afl** (AFL++) | Different mutation strategy than libFuzzer. Finds different bugs — better at finding length-related issues and format-specific patterns. | Nightly: 1hr/target. |
| **Miri** | Detects undefined behavior. Even with `#![deny(unsafe_code)]`, dependencies may contain unsafe. Miri catches UB that manifests through safe APIs. | Weekly: run corpus through Miri. |

### Corpus management

Seed the initial corpus from the YAML Test Suite's 351 inputs. Extract the YAML content from each test case file and write it to `fuzz/corpus/seed/`. The fuzzer evolves from these seeds.

When a fuzzer finds a crash or disagreement, minimize the input (`cargo fuzz tmin`) and commit it to `fuzz/corpus/` as a permanent regression test. The CI fuzz job loads these on every run.

Periodically minimize the full corpus (`cargo fuzz cmin`) to remove redundant inputs that don't improve coverage.

### CI integration

**PR checks (GitHub Actions):**
```yaml
fuzz:
  runs-on: ubuntu-latest
  strategy:
    matrix:
      target: [fuzz_scanner_bytes, fuzz_parser_bytes, fuzz_scanner_structured,
               fuzz_parser_structured, fuzz_limits, fuzz_differential]
  steps:
    - cargo +nightly fuzz run ${{ matrix.target }} -- -max_total_time=60
```

Six targets at 60 seconds each, parallelized across matrix jobs. Total wall time: ~90 seconds (with overhead). Catches regressions from new code.

**Nightly scheduled:**
```yaml
on:
  schedule:
    - cron: '0 3 * * *'  # 3 AM UTC daily
```

Each target runs for 1 hour. cargo-fuzz and cargo-afl alternate nights. Results and any crashes are uploaded as artifacts.

**OSS-Fuzz submission** once the project is public. Google runs fuzzing continuously against accepted projects. Acceptance requires: fuzz targets, a `Dockerfile`, and a `project.yaml`. The differential target is particularly valuable here — continuous comparison against libfyaml.

### justfile recipes

```just
fuzz target='fuzz_scanner_bytes' duration='60':
    cargo +nightly fuzz run {{target}} -- -max_total_time={{duration}}

fuzz-all duration='60':
    #!/usr/bin/env bash
    for target in fuzz_scanner_bytes fuzz_parser_bytes fuzz_scanner_structured \
                  fuzz_parser_structured fuzz_limits fuzz_differential; do
        cargo +nightly fuzz run "$target" -- -max_total_time={{duration}}
    done

fuzz-long:
    just fuzz-all 3600

fuzz-afl target='fuzz_scanner_bytes':
    cargo afl build --release && cargo afl fuzz -i fuzz/corpus/seed -o fuzz/afl-out target/release/{{target}}

fuzz-corpus-min:
    cargo +nightly fuzz cmin fuzz_scanner_bytes
    cargo +nightly fuzz cmin fuzz_parser_bytes

fuzz-miri:
    MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly miri test -p yamalgam-scanner
    MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly miri test -p yamalgam-parser
```

---

## 2. Comparative Benchmarking

### Location

```
crates/yamalgam-bench/
├── Cargo.toml
├── benches/
│   ├── benchmarks.kdl          # single source of truth (extends existing pattern)
│   ├── divan_benchmarks.rs     # auto-generated
│   └── gungraun_benchmarks.rs  # auto-generated
├── src/
│   ├── lib.rs
│   ├── inputs.rs               # deterministic input generators
│   └── peers.rs                # thin wrappers around each peer's parse API
└── inputs/
    └── kubernetes-deployment.yaml  # real-world reference input
```

Separate crate because comparative benchmarks pull in 6 external parser crates as dependencies. These have no business in the library crates.

### Peer set

| Crate | API to benchmark | What it represents |
|-------|-----------------|-------------------|
| `yaml-serde` | `yaml_serde::from_str::<Value>()` | The incumbent serde path (unsafe-libyaml underneath) |
| `libyaml-safer` | `Document::load_from_str()` | Safe Rust port of libyaml, event-based |
| `yaml-rust2` | `YamlLoader::load_from_str()` | Pure Rust DOM, 28.5M downloads |
| `saphyr-parser` | `Parser::new_from_str()` + sink | What facet-yaml uses, the "next gen" pure Rust parser |
| `serde-saphyr` | `serde_saphyr::from_str::<Value>()` | Serde layer on saphyr fork |
| `rust-yaml` | `Yaml::load_str()` | Regex-based newcomer, sanity check |

Each peer gets a thin wrapper in `peers.rs` that normalizes the API to `fn parse(input: &str) -> Result<(), Error>`. We benchmark parse-to-completion, not deserialization into specific types (that comes when we have serde support).

### Input generators

Port the best ideas from peers, adapted to our needs:

**From yaml-rust2's `gen_large_yaml`:** Seeded PRNG (seed=42) producing 4 input shapes:
- `gen_records(n)` — block mappings with varied field types (strings, integers, URLs, hashes)
- `gen_nested(depth)` — deeply nested block mappings
- `gen_small_objects(n)` — many small 2-3 field mappings
- `gen_string_array(n)` — plain string sequence items

**From serde-saphyr-benchmark:** Anchor/alias stress generator at configurable sizes.

**Static inputs:** A real-world Kubernetes Deployment YAML (~2KB) as the "typical config file" benchmark.

### Benchmark matrix

| Benchmark | Input | Size | What it measures |
|-----------|-------|------|-----------------|
| `scan_kubernetes` | Kubernetes Deployment | ~2KB | Scanner latency on typical config |
| `scan_records` | 10K records | ~5MB | Scanner throughput on block-heavy input |
| `scan_nested_256` | 256-deep nesting | ~1KB | Scanner on pathological depth |
| `scan_large_scalar` | 1MB block scalar | 1MB | Large scalar handling |
| `parse_kubernetes` | Kubernetes Deployment | ~2KB | Parser latency on typical config |
| `parse_records` | 10K records | ~5MB | Parser throughput |
| `parse_anchors` | 1000 anchors, 5000 aliases | ~500KB | Anchor/alias overhead |
| `comparative_small` | Kubernetes Deployment | ~2KB | All peers, same input |
| `comparative_medium` | 1K records | ~500KB | All peers, same input |
| `comparative_large` | 10K records | ~5MB | All peers, same input |

Self-benchmarks (yamalgam only) run in the scanner and parser crates' own benchmark files. Comparative benchmarks (all peers) run in yamalgam-bench.

### Metrics

- **Wall-clock time** (divan) — primary metric, runs on all platforms
- **Instruction count** (gungraun + Valgrind) — deterministic, Linux/Intel only
- **Throughput** — bytes/second, derived from wall-clock time and input size

Memory high-water mark is deferred. jemalloc instrumentation or `/proc/self/status` polling adds complexity. Worth doing later, not now.

### Regression tracking

CI stores benchmark results as JSON artifacts. A baseline file (`bench-reports/baseline.json`) is committed to the repo. The `just bench-compare` recipe diffs current results against the baseline and flags regressions exceeding 5%.

### Public reporting

Benchmark results publish to `bench.yamalgam.com`. The site shows:
- Comparative throughput charts (yamalgam vs peers)
- Historical trend lines (regression detection across commits)
- Input descriptions and methodology

Fuzzing status publishes to `fuzz.yamalgam.com`:
- Corpus size and coverage stats
- Time since last crash found
- Differential fuzzing disagreement log

Site generation is a later milestone. The data pipeline (JSON artifacts → site) comes after the benchmarks themselves produce stable output.

### justfile recipes

```just
bench-comparative:
    cargo bench -p yamalgam-bench --bench divan_benchmarks

bench-scanner:
    cargo bench -p yamalgam-scanner --bench divan_benchmarks

bench-parser:
    cargo bench -p yamalgam-parser --bench divan_benchmarks
```

---

## 3. LoaderConfig Implementation

### Location

New module `crates/yamalgam-core/src/loader.rs`, re-exported from `lib.rs`.

yamalgam-core is the right home. It already holds shared pipeline types (`Span`, `Mark`, `Diagnostic`) used by both scanner and parser. `LoaderConfig` is the same kind of cross-cutting type. No restructuring needed.

### Types (per ADR-0006)

```rust
// crates/yamalgam-core/src/loader.rs

use std::path::PathBuf;
use std::time::Duration;

/// Controls resource consumption and security boundaries
/// for the yamalgam loading pipeline.
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    pub limits: ResourceLimits,
    pub resolution: ResolutionPolicy,
}

#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum input size in bytes (caps `from_reader()` buffer growth).
    pub max_input_bytes: Option<usize>,
    /// Maximum size of a single scalar value in bytes.
    pub max_scalar_bytes: Option<usize>,
    /// Maximum size of a mapping key in bytes.
    pub max_key_bytes: Option<usize>,
    /// Maximum nesting depth (flow collections + block indentation).
    pub max_depth: Option<usize>,
    /// Maximum number of alias expansions (Billion Laughs protection).
    /// Enforced by the composer layer (future).
    pub max_alias_expansions: Option<usize>,
    /// Maximum number of anchors in a single document.
    /// Enforced by the composer layer (future).
    pub max_anchor_count: Option<usize>,
    /// Maximum recursion depth for `<<` merge keys.
    /// Enforced by the composer layer (future).
    pub max_merge_depth: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ResolutionPolicy {
    pub include: IncludePolicy,
    pub refs: RefPolicy,
}

#[derive(Debug, Clone)]
pub struct IncludePolicy {
    pub enabled: bool,
    pub root: Option<PathBuf>,
    pub allow: Vec<String>,       // glob patterns as strings for now
    pub deny: Vec<String>,
    pub max_depth: usize,
    pub max_total_bytes: Option<usize>,
    pub follow_symlinks: bool,
}

#[derive(Debug, Clone)]
pub struct RefPolicy {
    pub enabled: bool,
    pub allow_schemes: Vec<String>,
    pub allow_hosts: Vec<String>,
    pub timeout: Duration,
}
```

### Presets

```rust
impl Default for LoaderConfig {
    fn default() -> Self {
        Self::moderate()
    }
}

impl LoaderConfig {
    /// Moderate limits suitable for general use.
    /// External resolution disabled.
    pub fn moderate() -> Self {
        Self {
            limits: ResourceLimits {
                max_input_bytes: Some(256 * 1024 * 1024),  // 256 MB
                max_scalar_bytes: Some(64 * 1024 * 1024),  // 64 MB
                max_key_bytes: Some(1024 * 1024),           // 1 MB
                max_depth: Some(256),
                max_alias_expansions: Some(10_000),
                max_anchor_count: Some(10_000),
                max_merge_depth: Some(64),
            },
            resolution: ResolutionPolicy::disabled(),
        }
    }

    /// Strict limits for untrusted input.
    pub fn strict() -> Self {
        Self {
            limits: ResourceLimits {
                max_input_bytes: Some(10 * 1024 * 1024),   // 10 MB
                max_scalar_bytes: Some(1024 * 1024),        // 1 MB
                max_key_bytes: Some(4096),                  // 4 KB
                max_depth: Some(64),
                max_alias_expansions: Some(100),
                max_anchor_count: Some(100),
                max_merge_depth: Some(10),
            },
            resolution: ResolutionPolicy::disabled(),
        }
    }

    /// Generous limits for trusted local files.
    pub fn trusted() -> Self {
        Self {
            limits: ResourceLimits {
                max_input_bytes: None,                     // unlimited
                max_scalar_bytes: None,
                max_key_bytes: None,
                max_depth: Some(1024),
                max_alias_expansions: Some(1_000_000),
                max_anchor_count: None,
                max_merge_depth: Some(256),
            },
            resolution: ResolutionPolicy::disabled(),
        }
    }

    /// No limits whatsoever. Use only when you control the input completely.
    pub fn unchecked() -> Self {
        Self {
            limits: ResourceLimits::none(),
            resolution: ResolutionPolicy::disabled(),
        }
    }
}
```

### Scanner integration

Add `Scanner::with_config(input: &str, config: &LoaderConfig) -> Scanner`. The existing `Scanner::new(input)` calls `Self::with_config(input, &LoaderConfig::default())`.

Enforcement points:
- `flow_level` increment → check `config.limits.max_depth`
- `scan_block_scalar` / `scan_single_quoted` / `scan_double_quoted` / `scan_plain_scalar_text` → check accumulated `content.len()` against `config.limits.max_scalar_bytes`
- Simple key value length → check against `config.limits.max_key_bytes`

Errors use the existing `ScanError` type with new variants:
```rust
MaxDepthExceeded { depth: usize, limit: usize },
MaxScalarSizeExceeded { size: usize, limit: usize },
MaxKeySizeExceeded { size: usize, limit: usize },
```

### Parser integration

Add `Parser::with_config(input: &str, config: &LoaderConfig) -> Parser`. The existing `Parser::new(input)` calls `Self::with_config(input, &LoaderConfig::default())`.

Enforcement point:
- `state_stack.push()` → check `state_stack.len()` against `config.limits.max_depth`

Parser passes `config` through to the scanner it constructs internally. `Parser::from_tokens()` also gets a `from_tokens_with_config()` variant.

### Input integration

Add `Input::from_reader_with_config(reader, config: &LoaderConfig)`. Checks `buf.len()` against `config.limits.max_input_bytes` during `read_to_end`. Returns an error if the limit is exceeded before the read completes.

### Backward compatibility

All existing constructors continue to work unchanged. `new()` methods use `LoaderConfig::default()` (moderate limits). The only behavioral change: inputs that previously caused unbounded allocation now fail with a clear error at moderate limits. This is intentional and desirable.

---

## Implementation order

1. **LoaderConfig types** — define in yamalgam-core, unit tests for presets
2. **Scanner limit enforcement** — `with_config()`, depth/scalar/key checks
3. **Parser limit enforcement** — `with_config()`, state stack depth check
4. **Input limit enforcement** — `from_reader_with_config()`, buffer cap
5. **Fuzz infrastructure** — `fuzz/` directory, cargo-fuzz setup, seed corpus
6. **Fuzz targets** — all 6 targets, starting with byte-level
7. **Fuzz CI** — PR checks (60s) and nightly (1hr) workflows
8. **yamalgam-bench crate** — scaffold, peer wrappers, input generators
9. **Self-benchmarks** — scanner and parser throughput benchmarks
10. **Comparative benchmarks** — all peers on shared inputs
11. **CI benchmark workflow** — artifact storage, regression detection
12. **Public reporting sites** — bench.yamalgam.com, fuzz.yamalgam.com (later milestone)

Steps 1-4 (LoaderConfig) must precede step 6 (fuzz targets that test limits). Steps 5-7 (fuzzing) and 8-11 (benchmarks) are independent and can proceed in parallel.

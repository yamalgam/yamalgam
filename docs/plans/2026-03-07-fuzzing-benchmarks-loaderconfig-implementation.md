# Fuzzing, Benchmarking, and LoaderConfig Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement LoaderConfig resource limits, comprehensive fuzzing infrastructure, and comparative benchmarks against 6 Rust YAML peers.

**Architecture:** Three parallel tracks with one dependency: LoaderConfig (Track A) must land before fuzz_limits target (Track B, Task 5). Benchmarking (Track C) is fully independent. Each track produces a separate PR.

**Tech Stack:** cargo-fuzz (libFuzzer), arbitrary, divan/gungraun (existing benchmark framework), criterion (for yamalgam-bench comparative crate). Peers: yaml-serde, libyaml-safer, yaml-rust2, saphyr-parser, serde-saphyr, rust-yaml.

**Design doc:** `docs/plans/2026-03-07-fuzzing-benchmarks-loaderconfig-design.md`
**ADR:** `docs/decisions/0006-loaderconfig-for-resource-limits-and-security-policy.md`

---

## Track A: LoaderConfig

### Task A1: Define LoaderConfig types in yamalgam-core

**Files:**
- Create: `crates/yamalgam-core/src/loader.rs`
- Modify: `crates/yamalgam-core/src/lib.rs`

**Step 1: Write the failing test**

Create `crates/yamalgam-core/src/loader.rs` with tests at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_moderate() {
        let config = LoaderConfig::default();
        assert_eq!(config.limits.max_depth, Some(256));
        assert_eq!(config.limits.max_input_bytes, Some(256 * 1024 * 1024));
        assert!(!config.resolution.include.enabled);
        assert!(!config.resolution.refs.enabled);
    }

    #[test]
    fn strict_has_tight_limits() {
        let config = LoaderConfig::strict();
        assert_eq!(config.limits.max_depth, Some(64));
        assert_eq!(config.limits.max_scalar_bytes, Some(1024 * 1024));
        assert_eq!(config.limits.max_alias_expansions, Some(100));
    }

    #[test]
    fn trusted_has_generous_limits() {
        let config = LoaderConfig::trusted();
        assert!(config.limits.max_input_bytes.is_none());
        assert!(config.limits.max_scalar_bytes.is_none());
        assert_eq!(config.limits.max_depth, Some(1024));
    }

    #[test]
    fn unchecked_has_no_limits() {
        let config = LoaderConfig::unchecked();
        assert!(config.limits.max_depth.is_none());
        assert!(config.limits.max_input_bytes.is_none());
        assert!(config.limits.max_scalar_bytes.is_none());
        assert!(config.limits.max_alias_expansions.is_none());
    }

    #[test]
    fn resolution_disabled_by_default() {
        let disabled = ResolutionPolicy::disabled();
        assert!(!disabled.include.enabled);
        assert!(!disabled.refs.enabled);
        assert!(disabled.refs.allow_schemes.is_empty());
    }

    #[test]
    fn resource_limits_none_is_all_none() {
        let none = ResourceLimits::none();
        assert!(none.max_input_bytes.is_none());
        assert!(none.max_scalar_bytes.is_none());
        assert!(none.max_key_bytes.is_none());
        assert!(none.max_depth.is_none());
        assert!(none.max_alias_expansions.is_none());
        assert!(none.max_anchor_count.is_none());
        assert!(none.max_merge_depth.is_none());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo nextest run -p yamalgam-core -- loader`
Expected: Compilation error — `loader` module doesn't exist yet.

**Step 3: Write the implementation**

Full `loader.rs` with all types, presets, and `ResourceLimits::check_depth()` / `check_scalar_size()` / `check_key_size()` helper methods that return `Result<(), String>`:

```rust
//! Resource limits and security policy for the yamalgam loading pipeline.
//!
//! See ADR-0006 for design rationale.

use std::path::PathBuf;
use std::time::Duration;

/// Controls resource consumption and security boundaries
/// for the yamalgam loading pipeline.
///
/// Thread this through Scanner -> Parser -> Composer -> Resolver.
/// Each layer reads only the fields it needs.
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    /// Hard caps on resource consumption (DoS prevention).
    pub limits: ResourceLimits,
    /// Controls for external reference resolution (!include, $ref).
    pub resolution: ResolutionPolicy,
}

/// Hard caps on resource consumption at each pipeline layer.
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

/// Controls for external reference resolution.
#[derive(Debug, Clone)]
pub struct ResolutionPolicy {
    /// Policy for `!include` directives.
    pub include: IncludePolicy,
    /// Policy for `$ref` references.
    pub refs: RefPolicy,
}

/// Policy for `!include` file resolution.
#[derive(Debug, Clone)]
pub struct IncludePolicy {
    /// Whether `!include` is enabled at all.
    pub enabled: bool,
    /// Sandbox root — resolved paths must stay under this directory.
    pub root: Option<PathBuf>,
    /// Glob patterns to allow (checked after deny).
    pub allow: Vec<String>,
    /// Glob patterns to deny (checked first).
    pub deny: Vec<String>,
    /// Maximum include recursion depth.
    pub max_depth: usize,
    /// Maximum total bytes across all included files.
    pub max_total_bytes: Option<usize>,
    /// Whether to follow symbolic links.
    pub follow_symlinks: bool,
}

/// Policy for `$ref` reference resolution.
#[derive(Debug, Clone)]
pub struct RefPolicy {
    /// Whether `$ref` resolution is enabled at all.
    pub enabled: bool,
    /// Allowed URL schemes (empty = no network access).
    pub allow_schemes: Vec<String>,
    /// Allowed hostnames (empty = no hosts).
    pub allow_hosts: Vec<String>,
    /// Timeout for network requests.
    pub timeout: Duration,
}
```

Implement `Default`, preset constructors (`moderate`, `strict`, `trusted`, `unchecked`), `ResourceLimits::none()`, `ResolutionPolicy::disabled()`, and check helpers:

```rust
impl ResourceLimits {
    /// No limits whatsoever.
    pub const fn none() -> Self { /* all fields None */ }

    /// Check nesting depth against limit.
    /// Returns `Err(message)` if depth exceeds the configured maximum.
    pub fn check_depth(&self, depth: usize) -> Result<(), String> {
        if let Some(max) = self.max_depth {
            if depth > max {
                return Err(format!(
                    "maximum nesting depth exceeded ({depth} > {max})"
                ));
            }
        }
        Ok(())
    }

    /// Check scalar size against limit.
    pub fn check_scalar_size(&self, size: usize) -> Result<(), String> { /* similar */ }

    /// Check key size against limit.
    pub fn check_key_size(&self, size: usize) -> Result<(), String> { /* similar */ }

    /// Check input size against limit.
    pub fn check_input_size(&self, size: usize) -> Result<(), String> { /* similar */ }
}
```

**Step 4: Wire into lib.rs**

Add to `crates/yamalgam-core/src/lib.rs`:

```rust
pub mod loader;

pub use loader::{
    IncludePolicy, LoaderConfig, RefPolicy, ResolutionPolicy, ResourceLimits,
};
```

**Step 5: Run tests to verify they pass**

Run: `cargo nextest run -p yamalgam-core -- loader`
Expected: All 6 tests pass.

**Step 6: Run full check**

Run: `cargo fmt --all && just clippy`
Expected: Clean.

**Step 7: Commit**

```
feat(core): add LoaderConfig types with resource limits and resolution policy

Implements ADR-0006. Defines LoaderConfig, ResourceLimits, ResolutionPolicy,
IncludePolicy, and RefPolicy with four presets: moderate (default), strict,
trusted, and unchecked.
```

---

### Task A2: Scanner integration — with_config() and depth check

**Files:**
- Modify: `crates/yamalgam-scanner/src/scanner.rs:84-179` (Scanner struct + new())
- Modify: `crates/yamalgam-scanner/tests/scanner.rs`

**Step 1: Write the failing test**

Add to `crates/yamalgam-scanner/tests/scanner.rs`:

```rust
#[test]
fn max_depth_rejects_deep_flow_nesting() {
    use yamalgam_core::LoaderConfig;

    let mut config = LoaderConfig::strict(); // max_depth = 64
    config.limits.max_depth = Some(3);

    let input = "[[[[nested]]]]";
    let result: Result<Vec<_>, _> = Scanner::with_config(input, &config).collect();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.message.contains("nesting depth"),
        "expected depth error, got: {err}"
    );
}

#[test]
fn max_depth_allows_within_limit() {
    use yamalgam_core::LoaderConfig;

    let mut config = LoaderConfig::strict();
    config.limits.max_depth = Some(3);

    let input = "[[[ok]]]";
    let result: Result<Vec<_>, _> = Scanner::with_config(input, &config).collect();
    assert!(result.is_ok());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo nextest run -p yamalgam-scanner -- max_depth`
Expected: Compilation error — `Scanner::with_config` doesn't exist.

**Step 3: Implement with_config and depth enforcement**

Add `config: ResourceLimits` field to `Scanner` struct (only the limits sub-struct — scanner doesn't need resolution policy). Store a clone. In `fetch_flow_collection_start` (line 755), after `self.flow_level += 1`, add:

```rust
if let Err(msg) = self.config.check_depth(self.flow_level as usize) {
    self.error = Some(ScanError { message: msg });
    return;
}
```

Add `with_config` constructor:

```rust
pub fn with_config(input: &'input str, config: &LoaderConfig) -> Self {
    let mut scanner = Self::new(input);
    scanner.config = config.limits.clone();
    scanner
}
```

Change `Scanner::new()` — drop `const`, initialize `config: ResourceLimits::none()`.

Also check depth on `roll_indent` (block nesting). The indent stack depth is another nesting dimension:

```rust
// In roll_indent, after indent_stack.push():
if let Err(msg) = self.config.check_depth(self.indent_stack.len() + self.flow_level as usize) {
    self.error = Some(ScanError { message: msg });
    return;
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p yamalgam-scanner`
Expected: All existing tests pass (they use `Scanner::new()` → `ResourceLimits::none()` → no limits). New depth tests pass.

**Step 5: Commit**

```
feat(scanner): add with_config() and max_depth enforcement

Scanner now accepts ResourceLimits via with_config(). Enforces max_depth
on flow collection nesting (flow_level) and block indentation (indent_stack).
Existing Scanner::new() remains unlimited for backward compatibility.
```

---

### Task A3: Scanner integration — scalar and key size limits

**Files:**
- Modify: `crates/yamalgam-scanner/src/scanner.rs` (fetch_block_scalar, fetch_single_quoted_scalar, fetch_double_quoted_scalar, scan_plain_scalar_text)
- Modify: `crates/yamalgam-scanner/tests/scanner.rs`

**Step 1: Write the failing tests**

```rust
#[test]
fn max_scalar_bytes_rejects_oversized_scalar() {
    use yamalgam_core::LoaderConfig;

    let mut config = LoaderConfig::strict();
    config.limits.max_scalar_bytes = Some(10);

    let input = "this scalar is definitely longer than ten bytes";
    let result: Result<Vec<_>, _> = Scanner::with_config(input, &config).collect();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("scalar size"), "got: {err}");
}

#[test]
fn max_scalar_bytes_allows_within_limit() {
    use yamalgam_core::LoaderConfig;

    let mut config = LoaderConfig::strict();
    config.limits.max_scalar_bytes = Some(100);

    let input = "short";
    let result: Result<Vec<_>, _> = Scanner::with_config(input, &config).collect();
    assert!(result.is_ok());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo nextest run -p yamalgam-scanner -- max_scalar`
Expected: FAIL — no size check exists yet.

**Step 3: Implement scalar size checks**

Add size checks at the end of each scalar scanning method, after content is fully accumulated but before the token is enqueued. Four methods need the check:

- `fetch_plain_scalar` (~line 1227) — after `scan_plain_scalar_text()` returns
- `fetch_block_scalar` (~line 1430) — after the content loop completes
- `fetch_single_quoted_scalar` (~line 1660) — after the loop `break`s
- `fetch_double_quoted_scalar` (~line 1774) — after the loop `break`s

Pattern (same in all four):

```rust
if let Err(msg) = self.config.check_scalar_size(content.len()) {
    self.error = Some(ScanError { message: msg });
    return;
}
```

For key size: check in `stale_simple_key_check` or when a simple key resolves — the key value is the scalar that was saved. Alternatively, since keys are just scalars, the scalar size check already catches oversized keys. Add `max_key_bytes` as a tighter check specifically on scalars that resolve as keys. Implement in `resolve_simple_key()` by checking the resolved token's data length against `config.check_key_size()`.

**Step 4: Run tests**

Run: `cargo nextest run -p yamalgam-scanner`
Expected: All pass.

**Step 5: Commit**

```
feat(scanner): enforce max_scalar_bytes and max_key_bytes limits

Checks scalar content size after accumulation in all four scalar scanning
methods (plain, block, single-quoted, double-quoted). Checks key size
when a simple key resolves.
```

---

### Task A4: Parser integration — with_config() and state stack depth

**Files:**
- Modify: `crates/yamalgam-parser/src/parser.rs:69-108` (Parser struct + constructors)
- Modify: `crates/yamalgam-parser/tests/parser.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn max_depth_rejects_deep_nesting() {
    use yamalgam_core::LoaderConfig;

    let mut config = LoaderConfig::strict();
    config.limits.max_depth = Some(3);

    // Deeply nested block mapping
    let input = "a:\n  b:\n    c:\n      d:\n        e: too deep";
    let result: Vec<_> = Parser::with_config(input, &config).collect();
    assert!(result.iter().any(|r| r.is_err()));
}

#[test]
fn max_depth_allows_within_limit() {
    use yamalgam_core::LoaderConfig;

    let mut config = LoaderConfig::strict();
    config.limits.max_depth = Some(10);

    let input = "a:\n  b:\n    c: ok";
    let result: Result<Vec<_>, _> = Parser::with_config(input, &config).collect();
    assert!(result.is_ok());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo nextest run -p yamalgam-parser -- max_depth`
Expected: Compilation error — `Parser::with_config` doesn't exist.

**Step 3: Implement**

Add `config: ResourceLimits` field to `Parser`. Add `with_config` constructor that passes config through to `Scanner::with_config`:

```rust
pub fn with_config(input: &'input str, config: &LoaderConfig) -> Self {
    Self::from_tokens_with_config(Scanner::with_config(input, config), config)
}

pub fn from_tokens_with_config(
    tokens: impl Iterator<Item = Result<Token<'input>, ScanError>> + 'input,
    config: &LoaderConfig,
) -> Self {
    Self {
        tokens: Box::new(tokens),
        state: ParserState::StreamStart,
        state_stack: Vec::new(),
        peeked: None,
        done: false,
        seen_directive: false,
        config: config.limits.clone(),
    }
}
```

In `push_state` (line 128-130), add depth check:

```rust
fn push_state(&mut self, state: ParserState) -> Result<(), ParseError> {
    self.state_stack.push(state);
    if let Err(msg) = self.config.check_depth(self.state_stack.len()) {
        return Err(ParseError::MaxDepthExceeded(msg));
    }
    Ok(())
}
```

This changes `push_state` from infallible to fallible. Update all call sites to propagate the error with `?`.

**Step 4: Run tests**

Run: `cargo nextest run -p yamalgam-parser`
Expected: All pass.

**Step 5: Commit**

```
feat(parser): add with_config() and max_depth enforcement on state stack

Parser now accepts LoaderConfig via with_config(). Enforces max_depth
on state stack pushes. Passes config through to Scanner::with_config().
```

---

### Task A5: Input integration — from_reader size limit

**Files:**
- Modify: `crates/yamalgam-scanner/src/input.rs:142-165` (from_reader)
- Modify: `crates/yamalgam-scanner/tests/scanner.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn from_reader_with_config_rejects_oversized_input() {
    use yamalgam_core::LoaderConfig;
    use yamalgam_scanner::Input;

    let mut config = LoaderConfig::strict();
    config.limits.max_input_bytes = Some(10);

    let data = b"this input is way too long for the configured limit";
    let result = Input::from_reader_with_config(&data[..], &config);
    assert!(result.is_err());
}
```

**Step 2: Implement**

Add `from_reader_with_config` to `Input`:

```rust
pub fn from_reader_with_config(
    reader: impl Read,
    config: &LoaderConfig,
) -> Result<Input<'static>, Diagnostic> {
    let mut buf = Vec::new();
    if let Some(max) = config.limits.max_input_bytes {
        // Read in chunks, checking size after each
        let mut limited = reader.take((max + 1) as u64);
        limited.read_to_end(&mut buf)
            .map_err(|e| encoding_error(&format!("I/O error: {e}")))?;
        if buf.len() > max {
            return Err(encoding_error(&format!(
                "input exceeds maximum size ({} > {max} bytes)", buf.len()
            )));
        }
    } else {
        reader.read_to_end(&mut buf)
            .map_err(|e| encoding_error(&format!("I/O error: {e}")))?;
    }
    // ... rest same as from_reader (encoding detection + decode)
}
```

Note: `reader.take()` prevents reading more than `max + 1` bytes, so the process doesn't OOM before the check fires.

**Step 3: Run tests**

Run: `cargo nextest run -p yamalgam-scanner`
Expected: All pass.

**Step 4: Commit**

```
feat(scanner): add Input::from_reader_with_config() with max_input_bytes enforcement

Uses reader.take() to cap buffer growth before the size check, preventing
OOM on adversarial input streams.
```

---

### Task A6: Run full check, verify compliance unchanged

**Step 1:** Run `cargo fmt --all && just clippy`
**Step 2:** Run `cargo nextest run` (all 917+ tests)
**Step 3:** Run compliance: `cargo nextest run -p yamalgam-compare --test compliance --success-output=immediate 2>&1 | grep -oE "^    (PASS|UNEXPECTED|MISMATCH|EXPECTED|EVENT_PASS|EVENT_UNEXPECTED|EVENT_MISMATCH|EVENT_EXPECTED)" | sort | uniq -c | sort -rn`

Expected: Compliance numbers unchanged (349 EVENT_PASS, 334 TOKEN_PASS, 0 UNEXPECTED).

**Step 4: Commit** (if any fmt/clippy fixes needed)

---

## Track B: Fuzzing Infrastructure

### Task B1: Scaffold fuzz directory with cargo-fuzz

**Files:**
- Create: `fuzz/Cargo.toml`
- Create: `fuzz/.gitignore`
- Modify: `Cargo.toml` (workspace members)

**Step 1: Install cargo-fuzz if not present**

Run: `cargo install cargo-fuzz` (if not already installed)

**Step 2: Create fuzz/Cargo.toml**

```toml
[package]
name = "yamalgam-fuzz"
version = "0.0.0"
publish = false
edition = "2024"

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.4"
arbitrary = { version = "1", features = ["derive"] }
yamalgam-scanner = { path = "../crates/yamalgam-scanner" }
yamalgam-parser = { path = "../crates/yamalgam-parser" }
yamalgam-core = { path = "../crates/yamalgam-core" }

# Prevent this from interfering with workspace lints
[lints.rust]
missing_docs = "allow"
[lints.clippy]
all = "allow"
nursery = "allow"

[[bin]]
name = "fuzz_scanner_bytes"
path = "fuzz_targets/fuzz_scanner_bytes.rs"
doc = false

[[bin]]
name = "fuzz_parser_bytes"
path = "fuzz_targets/fuzz_parser_bytes.rs"
doc = false

[[bin]]
name = "fuzz_scanner_structured"
path = "fuzz_targets/fuzz_scanner_structured.rs"
doc = false

[[bin]]
name = "fuzz_parser_structured"
path = "fuzz_targets/fuzz_parser_structured.rs"
doc = false

[[bin]]
name = "fuzz_limits"
path = "fuzz_targets/fuzz_limits.rs"
doc = false

[[bin]]
name = "fuzz_differential"
path = "fuzz_targets/fuzz_differential.rs"
doc = false
```

**Step 3: Create fuzz/.gitignore**

```
target/
corpus/
artifacts/
!corpus/seed/
```

**Step 4: Add to workspace**

In root `Cargo.toml`, change members to include fuzz:

```toml
members = ["xtask", "crates/*", "fuzz"]
```

**Step 5: Verify workspace compiles**

Run: `cargo check --workspace`

**Step 6: Commit**

```
feat(fuzz): scaffold cargo-fuzz infrastructure with 6 target bins
```

---

### Task B2: Byte-level fuzz targets

**Files:**
- Create: `fuzz/fuzz_targets/fuzz_scanner_bytes.rs`
- Create: `fuzz/fuzz_targets/fuzz_parser_bytes.rs`

**Step 1: Write fuzz_scanner_bytes.rs**

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use yamalgam_scanner::Scanner;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = Scanner::new(input).collect::<Result<Vec<_>, _>>();
    }
});
```

**Step 2: Write fuzz_parser_bytes.rs**

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use yamalgam_parser::Parser;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = Parser::new(input).collect::<Result<Vec<_>, _>>();
    }
});
```

**Step 3: Verify targets compile**

Run: `cargo +nightly fuzz build`
Expected: All targets compile.

**Step 4: Smoke test (10 seconds each)**

Run: `cargo +nightly fuzz run fuzz_scanner_bytes -- -max_total_time=10`
Run: `cargo +nightly fuzz run fuzz_parser_bytes -- -max_total_time=10`
Expected: No crashes.

**Step 5: Commit**

```
feat(fuzz): add byte-level fuzz targets for scanner and parser
```

---

### Task B3: Seed corpus from YAML Test Suite

**Files:**
- Create: `fuzz/corpus/seed/` (directory with extracted YAML inputs)
- Create: `scripts/seed-fuzz-corpus` (extraction script)

**Step 1: Write extraction script**

`scripts/seed-fuzz-corpus`:

```bash
#!/usr/bin/env bash
set -euo pipefail

SUITE_DIR="vendor/yaml-test-suite"
SEED_DIR="fuzz/corpus/seed"

mkdir -p "$SEED_DIR"

count=0
for f in "$SUITE_DIR"/*.yaml; do
    id=$(basename "$f" .yaml)
    # Extract the YAML input section (between markers)
    # The test suite uses ↵ for newlines, — for ---, etc.
    cp "$f" "$SEED_DIR/${id}.yaml"
    count=$((count + 1))
done

echo "Seeded $count test cases into $SEED_DIR"
```

**Step 2: Run it**

Run: `bash scripts/seed-fuzz-corpus`
Expected: 351 files in `fuzz/corpus/seed/`.

**Step 3: Commit**

```
feat(fuzz): seed corpus from YAML Test Suite (351 inputs)
```

---

### Task B4: Structured fuzzer with arbitrary YAML generator

**Files:**
- Create: `fuzz/src/lib.rs` (shared arbitrary YAML generator)
- Create: `fuzz/fuzz_targets/fuzz_scanner_structured.rs`
- Create: `fuzz/fuzz_targets/fuzz_parser_structured.rs`

**Step 1: Create the Arbitrary YAML generator**

`fuzz/src/lib.rs`:

```rust
//! Structured YAML generator for fuzzing.

use arbitrary::{Arbitrary, Unstructured};

#[derive(Debug)]
pub struct YamlDoc {
    pub nodes: Vec<YamlNode>,
}

#[derive(Debug, Arbitrary)]
pub enum ScalarStyle {
    Plain,
    SingleQuoted,
    DoubleQuoted,
    Literal,
    Folded,
}

#[derive(Debug, Arbitrary)]
pub enum YamlNode {
    Scalar {
        value: String,
        style: ScalarStyle,
    },
    Mapping {
        entries: Vec<(YamlNode, YamlNode)>,
    },
    Sequence {
        items: Vec<YamlNode>,
        flow: bool,
    },
    Alias {
        anchor_ref: u8,
    },
    Anchored {
        anchor_id: u8,
        node: Box<YamlNode>,
    },
}

impl<'a> Arbitrary<'a> for YamlDoc {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let count = u.int_in_range(1..=3)?;
        let mut nodes = Vec::with_capacity(count);
        for _ in 0..count {
            nodes.push(YamlNode::arbitrary(u)?);
        }
        Ok(YamlDoc { nodes })
    }
}

impl YamlDoc {
    /// Render to a YAML string. Best-effort — may produce invalid YAML
    /// (which is fine for fuzzing, we just want to exercise the parser).
    pub fn render(&self) -> String {
        let mut out = String::new();
        for (i, node) in self.nodes.iter().enumerate() {
            if i > 0 {
                out.push_str("---\n");
            }
            node.render_to(&mut out, 0);
            out.push('\n');
        }
        out
    }
}

impl YamlNode {
    fn render_to(&self, out: &mut String, indent: usize) {
        let pad: String = " ".repeat(indent);
        match self {
            YamlNode::Scalar { value, style } => {
                match style {
                    ScalarStyle::Plain => out.push_str(value),
                    ScalarStyle::SingleQuoted => {
                        out.push('\'');
                        out.push_str(&value.replace('\'', "''"));
                        out.push('\'');
                    }
                    ScalarStyle::DoubleQuoted => {
                        out.push('"');
                        out.push_str(&value.replace('\\', "\\\\").replace('"', "\\\""));
                        out.push('"');
                    }
                    ScalarStyle::Literal => {
                        out.push_str("|\n");
                        for line in value.lines() {
                            out.push_str(&pad);
                            out.push_str("  ");
                            out.push_str(line);
                            out.push('\n');
                        }
                    }
                    ScalarStyle::Folded => {
                        out.push_str(">\n");
                        for line in value.lines() {
                            out.push_str(&pad);
                            out.push_str("  ");
                            out.push_str(line);
                            out.push('\n');
                        }
                    }
                }
            }
            YamlNode::Mapping { entries } => {
                if entries.is_empty() {
                    out.push_str("{}");
                    return;
                }
                for (k, v) in entries {
                    out.push_str(&pad);
                    k.render_to(out, indent);
                    out.push_str(": ");
                    v.render_to(out, indent + 2);
                    out.push('\n');
                }
            }
            YamlNode::Sequence { items, flow } => {
                if *flow || items.is_empty() {
                    out.push('[');
                    for (i, item) in items.iter().enumerate() {
                        if i > 0 { out.push_str(", "); }
                        item.render_to(out, 0);
                    }
                    out.push(']');
                } else {
                    for item in items {
                        out.push_str(&pad);
                        out.push_str("- ");
                        item.render_to(out, indent + 2);
                        out.push('\n');
                    }
                }
            }
            YamlNode::Alias { anchor_ref } => {
                out.push_str(&format!("*a{anchor_ref}"));
            }
            YamlNode::Anchored { anchor_id, node } => {
                out.push_str(&format!("&a{anchor_id} "));
                node.render_to(out, indent);
            }
        }
    }
}
```

**Step 2: Write structured fuzz targets**

`fuzz/fuzz_targets/fuzz_scanner_structured.rs`:
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use yamalgam_fuzz::YamlDoc;
use yamalgam_scanner::Scanner;

fuzz_target!(|doc: YamlDoc| {
    let yaml = doc.render();
    let _ = Scanner::new(&yaml).collect::<Result<Vec<_>, _>>();
});
```

`fuzz/fuzz_targets/fuzz_parser_structured.rs`:
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use yamalgam_fuzz::YamlDoc;
use yamalgam_parser::Parser;

fuzz_target!(|doc: YamlDoc| {
    let yaml = doc.render();
    let _ = Parser::new(&yaml).collect::<Result<Vec<_>, _>>();
});
```

**Step 3: Update fuzz/Cargo.toml**

Add `[lib]` section:
```toml
[lib]
name = "yamalgam_fuzz"
path = "src/lib.rs"
```

**Step 4: Build and smoke test**

Run: `cargo +nightly fuzz build`
Run: `cargo +nightly fuzz run fuzz_scanner_structured -- -max_total_time=10`
Expected: Compiles and runs without crashing.

**Step 5: Commit**

```
feat(fuzz): add structured YAML generator and structured fuzz targets
```

---

### Task B5: Limits and differential fuzz targets

**Files:**
- Create: `fuzz/fuzz_targets/fuzz_limits.rs`
- Create: `fuzz/fuzz_targets/fuzz_differential.rs`

**Step 1: Write fuzz_limits.rs**

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use yamalgam_core::LoaderConfig;
use yamalgam_scanner::Scanner;
use yamalgam_parser::Parser;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let config = LoaderConfig::strict();
        // Must not panic. Errors are expected and acceptable.
        let _ = Scanner::with_config(input, &config).collect::<Result<Vec<_>, _>>();
        let _ = Parser::with_config(input, &config).collect::<Result<Vec<_>, _>>();
    }
});
```

**Step 2: Write fuzz_differential.rs**

This target requires the C harness binary. It compares yamalgam output against libfyaml. The differential target shells out to the C harness rather than linking it:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use std::process::Command;
use yamalgam_scanner::Scanner;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else { return };
    if input.is_empty() { return; }

    // Run yamalgam scanner
    let rust_result = Scanner::new(input).collect::<Result<Vec<_>, _>>();

    // Run C harness
    let c_output = Command::new("./tools/fyaml-tokenize/fyaml-tokenize")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    let Ok(mut child) = c_output else { return }; // C harness not built, skip

    use std::io::Write;
    if let Some(ref mut stdin) = child.stdin {
        let _ = stdin.write_all(input.as_bytes());
    }
    drop(child.stdin.take());

    let Ok(output) = child.wait_with_output() else { return };

    let c_succeeded = output.status.success();
    let rust_succeeded = rust_result.is_ok();

    // Both accept or both reject = fine.
    // Disagreement = interesting, log but don't panic.
    // (Panicking on disagreement would make this a regression-finder
    //  rather than a crash-finder. We want both behaviors configurable.)
    if rust_succeeded != c_succeeded {
        // Write to stderr for the fuzzer to capture
        eprintln!(
            "DISAGREEMENT: rust={}, c={}, input={:?}",
            if rust_succeeded { "OK" } else { "ERR" },
            if c_succeeded { "OK" } else { "ERR" },
            &input[..input.len().min(200)]
        );
    }
});
```

**Step 3: Build and smoke test**

Run: `cargo +nightly fuzz build`
Run: `cargo +nightly fuzz run fuzz_limits -- -max_total_time=10`
Expected: Compiles and runs.

**Step 4: Commit**

```
feat(fuzz): add limits enforcement and differential fuzz targets
```

---

### Task B6: Justfile recipes for fuzzing

**Files:**
- Modify: `.justfile`

**Step 1: Add recipes**

```just
# Fuzzing
fuzz target='fuzz_scanner_bytes' duration='60':
    cargo +nightly fuzz run {{target}} -- -max_total_time={{duration}}

fuzz-all duration='60':
    #!/usr/bin/env bash
    set -euo pipefail
    for target in fuzz_scanner_bytes fuzz_parser_bytes fuzz_scanner_structured \
                  fuzz_parser_structured fuzz_limits fuzz_differential; do
        echo "=== Fuzzing $target for {{duration}}s ==="
        cargo +nightly fuzz run "$target" -- -max_total_time={{duration}} || true
    done

fuzz-long:
    just fuzz-all 3600

fuzz-corpus-min:
    cargo +nightly fuzz cmin fuzz_scanner_bytes
    cargo +nightly fuzz cmin fuzz_parser_bytes
```

**Step 2: Verify**

Run: `just fuzz fuzz_scanner_bytes 5`
Expected: Runs for 5 seconds.

**Step 3: Commit**

```
feat(fuzz): add just recipes for fuzzing operations
```

---

### Task B7: Fuzzing CI workflow

**Files:**
- Create: `.github/workflows/fuzz.yml`

**Step 1: Write the workflow**

```yaml
name: Fuzz

on:
  pull_request:
  schedule:
    - cron: '0 3 * * *'  # 3 AM UTC daily
  workflow_dispatch:
    inputs:
      duration:
        description: 'Seconds per target'
        default: '60'

jobs:
  fuzz:
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        target:
          - fuzz_scanner_bytes
          - fuzz_parser_bytes
          - fuzz_scanner_structured
          - fuzz_parser_structured
          - fuzz_limits
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: true
      - uses: dtolnay/rust-toolchain@nightly
      - run: cargo install cargo-fuzz
      - name: Run fuzzer
        run: |
          DURATION=${{ github.event.inputs.duration || (github.event_name == 'schedule' && '3600' || '60') }}
          cargo +nightly fuzz run ${{ matrix.target }} -- -max_total_time=$DURATION
      - name: Upload crashes
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: fuzz-crashes-${{ matrix.target }}
          path: fuzz/artifacts/
          retention-days: 90
```

Note: `fuzz_differential` is excluded from CI (requires C harness build). Add separately when the C harness is in CI.

**Step 2: Commit**

```
ci(fuzz): add PR and nightly fuzzing workflow
```

---

## Track C: Comparative Benchmarks

### Task C1: Scaffold yamalgam-bench crate

**Files:**
- Create: `crates/yamalgam-bench/Cargo.toml`
- Create: `crates/yamalgam-bench/src/lib.rs`

**Step 1: Create Cargo.toml**

```toml
[package]
name = "yamalgam-bench"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Comparative benchmarks for yamalgam vs YAML peers"
publish = false

[dependencies]
yamalgam-scanner = { path = "../yamalgam-scanner" }
yamalgam-parser = { path = "../yamalgam-parser" }
yamalgam-core = { path = "../yamalgam-core" }

# Peers
yaml-serde = "0.9"
libyaml-safer = "0.3"
yaml-rust2 = "0.11"
saphyr-parser = "0.2"
serde-saphyr = "0.0.21"
rust-yaml = "0.0.5"
serde = { version = "1.0", features = ["derive"] }

[dev-dependencies]
divan = "0.1"
rand = { version = "0.9", features = ["small_rng"] }

[lints]
workspace = true

[[bench]]
name = "comparative"
harness = false
```

Note: Exact versions for peers may need adjustment — check crates.io at implementation time.

**Step 2: Create src/lib.rs with input generators**

```rust
//! Comparative benchmark infrastructure.

pub mod inputs;
pub mod peers;
```

**Step 3: Verify workspace compiles**

Run: `cargo check -p yamalgam-bench`
Expected: Compiles (may need version adjustments for peers).

**Step 4: Commit**

```
feat(bench): scaffold yamalgam-bench comparative benchmark crate
```

---

### Task C2: Input generators

**Files:**
- Create: `crates/yamalgam-bench/src/inputs.rs`

**Step 1: Write deterministic input generators**

Inspired by yaml-rust2's `gen_large_yaml` (seeded PRNG, seed=42):

```rust
//! Deterministic YAML input generators for benchmarks.

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// Kubernetes-style deployment YAML (~2KB).
pub fn kubernetes_deployment() -> String {
    // Static, hand-written, real-world representative input
    include_str!("../inputs/kubernetes-deployment.yaml").to_string()
}

/// N records with varied field types (strings, ints, URLs, hashes).
pub fn records(n: usize) -> String {
    let mut rng = SmallRng::seed_from_u64(42);
    let mut out = String::new();
    out.push_str("records:\n");
    for i in 0..n {
        out.push_str(&format!("  - id: {i}\n"));
        out.push_str(&format!("    name: \"record-{}\"\n", rng.random::<u32>()));
        out.push_str(&format!("    value: {}\n", rng.random::<f64>()));
        out.push_str(&format!("    active: {}\n", rng.random::<bool>()));
        out.push_str("    tags:\n");
        let tag_count = rng.random_range(1..=4);
        for _ in 0..tag_count {
            out.push_str(&format!("      - tag-{}\n", rng.random::<u16>()));
        }
    }
    out
}

/// Deeply nested block mapping.
pub fn nested(depth: usize) -> String {
    let mut out = String::new();
    for i in 0..depth {
        let indent = "  ".repeat(i);
        out.push_str(&format!("{indent}level{i}:\n"));
    }
    let indent = "  ".repeat(depth);
    out.push_str(&format!("{indent}leaf: value\n"));
    out
}

/// Many small 2-field objects.
pub fn small_objects(n: usize) -> String {
    let mut rng = SmallRng::seed_from_u64(42);
    let mut out = String::new();
    for _ in 0..n {
        out.push_str(&format!("- name: obj-{}\n", rng.random::<u16>()));
        out.push_str(&format!("  score: {}\n", rng.random::<u8>()));
    }
    out
}

/// N anchors with M aliases each (anchor/alias stress test).
pub fn anchored(anchors: usize, aliases_per: usize) -> String {
    let mut out = String::new();
    // Define anchors
    for i in 0..anchors {
        out.push_str(&format!("anchor{i}: &a{i}\n"));
        out.push_str(&format!("  field: value-{i}\n"));
    }
    // Use aliases
    out.push_str("refs:\n");
    for i in 0..anchors {
        for _ in 0..aliases_per {
            out.push_str(&format!("  - *a{i}\n"));
        }
    }
    out
}

/// Single large block scalar.
pub fn large_scalar(bytes: usize) -> String {
    let mut out = String::new();
    out.push_str("content: |\n");
    let line = "  The quick brown fox jumps over the lazy dog.\n";
    while out.len() < bytes {
        out.push_str(line);
    }
    out
}
```

**Step 2: Create static input file**

Create `crates/yamalgam-bench/inputs/kubernetes-deployment.yaml` — a realistic ~2KB Kubernetes Deployment manifest.

**Step 3: Commit**

```
feat(bench): add deterministic input generators for comparative benchmarks
```

---

### Task C3: Peer wrappers

**Files:**
- Create: `crates/yamalgam-bench/src/peers.rs`

**Step 1: Write thin wrappers**

```rust
//! Thin wrappers around each peer's parse API for uniform benchmarking.

/// Parse with yamalgam scanner (tokens only).
pub fn yamalgam_scan(input: &str) {
    let _ = yamalgam_scanner::Scanner::new(input)
        .collect::<Result<Vec<_>, _>>();
}

/// Parse with yamalgam parser (events).
pub fn yamalgam_parse(input: &str) {
    let _ = yamalgam_parser::Parser::new(input)
        .collect::<Result<Vec<_>, _>>();
}

/// Parse with yaml-serde (serde Value).
pub fn yaml_serde_parse(input: &str) {
    let _ = yaml_serde::from_str::<yaml_serde::Value>(input);
}

/// Parse with libyaml-safer (Document load).
pub fn libyaml_safer_parse(input: &str) {
    // API may vary — adjust at implementation time
    let _ = libyaml_safer::Document::load_from_str(input);
}

/// Parse with yaml-rust2 (DOM load).
pub fn yaml_rust2_parse(input: &str) {
    let _ = yaml_rust2::YamlLoader::load_from_str(input);
}

/// Parse with saphyr-parser (event sink).
pub fn saphyr_parser_parse(input: &str) {
    // saphyr-parser has its own Parser; API may vary
    // let mut parser = saphyr_parser::Parser::new_from_str(input);
    // while parser.next().is_some() {}
}

/// Parse with serde-saphyr (serde Value).
pub fn serde_saphyr_parse(input: &str) {
    let _ = serde_saphyr::from_str::<serde_saphyr::Value>(input);
}

/// Parse with rust-yaml (DOM load).
pub fn rust_yaml_parse(input: &str) {
    let _ = rust_yaml::Yaml::load_str(input);
}
```

Note: Exact APIs will need adjustment at implementation time — check each crate's docs.

**Step 2: Verify compiles**

Run: `cargo check -p yamalgam-bench`

**Step 3: Commit**

```
feat(bench): add peer parser wrappers for comparative benchmarks
```

---

### Task C4: Divan comparative benchmarks

**Files:**
- Create: `crates/yamalgam-bench/benches/comparative.rs`

**Step 1: Write the benchmark harness**

```rust
use divan::Bencher;
use yamalgam_bench::{inputs, peers};

fn main() {
    divan::main();
}

// --- Small input (Kubernetes deployment, ~2KB) ---

#[divan::bench]
fn small_yamalgam_scan(b: Bencher) {
    let input = inputs::kubernetes_deployment();
    b.bench(|| peers::yamalgam_scan(&input));
}

#[divan::bench]
fn small_yamalgam_parse(b: Bencher) {
    let input = inputs::kubernetes_deployment();
    b.bench(|| peers::yamalgam_parse(&input));
}

#[divan::bench]
fn small_yaml_serde(b: Bencher) {
    let input = inputs::kubernetes_deployment();
    b.bench(|| peers::yaml_serde_parse(&input));
}

#[divan::bench]
fn small_libyaml_safer(b: Bencher) {
    let input = inputs::kubernetes_deployment();
    b.bench(|| peers::libyaml_safer_parse(&input));
}

#[divan::bench]
fn small_yaml_rust2(b: Bencher) {
    let input = inputs::kubernetes_deployment();
    b.bench(|| peers::yaml_rust2_parse(&input));
}

#[divan::bench]
fn small_saphyr_parser(b: Bencher) {
    let input = inputs::kubernetes_deployment();
    b.bench(|| peers::saphyr_parser_parse(&input));
}

#[divan::bench]
fn small_serde_saphyr(b: Bencher) {
    let input = inputs::kubernetes_deployment();
    b.bench(|| peers::serde_saphyr_parse(&input));
}

#[divan::bench]
fn small_rust_yaml(b: Bencher) {
    let input = inputs::kubernetes_deployment();
    b.bench(|| peers::rust_yaml_parse(&input));
}

// --- Medium input (1K records, ~500KB) ---

#[divan::bench]
fn medium_yamalgam_scan(b: Bencher) {
    let input = inputs::records(1_000);
    b.bench(|| peers::yamalgam_scan(&input));
}

#[divan::bench]
fn medium_yamalgam_parse(b: Bencher) {
    let input = inputs::records(1_000);
    b.bench(|| peers::yamalgam_parse(&input));
}

#[divan::bench]
fn medium_yaml_serde(b: Bencher) {
    let input = inputs::records(1_000);
    b.bench(|| peers::yaml_serde_parse(&input));
}

#[divan::bench]
fn medium_yaml_rust2(b: Bencher) {
    let input = inputs::records(1_000);
    b.bench(|| peers::yaml_rust2_parse(&input));
}

// ... repeat for each peer at medium size ...

// --- Large input (10K records, ~5MB) ---

#[divan::bench]
fn large_yamalgam_scan(b: Bencher) {
    let input = inputs::records(10_000);
    b.bench(|| peers::yamalgam_scan(&input));
}

#[divan::bench]
fn large_yamalgam_parse(b: Bencher) {
    let input = inputs::records(10_000);
    b.bench(|| peers::yamalgam_parse(&input));
}

#[divan::bench]
fn large_yaml_serde(b: Bencher) {
    let input = inputs::records(10_000);
    b.bench(|| peers::yaml_serde_parse(&input));
}

// ... repeat for each peer at large size ...
```

**Step 2: Run benchmarks**

Run: `cargo bench -p yamalgam-bench --bench comparative`
Expected: Results printed for all peers.

**Step 3: Commit**

```
feat(bench): add divan comparative benchmarks across 3 input sizes and 6 peers
```

---

### Task C5: Self-benchmarks in scanner and parser crates

**Files:**
- Create: `crates/yamalgam-scanner/benches/benchmarks.kdl` (or direct divan file)
- Create: `crates/yamalgam-parser/benches/benchmarks.kdl`
- Modify: `crates/yamalgam-scanner/Cargo.toml` (add divan dev-dep + [[bench]])
- Modify: `crates/yamalgam-parser/Cargo.toml` (add divan dev-dep + [[bench]])

These measure yamalgam's own scanner and parser throughput in isolation — not comparative. Use the existing KDL → divan codegen pattern from xtask, or write divan benchmarks directly if simpler.

Scanner benchmarks: `scan_small`, `scan_medium`, `scan_large`, `scan_nested_256`, `scan_large_scalar`.
Parser benchmarks: `parse_small`, `parse_medium`, `parse_large`, `parse_anchored`.

**Step 1: Add dev-dependencies and bench targets to Cargo.toml files**

**Step 2: Write benchmark files**

**Step 3: Run and verify**

Run: `cargo bench -p yamalgam-scanner` and `cargo bench -p yamalgam-parser`

**Step 4: Commit**

```
feat(bench): add scanner and parser self-benchmarks (throughput at 3 input sizes)
```

---

### Task C6: Justfile recipes and CI workflow for benchmarks

**Files:**
- Modify: `.justfile`
- Modify: `.github/workflows/benchmarks.yml`

**Step 1: Add justfile recipes**

```just
bench-comparative:
    cargo bench -p yamalgam-bench --bench comparative

bench-scanner:
    cargo bench -p yamalgam-scanner

bench-parser:
    cargo bench -p yamalgam-parser
```

**Step 2: Add comparative benchmark job to CI workflow**

Add a `comparative` job to `.github/workflows/benchmarks.yml` that runs `cargo bench -p yamalgam-bench` and uploads results.

**Step 3: Commit**

```
ci(bench): add comparative benchmark recipes and CI job
```

---

## Track Summary

| Track | Tasks | Estimated commits | Dependencies |
|-------|-------|-------------------|--------------|
| **A: LoaderConfig** | A1-A6 | 5-6 | None |
| **B: Fuzzing** | B1-B7 | 6-7 | A (for fuzz_limits target) |
| **C: Benchmarks** | C1-C6 | 5-6 | None |

**Parallel execution:** Tracks A and C can run simultaneously. Track B starts after A is merged but C can proceed independently throughout.

**Future milestones (not in this plan):**
- bench.yamalgam.com / fuzz.yamalgam.com reporting sites (Tufte-inspired design)
- OSS-Fuzz submission
- Miri weekly runs
- AFL++ nightly alternation
- Cross-language benchmarks via hyperfine tier + WASM bridges

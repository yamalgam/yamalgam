# Scanner Foundation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the yamalgam scanner/tokenizer with comparison infrastructure that validates behavioral equivalence against libfyaml.

**Architecture:** Hybrid port of libfyaml 0.9.5 — state machine ported faithfully with `// cref:` annotations, data structures redesigned idiomatically in Rust. A comparison MCP server feeds identical YAML to both implementations and diffs token streams. YAML Test Suite vendored for compliance testing.

**Tech Stack:** Rust (edition 2024, MSRV 1.88.0), `bitflags`, `encoding_rs`, `rmcp` (MCP SDK), `datatest-stable`, `miette`, `serde`/`serde_json` (for token serialization in comparison). C (libfyaml via small harness).

**Design doc:** `docs/plans/2026-03-06-scanner-foundation-design.md`

**Skills:** @idiomatic-rust @code-annotations

**Branch strategy:** Each task group = one PR on a feature branch. `just check` must pass before merging.

---

## Task 1: Project Scaffolding

**Branch:** `chore/scaffold/milestone-1-crates`

**Files:**
- Create: `crates/yamalgam-scanner/Cargo.toml`
- Create: `crates/yamalgam-scanner/src/lib.rs`
- Create: `crates/yamalgam-compare/Cargo.toml`
- Create: `crates/yamalgam-compare/src/lib.rs`
- Create: `crates/yamalgam-mcp/Cargo.toml`
- Create: `crates/yamalgam-mcp/src/main.rs`
- Create: `tools/fyaml-tokenize/` (directory only)
- Modify: `crates/yamalgam-core/src/lib.rs`
- Modify: `crates/yamalgam-core/Cargo.toml`
- Create: `crates/yamalgam-core/src/diagnostic.rs`

### Step 1: Create crates via add-crate script

Run:
```bash
scripts/add-crate internal yamalgam-scanner -d "YAML scanner/tokenizer ported from libfyaml"
scripts/add-crate internal yamalgam-compare -d "Comparison logic for validating against libfyaml"
scripts/add-crate bin yamalgam-mcp -d "MCP server for scanner comparison during development"
```

Expected: Three new directories under `crates/`, each with `Cargo.toml` and `src/`.

### Step 2: Add initial dependencies to yamalgam-scanner

Edit `crates/yamalgam-scanner/Cargo.toml`:
```toml
[dependencies]
bitflags = "2"
encoding_rs = "0.8"
thiserror = "2.0"
yamalgam-core = { path = "../yamalgam-core" }

[dev-dependencies]
pretty_assertions = "1"
```

### Step 3: Add initial dependencies to yamalgam-compare

Edit `crates/yamalgam-compare/Cargo.toml`:
```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
yamalgam-core = { path = "../yamalgam-core" }
yamalgam-scanner = { path = "../yamalgam-scanner" }

[dev-dependencies]
pretty_assertions = "1"
```

### Step 4: Add initial dependencies to yamalgam-mcp

Edit `crates/yamalgam-mcp/Cargo.toml`:
```toml
[dependencies]
anyhow = "1.0"
rmcp = { version = "0.12", features = ["server", "transport-io", "macros"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
yamalgam-compare = { path = "../yamalgam-compare" }
yamalgam-scanner = { path = "../yamalgam-scanner" }
```

### Step 5: Add Diagnostic type to yamalgam-core

Create `crates/yamalgam-core/src/diagnostic.rs`:

```rust
//! Diagnostic types shared across all yamalgam crates.
//!
//! Designed for compatibility with `miette` for terminal rendering.

use serde::{Deserialize, Serialize};

/// Severity level for a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    /// Informational hint.
    Hint,
    /// Informational message.
    Info,
    /// Non-fatal warning.
    Warning,
    /// Fatal error.
    Error,
}

/// A source position in the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Mark {
    /// Zero-indexed line number.
    pub line: u32,
    /// Zero-indexed column number (in bytes).
    pub column: u32,
    /// Byte offset from start of input.
    pub offset: usize,
}

/// A source range (start..end) in the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Span {
    /// Start position (inclusive).
    pub start: Mark,
    /// End position (exclusive).
    pub end: Mark,
}

/// A labeled region of source code within a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    /// The source range this label covers.
    pub span: Span,
    /// Human-readable message for this label.
    pub message: String,
}

/// A diagnostic message with source location and severity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Severity of this diagnostic.
    pub severity: Severity,
    /// Machine-readable error code (e.g., "E001").
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Primary source location (if applicable).
    pub span: Option<Span>,
    /// Additional labeled spans for context.
    pub labels: Vec<Label>,
}
```

### Step 6: Wire diagnostic module into yamalgam-core

Modify `crates/yamalgam-core/src/lib.rs` to add:
```rust
pub mod diagnostic;
pub use diagnostic::{Diagnostic, Label, Mark, Severity, Span};
```

Add `serde` derive dependencies if not already present in `crates/yamalgam-core/Cargo.toml`.

### Step 7: Create tools directory for C harness

Run:
```bash
mkdir -p tools/fyaml-tokenize
```

### Step 8: Run just check

Run: `just check`
Expected: All format, clippy, deny, test, doc-test pass.

### Step 9: Commit

```bash
git checkout -b chore/scaffold/milestone-1-crates
git add crates/yamalgam-scanner/ crates/yamalgam-compare/ crates/yamalgam-mcp/ \
        crates/yamalgam-core/src/diagnostic.rs crates/yamalgam-core/src/lib.rs \
        crates/yamalgam-core/Cargo.toml tools/fyaml-tokenize/ \
        docs/plans/2026-03-06-scanner-foundation-design.md \
        docs/plans/2026-03-06-scanner-foundation-plan.md \
        .claude/skills/idiomatic-rust/SKILL.md
git commit -m "chore(scaffold): add milestone-1 crates, diagnostic type, and design docs"
```

---

## Task 2: Vendor YAML Test Suite

**Branch:** `chore/vendor/yaml-test-suite`

**Files:**
- Create: `vendor/yaml-test-suite/` (git archive snapshot)
- Modify: `.justfile` (add update-test-suite and test-compliance recipes)
- Create: `scripts/update-test-suite`

### Step 1: Clone and vendor the YAML Test Suite

Run:
```bash
git clone --depth 1 https://github.com/yaml/yaml-test-suite.git /tmp/yaml-test-suite
cp -r /tmp/yaml-test-suite/src vendor/yaml-test-suite
```

Inspect the structure — each test case is a directory with files like `in.yaml`, `test.event`, `===`, `error` (if expected error), etc.

### Step 2: Create update script

Create `scripts/update-test-suite`:
```bash
#!/usr/bin/env bash
# update-test-suite - Refresh vendored YAML Test Suite snapshot
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEST="$PROJECT_ROOT/vendor/yaml-test-suite"

echo "Fetching latest YAML Test Suite..."
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

git clone --depth 1 https://github.com/yaml/yaml-test-suite.git "$TMP/yaml-test-suite"
rm -rf "$DEST"
cp -r "$TMP/yaml-test-suite/src" "$DEST"

# Record the commit hash for traceability
HASH=$(git -C "$TMP/yaml-test-suite" rev-parse HEAD)
echo "$HASH" > "$DEST/COMMIT_SHA"
echo "Vendored yaml-test-suite at $HASH"
```

Run: `chmod +x scripts/update-test-suite`

### Step 3: Add justfile recipes

Add to `.justfile`:
```just
# Update vendored YAML Test Suite snapshot
update-test-suite:
    scripts/update-test-suite

# Run YAML Test Suite compliance tests
test-compliance:
    cargo nextest run -p yamalgam-compare --test compliance
```

### Step 4: Run just check

Run: `just check`
Expected: Pass (no tests exist yet that reference the suite).

### Step 5: Commit

```bash
git checkout -b chore/vendor/yaml-test-suite
git add vendor/yaml-test-suite/ scripts/update-test-suite .justfile
git commit -m "chore(vendor): add YAML Test Suite snapshot"
```

---

## Task 3: Scanner Core Types

**Branch:** `feat/scanner/core-types`

**Files:**
- Create: `crates/yamalgam-scanner/src/token.rs`
- Create: `crates/yamalgam-scanner/src/atom.rs`
- Create: `crates/yamalgam-scanner/src/style.rs`
- Modify: `crates/yamalgam-scanner/src/lib.rs`
- Create: `crates/yamalgam-scanner/tests/types.rs`

### Step 1: Write tests for core types

Create `crates/yamalgam-scanner/tests/types.rs`:

```rust
use yamalgam_scanner::{Atom, AtomFlags, Chomp, Mark, ScalarStyle, Span, Token, TokenKind};
use std::borrow::Cow;

#[test]
fn token_display_includes_kind_and_span() {
    let token = Token {
        kind: TokenKind::Scalar,
        atom: Atom {
            data: Cow::Borrowed("hello"),
            span: Span {
                start: Mark { line: 0, column: 0, offset: 0 },
                end: Mark { line: 0, column: 5, offset: 5 },
            },
            style: ScalarStyle::Plain,
            chomp: Chomp::Clip,
            flags: AtomFlags::empty(),
        },
    };
    assert_eq!(token.kind, TokenKind::Scalar);
    assert_eq!(token.atom.data.as_ref(), "hello");
}

#[test]
fn atom_flags_compose() {
    let flags = AtomFlags::HAS_LB | AtomFlags::HAS_WS;
    assert!(flags.contains(AtomFlags::HAS_LB));
    assert!(flags.contains(AtomFlags::HAS_WS));
    assert!(!flags.contains(AtomFlags::DIRECT_OUTPUT));
}

#[test]
fn mark_default_is_zero() {
    let mark = Mark::default();
    assert_eq!(mark.line, 0);
    assert_eq!(mark.column, 0);
    assert_eq!(mark.offset, 0);
}

#[test]
fn scalar_style_variants_exist() {
    // Ensure all five YAML scalar styles are represented
    let styles = [
        ScalarStyle::Plain,
        ScalarStyle::SingleQuoted,
        ScalarStyle::DoubleQuoted,
        ScalarStyle::Literal,
        ScalarStyle::Folded,
    ];
    assert_eq!(styles.len(), 5);
}
```

### Step 2: Run tests, verify they fail

Run: `cargo nextest run -p yamalgam-scanner`
Expected: Compilation error — types don't exist yet.

### Step 3: Implement TokenKind enum

Create `crates/yamalgam-scanner/src/token.rs`. Port all token type variants from libfyaml's `enum fy_token_type` in `fy-token.h`. Add `// cref: fy_token_type` at the top. Each variant gets a doc comment explaining what YAML syntax it represents.

### Step 4: Implement Atom, ScalarStyle, Chomp, AtomFlags

Create `crates/yamalgam-scanner/src/atom.rs` with `Atom<'input>` struct.
Create `crates/yamalgam-scanner/src/style.rs` with `ScalarStyle`, `Chomp` enums and `AtomFlags` bitflags.

All types derive `Debug, Clone, PartialEq, Eq` at minimum. `Serialize`/`Deserialize` on types needed by the comparison infrastructure.

Use `Mark` and `Span` from `yamalgam-core::diagnostic` (re-export from scanner for convenience).

### Step 5: Wire up lib.rs

Modify `crates/yamalgam-scanner/src/lib.rs`:
```rust
//! YAML scanner/tokenizer ported from libfyaml 0.9.5.
//!
//! This crate provides the lowest layer of the yamalgam YAML pipeline:
//! byte input → token stream. The scanner is version-agnostic; YAML version
//! directives are emitted as `Directive` tokens for the parser layer to handle.
#![deny(unsafe_code)]

mod atom;
mod style;
mod token;

pub use atom::Atom;
pub use style::{AtomFlags, Chomp, ScalarStyle};
pub use token::{Token, TokenKind};

// Re-export position types from core for convenience
pub use yamalgam_core::{Mark, Span};
```

### Step 6: Run tests, verify they pass

Run: `cargo nextest run -p yamalgam-scanner`
Expected: All 4 tests pass.

### Step 7: Run just check

Run: `just check`
Expected: Pass.

### Step 8: Commit

```bash
git checkout -b feat/scanner/core-types
git add crates/yamalgam-scanner/
git commit -m "feat(scanner): add core types — Token, Atom, TokenKind, ScalarStyle, AtomFlags"
```

---

## Task 4: C Harness (`fyaml-tokenize`)

**Branch:** `feat/compare/c-harness`

**Files:**
- Create: `tools/fyaml-tokenize/main.c`
- Create: `tools/fyaml-tokenize/Makefile`
- Create: `tools/fyaml-tokenize/README.md`

### Step 1: Write the C harness

Create `tools/fyaml-tokenize/main.c`:

A C program that:
1. Creates an `fy_parser` with default config
2. Reads from stdin (or filename arg)
3. Calls `fy_parser_parse()` in a loop
4. For each token/event, outputs a JSON object to stdout with fields: `type`, `value`, `line`, `column`, `offset`, `style`
5. Flushes after each line
6. Exits 0 on success, 1 on error

Reference: `vendor/libfyaml-0.9.5/include/libfyaml.h` for the API.

Add `// cref:` comments noting which libfyaml API functions are used.

### Step 2: Write the Makefile

Create `tools/fyaml-tokenize/Makefile` that:
- Compiles `main.c` against vendored libfyaml
- Links the necessary libfyaml source files
- Produces `fyaml-tokenize` binary
- Has `clean` target

### Step 3: Build and smoke test

Run:
```bash
cd tools/fyaml-tokenize && make
echo 'key: value' | ./fyaml-tokenize
```

Expected: JSON lines on stdout showing stream-start, document-start, mapping-start, key, scalar("key"), value, scalar("value"), mapping-end, document-end, stream-end (or the token-level equivalent).

### Step 4: Test with a YAML Test Suite case

Run:
```bash
cat vendor/yaml-test-suite/229Q/in.yaml | tools/fyaml-tokenize/fyaml-tokenize
```

Expected: Valid JSON token output.

### Step 5: Test error handling

Run:
```bash
echo -e 'bad:\n  - [unclosed' | tools/fyaml-tokenize/fyaml-tokenize
```

Expected: Non-zero exit, error JSON on stderr.

### Step 6: Commit

```bash
git checkout -b feat/compare/c-harness
git add tools/fyaml-tokenize/
git commit -m "feat(compare): add fyaml-tokenize C harness for token stream comparison"
```

---

## Task 5: Input Layer

**Branch:** `feat/scanner/input-layer`

**Files:**
- Create: `crates/yamalgam-scanner/src/input.rs`
- Create: `crates/yamalgam-scanner/src/reader.rs`
- Create: `crates/yamalgam-scanner/tests/input.rs`
- Modify: `crates/yamalgam-scanner/src/lib.rs`

### Step 1: Write tests for BOM detection and transcoding

Create `crates/yamalgam-scanner/tests/input.rs`:

```rust
use yamalgam_scanner::input::Input;

#[test]
fn utf8_no_bom() {
    let input = Input::from_bytes(b"hello: world").unwrap();
    assert_eq!(input.as_str(), "hello: world");
}

#[test]
fn utf8_with_bom() {
    let input = Input::from_bytes(b"\xEF\xBB\xBFhello: world").unwrap();
    assert_eq!(input.as_str(), "hello: world");
}

#[test]
fn utf16le_with_bom() {
    // "a: b" in UTF-16LE with BOM
    let bytes: Vec<u8> = vec![
        0xFF, 0xFE, // BOM
        b'a', 0, b':', 0, b' ', 0, b'b', 0,
    ];
    let input = Input::from_bytes(&bytes).unwrap();
    assert_eq!(input.as_str(), "a: b");
}

#[test]
fn utf16be_with_bom() {
    // "a: b" in UTF-16BE with BOM
    let bytes: Vec<u8> = vec![
        0xFE, 0xFF, // BOM
        0, b'a', 0, b':', 0, b' ', 0, b'b',
    ];
    let input = Input::from_bytes(&bytes).unwrap();
    assert_eq!(input.as_str(), "a: b");
}

#[test]
fn from_reader_works() {
    let cursor = std::io::Cursor::new(b"hello: world");
    let input = Input::from_reader(cursor).unwrap();
    assert_eq!(input.as_str(), "hello: world");
}
```

### Step 2: Run tests, verify they fail

Run: `cargo nextest run -p yamalgam-scanner`
Expected: Compilation error — `input` module doesn't exist.

### Step 3: Implement Input

Create `crates/yamalgam-scanner/src/input.rs`:

- `Input` enum or struct that owns or borrows UTF-8 data
- `from_bytes()` — detects BOM (UTF-8, UTF-16LE, UTF-16BE, UTF-32LE, UTF-32BE), transcodes via `encoding_rs`, strips BOM, returns Input
- `from_reader()` — reads all into Vec<u8>, delegates to `from_bytes()`
- `as_str()` — returns the UTF-8 content as `&str`
- For `&'input [u8]` that's already UTF-8 (no BOM): borrows without copying
- For transcoded input: owns the String

### Step 4: Implement Reader

Create `crates/yamalgam-scanner/src/reader.rs`:

- `Reader<'input>` wraps `&'input str`
- Tracks current `Mark` (line, column, offset)
- Methods: `peek() -> Option<char>`, `peek_at(n) -> Option<char>`, `advance() -> Option<char>`, `advance_by(n)`, `current_mark() -> Mark`, `is_eof() -> bool`
- Updates line/column on newlines

### Step 5: Wire up lib.rs

Add `pub mod input;` and `mod reader;` to `crates/yamalgam-scanner/src/lib.rs`.

### Step 6: Run tests, verify they pass

Run: `cargo nextest run -p yamalgam-scanner`
Expected: All input tests pass.

### Step 7: Run just check

Run: `just check`

### Step 8: Commit

```bash
git checkout -b feat/scanner/input-layer
git add crates/yamalgam-scanner/
git commit -m "feat(scanner): add input layer with BOM detection and UTF-8/16/32 transcoding"
```

---

## Task 6: Comparison Library

**Branch:** `feat/compare/library`

**Files:**
- Create: `crates/yamalgam-compare/src/snapshot.rs`
- Create: `crates/yamalgam-compare/src/compare.rs`
- Create: `crates/yamalgam-compare/src/harness.rs`
- Modify: `crates/yamalgam-compare/src/lib.rs`
- Create: `crates/yamalgam-compare/tests/compare_tests.rs`
- Create: `crates/yamalgam-compare/tests/compliance.rs`

### Step 1: Write tests for comparison logic

Create `crates/yamalgam-compare/tests/compare_tests.rs`:

```rust
use yamalgam_compare::{CompareResult, TokenSnapshot, SpanSnapshot, compare_token_streams};

#[test]
fn identical_streams_match() {
    let tokens = vec![
        TokenSnapshot {
            kind: "StreamStart".to_string(),
            value: None,
            style: None,
            span: SpanSnapshot::default(),
        },
        TokenSnapshot {
            kind: "StreamEnd".to_string(),
            value: None,
            style: None,
            span: SpanSnapshot::default(),
        },
    ];
    let result = compare_token_streams(&tokens, &tokens);
    assert!(matches!(result, CompareResult::Match { token_count: 2 }));
}

#[test]
fn different_kinds_produce_mismatch() {
    let c_tokens = vec![TokenSnapshot {
        kind: "Scalar".to_string(),
        value: Some("foo".to_string()),
        style: Some("Plain".to_string()),
        span: SpanSnapshot::default(),
    }];
    let rust_tokens = vec![TokenSnapshot {
        kind: "Scalar".to_string(),
        value: Some("bar".to_string()),
        style: Some("Plain".to_string()),
        span: SpanSnapshot::default(),
    }];
    let result = compare_token_streams(&c_tokens, &rust_tokens);
    assert!(matches!(result, CompareResult::TokenMismatch { index: 0, .. }));
}
```

### Step 2: Run tests, verify they fail

Run: `cargo nextest run -p yamalgam-compare`

### Step 3: Implement snapshot types and comparison

Create `crates/yamalgam-compare/src/snapshot.rs` with `TokenSnapshot`, `SpanSnapshot`.
Create `crates/yamalgam-compare/src/compare.rs` with `CompareResult` enum and `compare_token_streams()`.
Create `crates/yamalgam-compare/src/harness.rs` with:
- `run_c_tokenizer(input: &[u8]) -> Result<Vec<TokenSnapshot>>` — invokes `fyaml-tokenize` subprocess, parses JSON output
- `run_rust_scanner(input: &[u8]) -> Result<Vec<TokenSnapshot>>` — runs yamalgam scanner, converts tokens to snapshots
- `compare_input(input: &[u8]) -> Result<CompareResult>` — runs both, compares

### Step 4: Create compliance test harness

Create `crates/yamalgam-compare/tests/compliance.rs` using `datatest-stable`:

- Discovers all directories in `vendor/yaml-test-suite/`
- For each case, reads `in.yaml`, feeds to both implementations
- Reports pass/fail per case with `CompareResult` details

This will initially show 0/320+ passing since the scanner has no state machine yet. That's expected — the infrastructure is what we're proving.

### Step 5: Run tests, verify they pass

Run: `cargo nextest run -p yamalgam-compare`
Expected: Unit tests pass. Compliance tests run but report expected failures.

### Step 6: Commit

```bash
git checkout -b feat/compare/library
git add crates/yamalgam-compare/
git commit -m "feat(compare): add comparison library with token snapshot diffing and compliance harness"
```

---

## Task 7: MCP Server

**Branch:** `feat/mcp/comparison-server`

**Files:**
- Modify: `crates/yamalgam-mcp/src/main.rs`
- Create: `crates/yamalgam-mcp/src/tools.rs`
- Create: `crates/yamalgam-mcp/src/paths.rs`

### Step 1: Implement path discovery

Create `crates/yamalgam-mcp/src/paths.rs`:
- Find project root by walking up from executable looking for `Cargo.toml` with `[workspace]`
- Locate `tools/fyaml-tokenize/fyaml-tokenize` binary
- Locate `vendor/yaml-test-suite/` directory
- Return `YamalgamPaths` struct with validated paths

### Step 2: Implement tools

Create `crates/yamalgam-mcp/src/tools.rs` using `rmcp` macros:

**`list_test_cases`:**
- Parameters: optional `filter` (string)
- Reads `vendor/yaml-test-suite/` directories
- Returns JSON array of test case IDs with metadata (has error file, description from `===` file)

**`compare_tokens`:**
- Parameters: `test_case` (optional string, test suite ID) or `input` (optional string, raw YAML)
- Calls `yamalgam_compare::compare_input()`
- Returns structured JSON `CompareResult`

**`debug_scanner`:**
- Parameters: `input` (string), optional `filter` (string for grep)
- Runs yamalgam scanner with `RUST_LOG=debug`
- Returns filtered trace output

### Step 3: Wire up main.rs

Modify `crates/yamalgam-mcp/src/main.rs`:
- Initialize tracing to stderr
- Create `YamalgamServer` with discovered paths
- Serve via `rmcp` stdio transport

### Step 4: Smoke test the MCP server

Run:
```bash
echo '{"jsonrpc":"2.0","method":"initialize","params":{"capabilities":{}},"id":1}' | cargo run -p yamalgam-mcp
```

Expected: JSON-RPC response with server capabilities listing the three tools.

### Step 5: Run just check

Run: `just check`

### Step 6: Commit

```bash
git checkout -b feat/mcp/comparison-server
git add crates/yamalgam-mcp/
git commit -m "feat(mcp): add comparison MCP server with list, compare, and debug tools"
```

---

## Task 8+: Scanner State Machine (Incremental)

**Branch pattern:** `feat/scanner/<token-category>`

This is the main porting work. Each PR adds recognition of a category of tokens, with compliance test counts tracked per PR.

Suggested order (following libfyaml's scanner flow):

| PR | Category | Tokens Added |
|---|---|---|
| 8 | Stream markers | `StreamStart`, `StreamEnd` |
| 9 | Document markers | `DocumentStart` (`---`), `DocumentEnd` (`...`), `Directive` (`%YAML`, `%TAG`) |
| 10 | Flow indicators | `FlowSequenceStart/End`, `FlowMappingStart/End`, `FlowEntry` |
| 11 | Block indicators | `BlockSequenceStart`, `BlockMappingStart`, `BlockEntry`, `Key`, `Value` |
| 12 | Plain scalars | `Scalar` with `ScalarStyle::Plain` |
| 13 | Quoted scalars | `Scalar` with `SingleQuoted`, `DoubleQuoted` (escape processing) |
| 14 | Block scalars | `Scalar` with `Literal`, `Folded` (chomp, indentation) |
| 15 | Anchors and aliases | `Anchor`, `Alias` |
| 16 | Tags | `Tag` (verbatim, shorthand, non-specific) |
| 17 | Comments | Comment handling (not a token, but affects scanner state) |
| 18 | Edge cases | Remaining test suite failures, complex key handling, nested flows |

Each PR follows TDD:
1. Write/enable compliance tests for the target category
2. Run, observe failures
3. Port the relevant state machine code from libfyaml with `// cref:` annotations
4. Run, observe passing
5. Run `just check`
6. Commit with compliance count: `feat(scanner): add plain scalars (N/320 passing)`

### Reference files in libfyaml

- **Scanner states:** `vendor/libfyaml-0.9.5/src/lib/fy-parse.h` — `enum fy_parser_state`
- **Scanner implementation:** `vendor/libfyaml-0.9.5/src/lib/fy-parse.c` — the state machine
- **Token types:** `vendor/libfyaml-0.9.5/src/lib/fy-token.h`
- **Atom handling:** `vendor/libfyaml-0.9.5/src/lib/fy-atom.c` / `fy-atom.h`
- **Parser internals:** `vendor/libfyaml-0.9.5/src/internal/libfyaml-parser.c`

### Testing strategy per PR

- `cargo nextest run -p yamalgam-scanner` — unit tests for the new token category
- `cargo nextest run -p yamalgam-compare --test compliance` — full suite compliance count
- MCP `compare_tokens` tool — ad-hoc testing during development
- `just check` — gate before merge

---

## Summary: PR Sequence

| # | Branch | Description |
|---|---|---|
| 1 | `chore/scaffold/milestone-1-crates` | Crate scaffolding, Diagnostic type, design docs |
| 2 | `chore/vendor/yaml-test-suite` | Vendored test suite, update script, justfile recipes |
| 3 | `feat/scanner/core-types` | Token, Atom, TokenKind, ScalarStyle, AtomFlags |
| 4 | `feat/compare/c-harness` | fyaml-tokenize C binary |
| 5 | `feat/scanner/input-layer` | BOM detection, UTF-8/16/32 transcoding, Reader |
| 6 | `feat/compare/library` | CompareResult, TokenSnapshot, compliance harness |
| 7 | `feat/mcp/comparison-server` | MCP server with 3 tools |
| 8-18 | `feat/scanner/<category>` | Incremental state machine porting |

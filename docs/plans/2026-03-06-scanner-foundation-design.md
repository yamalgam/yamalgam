# Scanner Foundation Design

**Date:** 2026-03-06
**Status:** Approved
**Scope:** Milestone 1 — scanner, comparison infrastructure, development practice

---

## Context

yamalgam is a pure Rust port of libfyaml 0.9.5, targeting full YAML 1.2 compliance with a three-layer API: serde (direct-to-struct), DOM (node tree), and CST (round-trip comment/style preservation). This document covers the first milestone: the scanner/tokenizer foundation and the comparison infrastructure that validates it.

### Why libfyaml

libfyaml is the most compliant and performant C YAML parser available: 100% YAML Test Suite, zero-copy, up to 24x faster streaming than libyaml. Its scanner architecture was designed from scratch for YAML 1.2, avoiding the inherited limitations of libyaml-derived parsers (1024-char implicit key limit, incremental spec compliance via patching).

### Why from scratch

yamalgam's goals — `!include`, `$ref`, linting, schema validation, CST round-trip, serde + facet serialization — require deep control over the parser pipeline. These features cannot be bolted onto an existing parser without fundamental changes to its architecture.

### Porting approach

**Hybrid (Approach C):** Port libfyaml's state machine faithfully. Redesign the data structures idiomatically in Rust. The state machine is where correctness lives; the data structures are where C-isms live.

- `// cref:` annotations on all blocks covering the behavioral equivalent of a C function, even when the Rust structure differs
- A comparison MCP server validates behavioral equivalence against libfyaml continuously during development

See: [ADR-001](../decisions/), `.claude/skills/idiomatic-rust/`, `.claude/skills/code-annotations/`

---

## Workspace Structure

```
crates/
  yamalgam/              # CLI binary (exists)
  yamalgam-core/         # Shared library: diagnostics, config, types (exists)
  yamalgam-scanner/      # Internal: tokenizer/scanner (new, publish=false)
  yamalgam-compare/      # Internal: comparison logic library (new, publish=false)
  yamalgam-mcp/          # Internal: MCP server for comparison (new, publish=false)

vendor/
  libfyaml-0.9.5/       # Reference C implementation (exists)
  yaml-test-suite/       # Vendored snapshot (new)

tools/
  fyaml-tokenize/        # C harness: stdin YAML -> stdout JSON tokens (new)
```

### Future crates (not in this milestone)

`yamalgam-lint`, `yamalgam-schema`, `yamalgam-serde`, `yamalgam-facet`, `yamalgam-figment`. Listed for orientation; none are built in Milestone 1.

---

## Scanner Crate (`yamalgam-scanner`)

### Public API

```rust
pub struct Scanner<'input> { ... }

impl<'input> Scanner<'input> {
    /// Create from a byte slice (zero-copy path).
    pub fn new(input: &'input [u8]) -> Result<Self, Diagnostic>;

    /// Create from a reader (buffered/owned path).
    pub fn from_reader(reader: impl Read) -> Result<Self, Diagnostic>;

    /// Advance to the next token.
    pub fn next_token(&mut self) -> Result<Option<Token<'input>>, Diagnostic>;
}

/// Also implements Iterator<Item = Result<Token<'input>, Diagnostic>>
```

### Core Types

```rust
pub struct Token<'input> {
    pub kind: TokenKind,
    pub atom: Atom<'input>,
}

pub struct Atom<'input> {
    pub data: Cow<'input, str>,
    pub span: Span,
    pub style: ScalarStyle,
    pub chomp: Chomp,
    pub flags: AtomFlags,
}

pub struct Mark { pub line: u32, pub column: u32, pub offset: usize }
pub struct Span { pub start: Mark, pub end: Mark }

pub enum TokenKind {
    StreamStart, StreamEnd,
    DocumentStart, DocumentEnd,           // --- / ...
    BlockSequenceStart, BlockMappingStart,
    FlowSequenceStart, FlowSequenceEnd,   // [ / ]
    FlowMappingStart, FlowMappingEnd,     // { / }
    BlockEntry, FlowEntry,                // - / ,
    Key, Value,                           // ? / :
    Anchor, Alias,                        // &name / *name
    Tag,                                  // !tag / !!tag
    Scalar,
    Directive,                            // %YAML, %TAG
    // ... remaining variants from libfyaml's fy_token_type
}

pub enum ScalarStyle { Plain, SingleQuoted, DoubleQuoted, Literal, Folded }
pub enum Chomp { Strip, Clip, Keep }

bitflags! {
    pub struct AtomFlags: u32 {
        const HAS_LB            = 0b0000_0001;
        const HAS_WS            = 0b0000_0010;
        const STARTS_WITH_WS    = 0b0000_0100;
        const ENDS_WITH_WS      = 0b0000_1000;
        const DIRECT_OUTPUT     = 0b0001_0000;
        // ... remaining flags from libfyaml's fy_atom
    }
}
```

### Internal Architecture

- **Input layer:** BOM detection, UTF-8/16/32 transcoding (via `encoding_rs`). UTF-8 input borrows directly; transcoded input is owned. The encoding boundary is at input, not scattered through the scanner.
- **Reader layer:** Lookahead over UTF-8 buffer. Tracks current `Mark`. Provides `peek()`, `advance()`, `match_char()`.
- **State machine:** Ported from libfyaml's scanner states with `// cref:` annotations. States stored as an enum + explicit stack (not recursion). Mirrors libfyaml's approach to avoid stack overflow on deeply nested documents.
- **Token queue:** `VecDeque<Token>` (mirrors libfyaml's `queued_tokens`). `next_token()` drains the queue, scanning more input when empty.
- **YAML version:** Scanner tokenizes uniformly regardless of version. `%YAML` directives are emitted as `Directive` tokens. Version-specific semantics (boolean resolution, octal parsing) are handled by the resolver layer above.

### Atom Design Rationale

The atom concept is preserved from libfyaml because:

1. **CST fidelity.** Style, chomp, and content flags are presentation information. Discarding them at scan time makes round-trip comment/style preservation impossible.
2. **Emitter performance.** Flags like `DIRECT_OUTPUT` let the emitter write scalars verbatim without re-analyzing content. libfyaml proved these optimizations matter.
3. **The C code already proved it.** These flags solve real problems Antoniou encountered during libfyaml development. Stripping them invites re-discovering the same problems.

The Rust representation is cleaner (bitflags, enums, methods) but the information content matches libfyaml's `fy_atom`.

---

## Diagnostics

Defined in `yamalgam-core` so all crates share the type. Compatible with `miette` for terminal rendering.

```rust
pub struct Diagnostic {
    pub severity: Severity,
    pub code: DiagnosticCode,
    pub message: String,
    pub span: Option<Span>,
    pub labels: Vec<Label>,
}

pub struct Label {
    pub span: Span,
    pub message: String,
}

pub enum Severity { Error, Warning, Info, Hint }
```

The scanner can produce multiple diagnostics (warnings) before a fatal error. The comparison infrastructure compares diagnostics between implementations.

---

## Comparison Infrastructure

### C Harness (`tools/fyaml-tokenize`)

A small C program (~50-100 lines) linked against vendored libfyaml:

- Reads YAML from stdin
- Tokenizes via `fy_parser_create()` / `fy_parser_parse()`
- Outputs JSON to stdout: one object per token (`kind`, `span`, `value`, `style`, `chomp`)
- Exits 0 on success, non-zero on error (error JSON on stderr)

Invoked as a subprocess. No FFI bindings to maintain.

### Comparison Library (`yamalgam-compare`)

Internal crate, no MCP dependency. Reusable by both the MCP server and `cargo nextest` tests.

```rust
pub enum CompareResult {
    Match { token_count: usize },
    BothErrorMatch,
    BothErrorMismatch { c_error: String, rust_error: String },
    CSuccessRustError { rust_error: String, c_token_count: usize },
    RustSuccessCError { c_error: String, rust_token_count: usize },
    TokenMismatch {
        index: usize,
        c_token: TokenSnapshot,
        rust_token: TokenSnapshot,
        context: Vec<TokenSnapshot>,
    },
}

pub struct TokenSnapshot {
    pub kind: String,
    pub value: Option<String>,
    pub style: Option<String>,
    pub span: SpanSnapshot,
}
```

`TokenSnapshot` is implementation-neutral and serializable. Comparison is structural: kind must match exactly, values must match, spans have configurable tolerance.

### MCP Server (`yamalgam-mcp`)

Built on `rmcp`. Stdio transport. Three tools:

| Tool | Purpose |
|---|---|
| `list_test_cases` | List YAML Test Suite cases, filterable by tag/error/name |
| `compare_tokens` | Feed input to both implementations, return `CompareResult` as JSON |
| `debug_scanner` | Run yamalgam scanner with trace logging, return filtered output |

`compare_tokens` accepts a test suite case ID or raw YAML input.

### YAML Test Suite

Vendored snapshot at `vendor/yaml-test-suite/`. Updated via `just update-test-suite`. Compliance tests use `datatest-stable` for file-driven test generation: one test per case.

---

## Development Practice

### Branches and PRs

- `main` is always green (`just check` passes)
- Feature branches: `feat/<scope>/<short-description>`
- Bug fixes: `fix/<scope>/<short-description>`
- Chore/infra: `chore/<scope>/<short-description>`
- Every change gets a PR with a description explaining what and why
- Scopes map to crate names: `scanner`, `core`, `compare`, `mcp`, `cli`, `vendor`

### Conventional Commits

```
feat(scanner): implement BOM detection and UTF-8/16/32 transcoding
fix(compare): handle trailing newline difference in token values
chore(vendor): update yaml-test-suite to 2026-03-01 snapshot
```

### PR Requirements

- Rationale in the description (not just what changed, but why)
- Link to design doc or ADR for significant decisions
- `just check` passes
- Compliance test counts for scanner work (N/320+ passing)

### ADRs

Captured in `docs/decisions/` using MADR format. Initial ADRs from this design:

- ADR-001: Port libfyaml state machine, redesign data structures
- ADR-002: Preserve atom concept for CST fidelity
- ADR-003: Comparison MCP server as correctness verification
- ADR-004: UTF-8/16/32 encoding support from day one

### Bus-Factor Chain

A new contributor follows this path:

1. `AGENTS.md` — workspace layout, commands, conventions
2. `docs/plans/` — design docs with rationale
3. `docs/decisions/` — ADRs for architectural choices
4. `.claude/skills/` — coding conventions for agents
5. `// cref:` annotations — traceable link from Rust to libfyaml C source
6. PR history — narrative of why things were built in sequence
7. Comparison MCP — verifiable correctness against the C reference

---

## Milestone 1 PR Sequence

| PR | Scope |
|---|---|
| 1 | Workspace scaffolding: new crates, vendored test suite, `Diagnostic` type |
| 2 | C harness: build system, JSON token output, smoke tests |
| 3 | Scanner core types: `Token`, `Atom`, `Span`, `Mark`, `TokenKind`, `AtomFlags` |
| 4 | Input layer: BOM detection, UTF-8/16/32 transcoding, scanner constructors |
| 5 | Comparison library: `CompareResult`, `TokenSnapshot`, `datatest-stable` harness |
| 6 | MCP server: `list_test_cases`, `compare_tokens`, `debug_scanner` |
| 7+ | Scanner state machine, ported incrementally with compliance counts per PR |

### Success Criteria

- Scanner tokenizes all 320+ YAML Test Suite cases
- Comparison MCP confirms token-level equivalence with libfyaml on passing cases
- Error cases match libfyaml's error detection
- `just check` green throughout
- Every PR has conventional commits and rationale

### Out of Scope

Parser/event layer, document/DOM/CST, `!include`, `$ref`, serde, facet, figment, linter, schema validator, emitter, new CLI commands.

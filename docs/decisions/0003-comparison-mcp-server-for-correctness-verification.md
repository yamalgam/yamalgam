---
status: accepted
date: 2026-03-06
decision-makers: [Clay Loveless]
---

# 0003: Use a comparison MCP server to verify scanner correctness against libfyaml

## Context and Problem Statement

How do we verify that yamalgam's scanner produces correct output? The YAML 1.2 spec has 211 context-sensitive grammar productions, and every pure-Rust parser to date has had compliance gaps. We need a verification mechanism that catches behavioral drift early and continuously.

## Decision Drivers

- libfyaml passes 100% of the YAML Test Suite — it is the behavioral reference
- Manual comparison is impractical across 320+ test cases and growing
- The verification tool should be usable during development, not just in CI
- We have a proven pattern from the pikru project: an MCP server that compares output between a Rust port and its C reference implementation

## Considered Options

- **Option A: Test suite only** — run the YAML Test Suite and compare expected output files
- **Option B: FFI bindings** — link libfyaml directly into Rust tests via `bindgen`/`cc`
- **Option C: Comparison MCP server** — feed identical input to both implementations, diff the token streams, surface results via MCP tools

## Decision Outcome

Chosen option: "Comparison MCP server (Option C)," because it provides both CI-grade automated testing (via the shared comparison library) and interactive development feedback (via MCP tools). The MCP server invokes libfyaml through a small C harness as a subprocess, avoiding FFI binding maintenance. The comparison library is a separate crate usable by both the MCP server and `cargo nextest` tests.

### Consequences

- Good, because comparison happens at the token level — finer-grained than event or document comparison, catching scanner-specific bugs
- Good, because the MCP server provides interactive debugging during development (ad-hoc input, trace filtering)
- Good, because the C harness is ~50-100 lines with no FFI bindings to maintain
- Good, because the comparison library (`yamalgam-compare`) is reusable by CI tests, the MCP server, and future tools
- Bad, because the C harness must be compiled and available on the developer's machine (adds a build dependency on a C compiler)
- Bad, because subprocess invocation is slower than in-process FFI — acceptable for development, but compliance tests will be slower than pure-Rust tests

### Confirmation

The comparison MCP server is operational when it can run `compare_tokens` on any YAML Test Suite case and return a structured `CompareResult`. CI runs the full compliance suite via `just test-compliance`.

## Pros and Cons of the Options

### Option A: Test suite only

Compare yamalgam's output against the YAML Test Suite's expected event/token files.

- Good, because no C dependency at all — pure Rust tests
- Good, because the test suite is the canonical correctness reference
- Bad, because the test suite provides expected *events*, not expected *tokens* — we'd be testing the wrong layer for scanner work
- Bad, because no interactive debugging capability — can only run fixed test cases

### Option B: FFI bindings

Use `bindgen` to generate Rust bindings to libfyaml, call it in-process from tests.

- Good, because fastest comparison (no subprocess overhead)
- Good, because in-process access to libfyaml's full API
- Bad, because `bindgen` bindings are fragile and require maintenance as libfyaml evolves
- Bad, because FFI introduces `unsafe` code into the test infrastructure
- Bad, because the binding complexity is disproportionate to the need — we only need token output

### Option C: Comparison MCP server (chosen)

Small C harness outputs JSON tokens via subprocess. Comparison library diffs token streams. MCP server provides interactive tools.

- Good, because the C harness is trivial to maintain (~50-100 lines, stable libfyaml API)
- Good, because MCP tools enable interactive development workflows (compare ad-hoc input, filter debug traces)
- Good, because clean separation: comparison logic is a library, MCP is a thin transport layer
- Bad, because requires C compiler on developer machine
- Bad, because subprocess invocation adds latency to compliance tests

## More Information

- pikru reference implementation: `~/source/reference/pikru/crates/pikru-mcp/`
- Design doc: [`docs/plans/2026-03-06-scanner-foundation-design.md`](../plans/2026-03-06-scanner-foundation-design.md)
- Related: [ADR-0001](0001-port-libfyaml-state-machine-redesign-data-structures.md) (porting approach)

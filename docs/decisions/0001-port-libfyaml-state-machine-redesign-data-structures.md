---
status: accepted
date: 2026-03-06
decision-makers: [Clay Loveless]
---

# 0001: Port libfyaml's state machine, redesign data structures in idiomatic Rust

## Context and Problem Statement

How should we translate libfyaml's scanner into Rust? The scanner is 242KB of C — a state machine that produces tokens from YAML input. We need behavioral equivalence with libfyaml while producing maintainable, idiomatic Rust code.

## Decision Drivers

- 100% YAML Test Suite compliance from the start — we cannot afford a "close enough" scanner
- The scanner is the foundation for every layer above it (parser, document, CST, linter)
- libfyaml's C-isms (intrusive linked lists, manual refcounting, goto error handling) translate poorly to Rust
- Bus-factor reduction requires traceability between Rust and C implementations

## Considered Options

- **Option A: Faithful line-by-line port** — translate every C function into a Rust function with the same structure
- **Option B: Spec-driven rewrite** — implement from the YAML 1.2 spec, using libfyaml only as a behavioral reference
- **Option C: Hybrid** — port the state machine faithfully, redesign data structures idiomatically

## Decision Outcome

Chosen option: "Hybrid (Option C)," because the state machine *is* the hard part — that's where libfyaml invested years of effort achieving 100% compliance. Porting it faithfully with `// cref:` annotations gives us traceability and high confidence in correctness. The data structures, however, are where C-isms accumulate — intrusive linked lists, raw pointers, manual refcounting — and these have strictly better Rust equivalents.

### Consequences

- Good, because we inherit libfyaml's proven compliance without re-discovering spec ambiguities
- Good, because `// cref:` annotations on state machine blocks create a traceable link for future maintainers
- Good, because idiomatic data structures (`Vec`, `VecDeque`, `Cow`, `bitflags`, enums with data) are safer and more maintainable than C translations
- Bad, because the seam between ported logic and new data structures requires care — behavioral drift is possible where the two meet
- Bad, because we must understand the C code deeply to know which parts are "state machine" and which are "data structure," and the line is not always obvious

### Confirmation

The comparison MCP server (see [ADR-0003](0003-comparison-mcp-server-for-correctness-verification.md)) validates behavioral equivalence by comparing token streams between libfyaml and yamalgam on every YAML Test Suite case.

## Pros and Cons of the Options

### Option A: Faithful line-by-line port

Translate each C function into a Rust function with the same name, parameters, and structure.

- Good, because `// cref:` mapping is trivial — one-to-one correspondence
- Good, because lowest risk of behavioral drift
- Bad, because we'd be writing C in Rust — boolean flag soup, raw-pointer-style indexing, manual state tracking
- Bad, because every layer above the scanner inherits C-shaped types, making the entire codebase harder to maintain

### Option B: Spec-driven rewrite

Implement the scanner from the YAML 1.2 spec's 211 grammar productions, using libfyaml only as a behavioral reference via the comparison tool.

- Good, because the result would be pure idiomatic Rust with no C heritage
- Good, because we might find simpler designs the spec allows
- Bad, because every spec-driven YAML parser in the Rust ecosystem has compliance issues — the spec is ambiguous in practice
- Bad, because we'd be re-discovering problems libfyaml already solved, with no structural guide

### Option C: Hybrid (chosen)

Port the state machine (states, transitions, lookahead) faithfully. Build the surrounding types (tokens, atoms, spans, input handling) from scratch in idiomatic Rust.

- Good, because we get correctness from the state machine and maintainability from the data structures
- Good, because `// cref:` annotations cover behavioral blocks, not just individual functions — more useful for understanding intent
- Neutral, because the porting effort is comparable to Option A (we still read every line of the C)
- Bad, because the definition of "state machine" vs "data structure" is a judgment call at the boundaries

## More Information

- Design doc: [`docs/plans/2026-03-06-scanner-foundation-design.md`](../plans/2026-03-06-scanner-foundation-design.md)
- Idiomatic Rust patterns: [`.claude/skills/idiomatic-rust/SKILL.md`](../../.claude/skills/idiomatic-rust/SKILL.md)
- Code annotation conventions: [`.claude/skills/code-annotations/SKILL.md`](../../.claude/skills/code-annotations/SKILL.md)

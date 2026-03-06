---
status: accepted
date: 2026-03-06
decision-makers: [Clay Loveless]
---

# 0002: Preserve libfyaml's atom concept to enable CST round-trip fidelity

## Context and Problem Statement

libfyaml's `fy_atom` bundles scalar text data with 24 boolean flags about content properties (has line breaks, starts with whitespace, can be output directly, etc.), plus style and chomp metadata. Should we carry this concept into the Rust port, or simplify the scanner's output and recompute metadata when later layers need it?

## Decision Drivers

- yamalgam's CST layer must preserve how scalars were originally expressed (quoting style, block chomp mode) for round-trip editing
- The emitter needs content flags to decide output formatting without re-scanning text
- libfyaml's author built these flags to solve real performance and correctness problems encountered during development
- Simplicity is valuable, but only if it doesn't create rework

## Considered Options

- **Option A: Flatten and defer** — put style/chomp on `Token`, drop content flags, recompute lazily in the emitter
- **Option B: Preserve the atom concept** — `Token` contains an `Atom` that bundles text, span, style, chomp, and content flags

## Decision Outcome

Chosen option: "Preserve the atom concept (Option B)," because the C code already proved these flags are needed. Style and chomp are presentation information required for CST round-trip — discarding them at scan time makes faithful reconstruction impossible. Content flags like `DIRECT_OUTPUT` are performance optimizations the emitter relies on.

The Rust representation uses `bitflags` instead of 24 bare booleans, enums instead of integer constants, and methods instead of flag-checking helper functions. The information content matches libfyaml's `fy_atom`; the representation is idiomatic Rust.

### Consequences

- Good, because the CST layer can reconstruct original formatting from atom metadata without re-parsing input
- Good, because the emitter can use `DIRECT_OUTPUT` and similar flags for zero-analysis output of simple scalars
- Good, because we avoid the "strip now, regret later" pattern — no mid-project scanner output restructuring
- Bad, because the scanner produces richer output than strictly necessary for simple deserialization use cases
- Bad, because we must understand what each of libfyaml's 24 atom flags does to port them correctly

### Confirmation

We verify atom metadata correctness through the comparison MCP server: for each YAML Test Suite case, the token snapshots include style, chomp, and flag values, compared against libfyaml's output.

## Pros and Cons of the Options

### Option A: Flatten and defer

Style and chomp live on `Token` directly. Content flags are dropped entirely, recomputed by the emitter when needed.

- Good, because the scanner's output type is simpler
- Good, because we defer complexity until we know what the emitter actually needs
- Bad, because style and chomp are presentation data — dropping them at scan time is irreversible for CST round-trip
- Bad, because recomputing content flags in the emitter means re-scanning every scalar's content, adding cost that libfyaml specifically engineered away

### Option B: Preserve the atom concept (chosen)

`Token` contains an `Atom<'input>` with text (`Cow<'input, str>`), span, style, chomp, and `AtomFlags` (bitflags).

- Good, because all presentation metadata survives scanning — nothing to recover later
- Good, because `bitflags` is cleaner than 24 booleans and just as efficient
- Bad, because the atom adds conceptual weight to the scanner's output type
- Bad, because some flags may turn out to be unnecessary for Rust-specific emitter designs (we can deprecate individual flags later without restructuring)

## More Information

- libfyaml atom definition: `vendor/libfyaml-0.9.5/src/lib/fy-atom.h`
- Design doc: [`docs/plans/2026-03-06-scanner-foundation-design.md`](../plans/2026-03-06-scanner-foundation-design.md)
- Related: [ADR-0001](0001-port-libfyaml-state-machine-redesign-data-structures.md) (porting approach)

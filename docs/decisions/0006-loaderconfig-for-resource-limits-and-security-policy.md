---
status: accepted
date: 2026-03-07
decision-makers: [Clay Loveless]
---

# 0006: LoaderConfig for resource limits, resolution policy, and security boundaries

## Context and Problem Statement

yamalgam's scanner and parser currently accept arbitrary input with no resource limits. There are no caps on nesting depth, scalar size, document size, or alias expansion count. The planned `!include` and `$ref` features will add filesystem and network access to the processing pipeline. How should we structure the configuration that controls resource consumption and security boundaries across all layers of the YAML loading pipeline?

This is not a single decision about "add a max depth check." It is a taxonomy question: what is the right abstraction for threading security policy through scanner, parser, composer, and resolver — layers that don't all exist yet — without painting ourselves into a corner?

## Decision Drivers

- **No protections exist today.** A malicious input can exhaust memory via deep nesting, giant scalars, or (once the composer exists) recursive alias expansion (billion laughs). This must be addressed before any production use.
- **`!include` and `$ref` expand the threat model.** These features turn yamalgam from a pure parser into something that touches the filesystem and potentially the network — SSRF, path traversal, symlink escapes, and cycle-based DoS all become possible.
- **The YAML specification defines distinct pipeline stages** (parse, compose, construct) with different security surfaces. The configuration model should respect these boundaries rather than conflating them.
- **Secure by default, opt-in to danger.** External resolution must be off by default. Resource limits must have sensible defaults that prevent DoS without rejecting legitimate documents.
- **The configuration must thread cleanly through layers that don't exist yet** (composer, resolver, constructor) without requiring redesign when they arrive.

## Considered Options

### Option A: Flat `SecurityPolicy` struct

A single flat struct with all limits and policies as top-level fields. Every layer receives the same struct and reads the fields it cares about.

- Good, because it is simple to construct and pass around
- Bad, because it conflates unrelated concerns — nesting depth and URL allowlists have nothing to do with each other
- Bad, because the name "SecurityPolicy" mischaracterizes resource limits (robustness, not security) and may discourage use for trusted inputs

### Option B: Per-layer configuration structs

Each layer (scanner, parser, composer, resolver) gets its own config struct. No shared top-level type.

- Good, because each layer's interface is minimal and self-documenting
- Bad, because the caller must construct and thread four separate config objects
- Bad, because cross-cutting concerns (e.g., `max_depth` applies to both scanner flow nesting and parser state stack depth) require duplication or a shared sub-struct, undermining the per-layer separation

### Option C (chosen): `LoaderConfig` with domain sub-structs

A top-level `LoaderConfig` struct composed of `ResourceLimits` and `ResolutionPolicy` sub-structs. Each layer receives `&LoaderConfig` (or a reference to the relevant sub-struct) and reads only what it needs. Named after the YAML specification's "load" operation — the full pipeline from byte stream to native data.

- Good, because the caller constructs one object and threads it everywhere
- Good, because sub-structs group related concerns (DoS prevention vs. external access control)
- Good, because "LoaderConfig" aligns with YAML spec terminology and naturally encompasses the full pipeline
- Bad, because layers receive fields they don't use (scanner sees resolver policy, resolver sees scalar limits)

## Decision Outcome

**Chosen option: Option C** — `LoaderConfig` with `ResourceLimits` and `ResolutionPolicy` sub-structs.

The YAML spec's "load" terminology gives us a name that covers the full pipeline without being scary (`SecurityPolicy`) or vague (`Options`). The two sub-structs separate "how much" (resource limits) from "where can you reach" (resolution policy), which are genuinely orthogonal concerns with different threat models.

### Structure

```rust
/// Controls resource consumption and security boundaries
/// for the yamalgam loading pipeline.
pub struct LoaderConfig {
    /// Hard caps on resource consumption (DoS prevention).
    pub limits: ResourceLimits,
    /// Controls for external reference resolution (!include, $ref).
    pub resolution: ResolutionPolicy,
}

pub struct ResourceLimits {
    // Input layer
    pub max_input_bytes: Option<usize>,

    // Scanner layer
    pub max_scalar_bytes: Option<usize>,
    pub max_key_bytes: Option<usize>,

    // Scanner + Parser (shared concern)
    pub max_depth: Option<usize>,

    // Composer layer (future)
    pub max_alias_expansions: Option<usize>,
    pub max_anchor_count: Option<usize>,
    pub max_merge_depth: Option<usize>,
}

pub struct ResolutionPolicy {
    pub include: IncludePolicy,
    pub refs: RefPolicy,
}

pub struct IncludePolicy {
    pub enabled: bool,                    // false by default
    pub root: Option<PathBuf>,
    pub allow: Vec<GlobPattern>,
    pub deny: Vec<GlobPattern>,
    pub max_depth: usize,
    pub max_total_bytes: Option<usize>,
    pub follow_symlinks: bool,            // false by default
}

pub struct RefPolicy {
    pub enabled: bool,                    // false by default
    pub allow_schemes: Vec<String>,
    pub allow_hosts: Vec<String>,
    pub timeout: Duration,
}
```

### Presets

```rust
impl LoaderConfig {
    /// Trusted local files: generous limits, no external resolution.
    pub fn trusted() -> Self { ... }

    /// Untrusted input: strict limits, no external resolution.
    pub fn strict() -> Self { ... }

    /// Default: moderate limits, no external resolution.
    /// Resolution features are always opt-in.
    pub fn default() -> Self { ... }
}
```

### Threading

Each layer takes `&LoaderConfig` and reads the sub-struct it needs:

| Layer | Reads | Enforces |
|-------|-------|----------|
| `Input` | `limits.max_input_bytes` | Caps `from_reader()` buffer growth |
| `Scanner` | `limits.max_scalar_bytes`, `max_key_bytes`, `max_depth` | Rejects oversized scalars, caps `flow_level` |
| `Parser` | `limits.max_depth` | Caps `state_stack` depth |
| `Composer` | `limits.max_alias_expansions`, `max_anchor_count`, `max_merge_depth` | Billion laughs protection, merge key recursion |
| `Resolver` | `resolution` | Path sandboxing, URL policy, cycle detection |

Existing constructors (`Scanner::new()`, `Parser::new()`) continue to work with `LoaderConfig::default()`. New `::with_config()` constructors accept explicit configuration.

### What is NOT included (yet)

**`TagPolicy`** is deliberately omitted. yamalgam does not construct native objects from tags — it passes tag strings through as data. The PyYAML `!!python/object` arbitrary-code-execution class of vulnerability only applies when tags trigger type construction. When we build a constructor layer, `TagPolicy` gets added to `LoaderConfig`. Not before.

## Consequences

### Positive

- Every layer of the pipeline has enforceable resource limits from day one
- `!include` and `$ref` are off by default — users must explicitly opt in with a policy
- Preset constructors (`trusted()`, `strict()`) make the common cases trivial
- The struct is extensible — adding `TagPolicy` or new limit fields is additive, not breaking
- Aligns with YAML spec terminology, which helps when explaining the architecture

### Negative

- Layers receive configuration they don't use (scanner sees `ResolutionPolicy`, resolver sees `max_scalar_bytes`). We accept this cost in exchange for a single config object that callers don't have to decompose.
- `Option<usize>` for limits means `None` = unlimited. Callers who forget to set limits on untrusted input get no protection unless they use a preset. Mitigated by documentation and by making `LoaderConfig::default()` include moderate limits.
- The `ResolutionPolicy` sub-structs are defined now but won't be enforced until the resolver layer exists. Early definition is intentional — it locks the API shape before implementation.

### Neutral

- `LoaderConfig` lives in `yamalgam-core` since it is shared across all crates in the workspace
- Fuzzing targets (ADR forthcoming) will test resource limit enforcement explicitly — crafted inputs that attempt to exceed each limit

## More Information

- Security audit findings that motivated this decision: conversation record, 2026-03-07
- YAML specification load/dump terminology: [YAML 1.2.2 Section 3.1](https://yaml.org/spec/1.2.2/#31-processes)
- Billion laughs attack: [Wikipedia](https://en.wikipedia.org/wiki/Billion_laughs_attack)
- Related: [ADR-0001](0001-port-libfyaml-state-machine-redesign-data-structures.md) (scanner architecture), [ADR-0005](0005-parser-event-model-with-inline-directives.md) (parser event model)
- Upstream discussion: `ref/architecture-discussion.md` (serde/facet/CST architecture, resolver security surface)

# Tag resolution trait — design

**Date:** 2026-03-09
**Status:** Draft
**Depends on:** ADR-0007 (resolver trait), M6 (Value DOM + Composer)

## Problem

Plain scalar resolution is hardcoded to YAML 1.2 Core Schema in
`yamalgam-core::tag::resolve_plain_scalar()`. The Composer calls it
unconditionally for every unquoted scalar. There's no way to use a
different tag resolution scheme — Failsafe, JSON, YAML 1.1 — without
forking the code.

This matters because:

1. **`yg --schema yaml11`** needs YAML 1.1 boolean resolution (`yes`/`no`/`on`/`off`).
2. **Failsafe mode** (all scalars are strings) is required for lossless round-trip editing.
3. **JSON-strict mode** rejects `0o`/`0x` integers and `.inf`/`.nan` floats.
4. **Custom schemas** (e.g., strict typing for CI config) need the same hook.

The YAML spec calls these "schemas" (Section 10), but that term collides
with validation schemas (JSON Schema, M12). The YAML spec operation is
**tag resolution** — how untagged plain scalars get implicitly typed. That's
what we're making pluggable.

## Design

### TagResolver trait

```rust
/// Tag resolution for untagged plain scalars.
///
/// Determines how plain (unquoted) scalar strings are typed when composed
/// into `Value`. Quoted scalars are always strings regardless of schema.
///
/// Named after the YAML spec operation "tag resolution" (§10) to avoid
/// collision with schema validation (JSON Schema, etc.).
pub trait TagResolver {
    /// Resolve a plain scalar string to a typed Value.
    fn resolve_scalar(&self, value: &str) -> Value;
}
```

Lives in `yamalgam-core::tag_resolution`. Object-safe, stateless by
convention (all built-in implementations are ZSTs). One vtable dispatch
per plain scalar — negligible since the Composer is already allocating
`Value` nodes.

### TagResolution enum

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TagResolution {
    /// YAML 1.2 Failsafe Schema — all scalars are strings.
    Failsafe,
    /// YAML 1.2 JSON Schema — strict null/bool/int/float, no octal/hex/inf/nan.
    Json,
    /// YAML 1.2 Core Schema (recommended default).
    #[default]
    Yaml12,
    /// YAML 1.1 type resolution — extended booleans, 0-prefix octal, binary ints.
    Yaml11,
}
```

`TagResolution` is `Copy`, so `LoaderConfig` stays `Clone`. The enum itself
implements `TagResolver` by dispatching to the right ZST implementation.

### Built-in implementations

Each is a unit struct implementing `TagResolver`.

**`Yaml12TagResolver`** — delegates to the existing `resolve_plain_scalar()`
in `tag.rs`. No code moves, no Tracey annotation churn.

**`FailsafeTagResolver`** — trivial: always returns `Value::String`.

**`JsonTagResolver`** — YAML 1.2 JSON Schema (§10.2):
- Null: `null` only (not `~`, not `Null`/`NULL`, not empty)
- Bool: `true`/`false` only (not `True`/`FALSE`)
- Int: `[+-]?[0-9]+` only — no `0x`, no `0o`
- Float: decimal with `.` or exponent only — no `.inf`, no `.nan`
- Fallback: string (lenient — strict error mode is a future option)

**`Yaml11TagResolver`** — YAML 1.1 type resolution:
- Null: Core set plus empty string
- Bool: Core set **plus** `yes`/`Yes`/`YES`, `no`/`No`/`NO`, `on`/`On`/`ON`,
  `off`/`Off`/`OFF`, `y`/`Y`, `n`/`N`
- Int: decimal, `0`-prefix octal (not `0o`), `0x` hex, `0b` binary,
  underscore separators in all bases
- Float: underscore separators, `.inf`/`.nan` case variants (same as Core)
- Fallback: string

YAML 1.1 sexagesimal integers (`1:30:00`) and floats (`1:30:00.5`) are
**not** implemented — they're obscure, and no major library still supports
them. If someone actually needs them, they can implement `TagResolver`.

### LoaderConfig integration

```rust
pub struct LoaderConfig {
    pub limits: ResourceLimits,
    pub resolution: ResolutionPolicy,
    pub tag_resolution: TagResolution,   // NEW — default: Yaml12
}
```

Presets unchanged — all four default to `TagResolution::Yaml12`.
Builder method: `with_tag_resolution(TagResolution) -> Self`.

### Composer changes

Add `tag_resolver: Box<dyn TagResolver>` to `Composer`:

```rust
pub struct Composer<'input, I> {
    events: Peekable<I>,
    anchors: HashMap<String, Value>,
    config: ResourceLimits,
    alias_expansion_count: usize,
    tag_resolver: Box<dyn TagResolver>,  // NEW
}
```

The free function `resolve_scalar()` becomes a method:

```rust
fn resolve_scalar(&self, value: &str, style: ScalarStyle) -> Value {
    match style {
        ScalarStyle::Plain => self.tag_resolver.resolve_scalar(value),
        _ => Value::String(value.to_owned()),
    }
}
```

Constructor changes:
- `new()` — defaults to `Box::new(Yaml12TagResolver)`
- `new_with_config()` — uses `config.tag_resolution` to create the right resolver
- `with_tag_resolver(resolver: impl TagResolver + 'static)` — builder for custom resolvers

Convenience APIs:
- `from_str()` / `from_str_single()` — unchanged (YAML 1.2 Core default)
- `from_str_with_config()` / `from_str_single_with_config()` — uses config's `tag_resolution`
- `from_str_with_tag_resolver()` — custom resolver, no config

### Relationship to the Resolver trait (event middleware)

These are **different layers** serving different purposes:

| Layer | Trait | Where | What it does |
|-------|-------|-------|-------------|
| Event middleware | `Resolver<'input>` | Between parser and composer | Transforms event stream (`!include`, `$ref`, custom tags) |
| Tag resolution | `TagResolver` | Inside composer | Types plain scalars (null, bool, int, float, string) |

They don't interact. The event `Resolver` operates on events before the
Composer sees them. `TagResolver` operates on scalar values during Value
composition. A user can configure both independently.

## File changes

| File | Change |
|------|--------|
| `yamalgam-core/src/tag_resolution.rs` | **New** — trait, enum, `FailsafeTagResolver`, `JsonTagResolver`, `Yaml11TagResolver` |
| `yamalgam-core/src/tag.rs` | Add `Yaml12TagResolver` struct + `TagResolver` impl (delegates to existing function) |
| `yamalgam-core/src/loader.rs` | Add `tag_resolution: TagResolution` to `LoaderConfig` + `with_tag_resolution()` |
| `yamalgam-core/src/lib.rs` | `pub mod tag_resolution`, re-exports |
| `yamalgam-parser/src/compose.rs` | Composer stores `Box<dyn TagResolver>`, uses it |
| `yamalgam-parser/src/lib.rs` | Update convenience API re-exports if needed |

## Testing

**Schema matrix** — same scalar through all 4 resolvers:

| Scalar | Failsafe | Json | Yaml12 | Yaml11 |
|--------|----------|------|--------|--------|
| `"yes"` | String | String | String | Bool(true) |
| `"null"` | String | Null | Null | Null |
| `"Null"` | String | String | Null | Null |
| `"~"` | String | String | Null | Null |
| `""` (empty) | String | String | Null | Null |
| `"true"` | String | Bool(true) | Bool(true) | Bool(true) |
| `"True"` | String | String | Bool(true) | Bool(true) |
| `"0o17"` | String | String | Integer(15) | String |
| `"017"` | String | String | Integer(17) | Integer(15) |
| `"0b1010"` | String | String | String | Integer(10) |
| `"1_000"` | String | String | String | Integer(1000) |
| `"0xFF"` | String | String | Integer(255) | Integer(255) |
| `".inf"` | String | String | Float(inf) | Float(inf) |
| `".nan"` | String | String | Float(NaN) | Float(NaN) |
| `"42"` | String | Integer(42) | Integer(42) | Integer(42) |
| `"3.14"` | String | Float(3.14) | Float(3.14) | Float(3.14) |

Plus:
- Composer integration tests with each tag resolution
- Existing tests pass unchanged (Yaml12 is default)
- Custom TagResolver impl test (proves extensibility)

## What this does NOT cover

- Tag resolution for **explicitly tagged** scalars (`!!int "123"`) — the Composer
  handles those separately, outside the `TagResolver` trait
- YAML 1.1 sexagesimal values — intentionally excluded
- Strict JSON schema error mode (rejecting unresolvable scalars) — future option
- `yg --schema` CLI flag — that's M8

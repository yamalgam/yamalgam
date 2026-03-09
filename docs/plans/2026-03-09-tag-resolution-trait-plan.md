# Tag Resolution Trait Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make plain scalar type resolution pluggable via a `TagResolver` trait with four built-in implementations (Yaml12, Failsafe, Json, Yaml11).

**Architecture:** `TagResolver` trait + `TagResolution` enum in `yamalgam-core`. Composer stores `Box<dyn TagResolver>` and delegates scalar typing through it. `LoaderConfig` gains a `tag_resolution` field (enum, Copy) so config-based construction picks the right resolver automatically.

**Tech Stack:** Pure Rust, no new dependencies. All implementations in `yamalgam-core`, wiring in `yamalgam-parser`.

**Skills:** @idiomatic-rust @code-annotations

---

### Task 1: TagResolver trait + FailsafeTagResolver

**Files:**
- Create: `crates/yamalgam-core/src/tag_resolution.rs`
- Modify: `crates/yamalgam-core/src/lib.rs`

**Step 1: Write the failing tests**

Add to bottom of `crates/yamalgam-core/src/tag_resolution.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    #[test]
    fn failsafe_always_returns_string() {
        let r = FailsafeTagResolver;
        assert_eq!(r.resolve_scalar("null"), Value::String("null".into()));
        assert_eq!(r.resolve_scalar("true"), Value::String("true".into()));
        assert_eq!(r.resolve_scalar("42"), Value::String("42".into()));
        assert_eq!(r.resolve_scalar("3.14"), Value::String("3.14".into()));
        assert_eq!(r.resolve_scalar("hello"), Value::String("hello".into()));
        assert_eq!(r.resolve_scalar(""), Value::String(String::new()));
        assert_eq!(r.resolve_scalar("~"), Value::String("~".into()));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo nextest run -p yamalgam-core --test-threads=1 -E 'test(failsafe_always_returns_string)'`
Expected: Compile error — `TagResolver` and `FailsafeTagResolver` don't exist.

**Step 3: Write minimal implementation**

In `crates/yamalgam-core/src/tag_resolution.rs`:

```rust
//! Pluggable tag resolution for plain scalars.
//!
//! The YAML spec (Section 10) defines "schemas" that determine how untagged
//! plain scalars are implicitly typed. This module calls the operation "tag
//! resolution" to avoid confusion with schema validation (JSON Schema, etc.).
//!
//! Four built-in implementations cover the spec-defined schemas plus YAML 1.1
//! legacy behavior. Users can implement [`TagResolver`] for custom typing rules.

use crate::Value;

/// Resolves untagged plain scalars to typed [`Value`]s.
///
/// Only plain (unquoted) scalars undergo tag resolution. Quoted scalars are
/// always strings — the Composer handles that distinction before calling
/// this trait.
///
/// Named after the YAML spec's "tag resolution" operation (§10) to avoid
/// collision with schema validation (JSON Schema, etc.).
pub trait TagResolver {
    /// Resolve a plain scalar string to a typed Value.
    fn resolve_scalar(&self, value: &str) -> Value;
}

/// Tag resolution scheme selector.
///
/// Used in [`LoaderConfig`](crate::LoaderConfig) to select a built-in tag
/// resolution scheme. Implements [`TagResolver`] by dispatching to the
/// corresponding implementation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TagResolution {
    /// YAML 1.2 Failsafe Schema — all scalars are strings.
    Failsafe,
    /// YAML 1.2 JSON Schema — strict null/bool/int/float.
    Json,
    /// YAML 1.2 Core Schema (recommended default).
    #[default]
    Yaml12,
    /// YAML 1.1 type resolution — extended booleans, legacy octal, binary.
    Yaml11,
}

/// YAML 1.2 Failsafe Schema — all plain scalars are strings.
///
/// The simplest schema: no implicit typing at all. Every plain scalar
/// becomes `Value::String`. Use this for lossless round-trip processing
/// where you don't want type coercion.
// y[impl schema.failsafe.tag-str+3]
#[derive(Debug, Clone, Copy, Default)]
pub struct FailsafeTagResolver;

impl TagResolver for FailsafeTagResolver {
    fn resolve_scalar(&self, value: &str) -> Value {
        Value::String(value.to_owned())
    }
}
```

In `crates/yamalgam-core/src/lib.rs`, add the module and re-exports:

```rust
pub mod tag_resolution;
```

And add re-exports:

```rust
pub use tag_resolution::{FailsafeTagResolver, TagResolution, TagResolver};
```

**Step 4: Run test to verify it passes**

Run: `cargo nextest run -p yamalgam-core -E 'test(failsafe_always_returns_string)'`
Expected: PASS

**Step 5: Commit**

```
feat(core): add TagResolver trait and FailsafeTagResolver

Introduces pluggable tag resolution for plain scalars. The TagResolver
trait and TagResolution enum replace the hardcoded YAML 1.2 Core
resolution, allowing Failsafe, JSON, Yaml11, and custom schemas.

Starts with FailsafeTagResolver — all scalars are strings.
```

---

### Task 2: Yaml12TagResolver

**Files:**
- Modify: `crates/yamalgam-core/src/tag.rs`
- Modify: `crates/yamalgam-core/src/tag_resolution.rs` (tests)
- Modify: `crates/yamalgam-core/src/lib.rs` (re-export)

**Step 1: Write the failing test**

Add to tests in `crates/yamalgam-core/src/tag_resolution.rs`:

```rust
    #[test]
    fn yaml12_matches_resolve_plain_scalar() {
        use crate::resolve_plain_scalar;
        let r = Yaml12TagResolver;
        let cases = [
            "null", "Null", "NULL", "~", "",
            "true", "True", "TRUE", "false", "False", "FALSE",
            "42", "-17", "+99", "0o17", "0xFF",
            "1.0", "-0.5", "1e10", ".inf", "-.inf", ".nan",
            "hello", "yes", "no", "on", "off",
        ];
        for s in cases {
            assert_eq!(
                r.resolve_scalar(s),
                resolve_plain_scalar(s),
                "Yaml12TagResolver disagrees with resolve_plain_scalar for {s:?}"
            );
        }
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo nextest run -p yamalgam-core -E 'test(yaml12_matches)'`
Expected: Compile error — `Yaml12TagResolver` doesn't exist.

**Step 3: Write minimal implementation**

In `crates/yamalgam-core/src/tag.rs`, add after the existing imports:

```rust
use crate::tag_resolution::TagResolver;
```

Then add at the end of the file (before tests):

```rust
/// YAML 1.2 Core Schema tag resolver.
///
/// Delegates to [`resolve_plain_scalar()`] — the existing YAML 1.2 Core
/// implementation. This struct exists to provide a [`TagResolver`] impl
/// without moving or duplicating the resolution logic.
// y[impl schema.core.recommended-default+3]
// y[impl schema.core.tag-resolution-scalars+3]
#[derive(Debug, Clone, Copy, Default)]
pub struct Yaml12TagResolver;

impl TagResolver for Yaml12TagResolver {
    fn resolve_scalar(&self, value: &str) -> Value {
        resolve_plain_scalar(value)
    }
}
```

In `crates/yamalgam-core/src/tag_resolution.rs`, add the re-export so tests can find it:

```rust
pub use crate::tag::Yaml12TagResolver;
```

In `crates/yamalgam-core/src/lib.rs`, add to re-exports:

```rust
pub use tag::Yaml12TagResolver;
```

**Step 4: Run test to verify it passes**

Run: `cargo nextest run -p yamalgam-core -E 'test(yaml12_matches)'`
Expected: PASS

**Step 5: Commit**

```
feat(core): add Yaml12TagResolver delegating to resolve_plain_scalar
```

---

### Task 3: JsonTagResolver

**Files:**
- Modify: `crates/yamalgam-core/src/tag_resolution.rs`

**Step 1: Write the failing tests**

Add to tests in `tag_resolution.rs`:

```rust
    #[test]
    fn json_null() {
        let r = JsonTagResolver;
        assert_eq!(r.resolve_scalar("null"), Value::Null);
        // Core nulls that JSON rejects:
        assert_eq!(r.resolve_scalar("Null"), Value::String("Null".into()));
        assert_eq!(r.resolve_scalar("NULL"), Value::String("NULL".into()));
        assert_eq!(r.resolve_scalar("~"), Value::String("~".into()));
        assert_eq!(r.resolve_scalar(""), Value::String(String::new()));
    }

    #[test]
    fn json_bool() {
        let r = JsonTagResolver;
        assert_eq!(r.resolve_scalar("true"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("false"), Value::Bool(false));
        // Core bools that JSON rejects:
        assert_eq!(r.resolve_scalar("True"), Value::String("True".into()));
        assert_eq!(r.resolve_scalar("FALSE"), Value::String("FALSE".into()));
    }

    #[test]
    fn json_integer() {
        let r = JsonTagResolver;
        assert_eq!(r.resolve_scalar("0"), Value::Integer(0));
        assert_eq!(r.resolve_scalar("42"), Value::Integer(42));
        assert_eq!(r.resolve_scalar("-17"), Value::Integer(-17));
        // No octal or hex in JSON:
        assert_eq!(r.resolve_scalar("0o17"), Value::String("0o17".into()));
        assert_eq!(r.resolve_scalar("0xFF"), Value::String("0xFF".into()));
        // No leading + in JSON:
        assert_eq!(r.resolve_scalar("+42"), Value::String("+42".into()));
    }

    #[test]
    fn json_float() {
        let r = JsonTagResolver;
        assert_eq!(r.resolve_scalar("1.0"), Value::Float(1.0));
        assert_eq!(r.resolve_scalar("-0.5"), Value::Float(-0.5));
        assert_eq!(r.resolve_scalar("1e10"), Value::Float(1e10));
        assert_eq!(r.resolve_scalar("1.5e-3"), Value::Float(1.5e-3));
        // No .inf/.nan in JSON:
        assert_eq!(r.resolve_scalar(".inf"), Value::String(".inf".into()));
        assert_eq!(r.resolve_scalar(".nan"), Value::String(".nan".into()));
        assert_eq!(r.resolve_scalar(".Inf"), Value::String(".Inf".into()));
        // No leading + on floats:
        assert_eq!(r.resolve_scalar("+1.0"), Value::String("+1.0".into()));
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo nextest run -p yamalgam-core -E 'test(json_)'`
Expected: Compile error — `JsonTagResolver` doesn't exist.

**Step 3: Write implementation**

In `tag_resolution.rs`, add:

```rust
/// YAML 1.2 JSON Schema — strict JSON-compatible type resolution.
///
/// Only recognizes the JSON subset of YAML scalars:
/// - Null: `null` only (not `~`, `Null`, `NULL`, or empty)
/// - Bool: `true`/`false` only (case-sensitive)
/// - Int: `[0-9]+` or `-[0-9]+` (no `+` sign, no `0x`, no `0o`)
/// - Float: decimal with `.` or exponent (no `.inf`, `.nan`, no `+` sign)
/// - Fallback: string
// y[impl schema.json.tag-null+3]
// y[impl schema.json.tag-bool+3]
// y[impl schema.json.tag-int+3]
// y[impl schema.json.tag-float+3]
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonTagResolver;

impl TagResolver for JsonTagResolver {
    fn resolve_scalar(&self, value: &str) -> Value {
        // Null: only "null"
        if value == "null" {
            return Value::Null;
        }

        // Bool: only "true" / "false" (case-sensitive)
        if value == "true" {
            return Value::Bool(true);
        }
        if value == "false" {
            return Value::Bool(false);
        }

        // Try numeric — JSON numbers have no leading +, no 0x/0o, no .inf/.nan
        if let Some(v) = try_json_number(value) {
            return v;
        }

        Value::String(value.to_owned())
    }
}

/// Try parsing as a JSON-compatible number.
///
/// JSON numbers: optional `-`, digits, optional `.` + digits, optional `e`/`E` exponent.
/// No leading `+`, no `0x`/`0o`, no `.inf`/`.nan`.
fn try_json_number(s: &str) -> Option<Value> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    // Optional leading minus (no plus allowed)
    let body = if bytes[0] == b'-' { &s[1..] } else { s };

    if body.is_empty() || !body.as_bytes()[0].is_ascii_digit() {
        return None;
    }

    // Check for float indicators
    let is_float = body.contains('.') || body.contains('e') || body.contains('E');

    if is_float {
        // Validate: only digits, '.', 'e', 'E', '+', '-' (in exponent)
        s.parse::<f64>().ok().map(Value::Float)
    } else {
        // Pure decimal integer — all remaining chars must be digits
        if !body.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        s.parse::<i64>().ok().map(Value::Integer)
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p yamalgam-core -E 'test(json_)'`
Expected: PASS

**Step 5: Commit**

```
feat(core): add JsonTagResolver for YAML 1.2 JSON Schema
```

---

### Task 4: Yaml11TagResolver

**Files:**
- Modify: `crates/yamalgam-core/src/tag_resolution.rs`

**Step 1: Write the failing tests**

Add to tests in `tag_resolution.rs`:

```rust
    #[test]
    fn yaml11_booleans() {
        let r = Yaml11TagResolver;
        // Core booleans still work:
        assert_eq!(r.resolve_scalar("true"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("True"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("TRUE"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("false"), Value::Bool(false));
        assert_eq!(r.resolve_scalar("False"), Value::Bool(false));
        assert_eq!(r.resolve_scalar("FALSE"), Value::Bool(false));
        // Extended 1.1 booleans:
        assert_eq!(r.resolve_scalar("yes"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("Yes"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("YES"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("no"), Value::Bool(false));
        assert_eq!(r.resolve_scalar("No"), Value::Bool(false));
        assert_eq!(r.resolve_scalar("NO"), Value::Bool(false));
        assert_eq!(r.resolve_scalar("on"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("On"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("ON"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("off"), Value::Bool(false));
        assert_eq!(r.resolve_scalar("Off"), Value::Bool(false));
        assert_eq!(r.resolve_scalar("OFF"), Value::Bool(false));
        assert_eq!(r.resolve_scalar("y"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("Y"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("n"), Value::Bool(false));
        assert_eq!(r.resolve_scalar("N"), Value::Bool(false));
    }

    #[test]
    fn yaml11_null() {
        let r = Yaml11TagResolver;
        assert_eq!(r.resolve_scalar("null"), Value::Null);
        assert_eq!(r.resolve_scalar("Null"), Value::Null);
        assert_eq!(r.resolve_scalar("NULL"), Value::Null);
        assert_eq!(r.resolve_scalar("~"), Value::Null);
        assert_eq!(r.resolve_scalar(""), Value::Null);
    }

    #[test]
    fn yaml11_octal() {
        let r = Yaml11TagResolver;
        // 1.1 octal: 0-prefix (not 0o)
        assert_eq!(r.resolve_scalar("017"), Value::Integer(15));
        assert_eq!(r.resolve_scalar("0"), Value::Integer(0));
        assert_eq!(r.resolve_scalar("010"), Value::Integer(8));
        // 0o is NOT 1.1 octal — it's a plain string in 1.1
        assert_eq!(r.resolve_scalar("0o17"), Value::String("0o17".into()));
    }

    #[test]
    fn yaml11_binary() {
        let r = Yaml11TagResolver;
        assert_eq!(r.resolve_scalar("0b1010"), Value::Integer(10));
        assert_eq!(r.resolve_scalar("0b0"), Value::Integer(0));
        assert_eq!(r.resolve_scalar("0b11111111"), Value::Integer(255));
    }

    #[test]
    fn yaml11_hex() {
        let r = Yaml11TagResolver;
        assert_eq!(r.resolve_scalar("0xFF"), Value::Integer(255));
        assert_eq!(r.resolve_scalar("0x0"), Value::Integer(0));
        assert_eq!(r.resolve_scalar("0xDEAD"), Value::Integer(0xDEAD));
    }

    #[test]
    fn yaml11_underscores_in_numbers() {
        let r = Yaml11TagResolver;
        assert_eq!(r.resolve_scalar("1_000"), Value::Integer(1000));
        assert_eq!(r.resolve_scalar("1_000_000"), Value::Integer(1_000_000));
        assert_eq!(r.resolve_scalar("0xFF_FF"), Value::Integer(0xFFFF));
        assert_eq!(r.resolve_scalar("1_0.5"), Value::Float(10.5));
    }

    #[test]
    fn yaml11_special_floats() {
        let r = Yaml11TagResolver;
        assert_eq!(r.resolve_scalar(".inf"), Value::Float(f64::INFINITY));
        assert_eq!(r.resolve_scalar(".Inf"), Value::Float(f64::INFINITY));
        assert_eq!(r.resolve_scalar("-.inf"), Value::Float(f64::NEG_INFINITY));
        match r.resolve_scalar(".nan") {
            Value::Float(f) => assert!(f.is_nan()),
            other => panic!("expected Float(NaN), got {other:?}"),
        }
    }

    #[test]
    fn yaml11_signed_integers() {
        let r = Yaml11TagResolver;
        assert_eq!(r.resolve_scalar("+42"), Value::Integer(42));
        assert_eq!(r.resolve_scalar("-17"), Value::Integer(-17));
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo nextest run -p yamalgam-core -E 'test(yaml11_)'`
Expected: Compile error — `Yaml11TagResolver` doesn't exist.

**Step 3: Write implementation**

In `tag_resolution.rs`, add:

```rust
/// YAML 1.1 tag resolution — extended booleans, legacy octal, binary integers.
///
/// Recognizes all YAML 1.2 Core scalars plus:
/// - Bool: `yes`/`no`, `on`/`off`, `y`/`n` (all case variants)
/// - Int: `0`-prefix octal (`017` = 15), `0b` binary, underscore separators
/// - Float: underscore separators
///
/// Sexagesimal values (`1:30:00`) are intentionally not supported.
#[derive(Debug, Clone, Copy, Default)]
pub struct Yaml11TagResolver;

impl TagResolver for Yaml11TagResolver {
    fn resolve_scalar(&self, value: &str) -> Value {
        // Null
        if value.is_empty() || value == "~" || value == "null" || value == "Null" || value == "NULL"
        {
            return Value::Null;
        }

        // Booleans (YAML 1.1 extended set)
        match value {
            "true" | "True" | "TRUE" | "yes" | "Yes" | "YES" | "on" | "On" | "ON" | "y" | "Y" => {
                return Value::Bool(true);
            }
            "false" | "False" | "FALSE" | "no" | "No" | "NO" | "off" | "Off" | "OFF" | "n"
            | "N" => {
                return Value::Bool(false);
            }
            _ => {}
        }

        // Special floats (same as Core)
        match value {
            ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => {
                return Value::Float(f64::INFINITY);
            }
            "-.inf" | "-.Inf" | "-.INF" => return Value::Float(f64::NEG_INFINITY),
            ".nan" | ".NaN" | ".NAN" => return Value::Float(f64::NAN),
            _ => {}
        }

        // Try numeric (with underscore stripping and 1.1 octal/binary)
        if let Some(v) = try_yaml11_number(value) {
            return v;
        }

        Value::String(value.to_owned())
    }
}

/// Try parsing as a YAML 1.1 number (supports underscores, 0-prefix octal, 0b binary).
fn try_yaml11_number(s: &str) -> Option<Value> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    // Strip underscores for parsing
    let stripped: String;
    let clean = if s.contains('_') {
        stripped = s.replace('_', "");
        stripped.as_str()
    } else {
        s
    };

    if clean.is_empty() {
        return None;
    }

    let clean_bytes = clean.as_bytes();

    // Determine sign and numeric body
    let (negative, body) = match clean_bytes[0] {
        b'+' => (false, &clean[1..]),
        b'-' => (true, &clean[1..]),
        _ => (false, clean),
    };

    if body.is_empty() {
        return None;
    }

    // Binary: 0b...
    if let Some(bin) = body.strip_prefix("0b") {
        if bin.is_empty() || !bin.bytes().all(|b| b == b'0' || b == b'1') {
            return None;
        }
        let n = i64::from_str_radix(bin, 2).ok()?;
        return Some(Value::Integer(if negative { -n } else { n }));
    }

    // Hex: 0x...
    if let Some(hex) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let n = i64::from_str_radix(hex, 16).ok()?;
        return Some(Value::Integer(if negative { -n } else { n }));
    }

    // Check for float indicators
    let is_float = body.contains('.') || body.contains('e') || body.contains('E');

    if is_float {
        clean.parse::<f64>().ok().map(Value::Float)
    } else if body.bytes().all(|b| b.is_ascii_digit()) {
        // 1.1 octal: starts with 0 and has more digits
        if body.len() > 1 && body.starts_with('0') {
            // Octal (0-prefix)
            let n = i64::from_str_radix(body, 8).ok()?;
            Some(Value::Integer(if negative { -n } else { n }))
        } else {
            // Decimal
            let n: i64 = body.parse().ok()?;
            Some(Value::Integer(if negative { -n } else { n }))
        }
    } else {
        None
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p yamalgam-core -E 'test(yaml11_)'`
Expected: PASS

**Step 5: Commit**

```
feat(core): add Yaml11TagResolver with extended booleans and legacy octal
```

---

### Task 5: TagResolution enum dispatch + re-exports

**Files:**
- Modify: `crates/yamalgam-core/src/tag_resolution.rs`
- Modify: `crates/yamalgam-core/src/lib.rs`

**Step 1: Write the failing test**

Add to tests in `tag_resolution.rs`:

```rust
    #[test]
    fn tag_resolution_enum_dispatches_correctly() {
        // Yaml12 default
        let r = TagResolution::default();
        assert_eq!(r, TagResolution::Yaml12);
        assert_eq!(r.resolve_scalar("true"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("yes"), Value::String("yes".into()));

        // Failsafe
        let r = TagResolution::Failsafe;
        assert_eq!(r.resolve_scalar("true"), Value::String("true".into()));
        assert_eq!(r.resolve_scalar("42"), Value::String("42".into()));

        // Json
        let r = TagResolution::Json;
        assert_eq!(r.resolve_scalar("true"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("True"), Value::String("True".into()));

        // Yaml11
        let r = TagResolution::Yaml11;
        assert_eq!(r.resolve_scalar("yes"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("017"), Value::Integer(15));
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo nextest run -p yamalgam-core -E 'test(tag_resolution_enum_dispatches)'`
Expected: Compile error — `TagResolution` doesn't implement `TagResolver`.

**Step 3: Write implementation**

In `tag_resolution.rs`, add the `TagResolver` impl for `TagResolution`:

```rust
impl TagResolver for TagResolution {
    fn resolve_scalar(&self, value: &str) -> Value {
        match self {
            Self::Failsafe => FailsafeTagResolver.resolve_scalar(value),
            Self::Json => JsonTagResolver.resolve_scalar(value),
            Self::Yaml12 => Yaml12TagResolver.resolve_scalar(value),
            Self::Yaml11 => Yaml11TagResolver.resolve_scalar(value),
        }
    }
}
```

Update `crates/yamalgam-core/src/lib.rs` re-exports to include all types:

```rust
pub use tag_resolution::{
    FailsafeTagResolver, JsonTagResolver, TagResolution, TagResolver, Yaml11TagResolver,
};
```

**Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p yamalgam-core -E 'test(tag_resolution)'`
Expected: All tag_resolution tests PASS.

**Step 5: Run full core tests**

Run: `cargo nextest run -p yamalgam-core`
Expected: All tests PASS (existing tag.rs tests unaffected).

**Step 6: Commit**

```
feat(core): implement TagResolver dispatch for TagResolution enum
```

---

### Task 6: LoaderConfig integration

**Files:**
- Modify: `crates/yamalgam-core/src/loader.rs`

**Step 1: Write the failing test**

Add to tests in `loader.rs`:

```rust
    #[test]
    fn default_tag_resolution_is_yaml12() {
        let cfg = LoaderConfig::default();
        assert_eq!(cfg.tag_resolution, TagResolution::Yaml12);
    }

    #[test]
    fn with_tag_resolution_builder() {
        let cfg = LoaderConfig::strict().with_tag_resolution(TagResolution::Yaml11);
        assert_eq!(cfg.tag_resolution, TagResolution::Yaml11);
        // Limits unchanged:
        assert_eq!(cfg.limits.max_depth, Some(64));
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo nextest run -p yamalgam-core -E 'test(tag_resolution)'`
Expected: Compile error — `tag_resolution` field doesn't exist on `LoaderConfig`.

**Step 3: Write implementation**

In `loader.rs`:

1. Add import at top:

```rust
use crate::tag_resolution::TagResolution;
```

2. Add field to `LoaderConfig`:

```rust
pub struct LoaderConfig {
    pub limits: ResourceLimits,
    pub resolution: ResolutionPolicy,
    /// Tag resolution scheme for plain scalar typing.
    pub tag_resolution: TagResolution,
}
```

3. Update all four preset constructors to include the field:

```rust
    tag_resolution: TagResolution::Yaml12,
```

4. Add builder method to `impl LoaderConfig`:

```rust
    /// Set the tag resolution scheme.
    #[must_use]
    pub const fn with_tag_resolution(mut self, tag_resolution: TagResolution) -> Self {
        self.tag_resolution = tag_resolution;
        self
    }
```

5. Fix any existing tests that construct `LoaderConfig` with struct literals — they'll need the new field. Search for `LoaderConfig {` in test code. The `compose.rs` tests construct it:

```rust
let config = LoaderConfig {
    limits: ...,
    ..LoaderConfig::unchecked()
};
```

These will still work because `..LoaderConfig::unchecked()` fills in `tag_resolution`.

**Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p yamalgam-core`
Expected: All tests PASS.

**Step 5: Run parser tests too (struct literal usage)**

Run: `cargo nextest run -p yamalgam-parser`
Expected: All tests PASS (struct update syntax fills in new field).

**Step 6: Commit**

```
feat(core): add tag_resolution field to LoaderConfig
```

---

### Task 7: Composer integration

**Files:**
- Modify: `crates/yamalgam-parser/src/compose.rs`

This is the core wiring task. The Composer gets a `Box<dyn TagResolver>` field and uses it instead of the hardcoded `resolve_plain_scalar()` call.

**Step 1: Write the failing tests**

Add to tests in `compose.rs`:

```rust
    #[test]
    fn compose_with_failsafe_schema() {
        use yamalgam_core::{LoaderConfig, TagResolution};
        let config = LoaderConfig::unchecked().with_tag_resolution(TagResolution::Failsafe);
        let docs = Composer::from_str_with_config("true", &config).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0], Value::String("true".into()));
    }

    #[test]
    fn compose_with_yaml11_booleans() {
        use yamalgam_core::{LoaderConfig, TagResolution};
        let config = LoaderConfig::unchecked().with_tag_resolution(TagResolution::Yaml11);
        let docs = Composer::from_str_with_config("yes", &config).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0], Value::Bool(true));
    }

    #[test]
    fn compose_with_json_schema() {
        use yamalgam_core::{LoaderConfig, TagResolution};
        let config = LoaderConfig::unchecked().with_tag_resolution(TagResolution::Json);
        let docs = Composer::from_str_with_config("True", &config).unwrap();
        assert_eq!(docs.len(), 1);
        // JSON schema: "True" is not a bool, only "true" is
        assert_eq!(docs[0], Value::String("True".into()));
    }

    #[test]
    fn compose_default_is_yaml12() {
        // Existing behavior unchanged — "yes" is a string in YAML 1.2
        let docs = Composer::from_str("yes").unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0], Value::String("yes".into()));
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo nextest run -p yamalgam-parser -E 'test(compose_with_)'`
Expected: FAIL — `from_str_with_config` ignores tag_resolution (still uses hardcoded Core).

**Step 3: Write implementation**

In `compose.rs`:

1. Replace the `resolve_plain_scalar` import with tag_resolution imports:

```rust
use yamalgam_core::{Mapping, ResourceLimits, Value};
use yamalgam_core::tag_resolution::TagResolver;
use yamalgam_core::tag::Yaml12TagResolver;
```

2. Add field to `Composer`:

```rust
pub struct Composer<'input, I>
where
    I: Iterator<Item = Result<Event<'input>, ResolveError>>,
{
    events: std::iter::Peekable<I>,
    anchors: HashMap<String, Value>,
    config: ResourceLimits,
    alias_expansion_count: usize,
    tag_resolver: Box<dyn TagResolver>,
}
```

3. Update `new()`:

```rust
    pub fn new(events: I) -> Self {
        Self {
            events: events.peekable(),
            anchors: HashMap::new(),
            config: ResourceLimits::none(),
            alias_expansion_count: 0,
            tag_resolver: Box::new(Yaml12TagResolver),
        }
    }
```

4. Update `new_with_config()`:

```rust
    pub fn new_with_config(events: I, config: &yamalgam_core::LoaderConfig) -> Self {
        Self {
            events: events.peekable(),
            anchors: HashMap::new(),
            config: config.limits.clone(),
            alias_expansion_count: 0,
            tag_resolver: Box::new(config.tag_resolution),
        }
    }
```

5. Change the free function `resolve_scalar` to a method on `Composer`:

```rust
    /// Resolve a scalar value based on its style.
    fn resolve_scalar(&self, value: &str, style: ScalarStyle) -> Value {
        match style {
            ScalarStyle::Plain => self.tag_resolver.resolve_scalar(value),
            _ => Value::String(value.to_owned()),
        }
    }
```

6. In `compose_node()`, change the call from `resolve_scalar(&value, style)` to `self.resolve_scalar(&value, style)`.

7. Remove the old free function `resolve_scalar` (it's now a method).

**Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p yamalgam-parser`
Expected: All tests PASS (new tests + existing tests).

**Step 5: Commit**

```
feat(parser): wire TagResolver into Composer for pluggable scalar typing
```

---

### Task 8: Convenience API for custom TagResolver

**Files:**
- Modify: `crates/yamalgam-parser/src/compose.rs`

**Step 1: Write the failing test**

Add to tests in `compose.rs`:

```rust
    #[test]
    fn compose_with_custom_tag_resolver() {
        use yamalgam_core::tag_resolution::TagResolver;

        /// Custom resolver: resolves "MAGIC" to Integer(42), everything else is a string.
        struct MagicResolver;
        impl TagResolver for MagicResolver {
            fn resolve_scalar(&self, value: &str) -> Value {
                if value == "MAGIC" {
                    Value::Integer(42)
                } else {
                    Value::String(value.to_owned())
                }
            }
        }

        let docs = Composer::from_str_with_tag_resolver("MAGIC", &MagicResolver).unwrap();
        assert_eq!(docs[0], Value::Integer(42));

        let docs = Composer::from_str_with_tag_resolver("true", &MagicResolver).unwrap();
        assert_eq!(docs[0], Value::String("true".into()));
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo nextest run -p yamalgam-parser -E 'test(custom_tag_resolver)'`
Expected: Compile error — `from_str_with_tag_resolver` doesn't exist.

**Step 3: Write implementation**

Add to the `impl<'input> Composer<'input, ResolvedEvents<'input, NoopResolver>>` block:

```rust
    /// Parse and compose all documents using a custom tag resolver.
    pub fn from_str_with_tag_resolver(
        input: &'input str,
        tag_resolver: &dyn TagResolver,
    ) -> Result<Vec<Value>, ComposeError> {
        let parser = crate::parser::Parser::new(input);
        let events = ResolvedEvents::new(
            Box::new(parser.map(|r| r.map_err(ResolveError::Parse))),
            NoopResolver,
        );
        let mut composer = Composer {
            events: events.peekable(),
            anchors: HashMap::new(),
            config: ResourceLimits::none(),
            alias_expansion_count: 0,
            tag_resolver: Box::new(tag_resolver.resolve_to_owned()),
        };
        composer.compose_stream()
    }
```

Wait — `&dyn TagResolver` can't be boxed directly since we need `'static`. Let me reconsider. The trait doesn't have a lifetime parameter, so we need the resolver to be owned. Better API:

```rust
    /// Parse and compose all documents using a custom tag resolver.
    pub fn from_str_with_tag_resolver(
        input: &'input str,
        tag_resolver: impl TagResolver + 'static,
    ) -> Result<Vec<Value>, ComposeError> {
        let parser = crate::parser::Parser::new(input);
        let events = ResolvedEvents::new(
            Box::new(parser.map(|r| r.map_err(ResolveError::Parse))),
            NoopResolver,
        );
        let mut composer = Composer {
            events: events.peekable(),
            anchors: HashMap::new(),
            config: ResourceLimits::none(),
            alias_expansion_count: 0,
            tag_resolver: Box::new(tag_resolver),
        };
        composer.compose_stream()
    }
```

Update the test to pass by value (ZSTs are Copy):

```rust
        let docs = Composer::from_str_with_tag_resolver("MAGIC", MagicResolver).unwrap();
```

Also add re-export in `crates/yamalgam-parser/src/lib.rs` — make sure `TagResolver` is accessible:

No new re-export needed — users import `TagResolver` from `yamalgam_core`.

**Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p yamalgam-parser -E 'test(custom_tag_resolver)'`
Expected: PASS

**Step 5: Commit**

```
feat(parser): add from_str_with_tag_resolver for custom schemas
```

---

### Task 9: Full validation

**Step 1: Run all tests**

Run: `cargo nextest run`
Expected: All tests PASS.

**Step 2: Run clippy**

Run: `just clippy`
Expected: No warnings.

**Step 3: Run full check**

Run: `just check`
Expected: All green (fmt + clippy + deny + test + doc-test).

**Step 4: Fix any issues found, then commit fixes if needed**

---

### Task 10: Update roadmap

**Files:**
- Modify: `docs/plans/README.md` — update M7 status to "Complete"

**Step 1: Update the M7 entry**

Change:

```markdown
## Milestone 7 — Schema Resolver Trait
**Status:** Not started
```

To:

```markdown
## Milestone 7 — Tag Resolution Trait
**Status:** Complete

Tag resolution for plain scalar typing, pluggable via `TagResolver` trait.
Four built-in implementations: Yaml12 (default), Failsafe (all strings),
Json (strict subset), Yaml11 (extended booleans, legacy octal).

- Design: `2026-03-09-tag-resolution-trait-design.md`
- Plan: `2026-03-09-tag-resolution-trait-plan.md`
```

**Step 2: Commit**

```
docs: update roadmap — M7 tag resolution complete
```

---

Plan complete and saved to `docs/plans/2026-03-09-tag-resolution-trait-plan.md`. Two execution options:

**1. Subagent-Driven (this session)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Parallel Session (separate)** — Open new session with executing-plans, batch execution with checkpoints.

Which approach?
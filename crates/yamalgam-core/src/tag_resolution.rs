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

/// YAML 1.2 JSON Schema (§10.2) — strict JSON-compatible type resolution.
///
/// Only recognises the exact JSON spellings:
/// - `null` (not `~`, `Null`, `NULL`, or empty)
/// - `true` / `false` (case-sensitive)
/// - Integers: `[0-9]+` or `-[0-9]+` (no `+`, no `0x`, no `0o`)
/// - Floats: decimal with `.` or exponent (no `+` prefix, no `.inf`, no `.nan`)
///
/// Anything else becomes `Value::String` (lenient fallback).
// y[impl schema.json.tag-null+3]
// y[impl schema.json.tag-bool+3]
// y[impl schema.json.tag-int+3]
// y[impl schema.json.tag-float+3]
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonTagResolver;

impl TagResolver for JsonTagResolver {
    fn resolve_scalar(&self, value: &str) -> Value {
        match value {
            "null" => Value::Null,
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            _ => try_json_number(value).unwrap_or_else(|| Value::String(value.to_owned())),
        }
    }
}

/// Try to parse `s` as a JSON-schema number (integer or float).
///
/// JSON schema integers: `[0-9]+` or `-[0-9]+` (no leading `+`, no `0x`/`0o`).
/// JSON schema floats: must contain `.` or `e`/`E`, no leading `+`, no `.inf`/`.nan`.
fn try_json_number(s: &str) -> Option<Value> {
    if s.is_empty() {
        return None;
    }

    let body = s.strip_prefix('-').unwrap_or(s);

    // Must start with a digit after optional `-`
    if !body.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }

    // No leading `+`
    if s.starts_with('+') {
        return None;
    }

    let is_float = s.contains('.') || s.contains('e') || s.contains('E');

    if is_float {
        let f: f64 = s.parse().ok()?;
        Some(Value::Float(f))
    } else {
        // Integer — only ASCII digits (plus optional leading `-`)
        if !body.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let i: i64 = s.parse().ok()?;
        Some(Value::Integer(i))
    }
}

// Re-export Yaml12TagResolver from tag.rs so it's accessible via this module.
pub use crate::tag::Yaml12TagResolver;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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

    #[test]
    fn json_null() {
        let r = JsonTagResolver;
        assert_eq!(r.resolve_scalar("null"), Value::Null);
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
        assert_eq!(r.resolve_scalar("True"), Value::String("True".into()));
        assert_eq!(r.resolve_scalar("FALSE"), Value::String("FALSE".into()));
    }

    #[test]
    fn json_integer() {
        let r = JsonTagResolver;
        assert_eq!(r.resolve_scalar("0"), Value::Integer(0));
        assert_eq!(r.resolve_scalar("42"), Value::Integer(42));
        assert_eq!(r.resolve_scalar("-17"), Value::Integer(-17));
        assert_eq!(r.resolve_scalar("0o17"), Value::String("0o17".into()));
        assert_eq!(r.resolve_scalar("0xFF"), Value::String("0xFF".into()));
        assert_eq!(r.resolve_scalar("+42"), Value::String("+42".into()));
    }

    #[test]
    fn json_float() {
        let r = JsonTagResolver;
        assert_eq!(r.resolve_scalar("1.0"), Value::Float(1.0));
        assert_eq!(r.resolve_scalar("-0.5"), Value::Float(-0.5));
        assert_eq!(r.resolve_scalar("1e10"), Value::Float(1e10));
        assert_eq!(r.resolve_scalar("1.5e-3"), Value::Float(1.5e-3));
        assert_eq!(r.resolve_scalar(".inf"), Value::String(".inf".into()));
        assert_eq!(r.resolve_scalar(".nan"), Value::String(".nan".into()));
        assert_eq!(r.resolve_scalar(".Inf"), Value::String(".Inf".into()));
        assert_eq!(r.resolve_scalar("+1.0"), Value::String("+1.0".into()));
    }

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
            let resolver_result = r.resolve_scalar(s);
            let direct_result = resolve_plain_scalar(s);
            // NaN != NaN, so handle that case explicitly
            match (&resolver_result, &direct_result) {
                (Value::Float(a), Value::Float(b)) if a.is_nan() && b.is_nan() => {}
                _ => assert_eq!(
                    resolver_result, direct_result,
                    "Yaml12TagResolver disagrees with resolve_plain_scalar for {s:?}"
                ),
            }
        }
    }
}

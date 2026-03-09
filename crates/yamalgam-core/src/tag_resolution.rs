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

    // JSON numbers: 0 | -?[1-9][0-9]* (no leading zeros)
    if body.len() > 1 && body.starts_with('0') && body.as_bytes()[1].is_ascii_digit() {
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

// Re-export Yaml12TagResolver from tag.rs so it's accessible via this module.
pub use crate::tag::Yaml12TagResolver;

/// YAML 1.1 type resolution — extended booleans, legacy octal, binary integers,
/// and underscore separators in numbers.
///
/// Extends YAML 1.2 Core with:
/// - Extra booleans: `yes`/`no`, `on`/`off`, `y`/`n` (all case variants)
/// - `0`-prefix octal: `017` = 15 (not `0o17`)
/// - `0b` binary: `0b1010` = 10
/// - Underscore separators in all numeric bases: `1_000`, `0xFF_FF`
///
/// Does **not** implement sexagesimal values (`1:30:00`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Yaml11TagResolver;

impl TagResolver for Yaml11TagResolver {
    fn resolve_scalar(&self, value: &str) -> Value {
        // Null — same as Core
        if value.is_empty() || value == "~" || value == "null" || value == "Null" || value == "NULL"
        {
            return Value::Null;
        }

        // Booleans — Core set plus extended 1.1 set
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

        // Special float values: .inf, .nan (with optional sign for .inf)
        match value {
            ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => {
                return Value::Float(f64::INFINITY);
            }
            "-.inf" | "-.Inf" | "-.INF" => return Value::Float(f64::NEG_INFINITY),
            ".nan" | ".NaN" | ".NAN" => return Value::Float(f64::NAN),
            _ => {}
        }

        // Try numeric resolution (YAML 1.1 rules: 0-prefix octal, 0b binary, underscores)
        if let Some(v) = try_yaml11_number(value) {
            return v;
        }

        // Fallback: string
        Value::String(value.to_owned())
    }
}

/// Try to parse `s` as a YAML 1.1 number.
///
/// Supports:
/// - Decimal integers with optional sign and underscore separators
/// - `0`-prefix octal: `0[0-7_]+` (NOT `0o`)
/// - `0x` hex: `0x[0-9a-fA-F_]+`
/// - `0b` binary: `0b[01_]+`
/// - Floats with `.` or exponent, with underscore separators
fn try_yaml11_number(s: &str) -> Option<Value> {
    if s.is_empty() {
        return None;
    }

    let bytes = s.as_bytes();

    // Determine sign and body
    let (negative, body) = match bytes[0] {
        b'+' => (false, &s[1..]),
        b'-' => (true, &s[1..]),
        _ => (false, s),
    };

    if body.is_empty() {
        return None;
    }

    // Must start with a digit or '.' (for floats like .5)
    let first = body.as_bytes()[0];
    if !first.is_ascii_digit() && first != b'.' {
        return None;
    }

    // Check for 0-prefix special forms
    if body.starts_with('0') && body.len() > 1 {
        let second = body.as_bytes()[1];

        // 0x hex
        if second == b'x' || second == b'X' {
            let hex = &body[2..];
            if hex.is_empty() {
                return None;
            }
            let cleaned: String = hex.chars().filter(|&c| c != '_').collect();
            if cleaned.is_empty() || !cleaned.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            let n = i64::from_str_radix(&cleaned, 16).ok()?;
            return Some(Value::Integer(if negative { -n } else { n }));
        }

        // 0b binary
        if second == b'b' || second == b'B' {
            let bin = &body[2..];
            if bin.is_empty() {
                return None;
            }
            let cleaned: String = bin.chars().filter(|&c| c != '_').collect();
            if cleaned.is_empty() || !cleaned.bytes().all(|b| b == b'0' || b == b'1') {
                return None;
            }
            let n = i64::from_str_radix(&cleaned, 2).ok()?;
            return Some(Value::Integer(if negative { -n } else { n }));
        }

        // 0-prefix octal: 0[0-7_]+ (but NOT 0o, and NOT digits 8-9)
        if second.is_ascii_digit() || second == b'_' {
            let oct = &body[1..];
            let cleaned: String = oct.chars().filter(|&c| c != '_').collect();
            if cleaned.is_empty() {
                return None;
            }
            // All chars must be 0-7
            if !cleaned.bytes().all(|b| (b'0'..=b'7').contains(&b)) {
                return None;
            }
            let n = i64::from_str_radix(&cleaned, 8).ok()?;
            return Some(Value::Integer(if negative { -n } else { n }));
        }
    }

    // Strip underscores for decimal/float parsing
    let cleaned: String = body.chars().filter(|&c| c != '_').collect();
    if cleaned.is_empty() {
        return None;
    }

    // Float check: contains '.' or 'e'/'E'
    let is_float = cleaned.contains('.') || cleaned.contains('e') || cleaned.contains('E');

    if is_float {
        // Must start with a digit
        if !cleaned.as_bytes()[0].is_ascii_digit() {
            return None;
        }
        let full = if negative {
            format!("-{cleaned}")
        } else {
            cleaned
        };
        let f: f64 = full.parse().ok()?;
        Some(Value::Float(f))
    } else {
        // Decimal integer — all digits
        if !cleaned.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let n: i64 = cleaned.parse().ok()?;
        Some(Value::Integer(if negative { -n } else { n }))
    }
}

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
        // No leading zeros in JSON integers
        assert_eq!(r.resolve_scalar("017"), Value::String("017".into()));
        assert_eq!(r.resolve_scalar("00"), Value::String("00".into()));
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
        // No leading zeros in JSON floats
        assert_eq!(r.resolve_scalar("01.5"), Value::String("01.5".into()));
        assert_eq!(r.resolve_scalar("+1.0"), Value::String("+1.0".into()));
    }

    #[test]
    fn yaml12_matches_resolve_plain_scalar() {
        use crate::resolve_plain_scalar;
        let r = Yaml12TagResolver;
        let cases = [
            "null", "Null", "NULL", "~", "", "true", "True", "TRUE", "false", "False", "FALSE",
            "42", "-17", "+99", "0o17", "0xFF", "1.0", "-0.5", "1e10", ".inf", "-.inf", ".nan",
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

    #[test]
    fn yaml11_booleans() {
        let r = Yaml11TagResolver;
        assert_eq!(r.resolve_scalar("true"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("True"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("TRUE"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("false"), Value::Bool(false));
        assert_eq!(r.resolve_scalar("False"), Value::Bool(false));
        assert_eq!(r.resolve_scalar("FALSE"), Value::Bool(false));
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
        assert_eq!(r.resolve_scalar("017"), Value::Integer(15));
        assert_eq!(r.resolve_scalar("0"), Value::Integer(0));
        assert_eq!(r.resolve_scalar("010"), Value::Integer(8));
        // 0o is NOT 1.1 octal
        assert_eq!(r.resolve_scalar("0o17"), Value::String("0o17".into()));
        // 09 is not valid octal, should be string
        assert_eq!(r.resolve_scalar("09"), Value::String("09".into()));
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

    #[test]
    fn tag_resolution_enum_dispatches_correctly() {
        let r = TagResolution::default();
        assert_eq!(r, TagResolution::Yaml12);
        assert_eq!(r.resolve_scalar("true"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("yes"), Value::String("yes".into()));

        let r = TagResolution::Failsafe;
        assert_eq!(r.resolve_scalar("true"), Value::String("true".into()));
        assert_eq!(r.resolve_scalar("42"), Value::String("42".into()));

        let r = TagResolution::Json;
        assert_eq!(r.resolve_scalar("true"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("True"), Value::String("True".into()));

        let r = TagResolution::Yaml11;
        assert_eq!(r.resolve_scalar("yes"), Value::Bool(true));
        assert_eq!(r.resolve_scalar("017"), Value::Integer(15));
    }
}

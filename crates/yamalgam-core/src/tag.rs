//! YAML 1.2 Core Schema scalar resolution.
//!
//! Implements plain scalar resolution per [YAML 1.2 Core Schema §10.3.2][spec].
//! Only plain (unquoted) scalars undergo type resolution; quoted scalars are
//! always strings.
//!
//! **YAML 1.2 vs 1.1 booleans:** YAML 1.1 treated `yes`, `no`, `on`, `off` as
//! booleans. YAML 1.2 Core Schema does **not** — those are plain strings. Only
//! `true`/`True`/`TRUE` and `false`/`False`/`FALSE` resolve to booleans.
//!
//! [spec]: https://yaml.org/spec/1.2.2/#103-core-schema

use crate::Value;

/// Resolve a plain (unquoted) scalar string to a typed [`Value`] per the
/// YAML 1.2 Core Schema.
///
/// Quoted scalars should **not** be passed through this function — they are
/// always `Value::String`.
#[must_use]
pub fn resolve_plain_scalar(s: &str) -> Value {
    // Null
    if s.is_empty() || s == "~" || s == "null" || s == "Null" || s == "NULL" {
        return Value::Null;
    }

    // Booleans (YAML 1.2 only — no yes/no/on/off)
    match s {
        "true" | "True" | "TRUE" => return Value::Bool(true),
        "false" | "False" | "FALSE" => return Value::Bool(false),
        _ => {}
    }

    // Special float values: .inf, .nan (with optional sign for .inf)
    match s {
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => {
            return Value::Float(f64::INFINITY);
        }
        "-.inf" | "-.Inf" | "-.INF" => return Value::Float(f64::NEG_INFINITY),
        ".nan" | ".NaN" | ".NAN" => return Value::Float(f64::NAN),
        _ => {}
    }

    // Try numeric resolution
    if let Some(v) = try_integer(s) {
        return v;
    }

    if looks_like_float(s)
        && let Ok(f) = s.parse::<f64>()
    {
        return Value::Float(f);
    }

    // Fallback: string
    Value::String(s.to_owned())
}

/// Check whether `s` looks like a YAML float (has `.` or `e`/`E` exponent).
///
/// This prevents Rust's `f64::parse` from accepting bare integers or other
/// strings that YAML doesn't consider floats.
fn looks_like_float(s: &str) -> bool {
    // Must start with optional sign followed by a digit
    let rest = s.strip_prefix(['+', '-']).unwrap_or(s);
    if rest.is_empty() || !rest.as_bytes()[0].is_ascii_digit() {
        return false;
    }
    // Must contain `.` or `e`/`E` to be a float
    s.contains('.') || s.contains('e') || s.contains('E')
}

/// Try parsing `s` as a YAML 1.2 integer (decimal, octal, or hex).
fn try_integer(s: &str) -> Option<Value> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    // Determine sign and numeric body
    let (negative, body) = match bytes[0] {
        b'+' => (false, &s[1..]),
        b'-' => (true, &s[1..]),
        _ => (false, s),
    };

    if body.is_empty() {
        return None;
    }

    // Octal: 0o...
    if let Some(oct) = body.strip_prefix("0o") {
        if oct.is_empty() || !oct.bytes().all(|b| b.is_ascii_digit() && b < b'8') {
            return None;
        }
        let n = i64::from_str_radix(oct, 8).ok()?;
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

    // Decimal: all digits (no leading dot, no e/E — those are floats)
    if body.bytes().all(|b| b.is_ascii_digit()) {
        let n: i64 = body.parse().ok()?;
        return Some(Value::Integer(if negative { -n } else { n }));
    }

    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_variants() {
        assert_eq!(resolve_plain_scalar("null"), Value::Null);
        assert_eq!(resolve_plain_scalar("Null"), Value::Null);
        assert_eq!(resolve_plain_scalar("NULL"), Value::Null);
        assert_eq!(resolve_plain_scalar("~"), Value::Null);
        assert_eq!(resolve_plain_scalar(""), Value::Null);
    }

    #[test]
    fn bool_true_variants() {
        assert_eq!(resolve_plain_scalar("true"), Value::Bool(true));
        assert_eq!(resolve_plain_scalar("True"), Value::Bool(true));
        assert_eq!(resolve_plain_scalar("TRUE"), Value::Bool(true));
    }

    #[test]
    fn bool_false_variants() {
        assert_eq!(resolve_plain_scalar("false"), Value::Bool(false));
        assert_eq!(resolve_plain_scalar("False"), Value::Bool(false));
        assert_eq!(resolve_plain_scalar("FALSE"), Value::Bool(false));
    }

    #[test]
    fn yaml_11_booleans_are_strings() {
        for s in &[
            "yes", "no", "on", "off", "Yes", "No", "On", "Off", "YES", "NO", "ON", "OFF",
        ] {
            assert_eq!(
                resolve_plain_scalar(s),
                Value::String((*s).to_owned()),
                "{s:?} should be a string, not a boolean"
            );
        }
    }

    #[test]
    fn integers_decimal() {
        assert_eq!(resolve_plain_scalar("0"), Value::Integer(0));
        assert_eq!(resolve_plain_scalar("42"), Value::Integer(42));
        assert_eq!(resolve_plain_scalar("-17"), Value::Integer(-17));
        assert_eq!(resolve_plain_scalar("+99"), Value::Integer(99));
    }

    #[test]
    fn integers_octal() {
        assert_eq!(resolve_plain_scalar("0o17"), Value::Integer(0o17));
        assert_eq!(resolve_plain_scalar("0o0"), Value::Integer(0));
    }

    #[test]
    fn integers_hex() {
        assert_eq!(resolve_plain_scalar("0xFF"), Value::Integer(0xFF));
        assert_eq!(resolve_plain_scalar("0x0"), Value::Integer(0));
        assert_eq!(resolve_plain_scalar("0xDEAD"), Value::Integer(0xDEAD));
    }

    #[test]
    fn floats() {
        assert_eq!(resolve_plain_scalar("1.0"), Value::Float(1.0));
        assert_eq!(resolve_plain_scalar("-0.5"), Value::Float(-0.5));
        assert_eq!(resolve_plain_scalar("+12.5"), Value::Float(12.5));
        assert_eq!(resolve_plain_scalar("1e10"), Value::Float(1e10));
        assert_eq!(resolve_plain_scalar("1.5e-3"), Value::Float(1.5e-3));
    }

    #[test]
    fn float_special_values() {
        assert_eq!(resolve_plain_scalar(".inf"), Value::Float(f64::INFINITY));
        assert_eq!(resolve_plain_scalar(".Inf"), Value::Float(f64::INFINITY));
        assert_eq!(resolve_plain_scalar(".INF"), Value::Float(f64::INFINITY));
        assert_eq!(resolve_plain_scalar("+.inf"), Value::Float(f64::INFINITY));
        assert_eq!(
            resolve_plain_scalar("-.inf"),
            Value::Float(f64::NEG_INFINITY)
        );

        // NaN doesn't equal itself, so check via is_nan()
        match resolve_plain_scalar(".nan") {
            Value::Float(f) => assert!(f.is_nan(), ".nan should be NaN"),
            other => panic!("expected Float, got {other:?}"),
        }
        match resolve_plain_scalar(".NaN") {
            Value::Float(f) => assert!(f.is_nan(), ".NaN should be NaN"),
            other => panic!("expected Float, got {other:?}"),
        }
        match resolve_plain_scalar(".NAN") {
            Value::Float(f) => assert!(f.is_nan(), ".NAN should be NaN"),
            other => panic!("expected Float, got {other:?}"),
        }
    }

    #[test]
    fn plain_strings() {
        assert_eq!(
            resolve_plain_scalar("hello"),
            Value::String("hello".to_owned())
        );
        assert_eq!(
            resolve_plain_scalar("hello world"),
            Value::String("hello world".to_owned())
        );
        assert_eq!(
            resolve_plain_scalar("not-a-number"),
            Value::String("not-a-number".to_owned())
        );
    }

    #[test]
    fn edge_cases() {
        // Bare signs are strings
        assert_eq!(resolve_plain_scalar("+"), Value::String("+".to_owned()));
        assert_eq!(resolve_plain_scalar("-"), Value::String("-".to_owned()));
        // Leading zeros in decimal are still valid integers
        assert_eq!(resolve_plain_scalar("007"), Value::Integer(7));
        // 0x with uppercase X
        assert_eq!(resolve_plain_scalar("0XFF"), Value::Integer(0xFF));
    }
}

#![allow(missing_docs)]

use serde::Deserialize;
use yamalgam_serde::from_str;

#[derive(Debug, Deserialize, PartialEq)]
struct Server {
    host: String,
    port: u16,
}

#[test]
fn deserialize_struct() {
    let s: Server = from_str("host: localhost\nport: 8080").unwrap();
    assert_eq!(
        s,
        Server {
            host: "localhost".into(),
            port: 8080,
        }
    );
}

#[derive(Debug, Deserialize, PartialEq)]
struct Config {
    server: Server,
    debug: bool,
}

#[test]
fn deserialize_nested_struct() {
    let c: Config = from_str("server:\n  host: 0.0.0.0\n  port: 443\ndebug: false").unwrap();
    assert_eq!(c.server.host, "0.0.0.0");
    assert_eq!(c.server.port, 443);
    assert!(!c.debug);
}

#[derive(Debug, Deserialize, PartialEq)]
struct WithDefaults {
    name: String,
    #[serde(default)]
    count: u32,
    #[serde(default)]
    tags: Vec<String>,
}

#[test]
fn deserialize_with_defaults() {
    let w: WithDefaults = from_str("name: test").unwrap();
    assert_eq!(
        w,
        WithDefaults {
            name: "test".into(),
            count: 0,
            tags: vec![],
        }
    );
}

#[derive(Debug, Deserialize, PartialEq)]
struct WithOption {
    name: String,
    label: Option<String>,
}

#[test]
fn deserialize_optional_present() {
    let w: WithOption = from_str("name: foo\nlabel: bar").unwrap();
    assert_eq!(w.label, Some("bar".into()));
}

#[test]
fn deserialize_optional_null() {
    let w: WithOption = from_str("name: foo\nlabel: ~").unwrap();
    assert_eq!(w.label, None);
}

#[test]
fn deserialize_optional_missing() {
    let w: WithOption = from_str("name: foo").unwrap();
    assert_eq!(w.label, None);
}

#[derive(Debug, Deserialize, PartialEq)]
enum Color {
    Red,
    Green,
    Blue,
}

#[test]
fn deserialize_unit_enum() {
    assert_eq!(from_str::<Color>("Red").unwrap(), Color::Red);
    assert_eq!(from_str::<Color>("Blue").unwrap(), Color::Blue);
}

#[derive(Debug, Deserialize, PartialEq)]
enum Shape {
    Circle(f64),
    Rectangle { width: f64, height: f64 },
    Point,
}

#[test]
fn deserialize_newtype_enum() {
    let s: Shape = from_str("Circle: 5.0").unwrap();
    assert_eq!(s, Shape::Circle(5.0));
}

#[test]
fn deserialize_struct_enum() {
    let s: Shape = from_str("Rectangle:\n  width: 10.0\n  height: 20.0").unwrap();
    assert_eq!(
        s,
        Shape::Rectangle {
            width: 10.0,
            height: 20.0,
        }
    );
}

#[test]
fn deserialize_unit_enum_variant() {
    let s: Shape = from_str("Point").unwrap();
    assert_eq!(s, Shape::Point);
}

#[derive(Debug, Deserialize, PartialEq)]
struct WithVec {
    items: Vec<Server>,
}

#[test]
fn deserialize_vec_of_structs() {
    let input = "items:\n  - host: a\n    port: 1\n  - host: b\n    port: 2";
    let w: WithVec = from_str(input).unwrap();
    assert_eq!(w.items.len(), 2);
    assert_eq!(w.items[0].host, "a");
    assert_eq!(w.items[1].port, 2);
}

#[test]
fn deserialize_tuple() {
    let v: (i64, String, bool) = from_str("- 1\n- hello\n- true").unwrap();
    assert_eq!(v, (1, "hello".into(), true));
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum StringOrInt {
    Int(i64),
    Str(String),
}

#[test]
fn deserialize_untagged_enum() {
    assert_eq!(from_str::<StringOrInt>("42").unwrap(), StringOrInt::Int(42));
    assert_eq!(
        from_str::<StringOrInt>("hello").unwrap(),
        StringOrInt::Str("hello".into())
    );
}

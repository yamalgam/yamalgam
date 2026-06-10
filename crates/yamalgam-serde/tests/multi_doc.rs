#![allow(missing_docs)]

use serde::Deserialize;
use yamalgam_serde::{Deserializer, from_str};

#[test]
fn single_document_via_from_str() {
    let v: i64 = from_str("42").unwrap();
    assert_eq!(v, 42);
}

#[test]
fn from_str_errors_on_multiple_documents() {
    let result = from_str::<i64>("42\n---\n99");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("more than one document"),
        "error should mention multiple documents"
    );
}

#[derive(Debug, Deserialize, PartialEq)]
struct Item {
    name: String,
}

#[test]
fn documents_iterator() {
    let input = "---\nname: first\n---\nname: second\n---\nname: third";
    let docs: Vec<Item> = Deserializer::from_str(input)
        .documents::<Item>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(docs.len(), 3);
    assert_eq!(docs[0].name, "first");
    assert_eq!(docs[1].name, "second");
    assert_eq!(docs[2].name, "third");
}

#[test]
fn documents_mixed_scalars() {
    let input = "---\n42\n---\n99";
    let docs: Vec<i64> = Deserializer::from_str(input)
        .documents::<i64>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(docs, vec![42, 99]);
}

#[test]
fn documents_empty_stream() {
    let docs: Vec<i64> = Deserializer::from_str("")
        .documents::<i64>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(docs.is_empty());
}

#[test]
fn documents_single_implicit() {
    let docs: Vec<i64> = Deserializer::from_str("42")
        .documents::<i64>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(docs, vec![42]);
}

#[test]
fn documents_with_doc_end_markers() {
    let input = "---\n42\n...\n---\n99\n...";
    let docs: Vec<i64> = Deserializer::from_str(input)
        .documents::<i64>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(docs, vec![42, 99]);
}

#[test]
fn documents_stops_on_error() {
    // An error in one document should stop iteration
    let input = "---\n42\n---\n[invalid";
    let results: Vec<Result<i64, _>> = Deserializer::from_str(input).documents::<i64>().collect();
    assert!(results[0].is_ok());
    assert_eq!(results[0].as_ref().unwrap(), &42);
    // The second document should fail (it's a sequence, not a scalar)
    assert!(results.len() >= 2);
    assert!(results[1].is_err());
}

// ---------------------------------------------------------------------------
// Explicit empty documents
// ---------------------------------------------------------------------------

#[test]
fn documents_explicit_empty_docs_yield_null() {
    use yamalgam_core::Value;
    let docs: Vec<Value> = Deserializer::from_str("---\n---\n")
        .documents::<Value>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(docs, vec![Value::Null, Value::Null]);
}

#[test]
fn documents_single_explicit_empty_doc() {
    use yamalgam_core::Value;
    let docs: Vec<Value> = Deserializer::from_str("---\n")
        .documents::<Value>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(docs, vec![Value::Null]);
}

#[test]
fn documents_content_then_empty_doc() {
    use yamalgam_core::Value;
    let docs: Vec<Value> = Deserializer::from_str("---\nfoo\n---\n")
        .documents::<Value>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(docs, vec![Value::String("foo".into()), Value::Null]);
}

#[test]
fn documents_empty_then_content_doc() {
    use yamalgam_core::Value;
    let docs: Vec<Value> = Deserializer::from_str("---\n---\nfoo")
        .documents::<Value>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(docs, vec![Value::Null, Value::String("foo".into())]);
}

#[test]
fn documents_empty_doc_with_explicit_end_marker() {
    use yamalgam_core::Value;
    let docs: Vec<Value> = Deserializer::from_str("---\n...\n---\n42")
        .documents::<Value>()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(docs, vec![Value::Null, Value::Integer(42)]);
}

#![allow(missing_docs)]

use std::error::Error as StdError;
use std::io::{self, Cursor, Read};

use serde::Deserialize;
use yamalgam_serde::{Error, from_reader, from_reader_unchecked};

#[derive(Debug, Deserialize, PartialEq)]
struct Item {
    name: String,
}

struct FailingReader(io::ErrorKind);

impl Read for FailingReader {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::new(self.0, "reader fixture failed"))
    }
}

fn assert_io_error(error: Error, expected_kind: io::ErrorKind) {
    let Error::Io(io_error) = &error else {
        panic!("expected Error::Io, got {error:?}");
    };
    assert_eq!(io_error.kind(), expected_kind);
    assert_eq!(error.to_string(), "I/O error: reader fixture failed");

    let source = StdError::source(&error).expect("I/O error should expose its source");
    let io_source = source
        .downcast_ref::<io::Error>()
        .expect("source should remain std::io::Error");
    assert_eq!(io_source.kind(), expected_kind);
}

#[test]
fn from_reader_preserves_io_error() {
    let error = from_reader::<_, serde_json::Value>(FailingReader(io::ErrorKind::TimedOut))
        .expect_err("reader should fail");
    assert_io_error(error, io::ErrorKind::TimedOut);
}

#[test]
fn from_reader_unchecked_preserves_io_error() {
    let error =
        from_reader_unchecked::<_, serde_json::Value>(FailingReader(io::ErrorKind::TimedOut))
            .expect_err("reader should fail");
    assert_io_error(error, io::ErrorKind::TimedOut);
}

#[test]
fn deserialize_from_reader() {
    let data = b"name: yamalgam";
    let item: Item = from_reader(Cursor::new(data)).unwrap();
    assert_eq!(
        item,
        Item {
            name: "yamalgam".into()
        }
    );
}

#[test]
fn deserialize_from_reader_multi_line() {
    let data = b"host: localhost\nport: 8080";
    let item: std::collections::HashMap<String, String> = from_reader(Cursor::new(data)).unwrap();
    assert_eq!(item["host"], "localhost");
    assert_eq!(item["port"], "8080");
}

#[test]
fn deserialize_from_reader_sequence() {
    let data = b"- one\n- two\n- three";
    let items: Vec<String> = from_reader(Cursor::new(data)).unwrap();
    assert_eq!(items, vec!["one", "two", "three"]);
}

#[test]
fn deserialize_from_reader_empty() {
    let data = b"";
    // Empty input deserializes as unit / null.
    let result: Option<String> = from_reader(Cursor::new(data)).unwrap();
    assert_eq!(result, None);
}

//! Integration tests for query result emitters.

use yamalgam_query::{EmitOpts, emit::to_json, parse_str};

#[test]
fn json_output_materializes_aliases_and_stringifies_keys() {
    let value = parse_str("a: &x [1, .inf]\nb: *x\n1: *x\n")
        .unwrap()
        .remove(0);

    assert_eq!(
        to_json(&value, &EmitOpts::compact()),
        r#"{"a":[1,null],"b":[1,null],"1":[1,null]}"#
    );
}

#[test]
fn json_pretty_output_indents_two_spaces() {
    let value = parse_str("a: [1]\n").unwrap().remove(0);

    assert_eq!(
        to_json(&value, &EmitOpts::default()),
        "{\n  \"a\": [\n    1\n  ]\n}"
    );
}

#[test]
fn json_escapes_strings_and_can_sort_keys() {
    let value = parse_str("b: 'line\\break'\na: \"tab\\tquote\\\"\"\n")
        .unwrap()
        .remove(0);
    let options = EmitOpts {
        sort_keys: true,
        ..EmitOpts::compact()
    };

    assert_eq!(
        to_json(&value, &options),
        r#"{"a":"tab\tquote\"","b":"line\\break"}"#
    );
}

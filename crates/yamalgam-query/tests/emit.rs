//! Integration tests for query result emitters.

use yamalgam_query::{
    EmitOpts, YamlVal,
    emit::{to_json, to_yaml},
    parse_str,
};

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

#[test]
fn yaml_scalars_quote_when_plain_text_would_change_type() {
    let value = YamlVal::seq(
        ["a", "1", "true", "null", "x: y"]
            .into_iter()
            .map(YamlVal::str)
            .collect(),
    );

    assert_eq!(
        to_yaml(&value, &EmitOpts::default()),
        "- a\n- \"1\"\n- \"true\"\n- \"null\"\n- \"x: y\"\n"
    );
}

#[test]
fn yaml_multiline_strings_use_literal_blocks() {
    let value = parse_str("key: |\n  line1\n  line2\n").unwrap().remove(0);

    assert_eq!(
        to_yaml(&value, &EmitOpts::default()),
        "key: |\n  line1\n  line2\n"
    );
}

#[test]
fn yaml_scalar_types_survive_emit_and_reparse() {
    let source = "int: 1\nfloat: 1.0\nbool: true\nnully: null\nstr: '1'\n";
    let value = parse_str(source).unwrap().remove(0);
    let emitted = to_yaml(&value, &EmitOpts::default());
    let reparsed = parse_str(&emitted).unwrap().remove(0);

    assert_eq!(value, reparsed);
}

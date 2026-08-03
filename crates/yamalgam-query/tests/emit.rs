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

#[test]
fn yaml_block_collections_are_the_default() {
    let value = parse_str("a:\n  - 1\n  - x: 2\n").unwrap().remove(0);

    assert_eq!(
        to_yaml(&value, &EmitOpts::default()),
        "a:\n  - 1\n  - x: 2\n"
    );
}

#[test]
fn yaml_honors_flow_style_and_compact_forces_flow() {
    let flow = parse_str("a: {b: 1}\n").unwrap().remove(0);
    assert_eq!(to_yaml(&flow, &EmitOpts::default()), "a: {b: 1}\n");

    let block = parse_str("a:\n  b: 1\n").unwrap().remove(0);
    assert_eq!(to_yaml(&block, &EmitOpts::compact()), "{a: {b: 1}}\n");
}

#[test]
fn yaml_reemits_anchors_aliases_and_tags() {
    let anchored = parse_str("base: &b\n  x: 1\nchild: *b\n")
        .unwrap()
        .remove(0);
    assert_eq!(
        to_yaml(&anchored, &EmitOpts::default()),
        "base: &b\n  x: 1\nchild: *b\n"
    );

    let tagged = parse_str("a: !custom hi\nb: !!binary aGk=\n")
        .unwrap()
        .remove(0);
    assert_eq!(
        to_yaml(&tagged, &EmitOpts::default()),
        "a: !custom hi\nb: !!binary aGk=\n"
    );
}

#[test]
fn yaml_sort_keys_changes_output_not_the_value() {
    let value = parse_str("b: 1\na: 2\n").unwrap().remove(0);
    let options = EmitOpts {
        sort_keys: true,
        ..EmitOpts::default()
    };

    assert_eq!(to_yaml(&value, &options), "a: 2\nb: 1\n");
    assert_eq!(
        value.keys().cloned().collect::<Vec<_>>(),
        vec![YamlVal::str("b"), YamlVal::str("a")]
    );
}

#[test]
fn yaml_emits_non_string_and_complex_keys() {
    let value = YamlVal::map(vec![
        (YamlVal::int(1), YamlVal::str("a")),
        (YamlVal::from(true), YamlVal::str("b")),
        (
            YamlVal::seq(vec![YamlVal::int(1), YamlVal::int(2)]),
            YamlVal::str("c"),
        ),
    ]);

    assert_eq!(
        to_yaml(&value, &EmitOpts::default()),
        "1: a\ntrue: b\n[1, 2]: c\n"
    );
}

#[test]
fn yaml_emit_reparse_round_trips_fixture_corpus() {
    const ALLOWLIST: &[&str] = &[
        // Known scanner/parser gaps: root indentation and tabs in separation.
        "prettier/root/example.yml",
        "prettier/spec/spec-example-5-12-tabs-and-spaces.yml",
        "prettier/spec/spec-example-6-1-indentation-spaces.yml",
        "prettier/spec/spec-example-6-3-separation-spaces.yml",
        "prettier/spec/spec-example-7-6-double-quoted-lines.yml",
        "prettier/spec/various-trailing-tabs.yml",
    ];

    let fixture_root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../yamalgam-serde/tests/fixtures");
    let mut files = Vec::new();
    collect_yaml_files(&fixture_root, &mut files);
    files.sort();

    let mut failures = Vec::new();
    for path in files {
        let relative = path
            .strip_prefix(&fixture_root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if ALLOWLIST.contains(&relative.as_str()) {
            continue;
        }

        let source = std::fs::read_to_string(&path).unwrap();
        let documents = match parse_str(&source) {
            Ok(documents) => documents,
            Err(error) => {
                failures.push(format!("{relative}: source parse failed: {error}"));
                continue;
            }
        };
        for (index, document) in documents.into_iter().enumerate() {
            let emitted = to_yaml(&document, &EmitOpts::default());
            match parse_str(&emitted) {
                Ok(reparsed) if reparsed.as_slice() == [document.clone()] => {}
                Ok(reparsed) => failures.push(format!(
                    "{relative} document {index}: value changed: {document} -> {reparsed:?}"
                )),
                Err(error) => failures.push(format!(
                    "{relative} document {index}: emitted YAML failed to parse: {error}\n{emitted}"
                )),
            }
        }
    }

    assert!(
        failures.is_empty(),
        "YAML emit/reparse corpus failures:\n{}",
        failures.join("\n")
    );
}

fn collect_yaml_files(directory: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_yaml_files(&path, files);
        } else if matches!(
            path.extension().and_then(std::ffi::OsStr::to_str),
            Some("yaml" | "yml")
        ) {
            files.push(path);
        }
    }
}

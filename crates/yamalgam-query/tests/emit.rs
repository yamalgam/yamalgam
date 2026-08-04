//! Integration tests for query result emitters.

use yamalgam_core::{ResourceLimits, TagResolution};
use yamalgam_query::{
    EmitFormat, EmitOpts, YamlVal,
    emit::{emit, to_json, to_yaml},
    parse_str,
};

#[test]
fn emit_dispatches_by_format() {
    let value = YamlVal::map(vec![(YamlVal::str("answer"), YamlVal::int(42))]);
    let options = EmitOpts::compact();

    assert_eq!(
        emit(&value, EmitFormat::Yaml, &options).unwrap(),
        "{answer: 42}\n"
    );
    assert_eq!(
        emit(&value, EmitFormat::Json, &options).unwrap(),
        r#"{"answer":42}"#
    );
}

#[test]
fn json_output_materializes_aliases_and_stringifies_keys() {
    let value = parse_str("a: &x [1, .inf]\nb: *x\n1: *x\n")
        .unwrap()
        .remove(0);

    assert_eq!(
        to_json(&value, &EmitOpts::compact()).unwrap(),
        r#"{"a":[1,null],"b":[1,null],"1":[1,null]}"#
    );
}

#[test]
fn json_pretty_output_indents_two_spaces() {
    let value = parse_str("a: [1]\n").unwrap().remove(0);

    assert_eq!(
        to_json(&value, &EmitOpts::default()).unwrap(),
        "{\n  \"a\": [\n    1\n  ]\n}"
    );
}

#[test]
fn json_rejects_keys_that_collide_after_string_conversion() {
    let value = parse_str("{1: alpha, \"1\": beta}\n").unwrap().remove(0);

    assert!(
        to_json(&value, &EmitOpts::compact())
            .unwrap_err()
            .to_string()
            .contains("duplicated")
    );
}

#[test]
fn json_escapes_strings_and_can_sort_keys() {
    let value = parse_str("b: 'line\\break'\na: \"tab\\tquote\\\"\"\n")
        .unwrap()
        .remove(0);
    let mut options = EmitOpts::compact();
    options.sort_keys = true;

    assert_eq!(
        to_json(&value, &options).unwrap(),
        r#"{"a":"tab\tquote\"","b":"line\\break"}"#
    );
}

#[test]
fn json_escapes_extended_control_characters() {
    let value = YamlVal::seq(vec![
        YamlVal::str("x\u{7f}y"),
        YamlVal::str("x\u{85}y"),
        YamlVal::str("x\u{9f}y"),
    ]);

    assert_eq!(
        to_json(&value, &EmitOpts::compact()).unwrap(),
        r#"["x\u007fy","x\u0085y","x\u009fy"]"#
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
        to_yaml(&value, &EmitOpts::default()).unwrap(),
        "- a\n- \"1\"\n- \"true\"\n- \"null\"\n- \"x: y\"\n"
    );
}

#[test]
fn yaml_document_markers_are_quoted_at_the_root() {
    for marker in ["...", "... text", "---", "--- text"] {
        let value = YamlVal::str(marker);
        let emitted = to_yaml(&value, &EmitOpts::default()).unwrap();

        assert_eq!(emitted, format!("\"{marker}\"\n"));
        assert_eq!(parse_str(&emitted).unwrap(), vec![value]);
    }
}

#[test]
fn yaml_flow_scalars_quote_flow_indicators() {
    let value = YamlVal::seq(vec![
        YamlVal::str("x,y"),
        YamlVal::str("x[y"),
        YamlVal::str("x]y"),
        YamlVal::str("x{y"),
        YamlVal::str("x}y"),
    ]);

    let emitted = to_yaml(&value, &EmitOpts::compact()).unwrap();
    assert_eq!(emitted, "[\"x,y\", \"x[y\", \"x]y\", \"x{y\", \"x}y\"]\n");
    assert_eq!(parse_str(&emitted).unwrap(), vec![value]);
}

#[test]
fn yaml_scalars_quote_for_active_schema() {
    let value = YamlVal::str("yes");
    let mut options = EmitOpts::default();
    options.schema = TagResolution::Yaml11;

    let emitted = to_yaml(&value, &options).unwrap();
    assert_eq!(emitted, "\"yes\"\n");
    assert_eq!(
        yamalgam_query::parse_str_with_config(
            &emitted,
            &yamalgam_core::LoaderConfig::moderate().with_tag_resolution(TagResolution::Yaml11),
        )
        .unwrap(),
        vec![value]
    );
}

#[test]
fn yaml_control_characters_are_double_quoted() {
    let value = YamlVal::seq(vec![
        YamlVal::str("x\ry"),
        YamlVal::str("x\u{1b}y"),
        YamlVal::str("x\u{7f}y"),
        YamlVal::str("x\u{85}y"),
        YamlVal::str("x\u{9f}y"),
        YamlVal::str("x\u{2028}y"),
        YamlVal::str("x\u{2029}y"),
    ]);

    assert_eq!(
        to_yaml(&value, &EmitOpts::default()).unwrap(),
        concat!(
            "- \"x\\ry\"\n",
            "- \"x\\x1by\"\n",
            "- \"x\\x7fy\"\n",
            "- \"x\\x85y\"\n",
            "- \"x\\x9fy\"\n",
            "- \"x\\Ly\"\n",
            "- \"x\\Py\"\n",
        )
    );
}

#[test]
fn emitters_bound_materialized_aliases() {
    let document = parse_str("a: &a [1]\nb: &b [*a, *a]\nc: [*b, *b]\n")
        .unwrap()
        .remove(0);
    let value = document.get_str("c").unwrap();
    let mut options = EmitOpts::compact();
    options.limits = ResourceLimits {
        max_alias_expansions: Some(2),
        ..ResourceLimits::none()
    };

    assert!(
        to_json(value, &options)
            .unwrap_err()
            .to_string()
            .contains("alias")
    );
    assert!(
        to_yaml(value, &options)
            .unwrap_err()
            .to_string()
            .contains("alias")
    );
}

#[test]
fn emitters_bound_constructed_depth_and_output_size() {
    let deep = YamlVal::seq(vec![YamlVal::seq(vec![YamlVal::seq(
        vec![YamlVal::null()],
    )])]);
    let mut depth_options = EmitOpts::compact();
    depth_options.limits = ResourceLimits {
        max_depth: Some(2),
        ..ResourceLimits::none()
    };
    assert!(to_json(&deep, &depth_options).is_err());
    assert!(to_yaml(&deep, &depth_options).is_err());

    let wide = YamlVal::seq(vec![YamlVal::str("12345"), YamlVal::str("67890")]);
    let mut size_options = EmitOpts::compact();
    size_options.limits = ResourceLimits {
        max_output_bytes: Some(10),
        ..ResourceLimits::none()
    };
    assert!(to_json(&wide, &size_options).is_err());
    assert!(to_yaml(&wide, &size_options).is_err());
}

#[test]
fn yaml_multiline_strings_use_literal_blocks() {
    let value = parse_str("key: |\n  line1\n  line2\n").unwrap().remove(0);

    assert_eq!(
        to_yaml(&value, &EmitOpts::default()).unwrap(),
        "key: |\n  line1\n  line2\n"
    );
}

#[test]
fn yaml_scalar_types_survive_emit_and_reparse() {
    let source = "int: 1\nfloat: 1.0\nbool: true\nnully: null\nstr: '1'\n";
    let value = parse_str(source).unwrap().remove(0);
    let emitted = to_yaml(&value, &EmitOpts::default()).unwrap();
    let reparsed = parse_str(&emitted).unwrap().remove(0);

    assert_eq!(value, reparsed);
}

#[test]
fn yaml_block_collections_are_the_default() {
    let value = parse_str("a:\n  - 1\n  - x: 2\n").unwrap().remove(0);

    assert_eq!(
        to_yaml(&value, &EmitOpts::default()).unwrap(),
        "a:\n  - 1\n  - x: 2\n"
    );
}

#[test]
fn yaml_honors_flow_style_and_compact_forces_flow() {
    let flow = parse_str("a: {b: 1}\n").unwrap().remove(0);
    assert_eq!(to_yaml(&flow, &EmitOpts::default()).unwrap(), "a: {b: 1}\n");

    let block = parse_str("a:\n  b: 1\n").unwrap().remove(0);
    assert_eq!(
        to_yaml(&block, &EmitOpts::compact()).unwrap(),
        "{a: {b: 1}}\n"
    );
}

#[test]
fn yaml_reemits_anchors_aliases_and_tags() {
    let anchored = parse_str("base: &b\n  x: 1\nchild: *b\n")
        .unwrap()
        .remove(0);
    assert_eq!(
        to_yaml(&anchored, &EmitOpts::default()).unwrap(),
        "base: &b\n  x: 1\nchild: *b\n"
    );

    let tagged = parse_str("a: !custom hi\nb: !!binary aGk=\n")
        .unwrap()
        .remove(0);
    assert_eq!(
        to_yaml(&tagged, &EmitOpts::default()).unwrap(),
        "a: !custom hi\nb: !!binary aGk=\n"
    );
}

#[test]
fn yaml_omits_tags_that_cannot_be_reemitted_safely() {
    let value = parse_str("k: !x%0Ainjected%3A%20true v\n")
        .unwrap()
        .remove(0);

    assert_eq!(to_yaml(&value, &EmitOpts::default()).unwrap(), "k: v\n");
}

#[test]
fn yaml_sort_keys_changes_output_not_the_value() {
    let value = parse_str("b: 1\na: 2\n").unwrap().remove(0);
    let mut options = EmitOpts::default();
    options.sort_keys = true;

    assert_eq!(to_yaml(&value, &options).unwrap(), "a: 2\nb: 1\n");
    assert_eq!(
        value.keys().unwrap().cloned().collect::<Vec<_>>(),
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
        to_yaml(&value, &EmitOpts::default()).unwrap(),
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
            let emitted = to_yaml(&document, &EmitOpts::default()).unwrap();
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

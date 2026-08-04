//! End-to-end tests for jaq filter compilation and execution.

use yamalgam_core::LoaderConfig;
use yamalgam_query::YamlVal;
use yamalgam_query::engine::{self, RunCtx};

fn eval(filter: &str, input: &str) -> Vec<String> {
    let context = RunCtx::new(LoaderConfig::moderate());
    let compiled = engine::compile_filter(filter, &[]).unwrap();
    let documents = yamalgam_query::parse_str(input).unwrap();
    let mut output = Vec::new();
    engine::run_filter(
        &compiled,
        &context,
        documents.into_iter().map(Ok::<_, String>),
        |value| {
            output.push(value.to_string());
            Ok::<(), yamalgam_query::YgError>(())
        },
    )
    .unwrap();
    output
}

fn eval_yaml(filter: &str, input: &str) -> Vec<String> {
    let context = RunCtx::new(LoaderConfig::moderate());
    let compiled = engine::compile_filter(filter, &[]).unwrap();
    let documents = yamalgam_query::parse_str(input).unwrap();
    let mut output = Vec::new();
    engine::run_filter(
        &compiled,
        &context,
        documents.into_iter().map(Ok::<_, String>),
        |value| {
            output.push(
                yamalgam_query::emit::to_yaml(&value, &yamalgam_query::EmitOpts::compact())
                    .unwrap()
                    .trim_end()
                    .to_owned(),
            );
            Ok::<(), yamalgam_query::YgError>(())
        },
    )
    .unwrap();
    output
}

#[test]
fn identity_and_field_access_work() {
    assert_eq!(eval(".", "a: 1\n"), vec!["{a: 1}"]);
    assert_eq!(eval(".a", "a: 1\n"), vec!["1"]);
    assert_eq!(eval(".a.b", "a: {b: 2}\n"), vec!["2"]);
    assert_eq!(eval(".missing", "a: 1\n"), vec!["null"]);
}

#[test]
fn pipes_commas_and_iteration_work() {
    assert_eq!(eval(".[] | . + 1", "[1, 2]\n"), vec!["2", "3"]);
    assert_eq!(eval(".a, .b", "a: 1\nb: 2\n"), vec!["1", "2"]);
}

#[test]
fn keys_preserve_insertion_order_unless_sorted() {
    assert_eq!(eval("keys", "b: 1\na: 2\n"), vec!["[b, a]"]);
    assert_eq!(eval("keys_sorted", "b: 1\na: 2\n"), vec!["[a, b]"]);
    assert_eq!(eval("keys", "[9, 8]\n"), vec!["[0, 1]"]);
}

#[test]
fn type_returns_yaml_data_model_names() {
    assert_eq!(
        eval("map(type)", r#"[null, true, 1, 1.5, "s", [], {}]"#),
        vec!["[\"null\", boolean, integer, float, string, sequence, mapping]"]
    );
}

#[test]
fn definitions_reductions_and_conditionals_work() {
    assert_eq!(
        eval(
            "def double: . * 2; [.[] | if . > 1 then double else . end]",
            "[1, 2]\n"
        ),
        vec!["[1, 4]"]
    );
    assert_eq!(
        eval("reduce .[] as $x (0; . + $x)", "[1, 2, 3]\n"),
        vec!["6"]
    );
}

#[test]
fn string_interpolation_uses_yaml_values() {
    assert_eq!(eval(r#""v=\(.a)""#, "a: 7\n"), vec!["v=7"]);
}

#[test]
fn syntax_and_compile_errors_are_distinct() {
    let syntax = engine::compile_filter(".a |", &[])
        .err()
        .expect("invalid syntax must fail");
    assert!(matches!(syntax, yamalgam_query::YgError::FilterSyntax(_)));
    let compile = engine::compile_filter("nosuchfilter", &[])
        .err()
        .expect("undefined filter must fail");
    assert!(matches!(compile, yamalgam_query::YgError::FilterCompile(_)));
}

#[test]
fn core_native_filters_match_jq_semantics() {
    assert_eq!(eval("length", "[1, 2, 3]\n"), vec!["3"]);
    assert_eq!(eval("length", "\"héllo\"\n"), vec!["5"]);
    assert_eq!(eval("length", "a: 1\n"), vec!["1"]);
    assert_eq!(eval("length", "null\n"), vec!["0"]);
    assert_eq!(eval("has(\"a\")", "a: 1\n"), vec!["true"]);
    assert_eq!(eval("has(1)", "[9, 8]\n"), vec!["true"]);
    assert_eq!(eval("contains(\"ell\")", "\"hello\"\n"), vec!["true"]);
    assert_eq!(eval("indices(\",\")", "\"a,b,c\"\n"), vec!["[1, 3]"]);
    assert_eq!(eval("bsearch(3)", "[1, 3, 5]\n"), vec!["1"]);
    assert_eq!(eval("bsearch(4)", "[1, 3, 5]\n"), vec!["-3"]);
}

#[test]
fn json_conversion_and_standard_definitions_work() {
    assert_eq!(
        eval("tojson | fromjson | .a", "a: [1, null]\n"),
        vec!["[1, null]"]
    );
    assert_eq!(eval("tojson | length", "a: [1, null]\n"), vec!["14"]);
    assert_eq!(eval("fromjson", "'{\"a\": 1}'\n"), vec!["{a: 1}"]);
    assert_eq!(eval("add", "[{a: 1}, {b: 2}]\n"), vec!["{a: 1, b: 2}"]);
    assert_eq!(eval("to_entries", "a: 1\n"), vec!["[{key: a, value: 1}]"]);
    assert_eq!(eval("explode", "\"ab\"\n"), vec!["[97, 98]"]);
    assert_eq!(eval(r#""42" | tonumber"#, "null\n"), vec!["42"]);
}

#[test]
fn yaml_metadata_filters_expose_source_properties() {
    let document = concat!(
        "plain: 1\n",
        "quoted: \"1\"\n",
        "lit: |\n  x\n",
        "anchored: &a [1]\n",
        "ref: *a\n",
        "tagged: !custom v\n",
    );

    assert_eq!(eval(".tagged | tag", document), vec!["\"!custom\""]);
    assert_eq!(eval(".plain | tag", document), vec!["null"]);
    assert_eq!(eval(".anchored | anchor", document), vec!["a"]);
    assert_eq!(eval(".ref | anchor", document), vec!["a"]);
    assert_eq!(eval(".plain | anchor", document), vec!["null"]);
    assert_eq!(eval(".quoted | style", document), vec!["double-quoted"]);
    assert_eq!(eval(".lit | style", document), vec!["literal"]);
    assert_eq!(eval(".anchored | style", document), vec!["flow"]);
    assert_eq!(eval(".ref | kind", document), vec!["alias"]);
    assert_eq!(eval(".plain | kind", document), vec!["scalar"]);
    assert_eq!(eval(".ref | is_alias", document), vec!["true"]);
    assert_eq!(eval(".plain | raw", document), vec!["\"1\""]);
    assert_eq!(eval(r#""1.0" | raw"#, "null\n"), vec!["\"1.0\""]);
    assert_eq!(eval(".plain | resolved_type", document), vec!["integer"]);
}

#[test]
fn raw_rejects_collections() {
    let context = RunCtx::new(LoaderConfig::moderate());
    let filter = engine::compile_filter("raw", &[]).unwrap();
    let documents = yamalgam_query::parse_str("[1]\n").unwrap();
    let result = engine::run_filter(
        &filter,
        &context,
        documents.into_iter().map(Ok::<_, String>),
        |_| Ok::<_, yamalgam_query::YgError>(()),
    );

    assert!(result.is_err());
}

#[test]
fn schema_filters_reresolve_scalar_text_explicitly() {
    assert_eq!(eval("schema", "null\n"), vec!["core"]);
    assert_eq!(eval(r#""yes" | resolve_as("1.1")"#, "null\n"), vec!["true"]);
    assert_eq!(
        eval(r#""yes" | resolve_as("1.1") | type"#, "null\n"),
        vec!["boolean"]
    );
    assert_eq!(eval(".a | resolve", "a: '123'\n"), vec!["123"]);
    assert_eq!(eval(".a | resolve | type", "a: '123'\n"), vec!["integer"]);
}

#[test]
fn input_context_filters_read_run_metadata() {
    let context = RunCtx::new(LoaderConfig::moderate());
    {
        let mut metadata = context.input_meta.borrow_mut();
        metadata.filename = Some("test.yml".into());
        metadata.document_index = 2;
        metadata.file_index = 3;
    }
    let filter = engine::compile_filter("[filename, document_index, file_index]", &[]).unwrap();
    let mut output = Vec::new();
    engine::run_filter(
        &filter,
        &context,
        std::iter::once(Ok(YamlVal::null())),
        |value| {
            output.push(value.to_string());
            Ok::<_, yamalgam_query::YgError>(())
        },
    )
    .unwrap();

    assert_eq!(output, vec!["[test.yml, 2, 3]"]);
}

#[test]
fn alias_audit_filters_report_and_strip_identity() {
    let document = "a: &x 1\nb: *x\nc: &unused 2\n";

    assert_eq!(eval("aliases", document), vec!["{x: [[b]]}"]);
    assert_eq!(eval("orphan_anchors", document), vec!["[unused]"]);
    assert_eq!(
        eval("explode_aliases | .b | is_alias", document),
        vec!["false"]
    );
    assert_eq!(
        eval("explode_aliases | .a | anchor", document),
        vec!["null"]
    );
}

#[test]
fn jq_18_compatibility_definitions_cover_manual_examples() {
    assert_eq!(eval("have_decnum", "null\n"), vec!["false"]);
    assert_eq!(eval("setpath([0, \"a\"]; 1)", "null\n"), vec!["[{a: 1}]"]);
    assert_eq!(
        eval("pick(.[2], .[0], .[0])", "[1, 2, 3, 4]\n"),
        vec!["[1, null, 3]"]
    );
    assert_eq!(
        eval(
            r#"[.[] | trimstr("foo")]"#,
            "[fo, foo, barfoo, foobarfoo, foob]\n"
        ),
        vec!["[fo, \"\", bar, bar, b]"]
    );
    assert_eq!(eval(r#"join(", ")"#, "{a: 1, b: 2}\n"), vec!["1, 2"]);
    assert_eq!(eval(r#"[scan("c")]"#, "abcdefabc\n"), vec!["[c, c]"]);
}

#[test]
fn assignment_operators_update_paths_and_collections() {
    assert_eq!(eval(".a = 9", "a: 1\nb: 2\n"), vec!["{a: 9, b: 2}"]);
    assert_eq!(eval(".a |= . + 1", "a: 1\n"), vec!["{a: 2}"]);
    assert_eq!(eval(".a += 5", "a: 1\n"), vec!["{a: 6}"]);
    assert_eq!(eval(".a //= 9", "a: null\n"), vec!["{a: 9}"]);
    assert_eq!(eval(".a.b.c = 1", "a: {}\n"), vec!["{a: {b: {c: 1}}}"]);
    assert_eq!(eval(".[] |= . * 2", "[1, 2]\n"), vec!["[2, 4]"]);
    assert_eq!(eval("del(.a)", "a: 1\nb: 2\n"), vec!["{b: 2}"]);
    assert_eq!(eval("del(.b)", "a: 1\nb: 2\nc: 3\n"), vec!["{a: 1, c: 3}"]);
    assert_eq!(eval(".[1:2] = [9, 9]", "[1, 2, 3]\n"), vec!["[1, 9, 9, 3]"]);
}

#[test]
fn assignment_preserves_untouched_sibling_metadata() {
    let source = "a: 1\nb: \"quoted\"\nflow: {x: 1}\n";
    let filter = engine::compile_filter(".a = 2", &[]).unwrap();
    let context = RunCtx::new(LoaderConfig::moderate());
    let documents = yamalgam_query::parse_str(source).unwrap();
    let mut output = Vec::new();
    engine::run_filter(
        &filter,
        &context,
        documents.into_iter().map(Ok::<_, String>),
        |value| {
            output.push(value);
            Ok::<_, yamalgam_query::YgError>(())
        },
    )
    .unwrap();

    assert_eq!(
        yamalgam_query::emit::to_yaml(&output[0], &Default::default()).unwrap(),
        "a: 2\nb: \"quoted\"\nflow: {x: 1}\n"
    );
}

#[test]
fn assignment_through_alias_emits_the_mutated_value() {
    assert_eq!(
        eval_yaml(".b.p = 2", "a: &x {p: 1}\nb: *x\n"),
        vec!["{a: &x {p: 1}, b: {p: 2}}"]
    );
}

#[test]
fn string_splice_invalidates_stale_raw_text() {
    assert_eq!(
        eval(r#".a |= .[0:1] = "z" | .a | raw"#, "a: \"abc\"\n"),
        vec!["zbc"]
    );
}

#[test]
fn metadata_mutation_filters_update_and_clear_properties() {
    assert_eq!(
        eval(".a |= set_tag(\"!special\") | .a | tag", "a: 1\n"),
        vec!["\"!special\""]
    );
    assert_eq!(eval("set_tag(null) | tag", "!x v\n"), vec!["null"]);
    assert_eq!(eval("set_tag(\"!!str\") | tag", "1\n"), vec!["\"!!str\""]);
    assert_eq!(
        eval_yaml(".a |= set_anchor(\"base\")", "a: {x: 1}\nb: 2\n"),
        vec!["{a: &base {x: 1}, b: 2}"]
    );
    assert_eq!(
        eval_yaml(".a |= set_style(\"double-quoted\")", "a: hi\n"),
        vec!["{a: \"hi\"}"]
    );
    assert_eq!(
        eval_yaml(".a |= set_style(\"flow\")", "a:\n  - 1\n"),
        vec!["{a: [1]}"]
    );
}

#[test]
fn set_style_rejects_node_kind_mismatches() {
    let filter = engine::compile_filter("set_style(\"flow\")", &[]).unwrap();
    let context = RunCtx::new(LoaderConfig::moderate());
    let documents = yamalgam_query::parse_str("scalar\n").unwrap();
    let result = engine::run_filter(
        &filter,
        &context,
        documents.into_iter().map(Ok::<_, String>),
        |_| Ok::<_, yamalgam_query::YgError>(()),
    );
    assert!(result.is_err());
}

#[test]
fn set_anchor_rejects_invalid_names() {
    let filter = engine::compile_filter("set_anchor(\"not valid\")", &[]).unwrap();
    let context = RunCtx::new(LoaderConfig::moderate());
    let documents = yamalgam_query::parse_str("scalar\n").unwrap();
    let result = engine::run_filter(
        &filter,
        &context,
        documents.into_iter().map(Ok::<_, String>),
        |_| Ok::<_, yamalgam_query::YgError>(()),
    );
    assert!(result.is_err());
}

#[test]
fn merge_expands_merge_keys_with_yaml_precedence_and_order() {
    let document = "base: &b {x: 1, y: 1}\nchild:\n  <<: *b\n  y: 2\n";
    assert_eq!(eval(".child | merge", document), vec!["{x: 1, y: 2}"]);

    let document = "a: &a {x: 1}\nb: &b {x: 2, y: 9}\nc:\n  <<: [*a, *b]\n";
    assert_eq!(eval(".c | merge", document), vec!["{x: 1, y: 9}"]);

    let document = "base: &b {x: 1, y: 1}\nchild:\n  y: 2\n  <<: *b\n";
    assert_eq!(eval(".child | merge", document), vec!["{y: 2, x: 1}"]);
}

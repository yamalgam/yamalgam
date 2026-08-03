//! End-to-end tests for jaq filter compilation and execution.

use yamalgam_core::LoaderConfig;
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

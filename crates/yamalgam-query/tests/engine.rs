//! End-to-end tests for jaq filter compilation and execution.

use yamalgam_core::LoaderConfig;
use yamalgam_query::YamlVal;
use yamalgam_query::engine::{self, InputMeta, RunCtx};

fn eval(filter: &str, input: &str) -> Vec<String> {
    let context = RunCtx::new(LoaderConfig::moderate());
    let compiled = engine::compile_filter(filter, &[]).unwrap();
    let documents = yamalgam_query::parse_str(input).unwrap();
    let mut output = Vec::new();
    engine::run_filter(
        &compiled,
        &context,
        documents.into_iter().map(Ok::<_, yamalgam_query::YgError>),
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
        documents.into_iter().map(Ok::<_, yamalgam_query::YgError>),
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
fn integral_float_indices_match_equal_integer_indices() {
    assert_eq!(eval(".[1.0]", "[1, 2, 3]\n"), vec!["2"]);
    assert_eq!(eval(".[1.0] = 9", "[1, 2, 3]\n"), vec!["[1, 9, 3]"]);
    assert_eq!(eval(".[1.0:3.0]", "[1, 2, 3, 4]\n"), vec!["[2, 3]"]);
    assert_eq!(eval("has(1.0)", "[1, 2, 3]\n"), vec!["true"]);
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
fn sandboxed_compilation_omits_environment_access() {
    let error =
        engine::compile_filter_with_options("env", &[], engine::CompileOptions::sandboxed())
            .err()
            .expect("sandboxed filters must not expose env");

    assert!(matches!(error, yamalgam_query::YgError::FilterCompile(_)));
}

#[test]
fn run_filter_accepts_borrowed_iterators_and_typed_input_errors() {
    let context = RunCtx::new(LoaderConfig::moderate());
    let filter = engine::compile_filter(".", &[]).unwrap();
    let documents = yamalgam_query::parse_str("1\n").unwrap();
    let mut output = Vec::new();

    engine::run_filter(
        &filter,
        &context,
        documents
            .iter()
            .cloned()
            .map(Ok::<_, yamalgam_query::BuildError>),
        |value| {
            output.push(value);
            Ok::<_, yamalgam_query::YgError>(())
        },
    )
    .unwrap();
    assert_eq!(output, vec![YamlVal::int(1)]);

    let error = yamalgam_query::parse_str("a: *missing\n").unwrap_err();
    let result = engine::run_filter(&filter, &context, std::iter::once(Err(error)), |_| {
        Ok::<_, yamalgam_query::YgError>(())
    });
    assert!(matches!(
        result,
        Err(yamalgam_query::YgError::InputParse(_))
    ));

    let result = engine::run_filter(
        &filter,
        &context,
        std::iter::once(Err(yamalgam_query::YgError::InputRead(
            "cannot read input".into(),
        ))),
        |_| Ok::<_, yamalgam_query::YgError>(()),
    );
    assert!(matches!(result, Err(yamalgam_query::YgError::InputRead(_))));
}

#[test]
fn native_input_filter_preserves_typed_input_errors() {
    let context = RunCtx::new(LoaderConfig::moderate());
    let filter = engine::compile_filter("input", &[]).unwrap();
    let result = engine::run_filter_with_null_input(
        &filter,
        &context,
        std::iter::once(Err(yamalgam_query::YgError::InputRead(
            "cannot read native input".into(),
        ))),
        true,
        |_| Ok::<_, yamalgam_query::YgError>(()),
    );

    assert!(matches!(result, Err(yamalgam_query::YgError::InputRead(_))));
}

#[test]
fn caught_input_errors_do_not_reclassify_unrelated_runtime_errors() {
    let context = RunCtx::new(LoaderConfig::moderate());
    let filter = engine::compile_filter(r#"try input catch error("other")"#, &[]).unwrap();
    let result = engine::run_filter_with_null_input(
        &filter,
        &context,
        std::iter::once(Err(yamalgam_query::YgError::InputRead(
            "cannot read native input".into(),
        ))),
        true,
        |_| Ok::<_, yamalgam_query::YgError>(()),
    );

    assert!(matches!(
        result,
        Err(yamalgam_query::YgError::Runtime(message)) if message.contains("other")
    ));
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
        documents.into_iter().map(Ok::<_, yamalgam_query::YgError>),
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
    context.stage_input_meta(InputMeta {
        filename: Some("test.yml".into()),
        document_index: 2,
        file_index: 3,
    });
    let filter = engine::compile_filter("[filename, document_index, file_index]", &[]).unwrap();
    let mut output = Vec::new();
    engine::run_filter(
        &filter,
        &context,
        std::iter::once(Ok::<_, yamalgam_query::YgError>(YamlVal::null())),
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
fn explode_aliases_obeys_alias_expansion_limit() {
    let document = "a: &a [1]\nb: &b [*a, *a]\nc: [*b, *b]\n";
    let filter = engine::compile_filter("explode_aliases", &[]).unwrap();
    let mut config = LoaderConfig::moderate();
    config.limits.max_alias_expansions = Some(2);
    config.limits.max_collection_size = None;
    let context = RunCtx::new(config);
    let documents = yamalgam_query::parse_str(document).unwrap();

    let error = engine::run_filter(
        &filter,
        &context,
        documents.into_iter().map(Ok::<_, yamalgam_query::YgError>),
        |_| Ok::<_, yamalgam_query::YgError>(()),
    )
    .unwrap_err();

    assert!(error.to_string().contains("alias expansions"));
}

#[test]
fn explode_aliases_obeys_materialized_entry_budget() {
    let filter = engine::compile_filter("explode_aliases", &[]).unwrap();
    let mut config = LoaderConfig::moderate();
    config.limits.max_alias_expansions = None;
    config.limits.max_collection_size = Some(2);
    let context = RunCtx::new(config);
    let documents = yamalgam_query::parse_str("[[1, 2], [3, 4]]\n").unwrap();

    let error = engine::run_filter(
        &filter,
        &context,
        documents.into_iter().map(Ok::<_, yamalgam_query::YgError>),
        |_| Ok::<_, yamalgam_query::YgError>(()),
    )
    .unwrap_err();

    assert!(error.to_string().contains("collection size"));
}

#[test]
fn explode_aliases_obeys_depth_limit() {
    let filter = engine::compile_filter("explode_aliases", &[]).unwrap();
    let mut config = LoaderConfig::moderate();
    config.limits.max_alias_expansions = None;
    config.limits.max_collection_size = None;
    config.limits.max_depth = Some(1);
    let context = RunCtx::new(config);
    let documents = yamalgam_query::parse_str("[[[1]]]\n").unwrap();

    let error = engine::run_filter(
        &filter,
        &context,
        documents.into_iter().map(Ok::<_, yamalgam_query::YgError>),
        |_| Ok::<_, yamalgam_query::YgError>(()),
    )
    .unwrap_err();

    assert!(error.to_string().contains("depth"));
}

fn runtime_limit_error(filter_source: &str, config: LoaderConfig) -> String {
    let filter = engine::compile_filter(filter_source, &[]).unwrap();
    let context = RunCtx::new(config);
    engine::run_filter(
        &filter,
        &context,
        core::iter::once(Ok::<_, yamalgam_query::YgError>(YamlVal::null())),
        |_| Ok::<_, yamalgam_query::YgError>(()),
    )
    .unwrap_err()
    .to_string()
}

#[test]
fn runtime_limits_bound_string_concatenation() {
    let mut config = LoaderConfig::moderate();
    config.limits.max_scalar_bytes = Some(7);

    let error = runtime_limit_error(r#""abcd" + "efgh""#, config);

    assert!(error.contains("scalar size"), "unexpected error: {error}");
}

#[test]
fn runtime_limits_bound_collection_growth() {
    for filter_source in [
        "[1, 2] + [3, 4]",
        "{a: 1, b: 2} + {c: 3, d: 4}",
        "[range(4)]",
        "{a: 1, b: 2, c: 3, d: 4}",
        "[1, 2] | map_values(., .)",
    ] {
        let mut config = LoaderConfig::moderate();
        config.limits.max_collection_size = Some(3);

        let error = runtime_limit_error(filter_source, config);

        assert!(
            error.contains("collection size"),
            "unexpected result for {filter_source}: {error}"
        );
    }
}

fn tostring_error(input: &str, config: LoaderConfig) -> (String, usize) {
    let filter = engine::compile_filter("tostring", &[]).unwrap();
    let context = RunCtx::new(config);
    let documents = yamalgam_query::parse_str(input).unwrap();
    let mut outputs = 0;
    let error = engine::run_filter(
        &filter,
        &context,
        documents.into_iter().map(Ok::<_, yamalgam_query::YgError>),
        |_| {
            outputs += 1;
            Ok::<_, yamalgam_query::YgError>(())
        },
    )
    .unwrap_err()
    .to_string();
    (error, outputs)
}

#[test]
fn tostring_propagates_json_conversion_errors() {
    let (error, outputs) = tostring_error("{1: x, \"1\": y}\n", LoaderConfig::moderate());

    assert_eq!(outputs, 0);
    assert!(error.contains("duplicated"), "unexpected error: {error}");
}

#[test]
fn tostring_uses_the_active_runtime_limits() {
    let mut config = LoaderConfig::moderate();
    config.limits.max_depth = Some(0);

    let (error, outputs) = tostring_error("[1]\n", config);

    assert_eq!(outputs, 0);
    assert!(error.contains("depth"), "unexpected error: {error}");
}

#[test]
fn runtime_error_display_obeys_output_limit() {
    let document = concat!(
        "a: &a [1234567890]\n",
        "b: &b [*a, *a, *a]\n",
        "c: &c [*b, *b, *b]\n",
        "d: [*c, *c, *c]\n",
    );
    let filter = engine::compile_filter(".d | merge", &[]).unwrap();
    let mut config = LoaderConfig::moderate();
    config.limits.max_alias_expansions = None;
    config.limits.max_collection_size = None;
    config.limits.max_output_bytes = Some(48);
    let context = RunCtx::new(config);
    let documents = yamalgam_query::parse_str(document).unwrap();

    let error = engine::run_filter(
        &filter,
        &context,
        documents.into_iter().map(Ok::<_, yamalgam_query::YgError>),
        |_| Ok::<_, yamalgam_query::YgError>(()),
    )
    .unwrap_err();
    let message = error.to_string();

    assert!(message.contains("..."), "untruncated error: {message}");
    assert!(
        message.len() <= 96,
        "oversized error: {} bytes",
        message.len()
    );
}

#[test]
fn runtime_error_display_obeys_depth_limit() {
    let filter = engine::compile_filter("merge", &[]).unwrap();
    let mut config = LoaderConfig::moderate();
    config.limits.max_depth = Some(2);
    config.limits.max_output_bytes = None;
    let context = RunCtx::new(config);
    let documents = yamalgam_query::parse_str("[[[[1]]]]\n").unwrap();

    let error = engine::run_filter(
        &filter,
        &context,
        documents.into_iter().map(Ok::<_, yamalgam_query::YgError>),
        |_| Ok::<_, yamalgam_query::YgError>(()),
    )
    .unwrap_err();

    assert!(error.to_string().contains("[[[...]]]"));
}

#[test]
fn alias_audit_includes_mapping_keys() {
    let document = "first:\n  ? &k key\n  : one\nsecond:\n  ? *k\n  : two\n";

    assert_eq!(
        eval("aliases", document),
        vec!["{k: [[second, {$key: key}]]}"]
    );
    assert_eq!(eval("orphan_anchors", document), vec!["[]"]);
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
        documents.into_iter().map(Ok::<_, yamalgam_query::YgError>),
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
        documents.into_iter().map(Ok::<_, yamalgam_query::YgError>),
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
        documents.into_iter().map(Ok::<_, yamalgam_query::YgError>),
        |_| Ok::<_, yamalgam_query::YgError>(()),
    );
    assert!(result.is_err());
}

#[test]
fn set_tag_rejects_tags_that_cannot_be_emitted_safely() {
    for filter_source in [
        r#"set_tag("not valid")"#,
        r#"set_tag("!x\ninjected: true")"#,
        r#"set_tag("!x,y")"#,
        r#"set_tag("!<broken")"#,
    ] {
        let filter = engine::compile_filter(filter_source, &[]).unwrap();
        let context = RunCtx::new(LoaderConfig::moderate());
        let documents = yamalgam_query::parse_str("scalar\n").unwrap();
        let result = engine::run_filter(
            &filter,
            &context,
            documents.into_iter().map(Ok::<_, yamalgam_query::YgError>),
            |_| Ok::<_, yamalgam_query::YgError>(()),
        );

        assert!(
            result.is_err(),
            "filter unexpectedly succeeded: {filter_source}"
        );
    }
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

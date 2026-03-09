#![allow(missing_docs)]

use yamalgam_compare::{
    CompareResult, SpanSnapshot, TokenSnapshot, compare_token_streams, extract_test_cases,
    parse_tree, run_rust_scanner,
};

#[test]
fn identical_streams_match() {
    let tokens = vec![
        TokenSnapshot {
            kind: "StreamStart".to_string(),
            value: None,
            style: None,
            span: SpanSnapshot::default(),
        },
        TokenSnapshot {
            kind: "StreamEnd".to_string(),
            value: None,
            style: None,
            span: SpanSnapshot::default(),
        },
    ];
    let result = compare_token_streams(&tokens, &tokens);
    assert!(matches!(result, CompareResult::Match { token_count: 2 }));
}

#[test]
fn different_values_produce_mismatch() {
    let c_tokens = vec![TokenSnapshot {
        kind: "Scalar".to_string(),
        value: Some("foo".to_string()),
        style: Some("Plain".to_string()),
        span: SpanSnapshot::default(),
    }];
    let rust_tokens = vec![TokenSnapshot {
        kind: "Scalar".to_string(),
        value: Some("bar".to_string()),
        style: Some("Plain".to_string()),
        span: SpanSnapshot::default(),
    }];
    let result = compare_token_streams(&c_tokens, &rust_tokens);
    assert!(matches!(
        result,
        CompareResult::TokenMismatch { index: 0, .. }
    ));
}

#[test]
fn different_lengths_produce_mismatch() {
    let c_tokens = vec![
        TokenSnapshot {
            kind: "StreamStart".to_string(),
            value: None,
            style: None,
            span: SpanSnapshot::default(),
        },
        TokenSnapshot {
            kind: "StreamEnd".to_string(),
            value: None,
            style: None,
            span: SpanSnapshot::default(),
        },
    ];
    let rust_tokens = vec![TokenSnapshot {
        kind: "StreamStart".to_string(),
        value: None,
        style: None,
        span: SpanSnapshot::default(),
    }];
    let result = compare_token_streams(&c_tokens, &rust_tokens);
    assert!(matches!(result, CompareResult::TokenMismatch { .. }));
}

#[test]
fn different_kinds_produce_mismatch() {
    let c_tokens = vec![TokenSnapshot {
        kind: "Scalar".to_string(),
        value: None,
        style: None,
        span: SpanSnapshot::default(),
    }];
    let rust_tokens = vec![TokenSnapshot {
        kind: "Anchor".to_string(),
        value: None,
        style: None,
        span: SpanSnapshot::default(),
    }];
    let result = compare_token_streams(&c_tokens, &rust_tokens);
    assert!(matches!(
        result,
        CompareResult::TokenMismatch { index: 0, .. }
    ));
}

#[test]
fn rust_scanner_produces_stream_and_block_tokens() {
    let tokens = run_rust_scanner(b"key: value").unwrap();
    assert_eq!(tokens[0].kind, "StreamStart");
    assert!(tokens.iter().any(|t| t.kind == "Value"));
    assert_eq!(tokens.last().unwrap().kind, "StreamEnd");
}

// ---------------------------------------------------------------------------
// tree_format tests
// ---------------------------------------------------------------------------

#[test]
fn tree_simple_mapping() {
    let tree = "\
+STR
 +DOC
  +MAP
   =VAL :name
   =VAL :Mark McGwire
   =VAL :hr
   =VAL :65
  -MAP
 -DOC
-STR
";
    let events = parse_tree(tree);
    assert_eq!(events.len(), 10);
    assert_eq!(events[0].kind, "StreamStart");
    assert_eq!(events[1].kind, "DocumentStart");
    assert_eq!(events[1].implicit, Some(true));
    assert_eq!(events[2].kind, "MappingStart");
    assert_eq!(events[3].kind, "Scalar");
    assert_eq!(events[3].value.as_deref(), Some("name"));
    assert_eq!(events[4].value.as_deref(), Some("Mark McGwire"));
    assert_eq!(events[5].value.as_deref(), Some("hr"));
    assert_eq!(events[6].value.as_deref(), Some("65"));
    assert_eq!(events[7].kind, "MappingEnd");
    assert_eq!(events[8].kind, "DocumentEnd");
    assert_eq!(events[8].implicit, Some(true));
    assert_eq!(events[9].kind, "StreamEnd");
}

#[test]
fn tree_explicit_doc_markers() {
    let tree = "\
+STR
 +DOC ---
  =VAL :hello
 -DOC ...
-STR
";
    let events = parse_tree(tree);
    assert_eq!(events.len(), 5);
    assert_eq!(events[1].kind, "DocumentStart");
    assert_eq!(events[1].implicit, Some(false));
    assert_eq!(events[3].kind, "DocumentEnd");
    assert_eq!(events[3].implicit, Some(false));
}

#[test]
fn tree_implicit_doc_markers() {
    let tree = "\
+STR
 +DOC
  =VAL :hello
 -DOC
-STR
";
    let events = parse_tree(tree);
    assert_eq!(events[1].implicit, Some(true));
    assert_eq!(events[3].implicit, Some(true));
}

#[test]
fn tree_anchors_on_collections() {
    let tree = "\
+STR
 +DOC
  +MAP &node3
   =VAL :key
   =VAL :val
  -MAP
 -DOC
-STR
";
    let events = parse_tree(tree);
    let map_start = &events[2];
    assert_eq!(map_start.kind, "MappingStart");
    assert_eq!(map_start.anchor.as_deref(), Some("node3"));
    assert!(map_start.tag.is_none());
}

#[test]
fn tree_tags_on_collections() {
    let tree = "\
+STR
 +DOC ---
  +MAP <tag:yaml.org,2002:set>
   =VAL :key
   =VAL :
  -MAP
 -DOC
-STR
";
    let events = parse_tree(tree);
    let map_start = &events[2];
    assert_eq!(map_start.kind, "MappingStart");
    assert!(map_start.anchor.is_none());
    assert_eq!(map_start.tag.as_deref(), Some("tag:yaml.org,2002:set"));
}

#[test]
fn tree_anchor_and_tag_on_collection() {
    let tree = "\
+STR
 +DOC
  +MAP &a4 <tag:yaml.org,2002:map>
  -MAP
 -DOC
-STR
";
    let events = parse_tree(tree);
    let map_start = &events[2];
    assert_eq!(map_start.anchor.as_deref(), Some("a4"));
    assert_eq!(map_start.tag.as_deref(), Some("tag:yaml.org,2002:map"));
}

#[test]
fn tree_sequence_with_anchor() {
    let tree = "\
+STR
 +DOC
  +SEQ &items
   =VAL :one
   =VAL :two
  -SEQ
 -DOC
-STR
";
    let events = parse_tree(tree);
    let seq_start = &events[2];
    assert_eq!(seq_start.kind, "SequenceStart");
    assert_eq!(seq_start.anchor.as_deref(), Some("items"));
}

#[test]
fn tree_alias() {
    let tree = "\
+STR
 +DOC
  +MAP
   =ALI *alias1
   =VAL :scalar3
  -MAP
 -DOC
-STR
";
    let events = parse_tree(tree);
    let alias = &events[3];
    assert_eq!(alias.kind, "Alias");
    assert_eq!(alias.value.as_deref(), Some("alias1"));
    assert!(alias.anchor.is_none());
    assert!(alias.tag.is_none());
}

#[test]
fn tree_alias_with_colon_in_name() {
    // From test case 2SXE: anchors with colon in name.
    let tree = "=ALI *a:\n";
    let events = parse_tree(tree);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "Alias");
    assert_eq!(events[0].value.as_deref(), Some("a:"));
}

#[test]
fn tree_scalar_plain_style() {
    let events = parse_tree("=VAL :hello world\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "Scalar");
    assert_eq!(events[0].value.as_deref(), Some("hello world"));
}

#[test]
fn tree_scalar_single_quoted() {
    let events = parse_tree("=VAL 'hello world\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].value.as_deref(), Some("hello world"));
}

#[test]
fn tree_scalar_double_quoted() {
    let events = parse_tree("=VAL \"hello world\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].value.as_deref(), Some("hello world"));
}

#[test]
fn tree_scalar_literal_block() {
    let events = parse_tree("=VAL |hello\\nworld\\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].value.as_deref(), Some("hello\nworld\n"));
}

#[test]
fn tree_scalar_folded_block() {
    let events = parse_tree("=VAL >hello\\nworld\\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].value.as_deref(), Some("hello\nworld\n"));
}

#[test]
fn tree_scalar_empty_value() {
    let events = parse_tree("=VAL :\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].value.as_deref(), Some(""));
}

#[test]
fn tree_scalar_with_anchor() {
    let events = parse_tree("=VAL &alias1 :scalar1\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].anchor.as_deref(), Some("alias1"));
    assert_eq!(events[0].value.as_deref(), Some("scalar1"));
    assert!(events[0].tag.is_none());
}

#[test]
fn tree_scalar_with_tag() {
    let events = parse_tree("=VAL <tag:yaml.org,2002:int> :42\n");
    assert_eq!(events.len(), 1);
    assert!(events[0].anchor.is_none());
    assert_eq!(events[0].tag.as_deref(), Some("tag:yaml.org,2002:int"));
    assert_eq!(events[0].value.as_deref(), Some("42"));
}

#[test]
fn tree_scalar_with_anchor_and_tag() {
    let events = parse_tree("=VAL &a1 <tag:yaml.org,2002:str> :scalar1\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].anchor.as_deref(), Some("a1"));
    assert_eq!(events[0].tag.as_deref(), Some("tag:yaml.org,2002:str"));
    assert_eq!(events[0].value.as_deref(), Some("scalar1"));
}

#[test]
fn tree_escape_sequences() {
    // \n → newline, \t → tab, \\ → backslash, \r → CR, \b → BS, \0 → null
    let events = parse_tree("=VAL :a\\nb\\tc\\\\d\\re\\bf\\0g\n");
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].value.as_deref(),
        Some("a\nb\tc\\d\re\u{0008}f\0g")
    );
}

#[test]
fn tree_hex_escape() {
    let events = parse_tree("=VAL :start\\x41end\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].value.as_deref(), Some("startAend"));
}

#[test]
fn tree_literal_backslash_n_in_value() {
    // \\n in tree format → literal backslash + n in the value (not a newline).
    let events = parse_tree("=VAL 'x\\\\ny\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].value.as_deref(), Some("x\\ny"));
}

#[test]
fn tree_full_229q_example() {
    // Spec Example 2.4: Sequence of Mappings — from the YAML Test Suite.
    let tree = "\
+STR
 +DOC
  +SEQ
   +MAP
    =VAL :name
    =VAL :Mark McGwire
    =VAL :hr
    =VAL :65
    =VAL :avg
    =VAL :0.278
   -MAP
   +MAP
    =VAL :name
    =VAL :Sammy Sosa
    =VAL :hr
    =VAL :63
    =VAL :avg
    =VAL :0.288
   -MAP
  -SEQ
 -DOC
-STR
";
    let events = parse_tree(tree);
    assert_eq!(events.len(), 22);

    // Spot-check structure.
    assert_eq!(events[0].kind, "StreamStart");
    assert_eq!(events[2].kind, "SequenceStart");
    assert_eq!(events[3].kind, "MappingStart");
    assert_eq!(events[4].value.as_deref(), Some("name"));
    assert_eq!(events[5].value.as_deref(), Some("Mark McGwire"));
    assert_eq!(events[10].kind, "MappingEnd");
    assert_eq!(events[11].kind, "MappingStart");
    assert_eq!(events[18].kind, "MappingEnd");
    assert_eq!(events[19].kind, "SequenceEnd");
    assert_eq!(events[21].kind, "StreamEnd");
}

#[test]
fn tree_blank_lines_ignored() {
    let tree = "\n+STR\n\n +DOC\n\n-STR\n\n";
    let events = parse_tree(tree);
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].kind, "StreamStart");
    assert_eq!(events[1].kind, "DocumentStart");
    assert_eq!(events[2].kind, "StreamEnd");
}

#[test]
fn tree_anchor_with_colon_on_scalar() {
    // From test case 2SXE: anchor with colon.
    let events = parse_tree("=VAL &a: :key\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].anchor.as_deref(), Some("a:"));
    assert_eq!(events[0].value.as_deref(), Some("key"));
}

#[test]
fn tree_tagged_scalars() {
    // Multiple tagged scalars as in 33X3.
    let tree = "\
+STR
 +DOC ---
  +SEQ
   =VAL <tag:yaml.org,2002:int> :1
   =VAL <tag:yaml.org,2002:int> :-2
   =VAL <tag:yaml.org,2002:int> :33
  -SEQ
 -DOC
-STR
";
    let events = parse_tree(tree);
    assert_eq!(events.len(), 9);
    assert_eq!(events[3].tag.as_deref(), Some("tag:yaml.org,2002:int"));
    assert_eq!(events[3].value.as_deref(), Some("1"));
    assert_eq!(events[4].value.as_deref(), Some("-2"));
    assert_eq!(events[5].value.as_deref(), Some("33"));
}

#[test]
fn tree_literal_block_with_escapes() {
    // Literal block scalar: `|ab\n\n \n` → "ab\n\n \n"
    let events = parse_tree("=VAL |ab\\n\\n \\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].value.as_deref(), Some("ab\n\n \n"));
}

#[test]
fn tree_complex_26dv_example() {
    // From 26DV — anchors, aliases, mixed scalar styles.
    let tree = "\
+STR
 +DOC
  +MAP
   =VAL \"top1
   +MAP
    =VAL \"key1
    =VAL &alias1 :scalar1
   -MAP
   =VAL 'top2
   +MAP
    =VAL 'key2
    =VAL &alias2 :scalar2
   -MAP
   =VAL :top3
   +MAP &node3
    =ALI *alias1
    =VAL :scalar3
   -MAP
   =VAL :top4
   +MAP
    =ALI *alias2
    =VAL :scalar4
   -MAP
   =VAL :top5
   =VAL :scalar5
   =VAL :top6
   +MAP
    =VAL &anchor6 'key6
    =VAL :scalar6
   -MAP
  -MAP
 -DOC
-STR
";
    let events = parse_tree(tree);

    // 33 events: +STR +DOC +MAP (outer)
    //   =VAL"top1 +MAP =VAL"key1 =VAL&alias1:scalar1 -MAP
    //   =VAL'top2 +MAP =VAL'key2 =VAL&alias2:scalar2 -MAP
    //   =VAL:top3 +MAP&node3 =ALI*alias1 =VAL:scalar3 -MAP
    //   =VAL:top4 +MAP =ALI*alias2 =VAL:scalar4 -MAP
    //   =VAL:top5 =VAL:scalar5
    //   =VAL:top6 +MAP =VAL&anchor6'key6 =VAL:scalar6 -MAP
    //   -MAP (outer) -DOC -STR
    assert_eq!(events.len(), 33);

    // Check double-quoted scalar.
    assert_eq!(events[3].value.as_deref(), Some("top1"));

    // Check anchor on scalar value.
    let alias1_val = &events[6];
    assert_eq!(alias1_val.anchor.as_deref(), Some("alias1"));
    assert_eq!(alias1_val.value.as_deref(), Some("scalar1"));

    // Check single-quoted scalar.
    assert_eq!(events[8].value.as_deref(), Some("top2"));

    // Check anchor on mapping.
    let map_node3 = &events[14];
    assert_eq!(map_node3.kind, "MappingStart");
    assert_eq!(map_node3.anchor.as_deref(), Some("node3"));

    // Check alias.
    let ali1 = &events[15];
    assert_eq!(ali1.kind, "Alias");
    assert_eq!(ali1.value.as_deref(), Some("alias1"));

    // Check anchor on single-quoted scalar value.
    let anchor6_val = &events[27];
    assert_eq!(anchor6_val.anchor.as_deref(), Some("anchor6"));
    assert_eq!(anchor6_val.value.as_deref(), Some("key6"));
}

// ---------------------------------------------------------------------------
// test_case extractor tests
// ---------------------------------------------------------------------------

#[test]
fn extract_single_case() {
    let content = "\
---
- name: Simple test
  yaml: |
    key: value
  tree: |
    +STR
     +DOC
      +MAP
       =VAL :key
       =VAL :value
      -MAP
     -DOC
    -STR
";
    let cases = extract_test_cases(content);
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].index, 0);
    assert_eq!(cases[0].yaml, "key: value");
    assert!(!cases[0].fail);
    let tree = cases[0].tree.as_ref().unwrap();
    assert!(tree.starts_with("+STR"));
    assert!(tree.contains("-STR"));
}

#[test]
fn extract_multi_case() {
    let content = "\
---
- name: First
  yaml: |
    a: 1
  tree: |
    +STR
    -STR

- yaml: |
    b: 2
  tree: |
    +STR
    -STR

- yaml: |
    c: 3
  tree: |
    +STR
    -STR
";
    let cases = extract_test_cases(content);
    assert_eq!(cases.len(), 3);
    assert_eq!(cases[0].index, 0);
    assert_eq!(cases[0].yaml, "a: 1");
    assert_eq!(cases[1].index, 1);
    assert_eq!(cases[1].yaml, "b: 2");
    assert_eq!(cases[2].index, 2);
    assert_eq!(cases[2].yaml, "c: 3");
}

#[test]
fn extract_fail_true_as_first_key() {
    let content = "\
---
- fail: true
  yaml: |
    bad: [
  tree: |
    +STR
";
    let cases = extract_test_cases(content);
    assert_eq!(cases.len(), 1);
    assert!(cases[0].fail);
    assert_eq!(cases[0].yaml, "bad: [");
}

#[test]
fn extract_fail_true_as_indented_key() {
    let content = "\
---
- name: Error case
  fail: true
  yaml: |
    broken
  tree: |
    +STR
";
    let cases = extract_test_cases(content);
    assert_eq!(cases.len(), 1);
    assert!(cases[0].fail);
}

#[test]
fn extract_fail_defaults_false() {
    let content = "\
---
- yaml: |
    ok: true
  tree: |
    +STR
    -STR
";
    let cases = extract_test_cases(content);
    assert_eq!(cases.len(), 1);
    assert!(!cases[0].fail);
}

#[test]
fn extract_tree_field() {
    let content = "\
---
- yaml: |
    hello
  tree: |
    +STR
     +DOC
      =VAL :hello
     -DOC
    -STR
";
    let cases = extract_test_cases(content);
    let tree = cases[0].tree.as_ref().unwrap();
    assert!(tree.starts_with("+STR"));
    assert!(tree.contains("=VAL :hello"));
    assert!(tree.ends_with("-STR"));
}

#[test]
fn extract_no_tree_field() {
    let content = "\
---
- yaml: |
    hello
";
    let cases = extract_test_cases(content);
    assert_eq!(cases.len(), 1);
    assert!(cases[0].tree.is_none());
}

#[test]
fn marker_replacement_in_yaml_field() {
    // \u{2014} = em-dash (removed), \u{00BB} = guillemet (tab)
    let content = "---\n- yaml: |\n    foo:\n     \u{2014}\u{2014}\u{2014}\u{00BB}bar\n";
    let cases = extract_test_cases(content);
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].yaml, "foo:\n \tbar");
}

#[test]
fn marker_replacement_open_box_to_space() {
    let content = "---\n- yaml: |\n    hello\u{2423}world\n";
    let cases = extract_test_cases(content);
    assert_eq!(cases[0].yaml, "hello world");
}

#[test]
fn marker_end_of_proof_strips_trailing_newline() {
    let content = "---\n- yaml: |\n    data\u{220E}\n";
    let cases = extract_test_cases(content);
    assert_eq!(cases[0].yaml, "data");
}

#[test]
fn extract_yaml_block_scalar_with_indent_indicator() {
    // yaml: |2 is treated the same as yaml: | for test-file extraction:
    // indent is auto-detected from first content line. This mirrors how
    // DK95 case 3 works in the real test suite.
    let content = "\
---
- yaml: |2
    leading
    foo: 1
  tree: |
    +STR
    -STR
";
    let cases = extract_test_cases(content);
    assert_eq!(cases.len(), 1);
    assert!(cases[0].yaml.contains("leading"));
    assert!(cases[0].yaml.contains("foo: 1"));
    // tree: should NOT be consumed into yaml
    assert!(cases[0].tree.is_some());
}

#[test]
fn extract_mixed_fail_and_pass_multi_case() {
    let content = "\
---
- name: Pass case
  yaml: |
    a: 1
  tree: |
    +STR
    -STR

- fail: true
  yaml: |
    broken
  tree: |
    +STR

- yaml: |
    c: 3
  tree: |
    +STR
    -STR
";
    let cases = extract_test_cases(content);
    assert_eq!(cases.len(), 3);
    assert!(!cases[0].fail);
    assert!(cases[1].fail);
    assert!(!cases[2].fail);
}

#[test]
fn extract_real_test_suite_file_229q() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vendor/yaml-test-suite/229Q.yaml"
    );
    let content = std::fs::read_to_string(path).expect("229Q.yaml should exist");
    let cases = extract_test_cases(&content);

    assert_eq!(cases.len(), 1);
    assert!(!cases[0].fail);
    assert!(cases[0].yaml.contains("name: Mark McGwire"));
    assert!(cases[0].yaml.contains("name: Sammy Sosa"));

    let tree = cases[0].tree.as_ref().expect("229Q should have a tree");
    assert!(tree.starts_with("+STR"));
    assert!(tree.contains("+SEQ"));
    assert!(tree.contains("+MAP"));
    assert!(tree.contains("=VAL :Mark McGwire"));
    assert!(tree.ends_with("-STR"));
}

#[test]
fn extract_real_test_suite_file_dk95_multi_case() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vendor/yaml-test-suite/DK95.yaml"
    );
    let content = std::fs::read_to_string(path).expect("DK95.yaml should exist");
    let cases = extract_test_cases(&content);

    assert!(
        cases.len() > 1,
        "DK95 should be multi-case, got {}",
        cases.len()
    );

    let fail_case = cases
        .iter()
        .find(|c| c.index == 1)
        .expect("should have index 1");
    assert!(fail_case.fail, "DK95 case 1 should be fail:true");
    assert!(!cases[0].fail, "DK95 case 0 should not be fail");

    for case in &cases {
        assert!(
            !case.yaml.is_empty(),
            "case {} should have yaml",
            case.index
        );
    }
    for case in &cases {
        assert!(case.tree.is_some(), "case {} should have tree", case.index);
    }
}

#[test]
fn extract_real_test_suite_file_sf5v_fail_only() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vendor/yaml-test-suite/SF5V.yaml"
    );
    let content = std::fs::read_to_string(path).expect("SF5V.yaml should exist");
    let cases = extract_test_cases(&content);

    assert_eq!(cases.len(), 1);
    assert!(cases[0].fail);
    assert!(cases[0].yaml.contains("%YAML 1.2"));

    let tree = cases[0].tree.as_ref().expect("SF5V should have a tree");
    assert!(tree.starts_with("+STR"));
}

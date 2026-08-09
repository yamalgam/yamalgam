//! Built-in resolution through the lossless CST builder.

use yamalgam_core::{LoaderConfig, ResolutionMode};
use yamalgam_cst::{CstElement, CstNode, TokenKind, parse_to_cst_report_with_resolver_and_config};
use yamalgam_resolve::{
    BuiltinResolver, MemoryRetriever, ResourceId, ResourceProvenance, RetrievedResource,
};

fn fixture() -> (BuiltinResolver<MemoryRetriever>, LoaderConfig) {
    let root = ResourceId::from_file("/fixtures/root.yaml");
    let shared = ResourceId::from_file("/fixtures/shared.yaml");
    let included = ResourceId::from_file("/fixtures/included.yaml");
    let mut retriever = MemoryRetriever::default();
    for (id, input) in [
        (shared, "service: {name: api, ports: [80, 443]}\n"),
        (included, "enabled: true\n"),
    ] {
        retriever.insert(RetrievedResource {
            id,
            bytes: input.as_bytes().to_vec(),
            provenance: ResourceProvenance::default(),
        });
    }
    let mut config = LoaderConfig::unchecked();
    config.resolution.include.mode = ResolutionMode::Automatic;
    config.resolution.refs.mode = ResolutionMode::Automatic;
    config.resolution.refs.allow_schemes = vec!["file".into()];
    let resolver = BuiltinResolver::new(retriever, root, config.clone());
    (resolver, config)
}

fn synthetic_scalar_count(root: &CstNode<'_>) -> usize {
    let mut count = 0;
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        for child in &node.children {
            match child {
                CstElement::Node(child) => pending.push(child),
                CstElement::Token(token)
                    if token.kind == TokenKind::Scalar && token.span.start == token.span.end =>
                {
                    count += 1;
                }
                CstElement::Token(_) => {}
            }
        }
    }
    count
}

#[test]
fn builtin_resolution_classifies_synthetic_nodes_without_losing_source() {
    let input = concat!(
        "from_ref:\n",
        "  $ref: ./shared.yaml#/service\n",
        "from_include: !include ./included.yaml\n",
    );
    let (resolver, config) = fixture();

    let report = parse_to_cst_report_with_resolver_and_config(input, resolver, &config);

    assert!(report.diagnostics.is_empty());
    assert_eq!(report.cst.to_text(), input);
    assert!(
        report
            .cst
            .to_text()
            .contains("$ref: ./shared.yaml#/service")
    );
    assert!(report.cst.to_text().contains("!include ./included.yaml"));
    assert!(synthetic_scalar_count(&report.cst) >= 7);
}

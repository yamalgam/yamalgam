#![allow(missing_docs)]

use std::path::Path;

use yamalgam_cst::{
    CstElement, CstNode, NodeKind, SyntaxKind, SyntaxTree, TokenKind, parse_to_cst_report,
};

#[path = "../../yamalgam-compare/src/test_case.rs"]
mod test_case;

fn legacy_kinds(root: &CstNode<'_>) -> Vec<SyntaxKind> {
    let mut kinds = Vec::new();
    let mut pending = vec![CstElementRef::Node(root)];
    while let Some(element) = pending.pop() {
        match element {
            CstElementRef::Node(node) => {
                kinds.push(match node.kind {
                    NodeKind::Stream => SyntaxKind::Stream,
                    NodeKind::Document => SyntaxKind::Document,
                    NodeKind::Mapping => SyntaxKind::Mapping,
                    NodeKind::MappingEntry => SyntaxKind::MappingEntry,
                    NodeKind::Sequence => SyntaxKind::Sequence,
                    NodeKind::SequenceEntry => SyntaxKind::SequenceEntry,
                    NodeKind::Error => SyntaxKind::Error,
                });
                pending.extend(node.children.iter().rev().map(CstElementRef::from));
            }
            CstElementRef::Token(kind) => kinds.push(match kind {
                TokenKind::Scalar => SyntaxKind::Scalar,
                TokenKind::Anchor => SyntaxKind::Anchor,
                TokenKind::Alias => SyntaxKind::Alias,
                TokenKind::Tag => SyntaxKind::Tag,
                TokenKind::Comment => SyntaxKind::Comment,
                TokenKind::Whitespace => SyntaxKind::Whitespace,
                _ => SyntaxKind::Indicator,
            }),
        }
    }
    kinds
}

enum CstElementRef<'a> {
    Node(&'a CstNode<'a>),
    Token(TokenKind),
}

impl<'a> From<&'a CstElement<'a>> for CstElementRef<'a> {
    fn from(value: &'a CstElement<'a>) -> Self {
        match value {
            CstElement::Node(node) => Self::Node(node),
            CstElement::Token(token) => Self::Token(token.kind),
        }
    }
}

#[test]
fn rowan_and_legacy_trees_cover_the_yaml_test_suite() {
    let suite = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/yaml-test-suite");
    let mut paths: Vec<_> = std::fs::read_dir(suite)
        .expect("YAML Test Suite should be vendored")
        .map(|entry| entry.expect("suite entry should be readable").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "yaml")
        })
        .collect();
    paths.sort();
    assert_eq!(paths.len(), 351);

    for path in paths {
        let content = std::fs::read_to_string(&path).expect("suite case should be UTF-8");
        for case in test_case::extract_test_cases(&content) {
            let legacy_report = parse_to_cst_report(&case.yaml);
            let legacy = legacy_report.cst;
            let syntax = SyntaxTree::parse(&case.yaml);
            assert_eq!(legacy.to_text(), case.yaml, "legacy: {}", path.display());
            assert_eq!(syntax.text(), case.yaml, "rowan: {}", path.display());

            if !case.fail && legacy_report.diagnostics.is_empty() && syntax.diagnostics().is_empty()
            {
                let rowan_kinds: Vec<_> = syntax
                    .root()
                    .descendants_with_tokens()
                    .map(|element| element.kind())
                    .collect();
                assert_eq!(
                    rowan_kinds,
                    legacy_kinds(&legacy),
                    "{}#{}",
                    path.display(),
                    case.index
                );
            }
        }
    }
}

#[test]
fn node_lookup_uses_token_boundaries_and_error_nodes() {
    let valid = SyntaxTree::parse("key: [one, two]\n");
    let sequence_offset = valid.text().find('[').unwrap();
    assert_eq!(
        valid.node_at_offset(sequence_offset).unwrap().kind(),
        SyntaxKind::Sequence
    );

    let malformed = SyntaxTree::parse("bad: ]\nafter: ok\n");
    let error_offset = malformed.text().find(']').unwrap();
    assert_eq!(
        malformed.node_at_offset(error_offset).unwrap().kind(),
        SyntaxKind::Error
    );
    assert!(
        malformed
            .node_at_offset(malformed.text().len() + 1)
            .is_none()
    );
}

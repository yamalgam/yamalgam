#![allow(missing_docs)]

use yamalgam_core::LoaderConfig;
use yamalgam_cst::{
    CstElement, CstNode, NodeKind, RecoveryOptions, TokenKind, parse_to_cst_recovering_with_config,
};
use yamalgam_parser::ParseError;

fn contains_scalar(root: &CstNode<'_>, expected: &str) -> bool {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        for child in &node.children {
            match child {
                CstElement::Node(child) => pending.push(child),
                CstElement::Token(token)
                    if token.kind == TokenKind::Scalar && token.text == expected =>
                {
                    return true;
                }
                CstElement::Token(_) => {}
            }
        }
    }
    false
}

fn count_nodes(root: &CstNode<'_>, expected: NodeKind) -> usize {
    let mut count = 0;
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        count += usize::from(node.kind == expected);
        pending.extend(node.children.iter().filter_map(|child| match child {
            CstElement::Node(child) => Some(child),
            CstElement::Token(_) => None,
        }));
    }
    count
}

fn recovering(input: &str) -> yamalgam_cst::RecoveryReport<'_> {
    parse_to_cst_recovering_with_config(input, &LoaderConfig::default(), RecoveryOptions::default())
}

#[test]
fn recovery_preserves_valid_syntax_before_and_after_a_bad_line() {
    let input = "before: one\nbad: ]\nafter: two\n";
    let report = recovering(input);

    assert_eq!(report.cst.to_text(), input);
    assert_eq!(report.diagnostics.len(), 1);
    assert!(contains_scalar(&report.cst, "before"));
    assert!(contains_scalar(&report.cst, "after"));
    assert!(count_nodes(&report.cst, NodeKind::Error) >= 1);
}

#[test]
fn recovery_reports_multiple_errors_across_documents() {
    let input = "---\nok: one\nbad: ]\n---\nsecond: ok\nbroken: }\nlast: yes\n";
    let report = recovering(input);

    assert_eq!(report.cst.to_text(), input);
    assert_eq!(report.diagnostics.len(), 2);
    assert!(contains_scalar(&report.cst, "second"));
    assert!(contains_scalar(&report.cst, "last"));
    assert!(
        report
            .diagnostics
            .windows(2)
            .all(|pair| pair[0].span().unwrap().start.offset < pair[1].span().unwrap().start.offset)
    );
}

#[test]
fn recovery_makes_progress_through_repeated_failures_and_unicode() {
    let input = "emoji: 🦀\nbad: ]\nbad: ]\nsnow: 雪\n";
    let report = recovering(input);

    assert_eq!(report.cst.to_text(), input);
    assert_eq!(report.diagnostics.len(), 2);
    assert!(contains_scalar(&report.cst, "雪"));
    assert!(report.diagnostics.iter().all(|error| {
        error
            .span()
            .is_some_and(|span| input.is_char_boundary(span.start.offset))
    }));
}

#[test]
fn recovery_handles_unterminated_scalar_at_eof() {
    let input = "ok: yes\nbad: \"unterminated";
    let report = recovering(input);

    assert_eq!(report.cst.to_text(), input);
    assert_eq!(report.diagnostics.len(), 1);
    assert!(matches!(report.diagnostics[0], ParseError::Scan(_)));
}

#[test]
fn diagnostic_cap_wraps_the_unparsed_remainder() {
    let input = "bad: ]\nbad: }\nafter: yes\n";
    let report = parse_to_cst_recovering_with_config(
        input,
        &LoaderConfig::default(),
        RecoveryOptions { max_diagnostics: 1 },
    );

    assert_eq!(report.cst.to_text(), input);
    assert_eq!(report.diagnostics.len(), 1);
    assert!(count_nodes(&report.cst, NodeKind::Error) >= 1);
}

#[test]
fn default_recovery_cap_is_sixty_four() {
    assert_eq!(RecoveryOptions::default().max_diagnostics, 64);
}

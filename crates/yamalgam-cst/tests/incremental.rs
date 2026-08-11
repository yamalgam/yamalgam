#![allow(missing_docs)]

use yamalgam_core::LoaderConfig;
use yamalgam_cst::{IncrementalDocument, ReparseOutcome, TextEdit};

fn document(input: &str) -> IncrementalDocument {
    IncrementalDocument::new(input, &LoaderConfig::default())
}

#[test]
fn applies_insert_replace_delete_and_reverse_ordered_batches() {
    let mut doc = document("first: one\nsecond: two\n");
    let outcome = doc
        .apply(
            &[
                TextEdit {
                    range: 19..22,
                    replacement: "three".to_string(),
                },
                TextEdit {
                    range: 0..0,
                    replacement: "zero: 0\n".to_string(),
                },
                TextEdit {
                    range: 7..10,
                    replacement: String::new(),
                },
            ],
            &LoaderConfig::default(),
        )
        .unwrap();

    assert_eq!(outcome, ReparseOutcome::Document);
    assert_eq!(doc.text(), "zero: 0\nfirst: \nsecond: three\n");
    assert_eq!(doc.syntax().text(), doc.text());
}

#[test]
fn converts_crlf_and_non_bmp_utf16_positions_both_directions() {
    let input = "a: 🦀\r\nb: 雪\r\n";
    let doc = document(input);
    let after_emoji = input.find('🦀').unwrap() + '🦀'.len_utf8();
    let snow = input.find('雪').unwrap();

    assert_eq!(doc.byte_offset_to_utf16(after_emoji), Some((0, 5)));
    assert_eq!(doc.utf16_to_byte_offset(0, 5), Some(after_emoji));
    assert_eq!(doc.utf16_to_byte_offset(0, 4), None);
    assert_eq!(doc.byte_offset_to_utf16(snow), Some((1, 3)));
    assert_eq!(doc.utf16_to_byte_offset(1, 3), Some(snow));
    assert_eq!(doc.utf16_to_byte_offset(9, 0), None);
    assert_eq!(doc.byte_offset_to_utf16(input.len() + 1), None);
}

#[test]
fn lsp_lines_ignore_unicode_separators_and_honor_carriage_returns() {
    let input = "a\u{2028}b\u{2029}c\rd\n";
    let doc = document(input);
    let b = input.find('b').unwrap();
    let c = input.find('c').unwrap();
    let d = input.find('d').unwrap();

    assert_eq!(doc.byte_offset_to_utf16(b), Some((0, 2)));
    assert_eq!(doc.byte_offset_to_utf16(c), Some((0, 4)));
    assert_eq!(doc.byte_offset_to_utf16(d), Some((1, 0)));
    assert_eq!(doc.utf16_to_byte_offset(0, 2), Some(b));
    assert_eq!(doc.utf16_to_byte_offset(0, 4), Some(c));
    assert_eq!(doc.utf16_to_byte_offset(1, 0), Some(d));
}

#[test]
fn reparses_one_document_when_markers_and_boundaries_are_stable() {
    let mut doc = document("---\na: one\n---\nb: two\n");
    let start = doc.text().find("one").unwrap();
    let outcome = doc
        .apply(
            &[TextEdit {
                range: start..start + 3,
                replacement: "uno".to_string(),
            }],
            &LoaderConfig::default(),
        )
        .unwrap();

    assert_eq!(outcome, ReparseOutcome::Document);
    assert_eq!(doc.text(), "---\na: uno\n---\nb: two\n");
}

#[test]
fn falls_back_for_marker_cross_document_and_recovery_boundary_edits() {
    let config = LoaderConfig::default();

    let mut marker = document("---\na: one\n---\nb: two\n");
    assert_eq!(
        marker
            .apply(
                &[TextEdit {
                    range: 0..3,
                    replacement: "...".to_string(),
                }],
                &config,
            )
            .unwrap(),
        ReparseOutcome::Full
    );

    let mut cross_document = document("---\na: one\n---\nb: two\n");
    let start = cross_document.text().find("one").unwrap();
    let end = cross_document.text().find("two").unwrap() + 3;
    assert_eq!(
        cross_document
            .apply(
                &[TextEdit {
                    range: start..end,
                    replacement: "value".to_string(),
                }],
                &config,
            )
            .unwrap(),
        ReparseOutcome::Full
    );

    let mut recovery = document("good: yes\nbad: ]\nafter: ok\n");
    let bad = recovery.text().find(']').unwrap();
    assert_eq!(
        recovery
            .apply(
                &[TextEdit {
                    range: bad..bad + 1,
                    replacement: "fixed".to_string(),
                }],
                &config,
            )
            .unwrap(),
        ReparseOutcome::Full
    );
}

#[test]
fn rejects_out_of_range_non_boundary_and_overlapping_edits() {
    let config = LoaderConfig::default();
    let mut doc = document("emoji: 🦀\n");
    let emoji = doc.text().find('🦀').unwrap();

    assert!(
        doc.apply(
            &[TextEdit {
                range: emoji + 1..emoji + 2,
                replacement: "x".to_string(),
            }],
            &config,
        )
        .is_err()
    );
    assert!(
        doc.apply(
            &[TextEdit {
                range: 0..100,
                replacement: String::new(),
            }],
            &config,
        )
        .is_err()
    );
    assert!(
        doc.apply(
            &[
                TextEdit {
                    range: 0..5,
                    replacement: String::new(),
                },
                TextEdit {
                    range: 4..6,
                    replacement: String::new(),
                },
            ],
            &config,
        )
        .is_err()
    );
}

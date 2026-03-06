#![allow(missing_docs)]

use std::borrow::Cow;

use yamalgam_core::{Mark, Span};
use yamalgam_scanner::{Atom, AtomFlags, Chomp, ScalarStyle, Token, TokenKind};

#[test]
fn token_has_kind_and_atom() {
    let token = Token {
        kind: TokenKind::Scalar,
        atom: Atom {
            data: Cow::Borrowed("hello"),
            span: Span {
                start: Mark {
                    line: 0,
                    column: 0,
                    offset: 0,
                },
                end: Mark {
                    line: 0,
                    column: 5,
                    offset: 5,
                },
            },
            style: ScalarStyle::Plain,
            chomp: Chomp::Clip,
            flags: AtomFlags::empty(),
        },
    };
    assert_eq!(token.kind, TokenKind::Scalar);
    assert_eq!(token.atom.data.as_ref(), "hello");
}

#[test]
fn atom_flags_compose() {
    let flags = AtomFlags::HAS_LB | AtomFlags::HAS_WS;
    assert!(flags.contains(AtomFlags::HAS_LB));
    assert!(flags.contains(AtomFlags::HAS_WS));
    assert!(!flags.contains(AtomFlags::DIRECT_OUTPUT));
}

#[test]
fn atom_flags_empty_is_zero() {
    let flags = AtomFlags::empty();
    assert!(flags.is_empty());
    assert!(!flags.contains(AtomFlags::HAS_LB));
}

#[test]
fn scalar_style_all_variants() {
    let styles = [
        ScalarStyle::Plain,
        ScalarStyle::SingleQuoted,
        ScalarStyle::DoubleQuoted,
        ScalarStyle::Literal,
        ScalarStyle::Folded,
    ];
    assert_eq!(styles.len(), 5);
    // Verify they are distinct
    for (i, a) in styles.iter().enumerate() {
        for (j, b) in styles.iter().enumerate() {
            if i != j {
                assert_ne!(a, b);
            }
        }
    }
}

#[test]
fn chomp_all_variants() {
    let chomps = [Chomp::Strip, Chomp::Clip, Chomp::Keep];
    assert_eq!(chomps.len(), 3);
}

#[test]
fn chomp_default_is_clip() {
    assert_eq!(Chomp::default(), Chomp::Clip);
}

#[test]
fn token_kind_structural_variants() {
    // Verify key structural token kinds exist
    let kinds = [
        TokenKind::StreamStart,
        TokenKind::StreamEnd,
        TokenKind::DocumentStart,
        TokenKind::DocumentEnd,
        TokenKind::BlockSequenceStart,
        TokenKind::BlockMappingStart,
        TokenKind::BlockEnd,
        TokenKind::FlowSequenceStart,
        TokenKind::FlowSequenceEnd,
        TokenKind::FlowMappingStart,
        TokenKind::FlowMappingEnd,
        TokenKind::BlockEntry,
        TokenKind::FlowEntry,
        TokenKind::Key,
        TokenKind::Value,
        TokenKind::Anchor,
        TokenKind::Alias,
        TokenKind::Tag,
        TokenKind::Scalar,
        TokenKind::VersionDirective,
        TokenKind::TagDirective,
    ];
    // All distinct
    for (i, a) in kinds.iter().enumerate() {
        for (j, b) in kinds.iter().enumerate() {
            if i != j {
                assert_ne!(a, b);
            }
        }
    }
}

#[test]
fn token_kind_is_yaml_content() {
    // Content tokens are block/flow structure + data tokens
    assert!(TokenKind::BlockSequenceStart.is_content());
    assert!(TokenKind::Scalar.is_content());
    assert!(TokenKind::Key.is_content());
    assert!(TokenKind::Value.is_content());

    // Non-content tokens
    assert!(!TokenKind::StreamStart.is_content());
    assert!(!TokenKind::StreamEnd.is_content());
    assert!(!TokenKind::VersionDirective.is_content());
    assert!(!TokenKind::TagDirective.is_content());
    assert!(!TokenKind::DocumentStart.is_content());
    assert!(!TokenKind::DocumentEnd.is_content());
}

#[test]
fn atom_with_owned_data() {
    let atom = Atom {
        data: Cow::Owned("escaped\nnewline".to_string()),
        span: Span::default(),
        style: ScalarStyle::DoubleQuoted,
        chomp: Chomp::Clip,
        flags: AtomFlags::HAS_LB | AtomFlags::HAS_ESC,
    };
    assert!(atom.flags.contains(AtomFlags::HAS_LB));
    assert!(atom.flags.contains(AtomFlags::HAS_ESC));
    assert_eq!(atom.data, "escaped\nnewline");
}

#[test]
fn atom_flags_all_documented_flags_exist() {
    // Verify all documented flags exist and are distinct powers of 2
    let flags = [
        AtomFlags::HAS_LB,
        AtomFlags::HAS_WS,
        AtomFlags::STARTS_WITH_WS,
        AtomFlags::ENDS_WITH_WS,
        AtomFlags::STARTS_WITH_LB,
        AtomFlags::ENDS_WITH_LB,
        AtomFlags::TRAILING_LB,
        AtomFlags::EMPTY,
        AtomFlags::HAS_ESC,
        AtomFlags::DIRECT_OUTPUT,
        AtomFlags::IS_MULTILINE,
    ];
    // Each flag should be a single bit
    for flag in &flags {
        assert_eq!(
            flag.bits().count_ones(),
            1,
            "{flag:?} should be a single bit"
        );
    }
}

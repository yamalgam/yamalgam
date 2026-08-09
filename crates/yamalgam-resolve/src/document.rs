//! Owned semantic YAML documents used by resolution sessions.

use std::borrow::Cow;
use std::mem;

use yamalgam_core::Span;
use yamalgam_parser::{AtomFlags, Chomp, CollectionStyle, Event, ScalarStyle};

use crate::ResourceId;

/// One owned YAML document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedDocument {
    /// Semantic document root.
    pub root: OwnedNode,
}

/// One owned YAML node with source and presentation metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedNode {
    /// Resource containing this node.
    pub origin: ResourceId,
    /// Source span within the containing resource.
    pub span: Span,
    /// Optional anchor name without `&`.
    pub anchor: Option<String>,
    /// Optional resolved tag.
    pub tag: Option<String>,
    /// Semantic node content.
    pub kind: OwnedKind,
}

/// Owned YAML node variants.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OwnedKind {
    /// Scalar value and presentation.
    Scalar(OwnedScalar),
    /// Alias reference without `*`.
    Alias {
        /// Referenced anchor name.
        name: String,
    },
    /// Ordered YAML sequence.
    Sequence {
        /// Block or flow presentation.
        style: CollectionStyle,
        /// Sequence items in source order.
        items: Vec<OwnedNode>,
    },
    /// Ordered YAML mapping with arbitrary node keys.
    Mapping {
        /// Block or flow presentation.
        style: CollectionStyle,
        /// Key/value pairs in source order.
        entries: Vec<(OwnedNode, OwnedNode)>,
    },
}

/// Owned scalar value and scanner presentation flags.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedScalar {
    /// Decoded scalar value.
    pub value: String,
    /// Scalar presentation style.
    pub style: ScalarStyle,
    /// Block-scalar chomping mode.
    pub chomp: Chomp,
    /// Content and presentation flags.
    pub flags: AtomFlags,
}

/// Failure while converting or accounting for an owned document.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum DocumentError {
    /// The event stream contained no YAML document.
    #[error("YAML stream contains no document")]
    EmptyStream,
    /// A document marker had no semantic root node.
    #[error("YAML document contains no root node")]
    EmptyDocument,
    /// The event stream contained more than one YAML document.
    #[error("YAML stream contains multiple documents")]
    MultipleDocuments,
    /// One document contained more than one root node.
    #[error("YAML document contains multiple root nodes")]
    MultipleRoots,
    /// A collection ended with a mapping key but no value.
    #[error("YAML mapping event stream has an unmatched key")]
    MappingParity,
    /// The event stream ended before all collections closed.
    #[error("YAML event stream contains a truncated collection")]
    TruncatedCollection,
    /// The event stream ended before its document marker.
    #[error("YAML event stream contains a truncated document")]
    TruncatedDocument,
    /// An event appeared in an invalid structural position.
    #[error("unexpected {0} event in YAML document")]
    UnexpectedEvent(&'static str),
    /// Owned resource accounting overflowed `usize`.
    #[error("resource accounting overflow for {0}")]
    ResourceOverflow(&'static str),
}

impl OwnedDocument {
    /// Consume exactly one semantic YAML document from parser events.
    ///
    /// Structural comments and indicator events are intentionally omitted from
    /// imported semantic documents. Directives and stream/document framing are
    /// validated but are not retained.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] for empty or multi-document streams and for
    /// malformed event nesting.
    pub fn from_events<'input>(
        origin: ResourceId,
        events: impl IntoIterator<Item = Event<'input>>,
    ) -> Result<Self, DocumentError> {
        let mut stack = Vec::new();
        let mut current_root = None;
        let mut completed_root = None;
        let mut in_document = false;
        let mut saw_document = false;

        for event in events {
            if event.is_structural() {
                continue;
            }
            match event.into_owned() {
                Event::StreamStart => {
                    if in_document || !stack.is_empty() {
                        return Err(DocumentError::UnexpectedEvent("stream start"));
                    }
                }
                Event::StreamEnd => {
                    if in_document || !stack.is_empty() {
                        return Err(DocumentError::UnexpectedEvent("stream end"));
                    }
                }
                Event::VersionDirective { .. } | Event::TagDirective { .. } => {
                    if in_document || !stack.is_empty() {
                        return Err(DocumentError::UnexpectedEvent("directive"));
                    }
                }
                Event::DocumentStart { .. } => {
                    if in_document || saw_document {
                        return Err(DocumentError::MultipleDocuments);
                    }
                    saw_document = true;
                    in_document = true;
                }
                Event::DocumentEnd { .. } => {
                    if !in_document {
                        return Err(DocumentError::UnexpectedEvent("document end"));
                    }
                    if !stack.is_empty() {
                        return Err(DocumentError::TruncatedCollection);
                    }
                    completed_root = Some(current_root.take().ok_or(DocumentError::EmptyDocument)?);
                    in_document = false;
                }
                Event::SequenceStart {
                    anchor,
                    tag,
                    style,
                    span,
                } => {
                    ensure_node_position(in_document, &stack, &current_root)?;
                    stack.push(Frame::Sequence {
                        anchor: cow_option(anchor),
                        tag: cow_option(tag),
                        style,
                        span,
                        items: Vec::new(),
                    });
                }
                Event::MappingStart {
                    anchor,
                    tag,
                    style,
                    span,
                } => {
                    ensure_node_position(in_document, &stack, &current_root)?;
                    stack.push(Frame::Mapping {
                        anchor: cow_option(anchor),
                        tag: cow_option(tag),
                        style,
                        span,
                        entries: Vec::new(),
                        pending_key: None,
                    });
                }
                Event::Scalar {
                    anchor,
                    tag,
                    value,
                    style,
                    chomp,
                    flags,
                    span,
                } => {
                    ensure_document(in_document)?;
                    attach_node(
                        OwnedNode {
                            origin: origin.clone(),
                            span,
                            anchor: cow_option(anchor),
                            tag: cow_option(tag),
                            kind: OwnedKind::Scalar(OwnedScalar {
                                value: value.into_owned(),
                                style,
                                chomp,
                                flags,
                            }),
                        },
                        &mut stack,
                        &mut current_root,
                    )?;
                }
                Event::Alias { name, span } => {
                    ensure_document(in_document)?;
                    attach_node(
                        OwnedNode {
                            origin: origin.clone(),
                            span,
                            anchor: None,
                            tag: None,
                            kind: OwnedKind::Alias {
                                name: name.into_owned(),
                            },
                        },
                        &mut stack,
                        &mut current_root,
                    )?;
                }
                Event::SequenceEnd { .. } => {
                    let Some(Frame::Sequence {
                        anchor,
                        tag,
                        style,
                        span,
                        items,
                    }) = stack.pop()
                    else {
                        return Err(DocumentError::UnexpectedEvent("sequence end"));
                    };
                    attach_node(
                        OwnedNode {
                            origin: origin.clone(),
                            span,
                            anchor,
                            tag,
                            kind: OwnedKind::Sequence { style, items },
                        },
                        &mut stack,
                        &mut current_root,
                    )?;
                }
                Event::MappingEnd { .. } => {
                    let Some(Frame::Mapping {
                        anchor,
                        tag,
                        style,
                        span,
                        entries,
                        pending_key,
                    }) = stack.pop()
                    else {
                        return Err(DocumentError::UnexpectedEvent("mapping end"));
                    };
                    if pending_key.is_some() {
                        return Err(DocumentError::MappingParity);
                    }
                    attach_node(
                        OwnedNode {
                            origin: origin.clone(),
                            span,
                            anchor,
                            tag,
                            kind: OwnedKind::Mapping { style, entries },
                        },
                        &mut stack,
                        &mut current_root,
                    )?;
                }
                Event::Comment { .. }
                | Event::BlockEntry { .. }
                | Event::KeyIndicator { .. }
                | Event::ValueIndicator { .. } => unreachable!("structural events were skipped"),
            }
        }

        if !stack.is_empty() {
            return Err(DocumentError::TruncatedCollection);
        }
        if in_document {
            return Err(DocumentError::TruncatedDocument);
        }
        completed_root
            .map(|root| Self { root })
            .ok_or(DocumentError::EmptyStream)
    }

    /// Replay this document as a detached semantic event stream.
    #[must_use]
    pub fn to_events(&self) -> Vec<Event<'static>> {
        let mut events = Vec::new();
        events.push(Event::StreamStart);
        events.push(Event::DocumentStart {
            implicit: true,
            span: self.root.span,
        });
        events.extend(self.root.to_events());
        events.push(Event::DocumentEnd {
            implicit: true,
            span: self.root.span,
        });
        events.push(Event::StreamEnd);
        events
    }
}

impl OwnedNode {
    /// Replay this node as detached semantic events without stream framing.
    #[must_use]
    pub fn to_events(&self) -> Vec<Event<'static>> {
        let mut events = Vec::new();
        let mut pending = vec![Emit::Node(self)];
        while let Some(action) = pending.pop() {
            match action {
                Emit::Node(node) => match &node.kind {
                    OwnedKind::Scalar(scalar) => events.push(Event::Scalar {
                        anchor: owned_option(&node.anchor),
                        tag: owned_option(&node.tag),
                        value: Cow::Owned(scalar.value.clone()),
                        style: scalar.style,
                        chomp: scalar.chomp,
                        flags: scalar.flags,
                        span: node.span,
                    }),
                    OwnedKind::Alias { name } => events.push(Event::Alias {
                        name: Cow::Owned(name.clone()),
                        span: node.span,
                    }),
                    OwnedKind::Sequence { style, items } => {
                        events.push(Event::SequenceStart {
                            anchor: owned_option(&node.anchor),
                            tag: owned_option(&node.tag),
                            style: *style,
                            span: node.span,
                        });
                        pending.push(Emit::SequenceEnd(node.span));
                        pending.extend(items.iter().rev().map(Emit::Node));
                    }
                    OwnedKind::Mapping { style, entries } => {
                        events.push(Event::MappingStart {
                            anchor: owned_option(&node.anchor),
                            tag: owned_option(&node.tag),
                            style: *style,
                            span: node.span,
                        });
                        pending.push(Emit::MappingEnd(node.span));
                        for (key, value) in entries.iter().rev() {
                            pending.push(Emit::Node(value));
                            pending.push(Emit::Node(key));
                        }
                    }
                },
                Emit::SequenceEnd(span) => events.push(Event::SequenceEnd { span }),
                Emit::MappingEnd(span) => events.push(Event::MappingEnd { span }),
            }
        }
        events
    }

    /// Compute conservative materialized storage using checked, iterative
    /// accounting.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError::ResourceOverflow`] if any byte or collection
    /// storage calculation exceeds `usize`.
    pub fn materialized_weight(&self) -> Result<usize, DocumentError> {
        let mut total = 0_usize;
        let mut pending = vec![self];
        while let Some(node) = pending.pop() {
            add_weight(&mut total, mem::size_of::<Self>())?;
            if let Some(anchor) = &node.anchor {
                add_weight(&mut total, anchor.len())?;
            }
            if let Some(tag) = &node.tag {
                add_weight(&mut total, tag.len())?;
            }
            match &node.kind {
                OwnedKind::Scalar(scalar) => add_weight(&mut total, scalar.value.len())?,
                OwnedKind::Alias { name } => add_weight(&mut total, name.len())?,
                OwnedKind::Sequence { items, .. } => {
                    let storage = items
                        .len()
                        .checked_mul(mem::size_of::<Self>())
                        .ok_or(DocumentError::ResourceOverflow("sequence storage"))?;
                    add_weight(&mut total, storage)?;
                    pending.extend(items);
                }
                OwnedKind::Mapping { entries, .. } => {
                    let storage = entries
                        .len()
                        .checked_mul(mem::size_of::<(Self, Self)>())
                        .ok_or(DocumentError::ResourceOverflow("mapping storage"))?;
                    add_weight(&mut total, storage)?;
                    for (key, value) in entries {
                        pending.push(key);
                        pending.push(value);
                    }
                }
            }
        }
        Ok(total)
    }
}

enum Frame {
    Sequence {
        anchor: Option<String>,
        tag: Option<String>,
        style: CollectionStyle,
        span: Span,
        items: Vec<OwnedNode>,
    },
    Mapping {
        anchor: Option<String>,
        tag: Option<String>,
        style: CollectionStyle,
        span: Span,
        entries: Vec<(OwnedNode, OwnedNode)>,
        pending_key: Option<Box<OwnedNode>>,
    },
}

enum Emit<'a> {
    Node(&'a OwnedNode),
    SequenceEnd(Span),
    MappingEnd(Span),
}

const fn ensure_document(in_document: bool) -> Result<(), DocumentError> {
    if in_document {
        Ok(())
    } else {
        Err(DocumentError::UnexpectedEvent("node"))
    }
}

fn ensure_node_position(
    in_document: bool,
    stack: &[Frame],
    root: &Option<OwnedNode>,
) -> Result<(), DocumentError> {
    ensure_document(in_document)?;
    if stack.is_empty() && root.is_some() {
        Err(DocumentError::MultipleRoots)
    } else {
        Ok(())
    }
}

fn attach_node(
    node: OwnedNode,
    stack: &mut [Frame],
    root: &mut Option<OwnedNode>,
) -> Result<(), DocumentError> {
    match stack.last_mut() {
        Some(Frame::Sequence { items, .. }) => items.push(node),
        Some(Frame::Mapping {
            entries,
            pending_key,
            ..
        }) => {
            if let Some(key) = pending_key.take() {
                entries.push((*key, node));
            } else {
                *pending_key = Some(Box::new(node));
            }
        }
        None if root.is_none() => *root = Some(node),
        None => return Err(DocumentError::MultipleRoots),
    }
    Ok(())
}

fn cow_option(value: Option<Cow<'static, str>>) -> Option<String> {
    value.map(Cow::into_owned)
}

fn owned_option(value: &Option<String>) -> Option<Cow<'static, str>> {
    value.as_ref().map(|value| Cow::Owned(value.clone()))
}

fn add_weight(total: &mut usize, increment: usize) -> Result<(), DocumentError> {
    *total = total
        .checked_add(increment)
        .ok_or(DocumentError::ResourceOverflow(
            "document materialized weight",
        ))?;
    Ok(())
}

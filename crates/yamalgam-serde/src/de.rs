//! Streaming serde `Deserializer` over the yamalgam parser event stream.

use std::borrow::Cow;
use std::iter::Peekable;

use serde::de::{self, DeserializeSeed, Visitor};
use yamalgam_core::value::Value;
use yamalgam_core::{ResourceLimits, TagResolver, Yaml12TagResolver, resolve_plain_scalar};
use yamalgam_parser::{Event, ParseError, Parser, ScalarStyle};

use crate::error::Error;

/// A streaming serde `Deserializer` that consumes YAML parser events directly.
///
/// No intermediate DOM is built — events are consumed on the fly. This means
/// deserialization is single-pass and allocation-light for scalar values.
pub struct Deserializer<'input> {
    /// Event source (parser or resolved events), wrapped in Peekable.
    events: Peekable<Box<dyn Iterator<Item = Result<Event<'input>, ParseError>> + 'input>>,
    /// Tag resolver for plain scalar typing.
    tag_resolver: Box<dyn TagResolver>,
    /// Resource limits (reserved for future use in nested structures).
    #[allow(dead_code)]
    limits: ResourceLimits,
    /// True once `StreamEnd` has been consumed or an error occurred.
    finished: bool,
    /// True when we've consumed `DocumentStart` and are inside a document.
    at_document_start: bool,
}

impl<'input> Deserializer<'input> {
    /// Create a `Deserializer` from a YAML input string.
    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &'input str) -> Self {
        let parser = Parser::new(input);
        Self {
            events: (Box::new(parser) as Box<dyn Iterator<Item = _> + 'input>).peekable(),
            tag_resolver: Box::new(Yaml12TagResolver),
            limits: ResourceLimits::none(),
            finished: false,
            at_document_start: false,
        }
    }

    /// Consume the next event, skipping structural events (Comment, BlockEntry,
    /// `KeyIndicator`, `ValueIndicator`).
    ///
    /// Also auto-consumes `StreamStart` and `DocumentStart` when they appear,
    /// since serde consumers don't care about stream/document framing.
    fn next_event(&mut self) -> Result<Event<'input>, Error> {
        loop {
            let event = self
                .events
                .next()
                .ok_or_else(|| Error::Unexpected {
                    expected: "event",
                    found: "end of stream".to_string(),
                    span: None,
                })?
                .map_err(Error::Parse)?;

            // Skip structural events that serde consumers ignore.
            if event.is_structural() {
                continue;
            }

            match &event {
                Event::StreamStart => continue,
                Event::DocumentStart { .. } => {
                    self.at_document_start = true;
                    continue;
                }
                Event::StreamEnd => {
                    self.finished = true;
                    return Ok(event);
                }
                Event::DocumentEnd { .. } => {
                    self.at_document_start = false;
                    return Ok(event);
                }
                _ => return Ok(event),
            }
        }
    }

    /// Peek the next non-structural event without consuming it.
    ///
    /// Auto-consumes `StreamStart` and `DocumentStart` as a side effect
    /// (same as `next_event`).
    fn peek_event(&mut self) -> Result<&Event<'input>, Error> {
        // Drain structural/framing events until we see something meaningful.
        loop {
            let event = self
                .events
                .peek()
                .ok_or_else(|| Error::Unexpected {
                    expected: "event",
                    found: "end of stream".to_string(),
                    span: None,
                })?
                .as_ref()
                .map_err(|e| Error::Parse(e.clone()))?;

            match event {
                _ if event.is_structural() => {
                    // Consume and discard.
                    let _ = self.events.next();
                    continue;
                }
                Event::StreamStart => {
                    let _ = self.events.next();
                    continue;
                }
                Event::DocumentStart { .. } => {
                    self.at_document_start = true;
                    let _ = self.events.next();
                    continue;
                }
                _ => {
                    // Now peek returns a meaningful event. We need to re-peek
                    // since we may have consumed items above.
                    break;
                }
            }
        }

        // After draining, the next peeked event should be non-structural.
        self.events
            .peek()
            .ok_or_else(|| Error::Unexpected {
                expected: "event",
                found: "end of stream".to_string(),
                span: None,
            })?
            .as_ref()
            .map_err(|e| Error::Parse(e.clone()))
    }

    /// Check that the document and stream have ended.
    ///
    /// Called by `from_str()` after deserializing a value to ensure there
    /// isn't a second document in the input.
    pub fn check_end(&mut self) -> Result<(), Error> {
        loop {
            let event = match self.events.next() {
                Some(Ok(e)) => e,
                Some(Err(e)) => return Err(Error::Parse(e)),
                None => return Ok(()),
            };

            if event.is_structural() {
                continue;
            }

            match event {
                Event::StreamEnd => {
                    self.finished = true;
                    return Ok(());
                }
                Event::DocumentEnd { .. } => continue,
                Event::DocumentStart { .. } => return Err(Error::MoreThanOneDocument),
                _ => return Err(Error::MoreThanOneDocument),
            }
        }
    }

    /// Consume a scalar event and return (value, style).
    fn expect_scalar(&mut self) -> Result<(Cow<'input, str>, ScalarStyle), Error> {
        match self.next_event()? {
            Event::Scalar { value, style, .. } => Ok((value, style)),
            other => Err(Error::Unexpected {
                expected: "scalar",
                found: format!("{other:?}"),
                span: None,
            }),
        }
    }

    /// Returns `true` if the next event is a scalar that resolves to null,
    /// or if we've reached the end of the document/stream (empty doc = null).
    fn peek_is_null(&mut self) -> Result<bool, Error> {
        let event = self.peek_event()?;
        match event {
            Event::Scalar { value, style, .. } => {
                if *style != ScalarStyle::Plain {
                    return Ok(false);
                }
                Ok(matches!(resolve_plain_scalar(value), Value::Null))
            }
            // Empty document / stream → null.
            Event::StreamEnd | Event::DocumentEnd { .. } => Ok(true),
            _ => Ok(false),
        }
    }
}

impl<'de> de::Deserializer<'de> for &mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let event = self.peek_event()?.clone();
        match event {
            Event::Scalar { value, style, .. } => {
                // Consume the event.
                let _ = self.next_event()?;

                match style {
                    ScalarStyle::SingleQuoted | ScalarStyle::DoubleQuoted => {
                        // Quoted scalars are always strings.
                        visit_cow_str(visitor, value)
                    }
                    ScalarStyle::Literal | ScalarStyle::Folded => {
                        // Block scalars are always strings.
                        visit_cow_str(visitor, value)
                    }
                    ScalarStyle::Plain => {
                        // Resolve via tag resolver.
                        let resolved = self.tag_resolver.resolve_scalar(&value);
                        match resolved {
                            Value::Null => visitor.visit_unit(),
                            Value::Bool(b) => visitor.visit_bool(b),
                            Value::Integer(i) => visitor.visit_i64(i),
                            Value::Float(f) => visitor.visit_f64(f),
                            Value::String(_) => {
                                // Use the original Cow, not the resolved String.
                                visit_cow_str(visitor, value)
                            }
                            // Sequence/Mapping can't come from scalar resolution.
                            _ => visit_cow_str(visitor, value),
                        }
                    }
                }
            }
            Event::SequenceStart { .. } => self.deserialize_seq(visitor),
            Event::MappingStart { .. } => self.deserialize_map(visitor),
            Event::Alias { name, span, .. } => {
                let _ = self.next_event()?;
                Err(Error::UndefinedAlias {
                    name: name.into_owned(),
                    span: Some(span),
                })
            }
            Event::StreamEnd => visitor.visit_unit(),
            Event::DocumentEnd { .. } => {
                // Empty document — treat as null.
                let _ = self.next_event()?;
                visitor.visit_unit()
            }
            other => Err(Error::Unexpected {
                expected: "scalar, sequence, or mapping",
                found: format!("{other:?}"),
                span: None,
            }),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let (value, _style) = self.expect_scalar()?;
        let b = value.parse::<bool>().map_err(|_| Error::Unexpected {
            expected: "boolean",
            found: value.to_string(),
            span: None,
        })?;
        visitor.visit_bool(b)
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let i = self.parse_integer()?;
        visitor.visit_i8(i64_to_i8(i)?)
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let i = self.parse_integer()?;
        visitor.visit_i16(i64_to_i16(i)?)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let i = self.parse_integer()?;
        visitor.visit_i32(i64_to_i32(i)?)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let i = self.parse_integer()?;
        visitor.visit_i64(i)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let i = self.parse_integer()?;
        visitor.visit_u8(i64_to_u8(i)?)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let i = self.parse_integer()?;
        visitor.visit_u16(i64_to_u16(i)?)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let i = self.parse_integer()?;
        visitor.visit_u32(i64_to_u32(i)?)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let i = self.parse_integer()?;
        visitor.visit_u64(i64_to_u64(i)?)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let f = self.parse_float()?;
        visitor.visit_f32(f as f32)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let f = self.parse_float()?;
        visitor.visit_f64(f)
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let (value, _style) = self.expect_scalar()?;
        let mut chars = value.chars();
        let c = chars.next().ok_or_else(|| Error::Unexpected {
            expected: "single character",
            found: "empty string".to_string(),
            span: None,
        })?;
        if chars.next().is_some() {
            return Err(Error::Unexpected {
                expected: "single character",
                found: format!("string of length > 1: {value:?}"),
                span: None,
            });
        }
        visitor.visit_char(c)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let (value, _style) = self.expect_scalar()?;
        visit_cow_str(visitor, value)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let (value, _style) = self.expect_scalar()?;
        visitor.visit_string(value.into_owned())
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let (value, _style) = self.expect_scalar()?;
        visitor.visit_bytes(value.as_bytes())
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let (value, _style) = self.expect_scalar()?;
        visitor.visit_byte_buf(value.into_owned().into_bytes())
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if self.peek_is_null()? {
            // Consume the null scalar.
            let _ = self.next_event()?;
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let event = self.peek_event()?.clone();
        match event {
            Event::StreamEnd | Event::DocumentEnd { .. } => {
                // Empty document = null.
                let _ = self.next_event()?;
                visitor.visit_unit()
            }
            Event::Scalar { .. } => {
                let (value, style) = self.expect_scalar()?;
                if style == ScalarStyle::Plain {
                    match resolve_plain_scalar(&value) {
                        Value::Null => visitor.visit_unit(),
                        _ => Err(Error::Unexpected {
                            expected: "null",
                            found: value.to_string(),
                            span: None,
                        }),
                    }
                } else if value.is_empty() {
                    // An empty quoted string can also represent unit.
                    visitor.visit_unit()
                } else {
                    Err(Error::Unexpected {
                        expected: "null",
                        found: value.to_string(),
                        span: None,
                    })
                }
            }
            other => Err(Error::Unexpected {
                expected: "null",
                found: format!("{other:?}"),
                span: None,
            }),
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.next_event()? {
            Event::SequenceStart { .. } => {
                let mut access = SeqAccess {
                    de: self,
                    finished: false,
                };
                let value = visitor.visit_seq(&mut access)?;
                // If the visitor returned early (e.g. fixed-size tuple)
                // without draining to SequenceEnd, consume remaining
                // elements and the end marker.
                if !access.finished {
                    access.drain()?;
                }
                Ok(value)
            }
            other => Err(Error::Unexpected {
                expected: "sequence",
                found: format!("{other:?}"),
                span: None,
            }),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.next_event()? {
            Event::MappingStart { .. } => {
                let mut access = MapAccess {
                    de: self,
                    finished: false,
                };
                let value = visitor.visit_map(&mut access)?;
                // If the visitor returned early without draining to MappingEnd,
                // skip remaining entries and consume the end marker.
                if !access.finished {
                    access.drain()?;
                }
                Ok(value)
            }
            other => Err(Error::Unexpected {
                expected: "mapping",
                found: format!("{other:?}"),
                span: None,
            }),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let event = self.peek_event()?.clone();
        match event {
            // Unit variant: bare scalar like `Red`
            Event::Scalar { value, .. } => {
                let _ = self.next_event()?;
                visitor.visit_enum(EnumAccess {
                    de: self,
                    variant: value,
                    in_mapping: false,
                })
            }
            // Newtype/tuple/struct variant: single-key mapping like `Circle: 5.0`
            Event::MappingStart { .. } => {
                let _ = self.next_event()?; // consume MappingStart
                let (variant_name, _style) = self.expect_scalar()?;
                visitor.visit_enum(EnumAccess {
                    de: self,
                    variant: variant_name,
                    in_mapping: true,
                })
            }
            other => Err(Error::Unexpected {
                expected: "string or mapping (for enum)",
                found: format!("{other:?}"),
                span: None,
            }),
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        // Skip the next value entirely, including nested structures.
        self.skip_value()?;
        visitor.visit_unit()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

impl Deserializer<'_> {
    /// Parse the next scalar event as an integer via tag resolution.
    fn parse_integer(&mut self) -> Result<i64, Error> {
        let (value, style) = self.expect_scalar()?;
        if style == ScalarStyle::Plain {
            match resolve_plain_scalar(&value) {
                Value::Integer(i) => Ok(i),
                _ => Err(Error::Unexpected {
                    expected: "integer",
                    found: value.to_string(),
                    span: None,
                }),
            }
        } else {
            // Quoted scalars: try direct parse.
            value.parse::<i64>().map_err(|_| Error::Unexpected {
                expected: "integer",
                found: value.to_string(),
                span: None,
            })
        }
    }

    /// Skip the next value in the event stream, including nested structures.
    ///
    /// Used by `deserialize_ignored_any` (e.g. for unknown struct fields with
    /// `#[serde(deny_unknown_fields)]` disabled).
    fn skip_value(&mut self) -> Result<(), Error> {
        let mut depth: u32 = 0;
        loop {
            let event = self.next_event()?;
            match &event {
                Event::SequenceStart { .. } | Event::MappingStart { .. } => {
                    depth += 1;
                }
                Event::SequenceEnd { .. } | Event::MappingEnd { .. } => {
                    depth -= 1;
                }
                _ => {}
            }
            if depth == 0 {
                return Ok(());
            }
        }
    }

    /// Parse the next scalar event as a float via tag resolution.
    fn parse_float(&mut self) -> Result<f64, Error> {
        let (value, style) = self.expect_scalar()?;
        if style == ScalarStyle::Plain {
            match resolve_plain_scalar(&value) {
                Value::Float(f) => Ok(f),
                Value::Integer(i) => Ok(i as f64),
                _ => Err(Error::Unexpected {
                    expected: "float",
                    found: value.to_string(),
                    span: None,
                }),
            }
        } else {
            // Quoted scalars: try direct parse.
            value.parse::<f64>().map_err(|_| Error::Unexpected {
                expected: "float",
                found: value.to_string(),
                span: None,
            })
        }
    }
}

/// Visit a `Cow<'de, str>` using zero-copy when possible.
fn visit_cow_str<'de, V: Visitor<'de>>(
    visitor: V,
    value: Cow<'de, str>,
) -> Result<V::Value, Error> {
    match value {
        Cow::Borrowed(s) => visitor.visit_borrowed_str(s),
        Cow::Owned(s) => visitor.visit_string(s),
    }
}

// ---------------------------------------------------------------------------
// Integer narrowing conversions
// ---------------------------------------------------------------------------

fn i64_to_i8(i: i64) -> Result<i8, Error> {
    i8::try_from(i).map_err(|_| Error::Custom(format!("integer {i} out of range for i8")))
}

fn i64_to_i16(i: i64) -> Result<i16, Error> {
    i16::try_from(i).map_err(|_| Error::Custom(format!("integer {i} out of range for i16")))
}

fn i64_to_i32(i: i64) -> Result<i32, Error> {
    i32::try_from(i).map_err(|_| Error::Custom(format!("integer {i} out of range for i32")))
}

fn i64_to_u8(i: i64) -> Result<u8, Error> {
    u8::try_from(i).map_err(|_| Error::Custom(format!("integer {i} out of range for u8")))
}

fn i64_to_u16(i: i64) -> Result<u16, Error> {
    u16::try_from(i).map_err(|_| Error::Custom(format!("integer {i} out of range for u16")))
}

fn i64_to_u32(i: i64) -> Result<u32, Error> {
    u32::try_from(i).map_err(|_| Error::Custom(format!("integer {i} out of range for u32")))
}

fn i64_to_u64(i: i64) -> Result<u64, Error> {
    u64::try_from(i).map_err(|_| Error::Custom(format!("integer {i} out of range for u64")))
}

// ---------------------------------------------------------------------------
// SeqAccess — iterates sequence elements until SequenceEnd
// ---------------------------------------------------------------------------

struct SeqAccess<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    /// `true` once `SequenceEnd` has been consumed.
    finished: bool,
}

impl SeqAccess<'_, '_> {
    /// Drain remaining elements and consume `SequenceEnd`.
    fn drain(&mut self) -> Result<(), Error> {
        let mut depth: u32 = 0;
        loop {
            let event = self.de.next_event()?;
            match &event {
                Event::SequenceStart { .. } | Event::MappingStart { .. } => depth += 1,
                Event::SequenceEnd { .. } if depth == 0 => {
                    self.finished = true;
                    return Ok(());
                }
                Event::SequenceEnd { .. } | Event::MappingEnd { .. } => depth -= 1,
                _ => {}
            }
        }
    }
}

impl<'de> de::SeqAccess<'de> for &mut SeqAccess<'_, 'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        let event = self.de.peek_event()?;
        if matches!(event, Event::SequenceEnd { .. }) {
            // Consume SequenceEnd.
            let _ = self.de.next_event()?;
            self.finished = true;
            return Ok(None);
        }
        seed.deserialize(&mut *self.de).map(Some)
    }
}

// ---------------------------------------------------------------------------
// MapAccess — iterates mapping key/value pairs until MappingEnd
// ---------------------------------------------------------------------------

struct MapAccess<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    /// `true` once `MappingEnd` has been consumed.
    finished: bool,
}

impl MapAccess<'_, '_> {
    /// Drain remaining key/value pairs and consume `MappingEnd`.
    fn drain(&mut self) -> Result<(), Error> {
        let mut depth: u32 = 0;
        loop {
            let event = self.de.next_event()?;
            match &event {
                Event::SequenceStart { .. } | Event::MappingStart { .. } => depth += 1,
                Event::MappingEnd { .. } if depth == 0 => {
                    self.finished = true;
                    return Ok(());
                }
                Event::SequenceEnd { .. } | Event::MappingEnd { .. } => depth -= 1,
                _ => {}
            }
        }
    }
}

impl<'de> de::MapAccess<'de> for &mut MapAccess<'_, 'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        let event = self.de.peek_event()?;
        if matches!(event, Event::MappingEnd { .. }) {
            // Consume MappingEnd.
            let _ = self.de.next_event()?;
            self.finished = true;
            return Ok(None);
        }
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Error> {
        seed.deserialize(&mut *self.de)
    }
}

// ---------------------------------------------------------------------------
// EnumAccess — handles enum variant dispatch
// ---------------------------------------------------------------------------

struct EnumAccess<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    /// Variant name, already consumed from the event stream.
    variant: Cow<'de, str>,
    /// `true` if we consumed a `MappingStart` (non-unit variant).
    in_mapping: bool,
}

impl<'de> de::EnumAccess<'de> for EnumAccess<'_, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self), Error> {
        let variant = seed.deserialize(VariantNameDeserializer {
            value: self.variant.clone(),
        })?;
        Ok((variant, self))
    }
}

impl<'de> de::VariantAccess<'de> for EnumAccess<'_, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        if self.in_mapping {
            // Unit variant shouldn't come via a mapping, but if it does,
            // consume the MappingEnd.
            // Actually: `Point` as a unit variant is a bare scalar, so
            // `in_mapping` should be false. If someone writes `Point: ~`
            // we should consume the null value + MappingEnd.
            self.de.skip_value()?;
            match self.de.next_event()? {
                Event::MappingEnd { .. } => Ok(()),
                other => Err(Error::Unexpected {
                    expected: "mapping end",
                    found: format!("{other:?}"),
                    span: None,
                }),
            }
        } else {
            Ok(())
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        let value = seed.deserialize(&mut *self.de)?;
        if self.in_mapping {
            match self.de.next_event()? {
                Event::MappingEnd { .. } => {}
                other => {
                    return Err(Error::Unexpected {
                        expected: "mapping end",
                        found: format!("{other:?}"),
                        span: None,
                    });
                }
            }
        }
        Ok(value)
    }

    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Error> {
        let value = de::Deserializer::deserialize_tuple(&mut *self.de, len, visitor)?;
        if self.in_mapping {
            match self.de.next_event()? {
                Event::MappingEnd { .. } => {}
                other => {
                    return Err(Error::Unexpected {
                        expected: "mapping end",
                        found: format!("{other:?}"),
                        span: None,
                    });
                }
            }
        }
        Ok(value)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        let value = de::Deserializer::deserialize_struct(&mut *self.de, "", fields, visitor)?;
        if self.in_mapping {
            match self.de.next_event()? {
                Event::MappingEnd { .. } => {}
                other => {
                    return Err(Error::Unexpected {
                        expected: "mapping end",
                        found: format!("{other:?}"),
                        span: None,
                    });
                }
            }
        }
        Ok(value)
    }
}

// ---------------------------------------------------------------------------
// VariantNameDeserializer — deserializes a variant name from a Cow<str>
// ---------------------------------------------------------------------------

/// Minimal deserializer that yields a string value for variant name dispatch.
struct VariantNameDeserializer<'de> {
    value: Cow<'de, str>,
}

impl<'de> de::Deserializer<'de> for VariantNameDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visit_cow_str(visitor, self.value)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visit_cow_str(visitor, self.value)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_string(self.value.into_owned())
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visit_cow_str(visitor, self.value)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum ignored_any
    }
}

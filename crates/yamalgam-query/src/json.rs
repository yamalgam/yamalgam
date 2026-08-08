//! Strict JSON validation and native value construction.

use yamalgam_core::{LimitExceeded, LoaderConfig, ResourceLimits, TagResolution};

use crate::{BuildError, YamlVal};

#[derive(Debug, thiserror::Error)]
pub enum JsonError {
    #[error("{0}")]
    Syntax(String),
    #[error("{0}")]
    Limit(#[from] LimitExceeded),
    #[error(transparent)]
    Build(#[from] BuildError),
    #[error("{0} overflow")]
    ResourceOverflow(&'static str),
}

pub fn parse(source: &str, config: &LoaderConfig) -> Result<YamlVal, JsonError> {
    Validator::new(source, &config.limits).validate()?;

    let config = config.clone().with_tag_resolution(TagResolution::Json);
    let mut documents = crate::build::parse_str_with_config(source, &config)?;
    if documents.len() != 1 {
        return Err(JsonError::Syntax(format!(
            "expected one JSON value, parsed {} documents",
            documents.len()
        )));
    }
    Ok(documents.remove(0))
}

struct Validator<'input, 'limits> {
    source: &'input str,
    bytes: &'input [u8],
    offset: usize,
    limits: &'limits ResourceLimits,
}

impl<'input, 'limits> Validator<'input, 'limits> {
    const fn new(source: &'input str, limits: &'limits ResourceLimits) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            offset: 0,
            limits,
        }
    }

    fn validate(mut self) -> Result<(), JsonError> {
        self.limits.check_input_size(self.bytes.len())?;
        self.skip_whitespace();
        if self.peek().is_none() {
            return self.fail("expected a JSON value");
        }
        self.value(0)?;
        self.skip_whitespace();
        if self.peek().is_some() {
            return self.fail("trailing content after the JSON value");
        }
        Ok(())
    }

    fn value(&mut self, depth: usize) -> Result<(), JsonError> {
        self.limits.check_depth(depth)?;
        match self.peek() {
            Some(b'n') => self.literal(b"null"),
            Some(b't') => self.literal(b"true"),
            Some(b'f') => self.literal(b"false"),
            Some(b'"') => self.string(false),
            Some(b'[') => self.array(depth),
            Some(b'{') => self.object(depth),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => self.fail("expected a JSON value"),
            None => self.fail("unexpected end of JSON input"),
        }
    }

    fn literal(&mut self, literal: &[u8]) -> Result<(), JsonError> {
        let start = self.offset;
        if !self.remaining().starts_with(literal) {
            return self.fail("invalid JSON literal");
        }
        self.offset += literal.len();
        self.require_value_boundary()?;
        self.limits.check_scalar_size(self.offset - start)?;
        Ok(())
    }

    fn number(&mut self) -> Result<(), JsonError> {
        let start = self.offset;
        self.consume_if(b'-');

        match self.peek() {
            Some(b'0') => {
                self.offset += 1;
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return self.fail("leading zeros are not allowed in JSON numbers");
                }
            }
            Some(b'1'..=b'9') => self.consume_digits(),
            _ => return self.fail("expected a digit in JSON number"),
        }

        if self.consume_if(b'.') {
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return self.fail("expected a digit after decimal point");
            }
            self.consume_digits();
        }

        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.offset += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.offset += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return self.fail("expected a digit in JSON exponent");
            }
            self.consume_digits();
        }

        self.require_value_boundary()?;
        let text = &self.source[start..self.offset];
        let value = text
            .parse::<f64>()
            .map_err(|_| self.error("invalid JSON number"))?;
        if !value.is_finite() {
            return self.fail("JSON number is outside the finite f64 range");
        }
        self.limits.check_scalar_size(text.len())?;
        Ok(())
    }

    fn string(&mut self, is_key: bool) -> Result<(), JsonError> {
        self.expect(b'"', "expected a JSON string")?;
        let mut decoded_bytes = 0usize;

        loop {
            match self.peek() {
                Some(b'"') => {
                    self.offset += 1;
                    self.limits.check_scalar_size(decoded_bytes)?;
                    if is_key {
                        self.limits.check_key_size(decoded_bytes)?;
                    }
                    return Ok(());
                }
                Some(0x00..=0x1f) => {
                    return self.fail("unescaped control character in JSON string");
                }
                Some(b'\\') => {
                    self.offset += 1;
                    decoded_bytes = decoded_bytes
                        .checked_add(self.escape_len()?)
                        .ok_or(JsonError::ResourceOverflow("decoded JSON string length"))?;
                }
                Some(byte) if byte.is_ascii() => {
                    self.offset += 1;
                    decoded_bytes = decoded_bytes
                        .checked_add(1)
                        .ok_or(JsonError::ResourceOverflow("decoded JSON string length"))?;
                }
                Some(_) => {
                    let character = self.source[self.offset..]
                        .chars()
                        .next()
                        .ok_or_else(|| self.error("invalid UTF-8 in JSON string"))?;
                    self.offset += character.len_utf8();
                    decoded_bytes = decoded_bytes
                        .checked_add(character.len_utf8())
                        .ok_or(JsonError::ResourceOverflow("decoded JSON string length"))?;
                }
                None => return self.fail("unterminated JSON string"),
            }
            self.limits.check_scalar_size(decoded_bytes)?;
        }
    }

    fn escape_len(&mut self) -> Result<usize, JsonError> {
        match self.next() {
            Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => Ok(1),
            Some(b'u') => {
                let first = self.hex_quad()?;
                match first {
                    0xd800..=0xdbff => {
                        if self.next() != Some(b'\\') || self.next() != Some(b'u') {
                            return self.fail("high surrogate must be followed by a low surrogate");
                        }
                        let second = self.hex_quad()?;
                        if !(0xdc00..=0xdfff).contains(&second) {
                            return self.fail("high surrogate must be followed by a low surrogate");
                        }
                        Ok(4)
                    }
                    0xdc00..=0xdfff => self.fail("unexpected low surrogate in JSON string"),
                    value => char::from_u32(u32::from(value))
                        .map(char::len_utf8)
                        .ok_or_else(|| self.error("invalid Unicode escape in JSON string")),
                }
            }
            Some(_) => self.fail("invalid escape in JSON string"),
            None => self.fail("unterminated escape in JSON string"),
        }
    }

    fn hex_quad(&mut self) -> Result<u16, JsonError> {
        let mut value = 0u16;
        for _ in 0..4 {
            let Some(digit) = self.next().and_then(hex_value) else {
                return self.fail("JSON Unicode escape requires four hexadecimal digits");
            };
            value = value * 16 + u16::from(digit);
        }
        Ok(value)
    }

    fn array(&mut self, depth: usize) -> Result<(), JsonError> {
        self.expect(b'[', "expected '['")?;
        self.skip_whitespace();
        if self.consume_if(b']') {
            return Ok(());
        }

        let mut count = 0usize;
        loop {
            count = count
                .checked_add(1)
                .ok_or(JsonError::ResourceOverflow("JSON array length"))?;
            self.limits.check_collection_size(count)?;
            let child_depth = depth
                .checked_add(1)
                .ok_or(JsonError::ResourceOverflow("JSON value depth"))?;
            self.value(child_depth)?;
            self.skip_whitespace();
            if self.consume_if(b']') {
                return Ok(());
            }
            self.expect(b',', "expected ',' or ']' in JSON array")?;
            self.skip_whitespace();
            if self.peek() == Some(b']') {
                return self.fail("trailing comma in JSON array");
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<(), JsonError> {
        self.expect(b'{', "expected '{'")?;
        self.skip_whitespace();
        if self.consume_if(b'}') {
            return Ok(());
        }

        let mut count = 0usize;
        loop {
            count = count
                .checked_add(1)
                .ok_or(JsonError::ResourceOverflow("JSON object length"))?;
            self.limits.check_collection_size(count)?;
            let child_depth = depth
                .checked_add(1)
                .ok_or(JsonError::ResourceOverflow("JSON value depth"))?;
            self.limits.check_depth(child_depth)?;
            if self.peek() != Some(b'"') {
                return self.fail("JSON object keys must be double-quoted strings");
            }
            self.string(true)?;
            self.skip_whitespace();
            self.expect(b':', "expected ':' after JSON object key")?;
            self.skip_whitespace();
            self.value(child_depth)?;
            self.skip_whitespace();
            if self.consume_if(b'}') {
                return Ok(());
            }
            self.expect(b',', "expected ',' or '}' in JSON object")?;
            self.skip_whitespace();
            if self.peek() == Some(b'}') {
                return self.fail("trailing comma in JSON object");
            }
        }
    }

    fn require_value_boundary(&self) -> Result<(), JsonError> {
        if matches!(
            self.peek(),
            None | Some(b' ' | b'\t' | b'\n' | b'\r' | b',' | b']' | b'}')
        ) {
            Ok(())
        } else {
            Err(self.error("invalid character after JSON value"))
        }
    }

    fn consume_digits(&mut self) {
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.offset += 1;
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.offset += 1;
        }
    }

    fn expect(&mut self, byte: u8, message: &str) -> Result<(), JsonError> {
        if self.consume_if(byte) {
            Ok(())
        } else {
            self.fail(message)
        }
    }

    fn consume_if(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.offset += 1;
            true
        } else {
            false
        }
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.offset += 1;
        Some(byte)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.offset).copied()
    }

    fn remaining(&self) -> &[u8] {
        &self.bytes[self.offset..]
    }

    fn fail<T>(&self, message: &str) -> Result<T, JsonError> {
        Err(self.error(message))
    }

    fn error(&self, message: &str) -> JsonError {
        JsonError::Syntax(format!("{message} at byte {}", self.offset))
    }
}

const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

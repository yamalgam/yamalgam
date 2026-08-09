//! RFC 6901 JSON Pointer parsing.

use std::fmt;

/// A decoded RFC 6901 JSON Pointer.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct JsonPointer {
    tokens: Vec<String>,
}

impl JsonPointer {
    /// Parse a decoded JSON Pointer.
    ///
    /// # Errors
    ///
    /// Returns [`PointerError::NotAbsolute`] unless a non-empty pointer starts
    /// with `/`. Returns [`PointerError::InvalidEscape`] for any `~` escape
    /// other than `~0` or `~1`.
    pub fn parse(pointer: &str) -> Result<Self, PointerError> {
        if pointer.is_empty() {
            return Ok(Self { tokens: Vec::new() });
        }
        let Some(rest) = pointer.strip_prefix('/') else {
            return Err(PointerError::NotAbsolute);
        };
        let tokens = rest
            .split('/')
            .map(decode_token)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { tokens })
    }

    /// Return the decoded reference tokens.
    #[must_use]
    pub fn tokens(&self) -> &[String] {
        &self.tokens
    }
}

/// Failure while decoding an RFC 6901 pointer.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PointerError {
    /// A non-empty pointer did not begin with `/`.
    #[error("JSON Pointer must be empty or begin with '/'")]
    NotAbsolute,
    /// A token contained an escape other than `~0` or `~1`.
    #[error("JSON Pointer contains an invalid '~' escape")]
    InvalidEscape,
}

fn decode_token(token: &str) -> Result<String, PointerError> {
    let mut decoded = String::with_capacity(token.len());
    let mut chars = token.chars();
    while let Some(character) = chars.next() {
        if character != '~' {
            decoded.push(character);
            continue;
        }
        match chars.next() {
            Some('0') => decoded.push('~'),
            Some('1') => decoded.push('/'),
            Some(_) | None => return Err(PointerError::InvalidEscape),
        }
    }
    Ok(decoded)
}

impl fmt::Display for JsonPointer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for token in &self.tokens {
            formatter.write_str("/")?;
            for character in token.chars() {
                match character {
                    '~' => formatter.write_str("~0")?,
                    '/' => formatter.write_str("~1")?,
                    other => write!(formatter, "{other}")?,
                }
            }
        }
        Ok(())
    }
}

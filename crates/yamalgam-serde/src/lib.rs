//! Streaming serde Deserializer for YAML.
//!
//! Consumes the yamalgam parser event stream directly -- no intermediate DOM
//! materialization. Uses erased-serde internally so the parser never
//! monomorphizes.
#![deny(unsafe_code)]

mod de;
mod error;

pub use de::Deserializer;
pub use error::Error;

/// Deserialize a single YAML document.
///
/// Errors if the input contains multiple documents. For multi-document
/// streams, use [`Deserializer::from_str`] with [`Deserializer::documents`].
///
/// # Errors
///
/// Returns [`Error`] on parse failure, type mismatch, or multiple documents.
pub fn from_str<'de, T: serde::Deserialize<'de>>(input: &'de str) -> Result<T, Error> {
    let mut de = Deserializer::from_str(input);
    let value = T::deserialize(&mut de)?;
    de.check_end()?;
    Ok(value)
}

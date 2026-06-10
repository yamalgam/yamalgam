//! Streaming serde Deserializer for YAML.
//!
//! Consumes the yamalgam parser event stream directly -- no intermediate DOM
//! materialization. Uses erased-serde internally so the parser never
//! monomorphizes.
#![deny(unsafe_code)]

mod de;
mod documents;
mod error;

pub use de::Deserializer;
pub use documents::Documents;
pub use error::Error;

/// Deserialize a single YAML document.
///
/// Uses erased-serde internally so the parser event-walking code compiles
/// once (no monomorphization per `T`).
///
/// Errors if the input contains multiple documents. For multi-document
/// streams, use [`Deserializer::from_str`] with [`Deserializer::documents`].
///
/// # Errors
///
/// Returns [`Error`] on parse failure, type mismatch, or multiple documents.
pub fn from_str<'de, T: serde::Deserialize<'de>>(input: &'de str) -> Result<T, Error> {
    let mut de = Deserializer::from_str(input);
    let value = {
        let mut erased = <dyn erased_serde::Deserializer>::erase(&mut de);
        erased_serde::deserialize::<T>(&mut erased)?
    };
    de.check_end()?;
    Ok(value)
}

/// Deserialize a single YAML document with a full [`LoaderConfig`].
///
/// Applies both resource limits and tag resolution scheme from the config.
/// Uses erased-serde internally.
///
/// # Errors
///
/// Returns [`Error`] on parse failure, type mismatch, limit exceeded, or
/// multiple documents.
pub fn from_str_with_config<'de, T: serde::Deserialize<'de>>(
    input: &'de str,
    config: &yamalgam_core::LoaderConfig,
) -> Result<T, Error> {
    let mut de = Deserializer::from_str_with_config(input, config);
    let value = {
        let mut erased = <dyn erased_serde::Deserializer>::erase(&mut de);
        erased_serde::deserialize::<T>(&mut erased)?
    };
    de.check_end()?;
    Ok(value)
}

/// Deserialize from a reader.
///
/// Reads the entire input into a string, then deserializes. Requires
/// `T: DeserializeOwned` because the buffer is local (can't borrow).
///
/// # Errors
///
/// Returns [`Error`] on I/O failure, parse failure, type mismatch, or
/// multiple documents.
pub fn from_reader<R: std::io::Read, T: serde::de::DeserializeOwned>(
    mut reader: R,
) -> Result<T, Error> {
    let mut buf = String::new();
    reader
        .read_to_string(&mut buf)
        .map_err(|e| Error::Custom(format!("I/O error: {e}")))?;
    from_str(&buf)
}

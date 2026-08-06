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

fn deserialize_one<'de, T: serde::Deserialize<'de>>(mut de: Deserializer<'de>) -> Result<T, Error> {
    let result = {
        let mut erased = <dyn erased_serde::Deserializer>::erase(&mut de);
        erased_serde::deserialize::<T>(&mut erased)
    };
    match result {
        Ok(value) => {
            de.clear_stashed();
            de.check_end()?;
            Ok(value)
        }
        Err(error) => Err(de.restore_stashed(Error::from(error))),
    }
}

/// Deserialize a single YAML document.
///
/// Uses erased-serde internally so the parser event-walking code compiles
/// once (no monomorphization per `T`). Applies [`LoaderConfig::default`]
/// resource limits; trusted callers can opt out through
/// [`from_str_unchecked`].
///
/// Errors if the input contains multiple documents. For multi-document
/// streams, use [`Deserializer::from_str`] with [`Deserializer::documents`].
///
/// # Errors
///
/// Returns [`Error`] on parse failure, type mismatch, or multiple documents.
/// Errors keep their structured form — variants and [`Span`]s survive the
/// erased-serde boundary.
///
/// [`Span`]: yamalgam_core::Span
pub fn from_str<'de, T: serde::Deserialize<'de>>(input: &'de str) -> Result<T, Error> {
    deserialize_one(Deserializer::from_str(input))
}

/// Deserialize a single trusted YAML document without resource limits.
///
/// Prefer [`from_str`] for ordinary input.
///
/// # Errors
///
/// Returns [`Error`] on parse failure, type mismatch, or multiple documents.
pub fn from_str_unchecked<'de, T: serde::Deserialize<'de>>(input: &'de str) -> Result<T, Error> {
    deserialize_one(Deserializer::from_str_unchecked(input))
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
    deserialize_one(Deserializer::from_str_with_config(input, config))
}

/// Deserialize from a reader.
///
/// Reads the entire input into a string, then deserializes. Requires
/// `T: DeserializeOwned` because the buffer is local (can't borrow). Applies
/// [`LoaderConfig::default`] resource limits; trusted callers can opt out
/// through [`from_reader_unchecked`].
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

/// Deserialize trusted YAML from a reader without resource limits.
///
/// Reads the entire input into a string, then deserializes. Prefer
/// [`from_reader`] for ordinary input.
///
/// # Errors
///
/// Returns [`Error`] on I/O failure, parse failure, type mismatch, or
/// multiple documents.
pub fn from_reader_unchecked<R: std::io::Read, T: serde::de::DeserializeOwned>(
    mut reader: R,
) -> Result<T, Error> {
    let mut buf = String::new();
    reader
        .read_to_string(&mut buf)
        .map_err(|e| Error::Custom(format!("I/O error: {e}")))?;
    from_str_unchecked(&buf)
}

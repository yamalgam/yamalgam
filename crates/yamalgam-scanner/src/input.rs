//! Encoding detection and transcoding for YAML input.
//!
//! Handles BOM detection for UTF-8, UTF-16LE/BE, and UTF-32LE/BE,
//! then transcodes everything to internal UTF-8 representation.
//! UTF-8 input without BOM takes the zero-copy path (`Cow::Borrowed`).

use std::borrow::Cow;
use std::io::Read;

use yamalgam_core::{Diagnostic, LoaderConfig, Severity, Span};

/// Detected byte-order mark / encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Encoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    Utf32Le,
    Utf32Be,
}

/// Decoded YAML input, always UTF-8 internally.
///
/// Wraps either a borrowed `&str` (zero-copy for UTF-8 input) or an owned
/// `String` (after transcoding from UTF-16/32 or reading from a `Read`).
#[derive(Debug)]
pub struct Input<'a> {
    data: Cow<'a, str>,
}

/// Detect encoding from the first few bytes (BOM sniffing).
///
/// Returns `(encoding, bom_length)`.
fn detect_encoding(bytes: &[u8]) -> (Encoding, usize) {
    // Check 4-byte BOMs first (UTF-32 before UTF-16 to avoid false match).
    if bytes.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) {
        return (Encoding::Utf32Le, 4);
    }
    if bytes.starts_with(&[0x00, 0x00, 0xFE, 0xFF]) {
        return (Encoding::Utf32Be, 4);
    }
    // 3-byte BOM: UTF-8
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return (Encoding::Utf8, 3);
    }
    // 2-byte BOMs: UTF-16
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return (Encoding::Utf16Le, 2);
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return (Encoding::Utf16Be, 2);
    }
    // No BOM — assume UTF-8.
    (Encoding::Utf8, 0)
}

/// Decode UTF-16LE or UTF-16BE bytes (after BOM is stripped) into a `String`.
fn decode_utf16(bytes: &[u8], big_endian: bool) -> Result<String, Diagnostic> {
    let decoder = if big_endian {
        encoding_rs::UTF_16BE
    } else {
        encoding_rs::UTF_16LE
    };
    let (result, _, had_errors) = decoder.decode(bytes);
    if had_errors {
        return Err(encoding_error("invalid UTF-16 sequence"));
    }
    Ok(result.into_owned())
}

/// Decode UTF-32LE or UTF-32BE bytes (after BOM is stripped) into a `String`.
///
/// `encoding_rs` doesn't support UTF-32, so we do it manually.
fn decode_utf32(bytes: &[u8], big_endian: bool) -> Result<String, Diagnostic> {
    if !bytes.len().is_multiple_of(4) {
        return Err(encoding_error(
            "UTF-32 input length is not a multiple of 4 bytes",
        ));
    }
    let mut out = String::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        let code_point = if big_endian {
            u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        } else {
            u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        };
        let ch = char::from_u32(code_point).ok_or_else(|| {
            encoding_error(&format!("invalid UTF-32 code point: U+{code_point:04X}"))
        })?;
        out.push(ch);
    }
    Ok(out)
}

/// Build a `Diagnostic` for an encoding error.
fn encoding_error(msg: &str) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        code: "E0001".to_string(),
        message: format!("encoding error: {msg}"),
        span: Some(Span::default()),
        labels: Vec::new(),
    }
}

impl<'a> Input<'a> {
    /// Create an `Input` from raw bytes, detecting encoding via BOM.
    ///
    /// UTF-8 input (with or without BOM) takes the zero-copy path when possible.
    /// UTF-16 and UTF-32 inputs are transcoded to owned UTF-8.
    ///
    /// # Errors
    ///
    /// Returns a [`Diagnostic`] if the input contains invalid sequences for the
    /// detected encoding.
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, Diagnostic> {
        let (encoding, bom_len) = detect_encoding(bytes);
        let payload = &bytes[bom_len..];

        let data = match encoding {
            Encoding::Utf8 => {
                let s = std::str::from_utf8(payload)
                    .map_err(|e| encoding_error(&format!("invalid UTF-8: {e}")))?;
                Cow::Borrowed(s)
            }
            Encoding::Utf16Le => Cow::Owned(decode_utf16(payload, false)?),
            Encoding::Utf16Be => Cow::Owned(decode_utf16(payload, true)?),
            Encoding::Utf32Le => Cow::Owned(decode_utf32(payload, false)?),
            Encoding::Utf32Be => Cow::Owned(decode_utf32(payload, true)?),
        };

        Ok(Self { data })
    }

    /// Create an `Input` by reading all bytes from a reader, then detecting encoding.
    ///
    /// Always produces owned data.
    ///
    /// # Errors
    ///
    /// Returns a [`Diagnostic`] on I/O errors or invalid encoding.
    pub fn from_reader(mut reader: impl Read) -> Result<Input<'static>, Diagnostic> {
        let mut buf = Vec::new();
        reader
            .read_to_end(&mut buf)
            .map_err(|e| encoding_error(&format!("I/O error: {e}")))?;
        Self::decode_buf(buf)
    }

    /// Create an `Input` by reading from a reader with size-limit enforcement.
    ///
    /// When `config.limits.max_input_bytes` is `Some(max)`, at most `max + 1`
    /// bytes are read from the reader. If the read yields more than `max`
    /// bytes, a [`Diagnostic`] error is returned before any decoding occurs.
    /// This prevents OOM when reading untrusted input from a stream.
    ///
    /// When the limit is `None`, the entire reader is consumed.
    ///
    /// # Errors
    ///
    /// Returns a [`Diagnostic`] on I/O errors, size-limit violations, or
    /// invalid encoding.
    pub fn from_reader_with_config(
        reader: impl Read,
        config: &LoaderConfig,
    ) -> Result<Input<'static>, Diagnostic> {
        let mut buf = Vec::new();
        if let Some(max) = config.limits.max_input_bytes {
            // Read at most max+1 bytes. If we get more than max, the input
            // exceeds the limit — reject without reading the full stream.
            let mut limited = reader.take((max as u64) + 1);
            limited
                .read_to_end(&mut buf)
                .map_err(|e| encoding_error(&format!("I/O error: {e}")))?;
            if buf.len() > max {
                return Err(encoding_error(&format!(
                    "input size {} bytes exceeds maximum of {max} bytes",
                    buf.len()
                )));
            }
        } else {
            let mut unlimited = reader;
            unlimited
                .read_to_end(&mut buf)
                .map_err(|e| encoding_error(&format!("I/O error: {e}")))?;
        }
        Self::decode_buf(buf)
    }

    /// Detect encoding and decode a raw byte buffer into an owned `Input`.
    fn decode_buf(mut buf: Vec<u8>) -> Result<Input<'static>, Diagnostic> {
        let (encoding, bom_len) = detect_encoding(&buf);
        let owned = match encoding {
            Encoding::Utf8 => {
                // Strip BOM bytes then validate+convert in a single pass.
                if bom_len > 0 {
                    buf.drain(..bom_len);
                }
                String::from_utf8(buf)
                    .map_err(|e| encoding_error(&format!("invalid UTF-8: {e}")))?
            }
            Encoding::Utf16Le => decode_utf16(&buf[bom_len..], false)?,
            Encoding::Utf16Be => decode_utf16(&buf[bom_len..], true)?,
            Encoding::Utf32Le => decode_utf32(&buf[bom_len..], false)?,
            Encoding::Utf32Be => decode_utf32(&buf[bom_len..], true)?,
        };
        Ok(Input {
            data: Cow::Owned(owned),
        })
    }

    /// Return the decoded UTF-8 content.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.data
    }
}

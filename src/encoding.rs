//! A module for wrappers that encode / decode data.

use std::borrow::Cow;

#[cfg(feature = "encoding")]
use encoding_rs::{Encoding, UTF_16BE, UTF_16LE, UTF_8};

#[cfg(feature = "encoding")]
use crate::Error;
use crate::Result;

/// Unicode "byte order mark" encoded as UTF-8
pub(crate) const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
/// Unicode "byte order mark" encoded as UTF-16 with little-endian byte order
#[allow(dead_code)]
pub(crate) const UTF16_LE_BOM: &[u8] = &[0xFF, 0xFE];
/// Unicode "byte order mark" encoded as UTF-16 with big-endian byte order
#[allow(dead_code)]
pub(crate) const UTF16_BE_BOM: &[u8] = &[0xFE, 0xFF];

/// Decoder of byte slices into strings.
///
/// If feature `encoding` is enabled, this encoding taken from the `"encoding"`
/// XML declaration or assumes UTF-8, if XML has no <?xml ?> declaration, encoding
/// key is not defined or contains unknown encoding.
///
/// The library supports any UTF-8 compatible encodings that crate `encoding_rs`
/// is supported. [*UTF-16 is not supported at the present*][utf16].
///
/// If feature `encoding` is disabled, the decoder is always UTF-8 decoder:
/// any XML declarations are ignored.
///
/// [utf16]: https://github.com/tafia/quick-xml/issues/158
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Decoder {
    #[cfg(feature = "encoding")]
    pub(crate) encoding: &'static Encoding,
}

impl Decoder {
    pub(crate) fn utf8() -> Self {
        Decoder {
            #[cfg(feature = "encoding")]
            encoding: UTF_8,
        }
    }

    #[cfg(all(test, feature = "encoding", feature = "serialize"))]
    pub(crate) fn utf16() -> Self {
        Decoder { encoding: UTF_16LE }
    }
}

#[cfg(not(feature = "encoding"))]
impl Decoder {
    /// Decodes a UTF8 slice regardless of XML declaration and ignoring BOM if
    /// it is present in the `bytes`.
    ///
    /// Returns an error in case of malformed sequences in the `bytes`.
    ///
    /// If you instead want to use XML declared encoding, use the `encoding` feature
    #[inline]
    pub fn decode<'b>(&self, bytes: &'b [u8]) -> Result<Cow<'b, str>> {
        Ok(Cow::Borrowed(std::str::from_utf8(bytes)?))
    }

    /// Decodes a slice regardless of XML declaration with BOM removal if
    /// it is present in the `bytes`.
    ///
    /// Returns an error in case of malformed sequences in the `bytes`.
    ///
    /// If you instead want to use XML declared encoding, use the `encoding` feature
    pub fn decode_with_bom_removal<'b>(&self, bytes: &'b [u8]) -> Result<Cow<'b, str>> {
        let bytes = if bytes.starts_with(UTF8_BOM) {
            &bytes[3..]
        } else {
            bytes
        };
        self.decode(bytes)
    }
}

#[cfg(feature = "encoding")]
impl Decoder {
    /// Returns the `Reader`s encoding.
    ///
    /// This encoding will be used by [`decode`].
    ///
    /// [`decode`]: Self::decode
    pub fn encoding(&self) -> &'static Encoding {
        self.encoding
    }

    /// Decodes specified bytes using encoding, declared in the XML, if it was
    /// declared there, or UTF-8 otherwise, and ignoring BOM if it is present
    /// in the `bytes`.
    ///
    /// Returns an error in case of malformed sequences in the `bytes`.
    pub fn decode<'b>(&self, bytes: &'b [u8]) -> Result<Cow<'b, str>> {
        decode(bytes, self.encoding)
    }

    /// Decodes a slice with BOM removal if it is present in the `bytes` using
    /// the reader encoding.
    ///
    /// If this method called after reading XML declaration with the `"encoding"`
    /// key, then this encoding is used, otherwise UTF-8 is used.
    ///
    /// If XML declaration is absent in the XML, UTF-8 is used.
    ///
    /// Returns an error in case of malformed sequences in the `bytes`.
    pub fn decode_with_bom_removal<'b>(&self, bytes: &'b [u8]) -> Result<Cow<'b, str>> {
        self.decode(remove_bom(bytes, self.encoding))
    }
}

/// Decodes the provided bytes using the specified encoding.
///
/// Returns an error in case of malformed or non-representable sequences in the `bytes`.
#[cfg(feature = "encoding")]
pub fn decode<'b>(bytes: &'b [u8], encoding: &'static Encoding) -> Result<Cow<'b, str>> {
    encoding
        .decode_without_bom_handling_and_without_replacement(bytes)
        .ok_or(Error::NonDecodable(None))
}

/// Decodes a slice with an unknown encoding, removing the BOM if it is present
/// in the bytes.
///
/// Returns an error in case of malformed or non-representable sequences in the `bytes`.
#[cfg(feature = "encoding")]
pub fn decode_with_bom_removal<'b>(bytes: &'b [u8]) -> Result<Cow<'b, str>> {
    if let Some(encoding) = detect_encoding(bytes) {
        let bytes = remove_bom(bytes, encoding);
        decode(bytes, encoding)
    } else {
        decode(bytes, UTF_8)
    }
}

#[cfg(feature = "encoding")]
fn split_at_bom<'b>(bytes: &'b [u8], encoding: &'static Encoding) -> (&'b [u8], &'b [u8]) {
    if encoding == UTF_8 && bytes.starts_with(UTF8_BOM) {
        bytes.split_at(3)
    } else if encoding == UTF_16LE && bytes.starts_with(UTF16_LE_BOM) {
        bytes.split_at(2)
    } else if encoding == UTF_16BE && bytes.starts_with(UTF16_BE_BOM) {
        bytes.split_at(2)
    } else {
        (&[], bytes)
    }
}

#[cfg(feature = "encoding")]
fn remove_bom<'b>(bytes: &'b [u8], encoding: &'static Encoding) -> &'b [u8] {
    let (_, bytes) = split_at_bom(bytes, encoding);
    bytes
}

/// Automatic encoding detection of XML files based using the
/// [recommended algorithm](https://www.w3.org/TR/xml11/#sec-guessing).
///
/// If encoding is detected, `Some` is returned, otherwise `None` is returned.
///
/// Because the [`encoding_rs`] crate supports only subset of those encodings, only
/// the supported subset are detected, which is UTF-8, UTF-16 BE and UTF-16 LE.
///
/// The algorithm suggests examine up to the first 4 bytes to determine encoding
/// according to the following table:
///
/// | Bytes       |Detected encoding
/// |-------------|------------------------------------------
/// |`FE FF ## ##`|UTF-16, big-endian
/// |`FF FE ## ##`|UTF-16, little-endian
/// |`EF BB BF`   |UTF-8
/// |-------------|------------------------------------------
/// |`00 3C 00 3F`|UTF-16 BE or ISO-10646-UCS-2 BE or similar 16-bit BE (use declared encoding to find the exact one)
/// |`3C 00 3F 00`|UTF-16 LE or ISO-10646-UCS-2 LE or similar 16-bit LE (use declared encoding to find the exact one)
/// |`3C 3F 78 6D`|UTF-8, ISO 646, ASCII, some part of ISO 8859, Shift-JIS, EUC, or any other 7-bit, 8-bit, or mixed-width encoding which ensures that the characters of ASCII have their normal positions, width, and values; the actual encoding declaration must be read to detect which of these applies, but since all of these encodings use the same bit patterns for the relevant ASCII characters, the encoding declaration itself may be read reliably
#[cfg(feature = "encoding")]
pub fn detect_encoding(bytes: &[u8]) -> Option<&'static Encoding> {
    match bytes {
        // with BOM
        _ if bytes.starts_with(UTF16_BE_BOM) => Some(UTF_16BE),
        _ if bytes.starts_with(UTF16_LE_BOM) => Some(UTF_16LE),
        _ if bytes.starts_with(UTF8_BOM) => Some(UTF_8),

        // without BOM
        _ if bytes.starts_with(&[0x00, b'<', 0x00, b'?']) => Some(UTF_16BE), // Some BE encoding, for example, UTF-16 or ISO-10646-UCS-2
        _ if bytes.starts_with(&[b'<', 0x00, b'?', 0x00]) => Some(UTF_16LE), // Some LE encoding, for example, UTF-16 or ISO-10646-UCS-2
        _ if bytes.starts_with(&[b'<', b'?', b'x', b'm']) => Some(UTF_8), // Some ASCII compatible

        _ => None,
    }
}

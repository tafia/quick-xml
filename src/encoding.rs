//! A module for wrappers that encode / decode data.

use std::borrow::Cow;

#[cfg(feature = "encoding")]
use encoding_rs::{Encoding, UTF_16BE, UTF_16LE, UTF_8};

use crate::{Error, Result};

/// Decoder of byte slices to the strings. This is lightweight object that can be copied.
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
#[derive(Clone, Copy, Debug)]
pub struct Decoder {
    #[cfg(feature = "encoding")]
    pub(crate) encoding: &'static Encoding,
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
        let bytes = if bytes.starts_with(b"\xEF\xBB\xBF") {
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
        match self
            .encoding
            .decode_without_bom_handling_and_without_replacement(bytes)
        {
            None => Err(Error::NonDecodable(None)),
            Some(s) => Ok(s),
        }
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
        self.decode(self.remove_bom(bytes))
    }
    /// Copied from [`Encoding::decode_with_bom_removal`]
    #[inline]
    fn remove_bom<'b>(&self, bytes: &'b [u8]) -> &'b [u8] {
        if self.encoding == UTF_8 && bytes.starts_with(b"\xEF\xBB\xBF") {
            return &bytes[3..];
        }
        if self.encoding == UTF_16LE && bytes.starts_with(b"\xFF\xFE") {
            return &bytes[2..];
        }
        if self.encoding == UTF_16BE && bytes.starts_with(b"\xFE\xFF") {
            return &bytes[2..];
        }

        bytes
    }
}

/// This implementation is required for tests of other parts of the library
#[cfg(test)]
#[cfg(feature = "serialize")]
impl Decoder {
    pub(crate) fn utf8() -> Self {
        Decoder {
            #[cfg(feature = "encoding")]
            encoding: UTF_8,
        }
    }

    #[cfg(feature = "encoding")]
    pub(crate) fn utf16() -> Self {
        Decoder { encoding: UTF_16LE }
    }
}

/// Automatic encoding detection of XML files based using the [recommended algorithm]
/// (https://www.w3.org/TR/xml11/#sec-guessing)
///
/// The algorithm suggests examine up to the first 4 bytes to determine encoding
/// according to the following table:
///
/// | Bytes       |Detected encoding
/// |-------------|------------------------------------------
/// |`00 00 FE FF`|UCS-4, big-endian machine (1234 order)
/// |`FF FE 00 00`|UCS-4, little-endian machine (4321 order)
/// |`00 00 FF FE`|UCS-4, unusual octet order (2143)
/// |`FE FF 00 00`|UCS-4, unusual octet order (3412)
/// |`FE FF ## ##`|UTF-16, big-endian
/// |`FF FE ## ##`|UTF-16, little-endian
/// |`EF BB BF`   |UTF-8
/// |-------------|------------------------------------------
/// |`00 00 00 3C`|UCS-4 or similar (use declared encoding to find the exact one), in big-endian (1234)
/// |`3C 00 00 00`|UCS-4 or similar (use declared encoding to find the exact one), in little-endian (4321)
/// |`00 00 3C 00`|UCS-4 or similar (use declared encoding to find the exact one), in unusual byte orders (2143)
/// |`00 3C 00 00`|UCS-4 or similar (use declared encoding to find the exact one), in unusual byte orders (3412)
/// |`00 3C 00 3F`|UTF-16 BE or ISO-10646-UCS-2 BE or similar 16-bit BE (use declared encoding to find the exact one)
/// |`3C 00 3F 00`|UTF-16 LE or ISO-10646-UCS-2 LE or similar 16-bit LE (use declared encoding to find the exact one)
/// |`3C 3F 78 6D`|UTF-8, ISO 646, ASCII, some part of ISO 8859, Shift-JIS, EUC, or any other 7-bit, 8-bit, or mixed-width encoding which ensures that the characters of ASCII have their normal positions, width, and values; the actual encoding declaration must be read to detect which of these applies, but since all of these encodings use the same bit patterns for the relevant ASCII characters, the encoding declaration itself may be read reliably
/// |`4C 6F A7 94`|EBCDIC (in some flavor; the full encoding declaration must be read to tell which code page is in use)
/// |_Other_      |UTF-8 without an encoding declaration, or else the data stream is mislabeled (lacking a required encoding declaration), corrupt, fragmentary, or enclosed in a wrapper of some kind
///
/// Because [`encoding_rs`] crate supported only subset of those encodings, only
/// supported subset are detected, which is UTF-8, UTF-16 BE and UTF-16 LE.
///
/// If encoding is detected, `Some` is returned, otherwise `None` is returned.
#[cfg(feature = "encoding")]
pub(crate) fn detect_encoding(bytes: &[u8]) -> Option<&'static Encoding> {
    match bytes {
        // with BOM
        _ if bytes.starts_with(&[0xFE, 0xFF]) => Some(UTF_16BE),
        _ if bytes.starts_with(&[0xFF, 0xFE]) => Some(UTF_16LE),
        _ if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) => Some(UTF_8),

        // without BOM
        _ if bytes.starts_with(&[0x00, b'<', 0x00, b'?']) => Some(UTF_16BE), // Some BE encoding, for example, UTF-16 or ISO-10646-UCS-2
        _ if bytes.starts_with(&[b'<', 0x00, b'?', 0x00]) => Some(UTF_16LE), // Some LE encoding, for example, UTF-16 or ISO-10646-UCS-2
        _ if bytes.starts_with(&[b'<', b'?', b'x', b'm']) => Some(UTF_8), // Some ASCII compatible

        _ => None,
    }
}

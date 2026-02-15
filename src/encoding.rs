//! A module for wrappers that encode / decode data.

use std::borrow::Cow;
use std::io::{self, Read};
use std::str::Utf8Error;

#[cfg(feature = "encoding")]
use encoding_rs;

/// Unicode "byte order mark" (\u{FEFF}) encoded as UTF-8.
/// See <https://unicode.org/faq/utf_bom.html#bom1>
pub(crate) const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
/// Unicode "byte order mark" (\u{FEFF}) encoded as UTF-16 with little-endian byte order.
/// See <https://unicode.org/faq/utf_bom.html#bom1>
pub(crate) const UTF16_LE_BOM: &[u8] = &[0xFF, 0xFE];
/// Unicode "byte order mark" (\u{FEFF}) encoded as UTF-16 with big-endian byte order.
/// See <https://unicode.org/faq/utf_bom.html#bom1>
pub(crate) const UTF16_BE_BOM: &[u8] = &[0xFE, 0xFF];

/// An error when decoding or encoding
///
/// If feature [`encoding`] is disabled, the [`EncodingError`] is always [`EncodingError::Utf8`]
///
/// [`encoding`]: ../index.html#encoding
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EncodingError {
    /// Input was not valid UTF-8
    Utf8(Utf8Error),
    /// Input did not adhere to the given encoding
    #[cfg(feature = "encoding")]
    Other(&'static encoding_rs::Encoding),
}

impl From<Utf8Error> for EncodingError {
    #[inline]
    fn from(e: Utf8Error) -> Self {
        Self::Utf8(e)
    }
}

impl std::error::Error for EncodingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Utf8(e) => Some(e),
            #[cfg(feature = "encoding")]
            Self::Other(_) => None,
        }
    }
}

impl std::fmt::Display for EncodingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Utf8(e) => write!(f, "cannot decode input using UTF-8: {}", e),
            #[cfg(feature = "encoding")]
            Self::Other(encoding) => write!(f, "cannot decode input using {}", encoding.name()),
        }
    }
}

/// Decoder of byte slices into strings.
///
/// If feature [`encoding`] is enabled, this encoding taken from the `"encoding"`
/// XML declaration or assumes UTF-8, if XML has no <?xml ?> declaration, encoding
/// key is not defined or contains unknown encoding.
///
/// The library supports any UTF-8 compatible encodings that crate `encoding_rs`
/// is supported. [*UTF-16 and ISO-2022-JP are not supported at the present*][utf16].
///
/// If feature [`encoding`] is disabled, the decoder is always UTF-8 decoder:
/// any XML declarations are ignored.
///
/// [utf16]: https://github.com/tafia/quick-xml/issues/158
/// [`encoding`]: ../index.html#encoding
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Decoder {
    #[cfg(feature = "encoding")]
    pub(crate) encoding: &'static encoding_rs::Encoding,
}

impl Decoder {
    pub(crate) const fn utf8() -> Self {
        Decoder {
            #[cfg(feature = "encoding")]
            encoding: encoding_rs::UTF_8,
        }
    }

    #[cfg(all(test, feature = "encoding", feature = "serialize"))]
    pub(crate) const fn utf16() -> Self {
        Decoder {
            encoding: encoding_rs::UTF_16LE,
        }
    }
}

impl Decoder {
    /// Returns the `Reader`s encoding.
    ///
    /// This encoding will be used by [`decode`].
    ///
    /// [`decode`]: Self::decode
    #[cfg(feature = "encoding")]
    pub const fn encoding(&self) -> &'static encoding_rs::Encoding {
        self.encoding
    }

    /// ## Without `encoding` feature
    ///
    /// Decodes an UTF-8 slice regardless of XML declaration and ignoring BOM
    /// if it is present in the `bytes`.
    ///
    /// ## With `encoding` feature
    ///
    /// Decodes specified bytes using encoding, declared in the XML, if it was
    /// declared there, or UTF-8 otherwise, and ignoring BOM if it is present
    /// in the `bytes`.
    ///
    /// ----
    /// Returns an error in case of malformed sequences in the `bytes`.
    pub fn decode<'b>(&self, bytes: &'b [u8]) -> Result<Cow<'b, str>, EncodingError> {
        #[cfg(not(feature = "encoding"))]
        let decoded = Ok(Cow::Borrowed(std::str::from_utf8(bytes)?));

        #[cfg(feature = "encoding")]
        let decoded = decode(bytes, self.encoding);

        decoded
    }

    /// Like [`decode`][Self::decode] but using a pre-allocated buffer.
    pub fn decode_into(&self, bytes: &[u8], buf: &mut String) -> Result<(), EncodingError> {
        #[cfg(not(feature = "encoding"))]
        buf.push_str(std::str::from_utf8(bytes)?);

        #[cfg(feature = "encoding")]
        decode_into(bytes, self.encoding, buf)?;

        Ok(())
    }

    /// Decodes the `Cow` buffer, preserves the lifetime
    pub(crate) fn decode_cow<'b>(
        &self,
        bytes: &Cow<'b, [u8]>,
    ) -> Result<Cow<'b, str>, EncodingError> {
        match bytes {
            Cow::Borrowed(bytes) => self.decode(bytes),
            // Convert to owned, because otherwise Cow will be bound with wrong lifetime
            Cow::Owned(bytes) => Ok(self.decode(bytes)?.into_owned().into()),
        }
    }

    /// Decodes the `Cow` buffer, normalizes XML EOLs, preserves the lifetime
    pub(crate) fn content<'b>(
        &self,
        bytes: &Cow<'b, [u8]>,
        normalize_eol: impl Fn(&str) -> Cow<str>,
    ) -> Result<Cow<'b, str>, EncodingError> {
        match bytes {
            Cow::Borrowed(bytes) => {
                let text = self.decode(bytes)?;
                match normalize_eol(&text) {
                    // If text borrowed after normalization that means that it's not changed
                    Cow::Borrowed(_) => Ok(text),
                    Cow::Owned(s) => Ok(Cow::Owned(s)),
                }
            }
            Cow::Owned(bytes) => {
                let text = self.decode(bytes)?;
                let text = normalize_eol(&text);
                // Convert to owned, because otherwise Cow will be bound with wrong lifetime
                Ok(text.into_owned().into())
            }
        }
    }
}

/// Decodes the provided bytes using the specified encoding.
///
/// Returns an error in case of malformed or non-representable sequences in the `bytes`.
#[cfg(feature = "encoding")]
pub fn decode<'b>(
    bytes: &'b [u8],
    encoding: &'static encoding_rs::Encoding,
) -> Result<Cow<'b, str>, EncodingError> {
    encoding
        .decode_without_bom_handling_and_without_replacement(bytes)
        .ok_or(EncodingError::Other(encoding))
}

/// Like [`decode`] but using a pre-allocated buffer.
#[cfg(feature = "encoding")]
pub fn decode_into(
    bytes: &[u8],
    encoding: &'static encoding_rs::Encoding,
    buf: &mut String,
) -> Result<(), EncodingError> {
    if encoding == encoding_rs::UTF_8 {
        buf.push_str(std::str::from_utf8(bytes)?);
        return Ok(());
    }

    let mut decoder = encoding.new_decoder_without_bom_handling();
    buf.reserve(
        decoder
            .max_utf8_buffer_length_without_replacement(bytes.len())
            // SAFETY: None can be returned only if required size will overflow usize,
            // but in that case String::reserve also panics
            .unwrap(),
    );
    let (result, read) = decoder.decode_to_string_without_replacement(bytes, buf, true);
    match result {
        encoding_rs::DecoderResult::InputEmpty => {
            debug_assert_eq!(read, bytes.len());
            Ok(())
        }
        encoding_rs::DecoderResult::Malformed(_, _) => Err(EncodingError::Other(encoding)),
        // SAFETY: We allocate enough space above
        encoding_rs::DecoderResult::OutputFull => unreachable!(),
    }
}

/// Automatic encoding detection of XML files based using the
/// [recommended algorithm](https://www.w3.org/TR/xml11/#sec-guessing).
///
/// If encoding is detected, `Some` is returned with a [`DetectedEncoding`] that provides
/// the BOM size in bytes (or zero if no BOM was present).
///
/// IF encoding was not recognized, `None` is returned.
///
/// Because the [`encoding_rs`] crate supports only subset of those encodings, only
/// the supported subset are detected, which is UTF-8, UTF-16 BE and UTF-16 LE.
///
/// The algorithm suggests examine up to the first 4 bytes to determine encoding
/// according to the following table:
///
/// | Bytes       |Detected encoding
/// |-------------|------------------------------------------
/// | **BOM**
/// |`FE_FF_##_##`|UTF-16, big-endian
/// |`FF FE ## ##`|UTF-16, little-endian
/// |`EF BB BF`   |UTF-8
/// | **No BOM**
/// |`00 3C 00 3F`|UTF-16 BE or ISO-10646-UCS-2 BE or similar 16-bit BE (use declared encoding to find the exact one)
/// |`3C 00 3F 00`|UTF-16 LE or ISO-10646-UCS-2 LE or similar 16-bit LE (use declared encoding to find the exact one)
/// |`3C 3F 78 6D`|UTF-8, ISO 646, ASCII, some part of ISO 8859, Shift-JIS, EUC, or any other 7-bit, 8-bit, or mixed-width encoding which ensures that the characters of ASCII have their normal positions, width, and values; the actual encoding declaration must be read to detect which of these applies, but since all of these encodings use the same bit patterns for the relevant ASCII characters, the encoding declaration itself may be read reliably
pub fn detect_encoding(bytes: &[u8]) -> Option<DetectedEncoding> {
    // Prevent suggesting "<?xm". We want to have the same formatted lines for all arms.
    #[allow(clippy::byte_char_slices)]
    match bytes {
        // with BOM
        _ if bytes.starts_with(UTF16_BE_BOM) => Some(DetectedEncoding::Utf16BeBom),
        _ if bytes.starts_with(UTF16_LE_BOM) => Some(DetectedEncoding::Utf16LeBom),
        _ if bytes.starts_with(UTF8_BOM) => Some(DetectedEncoding::Utf8Bom),

        // without BOM
        _ if bytes.starts_with(&[0x00, b'<', 0x00, b'?']) => Some(DetectedEncoding::Utf16BeLike), // Some BE encoding, for example, UTF-16 or ISO-10646-UCS-2
        _ if bytes.starts_with(&[b'<', 0x00, b'?', 0x00]) => Some(DetectedEncoding::Utf16LeLike), // Some LE encoding, for example, UTF-16 or ISO-10646-UCS-2
        _ if bytes.starts_with(&[b'<', b'?', b'x', b'm']) => {
            Some(DetectedEncoding::AsciiCompatible)
        } // Some ASCII compatible

        _ => None,
    }
}

/// Possible scenarios for start-of-xml detection of encoding
///
/// See the documentation of [`detect_encoding`]
pub enum DetectedEncoding {
    /// Matches UTF-8 or some other ascii-compatible encoding
    AsciiCompatible,
    /// We saw a UTF-8 BOM
    Utf8Bom,
    /// Matches UTF-16-LE or some other UTF-16 compatible encoding (e.g. ISO-10646-UCS-2)
    Utf16LeLike,
    /// We saw a UTF-16 BOM in little-endian orientation
    Utf16LeBom,
    /// Matches UTF-16-BE or some other UTF-16 compatible encoding (e.g. ISO-10646-UCS-2)
    Utf16BeLike,
    /// We saw a UTF-16 BOM in big-endian orientation
    Utf16BeBom,
}

impl DetectedEncoding {
    /// Return an Encoding object appropriate for the detected encoding
    #[cfg(feature = "encoding")]
    pub const fn encoding(&self) -> &'static encoding_rs::Encoding {
        match self {
            DetectedEncoding::AsciiCompatible | DetectedEncoding::Utf8Bom => encoding_rs::UTF_8,
            DetectedEncoding::Utf16LeLike | DetectedEncoding::Utf16LeBom => encoding_rs::UTF_16LE,
            DetectedEncoding::Utf16BeLike | DetectedEncoding::Utf16BeBom => encoding_rs::UTF_16BE,
        }
    }

    /// Length of the BOM, which may need to be stripped from the input
    pub const fn bom_len(&self) -> usize {
        match self {
            DetectedEncoding::Utf8Bom => 3,
            DetectedEncoding::Utf16LeBom | DetectedEncoding::Utf16BeBom => 2,
            DetectedEncoding::AsciiCompatible
            | DetectedEncoding::Utf16LeLike
            | DetectedEncoding::Utf16BeLike => 0,
        }
    }
}

/// A reader wrapper that ensures only valid UTF-8 bytes are read.
///
/// This reader uses [`str::from_utf8()`] and [`Utf8Error::valid_up_to()`] to validate
/// that only valid UTF-8 bytes are written to the output buffer. Incomplete UTF-8
/// sequences at read boundaries are buffered and combined with subsequent reads.
///
/// # Examples
///
/// ```
/// use std::io::Read;
/// use quick_xml::encoding::Utf8ValidatingReader;
///
/// let data = b"Hello, \xF0\x9F\x98\x80!"; // "Hello, 😀!"
/// let mut reader = Utf8ValidatingReader::new(&data[..]);
/// let mut buf = [0u8; 20];
/// let n = reader.read(&mut buf).unwrap();
/// assert_eq!(&buf[..n], data);
/// ```
#[derive(Debug)]
pub struct Utf8ValidatingReader<R> {
    inner: R,
    /// Buffer to hold incomplete UTF-8 sequences from previous reads (max 3 bytes)
    buffer: Vec<u8>,
}

impl<R> Utf8ValidatingReader<R> {
    /// Creates a new UTF-8 validating reader
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            buffer: Vec::with_capacity(4),
        }
    }

    /// Returns a reference to the underlying reader
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Returns a mutable reference to the underlying reader
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Consumes this reader and returns the underlying reader
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: Read> Read for Utf8ValidatingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        loop {
            // If we have buffered data, check if it's complete UTF-8
            if !self.buffer.is_empty() {
                match std::str::from_utf8(&self.buffer) {
                    Ok(s) => {
                        // All buffered bytes are valid UTF-8
                        // Find how many complete characters fit in the buffer
                        let mut bytes_to_copy = 0;
                        for (idx, _) in s.char_indices() {
                            if idx > buf.len() {
                                break;
                            }
                            bytes_to_copy = idx;
                        }
                        // Also consider the last character
                        if s.len() <= buf.len() {
                            bytes_to_copy = s.len();
                        }

                        if bytes_to_copy == 0 {
                            // Buffer too small for even one character
                            return Ok(0);
                        }

                        buf[..bytes_to_copy].copy_from_slice(&self.buffer[..bytes_to_copy]);
                        self.buffer.drain(..bytes_to_copy);
                        return Ok(bytes_to_copy);
                    }
                    Err(e) => {
                        let valid_up_to = e.valid_up_to();

                        if let Some(error_len) = e.error_len() {
                            // Invalid UTF-8 sequence found
                            if valid_up_to == 0 {
                                return Err(io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    format!("invalid UTF-8 sequence of {} bytes", error_len),
                                ));
                            }
                            // Write valid portion before the error
                            let len = valid_up_to.min(buf.len());
                            buf[..len].copy_from_slice(&self.buffer[..len]);

                            // Remove only the valid bytes, leave invalid bytes to error on next read
                            self.buffer.drain(..valid_up_to);
                            return Ok(len);
                        } else {
                            // Incomplete UTF-8 sequence - need to read more
                            // But first, if we have valid bytes, return them
                            if valid_up_to > 0 {
                                let len = valid_up_to.min(buf.len());
                                buf[..len].copy_from_slice(&self.buffer[..len]);
                                self.buffer.drain(..len);
                                return Ok(len);
                            }
                            // Otherwise fall through to read more data
                        }
                    }
                }
            }

            // Read more data from the underlying reader directly into self.buffer
            let read_size = buf.len().max(64); // Read at least 64 bytes for efficiency
            let buf_start = self.buffer.len();
            self.buffer.resize(buf_start + read_size, 0);
            let n = self.inner.read(&mut self.buffer[buf_start..])?;

            // Trim buffer to actual bytes read
            self.buffer.truncate(buf_start + n);

            // If we read nothing
            if n == 0 {
                if buf_start == 0 {
                    // True EOF with no buffered data
                    return Ok(0);
                } else {
                    // EOF with incomplete UTF-8 sequence
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "incomplete UTF-8 sequence at end of stream",
                    ));
                }
            }

            // Loop back to validate and potentially return data
        }
    }
}

#[cfg(test)]
mod utf8_validating_reader_tests {
    use super::*;
    use std::io::{Cursor, Read};

    /// Helper reader that returns data in fixed-size chunks
    struct ChunkedReader<'a> {
        data: &'a [u8],
        pos: usize,
        chunk_size: usize,
    }

    impl<'a> ChunkedReader<'a> {
        fn new(data: &'a [u8], chunk_size: usize) -> Self {
            Self {
                data,
                pos: 0,
                chunk_size,
            }
        }
    }

    impl<'a> Read for ChunkedReader<'a> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.pos >= self.data.len() {
                return Ok(0);
            }
            let len = self
                .chunk_size
                .min(buf.len())
                .min(self.data.len() - self.pos);
            buf[..len].copy_from_slice(&self.data[self.pos..self.pos + len]);
            self.pos += len;
            Ok(len)
        }
    }

    mod basic_access {
        use super::*;

        #[test]
        fn test_get_ref() {
            let data = b"Hello";
            let cursor = Cursor::new(data.to_vec());
            let reader = Utf8ValidatingReader::new(cursor);

            assert_eq!(reader.get_ref().get_ref(), data);
        }

        #[test]
        fn test_get_mut() {
            let data = b"Hello";
            let cursor = Cursor::new(data.to_vec());
            let mut reader = Utf8ValidatingReader::new(cursor);

            reader.get_mut().set_position(2);
            assert_eq!(reader.get_ref().position(), 2);
        }

        #[test]
        fn test_into_inner() {
            let data = b"Hello";
            let cursor = Cursor::new(data.to_vec());
            let reader = Utf8ValidatingReader::new(cursor);

            let inner = reader.into_inner();
            assert_eq!(inner.get_ref(), data);
        }
    }

    mod valid_utf8 {
        use super::*;

        #[test]
        fn valid_ascii() {
            let data = b"Hello, World!";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 20];
            let n = reader.read(&mut buf).unwrap();
            assert_eq!(n, 13);
            assert_eq!(&buf[..n], data);
        }

        #[test]
        fn valid_multibyte_characters() {
            // Mix of 1, 2, 3, and 4 byte UTF-8 sequences
            let data = "Hello, £€ 世界! 😀".as_bytes(); // ASCII + 2x2-byte + ASCII + 2x3-byte + ASCII + 4-byte
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = vec![0u8; 100];
            let n = reader.read(&mut buf).unwrap();
            assert_eq!(&buf[..n], data);
        }

        #[test]
        fn empty_input() {
            let data = b"";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];
            let n = reader.read(&mut buf).unwrap();
            assert_eq!(n, 0);
        }

        #[test]
        fn empty_buffer() {
            let data = b"Hello, World!";
            let mut reader = Utf8ValidatingReader::new(&data[..]);

            // Read with empty buffer - should return 0 without affecting state
            let mut empty_buf = [];
            let n = reader.read(&mut empty_buf).unwrap();
            assert_eq!(n, 0);

            // Read with actual buffer - should get data
            let mut buf = [0u8; 5];
            let n = reader.read(&mut buf).unwrap();
            assert_eq!(n, 5);
            assert_eq!(&buf[..n], b"Hello");

            // Read with empty buffer again - should return 0 without affecting state
            let n = reader.read(&mut empty_buf).unwrap();
            assert_eq!(n, 0);

            // Read remaining data - should continue from where we left off
            let mut buf2 = [0u8; 20];
            let n = reader.read(&mut buf2).unwrap();
            assert_eq!(&buf2[..n], b", World!");
        }

        #[test]
        fn two_byte_char_boundary() {
            // £ is 0xC2 0xA3 in UTF-8
            let data = b"Hi\xC2\xA3";

            let mut reader = Utf8ValidatingReader::new(ChunkedReader::new(data, 1));
            let mut result = Vec::new();

            loop {
                let mut buf = [0u8; 10];
                let n = reader.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                result.extend_from_slice(&buf[..n]);
            }

            assert_eq!(result, data);
        }

        #[test]
        fn three_byte_char_boundary() {
            // 世 is 0xE4 0xB8 0x96 in UTF-8
            let data = "Hi世".as_bytes();

            let mut reader = Utf8ValidatingReader::new(ChunkedReader::new(data, 1));
            let mut result = Vec::new();

            loop {
                let mut buf = [0u8; 10];
                let n = reader.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                result.extend_from_slice(&buf[..n]);
            }

            assert_eq!(result, data);
        }

        #[test]
        fn four_byte_char_boundary() {
            // 😀 is 0xF0 0x9F 0x98 0x80 in UTF-8
            let data = "Hi😀".as_bytes();

            let mut reader = Utf8ValidatingReader::new(ChunkedReader::new(data, 1));
            let mut result = Vec::new();

            loop {
                let mut buf = [0u8; 10];
                let n = reader.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                result.extend_from_slice(&buf[..n]);
            }

            assert_eq!(result, data);
        }

        #[test]
        fn consecutive_valid_multibyte() {
            // Multiple 2-byte chars in a row
            let data = "£€¥".as_bytes();
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 20];
            let n = reader.read(&mut buf).unwrap();
            assert_eq!(&buf[..n], data);
            assert_eq!(std::str::from_utf8(&buf[..n]).unwrap(), "£€¥");
        }

        #[test]
        fn read_exactly_at_char_boundary() {
            let data = "Hi世".as_bytes(); // 2 ASCII + 3-byte char = 5 bytes
            let mut reader = Utf8ValidatingReader::new(&data[..]);

            // Read exactly the size
            let mut buf = [0u8; 5];
            let n = reader.read(&mut buf).unwrap();
            assert_eq!(n, 5);
            assert_eq!(&buf[..n], data);
        }

        #[test]
        fn multiple_multibyte_chars() {
            let data = "世界😀🎉".as_bytes();
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = vec![0u8; 100];
            let n = reader.read(&mut buf).unwrap();
            assert_eq!(&buf[..n], data);
            assert_eq!(std::str::from_utf8(&buf[..n]).unwrap(), "世界😀🎉");
        }

        #[test]
        fn partial_read_with_buffering() {
            // Create data where multibyte char is at boundary
            let data = "ab😀cd".as_bytes(); // a, b, [4-byte emoji], c, d

            // Read 3 bytes at a time - will split the 4-byte emoji
            let mut reader = Utf8ValidatingReader::new(ChunkedReader::new(data, 3));

            let mut result = Vec::new();
            loop {
                let mut buf = [0u8; 20];
                let n = reader.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                result.extend_from_slice(&buf[..n]);
            }

            assert_eq!(result, data);
            assert_eq!(std::str::from_utf8(&result).unwrap(), "ab😀cd");
        }

        #[test]
        fn multiple_reads() {
            let data = "Hello, 世界! 😀 Test".as_bytes();
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut result = Vec::new();

            // Read in small chunks
            loop {
                let mut buf = [0u8; 5];
                let n = reader.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                result.extend_from_slice(&buf[..n]);
            }

            assert_eq!(result, data);
        }

        #[test]
        fn very_small_buffer() {
            let data = "😀".as_bytes(); // 4 bytes
            let mut reader = Utf8ValidatingReader::new(&data[..]);

            // Buffer smaller than character
            let mut buf = [0u8; 2];
            let n1 = reader.read(&mut buf).unwrap();

            // Should buffer the incomplete sequence
            assert_eq!(n1, 0);

            // Larger buffer should get the character
            let mut buf2 = [0u8; 10];
            let n2 = reader.read(&mut buf2).unwrap();
            assert_eq!(&buf2[..n2], data);
        }

        #[test]
        fn split_4byte_char_across_multiple_reads() {
            // 😀 is 0xF0 0x9F 0x98 0x80
            let data = b"\xF0\x9F\x98\x80";

            let mut reader = Utf8ValidatingReader::new(ChunkedReader::new(data, 2));
            let mut result = Vec::new();

            loop {
                let mut buf = [0u8; 10];
                let n = reader.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                result.extend_from_slice(&buf[..n]);
            }

            assert_eq!(result, data);
        }
    }

    mod invalid_utf8 {
        use super::*;

        #[test]
        fn incomplete_sequence_at_eof() {
            // Incomplete 2-byte sequence at end
            let data = b"Hi\xC2"; // Missing second byte of £
            let mut reader = Utf8ValidatingReader::new(&data[..]);

            let mut buf = [0u8; 10];
            let n1 = reader.read(&mut buf).unwrap();
            assert_eq!(&buf[..n1], b"Hi");

            // Second read should fail because incomplete sequence at EOF
            let result = reader.read(&mut buf);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        }

        #[test]
        fn invalid_start_byte() {
            // 0xFF is never valid in UTF-8
            let data = b"\xFF";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];
            let result = reader.read(&mut buf);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        }

        #[test]
        fn invalid_continuation_byte() {
            // 0xC2 should be followed by 0x80-0xBF, not 0x00
            let data = b"\xC2\x00";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];
            let result = reader.read(&mut buf);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        }

        #[test]
        fn invalid_with_valid_prefix() {
            // Valid UTF-8 followed by invalid
            let data = b"OK\xFF";
            let mut reader = Utf8ValidatingReader::new(&data[..]);

            let mut buf = [0u8; 10];
            let n = reader.read(&mut buf).unwrap();
            assert_eq!(&buf[..n], b"OK");

            // Second read should error on invalid byte
            let result = reader.read(&mut buf);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        }

        #[test]
        fn mixed_valid_and_invalid() {
            // Valid, invalid, valid - but we error on invalid so never see "More"
            let data = b"OK\xFFMore";
            let mut reader = Utf8ValidatingReader::new(&data[..]);

            let mut buf = [0u8; 20];

            // First read gets "OK"
            let n1 = reader.read(&mut buf).unwrap();
            assert_eq!(&buf[..n1], b"OK");

            // Second read should error on invalid byte (never reaches "More")
            let result = reader.read(&mut buf);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        }

        #[test]
        fn all_invalid_bytes() {
            let data = b"\xFF\xFE\xFD";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];

            let result = reader.read(&mut buf);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        }

        #[test]
        fn incomplete_3byte_at_eof() {
            // Incomplete 3-byte sequence
            let data = b"Hi\xE4\xB8"; // Missing third byte of 世
            let mut reader = Utf8ValidatingReader::new(&data[..]);

            let mut buf = [0u8; 10];
            let n1 = reader.read(&mut buf).unwrap();
            assert_eq!(&buf[..n1], b"Hi");

            let result = reader.read(&mut buf);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        }

        #[test]
        fn incomplete_4byte_at_eof() {
            // Incomplete 4-byte sequence
            let data = b"Hi\xF0\x9F\x98"; // Missing fourth byte of 😀
            let mut reader = Utf8ValidatingReader::new(&data[..]);

            let mut buf = [0u8; 10];
            let n1 = reader.read(&mut buf).unwrap();
            assert_eq!(&buf[..n1], b"Hi");

            let result = reader.read(&mut buf);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        }

        #[test]
        fn overlong_encoding() {
            // Overlong encoding of '/' (0x2F)
            // Valid: 0x2F
            // Overlong 2-byte: 0xC0 0xAF (invalid)
            let data = b"\xC0\xAF";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];

            let result = reader.read(&mut buf);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        }

        #[test]
        fn surrogate_pairs() {
            // UTF-16 surrogate pairs are invalid in UTF-8
            // 0xED 0xA0 0x80 (U+D800, invalid surrogate)
            let data = b"\xED\xA0\x80";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];

            let result = reader.read(&mut buf);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        }
    }
}

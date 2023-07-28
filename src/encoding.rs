//! A module for wrappers that encode / decode data.

use std::borrow::Cow;
use std::io;

#[cfg(feature = "encoding")]
use encoding_rs::{Encoding, UTF_16BE, UTF_16LE, UTF_8};
#[cfg(feature = "encoding")]
use encoding_rs_io::{DecodeReaderBytes, DecodeReaderBytesBuilder};

#[cfg(feature = "encoding")]
use crate::Error;
use crate::Result;

/// Unicode "byte order mark" (\u{FEFF}) encoded as UTF-8.
/// See <https://unicode.org/faq/utf_bom.html#bom1>
pub(crate) const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
/// Unicode "byte order mark" (\u{FEFF}) encoded as UTF-16 with little-endian byte order.
/// See <https://unicode.org/faq/utf_bom.html#bom1>
#[cfg(feature = "encoding")]
pub(crate) const UTF16_LE_BOM: &[u8] = &[0xFF, 0xFE];
/// Unicode "byte order mark" (\u{FEFF}) encoded as UTF-16 with big-endian byte order.
/// See <https://unicode.org/faq/utf_bom.html#bom1>
#[cfg(feature = "encoding")]
pub(crate) const UTF16_BE_BOM: &[u8] = &[0xFE, 0xFF];

/// A struct for transparently decoding / validating bytes as UTF-8.
#[derive(Debug)]
pub struct Utf8BytesReader<R> {
    #[cfg(feature = "encoding")]
    reader: io::BufReader<DecodeReaderBytes<R, Vec<u8>>>,
    #[cfg(not(feature = "encoding"))]
    reader: io::BufReader<R>,
}

impl<R: io::Read> Utf8BytesReader<R> {
    /// Build a new reader which decodes a stream of bytes in an unknown encoding into UTF-8.
    /// Note: The consumer is responsible for finding the correct character boundaries when
    /// treating a given range of bytes as UTF-8.
    #[cfg(feature = "encoding")]
    pub fn new(reader: R) -> Self {
        let decoder = DecodeReaderBytesBuilder::new()
            .bom_override(true)
            .build(reader);

        Self {
            reader: io::BufReader::new(decoder),
        }
    }

    /// Build a new reader which (will eventually) validate UTF-8.
    /// Note: The consumer is responsible for finding the correct character boundaries when
    /// treating a given range of bytes as UTF-8.
    #[cfg(not(feature = "encoding"))]
    pub fn new(reader: R) -> Self {
        Self {
            reader: io::BufReader::new(reader),
        }
    }
}

impl<R: io::Read> io::Read for Utf8BytesReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }
}

impl<R: io::Read> io::BufRead for Utf8BytesReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.reader.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.reader.consume(amt)
    }
}

///
#[derive(Debug)]
pub struct ValidatingReader<R> {
    reader: R,
    leftover_bytes_buf: [u8; 7],
    leftover_bytes: u8,
}

impl<R: io::Read> ValidatingReader<R> {
    ///
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            leftover_bytes_buf: [0; 7],
            leftover_bytes: 0,
        }
    }
}

impl<R: io::Read> io::Read for ValidatingReader<R> {
    // TODO: bug around the edges of the buffer
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let amt = {
            let leftover_bytes = &self.leftover_bytes_buf[..self.leftover_bytes.into()];
            let (dest_for_leftover_bytes, dest_for_bytes_read) = buf.split_at_mut(leftover_bytes.len());
            dest_for_leftover_bytes.copy_from_slice(&leftover_bytes);
            self.reader.read(dest_for_bytes_read)? + self.leftover_bytes as usize
        };

        let (bytes_in_buffer, _unused_buffer) = buf.split_at(amt);
        match std::str::from_utf8(bytes_in_buffer) {
            Ok(_) => {
                self.leftover_bytes = 0;
                Ok(amt)
            },
            Err(err) => {
                let (valid, leftover) = bytes_in_buffer.split_at(err.valid_up_to());
                self.leftover_bytes_buf[..leftover.len()].copy_from_slice(leftover);
                self.leftover_bytes = leftover.len() as u8;
                Ok(valid.len())
            }
        }
    }
}

// error::const_io_error!(
//     ErrorKind::InvalidData,
//     "stream did not contain valid UTF-8"
// )

/// Decodes the provided bytes using the specified encoding.
///
/// Returns an error in case of malformed or non-representable sequences in the `bytes`.
#[cfg(feature = "encoding")]
pub fn decode<'b>(bytes: &'b [u8], encoding: &'static Encoding) -> Result<Cow<'b, str>> {
    encoding
        .decode_without_bom_handling_and_without_replacement(bytes)
        .ok_or(Error::NonDecodable(None))
}

/// Automatic encoding detection of XML files based using the
/// [recommended algorithm](https://www.w3.org/TR/xml11/#sec-guessing).
///
/// If encoding is detected, `Some` is returned with an encoding and size of BOM
/// in bytes, if detection was performed using BOM, or zero, if detection was
/// performed without BOM.
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
#[cfg(feature = "encoding")]
pub fn detect_encoding(bytes: &[u8]) -> Option<(&'static Encoding, usize)> {
    match bytes {
        // with BOM
        _ if bytes.starts_with(UTF16_BE_BOM) => Some((UTF_16BE, 2)),
        _ if bytes.starts_with(UTF16_LE_BOM) => Some((UTF_16LE, 2)),
        _ if bytes.starts_with(UTF8_BOM) => Some((UTF_8, 3)),

        // without BOM
        _ if bytes.starts_with(&[0x00, b'<', 0x00, b'?']) => Some((UTF_16BE, 0)), // Some BE encoding, for example, UTF-16 or ISO-10646-UCS-2
        _ if bytes.starts_with(&[b'<', 0x00, b'?', 0x00]) => Some((UTF_16LE, 0)), // Some LE encoding, for example, UTF-16 or ISO-10646-UCS-2
        _ if bytes.starts_with(&[b'<', b'?', b'x', b'm']) => Some((UTF_8, 0)), // Some ASCII compatible

        _ => None,
    }
}

#[cfg(test)]
mod test {
    use std::io::Read;

    use super::*;

    #[track_caller]
    fn test_validate_input(input: &[u8]) {
        let mut reader = ValidatingReader::new(input);
        assert_eq!(reader.read_to_end(&mut Vec::new()).unwrap(), input.len());
    }

    mod decoding_reader {

    }

    mod validating_reader {
        use super::*;

        #[test]
        fn utf8_test_file() {
            let test_file = std::fs::read("tests/documents/encoding/utf8.txt").unwrap();

            // test_validate_input(b"asdf");
            // test_validate_input("\u{2014}asdfasdfasdfasdfasdfa\u{2014}asdf".as_bytes());
            test_validate_input(test_file.as_slice());
            // test_validate_input(b"\x82\xA0\x82\xA2\x82\xA4");
            // test_validate_input(b"\xEF\xBB\xBFfoo\xFFbar");
        }
    }
}

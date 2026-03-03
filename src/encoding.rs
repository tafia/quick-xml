//! A module for wrappers that encode / decode data.

use std::borrow::Cow;
use std::io::{self, BufRead, Read};
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

/// An error type representing UTF-8 validation failure.
///
/// Unlike [`std::str::Utf8Error`], instances can be created directly for custom error scenarios.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Utf8ValidationError {
    /// Error from standard library UTF-8 validation
    Utf8(Utf8Error),
    /// Invalid UTF-8 sequence found in the input
    InvalidSequence {
        /// Length of the invalid UTF-8 sequence in bytes
        error_len: usize,
    },
    /// Incomplete UTF-8 sequence at end of stream
    IncompleteSequence,
    /// Non-UTF-8 encoding detected at start of stream
    NonUtf8EncodingDetected(DetectedEncoding),
}

impl From<Utf8Error> for Utf8ValidationError {
    #[inline]
    fn from(e: Utf8Error) -> Self {
        Self::Utf8(e)
    }
}

impl std::fmt::Display for Utf8ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Utf8(e) => write!(f, "{}", e),
            Self::InvalidSequence { error_len } => {
                write!(f, "invalid UTF-8 sequence of {} bytes", error_len)
            }
            Self::IncompleteSequence => {
                write!(f, "incomplete UTF-8 sequence at end of stream")
            }
            Self::NonUtf8EncodingDetected(detected) => {
                write!(
                    f,
                    "non-UTF-8 encoding detected at start of stream: {:?}",
                    detected
                )
            }
        }
    }
}

impl std::error::Error for Utf8ValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Utf8(e) => Some(e),
            _ => None,
        }
    }
}

/// An error when decoding or encoding
///
/// If feature [`encoding`] is disabled, the [`EncodingError`] is always [`EncodingError::Utf8`]
///
/// [`encoding`]: ../index.html#encoding
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EncodingError {
    /// Input was not valid UTF-8
    Utf8(Utf8ValidationError),
    /// Input did not adhere to the given encoding
    #[cfg(feature = "encoding")]
    Other(&'static encoding_rs::Encoding),
}

impl From<Utf8Error> for EncodingError {
    #[inline]
    fn from(e: Utf8Error) -> Self {
        Self::Utf8(e.into())
    }
}

impl From<Utf8ValidationError> for EncodingError {
    #[inline]
    fn from(e: Utf8ValidationError) -> Self {
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
/// A struct for transparently decoding / validating bytes as UTF-8.
#[derive(Debug)]
pub struct Utf8BytesReader<R> {
    #[cfg(feature = "encoding")]
    reader: DecodingReader<io::BufReader<R>>,
    #[cfg(not(feature = "encoding"))]
    reader: io::BufReader<Utf8ValidatingReader<io::BufReader<R>>>,
}

impl<R: io::Read> Utf8BytesReader<R> {
    /// Build a new reader which decodes a stream of bytes in an unknown encoding into UTF-8.
    ///
    /// With the `encoding` feature, the encoding is auto-detected from BOM or XML
    /// declaration patterns, and the stream is decoded into UTF-8 using `encoding_rs`.
    ///
    /// Without the `encoding` feature, the stream is validated as UTF-8 and non-UTF-8
    /// encodings are rejected.
    #[cfg(feature = "encoding")]
    pub fn new(reader: R) -> Self {
        Self {
            reader: DecodingReader::new(io::BufReader::new(reader)),
        }
    }

    /// Build a new reader which validates UTF-8.
    #[cfg(not(feature = "encoding"))]
    pub fn new(reader: R) -> Self {
        Self {
            reader: io::BufReader::new(Utf8ValidatingReader::new(io::BufReader::new(reader))),
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

/// Returns the expected total number of bytes in a UTF-8 character given its first byte
/// (2, 3, or 4). Used to determine how many continuation bytes are needed to complete a
/// pending incomplete sequence.
fn utf8_char_width(first_byte: u8) -> usize {
    if first_byte & 0x80 == 0 {
        1
    } else if first_byte & 0xE0 == 0xC0 {
        2
    } else if first_byte & 0xF0 == 0xE0 {
        3
    } else if first_byte & 0xF8 == 0xF0 {
        4
    } else {
        1 // Invalid start byte; will be caught by from_utf8
    }
}

/// Finds the largest byte index <= `index` that falls on a UTF-8 character boundary.
/// Used when the caller's output buffer is smaller than the available valid data —
/// we must avoid consuming a partial multi-byte character from the BufRead, because
/// orphaned continuation bytes at the start of the next `fill_buf()` would be
/// misreported as invalid UTF-8.
///
/// The caller must ensure that `bytes[..index]` contains valid UTF-8 data.
fn floor_char_boundary(bytes: &[u8], index: usize) -> usize {
    if index >= bytes.len() {
        bytes.len()
    } else {
        let mut i = index;
        while i > 0 && (bytes[i] & 0xC0) == 0x80 {
            i -= 1;
        }
        i
    }
}

/// A reader wrapper that decodes a byte stream from any encoding into UTF-8.
///
/// This reader wraps a [`BufRead`] source and uses [`encoding_rs::Decoder`] to
/// transcode the input into valid UTF-8. On first access, it detects the encoding
/// from BOM or XML declaration byte patterns and configures the appropriate decoder.
///
/// For UTF-8 input, this acts as a validating passthrough. For UTF-16 or other
/// encodings, the bytes are transcoded into UTF-8 in an internal buffer.
///
/// # Examples
///
/// ```
/// use std::io::Read;
/// use quick_xml::encoding::DecodingReader;
///
/// // UTF-8 input passes through:
/// let data = b"Hello, World!";
/// let mut reader = DecodingReader::new(&data[..]);
/// let mut buf = [0u8; 20];
/// let n = reader.read(&mut buf).unwrap();
/// assert_eq!(&buf[..n], data);
/// ```
#[cfg(feature = "encoding")]
pub struct DecodingReader<R> {
    inner: R,
    decoder: encoding_rs::Decoder,
    /// Decoded UTF-8 output buffer
    out_buf: Box<[u8]>,
    /// Start of unconsumed data in out_buf
    out_pos: usize,
    /// End of valid data in out_buf
    out_len: usize,
    /// Whether the inner reader has reached EOF
    inner_eof: bool,
    /// Whether encoding detection has happened
    encoding_detected: bool,
}

#[cfg(feature = "encoding")]
impl<R: std::fmt::Debug> std::fmt::Debug for DecodingReader<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodingReader")
            .field("inner", &self.inner)
            .field("encoding", &self.decoder.encoding())
            .field("out_pos", &self.out_pos)
            .field("out_len", &self.out_len)
            .field("inner_eof", &self.inner_eof)
            .field("encoding_detected", &self.encoding_detected)
            .finish()
    }
}

#[cfg(feature = "encoding")]
impl<R> DecodingReader<R> {
    /// Creates a new decoding reader.
    ///
    /// The encoding is auto-detected from BOM or XML declaration patterns on
    /// first access. Defaults to UTF-8 if no pattern is recognized.
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            decoder: encoding_rs::UTF_8.new_decoder_without_bom_handling(),
            out_buf: vec![0u8; 8192].into_boxed_slice(),
            out_pos: 0,
            out_len: 0,
            inner_eof: false,
            encoding_detected: false,
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

#[cfg(feature = "encoding")]
impl<R: BufRead> BufRead for DecodingReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        // Fast path: serve already-decoded data
        if self.out_pos < self.out_len {
            return Ok(&self.out_buf[self.out_pos..self.out_len]);
        }

        // Reset output buffer
        self.out_pos = 0;
        self.out_len = 0;

        // Encoding detection on first access
        if !self.encoding_detected {
            self.encoding_detected = true;

            let available = self.inner.fill_buf()?;
            if let Some(detected) = detect_encoding(available) {
                let bom_len = detected.bom_len();
                if bom_len > 0 {
                    self.inner.consume(bom_len);
                }
                let encoding = detected.encoding();
                if encoding != encoding_rs::UTF_8 {
                    self.decoder = encoding.new_decoder_without_bom_handling();
                }
            }
        }

        // Loop until we produce output, hit EOF, or get an error.
        // The decoder may consume input into internal state (e.g., partial
        // UTF-16 code unit) without producing output — we must keep feeding
        // it more input rather than returning an empty slice (which signals EOF).
        loop {
            // EOF flush path: tell decoder this is the last chunk
            if self.inner_eof {
                let (result, _, written) = self.decoder.decode_to_utf8_without_replacement(
                    b"",
                    &mut self.out_buf[..],
                    true,
                );
                self.out_len = written;
                if let encoding_rs::DecoderResult::Malformed(_, _) = result {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        EncodingError::Other(self.decoder.encoding()),
                    ));
                }
                return Ok(&self.out_buf[..self.out_len]);
            }

            // Main decode path: read from inner, decode into out_buf
            let (result, read, written) = {
                let src = self.inner.fill_buf()?;
                if src.is_empty() {
                    self.inner_eof = true;
                    continue; // will hit EOF flush path on next iteration
                }
                self.decoder
                    .decode_to_utf8_without_replacement(src, &mut self.out_buf[..], false)
            };
            self.inner.consume(read);
            self.out_len = written;

            if let encoding_rs::DecoderResult::Malformed(_, _) = result {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    EncodingError::Other(self.decoder.encoding()),
                ));
            }

            if written > 0 {
                return Ok(&self.out_buf[..self.out_len]);
            }
            // written == 0: decoder consumed input into internal state but produced
            // no output yet (e.g., 1 byte of a 2-byte UTF-16 code unit). Loop to
            // get more input.
        }
    }

    fn consume(&mut self, amt: usize) {
        self.out_pos += amt;
    }
}

#[cfg(feature = "encoding")]
impl<R: BufRead> Read for DecodingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let available = self.fill_buf()?;
        if available.is_empty() {
            return Ok(0);
        }
        // No need for floor_char_boundary here — the decoder always produces
        // complete UTF-8 characters in out_buf, and Read::read() operates on
        // raw bytes, so splitting a multi-byte character across reads is fine.
        let len = available.len().min(buf.len());
        buf[..len].copy_from_slice(&available[..len]);
        self.consume(len);
        Ok(len)
    }
}

/// A reader wrapper that ensures only valid UTF-8 bytes are read.
///
/// This reader wraps a [`BufRead`] source and uses [`str::from_utf8()`] and
/// [`Utf8Error::valid_up_to()`] to validate that only valid UTF-8 bytes are
/// written to the output buffer. Incomplete UTF-8 sequences at buffer boundaries
/// are handled transparently.
///
/// Additionally, this reader checks the very beginning of the stream for encoding
/// signatures (BOMs or XML declaration patterns) and rejects streams that appear to
/// be encoded in UTF-16 or other non-UTF-8 encodings.
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
    /// Small buffer for incomplete UTF-8 sequences at BufRead boundaries.
    /// At most 3 bytes (the start of a 2, 3, or 4-byte sequence).
    pending: [u8; 3],
    pending_len: u8,
    /// Whether we've checked for encoding at the start of the stream
    encoding_checked: bool,
}

impl<R> Utf8ValidatingReader<R> {
    /// Creates a new UTF-8 validating reader
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            pending: [0; 3],
            pending_len: 0,
            encoding_checked: false,
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

impl<R: BufRead> Read for Utf8ValidatingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        // On the very first read, peek at the stream to detect encoding via BOM
        // or XML declaration patterns. UTF-16 is rejected; UTF-8 BOM is stripped.
        if !self.encoding_checked {
            self.encoding_checked = true;

            let available = self.inner.fill_buf()?;
            // detect_encoding uses starts_with, so patterns longer than the
            // available data simply won't match — no length guard needed.
            if let Some(detected) = detect_encoding(available) {
                match detected {
                    DetectedEncoding::Utf8Bom | DetectedEncoding::AsciiCompatible => {
                        let bom_len = detected.bom_len();
                        if bom_len > 0 {
                            self.inner.consume(bom_len);
                        }
                    }
                    DetectedEncoding::Utf16LeLike
                    | DetectedEncoding::Utf16LeBom
                    | DetectedEncoding::Utf16BeLike
                    | DetectedEncoding::Utf16BeBom => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            EncodingError::Utf8(Utf8ValidationError::NonUtf8EncodingDetected(
                                detected,
                            )),
                        ));
                    }
                }
            }
        }

        loop {
            // ------ Pending path ------------
            // On a previous iteration, the BufRead's buffer ended with the first 1-3 bytes of a
            // multi-byte character. We consumed those bytes into `self.pending` so the BufRead
            // could refill. Now we combine them with fresh data to complete the character.
            if self.pending_len > 0 {
                let available = self.inner.fill_buf()?;
                if available.is_empty() {
                    self.pending_len = 0;
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        EncodingError::Utf8(Utf8ValidationError::IncompleteSequence),
                    ));
                }

                // The first byte of a UTF-8 sequence encodes the total length (2, 3, or 4).
                // The pending buffer holds at most 3 bytes (seq_len - 1); the final byte always
                // comes from the inner reader. Use that total to determine how many more bytes
                // we need.
                let plen = self.pending_len as usize;
                let seq_len = utf8_char_width(self.pending[0]);
                let needed = seq_len - plen;

                if available.len() < needed {
                    // Inner reader still doesn't have enough bytes (e.g. a network reader
                    // returning one byte at a time). Accumulate what we can and loop to try again.
                    let take = available.len().min(3 - plen);
                    self.pending[plen..plen + take].copy_from_slice(&available[..take]);
                    self.pending_len += take as u8;
                    self.inner.consume(take);
                    continue;
                }

                // Reconstruct the full character from pending + fresh bytes.
                let mut seq = [0u8; 4];
                seq[..plen].copy_from_slice(&self.pending[..plen]);
                seq[plen..seq_len].copy_from_slice(&available[..needed]);

                match std::str::from_utf8(&seq[..seq_len]) {
                    Ok(_) => {
                        if buf.len() < seq_len {
                            return Ok(0);
                        }
                        buf[..seq_len].copy_from_slice(&seq[..seq_len]);
                        self.inner.consume(needed);
                        self.pending_len = 0;
                        return Ok(seq_len);
                    }
                    Err(e) => {
                        self.pending_len = 0;
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            EncodingError::Utf8(if let Some(error_len) = e.error_len() {
                                Utf8ValidationError::InvalidSequence { error_len }
                            } else {
                                Utf8ValidationError::IncompleteSequence
                            }),
                        ));
                    }
                }
            }

            // ------- Main path---------
            // Peek at the inner BufRead's buffer and validate its contents. We only consume bytes
            // we actually copy to the caller's buf, so bytes after an error remain available for
            // the next call.
            let available = self.inner.fill_buf()?;
            if available.is_empty() {
                return Ok(0);
            }

            match std::str::from_utf8(available) {
                Ok(_) => {
                    // All available bytes are valid UTF-8. Copy as many complete characters as
                    // fit in buf. We must land on a character boundary to avoid consuming a
                    // partial character from the BufRead — otherwise the next fill_buf() would
                    // start with orphaned continuation bytes, causing a false validation error.
                    let len = floor_char_boundary(available, buf.len());
                    if len == 0 {
                        return Ok(0);
                    }
                    buf[..len].copy_from_slice(&available[..len]);
                    self.inner.consume(len);
                    return Ok(len);
                }
                Err(e) => {
                    let valid_up_to = e.valid_up_to();

                    if let Some(error_len) = e.error_len() {
                        // Definite invalid UTF-8 sequence.
                        if valid_up_to == 0 {
                            // Starts with invalid bytes — error immediately.
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                EncodingError::Utf8(Utf8ValidationError::InvalidSequence {
                                    error_len,
                                }),
                            ));
                        }
                        // There is a valid prefix before the error. Return as much of it as
                        // fits in buf (on a char boundary); the invalid bytes stay unconsumed
                        // in the BufRead and will trigger an error on the next read() call.
                        let len = floor_char_boundary(available, valid_up_to.min(buf.len()));
                        if len == 0 {
                            return Ok(0);
                        }
                        buf[..len].copy_from_slice(&available[..len]);
                        self.inner.consume(len);
                        return Ok(len);
                    } else {
                        // Incomplete multi-byte sequence at the end of the BufRead's buffer — we
                        // need more data to decide whether it's valid. Return any valid prefix first.
                        if valid_up_to > 0 {
                            let len = floor_char_boundary(available, valid_up_to.min(buf.len()));
                            if len > 0 {
                                buf[..len].copy_from_slice(&available[..len]);
                                self.inner.consume(len);
                                return Ok(len);
                            }
                            // buf too small for even the first character.
                            return Ok(0);
                        }
                        // The BufRead's buffer contains ONLY incomplete leading bytes (1-3 bytes,
                        // e.g. [0xF0, 0x9F] for a 4-byte char). The BufRead won't refill until
                        // these are consumed, so we move them to our small pending buffer, consume
                        // them, and loop — the next fill_buf() will fetch fresh data that we can
                        // combine with pending.
                        let incomplete_len = available.len();
                        debug_assert!(
                            incomplete_len <= 3,
                            "incomplete UTF-8 prefix should be at most 3 bytes, got {}",
                            incomplete_len,
                        );
                        self.pending[..incomplete_len]
                            .copy_from_slice(&available[..incomplete_len]);
                        self.pending_len = incomplete_len as u8;
                        self.inner.consume(incomplete_len);
                        continue;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod utf8_bytes_reader_tests {
    use super::*;
    use std::io::{BufRead, Read};

    #[test]
    fn basic_read() {
        let data = b"Hello, World!";
        let mut reader = Utf8BytesReader::new(&data[..]);
        let mut buf = [0u8; 20];
        let n = reader.read(&mut buf).unwrap();
        assert!(n > 0);
        assert_eq!(&buf[..n], &data[..n]);
    }

    #[test]
    fn read_with_multibyte_chars() {
        let data = "Hello, 世界! 😀".as_bytes();
        let mut reader = Utf8BytesReader::new(&data[..]);
        let mut result = Vec::new();
        reader.read_to_end(&mut result).unwrap();
        assert_eq!(result, data);
        assert_eq!(std::str::from_utf8(&result).unwrap(), "Hello, 世界! 😀");
    }

    #[test]
    fn bufread_interface() {
        let data = b"Line1\nLine2\nLine3";
        let mut reader = Utf8BytesReader::new(&data[..]);

        // Test fill_buf
        let buf = reader.fill_buf().unwrap();
        assert!(!buf.is_empty());

        // Test consume
        let consumed = buf.len().min(5);
        reader.consume(consumed);

        // Read remaining
        let mut result = Vec::new();
        reader.read_to_end(&mut result).unwrap();
        assert_eq!(result, &data[consumed..]);
    }

    #[test]
    fn empty_input() {
        let data = b"";
        let mut reader = Utf8BytesReader::new(&data[..]);
        let mut buf = [0u8; 10];
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(n, 0);
    }
}

#[cfg(test)]
mod utf8_validating_reader_tests {
    use super::*;
    use std::io::{BufReader, Cursor, Read};

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

    /// Assert that a read result is an error wrapping the expected Utf8ValidationError.
    fn assert_utf8_error(result: io::Result<usize>, expected: Utf8ValidationError) {
        let err = result.expect_err("expected an error");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        let encoding_err = err
            .get_ref()
            .unwrap()
            .downcast_ref::<EncodingError>()
            .expect("error should downcast to EncodingError");
        assert_eq!(
            *encoding_err,
            EncodingError::Utf8(expected),
            "unexpected error variant"
        );
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

            let mut reader = Utf8ValidatingReader::new(BufReader::new(ChunkedReader::new(data, 1)));
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

            let mut reader = Utf8ValidatingReader::new(BufReader::new(ChunkedReader::new(data, 1)));
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

            let mut reader = Utf8ValidatingReader::new(BufReader::new(ChunkedReader::new(data, 1)));
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
            let mut reader = Utf8ValidatingReader::new(BufReader::new(ChunkedReader::new(data, 3)));

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

            // Buffer smaller than character — returns 0 (can't fit)
            let mut buf = [0u8; 2];
            let n1 = reader.read(&mut buf).unwrap();
            assert_eq!(n1, 0);

            // Larger buffer should get the character
            let mut buf2 = [0u8; 10];
            let n2 = reader.read(&mut buf2).unwrap();
            assert_eq!(&buf2[..n2], data);
        }

        #[test]
        fn split_4byte_char_across_bufread_boundary() {
            // 😀 is 0xF0 0x9F 0x98 0x80 — split across two BufRead fills
            let data = b"\xF0\x9F\x98\x80";

            let mut reader = Utf8ValidatingReader::new(BufReader::new(ChunkedReader::new(data, 2)));
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
        fn buf_truncates_at_char_boundary() {
            // "a世b" = [0x61, 0xE4, 0xB8, 0x96, 0x62] = 5 bytes
            // With buf.len() = 2, we can fit "a" (1 byte) but not "a" + 世
            // (4 bytes). floor_char_boundary must truncate to 1.
            let data = "a世b".as_bytes();
            let mut reader = Utf8ValidatingReader::new(&data[..]);

            let mut buf = [0u8; 2];
            let n = reader.read(&mut buf).unwrap();
            assert_eq!(n, 1);
            assert_eq!(&buf[..n], b"a");

            // Next read with larger buffer gets the rest
            let mut buf2 = [0u8; 10];
            let n = reader.read(&mut buf2).unwrap();
            assert_eq!(&buf2[..n], "世b".as_bytes());
        }

        #[test]
        fn buf_truncates_between_multibyte_chars() {
            // "世界" = [0xE4, 0xB8, 0x96, 0xE7, 0x95, 0x8C] = 6 bytes
            // With buf.len() = 4, we can fit "世" (3 bytes) but not
            // "世" + first byte of "界". Must return exactly 3.
            let data = "世界".as_bytes();
            let mut reader = Utf8ValidatingReader::new(&data[..]);

            let mut buf = [0u8; 4];
            let n = reader.read(&mut buf).unwrap();
            assert_eq!(n, 3);
            assert_eq!(&buf[..n], "世".as_bytes());

            let n = reader.read(&mut buf).unwrap();
            assert_eq!(n, 3);
            assert_eq!(&buf[..n], "界".as_bytes());
        }

        #[test]
        fn read_to_end() {
            let data = "Hello, 世界! 😀".as_bytes();
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut result = Vec::new();
            reader.read_to_end(&mut result).unwrap();
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
            assert_utf8_error(
                reader.read(&mut buf),
                Utf8ValidationError::IncompleteSequence,
            );
        }

        #[test]
        fn invalid_start_byte() {
            // 0xFF is never valid in UTF-8
            let data = b"\xFF";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];
            assert_utf8_error(
                reader.read(&mut buf),
                Utf8ValidationError::InvalidSequence { error_len: 1 },
            );
        }

        #[test]
        fn invalid_continuation_byte() {
            // 0xC2 should be followed by 0x80-0xBF, not 0x00
            let data = b"\xC2\x00";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];
            assert_utf8_error(
                reader.read(&mut buf),
                Utf8ValidationError::InvalidSequence { error_len: 1 },
            );
        }

        #[test]
        fn valid_prefix_then_invalid() {
            // First read returns the valid prefix, second read errors
            let data = b"OK\xFF";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];

            let n = reader.read(&mut buf).unwrap();
            assert_eq!(&buf[..n], b"OK");

            assert_utf8_error(
                reader.read(&mut buf),
                Utf8ValidationError::InvalidSequence { error_len: 1 },
            );
        }

        #[test]
        fn valid_then_invalid_then_valid() {
            // Valid prefix returned first, then error; trailing data unreachable
            let data = b"OK\xFFMore";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 20];

            let n = reader.read(&mut buf).unwrap();
            assert_eq!(&buf[..n], b"OK");

            assert_utf8_error(
                reader.read(&mut buf),
                Utf8ValidationError::InvalidSequence { error_len: 1 },
            );
        }

        #[test]
        fn all_invalid_bytes() {
            // All continuation bytes — invalid UTF-8 but no BOM match
            let data = b"\x80\x81\x82";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];
            assert_utf8_error(
                reader.read(&mut buf),
                Utf8ValidationError::InvalidSequence { error_len: 1 },
            );
        }

        #[test]
        fn incomplete_3byte_at_eof() {
            let data = b"Hi\xE4\xB8"; // Missing third byte of 世
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];

            let n = reader.read(&mut buf).unwrap();
            assert_eq!(&buf[..n], b"Hi");

            assert_utf8_error(
                reader.read(&mut buf),
                Utf8ValidationError::IncompleteSequence,
            );
        }

        #[test]
        fn incomplete_4byte_at_eof() {
            let data = b"Hi\xF0\x9F\x98"; // Missing fourth byte of 😀
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];

            let n = reader.read(&mut buf).unwrap();
            assert_eq!(&buf[..n], b"Hi");

            assert_utf8_error(
                reader.read(&mut buf),
                Utf8ValidationError::IncompleteSequence,
            );
        }

        #[test]
        fn overlong_encoding() {
            // Overlong encoding of '/' (0x2F): 0xC0 0xAF
            let data = b"\xC0\xAF";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];
            assert_utf8_error(
                reader.read(&mut buf),
                Utf8ValidationError::InvalidSequence { error_len: 1 },
            );
        }

        #[test]
        fn surrogate_half() {
            // U+D800 encoded in UTF-8: 0xED 0xA0 0x80 (invalid)
            let data = b"\xED\xA0\x80";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];
            assert_utf8_error(
                reader.read(&mut buf),
                Utf8ValidationError::InvalidSequence { error_len: 1 },
            );
        }

        #[test]
        fn incomplete_2byte_at_eof() {
            let data = b"Hi\xC2"; // Missing second byte of £
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];

            let n = reader.read(&mut buf).unwrap();
            assert_eq!(&buf[..n], b"Hi");

            assert_utf8_error(
                reader.read(&mut buf),
                Utf8ValidationError::IncompleteSequence,
            );
        }

        #[test]
        fn invalid_continuation_in_pending_path() {
            // \xC2 expects a continuation byte (0x80-0xBF). Deliver \xC2
            // and \xFF in separate 1-byte BufRead fills so that \xC2 goes
            // to the pending buffer, then \xFF triggers an error in the
            // pending completion handler.
            let data = b"\xC2\xFF";
            let mut reader =
                Utf8ValidatingReader::new(BufReader::with_capacity(1, ChunkedReader::new(data, 1)));
            let mut buf = [0u8; 10];
            assert_utf8_error(
                reader.read(&mut buf),
                Utf8ValidationError::InvalidSequence { error_len: 1 },
            );
        }
    }

    mod encoding_detection {
        use super::*;

        #[test]
        fn utf8_bom_stripped() {
            // UTF-8 BOM (0xEF 0xBB 0xBF) followed by "Hello"
            let data = b"\xEF\xBB\xBFHello";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 20];
            let n = reader.read(&mut buf).unwrap();

            // BOM should be stripped, only "Hello" should be returned
            assert_eq!(&buf[..n], b"Hello");
            assert_eq!(std::str::from_utf8(&buf[..n]).unwrap(), "Hello");
        }

        #[test]
        fn utf16le_bom_rejected() {
            // UTF-16 LE BOM (0xFF 0xFE)
            let data = b"\xFF\xFE<?xml";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 20];

            assert_utf8_error(
                reader.read(&mut buf),
                Utf8ValidationError::NonUtf8EncodingDetected(DetectedEncoding::Utf16LeBom),
            );
        }

        #[test]
        fn utf16be_bom_rejected() {
            // UTF-16 BE BOM (0xFE 0xFF)
            let data = b"\xFE\xFF\x00<\x00?";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 20];

            assert_utf8_error(
                reader.read(&mut buf),
                Utf8ValidationError::NonUtf8EncodingDetected(DetectedEncoding::Utf16BeBom),
            );
        }

        #[test]
        fn utf16le_without_bom_rejected() {
            // UTF-16 LE detected by XML declaration pattern (no BOM)
            let data = b"<\x00?\x00";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];
            assert_utf8_error(
                reader.read(&mut buf),
                Utf8ValidationError::NonUtf8EncodingDetected(DetectedEncoding::Utf16LeLike),
            );
        }

        #[test]
        fn utf16be_without_bom_rejected() {
            // UTF-16 BE detected by XML declaration pattern (no BOM)
            let data = b"\x00<\x00?";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];
            assert_utf8_error(
                reader.read(&mut buf),
                Utf8ValidationError::NonUtf8EncodingDetected(DetectedEncoding::Utf16BeLike),
            );
        }

        #[test]
        fn ascii_compatible_xml_declaration() {
            let data = b"<?xml version=\"1.0\"?><root/>";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 50];
            let n = reader.read(&mut buf).unwrap();
            assert_eq!(&buf[..n], data);
        }

        #[test]
        fn utf8_bom_only() {
            // BOM with no content after it — should return 0 after stripping
            let data = b"\xEF\xBB\xBF";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];
            let n = reader.read(&mut buf).unwrap();
            assert_eq!(n, 0);
        }

        #[test]
        fn short_data_no_pattern_match() {
            // Short data that doesn't match any detection pattern — treated as UTF-8
            let data = b"Hi";
            let mut reader = Utf8ValidatingReader::new(&data[..]);
            let mut buf = [0u8; 10];
            let n = reader.read(&mut buf).unwrap();
            assert_eq!(n, 2);
            assert_eq!(&buf[..n], b"Hi");
        }
    }
}

#[cfg(all(test, feature = "encoding"))]
mod decoding_reader_tests {
    use super::*;
    use std::io::{BufReader, Read};

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

    /// Encode a string as UTF-16 LE bytes with BOM
    fn utf16le_with_bom(s: &str) -> Vec<u8> {
        let mut out = vec![0xFF, 0xFE]; // UTF-16 LE BOM
        for code_unit in s.encode_utf16() {
            out.extend_from_slice(&code_unit.to_le_bytes());
        }
        out
    }

    /// Encode a string as UTF-16 BE bytes with BOM
    fn utf16be_with_bom(s: &str) -> Vec<u8> {
        let mut out = vec![0xFE, 0xFF]; // UTF-16 BE BOM
        for code_unit in s.encode_utf16() {
            out.extend_from_slice(&code_unit.to_be_bytes());
        }
        out
    }

    /// Encode a string as UTF-16 LE bytes without BOM
    fn utf16le_no_bom(s: &str) -> Vec<u8> {
        let mut out = Vec::new();
        for code_unit in s.encode_utf16() {
            out.extend_from_slice(&code_unit.to_le_bytes());
        }
        out
    }

    /// Encode a string as UTF-16 BE bytes without BOM
    fn utf16be_no_bom(s: &str) -> Vec<u8> {
        let mut out = Vec::new();
        for code_unit in s.encode_utf16() {
            out.extend_from_slice(&code_unit.to_be_bytes());
        }
        out
    }

    /// Read all bytes from a reader into a String
    fn read_all(reader: &mut DecodingReader<impl BufRead>) -> io::Result<String> {
        let mut result = Vec::new();
        reader.read_to_end(&mut result)?;
        Ok(String::from_utf8(result).expect("DecodingReader should produce valid UTF-8"))
    }

    mod utf8_passthrough {
        use super::*;

        #[test]
        fn ascii() {
            let data = b"Hello, World!";
            let mut reader = DecodingReader::new(&data[..]);
            let mut buf = [0u8; 20];
            let n = reader.read(&mut buf).unwrap();
            assert_eq!(&buf[..n], data);
        }

        #[test]
        fn multibyte_characters() {
            let data = "Hello, 世界! 😀".as_bytes();
            let mut reader = DecodingReader::new(&data[..]);
            assert_eq!(read_all(&mut reader).unwrap(), "Hello, 世界! 😀");
        }

        #[test]
        fn empty_input() {
            let data = b"";
            let mut reader = DecodingReader::new(&data[..]);
            let mut buf = [0u8; 10];
            let n = reader.read(&mut buf).unwrap();
            assert_eq!(n, 0);
        }

        #[test]
        fn read_to_end() {
            let data = "Hello, 世界! 😀".as_bytes();
            let mut reader = DecodingReader::new(&data[..]);
            let mut result = Vec::new();
            reader.read_to_end(&mut result).unwrap();
            assert_eq!(result, data);
        }
    }

    mod utf8_bom {
        use super::*;

        #[test]
        fn bom_stripped() {
            let data = b"\xEF\xBB\xBFHello";
            let mut reader = DecodingReader::new(&data[..]);
            assert_eq!(read_all(&mut reader).unwrap(), "Hello");
        }

        #[test]
        fn bom_only() {
            let data = b"\xEF\xBB\xBF";
            let mut reader = DecodingReader::new(&data[..]);
            assert_eq!(read_all(&mut reader).unwrap(), "");
        }
    }

    mod utf16le_decoding {
        use super::*;

        #[test]
        fn with_bom() {
            let data = utf16le_with_bom("Hello");
            let mut reader = DecodingReader::new(&data[..]);
            assert_eq!(read_all(&mut reader).unwrap(), "Hello");
        }

        #[test]
        fn with_bom_multibyte() {
            let data = utf16le_with_bom("Hello, 世界! 😀");
            let mut reader = DecodingReader::new(&data[..]);
            assert_eq!(read_all(&mut reader).unwrap(), "Hello, 世界! 😀");
        }

        #[test]
        fn without_bom_xml_declaration() {
            // UTF-16 LE without BOM is detected by the <?xml pattern
            let data = utf16le_no_bom("<?xml version=\"1.0\"?><root/>");
            let mut reader = DecodingReader::new(&data[..]);
            assert_eq!(
                read_all(&mut reader).unwrap(),
                "<?xml version=\"1.0\"?><root/>"
            );
        }
    }

    mod utf16be_decoding {
        use super::*;

        #[test]
        fn with_bom() {
            let data = utf16be_with_bom("Hello");
            let mut reader = DecodingReader::new(&data[..]);
            assert_eq!(read_all(&mut reader).unwrap(), "Hello");
        }

        #[test]
        fn with_bom_multibyte() {
            let data = utf16be_with_bom("Hello, 世界! 😀");
            let mut reader = DecodingReader::new(&data[..]);
            assert_eq!(read_all(&mut reader).unwrap(), "Hello, 世界! 😀");
        }

        #[test]
        fn without_bom_xml_declaration() {
            let data = utf16be_no_bom("<?xml version=\"1.0\"?><root/>");
            let mut reader = DecodingReader::new(&data[..]);
            assert_eq!(
                read_all(&mut reader).unwrap(),
                "<?xml version=\"1.0\"?><root/>"
            );
        }
    }

    mod chunked_input {
        use super::*;

        #[test]
        fn utf8_one_byte_at_a_time() {
            let data = "Hello, 世界!".as_bytes();
            let mut reader = DecodingReader::new(BufReader::new(ChunkedReader::new(data, 1)));
            assert_eq!(read_all(&mut reader).unwrap(), "Hello, 世界!");
        }

        #[test]
        fn utf16le_small_chunks() {
            let data = utf16le_with_bom("Hello, 世界! 😀");
            let mut reader = DecodingReader::new(BufReader::new(ChunkedReader::new(&data, 3)));
            assert_eq!(read_all(&mut reader).unwrap(), "Hello, 世界! 😀");
        }

        #[test]
        fn utf16be_small_chunks() {
            let data = utf16be_with_bom("Hello, 世界! 😀");
            let mut reader = DecodingReader::new(BufReader::new(ChunkedReader::new(&data, 3)));
            assert_eq!(read_all(&mut reader).unwrap(), "Hello, 世界! 😀");
        }
    }

    mod bufread_interface {
        use super::*;
        use std::io::BufRead;

        #[test]
        fn fill_buf_and_consume() {
            let data = b"Hello, World!";
            let mut reader = DecodingReader::new(&data[..]);

            let buf = reader.fill_buf().unwrap();
            assert!(!buf.is_empty());
            let first = buf[0];
            assert_eq!(first, b'H');

            reader.consume(5);

            let buf = reader.fill_buf().unwrap();
            assert_eq!(buf[0], b',');
        }

        #[test]
        fn utf16_fill_buf() {
            let data = utf16le_with_bom("Hello");
            let mut reader = DecodingReader::new(&data[..]);

            let buf = reader.fill_buf().unwrap();
            // Should be decoded UTF-8
            assert_eq!(std::str::from_utf8(buf).unwrap(), "Hello");
        }

        #[test]
        fn partial_consume_then_read_more() {
            let data = b"Hello, World!";
            let mut reader = DecodingReader::new(&data[..]);

            // Read all into buffer
            let buf = reader.fill_buf().unwrap();
            assert_eq!(buf.len(), 13);

            // Consume only 5 bytes ("Hello")
            reader.consume(5);

            // Next fill_buf should return the remainder
            let buf = reader.fill_buf().unwrap();
            assert_eq!(std::str::from_utf8(buf).unwrap(), ", World!");

            // Consume the rest
            reader.consume(8);

            // Should be EOF
            let buf = reader.fill_buf().unwrap();
            assert!(buf.is_empty());
        }
    }

    mod accessors {
        use super::*;
        use std::io::Cursor;

        #[test]
        fn get_ref() {
            let data = b"Hello";
            let cursor = Cursor::new(data.to_vec());
            let reader = DecodingReader::new(cursor);
            assert_eq!(reader.get_ref().get_ref(), data);
        }

        #[test]
        fn get_mut() {
            let data = b"Hello";
            let cursor = Cursor::new(data.to_vec());
            let mut reader = DecodingReader::new(cursor);
            reader.get_mut().set_position(2);
            assert_eq!(reader.get_ref().position(), 2);
        }

        #[test]
        fn into_inner() {
            let data = b"Hello";
            let cursor = Cursor::new(data.to_vec());
            let reader = DecodingReader::new(cursor);
            let inner = reader.into_inner();
            assert_eq!(inner.get_ref(), data);
        }
    }

    mod large_input {
        use super::*;

        #[test]
        fn utf8_larger_than_internal_buffer() {
            // Create input larger than the 8192-byte internal buffer
            let content: String = "abcdefghij".repeat(1000); // 10,000 bytes
            let data = content.as_bytes();
            let mut reader = DecodingReader::new(&data[..]);
            assert_eq!(read_all(&mut reader).unwrap(), content);
        }

        #[test]
        fn utf16le_larger_than_internal_buffer() {
            let content: String = "abcdefghij".repeat(1000);
            let data = utf16le_with_bom(&content);
            let mut reader = DecodingReader::new(&data[..]);
            assert_eq!(read_all(&mut reader).unwrap(), content);
        }

        #[test]
        fn utf8_multibyte_larger_than_internal_buffer() {
            // Mix of ASCII and multibyte characters exceeding internal buffer
            let content: String = "Hello, 世界! 😀 ".repeat(500); // ~9500 bytes UTF-8
            let data = content.as_bytes();
            let mut reader = DecodingReader::new(&data[..]);
            assert_eq!(read_all(&mut reader).unwrap(), content);
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn no_detection_pattern() {
            // Data that doesn't match any BOM or XML declaration pattern
            let data = b"just plain text";
            let mut reader = DecodingReader::new(&data[..]);
            assert_eq!(read_all(&mut reader).unwrap(), "just plain text");
        }

        #[test]
        fn utf16le_bom_only() {
            let data = &[0xFF, 0xFE]; // UTF-16 LE BOM, no content
            let mut reader = DecodingReader::new(&data[..]);
            assert_eq!(read_all(&mut reader).unwrap(), "");
        }

        #[test]
        fn utf16be_bom_only() {
            let data = &[0xFE, 0xFF]; // UTF-16 BE BOM, no content
            let mut reader = DecodingReader::new(&data[..]);
            assert_eq!(read_all(&mut reader).unwrap(), "");
        }

        #[test]
        fn read_with_one_byte_buffer() {
            // Exercises the partial-copy path in read()
            let data = "Hello, 世界!".as_bytes();
            let mut reader = DecodingReader::new(&data[..]);
            let mut result = Vec::new();
            let mut buf = [0u8; 1];
            loop {
                let n = reader.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                result.extend_from_slice(&buf[..n]);
            }
            assert_eq!(String::from_utf8(result).unwrap(), "Hello, 世界!");
        }

        #[test]
        fn utf16_surrogate_pairs() {
            // Characters above U+FFFF require surrogate pairs in UTF-16
            // 𝄞 (U+1D11E, MUSICAL SYMBOL G CLEF) and 🎵 (U+1F3B5)
            let content = "Music: 𝄞🎵";
            let data = utf16le_with_bom(content);
            let mut reader = DecodingReader::new(&data[..]);
            assert_eq!(read_all(&mut reader).unwrap(), content);
        }

        #[test]
        fn chunked_utf16_at_code_unit_boundary() {
            // chunk_size=2 aligns exactly with UTF-16 code units
            let data = utf16le_with_bom("Hello, 世界!");
            let mut reader = DecodingReader::new(BufReader::new(ChunkedReader::new(&data, 2)));
            assert_eq!(read_all(&mut reader).unwrap(), "Hello, 世界!");
        }

        #[test]
        fn chunked_utf16_misaligned_chunks() {
            // chunk_size=5 misaligns with UTF-16's 2-byte code units,
            // forcing splits within code units after the BOM
            let data = utf16le_with_bom("Hello, 世界!");
            let mut reader = DecodingReader::new(BufReader::new(ChunkedReader::new(&data, 5)));
            assert_eq!(read_all(&mut reader).unwrap(), "Hello, 世界!");
        }
    }
}

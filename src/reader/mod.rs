//! A module to handle `Reader`

#[cfg(not(feature = "encoding"))]
use crate::errors::Error;
#[cfg(not(feature = "encoding"))]
use crate::errors::Result;
use crate::events::{attributes::Attribute, BytesStart};
#[cfg(not(feature = "encoding"))]
use std::str::from_utf8;

#[cfg(feature = "encoding")]
use encoding_rs::{Encoding, UTF_16BE, UTF_16LE};
#[cfg(feature = "encoding")]
use std::borrow::Cow;

#[cfg(feature = "asynchronous")]
pub mod asynchronous;
pub mod sync;

/// Trait for decoding, which is shared by the sync and async `Reader`
pub trait Decode {
    /// Decodes a slice using the encoding specified in the XML declaration.
    ///
    /// Decode `bytes` with BOM sniffing and with malformed sequences replaced with the
    /// `U+FFFD REPLACEMENT CHARACTER`.
    ///
    /// If no encoding is specified, defaults to UTF-8.
    #[inline]
    #[cfg(feature = "encoding")]
    fn decode<'b, 'c>(&'b self, bytes: &'c [u8]) -> Cow<'c, str> {
        self.read_encoding().decode(bytes).0
    }

    /// Decodes a UTF8 slice regardless of XML declaration.
    ///
    /// Decode `bytes` with BOM sniffing and with malformed sequences replaced with the
    /// `U+FFFD REPLACEMENT CHARACTER`.
    ///
    /// # Note
    ///
    /// If you instead want to use XML declared encoding, use the `encoding` feature
    #[inline]
    #[cfg(not(feature = "encoding"))]
    fn decode<'c>(&self, bytes: &'c [u8]) -> Result<&'c str> {
        from_utf8(bytes).map_err(Error::Utf8)
    }

    /// Decodes a UTF8 slice without BOM (Byte order mark) regardless of XML declaration.
    ///
    /// Decode `bytes` without BOM and with malformed sequences replaced with the
    /// `U+FFFD REPLACEMENT CHARACTER`.
    ///
    /// # Note
    ///
    /// If you instead want to use XML declared encoding, use the `encoding` feature
    #[inline]
    #[cfg(not(feature = "encoding"))]
    fn decode_without_bom<'c>(&self, bytes: &'c [u8]) -> Result<&'c str> {
        if bytes.starts_with(b"\xEF\xBB\xBF") {
            from_utf8(&bytes[3..]).map_err(Error::Utf8)
        } else {
            from_utf8(bytes).map_err(Error::Utf8)
        }
    }

    /// Decodes a slice using without BOM (Byte order mark) the encoding specified in the XML declaration.
    ///
    /// Decode `bytes` without BOM and with malformed sequences replaced with the
    /// `U+FFFD REPLACEMENT CHARACTER`.
    ///
    /// If no encoding is specified, defaults to UTF-8.
    #[inline]
    #[cfg(feature = "encoding")]
    fn decode_without_bom<'b, 'c>(&'b mut self, mut bytes: &'c [u8]) -> Cow<'c, str> {
        if self.read_is_encoding_set() {
            return self.read_encoding().decode_with_bom_removal(bytes).0;
        }
        if bytes.starts_with(b"\xEF\xBB\xBF") {
            self.write_is_encoding_set(true);
            bytes = &bytes[3..];
        } else if bytes.starts_with(b"\xFF\xFE") {
            self.write_is_encoding_set(true);
            self.write_encoding(UTF_16LE);
            bytes = &bytes[2..];
        } else if bytes.starts_with(b"\xFE\xFF") {
            self.write_is_encoding_set(true);
            self.write_encoding(UTF_16BE);
            bytes = &bytes[3..];
        };
        self.read_encoding().decode_without_bom_handling(bytes).0
    }

    #[cfg(feature = "encoding")]
    /// Returns the encoding specified in the xml, defaults to utf8
    fn read_encoding(&self) -> &'static Encoding;

    #[cfg(feature = "encoding")]
    /// check if quick-rs could find out the encoding
    fn read_is_encoding_set(&self) -> bool;

    #[cfg(feature = "encoding")]
    /// Returns the encoding specified in the xml, defaults to utf8
    fn write_encoding(&mut self, val: &'static Encoding);

    #[cfg(feature = "encoding")]
    /// check if quick-rs could find out the encoding
    fn write_is_encoding_set(&mut self, val: bool);
}

#[derive(Clone, Debug)]
enum TagState {
    Opened,
    Closed,
    Empty,
    /// Either Eof or Errored
    Exit,
}

/// A function to check whether the byte is a whitespace (blank, new line, carriage return or tab)
#[inline]
pub(crate) fn is_whitespace(b: u8) -> bool {
    match b {
        b' ' | b'\r' | b'\n' | b'\t' => true,
        _ => false,
    }
}

/// A namespace declaration. Can either bind a namespace to a prefix or define the current default
/// namespace.
#[derive(Clone, Debug)]
struct Namespace {
    /// Index of the namespace in the buffer
    start: usize,
    /// Length of the prefix
    /// * if bigger than start, then binds this namespace to the corresponding slice.
    /// * else defines the current default namespace.
    prefix_len: usize,
    /// The namespace name (the URI) of this namespace declaration.
    ///
    /// The XML standard specifies that an empty namespace value 'removes' a namespace declaration
    /// for the extent of its scope. For prefix declarations that's not very interesting, but it is
    /// vital for default namespace declarations. With `xmlns=""` you can revert back to the default
    /// behaviour of leaving unqualified element names unqualified.
    value_len: usize,
    /// Level of nesting at which this namespace was declared. The declaring element is included,
    /// i.e., a declaration on the document root has `level = 1`.
    /// This is used to pop the namespace when the element gets closed.
    level: i32,
}

impl Namespace {
    /// Gets the value slice out of namespace buffer
    ///
    /// Returns `None` if `value_len == 0`
    #[inline]
    fn opt_value<'a, 'b>(&'a self, ns_buffer: &'b [u8]) -> Option<&'b [u8]> {
        if self.value_len == 0 {
            None
        } else {
            let start = self.start + self.prefix_len;
            Some(&ns_buffer[start..start + self.value_len])
        }
    }

    /// Check if the namespace matches the potentially qualified name
    #[inline]
    fn is_match(&self, ns_buffer: &[u8], qname: &[u8]) -> bool {
        if self.prefix_len == 0 {
            !qname.contains(&b':')
        } else {
            qname.get(self.prefix_len).map_or(false, |n| *n == b':')
                && qname.starts_with(&ns_buffer[self.start..self.start + self.prefix_len])
        }
    }
}

/// A namespace management buffer.
///
/// Holds all internal logic to push/pop namespaces with their levels.
#[derive(Clone, Debug, Default)]
struct NamespaceBufferIndex {
    /// a buffer of namespace ranges
    slices: Vec<Namespace>,
    /// The number of open tags at the moment. We need to keep track of this to know which namespace
    /// declarations to remove when we encounter an `End` event.
    nesting_level: i32,
    /// For `Empty` events keep the 'scope' of the element on the stack artificially. That way, the
    /// consumer has a chance to use `resolve` in the context of the empty element. We perform the
    /// pop as the first operation in the next `next()` call.
    pending_pop: bool,
}

impl NamespaceBufferIndex {
    #[inline]
    fn find_namespace_value<'a, 'b, 'c>(
        &'a self,
        element_name: &'b [u8],
        buffer: &'c [u8],
    ) -> Option<&'c [u8]> {
        self.slices
            .iter()
            .rfind(|n| n.is_match(buffer, element_name))
            .and_then(|n| n.opt_value(buffer))
    }

    fn pop_empty_namespaces(&mut self, buffer: &mut Vec<u8>) {
        if !self.pending_pop {
            return;
        }
        self.pending_pop = false;
        self.nesting_level -= 1;
        let current_level = self.nesting_level;
        // from the back (most deeply nested scope), look for the first scope that is still valid
        match self.slices.iter().rposition(|n| n.level <= current_level) {
            // none of the namespaces are valid, remove all of them
            None => {
                buffer.clear();
                self.slices.clear();
            }
            // drop all namespaces past the last valid namespace
            Some(last_valid_pos) => {
                if let Some(len) = self.slices.get(last_valid_pos + 1).map(|n| n.start) {
                    buffer.truncate(len);
                    self.slices.truncate(last_valid_pos + 1);
                }
            }
        }
    }

    fn push_new_namespaces(&mut self, e: &BytesStart, buffer: &mut Vec<u8>) {
        self.nesting_level += 1;
        let level = self.nesting_level;
        // adds new namespaces for attributes starting with 'xmlns:' and for the 'xmlns'
        // (default namespace) attribute.
        for a in e.attributes().with_checks(false) {
            if let Ok(Attribute { key: k, value: v }) = a {
                if k.starts_with(b"xmlns") {
                    match k.get(5) {
                        None => {
                            let start = buffer.len();
                            buffer.extend_from_slice(&*v);
                            self.slices.push(Namespace {
                                start,
                                prefix_len: 0,
                                value_len: v.len(),
                                level,
                            });
                        }
                        Some(&b':') => {
                            let start = buffer.len();
                            buffer.extend_from_slice(&k[6..]);
                            buffer.extend_from_slice(&*v);
                            self.slices.push(Namespace {
                                start,
                                prefix_len: k.len() - 6,
                                value_len: v.len(),
                                level,
                            });
                        }
                        _ => break,
                    }
                }
            } else {
                break;
            }
        }
    }

    /// Resolves a potentially qualified **attribute name** into (namespace name, local name).
    ///
    /// *Qualified* attribute names have the form `prefix:local-name` where the`prefix` is defined
    /// on any containing XML element via `xmlns:prefix="the:namespace:uri"`. The namespace prefix
    /// can be defined on the same element as the attribute in question.
    ///
    /// *Unqualified* attribute names do *not* inherit the current *default namespace*.
    #[inline]
    fn resolve_namespace<'a, 'b, 'c>(
        &'a self,
        qname: &'b [u8],
        buffer: &'c [u8],
        use_default: bool,
    ) -> (Option<&'c [u8]>, &'b [u8]) {
        self.slices
            .iter()
            .rfind(|n| n.is_match(buffer, qname))
            .map_or((None, qname), |n| {
                let len = n.prefix_len;
                if len > 0 {
                    (n.opt_value(buffer), &qname[len + 1..])
                } else if use_default {
                    (n.opt_value(buffer), qname)
                } else {
                    (None, qname)
                }
            })
    }
}

/// Utf8 Decoder
#[cfg(not(feature = "encoding"))]
#[derive(Clone, Copy)]
pub struct Decoder;

/// Utf8 Decoder
#[cfg(feature = "encoding")]
#[derive(Clone, Copy)]
pub struct Decoder {
    encoding: &'static Encoding,
}

impl Decoder {
    /// Decode a slice of u8 into a UTF8 str
    #[cfg(not(feature = "encoding"))]
    pub fn decode<'c>(&self, bytes: &'c [u8]) -> Result<&'c str> {
        from_utf8(bytes).map_err(Error::Utf8)
    }

    /// Decode a slice of u8 into a Cow str
    #[cfg(feature = "encoding")]
    pub fn decode<'c>(&self, bytes: &'c [u8]) -> Cow<'c, str> {
        self.encoding.decode(bytes).0
    }
}

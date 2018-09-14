//! A module to handle `Writer`

use std::io::Write;

use errors::{Error, Result};
use events::Event;

/// XML writer.
///
/// Writes XML `Event`s to a `Write` implementor.
///
/// # Examples
///
/// ```rust
/// # extern crate failure;
/// # extern crate quick_xml;
/// # fn main() {
/// use failure::Fail;
/// use quick_xml::{Reader, Writer};
/// use quick_xml::events::{Event, BytesEnd, BytesStart};
/// use std::io::Cursor;
///
/// let xml = r#"<this_tag k1="v1" k2="v2"><child>text</child></this_tag>"#;
/// let mut reader = Reader::from_str(xml);
/// reader.trim_text(true);
/// let mut writer = Writer::new(Cursor::new(Vec::new()));
/// let mut buf = Vec::new();
/// loop {
///     match reader.read_event(&mut buf) {
///         Ok(Event::Start(ref e)) if e.name() == b"this_tag" => {
///
///             // crates a new element ... alternatively we could reuse `e` by calling
///             // `e.into_owned()`
///             let mut elem = BytesStart::owned(b"my_elem".to_vec(), "my_elem".len());
///
///             // collect existing attributes
///             elem.extend_attributes(e.attributes().map(|attr| attr.unwrap()));
///
///             // copy existing attributes, adds a new my-key="some value" attribute
///             elem.push_attribute(("my-key", "some value"));
///
///             // writes the event to the writer
///             assert!(writer.write_event(Event::Start(elem)).is_ok());
///         },
///         Ok(Event::End(ref e)) if e.name() == b"this_tag" => {
///             assert!(writer.write_event(Event::End(BytesEnd::borrowed(b"my_elem"))).is_ok());
///         },
///         Ok(Event::Eof) => break,
///         // we can either move or borrow the event to write, depending on your use-case
///         Ok(e) => assert!(writer.write_event(&e).is_ok()),
///         // errors are chained, the last one usually being the
///         // position where the error has happened
///         Err(e) => panic!("{:?}", e.causes().map(|e| format!("{:?} -", e)).collect::<String>()),
///     }
///     buf.clear();
/// }
///
/// let result = writer.into_inner().into_inner();
/// let expected = r#"<my_elem k1="v1" k2="v2" my-key="some value"><child>text</child></my_elem>"#;
/// assert_eq!(result, expected.as_bytes());
/// # }
/// ```
#[derive(Clone)]
pub struct Writer<W: Write> {
    /// underlying writer
    writer: W,
    indent: Option<Indentation>,
}

impl<W: Write> Writer<W> {
    /// Creates a Writer from a generic Write
    pub fn new(inner: W) -> Writer<W> {
        Writer {
            writer: inner,
            indent: None,
        }
    }

    /// Creates a Writer with configured whitespace indents from a generic Write
    pub fn new_with_indent(inner: W, indent_char: u8, indent_size: usize) -> Writer<W> {
        Writer {
            writer: inner,
            indent: Some(Indentation::new(indent_char, indent_size)),
        }
    }

    /// Consumes this `Writer`, returning the underlying writer.
    pub fn into_inner(self) -> W {
        self.writer
    }

    /// Writes the given event to the underlying writer.
    pub fn write_event<'a, E: AsRef<Event<'a>>>(&mut self, event: E) -> Result<usize> {
        let mut next_should_line_break = true;
        let result = match *event.as_ref() {
            Event::Start(ref e) => {
                let result = self.write_wrapped(b"<", e, b">");
                if let Some(i) = self.indent.as_mut() {
                    i.grow();
                }
                result
            }
            Event::End(ref e) => {
                if let Some(i) = self.indent.as_mut() {
                    i.shrink();
                }
                self.write_wrapped(b"</", e, b">")
            }
            Event::Empty(ref e) => self.write_wrapped(b"<", e, b"/>"),
            Event::Text(ref e) => {
                next_should_line_break = false;
                self.write(&e.escaped())
            }
            Event::Comment(ref e) => self.write_wrapped(b"<!--", e, b"-->"),
            Event::CData(ref e) => self.write_wrapped(b"<![CDATA[", e, b"]]>"),
            Event::Decl(ref e) => self.write_wrapped(b"<?", e, b"?>"),
            Event::PI(ref e) => self.write_wrapped(b"<?", e, b"?>"),
            Event::DocType(ref e) => self.write_wrapped(b"<!DOCTYPE", e, b">"),
            Event::Eof => Ok(0),
        };
        if let Some(i) = self.indent.as_mut() {
            i.should_line_break = next_should_line_break;
        }
        result
    }

    /// Writes bytes
    #[inline]
    pub fn write(&mut self, value: &[u8]) -> Result<usize> {
        self.writer.write(value).map_err(Error::Io)
    }

    #[inline]
    fn write_wrapped(&mut self, before: &[u8], value: &[u8], after: &[u8]) -> Result<usize> {
        let mut wrote = 0;
        if let Some(ref i) = self.indent {
            if i.should_line_break {
                wrote = self.writer.write(b"\n").map_err(Error::Io)? + self
                    .writer
                    .write(&i.indents[..i.indents_len])
                    .map_err(Error::Io)?;
            }
        }
        Ok(wrote + self.write(before)? + self.write(value)? + self.write(after)?)
    }
}

#[derive(Clone)]
struct Indentation {
    should_line_break: bool,
    indent_char: u8,
    indent_size: usize,
    indents: Vec<u8>,
    indents_len: usize,
}

impl Indentation {
    fn new(indent_char: u8, indent_size: usize) -> Indentation {
        Indentation {
            should_line_break: false,
            indent_char,
            indent_size,
            indents: vec![indent_char; 128],
            indents_len: 0,
        }
    }

    fn grow(&mut self) {
        self.indents_len = self.indents_len + self.indent_size;
        if self.indents_len > self.indents.len() {
            self.indents.resize(self.indents_len, self.indent_char);
        }
    }

    fn shrink(&mut self) {
        self.indents_len = self.indents_len - self.indent_size;
    }
}

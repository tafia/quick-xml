//! A module to handle `Writer`

use std::io::Write;

use errors::Result;
use events::Event;

/// Xml writer
///
/// Consumes a `Write` and writes xml Events
///
/// ```rust
/// use quick_xml::writer::Writer;
/// use quick_xml::events::{Event, BytesEnd, BytesStart};
/// use quick_xml::reader::Reader;
/// use std::io::Cursor;
/// use std::iter;
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
///         // error are chained, the last one usually being the
///         // position where the error has happened
///         Err(e) => panic!("{:?}", e.iter().map(|e| format!("{:?} -", e)).collect::<String>()),
///     }
///     buf.clear();
/// }
///
/// let result = writer.into_inner().into_inner();
/// let expected = r#"<my_elem k1="v1" k2="v2" my-key="some value"><child>text</child></my_elem>"#;
/// assert_eq!(result, expected.as_bytes());
/// ```
#[derive(Clone)]
pub struct Writer<W: Write> {
    /// underlying writer
    writer: W,
}

impl<W: Write> Writer<W> {
    /// Creates a Writer from a generic Write
    pub fn new(inner: W) -> Writer<W> {
        Writer { writer: inner }
    }

    /// Consumes this `Writer`, returning the underlying writer.
    pub fn into_inner(self) -> W {
        self.writer
    }

    /// Writes the given event to the underlying writer.
    pub fn write_event<'a, E: AsRef<Event<'a>>>(&mut self, event: E) -> Result<usize> {
        match *event.as_ref() {
            Event::Start(ref e) => self.write_wrapped(b"<", e, b">"),
            Event::End(ref e) => self.write_wrapped(b"</", e, b">"),
            Event::Empty(ref e) => self.write_wrapped(b"<", e, b"/>"),
            Event::Text(ref e) => self.write(&e.escaped()),
            Event::Comment(ref e) => self.write_wrapped(b"<!--", e, b"-->"),
            Event::CData(ref e) => self.write_wrapped(b"<![CDATA[", e, b"]]>"),
            Event::Decl(ref e) => self.write_wrapped(b"<?", e, b"?>"),
            Event::PI(ref e) => self.write_wrapped(b"<?", e, b"?>"),
            Event::DocType(ref e) => self.write_wrapped(b"<!DOCTYPE", e, b">"),
            Event::Eof => Ok(0),
        }
    }

    /// Writes bytes
    #[inline]
    pub fn write(&mut self, value: &[u8]) -> Result<usize> {
        self.writer.write(value).map_err(|e| e.into())
    }

    #[inline]
    fn write_wrapped(&mut self, before: &[u8], value: &[u8], after: &[u8]) -> Result<usize> {
        Ok(
            self.writer.write(before)? + self.writer.write(value)? + self.writer.write(after)?,
        )
    }
}

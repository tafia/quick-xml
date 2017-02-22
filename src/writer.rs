//! A module to handle `Writer`

use std::io::Write;

use error::Result;
use events::BytesEvent;

/// Xml writer
///
/// Consumes a `Write` and writes xml Events
///
/// ```rust
/// use quick_xml::writer::Writer;
/// use quick_xml::events::{AsStr, BytesEvent, BytesEnd, BytesStart};
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
///         Ok(BytesEvent::Start(ref e)) if e.name() == b"this_tag" => {
///
///             // crates a new element ... alternatively we could reuse `e` by calling
///             // `e.into_owned()`
///             let mut elem = BytesStart::owned(b"my_elem".to_vec(), "my_elem".len());
///
///             // collect existing attributes
///             elem.with_attributes(e.attributes().map(|attr| attr.unwrap()));
///
///             // copy existing attributes, adds a new my-key="some value" attribute
///             elem.push_attribute(b"my-key", "some value");
///
///             // writes the event to the writer
///             assert!(writer.write(BytesEvent::Start(elem)).is_ok());
///         },
///         Ok(BytesEvent::End(ref e)) if e.name() == b"this_tag" => {
///             assert!(writer.write(BytesEvent::End(BytesEnd::borrowed(b"my_elem"))).is_ok());
///         },
///         Ok(BytesEvent::Eof) => break,
///         Ok(e) => assert!(writer.write(e).is_ok()),
///         Err((e, pos)) => panic!("{:?} at position {}", e, pos),
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
    pub fn write(&mut self, event: BytesEvent) -> Result<()> {
        match event {
            BytesEvent::Start(ref e) => self.write_wrapped_element(b"<", &e, b">"),
            BytesEvent::End(ref e) => self.write_wrapped_bytes(b"</", &e, b">"),
            BytesEvent::Empty(ref e) => self.write_wrapped_element(b"<", &e, b"/>"),
            BytesEvent::Text(ref e) => self.write_bytes(&e),
            BytesEvent::Comment(ref e) => self.write_wrapped_element(b"<!--", &e, b"-->"),
            BytesEvent::CData(ref e) => self.write_wrapped_element(b"<![CDATA[", &e, b"]]>"),
            BytesEvent::Decl(ref e) => self.write_wrapped_element(b"<?", &e, b"?>"),
            BytesEvent::PI(ref e) => self.write_wrapped_element(b"<?", &e, b"?>"),
            BytesEvent::DocType(ref e) => self.write_wrapped_element(b"<!DOCTYPE", &e, b">"),
            BytesEvent::Eof => Ok(()),
        }
    }

    #[inline]
    fn write_bytes(&mut self, value: &[u8]) -> Result<()> {
        try!(self.writer.write(value));
        Ok(())
    }

    fn write_wrapped_bytes(&mut self, before: &[u8], value: &[u8], after: &[u8])
        -> Result<()>
    {
        try!(self.writer.write(before)
            .and_then(|_| self.writer.write(value))
            .and_then(|_| self.writer.write(after)));
        Ok(())
    }

    #[inline]
    fn write_wrapped_element(&mut self, before: &[u8], element: &[u8], after: &[u8])
        -> Result<()>
    {
        self.write_wrapped_bytes(before, element, after)
    }
}

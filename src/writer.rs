//! A module to handle `XmlWriter`

use std::io::Write;

use error::Result;
use super::{Event, Element};

/// Xml writer
///
/// Consumes a `Write` and writes xml Events
///
/// ```
/// use quick_xml::{AsStr, Element, Event, XmlReader, XmlWriter};
/// use quick_xml::Event::*;
/// use std::io::Cursor;
/// use std::iter;
///
/// let xml = r#"<this_tag k1="v1" k2="v2"><child>text</child></this_tag>"#;
/// let reader = XmlReader::from(xml).trim_text(true);
/// let mut writer = XmlWriter::new(Cursor::new(Vec::new()));
/// for r in reader {
///     match r {
///         Ok(Event::Start(ref e)) if e.name() == b"this_tag" => {
///             // collect existing attributes
///             let mut attrs = e.attributes()
///                              .map(|attr| attr.unwrap()).collect::<Vec<_>>();
///
///             // copy existing attributes, adds a new my-key="some value" attribute
///             let mut elem = Element::new("my_elem").with_attributes(attrs);
///             elem.push_attribute(b"my-key", "some value");
///
///             // writes the event to the writer
///             assert!(writer.write(Start(elem)).is_ok());
///         },
///         Ok(Event::End(ref e)) if e.name() == b"this_tag" => {
///             assert!(writer.write(End(Element::new("my_elem"))).is_ok());
///         },
///         Ok(e) => assert!(writer.write(e).is_ok()),
///         Err((e, pos)) => panic!("{:?} at position {}", e, pos),
///     }
/// }
///
/// let result = writer.into_inner().into_inner();
/// let expected = r#"<my_elem k1="v1" k2="v2" my-key="some value"><child>text</child></my_elem>"#;
/// assert_eq!(result, expected.as_bytes());
/// ```
#[derive(Clone)]
pub struct XmlWriter<W: Write> {
    /// underlying writer
    writer: W,
}

impl<W: Write> XmlWriter<W> {
    /// Creates a XmlWriter from a generic Write
    pub fn new(inner: W) -> XmlWriter<W> {
        XmlWriter { writer: inner }
    }

    /// Consumes this `XmlWriter`, returning the underlying writer.
    pub fn into_inner(self) -> W {
        self.writer
    }

    /// Writes the given event to the underlying writer.
    pub fn write(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Start(ref e) => self.write_wrapped_element(b"<", e, b">"),
            Event::End(ref e) => self.write_wrapped_bytes(b"</", &e.name(), b">"),
            Event::Empty(ref e) => self.write_wrapped_element(b"<", e, b"/>"),
            Event::Text(ref e) => self.write_bytes(e.content()),
            Event::Comment(ref e) => self.write_wrapped_element(b"<!--", e, b"-->"),
            Event::CData(ref e) => self.write_wrapped_element(b"<![CDATA[", e, b"]]>"),
            Event::Decl(ref e) => self.write_wrapped_element(b"<?", &e.element, b"?>"),
            Event::PI(ref e) => self.write_wrapped_element(b"<?", e, b"?>"),
            Event::DocType(ref e) => self.write_wrapped_element(b"<!DOCTYPE", e, b">"),
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
    fn write_wrapped_element(&mut self, before: &[u8], element: &Element, after: &[u8])
        -> Result<()>
    {
        self.write_wrapped_bytes(before, &element.content(), after)
    }
}

//! This example demonstrate how custom entities can be extracted from the DOCTYPE
//! and usage of the high-level `Reader` API.
//!
//! NB: this example is deliberately kept simple:
//! * the regex in this example is simple but brittle.

use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt;
use std::io::{BufRead, Cursor};

use quick_xml::events::{BytesEnd, BytesStart, BytesText};
use quick_xml::reader::{
    EntityResolver, EntityResolverFactory, Reader, ReplacementText, XmlEvent, XmlReader,
};
use regex::bytes::Regex;

use pretty_assertions::assert_eq;

const XML1: &str = r#"
<!DOCTYPE test [
<!ENTITY text "hello world" >
<!ENTITY element1 "<dtd attr = 'Message: &text;'/>" >
<!ENTITY element2 "<a> &element1; </a>" >
]>
<test label="Message: &text;">&element2;</test>
&external;
"#;

/// Additional document which in reality would be referenced by
/// `<!ENTITY external SYSTEM "URI to the document, for example, relative file path" >`
const XML2: &str = r#"
<?xml version='1.0'?>
<external>text</external>
"#;

struct MyResolver<'i> {
    /// Map of captured internal _parsed general entities_. _Parsed_ means that
    /// value of the entity is parsed by XML reader.
    entities: HashMap<Cow<'i, [u8]>, Cow<'i, [u8]>>,
    /// In this example we use simple regular expression to capture entities from DTD.
    /// In real application you should use DTD parser.
    entity_re: Regex,
}
impl<'i> fmt::Debug for MyResolver<'i> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(self.entities.iter().map(|(k, v)| {
                (
                    std::str::from_utf8(k).unwrap(),
                    std::str::from_utf8(v).unwrap(),
                )
            }))
            .finish()
    }
}

impl<'i> MyResolver<'i> {
    fn new() -> Result<Self, regex::Error> {
        Ok(Self {
            entities: Default::default(),
            // Capture "name" and "content" from such string:
            // <!ENTITY name "content" >
            entity_re: Regex::new(r#"<!ENTITY\s+([^ \t\r\n]+)\s+"([^"]*)"\s*>"#)?,
        })
    }
    fn capture_borrowed(&mut self, doctype: &'i [u8]) {
        for cap in self.entity_re.captures_iter(doctype) {
            self.entities.insert(
                cap.get(1).unwrap().as_bytes().into(),
                cap.get(2).unwrap().as_bytes().into(),
            );
        }
    }
    fn capture_owned(&mut self, doctype: Vec<u8>) {
        for cap in self.entity_re.captures_iter(&doctype) {
            self.entities.insert(
                cap.get(1).unwrap().as_bytes().to_owned().into(),
                cap.get(2).unwrap().as_bytes().to_owned().into(),
            );
        }
    }
}

impl<'i> EntityResolverFactory<'i> for MyResolver<'i> {
    type CaptureError = Infallible;
    type Resolver = Self;

    fn new_resolver(&mut self) -> Self::Resolver {
        // We use valid regex so cannot fail
        Self::new().unwrap()
    }
}

impl<'i> EntityResolver<'i> for MyResolver<'i> {
    type CaptureError = Infallible;

    fn capture(&mut self, doctype: BytesText<'i>) -> Result<(), Self::CaptureError> {
        dbg!(&doctype);
        match doctype.into_inner() {
            Cow::Borrowed(doctype) => self.capture_borrowed(doctype),
            Cow::Owned(doctype) => self.capture_owned(doctype),
        }
        dbg!(self);
        Ok(())
    }

    fn resolve<'e>(&self, entity: &str) -> Option<ReplacementText<'i, 'e>> {
        dbg!((entity, self));
        if entity == "external" {
            return Some(ReplacementText::External(Box::new(Cursor::new(
                XML2.as_bytes(),
            ))));
        }
        match self.entities.get(entity.as_bytes()) {
            Some(replacement) => Some(ReplacementText::Internal(replacement.clone())),
            None => None,
        }
    }
}

/// In this example the events will borrow from the first document
fn borrowed() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = Reader::from_str(XML1);
    reader.config_mut().trim_text(true);

    let mut r = XmlReader::borrowed(reader, MyResolver::new()?);

    assert_eq!(
        r.read_event()?,
        XmlEvent::Start(BytesStart::from_content(
            r#"test label="Message: &text;""#,
            4
        ))
    );

    //--------------------------------------------------------------------------
    // This part was inserted into original document from entity defined in DTD
    assert_eq!(r.read_event()?, XmlEvent::Start(BytesStart::new("a")));
    assert_eq!(
        r.read_event()?,
        XmlEvent::Empty(BytesStart::from_content(
            r#"dtd attr = 'Message: &text;'"#,
            3
        ))
    );
    assert_eq!(r.read_event()?, XmlEvent::End(BytesEnd::new("a")));
    //--------------------------------------------------------------------------

    assert_eq!(r.read_event()?, XmlEvent::End(BytesEnd::new("test")));

    //--------------------------------------------------------------------------
    // Start of external document
    assert_eq!(
        r.read_event()?,
        XmlEvent::Start(BytesStart::new("external"))
    );
    assert_eq!(r.read_event()?, XmlEvent::Text(BytesText::new("text")));
    assert_eq!(r.read_event()?, XmlEvent::End(BytesEnd::new("external")));
    //--------------------------------------------------------------------------

    assert_eq!(r.read_event()?, XmlEvent::Eof);

    Ok(())
}

/// In this example the events will always copy data
fn buffered() -> Result<(), Box<dyn std::error::Error>> {
    let boxed: Box<dyn BufRead> = Box::new(Cursor::new(XML1.as_bytes()));
    let mut reader = Reader::from_reader(boxed);
    reader.config_mut().trim_text(true);

    let mut r = XmlReader::buffered(reader, MyResolver::new()?);

    assert_eq!(
        r.read_event()?,
        XmlEvent::Start(BytesStart::from_content(
            r#"test label="Message: &text;""#,
            4
        ))
    );

    //--------------------------------------------------------------------------
    // This part was inserted into original document from entity defined in DTD
    assert_eq!(r.read_event()?, XmlEvent::Start(BytesStart::new("a")));
    assert_eq!(
        r.read_event()?,
        XmlEvent::Empty(BytesStart::from_content(
            r#"dtd attr = 'Message: &text;'"#,
            3
        ))
    );
    assert_eq!(r.read_event()?, XmlEvent::End(BytesEnd::new("a")));
    //--------------------------------------------------------------------------

    assert_eq!(r.read_event()?, XmlEvent::End(BytesEnd::new("test")));

    //--------------------------------------------------------------------------
    // Start of external document
    assert_eq!(
        r.read_event()?,
        XmlEvent::Start(BytesStart::new("external"))
    );
    assert_eq!(r.read_event()?, XmlEvent::Text(BytesText::new("text")));
    assert_eq!(r.read_event()?, XmlEvent::End(BytesEnd::new("external")));
    //--------------------------------------------------------------------------

    assert_eq!(r.read_event()?, XmlEvent::Eof);

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", XML1);
    // In this example the events will borrow from the first document
    borrowed()?;

    println!("----------------------------------------------------------------");
    println!("{}", XML1);
    // In this example the events will always copy data
    buffered()?;
    Ok(())
}

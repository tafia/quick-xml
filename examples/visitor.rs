//! The visitor pattern: write the parser once, reuse it for many consumers.
//!
//! The hand-written readers in `reader_patterns.rs` weld two things together:
//! *how to walk the document* and *what to build from it*. If a second part of
//! your program needs the same document for a different purpose, you either
//! parse it again or bolt the new logic onto the existing loop.
//!
//! The visitor pattern separates the two. A single driver function knows how to
//! walk the document and, at each point of interest, calls a method on a
//! `Visitor` trait. Each consumer implements that trait, overriding only the
//! callbacks it cares about. The parsing lives in one place; the consumers stay
//! small and independent.
//!
//! Two details make it efficient and pleasant to use:
//!
//!   * Callbacks receive **borrowed** `&str` slices that point into the reader's
//!     buffers. A consumer that only needs to count or hash never allocates; one
//!     that wants to keep the data calls `.to_owned()` itself. The borrow is
//!     valid only for the duration of the call.
//!
//!   * Trait methods have **default no-op bodies**, so a consumer overrides just
//!     the events it uses. `Stats` below ignores titles entirely simply by not
//!     implementing `title`.
//!
//! The driver here dispatches purely on tag name because this grammar allows it.
//! For a more complex document where the same tag means different things by
//! context, `parse_catalog` would track its position with an explicit state
//! `enum` (see `reader_patterns.rs`) and call the appropriate visitor method
//! from each state. The two techniques compose cleanly: the state machine is a
//! private detail of the driver, but the visitor trait doesn't change.
//!
//! Run it with:
//!
//! ```console
//! cargo run --example visitor
//! ```

use quick_xml::XmlVersion;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;

const XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<catalog>
    <book id="b1">
        <title>The Rust Programming Language</title>
        <author>Steve Klabnik</author>
        <author>Carol Nichols</author>
    </book>
    <book id="b2">
        <title>Programming Rust</title>
        <author>Jim Blandy</author>
    </book>
</catalog>"#;

// ---------------------------------------------------------------------------
// The visitor trait and the driver
// ---------------------------------------------------------------------------

/// Callbacks invoked while walking a `<catalog>`.
///
/// Every `&str` is borrowed from the reader and valid only for the duration of
/// the call; keep what you need by cloning it. All methods default to doing
/// nothing, so an implementor writes only the ones it cares about.
#[allow(unused_variables)]
trait CatalogVisitor {
    /// A `<book>` opened; `id` is its `id` attribute.
    fn begin_book(&mut self, id: &str) {}
    /// The book's `<title>` text.
    fn title(&mut self, title: &str) {}
    /// One `<author>` of the current book (called once per author).
    fn author(&mut self, name: &str) {}
    /// The current `<book>` closed.
    fn end_book(&mut self) {}
}

/// Read the `id` attribute off a start tag, or return an empty string.
fn id_of(e: &BytesStart, version: XmlVersion) -> Result<String, quick_xml::Error> {
    for attr in e.attributes() {
        let attr = attr?;
        if attr.key.as_ref() == "id" {
            return Ok(attr.normalized_value(version)?.into_owned());
        }
    }
    Ok(String::new())
}

/// Walk a catalog document, dispatching to `visitor`. This is the *only* place
/// that knows the document's structure.
fn parse_catalog(xml: &str, visitor: &mut dyn CatalogVisitor) -> Result<(), quick_xml::Error> {
    let mut reader = Reader::from_str(xml);
    let mut xml_version = XmlVersion::Implicit1_0;

    loop {
        match reader.read_event()? {
            Event::Decl(e) => xml_version = e.xml_version()?,
            Event::Start(e) => match e.name().as_ref() {
                "book" => {
                    let id = id_of(&e, xml_version)?;
                    visitor.begin_book(&id);
                }
                // `read_text` yields the element's text; `xml_content` unescapes
                // it into a `Cow<str>` we can hand out as a borrowed `&str`.
                "title" => {
                    let text = reader.read_text(e.name())?.xml_content(xml_version);
                    visitor.title(&text);
                }
                "author" => {
                    let text = reader.read_text(e.name())?.xml_content(xml_version);
                    visitor.author(&text);
                }
                _ => {}
            },
            Event::End(e) if e.name().as_ref() == "book" => visitor.end_book(),
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Consumer 1: materialize the document into owned structs
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Default)]
struct Book {
    id: String,
    title: String,
    authors: Vec<String>,
}

/// Builds a `Vec<Book>`, cloning the borrowed slices into owned `String`s.
#[derive(Default)]
struct Collector {
    books: Vec<Book>,
    current: Option<Book>,
}

impl CatalogVisitor for Collector {
    fn begin_book(&mut self, id: &str) {
        self.current = Some(Book {
            id: id.to_owned(),
            ..Default::default()
        });
    }
    fn title(&mut self, title: &str) {
        self.current.as_mut().unwrap().title = title.to_owned();
    }
    fn author(&mut self, name: &str) {
        self.current.as_mut().unwrap().authors.push(name.to_owned());
    }
    fn end_book(&mut self) {
        self.books.push(self.current.take().unwrap());
    }
}

// ---------------------------------------------------------------------------
// Consumer 2: aggregate without building anything
// ---------------------------------------------------------------------------

/// Counts books and authors. It never keeps the borrowed text, so it allocates
/// nothing, and it leaves `title` / `end_book` as the default no-ops.
#[derive(Default)]
struct Stats {
    books: usize,
    authors: usize,
}

impl CatalogVisitor for Stats {
    fn begin_book(&mut self, _id: &str) {
        self.books += 1;
    }
    fn author(&mut self, _name: &str) {
        self.authors += 1;
    }
}

fn main() -> Result<(), quick_xml::Error> {
    // The same driver feeds two unrelated consumers.
    let mut collector = Collector::default();
    parse_catalog(XML, &mut collector)?;

    let mut stats = Stats::default();
    parse_catalog(XML, &mut stats)?;

    println!("collected: {:#?}", collector.books);
    println!("stats: {} books, {} authors", stats.books, stats.authors);

    let expected = vec![
        Book {
            id: "b1".to_string(),
            title: "The Rust Programming Language".to_string(),
            authors: vec!["Steve Klabnik".to_string(), "Carol Nichols".to_string()],
        },
        Book {
            id: "b2".to_string(),
            title: "Programming Rust".to_string(),
            authors: vec!["Jim Blandy".to_string()],
        },
    ];

    assert_eq!(collector.books, expected);
    assert_eq!(stats.books, 2);
    assert_eq!(stats.authors, 3);

    Ok(())
}

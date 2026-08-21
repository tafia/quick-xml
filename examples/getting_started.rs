//! Getting started with the `quick-xml` pull reader.
//!
//! This is the best example to read *first*. It walks through the core loop that
//! every hand-written `quick-xml` reader is built on:
//!
//!   1. Create a [`Reader`].
//!   2. Repeatedly ask it for the next [`Event`] until you reach [`Event::Eof`].
//!   3. `match` on the event and pull out the data you care about.
//!
//! quick-xml is a *pull* parser: it never builds a document tree in memory.
//! Instead it hands you one small event at a time (`<book>` start, some text,
//! `</book>` end, ...) and you decide what to keep. That is what makes it fast
//! and memory-light, but it also means *you* are responsible for tracking where
//! you are in the document, and the parser will be as permissive or as strict
//! as you decide to make it.
//!
//! Run it with:
//!
//! ```console
//! cargo run --example getting_started
//! ```
//!
//! Once this makes sense, see:
//! - `reader_patterns.rs` for structuring larger readers (state machines vs. nested readers)
//! - `serde_roundtrip.rs` for skipping the manual loop entirely with `#[derive(Deserialize)]`
//! - `writer.rs` for producing XML
//! - `examples/README.md` for a guide on which approach to choose

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use quick_xml::XmlVersion;

const XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<catalog>
    <book id="b1" available="true">
        <title lang="en">The Rust Programming Language</title>
        <author>Steve Klabnik</author>
        <author>Carol Nichols</author>
        <price currency="USD">39.95</price>
    </book>
    <book id="b2" available="false">
        <title lang="en">Programming Rust</title>
        <author>Jim Blandy</author>
        <price currency="USD">47.99</price>
    </book>
</catalog>"#;

fn main() -> Result<(), quick_xml::Error> {
    // `from_str` reads directly from a `&str`. Because the whole document is already in memory,
    // the events can *borrow* from it and no extra read buffer is needed. (For streaming from
    // a file or socket, see `read_buffered.rs`, which reuses a `Vec<u8>` to keep allocations low.)
    let mut reader = Reader::from_str(XML);

    let mut titles = Vec::new();
    let mut book_count = 0u32;

    // Track the XML version declared in the `<?xml?>` header. It affects attribute value
    // normalization and other parsing behavior.
    let mut xml_version = XmlVersion::Implicit1_0;

    // The reader does not implement `Iterator` (its events borrow from an internal buffer),
    // so we drive it with a plain loop.
    loop {
        match reader.read_event()? {
            Event::Decl(e) => xml_version = e.xml_version()?,
            // <book ...> — an opening tag. `BytesStart` gives us the name and access to attributes.
            Event::Start(e) if e.name().as_ref() == "book" => {
                book_count += 1;

                // Attributes are parsed lazily. Iterate them and decode the one
                // we want. `normalized_value` unescapes entities (`&amp;` -> `&`)
                // and applies the whitespace normalization the XML spec requires
                // for attribute values.
                for attr in e.attributes() {
                    let attr = attr?;
                    if attr.key.as_ref() == "id" {
                        let id = attr.normalized_value(xml_version)?;
                        println!("found book id={id}");
                    }
                }
            }

            // <title ...> — rather than wait for the following Text and End
            // events, `read_text` consumes everything up to `</title>` and hands
            // back the text in one step. It returns the raw (still-escaped)
            // text; `xml_content` unescapes it into the logical string value.
            Event::Start(e) if e.name().as_ref() == "title" => {
                let text = reader.read_text(e.name())?;
                titles.push(text.xml_content(xml_version).into_owned());
            }

            // `read_event` yields `Eof` exactly once, when the document ends.
            Event::Eof => break,

            // Start/End/Text/Comment/CData/PI/Decl/... — everything we don't
            // care about in this example is simply ignored. Production-grade
            // code might decide to perform more rigorous structural checks
            // on the XML, see `reader_patterns.rs`
            _ => {}
        }
    }

    println!("read {book_count} books: {titles:?}");

    assert_eq!(book_count, 2);
    assert_eq!(
        titles,
        ["The Rust Programming Language", "Programming Rust"]
    );

    Ok(())
}

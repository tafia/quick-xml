//! Writing XML with `quick-xml`.
//!
//! This example builds the same small document two ways so you can compare them:
//!
//!   1. `write_high_level` — the recommended [`Writer::create_element`] builder
//!      API. Attributes and nested content read top-to-bottom like the XML you
//!      are producing, and elements can be nested with `write_inner_content`.
//!   2. `write_low_level` — the underlying [`Writer::write_event`] API, where you
//!      emit each `Start`/`Text`/`End` event yourself. It is more verbose but
//!      gives you full control and is what `create_element` calls internally.
//!
//! Both produce byte-for-byte identical output.
//!
//! If your data already lives in Rust structs, prefer serde instead of either of
//! these (see `serde_roundtrip.rs`) — it is far less code. Reach for the `Writer`
//! when you need precise control over the output, are transforming XML on the
//! fly, or want to avoid the serde dependency.
//!
//! Run it with:
//!
//! ```console
//! cargo run --example writer
//! ```

use std::io::Cursor;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::writer::Writer;

/// The data we want to serialize into XML.
struct Book {
    id: &'static str,
    available: bool,
    title: &'static str,
    authors: &'static [&'static str],
    price: f64,
    currency: &'static str,
}

const BOOKS: &[Book] = &[
    Book {
        id: "b1",
        available: true,
        title: "The Rust Programming Language",
        authors: &["Steve Klabnik", "Carol Nichols"],
        price: 39.95,
        currency: "USD",
    },
    Book {
        id: "b2",
        available: false,
        title: "Programming Rust",
        authors: &["Jim Blandy"],
        price: 47.99,
        currency: "USD",
    },
];

/// Build the document with the high-level [`ElementWriter`] builder API.
///
/// [`ElementWriter`]: quick_xml::writer::ElementWriter
fn write_high_level() -> Result<String, quick_xml::Error> {
    // `new_with_indent` pretty-prints the output. Use plain `Writer::new` for
    // compact, unindented XML.
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 4);

    // Write out the Xml Declaration
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
    // You could use `create_element` for the root node as well... but it is rarely worth it.
    writer.write_event(Event::Start(BytesStart::new("catalog")))?;

    // `create_element` returns a builder. `write_inner_content` takes a closure
    // that receives the same writer, so elements nest naturally: everything
    // written inside the closure lands between `<catalog>` and `</catalog>`.
    for book in BOOKS {
        writer
            .create_element("book")
            // Chain `with_attribute` to add attributes. Values that are
            // not already `&str` need converting first.
            .with_attribute(("id", book.id))
            .with_attribute(("available", if book.available { "true" } else { "false" }))
            .write_inner_content(|writer| {
                writer
                    .create_element("title")
                    .with_attribute(("lang", "en"))
                    // `write_text_content` writes escaped text and the
                    // closing tag: <title ...>...</title>.
                    .write_text_content(BytesText::new(book.title))?;

                for author in book.authors {
                    writer
                        .create_element("author")
                        .write_text_content(BytesText::new(author))?;
                }

                writer
                    .create_element("price")
                    .with_attribute(("currency", book.currency))
                    .write_text_content(BytesText::new(&book.price.to_string()))?;
                Ok(())
            })?;
    }

    writer.write_event(Event::End(BytesEnd::new("catalog")))?;

    Ok(String::from_utf8(writer.into_inner().into_inner()).unwrap())
}

/// Build the same document by emitting individual events.
///
/// This is what `create_element` does under the hood. You are responsible for
/// pairing every `Start` with an `End`, which is easy to get wrong — hence the
/// builder API above is usually preferable.
fn write_low_level() -> Result<String, quick_xml::Error> {
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 4);

    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
    writer.write_event(Event::Start(BytesStart::new("catalog")))?;

    for book in BOOKS {
        // Build a start tag and attach attributes before writing it.
        let mut book_tag = BytesStart::new("book");
        book_tag.push_attribute(("id", book.id));
        book_tag.push_attribute(("available", if book.available { "true" } else { "false" }));
        writer.write_event(Event::Start(book_tag))?;

        let mut title_tag = BytesStart::new("title");
        title_tag.push_attribute(("lang", "en"));
        writer.write_event(Event::Start(title_tag))?;
        // `BytesText::new` escapes the text for you (`&` -> `&amp;`, etc.).
        writer.write_event(Event::Text(BytesText::new(book.title)))?;
        writer.write_event(Event::End(BytesEnd::new("title")))?;

        for author in book.authors {
            writer.write_event(Event::Start(BytesStart::new("author")))?;
            writer.write_event(Event::Text(BytesText::new(author)))?;
            writer.write_event(Event::End(BytesEnd::new("author")))?;
        }

        let mut price_tag = BytesStart::new("price");
        price_tag.push_attribute(("currency", book.currency));
        writer.write_event(Event::Start(price_tag))?;
        writer.write_event(Event::Text(BytesText::new(&book.price.to_string())))?;
        writer.write_event(Event::End(BytesEnd::new("price")))?;

        writer.write_event(Event::End(BytesEnd::new("book")))?;
    }

    writer.write_event(Event::End(BytesEnd::new("catalog")))?;

    Ok(String::from_utf8(writer.into_inner().into_inner()).unwrap())
}

fn main() -> Result<(), quick_xml::Error> {
    let high = write_high_level()?;
    let low = write_low_level()?;

    println!("{high}");

    // Both APIs produce exactly the same bytes.
    assert_eq!(high, low);
    assert!(high.contains(r#"<book id="b1" available="true">"#));
    assert!(high.contains("<author>Carol Nichols</author>"));

    Ok(())
}

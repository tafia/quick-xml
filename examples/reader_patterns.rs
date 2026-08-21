//! Two patterns for structuring a hand-written reader.
//!
//! Once a document is more than a flat list of elements, the naive "one big
//! `match` in one big loop" becomes hard to follow. This example shows the two
//! patterns most people reach for, parsing the *same* document into the *same*
//! `Vec<Book>` so you can compare them directly:
//!
//!   1. `parse_state_machine` — track "where am I?" in an explicit `enum` and
//!      `match` on `(state, event)` pairs. Matching the state and the event
//!      together makes the grammar you accept very visible, and it reuses a
//!      single buffer for the whole document.
//!
//!   2. `parse_nested_readers` — when you recognize the start of a subtree (a
//!      `<book>`), hand control to a helper function that consumes just that
//!      subtree with its own inner loop. This reads naturally when the document
//!      is deeply nested, at the cost of being harder to share one read buffer
//!      across levels (each helper tends to want its own).
//!
//! Neither is "correct" — pick whichever keeps *your* document readable. And
//! before writing either by hand, check whether serde (`serde_roundtrip.rs`)
//! would do the whole job for you. See `examples/README.md` for the full
//! decision guide.
//!
//! Run it with:
//!
//! ```console
//! cargo run --example reader_patterns
//! ```

use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use quick_xml::XmlVersion;

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

#[derive(Debug, PartialEq, Default)]
struct Book {
    id: String,
    title: String,
    authors: Vec<String>,
}

/// Read the `id` attribute off a start tag, or return an empty string.
fn id_of(e: &BytesStart) -> Result<String, quick_xml::Error> {
    for attr in e.attributes() {
        let attr = attr?;
        if attr.key.as_ref() == "id" {
            return Ok(attr.normalized_value(XmlVersion::Implicit1_0)?.into_owned());
        }
    }
    Ok(String::new())
}

// ---------------------------------------------------------------------------
// Pattern 1: explicit state machine
// ---------------------------------------------------------------------------

/// Where we currently are in the document.
#[derive(Debug)]
enum State {
    /// Outside any `<book>`.
    Idle,
    /// Inside a `<book>`, accumulating its fields.
    InBook(Book),
    /// Inside a text-bearing child of a book; remember which one so we know
    /// where to store the upcoming `Text` event.
    InField(Book, Field),
}

#[derive(Debug, Clone, Copy)]
enum Field {
    Title,
    Author,
}

fn parse_state_machine(xml: &str) -> Result<Vec<Book>, quick_xml::Error> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut books = Vec::new();
    let mut state = State::Idle;

    loop {
        // Matching on `(state, event)` together makes the accepted grammar
        // explicit: each arm says "in this state, on this event, do X and move
        // to state Y". Every arm returns the next state.
        state = match (state, reader.read_event()?) {
            // Enter a book, capturing its id up front.
            (State::Idle, Event::Start(e)) if e.name().as_ref() == "book" => State::InBook(Book {
                id: id_of(&e)?,
                ..Default::default()
            }),

            // Enter a known field inside a book (`Field` is `Copy`, so the text
            // arm below can store it and hand it back unchanged).
            (State::InBook(book), Event::Start(e)) if e.name().as_ref() == "title" => {
                State::InField(book, Field::Title)
            }
            (State::InBook(book), Event::Start(e)) if e.name().as_ref() == "author" => {
                State::InField(book, Field::Author)
            }
            // Any other child of a book: skip its whole subtree and stay put.
            (State::InBook(book), Event::Start(e)) => {
                reader.read_to_end(e.name())?;
                State::InBook(book)
            }

            // The text inside a field goes to the slot the state remembered.
            (State::InField(mut book, field), Event::Text(e)) => {
                let text = e.xml_content(XmlVersion::Implicit1_0).into_owned();
                match field {
                    Field::Title => book.title = text,
                    Field::Author => book.authors.push(text),
                }
                State::InField(book, field)
            }

            // Closing a field returns us to the book.
            (State::InField(book, _), Event::End(_)) => State::InBook(book),

            // Closing the book stores it.
            (State::InBook(book), Event::End(e)) if e.name().as_ref() == "book" => {
                books.push(book);
                State::Idle
            }

            (State::Idle, Event::Eof) => break,

            // Any other (state, event) combination is uninteresting: keep the state unchanged.
            // `read_event` still moves us forward.
            // NOTE: If you want to be strict about document structure, or implement some form of
            // validation, you would want to raise errors instead of ignore unexpected states.
            (state, _) => state,
        };
    }

    Ok(books)
}

// ---------------------------------------------------------------------------
// Pattern 2: nested readers
// ---------------------------------------------------------------------------

fn parse_nested_readers(xml: &str) -> Result<Vec<Book>, quick_xml::Error> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut books = Vec::new();

    // The outer loop only knows about the top level: find each `<book>` and
    // delegate the details to `read_book`.
    loop {
        match reader.read_event()? {
            Event::Start(e) if e.name().as_ref() == "book" => {
                books.push(read_book(&mut reader, &e)?);
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(books)
}

/// Consume a single `<book>...</book>` subtree, starting *after* its start tag
/// has been read, and stopping once its matching end tag is consumed.
fn read_book(reader: &mut Reader<&[u8]>, start: &BytesStart) -> Result<Book, quick_xml::Error> {
    let mut book = Book {
        id: id_of(start)?,
        ..Default::default()
    };

    loop {
        match reader.read_event()? {
            Event::Start(e) => match e.name().as_ref() {
                // `read_text` consumes the child element's text and its end tag
                // in one call, which keeps this loop flat.
                "title" => {
                    book.title = reader
                        .read_text(e.name())?
                        .xml_content(XmlVersion::Implicit1_0)
                        .into_owned();
                }
                "author" => {
                    book.authors.push(
                        reader
                            .read_text(e.name())?
                            .xml_content(XmlVersion::Implicit1_0)
                            .into_owned(),
                    );
                }
                // Unknown child: skip its subtree so we stay aligned.
                _ => {
                    reader.read_to_end(e.name())?;
                }
            },
            // The book's own end tag: we are done with this subtree.
            Event::End(e) if e.name().as_ref() == "book" => break,
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(book)
}

fn main() -> Result<(), quick_xml::Error> {
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

    let via_state_machine = parse_state_machine(XML)?;
    let via_nested = parse_nested_readers(XML)?;

    println!("state machine: {via_state_machine:#?}");

    // Both patterns produce the same result.
    assert_eq!(via_state_machine, expected);
    assert_eq!(via_nested, expected);

    Ok(())
}

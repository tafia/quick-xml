//! Reading *and* writing XML with serde.
//!
//! When your XML maps cleanly onto Rust types, serde is by far the least code:
//! you describe the shape once with `#[derive(Deserialize, Serialize)]` and get
//! both parsing and generation for free. This is the recommended approach unless
//! you need streaming, exotic control over the output, or want to avoid the
//! serde dependency (see `examples/README.md` for the trade-offs).
//!
//! There are unfortunately a handful of structural issues that can prevent using
//! serde with some document shapes, check the issue tracker for more details.
//!
//! The one thing to learn is how XML concepts map to struct fields:
//!
//! | XML                          | serde field                                  |
//! |------------------------------|----------------------------------------------|
//! | attribute `id="..."`         | `#[serde(rename = "@id")]`                    |
//! | text inside an element       | `#[serde(rename = "$text")]`                  |
//! | child element `<title>`      | a field named `title` (no prefix)             |
//! | repeated child elements      | a `Vec<T>` field                              |
//! | the element's own tag name   | chosen by the parent field / root, see below  |
//!
//! Types are converted for you: `available="true"` becomes a `bool`,
//! `<price>39.95</price>` becomes an `f64`, and so on.
//!
//! Run it with:
//!
//! ```console
//! cargo run --example serde_roundtrip --features serialize
//! ```

use quick_xml::de::from_str;
use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::se::to_string;
use quick_xml::writer::Writer;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Catalog {
    // A `Vec` collects every repeated `<book>` child. serde uses the field name
    // (`book`) as the expected element name.
    book: Vec<Book>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Book {
    // `@` marks an attribute rather than a child element.
    #[serde(rename = "@id")]
    id: String,

    // Attributes are decoded into whatever type you declare. `"true"`/`"false"`
    // parse straight into a `bool`.
    #[serde(rename = "@available")]
    available: bool,

    // Child elements are plain fields. Their type can be another struct...
    title: Title,

    // ...a `Vec` for repeated elements...
    author: Vec<String>,

    // ...or any other `Deserialize`/`Serialize` type.
    price: Price,
}

/// An element that carries both an attribute and text content, e.g.
/// `<title lang="en">The Rust Programming Language</title>`.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Title {
    #[serde(rename = "@lang")]
    lang: String,

    // `$text` captures the text between the tags.
    #[serde(rename = "$text")]
    value: String,
}

/// `<price currency="USD">39.95</price>` — an attribute plus a numeric body.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Price {
    #[serde(rename = "@currency")]
    currency: String,

    #[serde(rename = "$text")]
    amount: f64,
}

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reading: XML -> Rust

    let catalog: Catalog = from_str(XML)?;
    println!("{catalog:#?}");

    assert_eq!(catalog.book.len(), 2);
    assert_eq!(catalog.book[0].id, "b1");
    assert!(catalog.book[0].available);
    assert_eq!(catalog.book[0].title.value, "The Rust Programming Language");
    assert_eq!(catalog.book[0].title.lang, "en");
    assert_eq!(catalog.book[0].author, ["Steve Klabnik", "Carol Nichols"]);
    assert_eq!(catalog.book[0].price.amount, 39.95);
    assert!(!catalog.book[1].available);

    // Writing: Rust -> XML

    // `to_string` serializes back to XML. The root element name is derived from
    // the type name (`Catalog` -> `<Catalog>`); use `to_string_with_root` if you
    // need to control it. The output is compact (no indentation); wrap it in a
    // `Writer` if you need pretty-printing.
    let xml = to_string(&catalog)?;
    println!("\n{xml}");

    // Serialization is the inverse of deserialization: parsing the output back
    // yields the value we started with.
    let reparsed: Catalog = from_str(&xml)?;
    assert_eq!(catalog, reparsed);

    // Writing serde structs into a hand-driven `Writer`

    // `to_string` above produces compact output. When you need pretty-printed
    // output — or want to drop serde-serialized structs into a larger document
    // you are building by hand — drive a `Writer` yourself and hand each value
    // to `write_serializable`. It picks up the writer's indentation, which
    // `to_string` cannot give you. Here we emit the `<catalog>` wrapper
    // manually and serialize each `Book` into it.
    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 4);
    writer.write_event(Event::Start(BytesStart::new("catalog")))?;
    for book in &catalog.book {
        writer.write_serializable("book", book)?;
    }
    writer.write_event(Event::End(BytesEnd::new("catalog")))?;

    let pretty = String::from_utf8(writer.into_inner())?;
    println!("\n{pretty}");

    // Same content as `XML`, minus the declaration — and it round-trips too.
    let reparsed: Catalog = from_str(&pretty)?;
    assert_eq!(catalog, reparsed);

    Ok(())
}

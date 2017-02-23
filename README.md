# quick-xml

[![Build Status](https://travis-ci.org/tafia/quick-xml.svg?branch=master)](https://travis-ci.org/tafia/quick-xml)
[![Crate](http://meritbadge.herokuapp.com/quick-xml)](https://crates.io/crates/quick-xml)

High performance xml pull reader/writer.

[docs.rs](https://docs.rs/quick-xml)

Syntax is inspired by [xml-rs](https://github.com/netvl/xml-rs).

## Usage

```toml
[dependencies]
quick-xml = "0.6.0"
```
``` rust
extern crate quick_xml;
```

## Example

### Reader

```rust
use quick_xml::reader::Reader
use quick_xml::events::Event;

let xml = r#"<tag1 att1 = "test">
                <tag2><!--Test comment-->Test</tag2>
                <tag2>
                    Test 2
                </tag2>
            </tag1>"#;

let mut reader = Reader::from_str(xml);
reader.trim_text(true);

let mut count = 0;
let mut txt = Vec::new();
let mut buf = Vec::new();

// The `Reader` does not implement `Iterator` because it outputs borrowed data (`Cow`s)
loop {
    match reader.read_event(&mut buf) {
    // for triggering namespaced events, use this instead:
    // match reader.read_namespaced_event(&mut buf) {
        Ok(Event::Start(ref e)) => {
        // for namespaced:
        // Ok((ref namespace_value, Event::Start(ref e)))
            match e.name() {
                b"tag1" => println!("attributes values: {:?}",
                                    e.attributes().map(|a| a.unwrap().value).collect::<Vec<_>>()),
                b"tag2" => count += 1,
                _ => (),
            }
        },
        Ok(Event::Text(e)) => txt.push(e.unescape_and_decode(&reader).unwrap()),
        Ok(Event::Eof) => break, // exits the loop when reaching end of file
        Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
        _ => (), // There are several other `Event`s we do not consider here
    }

    // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
    buf.clear();
}
```

### Writer

```rust
use quick_xml::writer::Writer;
use quick_xml::reader::Reader;
use quick_xml::events::{Event, BytesEnd, BytesStart};
use std::io::Cursor;
use std::iter;

let xml = r#"<this_tag k1="v1" k2="v2"><child>text</child></this_tag>"#;
let mut reader = Reader::from_str(xml);
reader.trim_text(true);
let mut writer = Writer::new(Cursor::new(Vec::new()));
let mut buf = Vec::new();
loop {
    match reader.read_event(&mut buf) {
        Ok(Event::Start(ref e)) if e.name() == b"this_tag" => {

            // crates a new element ... alternatively we could reuse `e` by calling
            // `e.into_owned()`
            let mut elem = BytesStart::owned(b"my_elem".to_vec(), "my_elem".len());

            // collect existing attributes
            elem.with_attributes(e.attributes().map(|attr| attr.unwrap()));

            // copy existing attributes, adds a new my-key="some value" attribute
            elem.push_attribute(("my-key", "some value"));

            // writes the event to the writer
            assert!(writer.write_event(Event::Start(elem)).is_ok());
        },
        Ok(Event::End(ref e)) if e.name() == b"this_tag" => {
            assert!(writer.write_event(Event::End(BytesEnd::borrowed(b"my_elem"))).is_ok());
        },
        Ok(Event::Eof) => break,
        Ok(e) => assert!(writer.write_event(e).is_ok()),
        // or using the buffer
        // Ok(e) => assert!(writer.write(&buf).is_ok()),
        Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
    }
    buf.clear();
}

let result = writer.into_inner().into_inner();
let expected = r#"<my_elem k1="v1" k2="v2" my-key="some value"><child>text</child></my_elem>"#;
assert_eq!(result, expected.as_bytes());
```

## Performance

quick-xml is 40+ times faster than the widely used [xml-rs](https://crates.io/crates/xml-rs) crate.

```
// quick-xml benches
test bench_quick_xml            ... bench:     316,915 ns/iter (+/- 59,750)
test bench_quick_xml_escaped    ... bench:     430,226 ns/iter (+/- 19,036)
test bench_quick_xml_namespaced ... bench:     452,997 ns/iter (+/- 30,077)
test bench_quick_xml_wrapper    ... bench:     313,846 ns/iter (+/- 93,794)

// same bench with xml-rs
test bench_xml_rs               ... bench:  15,329,068 ns/iter (+/- 3,966,413)
```

## Contribute

Any PR is welcomed!

## License

MIT

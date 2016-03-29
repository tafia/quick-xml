# quick-xml

[![Build Status](https://travis-ci.org/tafia/quick-xml.svg?branch=master)](https://travis-ci.org/appsignal/quick-xml)
[![Crate](http://meritbadge.herokuapp.com/quick-xml)](https://crates.io/crates/quick-xml)

High performance xml pull reader/writer.

[Documentation](http://tafia.github.io/quick-xml/quick_xml/index.html)

Syntax is inspired by [xml-rs](https://github.com/netvl/xml-rs).

## Usage

Carto.toml
```toml
[dependencies]
quick-xml = "0.1"
```

``` rust
extern crate quick_xml;
```

## Example

### Reader

```rust
use quick_xml::{XmlReader, Event};

let xml = r#"<tag1 att1 = "test">
                <tag2><!--Test comment-->Test</tag2>
                <tag2>
                    Test 2
                </tag2>
            </tag1>"#;
let reader = XmlReader::from_str(xml).trim_text(true);
// if you want to use namespaces, you just need to convert the `XmlReader`
// to an `XmlnsReader`:
// let reader_ns = reader.namespaced();
let mut count = 0;
let mut txt = Vec::new();
for r in reader {
// namespaced: the `for` loop moves the reader
// => use `while let` so you can have access to `reader_ns.resolve` for attributes
// while let Some(r) = reader.next() {
    match r {
        Ok(Event::Start(ref e)) => {
        // for namespaced:
        // Ok((ref namespace_value, Event::Start(ref e)))
            match e.name() {
                b"tag1" => println!("attributes values: {:?}", 
                                 e.attributes().map(|a| a.unwrap().1)
                                 // namespaced: use `reader_ns.resolve`
                                 // e.attributes().map(|a| a.map(|(k, _)| reader_ns.resolve(k))) ...
                                 .collect::<Vec<_>>()),
                b"tag2" => count += 1,
                _ => (),
            }
        },
        Ok(Event::Text(e)) => txt.push(e.into_string()),
        Err((e, pos)) => panic!("{:?} at buffer position {}", e, pos),
        _ => (),
    }
}
```

### Writer

```rust
use quick_xml::{AsStr, Element, Event, XmlReader, XmlWriter};
use quick_xml::Event::*;
use std::io::Cursor;
use std::iter;

let xml = r#"<this_tag k1="v1" k2="v2"><child>text</child></this_tag>"#;
let reader = XmlReader::from_str(xml).trim_text(true);
let mut writer = XmlWriter::new(Cursor::new(Vec::new()));
for r in reader {
    match r {
        Ok(Event::Start(ref e)) if e.name() == b"this_tag" => {
            // collect existing attributes
            let mut attrs = e.attributes().map(|attr| attr.unwrap()).collect::<Vec<_>>();

            // copy existing attributes, adds a new my-key="some value" attribute
            let mut elem = Element::new("my_elem").with_attributes(attrs);
            elem.push_attribute(b"my-key", "some value");

            // writes the event to the writer
            assert!(writer.write(Start(elem)).is_ok());
        },
        Ok(Event::End(ref e)) if e.name() == b"this_tag" => {
            assert!(writer.write(End(Element::new("my_elem"))).is_ok());
        },
        Ok(e) => assert!(writer.write(e).is_ok()),
        Err((e, pos)) => panic!("{:?} at buffer position {}", e, pos),
    }
}

let result = writer.into_inner().into_inner();
let expected = r#"<my_elem k1="v1" k2="v2" my-key="some value"><child>text</child></my_elem>"#;
assert_eq!(result, expected.as_bytes());
```

## Performance

Here is a simple comparison with [xml-rs](https://github.com/netvl/xml-rs) for very basic operations.

```
test bench_quick_xml            ... bench:     692,552 ns/iter (+/- 8,580)
test bench_quick_xml_namespaced ... bench:     975,165 ns/iter (+/- 8,571)
test bench_xml_rs               ... bench:  14,032,716 ns/iter (+/- 830,202
```

## Todo

- [x] [namespaces](https://github.com/tafia/quick-xml/issues/14)
- [ ] non-utf8: most methods return `&u[u8]` => probably not too relevant for the moment
- [x] [parse xml declaration](https://github.com/tafia/quick-xml/pull/10)
- [x] [benchmarks](https://github.com/tafia/quick-xml/issues/13)
- [ ] [escape characters](https://github.com/tafia/quick-xml/issues/12)
- [ ] more checks
- [ ] ... ?

## Contribute

Any PR is welcomed!

## License

MIT

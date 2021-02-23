# quick-xml

[![Build Status](https://travis-ci.org/tafia/quick-xml.svg?branch=master)](https://travis-ci.org/tafia/quick-xml)
[![Crate](http://meritbadge.herokuapp.com/quick-xml)](https://crates.io/crates/quick-xml)

High performance xml pull reader/writer.

The reader:
- is almost zero-copy (use of `Cow` whenever possible)
- is easy on memory allocation (the API provides a way to reuse buffers)
- support various encoding (with `encoding` feature), namespaces resolution, special characters.

[docs.rs](https://docs.rs/quick-xml)

Syntax is inspired by [xml-rs](https://github.com/netvl/xml-rs).

## Example

### Reader

```rust
use quick_xml::Reader;
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
        Ok(Event::Start(ref e)) => {
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
use quick_xml::Writer;
use quick_xml::Reader;
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
            elem.extend_attributes(e.attributes().map(|attr| attr.unwrap()));

            // copy existing attributes, adds a new my-key="some value" attribute
            elem.push_attribute(("my-key", "some value"));

            // writes the event to the writer
            assert!(writer.write_event(Event::Start(elem)).is_ok());
        },
        Ok(Event::End(ref e)) if e.name() == b"this_tag" => {
            assert!(writer.write_event(Event::End(BytesEnd::borrowed(b"my_elem"))).is_ok());
        },
        Ok(Event::Eof) => break,
	// you can use either `e` or `&e` if you don't want to move the event
        Ok(e) => assert!(writer.write_event(&e).is_ok()),
        Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
    }
    buf.clear();
}

let result = writer.into_inner().into_inner();
let expected = r#"<my_elem k1="v1" k2="v2" my-key="some value"><child>text</child></my_elem>"#;
assert_eq!(result, expected.as_bytes());
```

## Serde

When using the `serialize` feature, quick-xml can be used with serde's `Serialize`/`Deserialize` traits.

Here is an example deserializing crates.io source:

```rust
// Cargo.toml
// [dependencies]
// serde = { version = "1.0", features = [ "derive" ] }
// quick-xml = { version = "0.21", features = [ "serialize" ] }
extern crate serde;
extern crate quick_xml;

use serde::Deserialize;
use quick_xml::de::{from_str, DeError};

#[derive(Debug, Deserialize, PartialEq)]
struct Link {
    rel: String,
    href: String,
    sizes: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Lang {
    En,
    Fr,
    De,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Head {
    title: String,
    #[serde(rename = "link", default)]
    links: Vec<Link>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Script {
    src: String,
    integrity: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Body {
    #[serde(rename = "script", default)]
    scripts: Vec<Script>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Html {
    lang: Option<String>,
    head: Head,
    body: Body,
}

fn crates_io() -> Result<Html, DeError> {
    let xml = "<!DOCTYPE html>
        <html lang=\"en\">
          <head>
            <meta charset=\"utf-8\">
            <meta http-equiv=\"X-UA-Compatible\" content=\"IE=edge\">
            <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">

            <title>crates.io: Rust Package Registry</title>


        <!-- EMBER_CLI_FASTBOOT_TITLE --><!-- EMBER_CLI_FASTBOOT_HEAD -->
        <link rel=\"manifest\" href=\"/manifest.webmanifest\">
        <link rel=\"apple-touch-icon\" href=\"/cargo-835dd6a18132048a52ac569f2615b59d.png\" sizes=\"227x227\">

            <link rel=\"stylesheet\" href=\"/assets/vendor-8d023d47762d5431764f589a6012123e.css\" integrity=\"sha256-EoB7fsYkdS7BZba47+C/9D7yxwPZojsE4pO7RIuUXdE= sha512-/SzGQGR0yj5AG6YPehZB3b6MjpnuNCTOGREQTStETobVRrpYPZKneJwcL/14B8ufcvobJGFDvnTKdcDDxbh6/A==\" >
            <link rel=\"stylesheet\" href=\"/assets/cargo-cedb8082b232ce89dd449d869fb54b98.css\" integrity=\"sha256-S9K9jZr6nSyYicYad3JdiTKrvsstXZrvYqmLUX9i3tc= sha512-CDGjy3xeyiqBgUMa+GelihW394pqAARXwsU+HIiOotlnp1sLBVgO6v2ZszL0arwKU8CpvL9wHyLYBIdfX92YbQ==\" >


            <link rel=\"shortcut icon\" href=\"/favicon.ico\" type=\"image/x-icon\">
            <link rel=\"icon\" href=\"/cargo-835dd6a18132048a52ac569f2615b59d.png\" type=\"image/png\">
            <link rel=\"search\" href=\"/opensearch.xml\" type=\"application/opensearchdescription+xml\" title=\"Cargo\">
          </head>
          <body>
            <!-- EMBER_CLI_FASTBOOT_BODY -->
            <noscript>
                <div id=\"main\">
                    <div class='noscript'>
                        This site requires JavaScript to be enabled.
                    </div>
                </div>
            </noscript>

            <script src=\"/assets/vendor-bfe89101b20262535de5a5ccdc276965.js\" integrity=\"sha256-U12Xuwhz1bhJXWyFW/hRr+Wa8B6FFDheTowik5VLkbw= sha512-J/cUUuUN55TrdG8P6Zk3/slI0nTgzYb8pOQlrXfaLgzr9aEumr9D1EzmFyLy1nrhaDGpRN1T8EQrU21Jl81pJQ==\" ></script>
            <script src=\"/assets/cargo-4023b68501b7b3e17b2bb31f50f5eeea.js\" integrity=\"sha256-9atimKc1KC6HMJF/B07lP3Cjtgr2tmET8Vau0Re5mVI= sha512-XJyBDQU4wtA1aPyPXaFzTE5Wh/mYJwkKHqZ/Fn4p/ezgdKzSCFu6FYn81raBCnCBNsihfhrkb88uF6H5VraHMA==\" ></script>

          </body>
        </html>
}";
    let html: Html = from_str(xml)?;
    assert_eq!(&html.head.title, "crates.io: Rust Package Registry");
    Ok(html)
}
```

### Credits

This has largely been inspired by [serde-xml-rs](https://github.com/RReverser/serde-xml-rs). 
quick-xml follows its convention for deserialization, including the 
[`$value`](https://github.com/RReverser/serde-xml-rs#parsing-the-value-of-a-tag) special name.

### Parsing the "value" of a tag

If you have an input of the form `<foo abc="xyz">bar</foo>`, and you want to get at the `bar`, you can use the special name `$value`:

```rust,ignore
struct Foo {
    pub abc: String,
    #[serde(rename = "$value")]
    pub body: String,
}
```

### Performance

Note that despite not focusing on performance (there are several unecessary copies), it remains about 10x faster than serde-xml-rs.

# Features

- `encoding`: support non utf8 xmls
- `serialize`: support serde `Serialize`/`Deserialize`

## Performance

Benchmarking is hard and the results depend on your input file and your machine.

Here on my particular file, quick-xml is around **50 times faster** than [xml-rs](https://crates.io/crates/xml-rs) crate.

```
// quick-xml benches
test bench_quick_xml            ... bench:     198,866 ns/iter (+/- 9,663)
test bench_quick_xml_escaped    ... bench:     282,740 ns/iter (+/- 61,625)
test bench_quick_xml_namespaced ... bench:     389,977 ns/iter (+/- 32,045)

// same bench with xml-rs
test bench_xml_rs               ... bench:  14,468,930 ns/iter (+/- 321,171)

// serde-xml-rs vs serialize feature
test bench_serde_quick_xml      ... bench:   1,181,198 ns/iter (+/- 138,290)
test bench_serde_xml_rs         ... bench:  15,039,564 ns/iter (+/- 783,485)
```

For a feature and performance comparison, you can also have a look at RazrFalcon's [parser comparison table](https://github.com/RazrFalcon/roxmltree#parsing).

## Contribute

Any PR is welcomed!

## License

MIT

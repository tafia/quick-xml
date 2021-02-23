extern crate quick_xml;

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, Result};
use std::str::from_utf8;

#[test]
fn sample_1_short() {
    test(
        include_bytes!("documents/sample_1.xml"),
        include_bytes!("documents/sample_1_short.txt"),
        true,
    );
}

#[test]
fn sample_1_full() {
    test(
        include_bytes!("documents/sample_1.xml"),
        include_bytes!("documents/sample_1_full.txt"),
        false,
    );
}

#[test]
fn sample_2_short() {
    test(
        include_bytes!("documents/sample_2.xml"),
        include_bytes!("documents/sample_2_short.txt"),
        true,
    );
}

#[test]
fn sample_2_full() {
    test(
        include_bytes!("documents/sample_2.xml"),
        include_bytes!("documents/sample_2_full.txt"),
        false,
    );
}

#[cfg(all(not(windows), feature = "escape-html"))]
#[test]
fn html5() {
    test(
        include_bytes!("documents/html5.html"),
        include_bytes!("documents/html5.txt"),
        false,
    );
}

#[cfg(all(windows, feature = "escape-html"))]
#[test]
fn html5() {
    test(
        include_bytes!("documents/html5.html"),
        include_bytes!("documents/html5-windows.txt"),
        false,
    );
}

// #[test]
// fn sample_3_short() {
//     test(
//         include_bytes!("documents/sample_3.xml"),
//         include_bytes!("documents/sample_3_short.txt"),
//         true
//     );
// }

// #[test]
// fn sample_3_full() {
//     test(
//         include_bytes!("documents/sample_3.xml"),
//         include_bytes!("documents/sample_3_full.txt"),
//         false
//     );
// }

// #[test]
// fn sample_4_short() {
//     test(
//         include_bytes!("documents/sample_4.xml"),
//         include_bytes!("documents/sample_4_short.txt"),
//         true
//     );
// }

// #[test]
// fn sample_4_full() {
//     test(
//         include_bytes!("documents/sample_4.xml"),
//         include_bytes!("documents/sample_4_full.txt"),
//         false
//     );
//
// }

#[test]
fn sample_ns_short() {
    test(
        include_bytes!("documents/sample_ns.xml"),
        include_bytes!("documents/sample_ns_short.txt"),
        true,
    );
}

#[test]
fn eof_1() {
    test(
        br#"<?xml"#,
        br#"Error: Unexpected EOF during reading XmlDecl."#,
        true,
    );
}

#[test]
fn bad_1() {
    test(
        br#"<?xml&.,"#,
        br#"1:6 Error: Unexpected EOF during reading XmlDecl."#,
        true,
    );
}

#[test]
fn dashes_in_comments() {
    test(
        br#"<!-- comment -- --><hello/>"#,
        br#"
        |Error: Unexpected token '--'
        "#,
        true,
    );

    test(
        br#"<!-- comment ---><hello/>"#,
        br#"
        |Error: Unexpected token '--'
        "#,
        true,
    );
}

#[test]
fn tabs_1() {
    test(
        b"\t<a>\t<b/></a>",
        br#"
            StartElement(a)
            EmptyElement(b)
            EndElement(a)
            EndDocument
        "#,
        true,
    );
}

#[test]
fn issue_83_duplicate_attributes() {
    // Error when parsing attributes won't stop main event reader
    // as it is a lazy operation => add ending events
    test(
        br#"<hello><some-tag a='10' a="20"/></hello>"#,
        b"
            |StartElement(hello)
            |1:30 EmptyElement(some-tag, attr-error: error while parsing \
                  attribute at position 16: Duplicate attribute at position 9 and 16)
            |EndElement(hello)
            |EndDocument
        ",
        true,
    );
}

#[test]
fn issue_93_large_characters_in_entity_references() {
    test(
        r#"<hello>&𤶼;</hello>"#.as_bytes(),
        r#"
            |StartElement(hello)
            |1:10 FailedUnescape([38, 240, 164, 182, 188, 59]; Error while escaping character at range 1..5: Unrecognized escape symbol: Ok("𤶼"))
            |EndElement(hello)
            |EndDocument
        "#
        .as_bytes(),
        true,
    )
}

#[test]
fn issue_98_cdata_ending_with_right_bracket() {
    test(
        br#"<hello><![CDATA[Foo [Bar]]]></hello>"#,
        br#"
            |StartElement(hello)
            |Characters()
            |CData(Foo [Bar])
            |Characters()
            |EndElement(hello)
            |EndDocument
        "#,
        false,
    )
}

#[test]
fn issue_105_unexpected_double_dash() {
    test(
        br#"<hello>-- </hello>"#,
        br#"
            |StartElement(hello)
            |Characters(-- )
            |EndElement(hello)
            |EndDocument
        "#,
        false,
    );

    test(
        br#"<hello>--</hello>"#,
        br#"
            |StartElement(hello)
            |Characters(--)
            |EndElement(hello)
            |EndDocument
        "#,
        false,
    );

    test(
        br#"<hello>--></hello>"#,
        br#"
            |StartElement(hello)
            |Characters(-->)
            |EndElement(hello)
            |EndDocument
        "#,
        false,
    );

    test(
        br#"<hello><![CDATA[--]]></hello>"#,
        br#"
            |StartElement(hello)
            |Characters()
            |CData(--)
            |Characters()
            |EndElement(hello)
            |EndDocument
        "#,
        false,
    );
}

#[test]
fn issue_attributes_have_no_default_namespace() {
    // At the moment, the 'test' method doesn't render namespaces for attribute names.
    // This test only checks whether the default namespace got applied to the EmptyElement.
    test(
        br#"<hello xmlns="urn:foo" x="y"/>"#,
        br#"
             |EmptyElement({urn:foo}hello [x="y"])
             |EndDocument
         "#,
        true,
    );
}

#[test]
fn issue_default_namespace_on_outermost_element() {
    // Regression test
    test(
        br#"<hello xmlns="urn:foo"/>"#,
        br#"
                |EmptyElement({urn:foo}hello)
                |EndDocument
            "#,
        true,
    );
}

#[test]
fn default_namespace_applies_to_end_elem() {
    test(
        br#"<hello xmlns="urn:foo" x="y">
              <inner/>
            </hello>"#,
        br#"
            |StartElement({urn:foo}hello [x="y"])
            |EmptyElement({urn:foo}inner)
            |EndElement({urn:foo}hello)
            |EndDocument
        "#,
        true,
    );
}

fn test(input: &[u8], output: &[u8], is_short: bool) {
    let mut reader = Reader::from_reader(input);
    reader
        .trim_text(is_short)
        .check_comments(true)
        .expand_empty_elements(false);

    let mut spec_lines = SpecIter(output).enumerate();
    let mut buf = Vec::new();
    let mut ns_buffer = Vec::new();

    if !is_short {
        // discard first whitespace
        reader.read_event(&mut buf).unwrap();
    }

    loop {
        buf.clear();
        let event = reader.read_namespaced_event(&mut buf, &mut ns_buffer);
        let line = xmlrs_display(&event);
        if let Some((n, spec)) = spec_lines.next() {
            if spec.trim() == "EndDocument" {
                break;
            }
            if line.trim() != spec.trim() {
                panic!(
                    "\n-------------------\n\
                     Unexpected event at line {}:\n\
                     Expected: {}\nFound: {}\n\
                     -------------------\n",
                    n + 1,
                    spec,
                    line
                );
            }
        } else {
            if line == "EndDocument" {
                break;
            }
            panic!("Unexpected event: {}", line);
        }

        if !is_short && line.starts_with("StartDocument") {
            // advance next Characters(empty space) ...
            if let Ok(Event::Text(ref e)) = reader.read_event(&mut Vec::new()) {
                if e.iter().any(|b| match *b {
                    b' ' | b'\r' | b'\n' | b'\t' => false,
                    _ => true,
                }) {
                    panic!("Reader expects empty Text event after a StartDocument");
                }
            } else {
                panic!("Reader expects empty Text event after a StartDocument");
            }
        }
    }
}

fn namespace_name(n: &Option<&[u8]>, name: &[u8]) -> String {
    match *n {
        Some(n) => format!("{{{}}}{}", from_utf8(n).unwrap(), from_utf8(name).unwrap()),
        None => from_utf8(name).unwrap().to_owned(),
    }
}

fn make_attrs(e: &BytesStart) -> ::std::result::Result<String, String> {
    let mut atts = Vec::new();
    for a in e.attributes() {
        match a {
            Ok(a) => {
                if a.key.len() < 5 || !a.key.starts_with(b"xmlns") {
                    atts.push(format!(
                        "{}=\"{}\"",
                        from_utf8(a.key).unwrap(),
                        from_utf8(&*a.unescaped_value().unwrap()).unwrap()
                    ));
                }
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(atts.join(", "))
}

fn xmlrs_display(opt_event: &Result<(Option<&[u8]>, Event)>) -> String {
    match opt_event {
        Ok((ref n, Event::Start(ref e))) => {
            let name = namespace_name(n, e.name());
            match make_attrs(e) {
                Ok(ref attrs) if attrs.is_empty() => format!("StartElement({})", &name),
                Ok(ref attrs) => format!("StartElement({} [{}])", &name, &attrs),
                Err(e) => format!("StartElement({}, attr-error: {})", &name, &e),
            }
        }
        Ok((ref n, Event::Empty(ref e))) => {
            let name = namespace_name(n, e.name());
            match make_attrs(e) {
                Ok(ref attrs) if attrs.is_empty() => format!("EmptyElement({})", &name),
                Ok(ref attrs) => format!("EmptyElement({} [{}])", &name, &attrs),
                Err(e) => format!("EmptyElement({}, attr-error: {})", &name, &e),
            }
        }
        Ok((ref n, Event::End(ref e))) => format!("EndElement({})", namespace_name(n, e.name())),
        Ok((_, Event::Comment(ref e))) => format!("Comment({})", from_utf8(e).unwrap()),
        Ok((_, Event::CData(ref e))) => format!("CData({})", from_utf8(e).unwrap()),
        Ok((_, Event::Text(ref e))) => match e.unescaped() {
            Ok(c) => match from_utf8(&*c) {
                Ok(c) => format!("Characters({})", c),
                Err(ref err) => format!("InvalidUtf8({:?}; {})", e.escaped(), err),
            },
            Err(ref err) => format!("FailedUnescape({:?}; {})", e.escaped(), err),
        },
        Ok((_, Event::Decl(ref e))) => {
            let version_cow = e.version().unwrap();
            let version = from_utf8(version_cow.as_ref()).unwrap();
            let encoding_cow = e.encoding().unwrap().unwrap();
            let encoding = from_utf8(encoding_cow.as_ref()).unwrap();
            format!("StartDocument({}, {})", version, encoding)
        }
        Ok((_, Event::Eof)) => format!("EndDocument"),
        Ok((_, Event::PI(ref e))) => format!("ProcessingInstruction(PI={})", from_utf8(e).unwrap()),
        Err(ref e) => format!("Error: {}", e),
        Ok((_, Event::DocType(ref e))) => format!("DocType({})", from_utf8(e).unwrap()),
    }
}

struct SpecIter<'a>(&'a [u8]);

impl<'a> Iterator for SpecIter<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        let start = self
            .0
            .iter()
            .position(|b| !matches!(*b, b' ' | b'\r' | b'\n' | b'\t' | b'|' | b':' | b'0'..=b'9'))
            .unwrap_or(0);

        if let Some(p) = self.0.windows(3).position(|w| w == b")\r\n") {
            let (prev, next) = self.0.split_at(p + 1);
            self.0 = &next[1..];
            Some(from_utf8(&prev[start..]).expect("Error decoding to uft8"))
        } else if let Some(p) = self.0.windows(2).position(|w| w == b")\n") {
            let (prev, next) = self.0.split_at(p + 1);
            self.0 = next;
            Some(from_utf8(&prev[start..]).expect("Error decoding to uft8"))
        } else if self.0.is_empty() {
            None
        } else {
            let p = self.0;
            self.0 = &[];
            Some(from_utf8(&p[start..]).unwrap())
        }
    }
}

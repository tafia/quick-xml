extern crate quick_xml;

use std::io::BufRead;
use std::str::from_utf8;

use quick_xml::errors::Result;
use quick_xml::reader::Reader;
use quick_xml::events::{BytesStart, Event};

use std::fmt;

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
    test(br#"<?xml"#, br#"Error: XmlDecl"#, true);
}

#[test]
fn bad_1() {
    test(br#"<?xml&.,"#, br#"1:6 Error: XmlDecl"#, true);
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
        ",
        true,
    );
}

#[test]
fn issue_93_large_characters_in_entity_references() {
    test(r#"<hello>&𤶼;</hello>"#.as_bytes(),
         r#"
            |StartElement(hello)
            |1:10 Error while escaping character at range 1..5: Unrecognized escape symbol: Ok("𤶼")
            |EndElement(hello)
        "#.as_bytes(),
         true)
}

#[test]
fn issue_98_cdata_ending_with_right_bracket() {
    test(
        br#"<hello><![CDATA[Foo [Bar]]]></hello>"#,
        br#"
            |StartElement(hello)
            |Characters()
            |CData("Foo [Bar]")
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
            |Characters("-- ")
            |EndElement(hello)
            |EndDocument
        "#,
        false,
    );

    test(
        br#"<hello>--</hello>"#,
        br#"
            |StartElement(hello)
            |Characters("--")
            |EndElement(hello)
            |EndDocument
        "#,
        false,
    );

    test(
        br#"<hello>--></hello>"#,
        br#"
            |StartElement(hello)
            |Characters("-->")
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
            |CData("--")
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

// clones a lot but that's fine
fn convert_to_quick_xml(s: &str) -> String {
    let mut s = match s.trim() {
        ts if ts.starts_with('|') => &ts[1..],
        s => s,
    };

    if !s.is_empty() && s.as_bytes()[0] >= b'0' && s.as_bytes()[0] <= b'9' {
        let p = s.chars().position(|c| c == ' ').unwrap();
        s = &s[(p + 1)..];
    }

    if s.starts_with("Whitespace") {
        format!("Characters{}", &s[10..])
    } else {
        s.to_string()
    }
}

fn test(input: &[u8], output: &[u8], is_short: bool) {
    let mut reader = Reader::from_reader(input);
    reader
        .trim_text(is_short)
        .check_comments(true)
        .expand_empty_elements(false);

    let mut spec_lines = output
        .lines()
        .map(|line| convert_to_quick_xml(&line.unwrap()))
        .filter(|line| !line.trim().is_empty())
        .enumerate();

    let mut buf = Vec::new();
    let mut ns_buffer = Vec::new();

    if !is_short {
        reader.read_event(&mut buf).unwrap();
    }

    loop {
        {
            let line = {
                let e = reader.read_namespaced_event(&mut buf, &mut ns_buffer);
                format!("{}", OptEvent(e))
            };
            if let Some((n, spec)) = spec_lines.next() {
                if spec == "EndDocument" {
                    break;
                }
                if line.trim() != spec.trim() {
                    const SPLITTER: &'static str = "-------------------";
                    panic!(
                        "\n{}\nUnexpected event at line {}:\nExpected: {}\nFound: {}\n{}\n",
                        SPLITTER,
                        n + 1,
                        spec,
                        line,
                        SPLITTER
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
                if let Ok(Event::Text(ref e)) = reader.read_event(&mut buf) {
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
            Ok(a) => if a.key.len() < 5 || !a.key.starts_with(b"xmlns") {
                atts.push(format!(
                    "{}={:?}",
                    from_utf8(a.key).unwrap(),
                    from_utf8(&*a.unescaped_value().unwrap()).unwrap()
                ));
            },
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(atts.join(", "))
}

struct OptEvent<'a, 'b>(Result<(Option<&'a [u8]>, Event<'b>)>);

impl<'a, 'b> fmt::Display for OptEvent<'a, 'b> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Ok((ref n, Event::Start(ref e))) => {
                let name = namespace_name(n, e.name());
                match make_attrs(e) {
                    Ok(ref attrs) if attrs.is_empty() => write!(f, "StartElement({})", &name),
                    Ok(ref attrs) => write!(f, "StartElement({} [{}])", &name, &attrs),
                    Err(e) => write!(f, "StartElement({}, attr-error: {})", &name, &e),
                }
            }
            Ok((ref n, Event::Empty(ref e))) => {
                let name = namespace_name(n, e.name());
                match make_attrs(e) {
                    Ok(ref attrs) if attrs.is_empty() => write!(f, "EmptyElement({})", &name),
                    Ok(ref attrs) => write!(f, "EmptyElement({} [{}])", &name, &attrs),
                    Err(e) => write!(f, "EmptyElement({}, attr-error: {})", &name, &e),
                }
            }
            Ok((ref n, Event::End(ref e))) => {
                write!(f, "EndElement({})", namespace_name(n, e.name()))
            }
            Ok((_, Event::Comment(ref e))) => write!(f, "Comment({:?})", from_utf8(e).unwrap()),
            Ok((_, Event::CData(ref e))) => write!(f, "CData({:?})", from_utf8(e).unwrap()),
            Ok((_, Event::Text(ref e))) => match e.unescaped() {
                Ok(c) => if c.is_empty() {
                    write!(f, "Characters()")
                } else {
                    write!(f, "Characters({:?})", from_utf8(&*c).unwrap())
                },
                Err(ref e) => write!(f, "{}", e),
            },
            Ok((_, Event::Decl(ref e))) => {
                let version_cow = e.version().unwrap();
                let version = from_utf8(version_cow.as_ref()).unwrap();
                let encoding_cow = e.encoding().unwrap().unwrap();
                let encoding = from_utf8(encoding_cow.as_ref()).unwrap();
                write!(f, "StartDocument({}, {})", version, encoding)
            }
            Ok((_, Event::Eof)) => write!(f, "EndDocument"),
            Ok((_, Event::PI(ref e))) => {
                write!(f, "ProcessingInstruction(PI={:?})", from_utf8(e).unwrap())
            }
            Err(ref e) => write!(f, "Error: {}", e.iter().last().unwrap()),
            Ok((_, Event::DocType(ref e))) => write!(f, "DocType({})", from_utf8(e).unwrap()),
        }
    }
}

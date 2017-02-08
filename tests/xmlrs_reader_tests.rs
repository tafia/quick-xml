extern crate quick_xml;

use std::io::{BufRead};

use quick_xml::{XmlReader, Event, AsStr, Element};
use quick_xml::error::ResultPos;
use std::fmt;

#[test]
fn sample_1_short() {
    test(
        include_bytes!("documents/sample_1.xml"),
        include_bytes!("documents/sample_1_short.txt"),
        true
    );
}

#[test]
fn sample_1_full() {
    test(
        include_bytes!("documents/sample_1.xml"),
        include_bytes!("documents/sample_1_full.txt"),
        false
    );
}

#[test]
fn sample_2_short() {
    test(
        include_bytes!("documents/sample_2.xml"),
        include_bytes!("documents/sample_2_short.txt"),
        true
    );
}

#[test]
fn sample_2_full() {
    test(
        include_bytes!("documents/sample_2.xml"),
        include_bytes!("documents/sample_2_full.txt"),
        false
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
//

#[test]
fn sample_ns_short() {
    test(
        include_bytes!("documents/sample_ns.xml"),
        include_bytes!("documents/sample_ns_short.txt"),
        true
    );
}

#[test]
fn eof_1() {
    test(
        br#"<?xml"#,
        br#"Malformed xml: Unescaped XmlDecl event"#,
        true
    );
}

#[test]
fn bad_1() {
    test(
        br#"<?xml&.,"#,
        br#"1:6 Malformed xml: Unescaped XmlDecl event"#,
        true
    );
}

#[test]
fn dashes_in_comments() {
    test(
        br#"<!-- comment -- --><hello/>"#,
        br#"
            |1:14 Malformed xml: Unexpected token '--'
        "#,
        false
    );

    test(
        br#"<!-- comment ---><hello/>"#,
        br#"
            |1:14 Malformed xml: Unexpected token '--'
        "#,
        true
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
        true
    );
}

#[test]
fn issue_83_duplicate_attributes() {
    // Error when parsing attributes won't stop main event reader
    // as it is a lazy operation => add ending events
    test(
        br#"<hello><some-tag a='10' a="20"/></hello>"#,
        br#"
            |StartElement(hello)
            |1:30 EmptyElement(some-tag, attr-error: Malformed xml: Duplicate attribute at position 9 and 16)
            |EndElement(hello)
        "#,
        true
    );
}

#[test]
fn issue_93_large_characters_in_entity_references() {
    test(
        r#"<hello>&𤶼;</hello>"#.as_bytes(),
        r#"
            |StartElement(hello)
            |1:10 Malformed xml: Unexpected entity: 𤶼
            |EndElement(hello)
        "#.as_bytes(),
        true
    )
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
        false
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
        false
    );

    test(
        br#"<hello>--</hello>"#,
        br#"
            |StartElement(hello)
            |Characters("--")
            |EndElement(hello)
            |EndDocument
        "#,
        false
    );

    test(
        br#"<hello>--></hello>"#,
        br#"
            |StartElement(hello)
            |Characters("-->")
            |EndElement(hello)
            |EndDocument
        "#,
        false
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
        false
    );
}

 #[test]
 fn issue_attributes_have_no_default_namespace () {
     // At the moment, the 'test' method doesn't render namespaces for attribute names.
     // This test only checks whether the default namespace got applied to the EmptyElement.
     test(
         br#"<hello xmlns="urn:foo" x="y"/>"#,
         br#"
             |EmptyElement({urn:foo}hello [x="y"])
             |EndDocument
         "#,
         true
     );
 }

    #[test]
    fn issue_default_namespace_on_outermost_element () {
        // Regression test
        test(
            br#"<hello xmlns="urn:foo"/>"#,
            br#"
                |EmptyElement({urn:foo}hello)
                |EndDocument
            "#,
            true
        );
    }

 #[test]
 fn default_namespace_applies_to_end_elem () {
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
        true
    );
}

// clones a lot but that's fine
fn convert_to_quick_xml(s: &str) -> String {
    
    let mut s = match s.trim() {
        ts if ts.starts_with('|') => &ts[1..],
        s => s
    };

    if !s.is_empty() && s.as_bytes()[0] >= b'0' && s.as_bytes()[0] <= b'9' {
        let p = s.chars().position(|c| c == ' ').unwrap();
        s = &s[(p + 1) ..];
    }

    if s.starts_with("Whitespace") {
        format!("Characters{}", &s[10..])
    } else {
        format!("{}", s)
    }
}

fn test(input: &[u8], output: &[u8], is_short: bool) {

    let mut reader = XmlReader::from_reader(input)
        .trim_text(is_short)
        .check_comments(true)
        .expand_empty_elements(false)
        .namespaced();

    let mut spec_lines = output.lines()
        .map(|line| convert_to_quick_xml(&line.unwrap()))
        .filter(|line| !line.trim().is_empty())
        .enumerate();

    if !is_short {
        reader.next();
    }

    loop {
        let e = reader.next();
        use std::ops::Deref;
        let e = match e {
             Some(Ok((Some(a),b))) => Some(Ok((Some(a.deref().clone()),b))),
             Some(Err(e)) => Some(Err(e)),
             Some(Ok((None, b))) => Some(Ok((None, b))),
             None => None
        };
        
        let line = format!("{}", OptEvent(e));

        if let Some((n, spec)) = spec_lines.next() {
            if spec == "EndDocument" {
                break;
            }
            if line != spec {
                const SPLITTER: &'static str = "-------------------";
                panic!("\n{}\nUnexpected event at line {}:\nExpected: {}\nFound: {}\n{}\n",
                       SPLITTER, n + 1, spec, line, SPLITTER);
            }
        } else {
            if line == "EndDocument" {
                break;
            }
            panic!("Unexpected event: {}", line);
        }

        if !is_short && line.starts_with("StartDocument") {
            // advance next Characters(empty space) ...
            if let Some(Ok((_, Event::Text(ref e)))) = reader.next() {
                if e.content().iter().any(|b| match *b {
                    b' ' | b'\r' | b'\n' | b'\t' => false,
                    _ => true,
                }) {
                    panic!("XmlReader expects empty Text event after a StartDocument");
                }
            } else {
                panic!("XmlReader expects empty Text event after a StartDocument");
            }
        }
    }
}

fn namespace_name(n: &Option<Vec<u8>>, e: &Element) -> String {
    match n {
        &Some(ref n) => 
            format!("{{{}}}{}", n.as_str().unwrap(), e.name().as_str().unwrap()),
        &None => e.name().as_str().unwrap().to_owned(),
    }
}

fn make_attrs(e: &Element) -> Result<String, String> {
    let mut atts = Vec::new();
    for a in e.attributes().unescaped() {
        match a {
            Ok((k, v)) => if k.len() < 5 || &k[..5] != b"xmlns" {
                atts.push(format!("{}={:?}", k.as_str().unwrap(), v.as_str().unwrap()));
            },
            Err((e, _)) => return Err(e.to_string()),
        }
    }
    Ok(atts.join(", "))
}

struct OptEvent(Option<ResultPos<(Option<Vec<u8>>, Event)>>);

impl fmt::Display for OptEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Some(Ok((ref n, Event::Start(ref e)))) => {
                let name = namespace_name(n, e);
                match make_attrs(e) {
                    Ok(ref attrs) if attrs.is_empty() => write!(f, "StartElement({})", &name),
                    Ok(ref attrs) => write!(f, "StartElement({} [{}])", &name, &attrs),
                    Err(e) => write!(f, "StartElement({}, attr-error: {})", &name, &e),
                }
            },
            Some(Ok((ref n, Event::Empty(ref e)))) => {
                let name = namespace_name(n, e);
                match make_attrs(e) {
                    Ok(ref attrs) if attrs.is_empty() => write!(f, "EmptyElement({})", &name),
                    Ok(ref attrs) => write!(f, "EmptyElement({} [{}])", &name, &attrs),
                    Err(e) => write!(f, "EmptyElement({}, attr-error: {})", &name, &e),
                }
            }
            Some(Ok((ref n, Event::End(ref e)))) =>
                write!(f, "EndElement({})", namespace_name(n, e)),
            Some(Ok((_, Event::Comment(ref e)))) =>
                write!(f, "Comment({:?})", e.content().as_str().unwrap()),
            Some(Ok((_, Event::CData(ref e)))) =>
                write!(f, "CData({:?})", e.content().as_str().unwrap()),
            Some(Ok((_, Event::Text(ref e)))) => {
                match e.unescaped_content() {
                    Ok(c) => {
                        if c.is_empty() {
                            write!(f, "Characters()")
                        } else {
                            write!(f, "Characters({:?})", c.as_str().unwrap())
                        }
                    },
                    Err((ref e, _)) => write!(f, "{}", e),
                }
            },
            Some(Ok((_, Event::Decl(ref e)))) => {
                let version = e.version().unwrap().as_str().unwrap();
                let encoding = e.encoding().unwrap().unwrap().as_str().unwrap();
                write!(f, "StartDocument({}, {})", version, encoding)
            },
            None => write!(f, "EndDocument"),
            Some(Ok((_, Event::PI(ref e)))) =>
                write!(f, "ProcessingInstruction({}={:?})", 
                    e.name().as_str().unwrap(), e.content().as_str().unwrap()),
            Some(Err((ref e, _))) => write!(f, "{}", e),
            Some(Ok((_, Event::DocType(ref e)))) => 
                write!(f, "DocType({})", e.content().as_str().unwrap()),
        }
    }
}

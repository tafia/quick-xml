extern crate quick_xml;

use std::io::{BufRead};

use quick_xml::{XmlReader, Event, AsStr};
use quick_xml::error::ResultPos;

#[test]
fn sample_1_short() {
    test(
        include_bytes!("documents/sample_1.xml"),
        include_bytes!("documents/sample_1_short.txt"),
        true
    );
}

// #[test]
// fn sample_1_full() {
//     test(
//         include_bytes!("documents/sample_1.xml"),
//         include_bytes!("documents/sample_1_full.txt"),
//         false
//     );
// }
// 
// #[test]
// fn sample_2_short() {
//     test(
//         include_bytes!("documents/sample_2.xml"),
//         include_bytes!("documents/sample_2_short.txt"),
//         true
//     );
// }
// 
// #[test]
// fn sample_2_full() {
//     test(
//         include_bytes!("documents/sample_2.xml"),
//         include_bytes!("documents/sample_2_full.txt"),
//         false
//     );
// }
// 
// #[test]
// fn sample_3_short() {
//     test(
//         include_bytes!("documents/sample_3.xml"),
//         include_bytes!("documents/sample_3_short.txt"),
//         true
//     );
// }
// 
// #[test]
// fn sample_3_full() {
//     test(
//         include_bytes!("documents/sample_3.xml"),
//         include_bytes!("documents/sample_3_full.txt"),
//         false
//     );
// }
// 
// #[test]
// fn sample_4_short() {
//     test(
//         include_bytes!("documents/sample_4.xml"),
//         include_bytes!("documents/sample_4_short.txt"),
//         true
//     );
// }
// 
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
// #[test]
// fn eof_1() {
//     test(
//         br#"<?xml"#,
//         br#"1:6 Unexpected end of stream: no root element found"#,
//         true
//     );
// }
// 
// #[test]
// fn bad_1() {
//     test(
//         br#"<?xml&.,"#,
//         br#"1:6 Unexpected token: <?xml&"#,
//         true
//     );
// }
// 
// #[test]
// fn dashes_in_comments() {
//     test(
//         br#"<!-- comment -- --><hello/>"#,
//         br#"
//             |1:14 Unexpected token '--' before ' '
//         "#,
//         true
//     );
// 
//     test(
//         br#"<!-- comment ---><hello/>"#,
//         br#"
//             |1:14 Unexpected token '--' before '-'
//         "#,
//         true
//     );
// }
// 
// #[test]
// fn tabs_1() {
//     test(
//         b"\t<a>\t<b/></a>",
//         br#"
//             StartDocument(1.0, UTF-8)
//             StartElement(a)
//             StartElement(b)
//             EndElement(b)
//             EndElement(a)
//             EndDocument
//         "#,
//         true
//     );
// }
// 
// #[test]
// fn issue_83_duplicate_attributes() {
//     test(
//         br#"<hello><some-tag a='10' a="20"></hello>"#,
//         br#"
//             |StartDocument(1.0, UTF-8)
//             |StartElement(hello)
//             |1:30 Attribute 'a' is redefined
//         "#,
//         true
//     );
// }
// 
// #[test]
// fn issue_93_large_characters_in_entity_references() {
//     test(
//         r#"<hello>&𤶼;</hello>"#.as_bytes(),
//         r#"
//             |StartDocument(1.0, UTF-8)
//             |StartElement(hello)
//             |1:10 Unexpected entity: 𤶼
//         "#.as_bytes(),  // FIXME: it shouldn't be 10, looks like indices are off slightly
//         true
//     )
// }
// 
// #[test]
// fn issue_98_cdata_ending_with_right_bracket() {
//     test(
//         br#"<hello><![CDATA[Foo [Bar]]]></hello>"#,
//         br#"
//             |StartDocument(1.0, UTF-8)
//             |StartElement(hello)
//             |CData("Foo [Bar]")
//             |EndElement(hello)
//             |EndDocument
//         "#,
//         true
//     )
// }
// 
// #[test]
// fn issue_105_unexpected_double_dash() {
//     test(
//         br#"<hello>-- </hello>"#,
//         br#"
//             |StartDocument(1.0, UTF-8)
//             |StartElement(hello)
//             |Characters("-- ")
//             |EndElement(hello)
//             |EndDocument
//         "#,
//         true
//     );
// 
//     test(
//         br#"<hello>--</hello>"#,
//         br#"
//             |StartDocument(1.0, UTF-8)
//             |StartElement(hello)
//             |Characters("--")
//             |EndElement(hello)
//             |EndDocument
//         "#,
//         true
//     );
// 
//     test(
//         br#"<hello>--></hello>"#,
//         br#"
//             |StartDocument(1.0, UTF-8)
//             |StartElement(hello)
//             |Characters("-->")
//             |EndElement(hello)
//             |EndDocument
//         "#,
//         true
//     );
// 
//     test(
//         br#"<hello><![CDATA[--]]></hello>"#,
//         br#"
//             |StartDocument(1.0, UTF-8)
//             |StartElement(hello)
//             |CData("--")
//             |EndElement(hello)
//             |EndDocument
//         "#,
//         true
//     );
// }
// 
// #[test]
// fn issue_attribues_have_no_default_namespace () {
//     test(
//         br#"<hello xmlns="urn:foo" x="y"/>"#,
//         br#"
//             |StartDocument(1.0, UTF-8)
//             |StartElement({urn:foo}hello [x="y"])
//             |EndElement({urn:foo}hello)
//             |EndDocument
//         "#,
//         true
//     );
// }

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
        format!("Character{}", s)
    } else {
        s.to_owned()
    }
}

fn test(input: &[u8], output: &[u8], is_short: bool) {

    let mut reader = XmlReader::from_reader(input).trim_text(is_short);

    let mut spec_lines = output.lines()
        .map(|line| line.unwrap())
        .enumerate()
        .map(|(i, line)| (i, convert_to_quick_xml(&line)))
        .filter(|&(_, ref line)| !line.trim().is_empty());

    loop {
        let e = reader.next();
        let line = quick_xml_to_xmlrs(&e, is_short);

        if let Some(line) = line {
            if let Some((n, spec)) = spec_lines.next() {
                if line != spec {
                    const SPLITTER: &'static str = "-------------------";
                    panic!("\n{}\nUnexpected event at line {}:\nExpected: {}\nFound:    {}\n{}\n",
                           SPLITTER, n + 1, spec, line, SPLITTER);
                }
            } else {
                panic!("Unexpected event: {}", line);
            }
        }
    }
}

fn quick_xml_to_xmlrs(e: &Option<ResultPos<Event>>, is_short: bool) -> Option<String> {
    match *e {
        Some(Ok(Event::Start(ref e))) => {
            let atts: String = e.attributes()
                .map(|a| a.unwrap())
                .map(|(k, v)| format!("{}=\"{}\"", k.as_str().unwrap(), v.as_str().unwrap()))
                .collect::<Vec<_>>()
                .join(", ");

            if atts.is_empty() {
                Some(format!("StartElement({})", e.name().as_str().unwrap()))
            } else {
                Some(format!("StartElement({} [{}])", e.name().as_str().unwrap(), atts))
            }
        },
        Some(Ok(Event::End(ref e))) => {
            Some(format!("EndElement({})", e.name().as_str().unwrap()))
        },
        Some(Ok(Event::Comment(ref e))) => {
            if is_short {
                None
            } else {
                Some(format!("Comment({})", e.content().as_str().unwrap()))
            }
        },
        Some(Ok(Event::CData(ref e))) => {
            if is_short {
                Some(format!("Characters({})", e.content().as_str().unwrap()))
            } else {
                Some(format!("CData({})", e.content().as_str().unwrap()))
            }
        },
        Some(Ok(Event::Text(ref e))) => {
            Some(format!("Characters({})", e.content().as_str().unwrap()))
        },
        Some(Ok(Event::Decl(ref e))) => {
            Some(format!("StartDocument({}, {})", 
                         e.version().unwrap().as_str().unwrap(), 
                         e.encoding().unwrap().unwrap().as_str().unwrap()))
        },
        Some(Err((ref e, i))) => Some(format!("error")),
        None => Some("EndDocument".to_owned()),
        _ => None,
    }

}

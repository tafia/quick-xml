use pretty_assertions::assert_eq;
use quick_xml::encoding::Decoder;
use quick_xml::escape::unescape;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::{QName, ResolveResult};
use quick_xml::reader::NsReader;
use std::str::from_utf8;

#[test]
fn sample_1_short() {
    test(
        include_str!("documents/sample_1.xml"),
        include_str!("documents/sample_1_short.txt"),
        true,
    );
}

#[test]
fn sample_1_full() {
    test(
        include_str!("documents/sample_1.xml"),
        include_str!("documents/sample_1_full.txt"),
        false,
    );
}

#[test]
fn sample_2_short() {
    test(
        include_str!("documents/sample_2.xml"),
        include_str!("documents/sample_2_short.txt"),
        true,
    );
}

#[test]
fn sample_2_full() {
    test(
        include_str!("documents/sample_2.xml"),
        include_str!("documents/sample_2_full.txt"),
        false,
    );
}

#[cfg(feature = "escape-html")]
#[test]
fn html5() {
    test(
        include_str!("documents/html5.html"),
        include_str!("documents/html5.txt"),
        false,
    );
}

#[test]
fn bom_removed_from_initial_text() {
    let expected = r#"
        |Characters(asdf)
        |StartElement(paired [attr1="value1", attr2="value2"])
        |Characters(text)
        |EndElement(paired)
        |EndDocument
    "#;

    // BOM right up against the text
    test(
        "\u{FEFF}asdf<paired attr1=\"value1\" attr2=\"value2\">text</paired>",
        expected,
        true,
    );
}

#[test]
fn escaped_characters() {
    test(
        r#"<e attr="&quot;Hello&quot;">&apos;a&apos; &lt; &apos;&amp;&apos;</e>"#,
        r#"
            |StartElement(e [attr=""Hello""])
            |Characters('a' < '&')
            |EndElement(e)
            |EndDocument
        "#,
        true,
    )
}

#[cfg(feature = "escape-html")]
#[test]
fn escaped_characters_html() {
    test(
        r#"<e attr="&planck;&Egrave;&ell;&#x1D55D;&bigodot;">&boxDR;&boxDL;&#x02554;&#x02557;&#9556;&#9559;</e>"#,
        r#"
            |StartElement(e [attr="ℏÈℓ𝕝⨀"])
            |Characters(╔╗╔╗╔╗)
            |EndElement(e)
            |EndDocument
        "#,
        true,
    )
}

#[cfg(feature = "encoding")]
#[test]
fn encoded_characters() {
    test_bytes(
        b"\
            <?xml version = \"1.0\" encoding = \"Shift_JIS\" ?>\n\
            <a>\x82\xA0\x82\xA2\x82\xA4</a>\
        ",
        "
            |StartDocument(1.0, Shift_JIS)
            |StartElement(a)
            |Characters(あいう)
            |EndElement(a)
            |EndDocument
        "
        .as_bytes(),
        true,
    )
}

// #[test]
// fn sample_3_short() {
//     test(
//         include_str!("documents/sample_3.xml"),
//         include_str!("documents/sample_3_short.txt"),
//         true
//     );
// }

// #[test]
// fn sample_3_full() {
//     test(
//         include_str!("documents/sample_3.xml"),
//         include_str!("documents/sample_3_full.txt"),
//         false
//     );
// }

// #[test]
// fn sample_4_short() {
//     test(
//         include_str!("documents/sample_4.xml"),
//         include_str!("documents/sample_4_short.txt"),
//         true
//     );
// }

// #[test]
// fn sample_4_full() {
//     test(
//         include_str!("documents/sample_4.xml"),
//         include_str!("documents/sample_4_full.txt"),
//         false
//     );
//
// }

#[test]
// FIXME: Trips on the first byte-order-mark byte
// Expected: StartDocument(1.0, utf-16)
// Found: InvalidUtf8([255, 254]; invalid utf-8 sequence of 1 bytes from index 0)
#[ignore]
fn sample_5_short() {
    test_bytes(
        include_bytes!("documents/sample_5_utf16bom.xml"),
        include_bytes!("documents/sample_5_short.txt"),
        true,
    );
}

#[test]
fn sample_ns_short() {
    test(
        include_str!("documents/sample_ns.xml"),
        include_str!("documents/sample_ns_short.txt"),
        true,
    );
}

#[test]
fn eof_1() {
    test(
        r#"<?xml"#,
        r#"Error: Unexpected EOF during reading XmlDecl"#,
        true,
    );
}

#[test]
fn bad_1() {
    test(
        r#"<?xml&.,"#,
        r#"1:6 Error: Unexpected EOF during reading XmlDecl"#,
        true,
    );
}

#[test]
fn dashes_in_comments() {
    test(
        r#"<!-- comment -- --><hello/>"#,
        r#"
        |Error: Unexpected token '--'
        "#,
        true,
    );

    test(
        r#"<!-- comment ---><hello/>"#,
        r#"
        |Error: Unexpected token '--'
        "#,
        true,
    );

    // Canary test for correct comments
    test(
        r#"<!-- comment --><hello/>"#,
        r#"
        |Comment( comment )
        |EmptyElement(hello)
        |EndDocument
        "#,
        true,
    );
}

#[test]
fn tabs_1() {
    test(
        "\t<a>\t<b/></a>",
        r#"
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
        r#"<hello><some-tag a='10' a="20"/></hello>"#,
        "
            |StartElement(hello)
            |1:30 EmptyElement(some-tag, attr-error: \
                  position 16: duplicated attribute, previous declaration at position 9)
            |EndElement(hello)
            |EndDocument
        ",
        true,
    );
}

#[test]
fn issue_93_large_characters_in_entity_references() {
    test(
        r#"<hello>&𤶼;</hello>"#,
        r#"
            |StartElement(hello)
            |1:10 FailedUnescape([38, 240, 164, 182, 188, 59]; Error while escaping character at range 1..5: Unrecognized escape symbol: "𤶼")
            |EndElement(hello)
            |EndDocument
        "#,
        true,
    )
}

#[test]
fn issue_98_cdata_ending_with_right_bracket() {
    test(
        r#"<hello><![CDATA[Foo [Bar]]]></hello>"#,
        r#"
            |StartElement(hello)
            |CData(Foo [Bar])
            |EndElement(hello)
            |EndDocument
        "#,
        false,
    )
}

#[test]
fn issue_105_unexpected_double_dash() {
    test(
        r#"<hello>-- </hello>"#,
        r#"
            |StartElement(hello)
            |Characters(-- )
            |EndElement(hello)
            |EndDocument
        "#,
        false,
    );

    test(
        r#"<hello>--</hello>"#,
        r#"
            |StartElement(hello)
            |Characters(--)
            |EndElement(hello)
            |EndDocument
        "#,
        false,
    );

    test(
        r#"<hello>--></hello>"#,
        r#"
            |StartElement(hello)
            |Characters(-->)
            |EndElement(hello)
            |EndDocument
        "#,
        false,
    );

    test(
        r#"<hello><![CDATA[--]]></hello>"#,
        r#"
            |StartElement(hello)
            |CData(--)
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
        r#"<hello xmlns="urn:foo" x="y"/>"#,
        r#"
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
        r#"<hello xmlns="urn:foo"/>"#,
        r#"
                |EmptyElement({urn:foo}hello)
                |EndDocument
            "#,
        true,
    );
}

#[test]
fn default_namespace_applies_to_end_elem() {
    test(
        r#"<hello xmlns="urn:foo" x="y">
              <inner/>
            </hello>"#,
        r#"
            |StartElement({urn:foo}hello [x="y"])
            |EmptyElement({urn:foo}inner)
            |EndElement({urn:foo}hello)
            |EndDocument
        "#,
        true,
    );
}

#[track_caller]
fn test(input: &str, output: &str, trim: bool) {
    test_bytes(input.as_bytes(), output.as_bytes(), trim);
}

#[track_caller]
fn test_bytes(input: &[u8], output: &[u8], trim: bool) {
    let mut reader = NsReader::from_reader(input);
    reader
        .trim_text(trim)
        .check_comments(true)
        .expand_empty_elements(false);

    let mut spec_lines = SpecIter(output).enumerate();

    let mut decoder = reader.decoder();
    loop {
        let line = match reader.read_resolved_event() {
            Ok((_, Event::Decl(e))) => {
                // Declaration could change decoder
                decoder = reader.decoder();

                let version_cow = e.version().unwrap();
                let version = decoder.decode(version_cow.as_ref()).unwrap();
                let encoding_cow = e.encoding().unwrap().unwrap();
                let encoding = decoder.decode(encoding_cow.as_ref()).unwrap();
                format!("StartDocument({}, {})", version, encoding)
            }
            Ok((_, Event::PI(e))) => {
                format!("ProcessingInstruction(PI={})", decoder.decode(&e).unwrap())
            }
            Ok((_, Event::DocType(e))) => format!("DocType({})", decoder.decode(&e).unwrap()),
            Ok((n, Event::Start(e))) => {
                let name = namespace_name(n, e.name(), decoder);
                match make_attrs(&e, decoder) {
                    Ok(attrs) if attrs.is_empty() => format!("StartElement({})", &name),
                    Ok(attrs) => format!("StartElement({} [{}])", &name, &attrs),
                    Err(e) => format!("StartElement({}, attr-error: {})", &name, &e),
                }
            }
            Ok((n, Event::Empty(e))) => {
                let name = namespace_name(n, e.name(), decoder);
                match make_attrs(&e, decoder) {
                    Ok(attrs) if attrs.is_empty() => format!("EmptyElement({})", &name),
                    Ok(attrs) => format!("EmptyElement({} [{}])", &name, &attrs),
                    Err(e) => format!("EmptyElement({}, attr-error: {})", &name, &e),
                }
            }
            Ok((n, Event::End(e))) => {
                let name = namespace_name(n, e.name(), decoder);
                format!("EndElement({})", name)
            }
            Ok((_, Event::Comment(e))) => format!("Comment({})", decoder.decode(&e).unwrap()),
            Ok((_, Event::CData(e))) => format!("CData({})", decoder.decode(&e).unwrap()),
            Ok((_, Event::Text(e))) => match unescape(&decoder.decode(&e).unwrap()) {
                Ok(c) => format!("Characters({})", &c),
                Err(err) => format!("FailedUnescape({:?}; {})", e.as_ref(), err),
            },
            Ok((_, Event::Eof)) => format!("EndDocument"),
            Err(e) => format!("Error: {}", e),
        };
        if let Some((n, spec)) = spec_lines.next() {
            if spec.trim() == "EndDocument" {
                break;
            }
            assert_eq!(
                line.trim(),
                spec.trim(),
                "Unexpected event at line {}",
                n + 1
            );
        } else {
            if line == "EndDocument" {
                break;
            }
            panic!("Unexpected event: {}", line);
        }
    }
}

fn namespace_name(n: ResolveResult, name: QName, decoder: Decoder) -> String {
    let name = decoder.decode(name.as_ref()).unwrap();
    match n {
        // Produces string '{namespace}prefixed_name'
        ResolveResult::Bound(n) => format!("{{{}}}{}", decoder.decode(n.as_ref()).unwrap(), name),
        _ => name.to_string(),
    }
}

fn make_attrs(e: &BytesStart, decoder: Decoder) -> ::std::result::Result<String, String> {
    let mut atts = Vec::new();
    for a in e.attributes() {
        match a {
            Ok(a) => {
                if a.key.as_namespace_binding().is_none() {
                    let key = decoder.decode(a.key.as_ref()).unwrap();
                    let value = decoder.decode(a.value.as_ref()).unwrap();
                    let unescaped_value = unescape(&value).unwrap();
                    atts.push(format!(
                        "{}=\"{}\"",
                        key,
                        // unescape does not change validity of an UTF-8 string
                        &unescaped_value
                    ));
                }
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(atts.join(", "))
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

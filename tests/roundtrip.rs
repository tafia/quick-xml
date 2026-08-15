//! Contains tests that checks that writing events from a reader produces the same documents.

use quick_xml::escape::unescape;
use quick_xml::events::attributes::AttrError;
use quick_xml::events::{BytesCData, BytesEnd, BytesStart, BytesText, Event::*};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use quick_xml::XmlVersion;

use pretty_assertions::assert_eq;

mod events {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Test start and end together because reading only end event requires special
    /// setting on the reader
    #[test]
    fn start_end() {
        let input = r#"<source attr="val" attr2 = ' "-->&entity;<-- '></source>"#;
        let mut reader = Reader::from_str(input);
        let mut writer = Writer::new(Vec::new());
        loop {
            match reader.read_event().unwrap() {
                Eof => break,
                e => assert!(writer.write_event(e).is_ok()),
            }
        }

        let result = writer.into_inner();
        assert_eq!(String::from_utf8(result).unwrap(), input);
    }

    #[test]
    fn empty() {
        let input = r#"<source attr="val" attr2 = ' "-->&entity;<-- '/>"#;
        let mut reader = Reader::from_str(input);
        let mut writer = Writer::new(Vec::new());
        loop {
            match reader.read_event().unwrap() {
                Eof => break,
                e => assert!(writer.write_event(e).is_ok()),
            }
        }

        let result = writer.into_inner();
        assert_eq!(String::from_utf8(result).unwrap(), input);
    }

    #[test]
    fn text() {
        let input = "it is just arbitrary text &amp; some character reference";
        let mut reader = Reader::from_str(input);
        let mut writer = Writer::new(Vec::new());
        loop {
            match reader.read_event().unwrap() {
                Eof => break,
                e => assert!(writer.write_event(e).is_ok()),
            }
        }

        let result = writer.into_inner();
        assert_eq!(String::from_utf8(result).unwrap(), input);
    }

    #[test]
    fn cdata() {
        let input = "<![CDATA[text & no references]]>";
        let mut reader = Reader::from_str(input);
        let mut writer = Writer::new(Vec::new());
        loop {
            match reader.read_event().unwrap() {
                Eof => break,
                e => assert!(writer.write_event(e).is_ok()),
            }
        }

        let result = writer.into_inner();
        assert_eq!(String::from_utf8(result).unwrap(), input);
    }

    #[test]
    fn pi() {
        let input = "<?!-- some strange processing instruction ?>";
        let mut reader = Reader::from_str(input);
        let mut writer = Writer::new(Vec::new());
        loop {
            match reader.read_event().unwrap() {
                Eof => break,
                e => assert!(writer.write_event(e).is_ok()),
            }
        }

        let result = writer.into_inner();
        assert_eq!(String::from_utf8(result).unwrap(), input);
    }

    #[test]
    fn decl() {
        let input = "<?xml some strange XML declaration ?>";
        let mut reader = Reader::from_str(input);
        let mut writer = Writer::new(Vec::new());
        loop {
            match reader.read_event().unwrap() {
                Eof => break,
                e => assert!(writer.write_event(e).is_ok()),
            }
        }

        let result = writer.into_inner();
        assert_eq!(String::from_utf8(result).unwrap(), input);
    }

    #[test]
    fn comment() {
        let input = "<!-- some comment with -- inside---->";
        let mut reader = Reader::from_str(input);
        let mut writer = Writer::new(Vec::new());
        loop {
            match reader.read_event().unwrap() {
                Eof => break,
                e => assert!(writer.write_event(e).is_ok()),
            }
        }

        let result = writer.into_inner();
        assert_eq!(String::from_utf8(result).unwrap(), input);
    }
}

/// Indent of the last tag mismatched intentionally
const XML: &str = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <section ns:label="header">
            <section ns:label="empty element section" />
            <section ns:label="start/end section"></section>
            <section ns:label="with text">data &lt;escaped&gt;</section>
            </section>
    "#;

/// Directly write event from reader without any processing.
#[test]
fn simple() {
    let mut reader = Reader::from_str(XML);
    let mut writer = Writer::new(Vec::new());
    loop {
        match reader.read_event().unwrap() {
            Eof => break,
            e => assert!(writer.write_event(e).is_ok()),
        }
    }

    let result = writer.into_inner();
    assert_eq!(String::from_utf8(result).unwrap(), XML);
}

/// Directly write event from reader without processing (except auto-trimming text).
#[test]
fn with_trim() {
    let input = include_str!("documents/test_writer.xml").trim();
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(true);
    let mut writer = Writer::new(Vec::new());
    loop {
        match reader.read_event().unwrap() {
            Eof => break,
            e => assert!(writer.write_event(e).is_ok()),
        }
    }

    let result = writer.into_inner();
    assert_eq!(String::from_utf8(result).unwrap(), input);
}

/// Directly write reference to event from reader without processing (except auto-trimming text).
#[test]
fn with_trim_ref() {
    let input = include_str!("documents/test_writer.xml").trim();
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(true);
    let mut writer = Writer::new(Vec::new());
    loop {
        match reader.read_event().unwrap() {
            Eof => break,
            e => assert!(writer.write_event(e.borrow()).is_ok()), // either `e` or `&e`
        }
    }

    let result = writer.into_inner();
    assert_eq!(String::from_utf8(result).unwrap(), input);
}

/// Directly write event from reader without processing (except auto-trimming text)
/// with the same indentation settings as in the original document.
#[test]
fn with_indent() {
    let input = include_str!("documents/test_writer_indent.xml");
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(true);
    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 4);
    loop {
        match reader.read_event().unwrap() {
            Eof => break,
            e => assert!(writer.write_event(e).is_ok()),
        }
    }

    let result = writer.into_inner();
    assert_eq!(String::from_utf8(result).unwrap(), input);
}

/// Directly write event from reader without processing (except auto-trimming text)
/// with the same indentation settings as in the original document.
/// Document contains CDATA section.
#[test]
fn with_indent_cdata() {
    let input = include_str!("documents/test_writer_indent_cdata.xml");
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(true);
    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 4);
    loop {
        match reader.read_event().unwrap() {
            Eof => break,
            e => assert!(writer.write_event(e).is_ok()),
        }
    }

    let result = writer.into_inner();
    assert_eq!(String::from_utf8(result).unwrap(), input);
}

/// Directly write event from reader with unescaping and re-escaping content of the `Text` events.
#[test]
fn reescape_text() {
    let mut reader = Reader::from_str(XML);
    let mut writer = Writer::new(Vec::new());
    loop {
        match reader.read_event().unwrap() {
            Eof => break,
            Text(e) => {
                let t = e.xml10_content();
                assert!(writer.write_event(Text(BytesText::new(&t))).is_ok());
            }
            e => assert!(writer.write_event(e).is_ok()),
        }
    }

    let result = writer.into_inner();
    assert_eq!(String::from_utf8(result).unwrap(), XML);
}

/// Verify that normalization-sensitive characters (\r in text, \r\n\t in attributes)
/// are properly escaped when using BytesText::new() and push_attribute(). Otherwise
/// these characters would be stripped upon being parsed by a compliant XML parser.
/// This is also consistent with libxml2.
#[test]
fn normalization_sensitive_roundtrip() {
    let mut writer = Writer::new(Vec::new());

    let mut start = BytesStart::new("root");
    start.push_attribute(("attr", "hello\r\nworld\there"));
    writer.write_event(Start(start)).unwrap();
    writer
        .write_event(Text(BytesText::new("text\rwith\r\ncr")))
        .unwrap();
    writer.write_event(End(BytesEnd::new("root"))).unwrap();

    let xml = String::from_utf8(writer.into_inner()).unwrap();

    assert_eq!(
        xml,
        "<root attr=\"hello&#13;&#10;world&#9;here\">text&#13;with&#13;\ncr</root>"
    );

    let mut reader = Reader::from_str(&xml);
    loop {
        match reader.read_event().unwrap() {
            Start(e) => {
                let attr = e.try_get_attribute("attr").unwrap().unwrap();
                let value = attr.normalized_value(XmlVersion::Implicit1_0).unwrap();
                assert_eq!(value.as_ref(), "hello\r\nworld\there");
            }
            Eof => break,
            _ => {}
        }
    }

    let mut reader = Reader::from_str(&xml);
    reader.config_mut().expand_empty_elements = true;
    loop {
        match reader.read_event().unwrap() {
            Start(e) if e.name().as_ref() == "root" => {
                let text = reader.read_text(e.name()).unwrap();
                let normalized = text.xml10_content();
                let content = unescape(&normalized).unwrap();
                assert_eq!(content.as_ref(), "text\rwith\r\ncr");
            }
            Eof => break,
            _ => {}
        }
    }
}

/// CDATA sections cannot contain character references. Verify that writing
/// normalization-sensitive characters via BytesCData produces literal characters
/// (not &#13; etc.), and that \r is normalized to \n on parse (inherent XML limitation).
#[test]
fn cdata_no_character_references() {
    for (label, value, expected_written, expected_parsed) in [
        (
            "CR",
            "hello\rworld",
            "<v><![CDATA[hello\rworld]]></v>",
            "hello\nworld",
        ),
        (
            "CRLF",
            "hello\r\nworld",
            "<v><![CDATA[hello\r\nworld]]></v>",
            "hello\nworld",
        ),
        (
            "LF",
            "hello\nworld",
            "<v><![CDATA[hello\nworld]]></v>",
            "hello\nworld",
        ),
        (
            "TAB",
            "col1\tcol2",
            "<v><![CDATA[col1\tcol2]]></v>",
            "col1\tcol2",
        ),
        (
            "all",
            "\t\r\n&<>\"'",
            "<v><![CDATA[\t\r\n&<>\"']]></v>",
            "\t\n&<>\"'",
        ),
    ] {
        let mut writer = Writer::new(Vec::new());
        writer.write_event(Start(BytesStart::new("v"))).unwrap();
        writer.write_event(CData(BytesCData::new(value))).unwrap();
        writer.write_event(End(BytesEnd::new("v"))).unwrap();
        let xml = String::from_utf8(writer.into_inner()).unwrap();

        assert_eq!(xml, expected_written, "{label}: serialized XML mismatch");
        assert!(
            !xml.contains("&#"),
            "{label}: CDATA must not contain character references, got: {xml:?}",
        );

        let mut reader = Reader::from_str(&xml);
        loop {
            match reader.read_event().unwrap() {
                CData(e) => {
                    let content = e.xml10_content();
                    assert_eq!(
                        content.as_ref(),
                        expected_parsed,
                        "{label}: parsed content mismatch"
                    );
                    break;
                }
                Eof => panic!("{label}: unexpected EOF"),
                _ => {}
            }
        }
    }
}

/// BytesCData::escape() converts CDATA to text, which should escape \r as &#13;.
/// This is correct because the result is a BytesText, not CDATA.
#[test]
fn cdata_escape_to_text_preserves_cr() {
    let cdata = BytesCData::new("hello\rworld");
    let text = cdata.escape().unwrap();
    assert_eq!(text.as_ref(), "hello&#13;world");

    let cdata = BytesCData::new("hello\r\nworld");
    let text = cdata.escape().unwrap();
    assert_eq!(text.as_ref(), "hello&#13;\nworld");

    let cdata = BytesCData::new("a\tb\nc");
    let text = cdata.partial_escape().unwrap();
    assert_eq!(
        text.as_ref(),
        "a\tb\nc",
        "partial_escape should not escape TAB or LF in text"
    );

    let cdata = BytesCData::new("a\rb");
    let text = cdata.minimal_escape().unwrap();
    assert_eq!(text.as_ref(), "a&#13;b");
}

/// Rewrite some events during processing
#[test]
fn partial_rewrite() {
    type AttrResult<T> = std::result::Result<T, AttrError>;

    let str_from = r#"<source attr="val"></source>"#;
    let expected = r#"<copy attr="val" a="b" c="d" x="y&quot;z"></copy>"#;
    let mut reader = Reader::from_str(str_from);
    let mut writer = Writer::new(Vec::new());
    loop {
        let event = match reader.read_event().unwrap() {
            Eof => break,
            Start(elem) => {
                let mut attrs = elem.attributes().collect::<AttrResult<Vec<_>>>().unwrap();
                attrs.extend_from_slice(&[("a", "b").into(), ("c", "d").into()]);
                let mut elem = BytesStart::new("copy");
                elem.extend_attributes(attrs);
                elem.push_attribute(("x", "y\"z"));
                Start(elem)
            }
            End(_) => End(BytesEnd::new("copy")),
            e => e,
        };
        assert!(writer.write_event(event).is_ok());
    }

    let result = writer.into_inner();
    assert_eq!(String::from_utf8(result).unwrap(), expected);
}

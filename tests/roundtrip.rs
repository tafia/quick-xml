//! Contains tests that checks that writing events from a reader produces the same documents.

use quick_xml::events::attributes::AttrError;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event::*};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;

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
                let t = e.decode().unwrap();
                assert!(writer.write_event(Text(BytesText::new(&t))).is_ok());
            }
            e => assert!(writer.write_event(e).is_ok()),
        }
    }

    let result = writer.into_inner();
    assert_eq!(String::from_utf8(result).unwrap(), XML);
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

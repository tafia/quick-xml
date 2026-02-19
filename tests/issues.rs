//! Regression tests found in various issues.
//!
//! Name each module / test as `issue<GH number>` and keep sorted by issue number

use std::io::{BufRead, BufReader, Cursor, Read};
use std::iter;
use std::sync::mpsc;

use quick_xml::errors::{Error, IllFormedError, SyntaxError};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::name::{Namespace, QName, ResolveResult};
use quick_xml::reader::{NsReader, Reader};
use quick_xml::utils::Bytes;

use pretty_assertions::assert_eq;

/// Regression test for https://github.com/tafia/quick-xml/issues/94
#[test]
fn issue94() {
    let data = br#"<Run>
<!B>
</Run>"#;
    let mut reader = Reader::from_reader(&data[..]);
    reader.config_mut().trim_text(true);
    loop {
        match reader.read_event() {
            Ok(Event::Eof) | Err(..) => break,
            _ => (),
        }
    }
}

/// Regression test for https://github.com/tafia/quick-xml/issues/115
#[test]
fn issue115() {
    let mut r = Reader::from_str("<tag1 attr1='line 1\nline 2'></tag1>");
    match r.read_event() {
        Ok(Event::Start(e)) if e.name() == QName(b"tag1") => {
            let v = e.attributes().map(|a| a.unwrap().value).collect::<Vec<_>>();
            assert_eq!(v[0].clone().into_owned(), b"line 1\nline 2");
        }
        _ => (),
    }
}

/// Regression test for https://github.com/tafia/quick-xml/issues/299
#[test]
fn issue299() -> Result<(), Error> {
    let xml = r#"
<?xml version="1.0" encoding="utf8"?>
<MICEX_DOC xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <SECURITY SecurityId="PLZL" ISIN="RU000A0JNAA8" SecShortName="Short Name" PriceType="CASH">
    <RECORDS RecNo="1" TradeNo="1111" TradeDate="2021-07-08" TradeTime="15:00:00" BuySell="S" SettleCode="Y1Dt" Decimals="3" Price="13057.034" Quantity="766" Value="10001688.29" AccInt="0" Amount="10001688.29" Balance="766" TrdAccId="X0011" ClientDetails="2222" CPFirmId="3333" CPFirmShortName="Firm Short Name" Price2="13057.034" RepoPart="2" ReportTime="16:53:27" SettleTime="17:47:06" ClientCode="4444" DueDate="2021-07-09" EarlySettleStatus="N" RepoRate="5.45" RateType="FIX"/>
  </SECURITY>
</MICEX_DOC>
"#;
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event()? {
            Event::Start(e) | Event::Empty(e) => {
                let attr_count = match e.name().as_ref() {
                    b"MICEX_DOC" => 1,
                    b"SECURITY" => 4,
                    b"RECORDS" => 26,
                    _ => unreachable!(),
                };
                assert_eq!(
                    attr_count,
                    e.attributes().filter(Result::is_ok).count(),
                    "mismatch att count on '{:?}'",
                    reader.decoder().decode(e.name().as_ref())
                );
            }
            Event::Eof => break,
            _ => (),
        }
    }
    Ok(())
}

/// Regression test for https://github.com/tafia/quick-xml/issues/344
#[test]
fn issue344() {
    let mut reader = Reader::from_str("<!D>");
    let mut buf = Vec::new();
    let _ = reader.read_event_into(&mut buf);
    let _ = reader.read_event_into(&mut buf);
}

/// Regression test for https://github.com/tafia/quick-xml/issues/360
#[test]
fn issue360() {
    let (tx, rx) = mpsc::channel::<Event>();

    std::thread::spawn(move || {
        let mut r = Reader::from_str("<tag1 attr1='line 1\nline 2'></tag1>");
        loop {
            let event = r.read_event().unwrap();
            if event == Event::Eof {
                tx.send(event).unwrap();
                break;
            } else {
                tx.send(event).unwrap();
            }
        }
    });
    for event in rx.iter() {
        println!("{:?}", event);
    }
}

/// Regression test for https://github.com/tafia/quick-xml/issues/514
mod issue514 {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Check that there is no unexpected error
    #[test]
    fn no_mismatch() {
        let mut reader = Reader::from_str("<some-tag><html>...</html></some-tag>");

        let outer_start = BytesStart::new("some-tag");
        let outer_end = outer_start.to_end().into_owned();

        let html_start = BytesStart::new("html");
        let html_end = html_start.to_end().into_owned();

        assert_eq!(reader.read_event().unwrap(), Event::Start(outer_start));
        assert_eq!(reader.read_event().unwrap(), Event::Start(html_start));

        reader.config_mut().check_end_names = false;

        assert_eq!(reader.read_text(html_end.name()).unwrap(), "...");

        reader.config_mut().check_end_names = true;

        assert_eq!(reader.read_event().unwrap(), Event::End(outer_end));
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }

    /// Canary check that legitimate error is reported
    #[test]
    fn mismatch() {
        let mut reader = Reader::from_str("<some-tag><html>...</html></other-tag>");

        let outer_start = BytesStart::new("some-tag");

        let html_start = BytesStart::new("html");
        let html_end = html_start.to_end().into_owned();

        assert_eq!(reader.read_event().unwrap(), Event::Start(outer_start));
        assert_eq!(reader.read_event().unwrap(), Event::Start(html_start));

        reader.config_mut().check_end_names = false;

        assert_eq!(reader.read_text(html_end.name()).unwrap(), "...");

        reader.config_mut().check_end_names = true;

        match reader.read_event() {
            Err(Error::IllFormed(cause)) => assert_eq!(
                cause,
                IllFormedError::MismatchedEndTag {
                    expected: "some-tag".into(),
                    found: "other-tag".into(),
                }
            ),
            x => panic!("Expected `Err(IllFormed(_))`, but got `{:?}`", x),
        }
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }
}

/// Regression test for https://github.com/tafia/quick-xml/issues/590
#[test]
fn issue590() {
    let mut reader = Reader::from_reader(BufReader::with_capacity(
        24,
        &b"<!DOCTYPE t [<!ENTITY x 'a'><!ENTITY y 'b'>]>"[..],
        // 0      7               ^23                  ^44
        //[                        ] = capacity
    ));
    // Ensure, that capacity was not increased unexpectedly
    assert_eq!(reader.get_ref().capacity(), 24);
    let mut buf = Vec::new();
    loop {
        if reader.read_event_into(&mut buf).unwrap() == Event::Eof {
            break;
        }
    }
}

#[test]
fn issue597() {
    const S: &'static str = r#"
    <?xml version="1.0" encoding="UTF-8"?>
    <oval_definitions xmlns="http://oval.mitre.org/XMLSchema/oval-definitions-5">
        <tests>
            <xmlfilecontent_test xmlns="http://oval.mitre.org/XMLSchema/oval-definitions-5#independent">
            </xmlfilecontent_test>
            <xmlfilecontent_test xmlns="http://oval.mitre.org/XMLSchema/oval-definitions-5#independent">
            </xmlfilecontent_test>
        </tests>
        <objects/>
    </oval_definitions>"#;

    let mut reader = NsReader::from_str(S);
    let objects_ns = loop {
        let (ns, ev) = reader.read_resolved_event().unwrap();
        match ev {
            Event::Start(v) if v.local_name().as_ref() == b"xmlfilecontent_test" => {
                reader.read_to_end(v.name()).unwrap();
            }
            Event::Empty(v) if v.local_name().as_ref() == b"objects" => break ns,
            _ => (),
        }
    };
    assert_eq!(
        objects_ns,
        ResolveResult::Bound(Namespace(
            b"http://oval.mitre.org/XMLSchema/oval-definitions-5"
        ))
    );
}

/// Regression test for https://github.com/tafia/quick-xml/issues/604
mod issue604 {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn short() {
        let data = b"<?xml version=\"1.0\"?><!-->";
        let mut reader = Reader::from_reader(data.as_slice());
        let mut buf = Vec::new();
        assert_eq!(
            reader.read_event_into(&mut buf).unwrap(),
            Event::Decl(BytesDecl::new("1.0", None, None))
        );
        match reader.read_event_into(&mut buf) {
            Err(Error::Syntax(SyntaxError::UnclosedComment)) => {}
            x => panic!("Expected `Err(Syntax(UnclosedComment))`, but got `{:?}`", x),
        }
        assert_eq!(reader.read_event_into(&mut buf).unwrap(), Event::Eof);
    }

    #[test]
    fn long() {
        let data = b"<?xml version=\"1.0\"?><!--->";
        let mut reader = Reader::from_reader(data.as_slice());
        let mut buf = Vec::new();
        assert_eq!(
            reader.read_event_into(&mut buf).unwrap(),
            Event::Decl(BytesDecl::new("1.0", None, None))
        );
        match reader.read_event_into(&mut buf) {
            Err(Error::Syntax(SyntaxError::UnclosedComment)) => {}
            x => panic!("Expected `Err(Syntax(UnclosedComment))`, but got `{:?}`", x),
        }
        assert_eq!(reader.read_event_into(&mut buf).unwrap(), Event::Eof);
    }

    /// According to the grammar, `>` is allowed just in start of comment.
    /// See https://www.w3.org/TR/xml11/#sec-comments
    #[test]
    fn short_valid() {
        let data = b"<?xml version=\"1.0\"?><!-->-->";
        let mut reader = Reader::from_reader(data.as_slice());
        let mut buf = Vec::new();
        assert_eq!(
            reader.read_event_into(&mut buf).unwrap(),
            Event::Decl(BytesDecl::new("1.0", None, None))
        );
        assert_eq!(
            reader.read_event_into(&mut buf).unwrap(),
            Event::Comment(BytesText::from_escaped(">"))
        );
        assert_eq!(reader.read_event_into(&mut buf).unwrap(), Event::Eof);
    }

    /// According to the grammar, `->` is allowed just in start of comment.
    /// See https://www.w3.org/TR/xml11/#sec-comments
    #[test]
    fn long_valid() {
        let data = b"<?xml version=\"1.0\"?><!--->-->";
        let mut reader = Reader::from_reader(data.as_slice());
        let mut buf = Vec::new();
        assert_eq!(
            reader.read_event_into(&mut buf).unwrap(),
            Event::Decl(BytesDecl::new("1.0", None, None))
        );
        assert_eq!(
            reader.read_event_into(&mut buf).unwrap(),
            Event::Comment(BytesText::from_escaped("->"))
        );
        assert_eq!(reader.read_event_into(&mut buf).unwrap(), Event::Eof);
    }
}

/// Regression test for https://github.com/tafia/quick-xml/issues/622
#[test]
fn issue622() {
    let mut reader = Reader::from_str("><");
    reader.config_mut().trim_text(true);

    assert_eq!(
        reader.read_event().unwrap(),
        Event::Text(BytesText::from_escaped(">"))
    );
    match reader.read_event() {
        Err(Error::Syntax(cause)) => assert_eq!(cause, SyntaxError::UnclosedTag),
        x => panic!("Expected `Err(Syntax(_))`, but got `{:?}`", x),
    }
}

/// Regression test for https://github.com/tafia/quick-xml/issues/623
mod issue623 {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn borrowed() {
        let mut reader = Reader::from_str(
            "
            <AppendedData>
                _binary << data&>
            </AppendedData>
        ",
        );
        reader.config_mut().trim_text(true);

        assert_eq!(
            (reader.read_event().unwrap(), reader.buffer_position()),
            (Event::Start(BytesStart::new("AppendedData")), 27)
        );

        let mut inner = reader.stream();
        // Read to start of data marker
        inner.read_until(b'_', &mut Vec::new()).unwrap();

        // Read binary data. We must know its size
        let mut binary = [0u8; 16];
        inner.read_exact(&mut binary).unwrap();
        assert_eq!(Bytes(&binary), Bytes(b"binary << data&>"));
        assert_eq!(inner.offset(), 61);
        assert_eq!(reader.buffer_position(), 61);

        assert_eq!(
            (reader.read_event().unwrap(), reader.buffer_position()),
            (Event::End(BytesEnd::new("AppendedData")), 89)
        );

        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }

    #[test]
    fn buffered() {
        let mut buf = Vec::new();
        let mut reader = Reader::from_reader(Cursor::new(
            b"
            <AppendedData>
                _binary << data&>
            </AppendedData>
        ",
        ));
        reader.config_mut().trim_text(true);

        assert_eq!(
            (
                reader.read_event_into(&mut buf).unwrap(),
                reader.buffer_position()
            ),
            (Event::Start(BytesStart::new("AppendedData")), 27)
        );

        let mut inner = reader.stream();
        // Read to start of data marker
        inner.read_until(b'_', &mut buf).unwrap();

        // Read binary data. We must know its size
        let mut binary = [0u8; 16];
        inner.read_exact(&mut binary).unwrap();
        assert_eq!(Bytes(&binary), Bytes(b"binary << data&>"));
        assert_eq!(inner.offset(), 61);
        assert_eq!(reader.buffer_position(), 61);

        assert_eq!(
            (
                reader.read_event_into(&mut buf).unwrap(),
                reader.buffer_position()
            ),
            (Event::End(BytesEnd::new("AppendedData")), 89)
        );

        assert_eq!(reader.read_event_into(&mut buf).unwrap(), Event::Eof);
    }
}

/// Regression test for https://github.com/tafia/quick-xml/issues/706
#[test]
fn issue706() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<?procinst-with-xml
	<parameters>
		<parameter id="version" value="0.1"/>
		<parameter id="timeStamp" value="2024-01-16T10:44:00Z"/>
	</parameters>
?>
<Document/>"#;
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event() {
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            Ok(Event::Eof) => break,
            _ => (),
        }
    }
}

/// Regression test for https://github.com/tafia/quick-xml/issues/751
#[test]
fn issue751() {
    let mut text = Vec::new();
    let mut chunk = Vec::new();
    chunk.extend_from_slice(b"<content>");
    for data in iter::repeat(b"some text inside").take(1000) {
        chunk.extend_from_slice(data);
        text.extend_from_slice(data);
    }
    chunk.extend_from_slice(b"</content>");

    let mut reader = Reader::from_reader(quick_xml::utils::Fountain {
        chunk: &chunk,
        consumed: 0,
        overall_read: 0,
    });
    let mut buf = Vec::new();
    let mut starts = 0u64;
    let mut ends = 0u64;
    let mut texts = 0u64;
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
            Ok(Event::Eof) => break,

            Ok(Event::Start(e)) => {
                starts += 1;
                assert_eq!(
                    e.name(),
                    QName(b"content"),
                    "starts: {starts}, ends: {ends}, texts: {texts}"
                );
            }
            Ok(Event::End(e)) => {
                ends += 1;
                assert_eq!(
                    e.name(),
                    QName(b"content"),
                    "starts: {starts}, ends: {ends}, texts: {texts}"
                );
            }
            Ok(Event::Text(e)) => {
                texts += 1;
                assert_eq!(
                    e.as_ref(),
                    text,
                    "starts: {starts}, ends: {ends}, texts: {texts}"
                );
            }
            _ => (),
        }
        // If we successfully read more than `u32::MAX`, the test is passed
        if reader.get_ref().overall_read >= u32::MAX as u64 {
            break;
        }
    }
}

/// Regression test for https://github.com/tafia/quick-xml/issues/774
///
/// Capacity of the buffer selected in that way, that "text" will be read into
/// one internal buffer of `BufReader` in one `fill_buf()` call and `<` of the
/// closing tag in the next call.
#[test]
fn issue774() {
    let xml = BufReader::with_capacity(9, b"<tag>text</tag>" as &[u8]);
    //                                      ^0       ^9
    let mut reader = Reader::from_reader(xml);
    let mut buf = Vec::new();

    assert_eq!(
        reader.read_event_into(&mut buf).unwrap(),
        Event::Start(BytesStart::new("tag"))
    );
    assert_eq!(
        reader.read_event_into(&mut buf).unwrap(),
        Event::Text(BytesText::new("text"))
    );
    assert_eq!(
        reader.read_event_into(&mut buf).unwrap(),
        Event::End(BytesEnd::new("tag"))
    );
}

/// Regression test for https://github.com/tafia/quick-xml/issues/776
#[test]
fn issue776() {
    let mut reader = Reader::from_str(r#"<tag></tag/><tag></tag attr=">">"#);
    // We still think that the name of the end tag is everything between `</` and `>`
    // and if we do not disable this check we get error
    reader.config_mut().check_end_names = false;

    assert_eq!(
        reader.read_event().unwrap(),
        Event::Start(BytesStart::new("tag"))
    );
    assert_eq!(
        reader.read_event().unwrap(),
        Event::End(BytesEnd::new("tag/"))
    );

    assert_eq!(
        reader.read_event().unwrap(),
        Event::Start(BytesStart::new("tag"))
    );
    assert_eq!(
        reader.read_event().unwrap(),
        Event::End(BytesEnd::new(r#"tag attr=">""#))
    );
}

/// Regression test for https://github.com/tafia/quick-xml/issues/801
/// Angle brackets are read in different buffer
#[test]
fn issue801() {
    let xml = b"<!DOCTYPE X [<!-- --> <!ENTITY a \"a\">]>";
    let reader = BufReader::with_capacity(2, &xml[..]);
    let mut reader = Reader::from_reader(reader);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
            Ok(Event::Eof) => break,
            Ok(_) => {}
        }
    }
}

/// Regression tests for https://github.com/tafia/quick-xml/issues/923
mod issue923 {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn case1() {
        let mut reader = Reader::from_str(
            r#"<?xml version="1.0"?>
<!-- sample.xml -->
<!DOCTYPE root [
        <!ENTITY ent ">">
        <!ELEMENT root (#PCDATA)>
]>
<root>&ent;</root>"#,
        );
        let mut buf = Vec::new();
        reader.read_event_into(&mut buf).unwrap(); // <?xml version="1.0"?>
        reader.read_event_into(&mut buf).unwrap();
        reader.read_event_into(&mut buf).unwrap(); // <!-- sample.xml -->
        reader.read_event_into(&mut buf).unwrap();
        reader.read_event_into(&mut buf).unwrap(); // DTD
        reader.read_event_into(&mut buf).unwrap();
        reader.read_event_into(&mut buf).unwrap(); // <root>
        reader.read_event_into(&mut buf).unwrap(); // &ent;
        reader.read_event_into(&mut buf).unwrap(); // </root>

        assert_eq!(reader.read_event_into(&mut buf).unwrap(), Event::Eof);
    }

    #[test]
    fn case2() {
        let mut reader = Reader::from_str(
            r#"<?xml version="1.0"?>
<!-- sample.xml -->
<!DOCTYPE root [
        <!ENTITY ent "<">
]>
<root />"#,
        );
        let mut buf = Vec::new();
        reader.read_event_into(&mut buf).unwrap(); // <?xml version="1.0"?>
        reader.read_event_into(&mut buf).unwrap();
        reader.read_event_into(&mut buf).unwrap(); // <!-- sample.xml -->
        reader.read_event_into(&mut buf).unwrap();
        reader.read_event_into(&mut buf).unwrap(); // DTD
        reader.read_event_into(&mut buf).unwrap();
        reader.read_event_into(&mut buf).unwrap(); // <root />

        assert_eq!(reader.read_event_into(&mut buf).unwrap(), Event::Eof);
    }
}

/// Regression test for https://github.com/tafia/quick-xml/issues/939
///
/// Checks that reading from a short internal buffer of `BufRead` implementation
/// does not break the parser. Here `BufReader` splits input into two 4-byte chunks:
/// - `<r><`
/// - `/r>`
///
/// Passing of this test shows that `<` in the end of the first chunk does not
/// considered as an incomplete tag and parser correctly consumes this byte and
/// requests the next chunk.
#[test]
fn issue939() {
    let xml_file = BufReader::with_capacity(4, &b"<r></r>"[..]);
    let mut reader = Reader::from_reader(xml_file);
    let mut buf = Vec::new();

    assert_eq!(
        reader.read_event_into(&mut buf).unwrap(),
        Event::Start(BytesStart::new("r"))
    );
    assert_eq!(
        quick_xml::utils::Bytes(&buf),
        quick_xml::utils::Bytes(b"<r")
    );
    assert_eq!(
        quick_xml::utils::Bytes(reader.get_ref().buffer()),
        quick_xml::utils::Bytes(b"<")
    );

    assert_eq!(
        reader.read_event_into(&mut buf).unwrap(),
        Event::End(BytesEnd::new("r"))
    );
    assert_eq!(
        quick_xml::utils::Bytes(&buf),
        quick_xml::utils::Bytes(b"<r</r")
    );

    assert_eq!(reader.read_event_into(&mut buf).unwrap(), Event::Eof);
}

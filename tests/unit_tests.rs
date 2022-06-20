use std::borrow::Cow;
use std::io::Cursor;
use std::str::from_utf8;

use quick_xml::events::attributes::{AttrError, Attribute};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::name::QName;
use quick_xml::{events::Event::*, Reader, Result, Writer};

use pretty_assertions::assert_eq;

macro_rules! next_eq_name {
    ($r:expr, $t:tt, $bytes:expr) => {
        let mut buf = Vec::new();
        match $r.read_event(&mut buf).unwrap() {
            $t(ref e) if e.name().as_ref() == $bytes => (),
            e => panic!(
                "expecting {}({:?}), found {:?}",
                stringify!($t),
                from_utf8($bytes),
                e
            ),
        }
        buf.clear();
    };
}

macro_rules! next_eq_content {
    ($r:expr, $t:tt, $bytes:expr) => {
        let mut buf = Vec::new();
        match $r.read_event(&mut buf).unwrap() {
            $t(ref e) if e.as_ref() == $bytes => (),
            e => panic!(
                "expecting {}({:?}), found {:?}",
                stringify!($t),
                from_utf8($bytes),
                e
            ),
        }
        buf.clear();
    };
}

macro_rules! next_eq {
    ($r:expr, StartText, $bytes:expr) => (next_eq_content!($r, StartText, $bytes););
    ($r:expr, Start, $bytes:expr) => (next_eq_name!($r, Start, $bytes););
    ($r:expr, End, $bytes:expr) => (next_eq_name!($r, End, $bytes););
    ($r:expr, Empty, $bytes:expr) => (next_eq_name!($r, Empty, $bytes););
    ($r:expr, Comment, $bytes:expr) => (next_eq_content!($r, Comment, $bytes););
    ($r:expr, Text, $bytes:expr) => (next_eq_content!($r, Text, $bytes););
    ($r:expr, CData, $bytes:expr) => (next_eq_content!($r, CData, $bytes););
    ($r:expr, $t0:tt, $b0:expr, $($t:tt, $bytes:expr),*) => {
        next_eq!($r, $t0, $b0);
        next_eq!($r, $($t, $bytes),*);
    };
}

#[test]
fn test_start() {
    let mut r = Reader::from_str("<a>");
    r.trim_text(true);
    next_eq!(r, Start, b"a");
}

#[test]
fn test_start_end() {
    let mut r = Reader::from_str("<a></a>");
    r.trim_text(true);
    next_eq!(r, Start, b"a", End, b"a");
}

#[test]
fn test_start_end_with_ws() {
    let mut r = Reader::from_str("<a></a >");
    r.trim_text(true);
    next_eq!(r, Start, b"a", End, b"a");
}

#[test]
fn test_start_end_attr() {
    let mut r = Reader::from_str("<a b=\"test\"></a>");
    r.trim_text(true);
    next_eq!(r, Start, b"a", End, b"a");
}

#[test]
fn test_empty() {
    let mut r = Reader::from_str("<a />");
    r.trim_text(true).expand_empty_elements(false);
    next_eq!(r, Empty, b"a");
}

#[test]
fn test_empty_can_be_expanded() {
    let mut r = Reader::from_str("<a />");
    r.trim_text(true).expand_empty_elements(true);
    next_eq!(r, Start, b"a", End, b"a");
}

#[test]
fn test_empty_attr() {
    let mut r = Reader::from_str("<a b=\"test\" />");
    r.trim_text(true).expand_empty_elements(false);
    next_eq!(r, Empty, b"a");
}

#[test]
fn test_start_end_comment() {
    let mut r = Reader::from_str("<b><a b=\"test\" c=\"test\"/> <a  /><!--t--></b>");
    r.trim_text(true).expand_empty_elements(false);
    next_eq!(r, Start, b"b", Empty, b"a", Empty, b"a", Comment, b"t", End, b"b");
}

#[test]
fn test_start_txt_end() {
    let mut r = Reader::from_str("<a>test</a>");
    r.trim_text(true);
    next_eq!(r, Start, b"a", Text, b"test", End, b"a");
}

#[test]
fn test_comment() {
    let mut r = Reader::from_str("<!--test-->");
    r.trim_text(true);
    next_eq!(r, Comment, b"test");
}

#[test]
fn test_xml_decl() {
    let mut r = Reader::from_str("<?xml version=\"1.0\" encoding='utf-8'?>");
    r.trim_text(true);
    let mut buf = Vec::new();
    match r.read_event(&mut buf).unwrap() {
        Decl(ref e) => {
            match e.version() {
                Ok(v) => assert_eq!(
                    &*v,
                    b"1.0",
                    "expecting version '1.0', got '{:?}",
                    from_utf8(&*v)
                ),
                Err(e) => assert!(false, "{:?}", e),
            }
            match e.encoding() {
                Some(Ok(v)) => assert_eq!(
                    &*v,
                    b"utf-8",
                    "expecting encoding 'utf-8', got '{:?}",
                    from_utf8(&*v)
                ),
                Some(Err(e)) => panic!("{:?}", e),
                None => panic!("cannot find encoding"),
            }
            match e.standalone() {
                None => (),
                e => panic!("doesn't expect standalone, got {:?}", e),
            }
        }
        _ => panic!("unable to parse XmlDecl"),
    }
}

#[test]
fn test_trim_test() {
    let txt = "<a><b>  </b></a>";
    let mut r = Reader::from_str(txt);
    r.trim_text(true);
    next_eq!(r, Start, b"a", Start, b"b", End, b"b", End, b"a");

    let mut r = Reader::from_str(txt);
    r.trim_text(false);
    next_eq!(r, Start, b"a", Start, b"b", Text, b"  ", End, b"b", End, b"a");
}

#[test]
fn test_cdata() {
    let mut r = Reader::from_str("<![CDATA[test]]>");
    r.trim_text(true);
    next_eq!(r, CData, b"test");
}

#[test]
fn test_cdata_open_close() {
    let mut r = Reader::from_str("<![CDATA[test <> test]]>");
    r.trim_text(true);
    next_eq!(r, CData, b"test <> test");
}

#[test]
fn test_start_attr() {
    let mut r = Reader::from_str("<a b=\"c\">");
    r.trim_text(true);
    next_eq!(r, Start, b"a");
}

#[test]
fn test_nested() {
    let mut r = Reader::from_str("<a><b>test</b><c/></a>");
    r.trim_text(true).expand_empty_elements(false);
    next_eq!(r, Start, b"a", Start, b"b", Text, b"test", End, b"b", Empty, b"c", End, b"a");
}

#[test]
fn test_writer() -> Result<()> {
    let txt = include_str!("../tests/documents/test_writer.xml").trim();
    let mut reader = Reader::from_str(txt);
    reader.trim_text(true);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf)? {
            Eof => break,
            e => assert!(writer.write_event(e).is_ok()),
        }
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(result, txt.as_bytes());
    Ok(())
}

#[test]
fn test_writer_borrow() -> Result<()> {
    let txt = include_str!("../tests/documents/test_writer.xml").trim();
    let mut reader = Reader::from_str(txt);
    reader.trim_text(true);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf)? {
            Eof => break,
            e => assert!(writer.write_event(&e).is_ok()), // either `e` or `&e`
        }
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(result, txt.as_bytes());
    Ok(())
}

#[test]
fn test_writer_indent() -> Result<()> {
    let txt = include_str!("../tests/documents/test_writer_indent.xml");
    // Normalize newlines on Windows to just \n, which is what the reader and
    // writer use.
    let normalized_txt = txt.replace("\r\n", "\n");
    let txt = normalized_txt.as_str();
    let mut reader = Reader::from_str(txt);
    reader.trim_text(true);
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 4);
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf)? {
            Eof => break,
            e => assert!(writer.write_event(e).is_ok()),
        }
    }

    let result = writer.into_inner().into_inner();
    // println!("{:?}", String::from_utf8_lossy(&result));

    #[cfg(windows)]
    assert!(result.into_iter().eq(txt.bytes().filter(|b| *b != 13)));

    #[cfg(not(windows))]
    assert_eq!(result, txt.as_bytes());

    Ok(())
}

#[test]
fn test_writer_indent_cdata() -> Result<()> {
    let txt = include_str!("../tests/documents/test_writer_indent_cdata.xml");
    let mut reader = Reader::from_str(txt);
    reader.trim_text(true);
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 4);
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf)? {
            Eof => break,
            e => assert!(writer.write_event(e).is_ok()),
        }
    }

    let result = writer.into_inner().into_inner();

    #[cfg(windows)]
    assert!(result.into_iter().eq(txt.bytes().filter(|b| *b != 13)));

    #[cfg(not(windows))]
    assert_eq!(result, txt.as_bytes());

    Ok(())
}

#[test]
fn test_write_empty_element_attrs() -> Result<()> {
    let str_from = r#"<source attr="val"/>"#;
    let expected = r#"<source attr="val"/>"#;
    let mut reader = Reader::from_str(str_from);
    reader.expand_empty_elements(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf)? {
            Eof => break,
            e => assert!(writer.write_event(e).is_ok()),
        }
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(String::from_utf8(result).unwrap(), expected);
    Ok(())
}

#[test]
fn test_write_attrs() -> Result<()> {
    type AttrResult<T> = std::result::Result<T, AttrError>;

    let str_from = r#"<source attr="val"></source>"#;
    let expected = r#"<copy attr="val" a="b" c="d" x="y&quot;z"></copy>"#;
    let mut reader = Reader::from_str(str_from);
    reader.trim_text(true);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    loop {
        let event = match reader.read_event(&mut buf)? {
            Eof => break,
            Start(elem) => {
                let mut attrs = elem.attributes().collect::<AttrResult<Vec<_>>>()?;
                attrs.extend_from_slice(&[("a", "b").into(), ("c", "d").into()]);
                let mut elem = BytesStart::owned(b"copy".to_vec(), 4);
                elem.extend_attributes(attrs);
                elem.push_attribute(("x", "y\"z"));
                Start(elem)
            }
            End(_) => End(BytesEnd::borrowed(b"copy")),
            e => e,
        };
        assert!(writer.write_event(event).is_ok());
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(result, expected.as_bytes());

    Ok(())
}

#[test]
fn test_new_xml_decl_full() {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(Decl(BytesDecl::new(b"1.2", Some(b"utf-X"), Some(b"yo"))))
        .expect("writing xml decl should succeed");

    let result = writer.into_inner();
    assert_eq!(
        String::from_utf8(result).expect("utf-8 output"),
        "<?xml version=\"1.2\" encoding=\"utf-X\" standalone=\"yo\"?>".to_owned(),
        "writer output (LHS)"
    );
}

#[test]
fn test_new_xml_decl_standalone() {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(Decl(BytesDecl::new(b"1.2", None, Some(b"yo"))))
        .expect("writing xml decl should succeed");

    let result = writer.into_inner();
    assert_eq!(
        String::from_utf8(result).expect("utf-8 output"),
        "<?xml version=\"1.2\" standalone=\"yo\"?>".to_owned(),
        "writer output (LHS)"
    );
}

#[test]
fn test_new_xml_decl_encoding() {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(Decl(BytesDecl::new(b"1.2", Some(b"utf-X"), None)))
        .expect("writing xml decl should succeed");

    let result = writer.into_inner();
    assert_eq!(
        String::from_utf8(result).expect("utf-8 output"),
        "<?xml version=\"1.2\" encoding=\"utf-X\"?>".to_owned(),
        "writer output (LHS)"
    );
}

#[test]
fn test_new_xml_decl_version() {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(Decl(BytesDecl::new(b"1.2", None, None)))
        .expect("writing xml decl should succeed");

    let result = writer.into_inner();
    assert_eq!(
        String::from_utf8(result).expect("utf-8 output"),
        "<?xml version=\"1.2\"?>".to_owned(),
        "writer output (LHS)"
    );
}

/// This test ensures that empty XML declaration attribute values are not a problem.
#[test]
fn test_new_xml_decl_empty() {
    let mut writer = Writer::new(Vec::new());
    // An empty version should arguably be an error, but we don't expect anyone to actually supply
    // an empty version.
    writer
        .write_event(Decl(BytesDecl::new(b"", Some(b""), Some(b""))))
        .expect("writing xml decl should succeed");

    let result = writer.into_inner();
    assert_eq!(
        String::from_utf8(result).expect("utf-8 output"),
        "<?xml version=\"\" encoding=\"\" standalone=\"\"?>".to_owned(),
        "writer output (LHS)"
    );
}

#[test]
fn test_buf_position_err_end_element() {
    let mut r = Reader::from_str("</a>");
    r.trim_text(true).check_end_names(true);

    let mut buf = Vec::new();
    match r.read_event(&mut buf) {
        Err(_) if r.buffer_position() == 2 => (), // error at char 2: no opening tag
        Err(e) => panic!(
            "expecting buf_pos = 2, found {}, err: {:?}",
            r.buffer_position(),
            e
        ),
        e => panic!("expecting error, found {:?}", e),
    }
}

#[test]
fn test_buf_position_err_comment() {
    let mut r = Reader::from_str("<a><!--b>");
    r.trim_text(true).check_end_names(true);

    next_eq!(r, Start, b"a");
    assert_eq!(r.buffer_position(), 3);

    let mut buf = Vec::new();
    match r.read_event(&mut buf) {
        // error at char 4: no closing --> tag found
        Err(e) => assert_eq!(
            r.buffer_position(),
            4,
            "expecting buf_pos = 4, found {}, err {:?}",
            r.buffer_position(),
            e
        ),
        e => assert!(false, "expecting error, found {:?}", e),
    }
}

#[test]
fn test_buf_position_err_comment_2_buf() {
    let mut r = Reader::from_str("<a><!--b>");
    r.trim_text(true).check_end_names(true);

    let mut buf = Vec::new();
    let _ = r.read_event(&mut buf).unwrap();
    assert_eq!(r.buffer_position(), 3);

    let mut buf = Vec::new();
    match r.read_event(&mut buf) {
        // error at char 4: no closing --> tag found
        Err(e) => assert_eq!(
            r.buffer_position(),
            4,
            "expecting buf_pos = 4, found {}, err {:?}",
            r.buffer_position(),
            e
        ),
        e => assert!(false, "expecting error, found {:?}", e),
    }
}

#[test]
fn test_buf_position_err_comment_trim_text() {
    let mut r = Reader::from_str("<a>\r\n <!--b>");
    r.trim_text(true).check_end_names(true);

    next_eq!(r, Start, b"a");
    assert_eq!(r.buffer_position(), 3);

    let mut buf = Vec::new();
    match r.read_event(&mut buf) {
        // error at char 7: no closing --> tag found
        Err(e) => assert_eq!(
            r.buffer_position(),
            7,
            "expecting buf_pos = 7, found {}, err {:?}",
            r.buffer_position(),
            e
        ),
        e => assert!(false, "expecting error, found {:?}", e),
    }
}

#[test]
fn test_escaped_content() {
    let mut r = Reader::from_str("<a>&lt;test&gt;</a>");
    r.trim_text(true);
    next_eq!(r, Start, b"a");
    let mut buf = Vec::new();
    match r.read_event(&mut buf) {
        Ok(Text(e)) => {
            assert_eq!(
                &*e,
                b"&lt;test&gt;",
                "content unexpected: expecting '&lt;test&gt;', got '{:?}'",
                from_utf8(&*e)
            );
            match e.unescaped() {
                Ok(ref c) => assert_eq!(
                    &**c,
                    b"<test>",
                    "unescaped content unexpected: expecting '&lt;test&lt;', got '{:?}'",
                    from_utf8(c)
                ),
                Err(e) => panic!(
                    "cannot escape content at position {}: {:?}",
                    r.buffer_position(),
                    e
                ),
            }
        }
        Ok(e) => panic!("Expecting text event, got {:?}", e),
        Err(e) => panic!(
            "Cannot get next event at position {}: {:?}",
            r.buffer_position(),
            e
        ),
    }
    next_eq!(r, End, b"a");
}

#[test]
fn test_read_write_roundtrip_results_in_identity() -> Result<()> {
    let input = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <section ns:label="header">
            <section ns:label="empty element section" />
            <section ns:label="start/end section"></section>
            <section ns:label="with text">data</section>
            </section>
    "#;

    let mut reader = Reader::from_str(input);
    reader.trim_text(false).expand_empty_elements(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf)? {
            Eof => break,
            e => assert!(writer.write_event(e).is_ok()),
        }
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(result, input.as_bytes());
    Ok(())
}

#[test]
fn test_read_write_roundtrip() -> Result<()> {
    let input = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <section ns:label="header">
            <section ns:label="empty element section" />
            <section ns:label="start/end section"></section>
            <section ns:label="with text">data &lt;escaped&gt;</section>
            </section>
    "#;

    let mut reader = Reader::from_str(input);
    reader.trim_text(false).expand_empty_elements(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf)? {
            Eof => break,
            e => assert!(writer.write_event(e).is_ok()),
        }
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(String::from_utf8(result).unwrap(), input.to_string());
    Ok(())
}

#[test]
fn test_read_write_roundtrip_escape() -> Result<()> {
    let input = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <section ns:label="header">
            <section ns:label="empty element section" />
            <section ns:label="start/end section"></section>
            <section ns:label="with text">data &lt;escaped&gt;</section>
            </section>
    "#;

    let mut reader = Reader::from_str(input);
    reader.trim_text(false).expand_empty_elements(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf)? {
            Eof => break,
            Text(e) => {
                let t = e.escaped();
                assert!(writer
                    .write_event(Event::Text(BytesText::from_escaped(t.to_vec())))
                    .is_ok());
            }
            e => assert!(writer.write_event(e).is_ok()),
        }
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(String::from_utf8(result).unwrap(), input.to_string());
    Ok(())
}

#[test]
fn test_read_write_roundtrip_escape_text() -> Result<()> {
    let input = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <section ns:label="header">
            <section ns:label="empty element section" />
            <section ns:label="start/end section"></section>
            <section ns:label="with text">data &lt;escaped&gt;</section>
            </section>
    "#;

    let mut reader = Reader::from_str(input);
    reader.trim_text(false).expand_empty_elements(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf)? {
            Eof => break,
            Text(e) => {
                let t = e.unescape_and_decode(&reader).unwrap();
                assert!(writer
                    .write_event(Event::Text(BytesText::from_plain_str(&t)))
                    .is_ok());
            }
            e => assert!(writer.write_event(e).is_ok()),
        }
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(String::from_utf8(result).unwrap(), input.to_string());
    Ok(())
}

#[test]
fn test_closing_bracket_in_single_quote_attr() {
    let mut r = Reader::from_str("<a attr='>' check='2'></a>");
    r.trim_text(true);
    let mut buf = Vec::new();
    match r.read_event(&mut buf) {
        Ok(Start(e)) => {
            let mut attrs = e.attributes();
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"attr"),
                    value: Cow::Borrowed(b">"),
                }))
            );
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"check"),
                    value: Cow::Borrowed(b"2"),
                }))
            );
            assert_eq!(attrs.next(), None);
        }
        x => panic!("expected <a attr='>'>, got {:?}", x),
    }
    next_eq!(r, End, b"a");
}

#[test]
fn test_closing_bracket_in_double_quote_attr() {
    let mut r = Reader::from_str(r#"<a attr=">" check="2"></a>"#);
    r.trim_text(true);
    let mut buf = Vec::new();
    match r.read_event(&mut buf) {
        Ok(Start(e)) => {
            let mut attrs = e.attributes();
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"attr"),
                    value: Cow::Borrowed(b">"),
                }))
            );
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"check"),
                    value: Cow::Borrowed(b"2"),
                }))
            );
            assert_eq!(attrs.next(), None);
        }
        x => panic!("expected <a attr='>'>, got {:?}", x),
    }
    next_eq!(r, End, b"a");
}

#[test]
fn test_closing_bracket_in_double_quote_mixed() {
    let mut r = Reader::from_str(r#"<a attr="'>'" check="'2'"></a>"#);
    r.trim_text(true);
    let mut buf = Vec::new();
    match r.read_event(&mut buf) {
        Ok(Start(e)) => {
            let mut attrs = e.attributes();
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"attr"),
                    value: Cow::Borrowed(b"'>'"),
                }))
            );
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"check"),
                    value: Cow::Borrowed(b"'2'"),
                }))
            );
            assert_eq!(attrs.next(), None);
        }
        x => panic!("expected <a attr='>'>, got {:?}", x),
    }
    next_eq!(r, End, b"a");
}

#[test]
fn test_closing_bracket_in_single_quote_mixed() {
    let mut r = Reader::from_str(r#"<a attr='">"' check='"2"'></a>"#);
    r.trim_text(true);
    let mut buf = Vec::new();
    match r.read_event(&mut buf) {
        Ok(Start(e)) => {
            let mut attrs = e.attributes();
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"attr"),
                    value: Cow::Borrowed(br#"">""#),
                }))
            );
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"check"),
                    value: Cow::Borrowed(br#""2""#),
                }))
            );
            assert_eq!(attrs.next(), None);
        }
        x => panic!("expected <a attr='>'>, got {:?}", x),
    }
    next_eq!(r, End, b"a");
}

mod decode_with_bom_removal {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    #[cfg(not(feature = "encoding"))]
    fn removes_utf8_bom() {
        let input: &str = std::str::from_utf8(b"\xEF\xBB\xBF<?xml version=\"1.0\"?>").unwrap();

        let mut reader = Reader::from_str(&input);
        reader.trim_text(true);

        let mut txt = Vec::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::StartText(e)) => {
                    txt.push(e.decode_with_bom_removal(reader.decoder()).unwrap())
                }
                Ok(Event::Eof) => break,
                _ => (),
            }
        }
        assert_eq!(txt, vec![""]);
    }

    /// Test is disabled: the input started with `[FE FF 00 3C 00 3F ...]` and currently
    /// quick-xml captures `[FE FF 00]` as a `StartText` event, because it is stopped
    /// at byte `<` (0x3C). That sequence represents UTF-16 BOM (=BE) and a first byte
    /// of the `<` symbol, encoded in UTF-16 BE (`00 3C`).
    #[test]
    #[cfg(feature = "encoding")]
    #[ignore = "Non-ASCII compatible encodings not properly supported yet. See https://github.com/tafia/quick-xml/issues/158"]
    fn removes_utf16be_bom() {
        let mut reader = Reader::from_file("./tests/documents/utf16be.xml").unwrap();
        reader.trim_text(true);

        let mut txt = Vec::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::StartText(e)) => {
                    txt.push(e.decode_with_bom_removal(reader.decoder()).unwrap())
                }
                Ok(Event::Eof) => break,
                _ => (),
            }
        }
        assert_eq!(Some(txt[0].as_ref()), Some(""));
    }

    #[test]
    #[cfg(feature = "encoding")]
    fn removes_utf16le_bom() {
        let mut reader = Reader::from_file("./tests/documents/utf16le.xml").unwrap();
        reader.trim_text(true);

        let mut txt = Vec::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::StartText(e)) => {
                    txt.push(e.decode_with_bom_removal(reader.decoder()).unwrap())
                }
                Ok(Event::Eof) => break,
                _ => (),
            }
        }
        assert_eq!(Some(txt[0].as_ref()), Some(""));
    }

    #[test]
    #[cfg(not(feature = "encoding"))]
    fn does_nothing_if_no_bom_exists() {
        let input: &str = std::str::from_utf8(b"<?xml version=\"1.0\"?>").unwrap();

        let mut reader = Reader::from_str(&input);
        reader.trim_text(true);

        let mut txt = Vec::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::StartText(e)) => {
                    txt.push(e.decode_with_bom_removal(reader.decoder()).unwrap())
                }
                Ok(Event::Eof) => break,
                _ => (),
            }
        }
        assert!(txt.is_empty());
    }
}

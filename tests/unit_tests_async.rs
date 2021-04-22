#[cfg(feature = "asynchronous")]
use quick_xml::events::Event::*;
#[cfg(feature = "asynchronous")]
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
#[cfg(feature = "asynchronous")]
use quick_xml::{AsyncReader, Result, Writer};
#[cfg(feature = "asynchronous")]
use std::io::Cursor;
#[cfg(feature = "asynchronous")]
use std::str::from_utf8;

#[cfg(feature = "asynchronous")]
macro_rules! next_eq_name {
    ($r:expr, $t:tt, $bytes:expr) => {
        let mut buf = Vec::new();

        match $r.read_event(&mut buf).await.unwrap() {
            $t(ref e) if e.name() == $bytes => (),
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

#[cfg(feature = "asynchronous")]
macro_rules! next_eq_content {
    ($r:expr, $t:tt, $bytes:expr) => {
        let mut buf = Vec::new();

        match $r.read_event(&mut buf).await.unwrap() {
            $t(ref e) if &**e == $bytes => (),
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

#[cfg(feature = "asynchronous")]
macro_rules! next_eq {
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

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_start() {
    let mut r = AsyncReader::from_str("<a>");
    r.trim_text(true);
    next_eq!(r, Start, b"a");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_start_end() {
    let mut r = AsyncReader::from_str("<a></a>");
    r.trim_text(true);
    next_eq!(r, Start, b"a", End, b"a");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_start_end_with_ws() {
    let mut r = AsyncReader::from_str("<a></a >");
    r.trim_text(true);
    next_eq!(r, Start, b"a", End, b"a");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_start_end_attr() {
    let mut r = AsyncReader::from_str("<a b=\"test\"></a>");
    r.trim_text(true);
    next_eq!(r, Start, b"a", End, b"a");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_empty() {
    let mut r = AsyncReader::from_str("<a />");
    r.trim_text(true).expand_empty_elements(false);
    next_eq!(r, Empty, b"a");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_empty_can_be_expanded() {
    let mut r = AsyncReader::from_str("<a />");
    r.trim_text(true).expand_empty_elements(true);
    next_eq!(r, Start, b"a", End, b"a");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_empty_attr() {
    let mut r = AsyncReader::from_str("<a b=\"test\" />");
    r.trim_text(true).expand_empty_elements(false);
    next_eq!(r, Empty, b"a");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_start_end_comment() {
    let mut r = AsyncReader::from_str("<b><a b=\"test\" c=\"test\"/> <a  /><!--t--></b>");
    r.trim_text(true).expand_empty_elements(false);
    next_eq!(r, Start, b"b", Empty, b"a", Empty, b"a", Comment, b"t", End, b"b");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_start_txt_end() {
    let mut r = AsyncReader::from_str("<a>test</a>");
    r.trim_text(true);
    next_eq!(r, Start, b"a", Text, b"test", End, b"a");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_comment() {
    let mut r = AsyncReader::from_str("<!--test-->");
    r.trim_text(true);
    next_eq!(r, Comment, b"test");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_xml_decl() {
    let mut r = AsyncReader::from_str("<?xml version=\"1.0\" encoding='utf-8'?>");
    r.trim_text(true);
    let mut buf = Vec::new();

    match r.read_event(&mut buf).await.unwrap() {
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

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_trim_test() {
    let txt = "<a><b>  </b></a>";
    let mut r = AsyncReader::from_str(txt);
    r.trim_text(true);
    next_eq!(r, Start, b"a", Start, b"b", End, b"b", End, b"a");

    let mut r = AsyncReader::from_str(txt);
    r.trim_text(false);
    next_eq!(
        r, Text, b"", Start, b"a", Text, b"", Start, b"b", Text, b"  ", End, b"b", Text, b"", End,
        b"a"
    );
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_cdata() {
    let mut r = AsyncReader::from_str("<![CDATA[test]]>");
    r.trim_text(true);
    next_eq!(r, CData, b"test");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_cdata_open_close() {
    let mut r = AsyncReader::from_str("<![CDATA[test <> test]]>");
    r.trim_text(true);
    next_eq!(r, CData, b"test &lt;&gt; test");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_start_attr() {
    let mut r = AsyncReader::from_str("<a b=\"c\">");
    r.trim_text(true);
    next_eq!(r, Start, b"a");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_nested() {
    let mut r = AsyncReader::from_str("<a><b>test</b><c/></a>");
    r.trim_text(true).expand_empty_elements(false);
    next_eq!(r, Start, b"a", Start, b"b", Text, b"test", End, b"b", Empty, b"c", End, b"a");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_writer() {
    let txt = include_str!("../tests/documents/test_writer.xml").trim();
    let mut reader = AsyncReader::from_str(txt);
    reader.trim_text(true);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf).await {
            Ok(Eof) => break,
            Ok(e) => assert!(writer.write_event(e).is_ok()),
            Err(e) => panic!("{}", e),
        }
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(result, txt.as_bytes());
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_writer_borrow() {
    let txt = include_str!("../tests/documents/test_writer.xml").trim();
    let mut reader = AsyncReader::from_str(txt);
    reader.trim_text(true);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf).await {
            Ok(Eof) => break,
            Ok(e) => assert!(writer.write_event(&e).is_ok()), // either `e` or `&e`
            Err(e) => panic!("{}", e),
        }
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(result, txt.as_bytes());
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_writer_indent() {
    let txt = include_str!("../tests/documents/test_writer_indent.xml");
    let mut reader = AsyncReader::from_str(txt);
    reader.trim_text(true);
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 4);
    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf).await {
            Ok(Eof) => break,
            Ok(e) => assert!(writer.write_event(e).is_ok()),
            Err(e) => panic!("{}", e),
        }
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(result, txt.as_bytes());
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_writer_indent_cdata() {
    let txt = include_str!("../tests/documents/test_writer_indent_cdata.xml");
    let mut reader = AsyncReader::from_str(txt);
    reader.trim_text(true);
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 4);
    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf).await {
            Ok(Eof) => break,
            Ok(e) => assert!(writer.write_event(e).is_ok()),
            Err(e) => panic!("{}", e),
        }
    }

    let result = writer.into_inner().into_inner();

    #[cfg(windows)]
    assert!(result.into_iter().eq(txt.bytes().filter(|b| *b != 13)));

    #[cfg(not(windows))]
    assert_eq!(result, txt.as_bytes());
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_write_empty_element_attrs() {
    let str_from = r#"<source attr="val"/>"#;
    let expected = r#"<source attr="val"/>"#;
    let mut reader = AsyncReader::from_str(str_from);
    reader.expand_empty_elements(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf).await {
            Ok(Eof) => break,
            Ok(e) => assert!(writer.write_event(e).is_ok()),
            Err(e) => panic!("{}", e),
        }
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(String::from_utf8(result).unwrap(), expected);
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_write_attrs() {
    let str_from = r#"<source attr="val"></source>"#;
    let expected = r#"<copy attr="val" a="b" c="d" x="y&quot;z"></copy>"#;
    let mut reader = AsyncReader::from_str(str_from);
    reader.trim_text(true);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();

    loop {
        let event = match reader.read_event(&mut buf).await {
            Ok(Eof) => break,
            Ok(Start(elem)) => {
                let mut attrs = elem.attributes().collect::<Result<Vec<_>>>().unwrap();
                attrs.extend_from_slice(&[("a", "b").into(), ("c", "d").into()]);
                let mut elem = BytesStart::owned(b"copy".to_vec(), 4);
                elem.extend_attributes(attrs);
                elem.push_attribute(("x", "y\"z"));
                Start(elem)
            }
            Ok(End(_)) => End(BytesEnd::borrowed(b"copy")),
            Ok(e) => e,
            Err(e) => panic!("{}", e),
        };
        assert!(writer.write_event(event).is_ok());
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(result, expected.as_bytes());
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_new_xml_decl_full() {
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

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_new_xml_decl_standalone() {
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

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_new_xml_decl_encoding() {
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

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_new_xml_decl_version() {
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
#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_new_xml_decl_empty() {
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

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_buf_position_err_end_element() {
    let mut r = AsyncReader::from_str("</a>");
    r.trim_text(true).check_end_names(true);

    let mut buf = Vec::new();

    match r.read_event(&mut buf).await {
        Err(_) if r.buffer_position() == 2 => (), // error at char 2: no opening tag
        Err(e) => panic!(
            "expecting buf_pos = 2, found {}, err: {:?}",
            r.buffer_position(),
            e
        ),
        e => panic!("expecting error, found {:?}", e),
    }
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_buf_position_err_comment() {
    let mut r = AsyncReader::from_str("<a><!--b>");
    r.trim_text(true).check_end_names(true);

    next_eq!(r, Start, b"a");
    assert_eq!(r.buffer_position(), 3);

    let mut buf = Vec::new();

    match r.read_event(&mut buf).await {
        Err(_) if r.buffer_position() == 4 => {
            // error at char 5: no closing --> tag found
            assert!(true);
        }
        Err(e) => panic!(
            "expecting buf_pos = 5, found {}, err: {:?}",
            r.buffer_position(),
            e
        ),
        e => assert!(false, "expecting error, found {:?}", e),
    }
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_buf_position_err_comment_2_buf() {
    let mut r = AsyncReader::from_str("<a><!--b>");
    r.trim_text(true).check_end_names(true);

    let mut buf = Vec::new();

    let _ = r.read_event(&mut buf).await;

    assert_eq!(r.buffer_position(), 3);

    let mut buf = Vec::new();

    match r.read_event(&mut buf).await {
        Err(_) if r.buffer_position() == 4 => {
            // error at char 5: no closing --> tag found
            assert!(true);
        }
        Err(e) => panic!(
            "expecting buf_pos = 4, found {}, err: {:?}",
            r.buffer_position(),
            e
        ),
        e => assert!(false, "expecting error, found {:?}", e),
    }
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_buf_position_err_comment_trim_text() {
    let mut r = AsyncReader::from_str("<a>\r\n <!--b>");
    r.trim_text(true).check_end_names(true);

    next_eq!(r, Start, b"a");
    assert_eq!(r.buffer_position(), 3);

    let mut buf = Vec::new();

    match r.read_event(&mut buf).await {
        Err(_) if r.buffer_position() == 7 => {
            // error at char 5: no closing --> tag found
            assert!(true);
        }
        Err(e) => panic!(
            "expecting buf_pos = 5, found {}, err: {:?}",
            r.buffer_position(),
            e
        ),
        e => assert!(false, "expecting error, found {:?}", e),
    }
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_namespace() {
    let mut r = AsyncReader::from_str("<a xmlns:myns='www1'><myns:b>in namespace!</myns:b></a>");
    r.trim_text(true);

    let mut buf = Vec::new();
    let mut ns_buf = Vec::new();

    if let Ok((None, Start(_))) = r.read_namespaced_event(&mut buf, &mut ns_buf).await {
    } else {
        assert!(false, "expecting start element with no namespace");
    }

    if let Ok((Some(a), Start(_))) = r.read_namespaced_event(&mut buf, &mut ns_buf).await {
        if &*a == b"www1" {
            assert!(true);
        } else {
            assert!(false, "expecting namespace to resolve to 'www1'");
        }
    } else {
        assert!(false, "expecting namespace resolution");
    }
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_default_namespace() {
    let mut r = AsyncReader::from_str("<a ><b xmlns=\"www1\"></b></a>");
    r.trim_text(true);

    let mut buf = Vec::new();
    let mut ns_buf = Vec::new();

    // <a>
    if let Ok((None, Start(_))) = r.read_namespaced_event(&mut buf, &mut ns_buf).await {
    } else {
        assert!(false, "expecting outer start element with no namespace");
    }

    // <b>
    if let Ok((Some(a), Start(_))) = r.read_namespaced_event(&mut buf, &mut ns_buf).await {
        if &*a == b"www1" {
            assert!(true);
        } else {
            assert!(false, "expecting namespace to resolve to 'www1'");
        }
    } else {
        assert!(false, "expecting namespace resolution");
    }

    // </b>
    if let Ok((Some(a), End(_))) = r.read_namespaced_event(&mut buf, &mut ns_buf).await {
        if &*a == b"www1" {
            assert!(true);
        } else {
            assert!(false, "expecting namespace to resolve to 'www1'");
        }
    } else {
        assert!(false, "expecting namespace resolution");
    }

    // </a> very important: a should not be in any namespace. The default namespace only applies to
    // the sub-document it is defined on.
    if let Ok((None, End(_))) = r.read_namespaced_event(&mut buf, &mut ns_buf).await {
    } else {
        assert!(false, "expecting outer end element with no namespace");
    }
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_default_namespace_reset() {
    let mut r = AsyncReader::from_str("<a xmlns=\"www1\"><b xmlns=\"\"></b></a>");
    r.trim_text(true);

    let mut buf = Vec::new();
    let mut ns_buf = Vec::new();

    if let Ok((Some(a), Start(_))) = r.read_namespaced_event(&mut buf, &mut ns_buf).await {
        assert_eq!(
            &a[..],
            b"www1",
            "expecting outer start element with to resolve to 'www1'"
        );
    } else {
        panic!("expecting outer start element with to resolve to 'www1'");
    }

    match r.read_namespaced_event(&mut buf, &mut ns_buf).await {
        Ok((None, Start(_))) => (),
        e => panic!("expecting inner start element, got {:?}", e),
    }

    if let Ok((None, End(_))) = r.read_namespaced_event(&mut buf, &mut ns_buf).await {
    } else {
        assert!(false, "expecting inner end element");
    }

    if let Ok((Some(a), End(_))) = r.read_namespaced_event(&mut buf, &mut ns_buf).await {
        assert_eq!(
            &a[..],
            b"www1",
            "expecting outer end element with to resolve to 'www1'"
        );
    } else {
        panic!("expecting outer end element with to resolve to 'www1'");
    }
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_escaped_content() {
    let mut r = AsyncReader::from_str("<a>&lt;test&gt;</a>");
    r.trim_text(true);
    next_eq!(r, Start, b"a");
    let mut buf = Vec::new();

    match r.read_event(&mut buf).await {
        Ok(Text(e)) => {
            if &*e != b"&lt;test&gt;" {
                panic!(
                    "content unexpected: expecting '&lt;test&gt;', got '{:?}'",
                    from_utf8(&*e)
                );
            }
            match e.unescaped() {
                Ok(ref c) => {
                    if &**c != b"<test>" {
                        panic!(
                            "unescaped content unexpected: expecting '&lt;test&lt;', got '{:?}'",
                            from_utf8(c)
                        )
                    }
                }
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

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_read_write_roundtrip_results_in_identity() {
    let input = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <section ns:label="header">
            <section ns:label="empty element section" />
            <section ns:label="start/end section"></section>
            <section ns:label="with text">data</section>
            </section>
    "#;

    let mut reader = AsyncReader::from_str(input);
    reader.trim_text(false).expand_empty_elements(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf).await {
            Ok(Eof) => break,
            Ok(e) => assert!(writer.write_event(e).is_ok()),
            Err(e) => panic!("{}", e),
        }
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(result, input.as_bytes());
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_read_write_roundtrip() {
    let input = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <section ns:label="header">
            <section ns:label="empty element section" />
            <section ns:label="start/end section"></section>
            <section ns:label="with text">data &lt;escaped&gt;</section>
            </section>
    "#;

    let mut reader = AsyncReader::from_str(input);
    reader.trim_text(false).expand_empty_elements(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf).await {
            Ok(Eof) => break,
            Ok(e) => assert!(writer.write_event(e).is_ok()),
            Err(e) => panic!("{}", e),
        }
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(String::from_utf8(result).unwrap(), input.to_string());
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_read_write_roundtrip_escape() {
    let input = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <section ns:label="header">
            <section ns:label="empty element section" />
            <section ns:label="start/end section"></section>
            <section ns:label="with text">data &lt;escaped&gt;</section>
            </section>
    "#;

    let mut reader = AsyncReader::from_str(input);
    reader.trim_text(false).expand_empty_elements(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf).await {
            Ok(Eof) => break,
            Ok(Text(e)) => {
                let t = e.escaped();
                assert!(writer
                    .write_event(Event::Text(BytesText::from_escaped(t.to_vec())))
                    .is_ok());
            }
            Ok(e) => assert!(writer.write_event(e).is_ok()),
            Err(e) => panic!("{}", e),
        }
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(String::from_utf8(result).unwrap(), input.to_string());
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_read_write_roundtrip_escape_text() {
    let input = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <section ns:label="header">
            <section ns:label="empty element section" />
            <section ns:label="start/end section"></section>
            <section ns:label="with text">data &lt;escaped&gt;</section>
            </section>
    "#;

    let mut reader = AsyncReader::from_str(input);
    reader.trim_text(false).expand_empty_elements(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf).await {
            Ok(Eof) => break,
            Ok(Text(e)) => {
                let t = e.unescape_and_decode(&reader).unwrap();
                assert!(writer
                    .write_event(Event::Text(BytesText::from_plain_str(&t)))
                    .is_ok());
            }
            Ok(e) => assert!(writer.write_event(e).is_ok()),
            Err(e) => panic!("{}", e),
        }
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(String::from_utf8(result).unwrap(), input.to_string());
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_closing_bracket_in_single_quote_attr() {
    let mut r = AsyncReader::from_str("<a attr='>' check='2'></a>");
    r.trim_text(true);
    let mut buf = Vec::new();

    match r.read_event(&mut buf).await {
        Ok(Start(e)) => {
            let mut attrs = e.attributes();
            match attrs.next() {
                Some(Ok(attr)) => assert_eq!(attr, ("attr".as_bytes(), ">".as_bytes()).into()),
                x => panic!("expected attribute 'attr', got {:?}", x),
            }
            match attrs.next() {
                Some(Ok(attr)) => assert_eq!(attr, ("check".as_bytes(), "2".as_bytes()).into()),
                x => panic!("expected attribute 'check', got {:?}", x),
            }
            assert!(attrs.next().is_none(), "expected only two attributes");
        }
        x => panic!("expected <a attr='>'>, got {:?}", x),
    }
    next_eq!(r, End, b"a");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_closing_bracket_in_double_quote_attr() {
    let mut r = AsyncReader::from_str("<a attr=\">\" check=\"2\"></a>");
    r.trim_text(true);
    let mut buf = Vec::new();

    match r.read_event(&mut buf).await {
        Ok(Start(e)) => {
            let mut attrs = e.attributes();
            match attrs.next() {
                Some(Ok(attr)) => assert_eq!(attr, ("attr".as_bytes(), ">".as_bytes()).into()),
                x => panic!("expected attribute 'attr', got {:?}", x),
            }
            match attrs.next() {
                Some(Ok(attr)) => assert_eq!(attr, ("check".as_bytes(), "2".as_bytes()).into()),
                x => panic!("expected attribute 'check', got {:?}", x),
            }
            assert!(attrs.next().is_none(), "expected only two attributes");
        }
        x => panic!("expected <a attr='>'>, got {:?}", x),
    }
    next_eq!(r, End, b"a");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_closing_bracket_in_double_quote_mixed() {
    let mut r = AsyncReader::from_str("<a attr=\"'>'\" check=\"'2'\"></a>");
    r.trim_text(true);
    let mut buf = Vec::new();

    match r.read_event(&mut buf).await {
        Ok(Start(e)) => {
            let mut attrs = e.attributes();
            match attrs.next() {
                Some(Ok(attr)) => assert_eq!(attr, ("attr".as_bytes(), "'>'".as_bytes()).into()),
                x => panic!("expected attribute 'attr', got {:?}", x),
            }
            match attrs.next() {
                Some(Ok(attr)) => assert_eq!(attr, ("check".as_bytes(), "'2'".as_bytes()).into()),
                x => panic!("expected attribute 'check', got {:?}", x),
            }
            assert!(attrs.next().is_none(), "expected only two attributes");
        }
        x => panic!("expected <a attr='>'>, got {:?}", x),
    }
    next_eq!(r, End, b"a");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
async fn test_closing_bracket_in_single_quote_mixed() {
    let mut r = AsyncReader::from_str("<a attr='\">\"' check='\"2\"'></a>");
    r.trim_text(true);
    let mut buf = Vec::new();

    match r.read_event(&mut buf).await {
        Ok(Start(e)) => {
            let mut attrs = e.attributes();
            match attrs.next() {
                Some(Ok(attr)) => assert_eq!(attr, ("attr".as_bytes(), "\">\"".as_bytes()).into()),
                x => panic!("expected attribute 'attr', got {:?}", x),
            }
            match attrs.next() {
                Some(Ok(attr)) => assert_eq!(attr, ("check".as_bytes(), "\"2\"".as_bytes()).into()),
                x => panic!("expected attribute 'check', got {:?}", x),
            }
            assert!(attrs.next().is_none(), "expected only two attributes");
        }
        x => panic!("expected <a attr='>'>, got {:?}", x),
    }
    next_eq!(r, End, b"a");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
#[cfg(not(feature = "encoding"))]
async fn test_unescape_and_decode_without_bom_removes_utf8_bom() {
    let input: &str = std::str::from_utf8(b"\xEF\xBB\xBF<?xml version=\"1.0\"?>").unwrap();

    let mut reader = AsyncReader::from_str(&input);
    reader.trim_text(true);

    let mut txt = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf).await {
            Ok(Event::Text(e)) => txt.push(e.unescape_and_decode_without_bom(&reader).unwrap()),
            Ok(Event::Eof) => break,
            _ => (),
        }
    }
    assert_eq!(txt, vec![""]);
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
#[cfg(feature = "encoding")]
async fn test_unescape_and_decode_without_bom_removes_utf16be_bom() {
    let mut reader = AsyncReader::from_file("./tests/documents/utf16be.xml")
        .await
        .unwrap();
    reader.trim_text(true);

    let mut txt = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf).await {
            Ok(Event::Text(e)) => txt.push(e.unescape_and_decode_without_bom(&mut reader).unwrap()),
            Ok(Event::Eof) => break,
            _ => (),
        }
    }
    assert_eq!(txt[0], "");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
#[cfg(feature = "encoding")]
async fn test_unescape_and_decode_without_bom_removes_utf16le_bom() {
    let mut reader = AsyncReader::from_file("./tests/documents/utf16le.xml")
        .await
        .unwrap();
    reader.trim_text(true);

    let mut txt = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf).await {
            Ok(Event::Text(e)) => txt.push(e.unescape_and_decode_without_bom(&mut reader).unwrap()),
            Ok(Event::Eof) => break,
            _ => (),
        }
    }
    assert_eq!(txt[0], "");
}

#[cfg(feature = "asynchronous")]
#[tokio::test]
#[cfg(not(feature = "encoding"))]
async fn test_unescape_and_decode_without_bom_does_nothing_if_no_bom_exists() {
    let input: &str = std::str::from_utf8(b"<?xml version=\"1.0\"?>").unwrap();

    let mut reader = AsyncReader::from_str(&input);
    reader.trim_text(true);

    let mut txt = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf).await {
            Ok(Event::Text(e)) => txt.push(e.unescape_and_decode_without_bom(&mut reader).unwrap()),
            Ok(Event::Eof) => break,
            _ => (),
        }
    }
    assert_eq!(txt.is_empty(), true);
}

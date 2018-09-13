extern crate quick_xml;

use quick_xml::events::attributes::Attribute;
use quick_xml::events::Event::*;
use quick_xml::Reader;
use std::borrow::Cow;
use std::io::Cursor;

#[test]
fn test_sample() {
    let src: &[u8] = include_bytes!("sample_rss.xml");
    let mut buf = Vec::new();
    let mut r = Reader::from_reader(src);
    let mut count = 0;
    loop {
        match r.read_event(&mut buf).unwrap() {
            Start(_) => count += 1,
            Decl(e) => println!("{:?}", e.version()),
            Eof => break,
            _ => (),
        }
        buf.clear();
    }
    println!("{}", count);
}

#[test]
fn test_attributes_empty() {
    let src = b"<a att1='a' att2='b'/>";
    let mut r = Reader::from_reader(src as &[u8]);
    r.trim_text(true).expand_empty_elements(false);
    let mut buf = Vec::new();
    match r.read_event(&mut buf) {
        Ok(Empty(e)) => {
            let mut atts = e.attributes();
            match atts.next() {
                Some(Ok(Attribute {
                    key: b"att1",
                    value: Cow::Borrowed(b"a"),
                })) => (),
                e => panic!("Expecting att1='a' attribute, found {:?}", e),
            }
            match atts.next() {
                Some(Ok(Attribute {
                    key: b"att2",
                    value: Cow::Borrowed(b"b"),
                })) => (),
                e => panic!("Expecting att2='b' attribute, found {:?}", e),
            }
            match atts.next() {
                None => (),
                e => panic!("Expecting None, found {:?}", e),
            }
        }
        e => panic!("Expecting Empty event, got {:?}", e),
    }
}

#[test]
fn test_attribute_equal() {
    let src = b"<a att1=\"a=b\"/>";
    let mut r = Reader::from_reader(src as &[u8]);
    r.trim_text(true).expand_empty_elements(false);
    let mut buf = Vec::new();
    match r.read_event(&mut buf) {
        Ok(Empty(e)) => {
            let mut atts = e.attributes();
            match atts.next() {
                Some(Ok(Attribute {
                    key: b"att1",
                    value: Cow::Borrowed(b"a=b"),
                })) => (),
                e => panic!("Expecting att1=\"a=b\" attribute, found {:?}", e),
            }
            match atts.next() {
                None => (),
                e => panic!("Expecting None, found {:?}", e),
            }
        }
        e => panic!("Expecting Empty event, got {:?}", e),
    }
}

#[test]
fn test_comment_starting_with_gt() {
    let src = b"<a /><!-->-->";
    let mut r = Reader::from_reader(src as &[u8]);
    r.trim_text(true).expand_empty_elements(false);
    let mut buf = Vec::new();
    loop {
        match r.read_event(&mut buf) {
            Ok(Comment(ref e)) if &**e == b">" => break,
            Ok(Eof) => panic!("Expecting Comment"),
            _ => (),
        }
    }
}

/// Single empty element with qualified attributes.
/// Empty element expansion: disabled
/// The code path for namespace handling is slightly different for `Empty` vs. `Start+End`.
#[test]
fn test_attributes_empty_ns() {
    let src = b"<a att1='a' r:att2='b' xmlns:r='urn:example:r' />";

    let mut r = Reader::from_reader(src as &[u8]);
    r.trim_text(true).expand_empty_elements(false);
    let mut buf = Vec::new();
    let mut ns_buf = Vec::new();

    let e = match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((None, Empty(e))) => e,
        e => panic!("Expecting Empty event, got {:?}", e),
    };

    let mut atts = e.attributes()
            .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
            // we don't care about xmlns attributes for this test
            .filter(|kv| !kv.key.starts_with(b"xmlns"))
            .map(|Attribute { key: name, value }| {
                let (opt_ns, local_name) = r.resolve_namespace(name, &ns_buf);
                (opt_ns, local_name, value)
            });
    match atts.next() {
        Some((None, b"att1", Cow::Borrowed(b"a"))) => (),
        e => panic!("Expecting att1='a' attribute, found {:?}", e),
    }
    match atts.next() {
        Some((Some(ns), b"att2", Cow::Borrowed(b"b"))) => {
            assert_eq!(&ns[..], b"urn:example:r");
        }
        e => panic!(
            "Expecting {{urn:example:r}}att2='b' attribute, found {:?}",
            e
        ),
    }
    match atts.next() {
        None => (),
        e => panic!("Expecting None, found {:?}", e),
    }
}

/// Single empty element with qualified attributes.
/// Empty element expansion: enabled
/// The code path for namespace handling is slightly different for `Empty` vs. `Start+End`.
#[test]
fn test_attributes_empty_ns_expanded() {
    let src = b"<a att1='a' r:att2='b' xmlns:r='urn:example:r' />";

    let mut r = Reader::from_reader(src as &[u8]);
    r.trim_text(true).expand_empty_elements(true);
    let mut buf = Vec::new();
    let mut ns_buf = Vec::new();
    {
        let e = match r.read_namespaced_event(&mut buf, &mut ns_buf) {
            Ok((None, Start(e))) => e,
            e => panic!("Expecting Empty event, got {:?}", e),
        };

        let mut atts = e.attributes()
            .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
            // we don't care about xmlns attributes for this test
            .filter(|kv| !kv.key.starts_with(b"xmlns"))
            .map(|Attribute { key: name, value }| {
                let (opt_ns, local_name) = r.resolve_namespace(name, &ns_buf);
                (opt_ns, local_name, value)
            });
        match atts.next() {
            Some((None, b"att1", Cow::Borrowed(b"a"))) => (),
            e => panic!("Expecting att1='a' attribute, found {:?}", e),
        }
        match atts.next() {
            Some((Some(ns), b"att2", Cow::Borrowed(b"b"))) => {
                assert_eq!(&ns[..], b"urn:example:r");
            }
            e => panic!(
                "Expecting {{urn:example:r}}att2='b' attribute, found {:?}",
                e
            ),
        }
        match atts.next() {
            None => (),
            e => panic!("Expecting None, found {:?}", e),
        }
    }

    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((None, End(e))) => assert_eq!(b"a", e.name()),
        e => panic!("Expecting End event, got {:?}", e),
    }
}

#[test]
fn test_default_ns_shadowing_empty() {
    let src = b"<e xmlns='urn:example:o'><e att1='a' xmlns='urn:example:i' /></e>";

    let mut r = Reader::from_reader(src as &[u8]);
    r.trim_text(true).expand_empty_elements(false);
    let mut buf = Vec::new();
    let mut ns_buf = Vec::new();

    // <outer xmlns='urn:example:o'>
    {
        match r.read_namespaced_event(&mut buf, &mut ns_buf) {
            Ok((Some(ns), Start(e))) => {
                assert_eq!(&ns[..], b"urn:example:o");
                assert_eq!(e.name(), b"e");
            }
            e => panic!("Expected Start event (<outer>), got {:?}", e),
        }
    }

    // <inner att1='a' xmlns='urn:example:i' />
    {
        let e = match r.read_namespaced_event(&mut buf, &mut ns_buf) {
            Ok((Some(ns), Empty(e))) => {
                assert_eq!(::std::str::from_utf8(ns).unwrap(), "urn:example:i");
                assert_eq!(e.name(), b"e");
                e
            }
            e => panic!("Expecting Empty event, got {:?}", e),
        };

        let mut atts = e.attributes()
            .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
            // we don't care about xmlns attributes for this test
            .filter(|kv| !kv.key.starts_with(b"xmlns"))
            .map(|Attribute { key: name, value }| {
                let (opt_ns, local_name) = r.resolve_namespace(name, &ns_buf);
                (opt_ns, local_name, value)
            });
        // the attribute should _not_ have a namespace name. The default namespace does not
        // apply to attributes.
        match atts.next() {
            Some((None, b"att1", Cow::Borrowed(b"a"))) => (),
            e => panic!("Expecting att1='a' attribute, found {:?}", e),
        }
        match atts.next() {
            None => (),
            e => panic!("Expecting None, found {:?}", e),
        }
    }

    // </outer>
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((Some(ns), End(e))) => {
            assert_eq!(&ns[..], b"urn:example:o");
            assert_eq!(e.name(), b"e");
        }
        e => panic!("Expected End event (<outer>), got {:?}", e),
    }
}

#[test]
fn test_default_ns_shadowing_expanded() {
    let src = b"<e xmlns='urn:example:o'><e att1='a' xmlns='urn:example:i' /></e>";

    let mut r = Reader::from_reader(src as &[u8]);
    r.trim_text(true).expand_empty_elements(true);
    let mut buf = Vec::new();
    let mut ns_buf = Vec::new();

    // <outer xmlns='urn:example:o'>
    {
        match r.read_namespaced_event(&mut buf, &mut ns_buf) {
            Ok((Some(ns), Start(e))) => {
                assert_eq!(&ns[..], b"urn:example:o");
                assert_eq!(e.name(), b"e");
            }
            e => panic!("Expected Start event (<outer>), got {:?}", e),
        }
    }
    buf.clear();

    // <inner att1='a' xmlns='urn:example:i' />
    {
        let e = match r.read_namespaced_event(&mut buf, &mut ns_buf) {
            Ok((Some(ns), Start(e))) => {
                assert_eq!(&ns[..], b"urn:example:i");
                assert_eq!(e.name(), b"e");
                e
            }
            e => panic!("Expecting Start event (<inner>), got {:?}", e),
        };
        let mut atts = e.attributes()
            .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
            // we don't care about xmlns attributes for this test
            .filter(|kv| !kv.key.starts_with(b"xmlns"))
            .map(|Attribute { key: name, value }| {
                let (opt_ns, local_name) = r.resolve_namespace(name, &ns_buf);
                (opt_ns, local_name, value)
            });
        // the attribute should _not_ have a namespace name. The default namespace does not
        // apply to attributes.
        match atts.next() {
            Some((None, b"att1", Cow::Borrowed(b"a"))) => (),
            e => panic!("Expecting att1='a' attribute, found {:?}", e),
        }
        match atts.next() {
            None => (),
            e => panic!("Expecting None, found {:?}", e),
        }
    }

    // virtual </inner>
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((Some(ns), End(e))) => {
            assert_eq!(&ns[..], b"urn:example:i");
            assert_eq!(e.name(), b"e");
        }
        e => panic!("Expected End event (</inner>), got {:?}", e),
    }
    // </outer>
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((Some(ns), End(e))) => {
            assert_eq!(&ns[..], b"urn:example:o");
            assert_eq!(e.name(), b"e");
        }
        e => panic!("Expected End event (</outer>), got {:?}", e),
    }
}

#[test]
fn test_koi8_r_encoding() {
    let src: &[u8] = include_bytes!("documents/opennews_all.rss");
    let mut r = Reader::from_reader(src as &[u8]);
    r.trim_text(true).expand_empty_elements(false);
    let mut buf = Vec::new();
    loop {
        match r.read_event(&mut buf) {
            Ok(Text(e)) => {
                e.unescape_and_decode(&r).unwrap();
            }
            Ok(Eof) => break,
            _ => (),
        }
    }
}

#[test]
fn fuzz_53() {
    let data: &[u8] = b"\xe9\x00\x00\x00\x00\x00\x00\x00\x00\
\x00\x00\x00\x00\n(\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\
\x00<>\x00\x08\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00<<\x00\x00\x00";
    let cursor = Cursor::new(data);
    let mut reader = Reader::from_reader(cursor);
    let mut buf = vec![];
    loop {
        match reader.read_event(&mut buf) {
            Ok(quick_xml::events::Event::Eof) | Err(..) => break,
            _ => buf.clear(),
        }
    }
}

#[test]
fn test_issue94() {
    let data = br#"<Run>
<!B>
</Run>"#;
    let mut reader = Reader::from_reader(&data[..]);
    reader.trim_text(true);
    let mut buf = vec![];
    loop {
        match reader.read_event(&mut buf) {
            Ok(quick_xml::events::Event::Eof) | Err(..) => break,
            _ => buf.clear(),
        }
        buf.clear();
    }
}

#[test]
fn fuzz_101() {
    let data: &[u8] = b"\x00\x00<\x00\x00\x0a>&#44444444401?#\x0a413518\
                       #\x0a\x0a\x0a;<:<)(<:\x0a\x0a\x0a\x0a;<:\x0a\x0a\
                       <:\x0a\x0a\x0a\x0a\x0a<\x00*\x00\x00\x00\x00";
    let cursor = Cursor::new(data);
    let mut reader = Reader::from_reader(cursor);
    let mut buf = vec![];
    loop {
        match reader.read_event(&mut buf) {
            Ok(Start(ref e)) | Ok(Empty(ref e)) => {
                if e.unescaped().is_err() {
                    break;
                }
                for a in e.attributes() {
                    if a.ok().map_or(true, |a| a.unescaped_value().is_err()) {
                        break;
                    }
                }
            }
            Ok(Text(ref e)) => {
                if e.unescaped().is_err() {
                    break;
                }
            }
            Ok(Eof) | Err(..) => break,
            _ => (),
        }
        buf.clear();
    }
}

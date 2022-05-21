use pretty_assertions::assert_eq;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::Event::*;
use quick_xml::Reader;
use std::borrow::Cow;

#[test]
fn namespace() {
    let mut r = Reader::from_str("<a xmlns:myns='www1'><myns:b>in namespace!</myns:b></a>");
    r.trim_text(true);

    let mut buf = Vec::new();
    let mut ns_buf = Vec::new();

    // <a>
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((ns, Start(_))) => assert_eq!(ns, None),
        e => panic!(
            "expecting outer start element with no namespace, got {:?}",
            e
        ),
    }

    // <b>
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((ns, Start(_))) => assert_eq!(ns, Some(b"www1".as_ref())),
        e => panic!(
            "expecting inner start element with to resolve to 'www1', got {:?}",
            e
        ),
    }
    // "in namespace!"
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        //TODO: Check in specification, it is true that namespace should be empty?
        Ok((ns, Text(_))) => assert_eq!(ns, None),
        e => panic!("expecting text content with no namespace, got {:?}", e),
    }
    // </b>
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((ns, End(_))) => assert_eq!(ns, Some(b"www1".as_ref())),
        e => panic!(
            "expecting inner end element with to resolve to 'www1', got {:?}",
            e
        ),
    }

    // </a>
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((ns, End(_))) => assert_eq!(ns, None),
        e => panic!("expecting outer end element with no namespace, got {:?}", e),
    }
}

#[test]
fn default_namespace() {
    let mut r = Reader::from_str(r#"<a ><b xmlns="www1"></b></a>"#);
    r.trim_text(true);

    let mut buf = Vec::new();
    let mut ns_buf = Vec::new();

    // <a>
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((ns, Start(_))) => assert_eq!(ns, None),
        e => panic!(
            "expecting outer start element with no namespace, got {:?}",
            e
        ),
    }

    // <b>
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((ns, Start(_))) => assert_eq!(ns, Some(b"www1".as_ref())),
        e => panic!(
            "expecting inner start element with to resolve to 'www1', got {:?}",
            e
        ),
    }
    // </b>
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((ns, End(_))) => assert_eq!(ns, Some(b"www1".as_ref())),
        e => panic!(
            "expecting inner end element with to resolve to 'www1', got {:?}",
            e
        ),
    }

    // </a> very important: a should not be in any namespace. The default namespace only applies to
    // the sub-document it is defined on.
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((ns, End(_))) => assert_eq!(ns, None),
        e => panic!("expecting outer end element with no namespace, got {:?}", e),
    }
}

#[test]
fn default_namespace_reset() {
    let mut r = Reader::from_str(r#"<a xmlns="www1"><b xmlns=""></b></a>"#);
    r.trim_text(true);

    let mut buf = Vec::new();
    let mut ns_buf = Vec::new();

    // <a>
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((ns, Start(_))) => assert_eq!(ns, Some(b"www1".as_ref())),
        e => panic!(
            "expecting outer start element with to resolve to 'www1', got {:?}",
            e
        ),
    }

    // <b>
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((ns, Start(_))) => assert_eq!(ns, None),
        e => panic!(
            "expecting inner start element with no namespace, got {:?}",
            e
        ),
    }
    // </b>
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((ns, End(_))) => assert_eq!(ns, None),
        e => panic!("expecting inner end element with no namespace, got {:?}", e),
    }

    // </a>
    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((ns, End(_))) => assert_eq!(ns, Some(b"www1".as_ref())),
        e => panic!(
            "expecting outer end element with to resolve to 'www1', got {:?}",
            e
        ),
    }
}

/// Single empty element with qualified attributes.
/// Empty element expansion: disabled
/// The code path for namespace handling is slightly different for `Empty` vs. `Start+End`.
#[test]
fn attributes_empty_ns() {
    let src = b"<a att1='a' r:att2='b' xmlns:r='urn:example:r' />";

    let mut r = Reader::from_reader(src as &[u8]);
    r.trim_text(true).expand_empty_elements(false);
    let mut buf = Vec::new();
    let mut ns_buf = Vec::new();

    let e = match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((None, Empty(e))) => e,
        e => panic!("Expecting Empty event, got {:?}", e),
    };

    let mut attrs = e
        .attributes()
        .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
        // we don't care about xmlns attributes for this test
        .filter(|kv| !kv.key.starts_with(b"xmlns"))
        .map(|Attribute { key: name, value }| {
            let (opt_ns, local_name) = r.attribute_namespace(name, &ns_buf);
            (opt_ns, local_name, value)
        });
    assert_eq!(
        attrs.next(),
        Some((None, &b"att1"[..], Cow::Borrowed(&b"a"[..])))
    );
    assert_eq!(
        attrs.next(),
        Some((
            Some(&b"urn:example:r"[..]),
            &b"att2"[..],
            Cow::Borrowed(&b"b"[..])
        ))
    );
    assert_eq!(attrs.next(), None);
}

/// Single empty element with qualified attributes.
/// Empty element expansion: enabled
/// The code path for namespace handling is slightly different for `Empty` vs. `Start+End`.
#[test]
fn attributes_empty_ns_expanded() {
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

        let mut attrs = e
            .attributes()
            .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
            // we don't care about xmlns attributes for this test
            .filter(|kv| !kv.key.starts_with(b"xmlns"))
            .map(|Attribute { key: name, value }| {
                let (opt_ns, local_name) = r.attribute_namespace(name, &ns_buf);
                (opt_ns, local_name, value)
            });
        assert_eq!(
            attrs.next(),
            Some((None, &b"att1"[..], Cow::Borrowed(&b"a"[..])))
        );
        assert_eq!(
            attrs.next(),
            Some((
                Some(&b"urn:example:r"[..]),
                &b"att2"[..],
                Cow::Borrowed(&b"b"[..])
            ))
        );
        assert_eq!(attrs.next(), None);
    }

    match r.read_namespaced_event(&mut buf, &mut ns_buf) {
        Ok((None, End(e))) => assert_eq!(b"a", e.name()),
        e => panic!("Expecting End event, got {:?}", e),
    }
}

#[test]
fn default_ns_shadowing_empty() {
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

        let mut attrs = e
            .attributes()
            .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
            // we don't care about xmlns attributes for this test
            .filter(|kv| !kv.key.starts_with(b"xmlns"))
            .map(|Attribute { key: name, value }| {
                let (opt_ns, local_name) = r.attribute_namespace(name, &ns_buf);
                (opt_ns, local_name, value)
            });
        // the attribute should _not_ have a namespace name. The default namespace does not
        // apply to attributes.
        assert_eq!(
            attrs.next(),
            Some((None, &b"att1"[..], Cow::Borrowed(&b"a"[..])))
        );
        assert_eq!(attrs.next(), None);
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
fn default_ns_shadowing_expanded() {
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
        let mut attrs = e
            .attributes()
            .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
            // we don't care about xmlns attributes for this test
            .filter(|kv| !kv.key.starts_with(b"xmlns"))
            .map(|Attribute { key: name, value }| {
                let (opt_ns, local_name) = r.attribute_namespace(name, &ns_buf);
                (opt_ns, local_name, value)
            });
        // the attribute should _not_ have a namespace name. The default namespace does not
        // apply to attributes.
        assert_eq!(
            attrs.next(),
            Some((None, &b"att1"[..], Cow::Borrowed(&b"a"[..])))
        );
        assert_eq!(attrs.next(), None);
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

use pretty_assertions::assert_eq;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::Event::*;
use quick_xml::events::{
    BytesCData, BytesDecl, BytesEnd, BytesPI, BytesRef, BytesStart, BytesText,
};
use quick_xml::name::ResolveResult::*;
use quick_xml::name::{Namespace, PrefixDeclaration, QName};
use quick_xml::reader::NsReader;
use std::borrow::Cow;

#[test]
fn namespace() {
    let mut r = NsReader::from_str("<a xmlns:myns='www1'><myns:b>in namespace!</myns:b></a>");

    // <a>
    match r.read_resolved_event() {
        Ok((ns, Start(_))) => assert_eq!(ns, Unbound),
        e => panic!(
            "expecting outer start element with no namespace, got {:?}",
            e
        ),
    }
    let it1 = r.resolver().bindings();
    let it2 = it1.clone();
    assert_eq!(it1.size_hint(), (0, Some(1)));
    assert_eq!(
        it1.collect::<Vec<_>>(),
        vec![(PrefixDeclaration::Named(b"myns"), Namespace(b"www1"))]
    );

    assert_eq!(it2.size_hint(), (0, Some(1)));
    assert_eq!(
        it2.collect::<Vec<_>>(),
        vec![(PrefixDeclaration::Named(b"myns"), Namespace(b"www1"))]
    );

    // <b>
    match r.read_resolved_event() {
        Ok((ns, Start(_))) => assert_eq!(ns, Bound(Namespace(b"www1"))),
        e => panic!(
            "expecting inner start element with to resolve to 'www1', got {:?}",
            e
        ),
    }
    let it = r.resolver().bindings();
    assert_eq!(it.size_hint(), (0, Some(1)));
    assert_eq!(
        it.collect::<Vec<_>>(),
        vec![(PrefixDeclaration::Named(b"myns"), Namespace(b"www1"))]
    );

    // "in namespace!"
    match r.read_resolved_event() {
        Ok((ns, Text(_))) => assert_eq!(ns, Unbound),
        e => panic!("expecting text content with no namespace, got {:?}", e),
    }
    let it = r.resolver().bindings();
    assert_eq!(it.size_hint(), (0, Some(1)));
    assert_eq!(
        it.collect::<Vec<_>>(),
        vec![(PrefixDeclaration::Named(b"myns"), Namespace(b"www1"))]
    );

    // </b>
    match r.read_resolved_event() {
        Ok((ns, End(_))) => assert_eq!(ns, Bound(Namespace(b"www1"))),
        e => panic!(
            "expecting inner end element with to resolve to 'www1', got {:?}",
            e
        ),
    }
    let it = r.resolver().bindings();
    assert_eq!(it.size_hint(), (0, Some(1)));
    assert_eq!(
        it.collect::<Vec<_>>(),
        vec![(PrefixDeclaration::Named(b"myns"), Namespace(b"www1"))]
    );

    // </a>
    match r.read_resolved_event() {
        Ok((ns, End(_))) => assert_eq!(ns, Unbound),
        e => panic!("expecting outer end element with no namespace, got {:?}", e),
    }
    let it = r.resolver().bindings();
    assert_eq!(it.size_hint(), (0, Some(1)));
    assert_eq!(
        it.collect::<Vec<_>>(),
        vec![(PrefixDeclaration::Named(b"myns"), Namespace(b"www1"))]
    );
}

mod default_namespace {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn event_empty() {
        let mut r = NsReader::from_str("<a attr='val' xmlns='ns' />");

        let e = match r.read_resolved_event() {
            Ok((ns, Empty(e))) => {
                assert_eq!(ns, Bound(Namespace(b"ns")));
                e
            }
            e => panic!("Expecting Empty event, got {:?}", e),
        };

        let mut attrs = e
            .attributes()
            .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
            // we don't care about xmlns attributes for this test
            .filter(|kv| kv.key.as_namespace_binding().is_none())
            .map(|Attribute { key: name, value }| {
                let (opt_ns, local_name) = r.resolver().resolve_attribute(name);
                (opt_ns, local_name.into_inner(), value)
            });
        assert_eq!(
            attrs.next(),
            Some((Unbound, &b"attr"[..], Cow::Borrowed(&b"val"[..])))
        );
        assert_eq!(attrs.next(), None);

        let it = r.resolver().bindings();
        assert_eq!(it.size_hint(), (0, Some(1)));
        assert_eq!(
            it.collect::<Vec<_>>(),
            vec![(PrefixDeclaration::Default, Namespace(b"ns"))]
        );
    }

    #[test]
    fn event_start_end() {
        let mut r = NsReader::from_str(r#"<a ><b xmlns="www1"></b></a>"#);

        // <a>
        match r.read_resolved_event() {
            Ok((ns, Start(_))) => assert_eq!(ns, Unbound),
            e => panic!(
                "expecting outer start element with no namespace, got {:?}",
                e
            ),
        }
        let it = r.resolver().bindings();
        assert_eq!(it.size_hint(), (0, Some(0)));
        assert_eq!(it.collect::<Vec<_>>(), vec![]);

        // <b>
        match r.read_resolved_event() {
            Ok((ns, Start(_))) => assert_eq!(ns, Bound(Namespace(b"www1"))),
            e => panic!(
                "expecting inner start element with to resolve to 'www1', got {:?}",
                e
            ),
        }
        let it = r.resolver().bindings();
        assert_eq!(it.size_hint(), (0, Some(1)));
        assert_eq!(
            it.collect::<Vec<_>>(),
            vec![(PrefixDeclaration::Default, Namespace(b"www1"))]
        );

        // </b>
        match r.read_resolved_event() {
            Ok((ns, End(_))) => assert_eq!(ns, Bound(Namespace(b"www1"))),
            e => panic!(
                "expecting inner end element with to resolve to 'www1', got {:?}",
                e
            ),
        }
        let it = r.resolver().bindings();
        assert_eq!(it.size_hint(), (0, Some(1)));
        assert_eq!(
            it.collect::<Vec<_>>(),
            vec![(PrefixDeclaration::Default, Namespace(b"www1"))]
        );

        // </a> very important: a should not be in any namespace. The default namespace only applies to
        // the sub-document it is defined on.
        match r.read_resolved_event() {
            Ok((ns, End(_))) => assert_eq!(ns, Unbound),
            e => panic!("expecting outer end element with no namespace, got {:?}", e),
        }
        let it = r.resolver().bindings();
        assert_eq!(it.size_hint(), (0, Some(0)));
        assert_eq!(it.collect::<Vec<_>>(), vec![]);
    }

    #[test]
    fn reset() {
        let mut r = NsReader::from_str(r#"<a xmlns="www1"><b xmlns=""></b></a>"#);

        // <a>
        match r.read_resolved_event() {
            Ok((ns, Start(_))) => assert_eq!(ns, Bound(Namespace(b"www1"))),
            e => panic!(
                "expecting outer start element with to resolve to 'www1', got {:?}",
                e
            ),
        }
        let it = r.resolver().bindings();
        assert_eq!(it.size_hint(), (0, Some(1)));
        assert_eq!(
            it.collect::<Vec<_>>(),
            vec![(PrefixDeclaration::Default, Namespace(b"www1"))]
        );

        // <b>
        match r.read_resolved_event() {
            Ok((ns, Start(_))) => assert_eq!(ns, Unbound),
            e => panic!(
                "expecting inner start element with no namespace, got {:?}",
                e
            ),
        }
        let it = r.resolver().bindings();
        assert_eq!(it.size_hint(), (0, Some(2)));
        assert_eq!(it.collect::<Vec<_>>(), vec![]);

        // </b>
        match r.read_resolved_event() {
            Ok((ns, End(_))) => assert_eq!(ns, Unbound),
            e => panic!("expecting inner end element with no namespace, got {:?}", e),
        }
        let it = r.resolver().bindings();
        assert_eq!(it.size_hint(), (0, Some(2)));
        assert_eq!(it.collect::<Vec<_>>(), vec![]);

        // </a>
        match r.read_resolved_event() {
            Ok((ns, End(_))) => assert_eq!(ns, Bound(Namespace(b"www1"))),
            e => panic!(
                "expecting outer end element with to resolve to 'www1', got {:?}",
                e
            ),
        }
        let it = r.resolver().bindings();
        assert_eq!(it.size_hint(), (0, Some(1)));
        assert_eq!(
            it.collect::<Vec<_>>(),
            vec![(PrefixDeclaration::Default, Namespace(b"www1"))]
        );
    }

    #[test]
    fn shadowing_empty() {
        let src = "<e xmlns='urn:example:o'><e att1='a' xmlns='urn:example:i' /></e>";

        let mut r = NsReader::from_str(src);

        // <outer xmlns='urn:example:o'>
        {
            match r.read_resolved_event() {
                Ok((ns, Start(e))) => {
                    assert_eq!(ns, Bound(Namespace(b"urn:example:o")));
                    assert_eq!(e.name(), QName(b"e"));
                }
                e => panic!("Expected Start event (<outer>), got {:?}", e),
            }

            let it = r.resolver().bindings();
            assert_eq!(it.size_hint(), (0, Some(1)));
            assert_eq!(
                it.collect::<Vec<_>>(),
                vec![(PrefixDeclaration::Default, Namespace(b"urn:example:o"))]
            );
        }

        // <inner att1='a' xmlns='urn:example:i' />
        {
            let e = match r.read_resolved_event() {
                Ok((ns, Empty(e))) => {
                    assert_eq!(ns, Bound(Namespace(b"urn:example:i")));
                    assert_eq!(e.name(), QName(b"e"));
                    e
                }
                e => panic!("Expecting Empty event, got {:?}", e),
            };

            let mut attrs = e
                .attributes()
                .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
                // we don't care about xmlns attributes for this test
                .filter(|kv| kv.key.as_namespace_binding().is_none())
                .map(|Attribute { key: name, value }| {
                    let (opt_ns, local_name) = r.resolver().resolve_attribute(name);
                    (opt_ns, local_name.into_inner(), value)
                });
            // the attribute should _not_ have a namespace name. The default namespace does not
            // apply to attributes.
            assert_eq!(
                attrs.next(),
                Some((Unbound, &b"att1"[..], Cow::Borrowed(&b"a"[..])))
            );
            assert_eq!(attrs.next(), None);

            let it = r.resolver().bindings();
            assert_eq!(it.size_hint(), (0, Some(2)));
            assert_eq!(
                it.collect::<Vec<_>>(),
                vec![(PrefixDeclaration::Default, Namespace(b"urn:example:i")),]
            );
        }

        // </outer>
        match r.read_resolved_event() {
            Ok((ns, End(e))) => {
                assert_eq!(ns, Bound(Namespace(b"urn:example:o")));
                assert_eq!(e.name(), QName(b"e"));
            }
            e => panic!("Expected End event (<outer>), got {:?}", e),
        }
        let it = r.resolver().bindings();
        assert_eq!(it.size_hint(), (0, Some(1)));
        assert_eq!(
            it.collect::<Vec<_>>(),
            vec![(PrefixDeclaration::Default, Namespace(b"urn:example:o"))]
        );
    }

    #[test]
    fn shadowing_expanded() {
        let src = "<e xmlns='urn:example:o'><e att1='a' xmlns='urn:example:i' /></e>";

        let mut r = NsReader::from_str(src);
        r.config_mut().expand_empty_elements = true;

        // <outer xmlns='urn:example:o'>
        {
            match r.read_resolved_event() {
                Ok((ns, Start(e))) => {
                    assert_eq!(ns, Bound(Namespace(b"urn:example:o")));
                    assert_eq!(e.name(), QName(b"e"));
                }
                e => panic!("Expected Start event (<outer>), got {:?}", e),
            }
        }

        // <inner att1='a' xmlns='urn:example:i' />
        {
            let e = match r.read_resolved_event() {
                Ok((ns, Start(e))) => {
                    assert_eq!(ns, Bound(Namespace(b"urn:example:i")));
                    assert_eq!(e.name(), QName(b"e"));
                    e
                }
                e => panic!("Expecting Start event (<inner>), got {:?}", e),
            };
            let mut attrs = e
                .attributes()
                .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
                // we don't care about xmlns attributes for this test
                .filter(|kv| kv.key.as_namespace_binding().is_none())
                .map(|Attribute { key: name, value }| {
                    let (opt_ns, local_name) = r.resolver().resolve_attribute(name);
                    (opt_ns, local_name.into_inner(), value)
                });
            // the attribute should _not_ have a namespace name. The default namespace does not
            // apply to attributes.
            assert_eq!(
                attrs.next(),
                Some((Unbound, &b"att1"[..], Cow::Borrowed(&b"a"[..])))
            );
            assert_eq!(attrs.next(), None);
        }

        // virtual </inner>
        match r.read_resolved_event() {
            Ok((ns, End(e))) => {
                assert_eq!(ns, Bound(Namespace(b"urn:example:i")));
                assert_eq!(e.name(), QName(b"e"));
            }
            e => panic!("Expected End event (</inner>), got {:?}", e),
        }
        // </outer>
        match r.read_resolved_event() {
            Ok((ns, End(e))) => {
                assert_eq!(ns, Bound(Namespace(b"urn:example:o")));
                assert_eq!(e.name(), QName(b"e"));
            }
            e => panic!("Expected End event (</outer>), got {:?}", e),
        }
    }
}

/// Single empty element with qualified attributes.
/// Empty element expansion: disabled
/// The code path for namespace handling is slightly different for `Empty` vs. `Start+End`.
#[test]
fn attributes_empty_ns() {
    let src = "<a att1='a' r:att2='b' xmlns:r='urn:example:r' />";

    let mut r = NsReader::from_str(src);

    let e = match r.read_resolved_event() {
        Ok((Unbound, Empty(e))) => e,
        e => panic!("Expecting Empty event, got {:?}", e),
    };

    let mut attrs = e
        .attributes()
        .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
        // we don't care about xmlns attributes for this test
        .filter(|kv| kv.key.as_namespace_binding().is_none())
        .map(|Attribute { key: name, value }| {
            let (opt_ns, local_name) = r.resolver().resolve_attribute(name);
            (opt_ns, local_name.into_inner(), value)
        });
    assert_eq!(
        attrs.next(),
        Some((Unbound, &b"att1"[..], Cow::Borrowed(&b"a"[..])))
    );
    assert_eq!(
        attrs.next(),
        Some((
            Bound(Namespace(b"urn:example:r")),
            &b"att2"[..],
            Cow::Borrowed(&b"b"[..])
        ))
    );
    assert_eq!(attrs.next(), None);

    let it = r.resolver().bindings();
    assert_eq!(it.size_hint(), (0, Some(1)));
    assert_eq!(
        it.collect::<Vec<_>>(),
        vec![(PrefixDeclaration::Named(b"r"), Namespace(b"urn:example:r"))]
    );
}

/// Single empty element with qualified attributes.
/// Empty element expansion: enabled
/// The code path for namespace handling is slightly different for `Empty` vs. `Start+End`.
#[test]
fn attributes_empty_ns_expanded() {
    let src = "<a att1='a' r:att2='b' xmlns:r='urn:example:r' />";

    let mut r = NsReader::from_str(src);
    r.config_mut().expand_empty_elements = true;
    {
        let e = match r.read_resolved_event() {
            Ok((Unbound, Start(e))) => e,
            e => panic!("Expecting Start event, got {:?}", e),
        };

        let mut attrs = e
            .attributes()
            .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
            // we don't care about xmlns attributes for this test
            .filter(|kv| kv.key.as_namespace_binding().is_none())
            .map(|Attribute { key: name, value }| {
                let (opt_ns, local_name) = r.resolver().resolve_attribute(name);
                (opt_ns, local_name.into_inner(), value)
            });
        assert_eq!(
            attrs.next(),
            Some((Unbound, &b"att1"[..], Cow::Borrowed(&b"a"[..])))
        );
        assert_eq!(
            attrs.next(),
            Some((
                Bound(Namespace(b"urn:example:r")),
                &b"att2"[..],
                Cow::Borrowed(&b"b"[..])
            ))
        );
        assert_eq!(attrs.next(), None);

        let it = r.resolver().bindings();
        assert_eq!(it.size_hint(), (0, Some(1)));
        assert_eq!(
            it.collect::<Vec<_>>(),
            vec![(PrefixDeclaration::Named(b"r"), Namespace(b"urn:example:r"))]
        );
    }

    match r.read_resolved_event() {
        Ok((Unbound, End(e))) => assert_eq!(e.name(), QName(b"a")),
        e => panic!("Expecting End event, got {:?}", e),
    }
}

/// Although the XML specification [recommends against] the use of names where
/// the local name portion begins with the letters "xml" (case insensitive),
/// it also specifies, that processors *MUST NOT* treat them as fatal errors.
/// That means, that processing should continue -- in our case we should read
/// an XML event and user should be able to check constraints later if he/she wish.
///
/// [recommends against]: https://www.w3.org/TR/xml-names11/#xmlReserved
#[test]
fn reserved_name() {
    // Name "xmlns-something" is reserved according to spec, because started with "xml"
    let mut r =
        NsReader::from_str(r#"<a xmlns-something="reserved attribute name" xmlns="www1"/>"#);

    // <a />
    match r.read_resolved_event() {
        Ok((ns, Empty(_))) => assert_eq!(ns, Bound(Namespace(b"www1"))),
        e => panic!(
            "Expected empty element bound to namespace 'www1', got {:?}",
            e
        ),
    }
}

mod read_to_end {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Yes, this test contains invalid XML but since we can parse it, we check
    /// that it does not break our parser
    #[test]
    fn decl() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <?xml version=\"1.0\"?>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Decl(BytesDecl::new("1.0", None, None))
        );
        assert_eq!(
            reader.read_to_end(QName(b"root")).unwrap(),
            45..65 // <root/><root></root>
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }

    /// Yes, this test contains invalid XML but since we can parse it, we check
    /// that it does not break our parser
    #[test]
    fn doctype() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <!DOCTYPE dtd>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(reader.read_event().unwrap(), DocType(BytesText::new("dtd")));
        assert_eq!(
            reader.read_to_end(QName(b"root")).unwrap(),
            38..58 // <root/><root></root>
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }

    #[test]
    fn pi() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <?pi?>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(reader.read_event().unwrap(), PI(BytesPI::new("pi")));
        assert_eq!(
            reader.read_to_end(QName(b"root")).unwrap(),
            30..50 // <root/><root></root>
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }

    #[test]
    fn comment() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <!--comment-->\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Comment(BytesText::new("comment"))
        );
        assert_eq!(
            reader.read_to_end(QName(b"root")).unwrap(),
            38..58 // <root/><root></root>
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }

    #[test]
    fn start() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <tag>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        reader.config_mut().check_end_names = false;
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::new("tag")),
            )
        );
        assert_eq!(
            reader.read_to_end(QName(b"root")).unwrap(),
            29..49 // <root/><root></root>
        );
        // NOTE: due to unbalanced XML namespace still not closed
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Empty(BytesStart::new("element"))
            )
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }

    #[test]
    fn end() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                </tag>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        reader.config_mut().check_end_names = false;
        reader.config_mut().allow_unmatched_ends = true;
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Bound(Namespace(b"namespace")), End(BytesEnd::new("tag")),)
        );
        assert_eq!(
            reader.read_to_end(QName(b"root")).unwrap(),
            30..50 // <root/><root></root>
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }

    #[test]
    fn empty() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <tag/>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Empty(BytesStart::new("tag")),
            )
        );
        assert_eq!(
            reader.read_to_end(QName(b"root")).unwrap(),
            30..50 // <root/><root></root>
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }

    #[test]
    fn text() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                text\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(reader.read_event().unwrap(), Text(BytesText::new("text")));
        assert_eq!(
            reader.read_to_end(QName(b"root")).unwrap(),
            28..48 // <root/><root></root>
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }

    #[test]
    fn cdata() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <![CDATA[cdata]]>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_event().unwrap(),
            CData(BytesCData::new("cdata"))
        );
        assert_eq!(
            reader.read_to_end(QName(b"root")).unwrap(),
            41..61 // <root/><root></root>
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }

    #[test]
    fn general_ref() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                &entity;\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_event().unwrap(),
            GeneralRef(BytesRef::new("entity"))
        );
        assert_eq!(
            reader.read_to_end(QName(b"root")).unwrap(),
            32..52 // <root/><root></root>
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }
}

mod read_to_end_into {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Yes, this test contains invalid XML but since we can parse it, we check
    /// that it does not break our parser
    #[test]
    fn decl() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <?xml version=\"1.0\"?>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        let buf = &mut Vec::new();
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_event_into(buf).unwrap(),
            Decl(BytesDecl::new("1.0", None, None))
        );
        assert_eq!(
            reader.read_to_end_into(QName(b"root"), buf).unwrap(),
            45..65 // <root/><root></root>
        );
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event_into(buf).unwrap(), Eof);
    }

    /// Yes, this test contains invalid XML but since we can parse it, we check
    /// that it does not break our parser
    #[test]
    fn doctype() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <!DOCTYPE dtd>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        let buf = &mut Vec::new();
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_event_into(buf).unwrap(),
            DocType(BytesText::new("dtd"))
        );
        assert_eq!(
            reader.read_to_end_into(QName(b"root"), buf).unwrap(),
            38..58 // <root/><root></root>
        );
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event_into(buf).unwrap(), Eof);
    }

    #[test]
    fn pi() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <?pi?>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        let buf = &mut Vec::new();
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(reader.read_event_into(buf).unwrap(), PI(BytesPI::new("pi")));
        assert_eq!(
            reader.read_to_end_into(QName(b"root"), buf).unwrap(),
            30..50 // <root/><root></root>
        );
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event_into(buf).unwrap(), Eof);
    }

    #[test]
    fn comment() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <!--comment-->\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        let buf = &mut Vec::new();
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_event_into(buf).unwrap(),
            Comment(BytesText::new("comment"))
        );
        assert_eq!(
            reader.read_to_end_into(QName(b"root"), buf).unwrap(),
            38..58 // <root/><root></root>
        );
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event_into(buf).unwrap(), Eof);
    }

    #[test]
    fn start() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <tag>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        let buf = &mut Vec::new();
        reader.config_mut().check_end_names = false;
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::new("tag")),
            )
        );
        assert_eq!(
            reader.read_to_end_into(QName(b"root"), buf).unwrap(),
            29..49 // <root/><root></root><root></root>
        );
        // NOTE: due to unbalanced XML namespace still not closed
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Empty(BytesStart::new("element"))
            )
        );
        assert_eq!(reader.read_event_into(buf).unwrap(), Eof);
    }

    #[test]
    fn end() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                </tag>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        let buf = &mut Vec::new();
        reader.config_mut().check_end_names = false;
        reader.config_mut().allow_unmatched_ends = true;
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (Bound(Namespace(b"namespace")), End(BytesEnd::new("tag")),)
        );
        assert_eq!(
            reader.read_to_end_into(QName(b"root"), buf).unwrap(),
            30..50 // <root/><root></root>
        );
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event_into(buf).unwrap(), Eof);
    }

    #[test]
    fn empty() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <tag/>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        let buf = &mut Vec::new();
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Empty(BytesStart::new("tag")),
            )
        );
        assert_eq!(
            reader.read_to_end_into(QName(b"root"), buf).unwrap(),
            30..50 // <root/><root></root>
        );
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event_into(buf).unwrap(), Eof);
    }

    #[test]
    fn text() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                text\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        let buf = &mut Vec::new();
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_event_into(buf).unwrap(),
            Text(BytesText::new("text"))
        );
        assert_eq!(
            reader.read_to_end_into(QName(b"root"), buf).unwrap(),
            28..48 // <root/><root></root>
        );
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event_into(buf).unwrap(), Eof);
    }

    #[test]
    fn cdata() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <![CDATA[cdata]]>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        let buf = &mut Vec::new();
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_event_into(buf).unwrap(),
            CData(BytesCData::new("cdata"))
        );
        assert_eq!(
            reader.read_to_end_into(QName(b"root"), buf).unwrap(),
            41..61 // <root/><root></root>
        );
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event_into(buf).unwrap(), Eof);
    }

    #[test]
    fn general_ref() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                &entity;\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        let buf = &mut Vec::new();
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_event_into(buf).unwrap(),
            GeneralRef(BytesRef::new("entity"))
        );
        assert_eq!(
            reader.read_to_end_into(QName(b"root"), buf).unwrap(),
            32..52 // <root/><root></root>
        );
        assert_eq!(
            reader.read_resolved_event_into(buf).unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event_into(buf).unwrap(), Eof);
    }
}

mod read_text {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Yes, this test contains invalid XML but since we can parse it, we check
    /// that it does not break our parser
    #[test]
    fn decl() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <?xml version=\"1.0\"?>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Decl(BytesDecl::new("1.0", None, None))
        );
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }

    /// Yes, this test contains invalid XML but since we can parse it, we check
    /// that it does not break our parser
    #[test]
    fn doctype() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <!DOCTYPE dtd>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(reader.read_event().unwrap(), DocType(BytesText::new("dtd")));
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }

    #[test]
    fn pi() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <?pi?>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(reader.read_event().unwrap(), PI(BytesPI::new("pi")));
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }

    #[test]
    fn comment() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <!--comment-->\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Comment(BytesText::new("comment"))
        );
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }

    #[test]
    fn start() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <tag>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        reader.config_mut().check_end_names = false;
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::new("tag")),
            )
        );
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        // NOTE: due to unbalanced XML namespace still not closed
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Empty(BytesStart::new("element"))
            )
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }

    #[test]
    fn end() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                </tag>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        reader.config_mut().check_end_names = false;
        reader.config_mut().allow_unmatched_ends = true;
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Bound(Namespace(b"namespace")), End(BytesEnd::new("tag")),)
        );
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }

    #[test]
    fn empty() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <tag/>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Empty(BytesStart::new("tag")),
            )
        );
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }

    #[test]
    fn text() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                text\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(reader.read_event().unwrap(), Text(BytesText::new("text")));
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }

    #[test]
    fn cdata() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                <![CDATA[cdata]]>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_event().unwrap(),
            CData(BytesCData::new("cdata"))
        );
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }

    #[test]
    fn general_ref() {
        let mut reader = NsReader::from_str(
            "\
            <root xmlns='namespace'>\
                &entity;\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (
                Bound(Namespace(b"namespace")),
                Start(BytesStart::from_content("root xmlns='namespace'", 4)),
            )
        );
        assert_eq!(
            reader.read_event().unwrap(),
            GeneralRef(BytesRef::new("entity"))
        );
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_resolved_event().unwrap(),
            (Unbound, Empty(BytesStart::new("element")))
        );
        assert_eq!(reader.read_event().unwrap(), Eof);
    }
}

extern crate quick_xml;

use quick_xml::XmlReader;
use quick_xml::Event::*;
   
#[test]
fn test_sample() {
    let src: &[u8] = include_bytes!("sample_rss.xml");
    let r = XmlReader::from_reader(src);
    let mut count = 0;
    for e in r {
        match e.unwrap() {
            Start(_) => count += 1,
            Decl(e) => println!("{:?}", e.version()),
            _ => (),
        }
    }
    println!("{}", count);
}

#[test]
fn test_attributes_empty() {
    let src = b"<a att1='a' att2='b'/>";
    let mut r = XmlReader::from_reader(src as &[u8])
        .trim_text(true)
        .expand_empty_elements(false);
    match r.next() {
        Some(Ok(Empty(e))) => {
            let mut atts = e.attributes();
            match atts.next() {
                Some(Ok((b"att1", b"a"))) => (),
                e => panic!("Expecting att1='a' attribute, found {:?}", e),
            }
            match atts.next() {
                Some(Ok((b"att2", b"b"))) => (),
                e => panic!("Expecting att2='b' attribute, found {:?}", e),
            }
            match atts.next() {
                None => (),
                e => panic!("Expecting None, found {:?}", e),
            }
        },
        e => panic!("Expecting Empty event, got {:?}", e),
    }
}

#[test]
fn test_attribute_equal() {
    let src = b"<a att1=\"a=b\"/>";
    let mut r = XmlReader::from_reader(src as &[u8])
        .trim_text(true)
        .expand_empty_elements(false);
    match r.next() {
        Some(Ok(Empty(e))) => {
            let mut atts = e.attributes();
            match atts.next() {
                Some(Ok((b"att1", b"a=b"))) => (),
                e => panic!("Expecting att1=\"a=b\" attribute, found {:?}", e),
            }
            match atts.next() {
                None => (),
                e => panic!("Expecting None, found {:?}", e),
            }
        },
        e => panic!("Expecting Empty event, got {:?}", e),
    }
}

/// Single empty element with qualified attributes.
/// Empty element expansion: disabled
/// The code path for namespace handling is slightly different for `Empty` vs. `Start+End`.
#[test]
fn test_attributes_empty_ns() {
    let src = b"<a att1='a' r:att2='b' xmlns:r='urn:example:r' />";
    let mut r = XmlReader::from_reader(src as &[u8])
        .trim_text(true)
        .expand_empty_elements(false)
        .namespaced();
    match r.next() {
        Some(Ok((None, Empty(e)))) => {
            let mut atts = e.attributes()
                    .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
                    // we don't care about xmlns attributes for this test
                    .filter(|kv| !kv.0.starts_with(b"xmlns"))
                    .map(|kv| {
                let (name,value) = kv;
                let (opt_ns, local_name) = r.resolve(name);
                (opt_ns, local_name, value)
            });
            match atts.next() {
                Some((None, b"att1", b"a")) => (),
                e => panic!("Expecting att1='a' attribute, found {:?}", e),
            }
            match atts.next() {
                Some((Some(ns), b"att2", b"b")) => {
                    assert_eq!(&ns[..], b"urn:example:r");
                },
                e => panic!("Expecting {{urn:example:r}}att2='b' attribute, found {:?}", e),
            }
            match atts.next() {
                None => (),
                e => panic!("Expecting None, found {:?}", e),
            }
        },
        e => panic!("Expecting Empty event, got {:?}", e),
    }
}

/// Single empty element with qualified attributes.
/// Empty element expansion: enabled
/// The code path for namespace handling is slightly different for `Empty` vs. `Start+End`.
#[test]
fn test_attributes_empty_ns_expanded() {
    let src = b"<a att1='a' r:att2='b' xmlns:r='urn:example:r' />";
    let mut r = XmlReader::from_reader(src as &[u8])
        .trim_text(true)
        .expand_empty_elements(true)
        .namespaced();
    match r.next() {
        Some(Ok((None, Start(e)))) => {
            let mut atts = e.attributes()
                .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
                // we don't care about xmlns attributes for this test
                .filter(|kv| !kv.0.starts_with(b"xmlns"))
                .map(|kv| {
                    let (name,value) = kv;
                    let (opt_ns, local_name) = r.resolve(name);
                    (opt_ns, local_name, value)
                });
            match atts.next() {
                Some((None, b"att1", b"a")) => (),
                e => panic!("Expecting att1='a' attribute, found {:?}", e),
            }
            match atts.next() {
                Some((Some(ns), b"att2", b"b")) => {
                    assert_eq!(&ns[..], b"urn:example:r");
                },
                e => panic!("Expecting {{urn:example:r}}att2='b' attribute, found {:?}", e),
            }
            match atts.next() {
                None => (),
                e => panic!("Expecting None, found {:?}", e),
            }
        },
        e => panic!("Expecting Start event, got {:?}", e),
    }
    match r.next() {
        Some(Ok((None, End(e)))) => {
            assert_eq!(e.name(), b"a");
        }
        e => panic!("Expecting End event, got {:?}", e),
    }
}

#[test]
fn test_default_ns_shadowing_empty() {
    let src = b"<e xmlns='urn:example:o'><e att1='a' xmlns='urn:example:i' /></e>";
    let mut r = XmlReader::from_reader(src as &[u8])
        .trim_text(true)
        .expand_empty_elements(false)
        .namespaced();
    // <outer xmlns='urn:example:o'>
    match r.next() {
        Some(Ok((Some(ns), Start(e)))) => {
            assert_eq!(&ns[..], b"urn:example:o");
            assert_eq!(e.name(), b"e");
        },
        e => panic!("Expected Start event (<outer>), got {:?}", e),
    }
    // <inner att1='a' xmlns='urn:example:i' />
    match r.next() {
        Some(Ok((Some(ns), Empty(e)))) => {
            assert_eq!(String::from_utf8(ns).unwrap(), "urn:example:i");
            assert_eq!(e.name(), b"e");
            let mut atts = e.attributes()
                .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
                // we don't care about xmlns attributes for this test
                .filter(|kv| !kv.0.starts_with(b"xmlns"))
                .map(|kv| {
                    let (name,value) = kv;
                    let (opt_ns, local_name) = r.resolve(name);
                    (opt_ns, local_name, value)
                });
            // the attribute should _not_ have a namespace name. The default namespace does not
            // apply to attributes.
            match atts.next() {
                Some((None, b"att1", b"a")) => (),
                e => panic!("Expecting att1='a' attribute, found {:?}", e),
            }
            match atts.next() {
                None => (),
                e => panic!("Expecting None, found {:?}", e),
            }
        },
        e => panic!("Expecting Empty event (<inner />, got {:?}", e),
    }
    // </outer>
    match r.next() {
        Some(Ok((Some(ns), End(e)))) => {
            assert_eq!(&ns[..], b"urn:example:o");
            assert_eq!(e.name(), b"e");
        },
        e => panic!("Expected End event (<outer>), got {:?}", e),
    }
}

#[test]
fn test_default_ns_shadowing_expanded() {
    let src = b"<e xmlns='urn:example:o'><e att1='a' xmlns='urn:example:i' /></e>";
    let mut r = XmlReader::from_reader(src as &[u8])
        .trim_text(true)
        .expand_empty_elements(true)
        .namespaced();
    // <outer xmlns='urn:example:o'>
    match r.next() {
        Some(Ok((Some(ns), Start(e)))) => {
            assert_eq!(&ns[..], b"urn:example:o");
            assert_eq!(e.name(), b"e");
        },
        e => panic!("Expected Start event (<outer>), got {:?}", e),
    }
    // <inner att1='a' xmlns='urn:example:i' />
    match r.next() {
        Some(Ok((Some(ns), Start(e)))) => {
            assert_eq!(&ns[..], b"urn:example:i");
            assert_eq!(e.name(), b"e");
            let mut atts = e.attributes()
                .map(|ar| ar.expect("Expecting attribute parsing to succeed."))
                // we don't care about xmlns attributes for this test
                .filter(|kv| !kv.0.starts_with(b"xmlns"))
                .map(|kv| {
                    let (name,value) = kv;
                    let (opt_ns, local_name) = r.resolve(name);
                    (opt_ns, local_name, value)
                });
            // the attribute should _not_ have a namespace name. The default namespace does not
            // apply to attributes.
            match atts.next() {
                Some((None, b"att1", b"a")) => (),
                e => panic!("Expecting att1='a' attribute, found {:?}", e),
            }
            match atts.next() {
                None => (),
                e => panic!("Expecting None, found {:?}", e),
            }
        },
        e => panic!("Expecting Start event (<inner>), got {:?}", e),
    }
    // virtual </inner>
    match r.next() {
        Some(Ok((Some(ns), End(e)))) => {
            assert_eq!(&ns[..], b"urn:example:i");
            assert_eq!(e.name(), b"e");
        },
        e => panic!("Expected End event (</inner>), got {:?}", e),
    }
    // </outer>
    match r.next() {
        Some(Ok((Some(ns), End(e)))) => {
            assert_eq!(&ns[..], b"urn:example:o");
            assert_eq!(e.name(), b"e");
        },
        e => panic!("Expected End event (</outer>), got {:?}", e),
    }
}

use pretty_assertions::assert_eq;
use quick_xml::escape::{self, EscapeError, ParseCharRefError};
use std::borrow::Cow;
use std::num::IntErrorKind;

#[test]
fn escape() {
    let unchanged = escape::escape("test");
    // assert_eq does not check that Cow is borrowed, but we explicitly use Cow
    // because it influences diff
    // TODO: use assert_matches! when stabilized and other features will bump MSRV
    assert_eq!(unchanged, Cow::Borrowed("test"));
    assert!(matches!(unchanged, Cow::Borrowed(_)));

    assert_eq!(escape::escape("<&\"'>"), "&lt;&amp;&quot;&apos;&gt;");
    assert_eq!(escape::escape("<test>"), "&lt;test&gt;");
    assert_eq!(escape::escape("\"a\"bc"), "&quot;a&quot;bc");
    assert_eq!(escape::escape("\"a\"b&c"), "&quot;a&quot;b&amp;c");
    assert_eq!(
        escape::escape("prefix_\"a\"b&<>c"),
        "prefix_&quot;a&quot;b&amp;&lt;&gt;c"
    );
    assert_eq!(escape::escape("hello\rworld"), "hello&#13;world");
    assert_eq!(escape::escape("a\r\nb"), "a&#13;\nb");
}

#[test]
fn partial_escape() {
    let unchanged = escape::partial_escape("test");
    // assert_eq does not check that Cow is borrowed, but we explicitly use Cow
    // because it influences diff
    // TODO: use assert_matches! when stabilized and other features will bump MSRV
    assert_eq!(unchanged, Cow::Borrowed("test"));
    assert!(matches!(unchanged, Cow::Borrowed(_)));

    assert_eq!(escape::partial_escape("<&\"'>"), "&lt;&amp;\"'&gt;");
    assert_eq!(escape::partial_escape("<test>"), "&lt;test&gt;");
    assert_eq!(escape::partial_escape("\"a\"bc"), "\"a\"bc");
    assert_eq!(escape::partial_escape("\"a\"b&c"), "\"a\"b&amp;c");
    assert_eq!(
        escape::partial_escape("prefix_\"a\"b&<>c"),
        "prefix_\"a\"b&amp;&lt;&gt;c"
    );
    assert_eq!(escape::partial_escape("hello\rworld"), "hello&#13;world");
}

#[test]
fn minimal_escape() {
    assert_eq!(escape::minimal_escape("test"), Cow::Borrowed("test"));
    assert_eq!(escape::minimal_escape("<&\"'>"), "&lt;&amp;\"'>");
    assert_eq!(escape::minimal_escape("<test>"), "&lt;test>");
    assert_eq!(escape::minimal_escape("\"a\"bc"), "\"a\"bc");
    assert_eq!(escape::minimal_escape("\"a\"b&c"), "\"a\"b&amp;c");
    assert_eq!(
        escape::minimal_escape("prefix_\"a\"b&<>c"),
        "prefix_\"a\"b&amp;&lt;>c"
    );
    assert_eq!(escape::minimal_escape("hello\rworld"), "hello&#13;world");
    // `]]>` is not allowed in text content
    assert_eq!(escape::minimal_escape("a]]>b"), "a]]&gt;b");
    assert_eq!(escape::minimal_escape("a]]]>b]>c] ]>"), "a]]]&gt;b]>c] ]>");
    assert_eq!(escape::minimal_escape("]]<>"), "]]&lt;>");
}

#[test]
fn unescape() {
    let unchanged = escape::unescape("test");
    // assert_eq does not check that Cow is borrowed, but we explicitly use Cow
    // because it influences diff
    // TODO: use assert_matches! when stabilized and other features will bump MSRV
    assert_eq!(unchanged, Ok(Cow::Borrowed("test")));
    assert!(matches!(unchanged, Ok(Cow::Borrowed(_))));

    assert_eq!(
        escape::unescape("&lt;&amp;test&apos;&quot;&gt;"),
        Ok("<&test'\">".into())
    );
    assert_eq!(escape::unescape("&#x30;"), Ok("0".into()));
    assert_eq!(escape::unescape("&#48;"), Ok("0".into()));
    assert_eq!(
        escape::unescape("&foo;"),
        Err(EscapeError::UnrecognizedEntity(1..4, "foo".into()))
    );
}

/// XML allows any number of leading zeroes. That is not explicitly mentioned
/// in the specification, but enforced by the conformance test suite
/// (https://www.w3.org/XML/Test/)
/// 100 digits should be enough to ensure that any artificial restrictions
/// (such as maximal string of u128 representation) does not applied
#[test]
fn unescape_long() {
    assert_eq!(
        escape::unescape(
            "&#0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000048;"
        ),
        Ok("0".into()),
    );
    assert_eq!(
        escape::unescape(
            "&#x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000030;"
        ),
        Ok("0".into()),
    );

    // Too big numbers for u32 should produce errors
    match escape::unescape(&format!("&#{};", u32::MAX as u64 + 1)) {
        Err(EscapeError::InvalidCharRef(ParseCharRefError::InvalidNumber(err))) => {
            assert_eq!(err.kind(), &IntErrorKind::PosOverflow)
        }
        x => panic!(
            "expected Err(InvalidCharRef(InvalidNumber(PosOverflow))), bug got {:?}",
            x
        ),
    }
    match escape::unescape(&format!("&#x{:x};", u32::MAX as u64 + 1)) {
        Err(EscapeError::InvalidCharRef(ParseCharRefError::InvalidNumber(err))) => {
            assert_eq!(err.kind(), &IntErrorKind::PosOverflow)
        }
        x => panic!(
            "expected Err(InvalidCharRef(InvalidNumber(PosOverflow))), bug got {:?}",
            x
        ),
    }
}

#[test]
fn unescape_sign() {
    assert_eq!(
        escape::unescape("&#+48;"),
        Err(EscapeError::InvalidCharRef(
            ParseCharRefError::UnexpectedSign
        )),
    );
    assert_eq!(
        escape::unescape("&#x+30;"),
        Err(EscapeError::InvalidCharRef(
            ParseCharRefError::UnexpectedSign
        )),
    );

    assert_eq!(
        escape::unescape("&#-48;"),
        Err(EscapeError::InvalidCharRef(
            ParseCharRefError::UnexpectedSign
        )),
    );
    assert_eq!(
        escape::unescape("&#x-30;"),
        Err(EscapeError::InvalidCharRef(
            ParseCharRefError::UnexpectedSign
        )),
    );
}

#[test]
fn unescape_illegal_char() {
    for (raw, code) in [
        ("&#0;", 0x0),
        ("&#x0;", 0x0),
        ("&#65534;", 0xFFFE),
        ("&#xFFFE;", 0xFFFE),
        ("&#65535;", 0xFFFF),
        ("&#xFFFF;", 0xFFFF),
    ] {
        assert_eq!(
            escape::unescape(raw),
            Err(EscapeError::InvalidCharRef(
                ParseCharRefError::IllegalCharacter(code)
            )),
            "{raw}",
        );
    }
    // Neighbours of the excluded code points are still allowed
    assert_eq!(escape::unescape("&#xFFFD;"), Ok("\u{FFFD}".into()));
    assert_eq!(escape::unescape("&#x10000;"), Ok("\u{10000}".into()));
}

#[test]
fn unescape_with() {
    let custom_entities = |ent: &str| match ent {
        "foo" => Some("BAR"),
        _ => None,
    };

    let unchanged = escape::unescape_with("test", custom_entities);
    // assert_eq does not check that Cow is borrowed, but we explicitly use Cow
    // because it influences diff
    // TODO: use assert_matches! when stabilized and other features will bump MSRV
    assert_eq!(unchanged, Ok(Cow::Borrowed("test")));
    assert!(matches!(unchanged, Ok(Cow::Borrowed(_))));

    assert_eq!(
        escape::unescape_with("&lt;", custom_entities),
        Err(EscapeError::UnrecognizedEntity(1..3, "lt".into())),
    );
    assert_eq!(
        escape::unescape_with("&#x30;", custom_entities),
        Ok("0".into())
    );
    assert_eq!(
        escape::unescape_with("&#48;", custom_entities),
        Ok("0".into())
    );
    assert_eq!(
        escape::unescape_with("&foo;", custom_entities),
        Ok("BAR".into())
    );
    assert_eq!(
        escape::unescape_with("&fop;", custom_entities),
        Err(EscapeError::UnrecognizedEntity(1..4, "fop".into()))
    );
}

/// XML allows any number of leading zeroes. That is not explicitly mentioned
/// in the specification, but enforced by the conformance test suite
/// (https://www.w3.org/XML/Test/)
/// 100 digits should be enough to ensure that any artificial restrictions
/// (such as maximal string of u128 representation) does not applied
#[test]
fn unescape_with_long() {
    assert_eq!(
        escape::unescape_with(
            "&#0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000048;",
            |_| None
        ),
        Ok("0".into()),
    );
    assert_eq!(
        escape::unescape_with(
            "&#x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000030;",
            |_| None
        ),
        Ok("0".into()),
    );

    // Too big numbers for u32 should produce errors
    match escape::unescape_with(&format!("&#{};", u32::MAX as u64 + 1), |_| None) {
        Err(EscapeError::InvalidCharRef(ParseCharRefError::InvalidNumber(err))) => {
            assert_eq!(err.kind(), &IntErrorKind::PosOverflow)
        }
        x => panic!(
            "expected Err(InvalidCharRef(InvalidNumber(PosOverflow))), bug got {:?}",
            x
        ),
    }
    match escape::unescape_with(&format!("&#x{:x};", u32::MAX as u64 + 1), |_| None) {
        Err(EscapeError::InvalidCharRef(ParseCharRefError::InvalidNumber(err))) => {
            assert_eq!(err.kind(), &IntErrorKind::PosOverflow)
        }
        x => panic!(
            "expected Err(InvalidCharRef(InvalidNumber(PosOverflow))), bug got {:?}",
            x
        ),
    }
}

#[test]
fn unescape_with_sign() {
    assert_eq!(
        escape::unescape_with("&#+48;", |_| None),
        Err(EscapeError::InvalidCharRef(
            ParseCharRefError::UnexpectedSign
        )),
    );
    assert_eq!(
        escape::unescape_with("&#x+30;", |_| None),
        Err(EscapeError::InvalidCharRef(
            ParseCharRefError::UnexpectedSign
        )),
    );

    assert_eq!(
        escape::unescape_with("&#-48;", |_| None),
        Err(EscapeError::InvalidCharRef(
            ParseCharRefError::UnexpectedSign
        )),
    );
    assert_eq!(
        escape::unescape_with("&#x-30;", |_| None),
        Err(EscapeError::InvalidCharRef(
            ParseCharRefError::UnexpectedSign
        )),
    );
}

/// Values from <https://html.spec.whatwg.org/entities.json>: at least one entity
/// from each Unicode block that had wrong values, and some neighbours that did not.
#[test]
#[cfg(feature = "escape-html")]
fn resolve_html5_entity() {
    use escape::resolve_html5_entity as resolve;

    // Latin Extended-A and -B
    assert_eq!(resolve("Amacr"), Some("\u{100}"));
    assert_eq!(resolve("scaron"), Some("\u{161}"));
    assert_eq!(resolve("fnof"), Some("\u{192}"));
    // Spacing modifier letters and combining marks
    assert_eq!(resolve("circ"), Some("\u{2C6}"));
    assert_eq!(resolve("DownBreve"), Some("\u{311}"));
    // Greek, including the curly and straight forms of epsilon and phi
    assert_eq!(resolve("alpha"), Some("\u{3B1}"));
    assert_eq!(resolve("Omega"), Some("\u{3A9}"));
    assert_eq!(resolve("epsilon"), Some("\u{3B5}"));
    assert_eq!(resolve("epsi"), Some("\u{3B5}"));
    assert_eq!(resolve("epsiv"), Some("\u{3F5}"));
    assert_eq!(resolve("phi"), Some("\u{3C6}"));
    assert_eq!(resolve("phiv"), Some("\u{3D5}"));
    // Cyrillic
    assert_eq!(resolve("Acy"), Some("\u{410}"));
    assert_eq!(resolve("iocy"), Some("\u{451}"));
    // Letterlike symbols and math operators
    assert_eq!(resolve("ohm"), Some("\u{3A9}"));
    assert_eq!(resolve("angst"), Some("\u{C5}"));
    assert_eq!(resolve("OverBar"), Some("\u{203E}"));
    assert_eq!(resolve("UnderBar"), Some("\u{5F}"));
    assert_eq!(resolve("race"), Some("\u{223D}\u{331}"));
    // Mathematical alphanumeric symbols
    assert_eq!(resolve("Ascr"), Some("\u{1D49C}"));
    assert_eq!(resolve("zfr"), Some("\u{1D537}"));
    assert_eq!(resolve("Aopf"), Some("\u{1D538}"));
    assert_eq!(resolve("zopf"), Some("\u{1D56B}"));

    // Neighbours that already had the right value
    assert_eq!(resolve("macr"), Some("\u{AF}"));
    assert_eq!(resolve("strns"), Some("\u{AF}"));
    assert_eq!(resolve("Copf"), Some("\u{2102}"));
    assert_eq!(resolve("semi"), Some(";"));

    assert_eq!(
        escape::unescape("&alpha; &Acy; &Aopf;").unwrap(),
        "\u{3B1} \u{410} \u{1D538}"
    );
}

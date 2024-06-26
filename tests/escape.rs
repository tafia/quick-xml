use pretty_assertions::assert_eq;
use quick_xml::escape;
use std::borrow::Cow;

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
    assert!(escape::unescape("&foo;").is_err());
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

    assert!(escape::unescape_with("&lt;", custom_entities).is_err());
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
    assert!(escape::unescape_with("&fop;", custom_entities).is_err());
}

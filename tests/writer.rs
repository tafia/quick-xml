use quick_xml::events::{
    BytesCData, BytesDecl, BytesEnd, BytesPI, BytesStart, BytesText, Event::*,
};
use quick_xml::writer::Writer;

use pretty_assertions::assert_eq;

mod declaration {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Written: version, encoding, standalone
    #[test]
    fn full() {
        let mut writer = Writer::new(Vec::new());
        writer
            .write_event(Decl(BytesDecl::new("1.2", Some("utf-X"), Some("yo"))))
            .expect("writing xml decl should succeed");

        let result = writer.into_inner();
        assert_eq!(
            String::from_utf8(result).expect("utf-8 output"),
            "<?xml version=\"1.2\" encoding=\"utf-X\" standalone=\"yo\"?>",
            "writer output (LHS)"
        );
    }

    /// Written: version, standalone
    #[test]
    fn standalone() {
        let mut writer = Writer::new(Vec::new());
        writer
            .write_event(Decl(BytesDecl::new("1.2", None, Some("yo"))))
            .expect("writing xml decl should succeed");

        let result = writer.into_inner();
        assert_eq!(
            String::from_utf8(result).expect("utf-8 output"),
            "<?xml version=\"1.2\" standalone=\"yo\"?>",
            "writer output (LHS)"
        );
    }

    /// Written: version, encoding
    #[test]
    fn encoding() {
        let mut writer = Writer::new(Vec::new());
        writer
            .write_event(Decl(BytesDecl::new("1.2", Some("utf-X"), None)))
            .expect("writing xml decl should succeed");

        let result = writer.into_inner();
        assert_eq!(
            String::from_utf8(result).expect("utf-8 output"),
            "<?xml version=\"1.2\" encoding=\"utf-X\"?>",
            "writer output (LHS)"
        );
    }

    /// Written: version
    #[test]
    fn version() {
        let mut writer = Writer::new(Vec::new());
        writer
            .write_event(Decl(BytesDecl::new("1.2", None, None)))
            .expect("writing xml decl should succeed");

        let result = writer.into_inner();
        assert_eq!(
            String::from_utf8(result).expect("utf-8 output"),
            "<?xml version=\"1.2\"?>",
            "writer output (LHS)"
        );
    }

    /// This test ensures that empty XML declaration attribute values are not a problem.
    #[test]
    fn empty() {
        let mut writer = Writer::new(Vec::new());
        // An empty version should arguably be an error, but we don't expect anyone to actually supply
        // an empty version.
        writer
            .write_event(Decl(BytesDecl::new("", Some(""), Some(""))))
            .expect("writing xml decl should succeed");

        let result = writer.into_inner();
        assert_eq!(
            String::from_utf8(result).expect("utf-8 output"),
            "<?xml version=\"\" encoding=\"\" standalone=\"\"?>",
            "writer output (LHS)"
        );
    }
}

#[test]
fn pi() {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(PI(BytesPI::new("xml-stylesheet href='theme.xls' ")))
        .expect("writing processing instruction should succeed");

    let result = writer.into_inner();
    assert_eq!(
        String::from_utf8(result).expect("utf-8 output"),
        "<?xml-stylesheet href='theme.xls' ?>",
        "writer output (LHS)"
    );
}

#[test]
fn empty() {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(Empty(
            BytesStart::new("game").with_attributes([("publisher", "Blizzard")]),
        ))
        .expect("writing empty tag should succeed");

    let result = writer.into_inner();
    assert_eq!(
        String::from_utf8(result).expect("utf-8 output"),
        r#"<game publisher="Blizzard"/>"#,
        "writer output (LHS)"
    );
}

#[test]
fn start() {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(Start(
            BytesStart::new("info").with_attributes([("genre", "RTS")]),
        ))
        .expect("writing start tag should succeed");

    let result = writer.into_inner();
    assert_eq!(
        String::from_utf8(result).expect("utf-8 output"),
        r#"<info genre="RTS">"#,
        "writer output (LHS)"
    );
}

#[test]
fn end() {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(End(BytesEnd::new("info")))
        .expect("writing end tag should succeed");

    let result = writer.into_inner();
    assert_eq!(
        String::from_utf8(result).expect("utf-8 output"),
        "</info>",
        "writer output (LHS)"
    );
}

#[test]
fn text() {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(Text(BytesText::new(
            "Kerrigan & Raynor: The Z[erg] programming language",
        )))
        .expect("writing text should succeed");

    let result = writer.into_inner();
    assert_eq!(
        String::from_utf8(result).expect("utf-8 output"),
        "Kerrigan &amp; Raynor: The Z[erg] programming language",
        "writer output (LHS)"
    );
}

#[test]
fn cdata() {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(CData(BytesCData::new(
            "Kerrigan & Raynor: The Z[erg] programming language",
        )))
        .expect("writing CDATA section should succeed");

    let result = writer.into_inner();
    assert_eq!(
        String::from_utf8(result).expect("utf-8 output"),
        "<![CDATA[Kerrigan & Raynor: The Z[erg] programming language]]>",
        "writer output (LHS)"
    );
}

#[test]
fn comment() {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(Comment(BytesText::from_escaped(
            "Kerrigan & Raynor: The Z[erg] programming language",
        )))
        .expect("writing comment should succeed");

    let result = writer.into_inner();
    assert_eq!(
        String::from_utf8(result).expect("utf-8 output"),
        "<!--Kerrigan & Raynor: The Z[erg] programming language-->",
        "writer output (LHS)"
    );
}

#[test]
fn doctype() {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(DocType(BytesText::new("some DTD here...")))
        .expect("writing DTD should succeed");

    let result = writer.into_inner();
    assert_eq!(
        String::from_utf8(result).expect("utf-8 output"),
        "<!DOCTYPE some DTD here...>",
        "writer output (LHS)"
    );
}

#[test]
fn eof() {
    let mut writer = Writer::new(Vec::new());
    writer.write_event(Eof).expect("writing EOF should succeed");

    let result = writer.into_inner();
    assert_eq!(
        String::from_utf8(result).expect("utf-8 output"),
        "",
        "writer output (LHS)"
    );
}

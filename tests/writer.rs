use quick_xml::events::{BytesDecl, Event::*};
use quick_xml::writer::Writer;

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

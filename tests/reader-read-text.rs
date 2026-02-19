use quick_xml::events::{BytesCData, BytesDecl, BytesEnd, BytesPI, BytesStart, BytesText, Event};
use quick_xml::name::QName;
use quick_xml::reader::Reader;

mod borrowed {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Yes, this test contains invalid XML but since we can parse it, we check
    /// that it does not break our parser
    #[test]
    fn decl() {
        let mut reader = Reader::from_str(
            "\
            <root>\
                <?xml version=\"1.0\"?>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::from_content("root", 4)),
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Decl(BytesDecl::new("1.0", None, None))
        );
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Empty(BytesStart::new("element"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }

    /// Yes, this test contains invalid XML but since we can parse it, we check
    /// that it does not break our parser
    #[test]
    fn doctype() {
        let mut reader = Reader::from_str(
            "\
            <root>\
                <!DOCTYPE dtd>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::from_content("root", 4)),
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::DocType(BytesText::new("dtd"))
        );
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Empty(BytesStart::new("element"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }

    #[test]
    fn pi() {
        let mut reader = Reader::from_str(
            "\
            <root>\
                <?pi?>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::from_content("root", 4)),
        );
        assert_eq!(reader.read_event().unwrap(), Event::PI(BytesPI::new("pi")));
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Empty(BytesStart::new("element"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }

    #[test]
    fn comment() {
        let mut reader = Reader::from_str(
            "\
            <root>\
                <!--comment-->\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::from_content("root", 4)),
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Comment(BytesText::new("comment"))
        );
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Empty(BytesStart::new("element"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }

    #[test]
    fn start() {
        let mut reader = Reader::from_str(
            "\
            <root>\
                <tag>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        reader.config_mut().check_end_names = false;
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::from_content("root", 4)),
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::new("tag")),
        );
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Empty(BytesStart::new("element"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }

    #[test]
    fn end() {
        let mut reader = Reader::from_str(
            "\
            <root>\
                </tag>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        reader.config_mut().check_end_names = false;
        reader.config_mut().allow_unmatched_ends = true;
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::from_content("root", 4)),
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::End(BytesEnd::new("tag")),
        );
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Empty(BytesStart::new("element"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }

    #[test]
    fn empty() {
        let mut reader = Reader::from_str(
            "\
            <root>\
                <tag/>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::from_content("root", 4)),
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Empty(BytesStart::new("tag")),
        );
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Empty(BytesStart::new("element"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }

    #[test]
    fn text() {
        let mut reader = Reader::from_str(
            "\
            <root>\
                text\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::from_content("root", 4)),
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new("text"))
        );
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Empty(BytesStart::new("element"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }

    #[test]
    fn cdata() {
        let mut reader = Reader::from_str(
            "\
            <root>\
                <![CDATA[cdata]]>\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::from_content("root", 4)),
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::CData(BytesCData::new("cdata"))
        );
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Empty(BytesStart::new("element"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }

    #[test]
    fn dangling_amp() {
        let mut reader = Reader::from_str(
            "\
            <root>\
                &\
                <root/>\
                <root></root>\
            </root>\
            <element/>",
        );
        reader.config_mut().allow_dangling_amp = true;
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::from_content("root", 4)),
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::from_escaped("&"))
        );
        assert_eq!(
            reader.read_text(QName(b"root")).unwrap(),
            "<root/><root></root>"
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Empty(BytesStart::new("element"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }
}

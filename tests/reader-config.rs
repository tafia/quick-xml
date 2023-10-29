//! Contains tests for config options of a parser.
//!
//! Each module has a name of a corresponding option and functions inside performs
//! testing of various option values.
//!
//! Please keep tests sorted (exceptions are allowed if options are tightly related).

use quick_xml::errors::{Error, IllFormedError};
use quick_xml::events::{BytesCData, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;

mod expand_empty_elements {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Self-closed elements should be reported as one `Empty` event
    #[test]
    fn false_() {
        let mut reader = Reader::from_str("<root/>");
        reader.expand_empty_elements(false);

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Empty(BytesStart::new("root"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }

    /// Self-closed elements should be reported as two events
    #[test]
    fn true_() {
        let mut reader = Reader::from_str("<root/>");
        reader.expand_empty_elements(true);

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::new("root"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::End(BytesEnd::new("root"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }
}

mod trim_markup_names_in_closing_tags {
    use super::*;
    use pretty_assertions::assert_eq;

    mod false_ {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn check_end_names_false() {
            let mut reader = Reader::from_str("<root></root \t\r\n>");
            reader.trim_markup_names_in_closing_tags(false);
            // We need to disable checks, otherwise the error will be returned when read end
            reader.check_end_names(false);

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Start(BytesStart::new("root"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::End(BytesEnd::new("root \t\r\n"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        #[test]
        fn check_end_names_true() {
            let mut reader = Reader::from_str("<root></root \t\r\n>");
            reader.trim_markup_names_in_closing_tags(false);
            reader.check_end_names(true);

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Start(BytesStart::new("root"))
            );
            match reader.read_event() {
                Err(Error::IllFormed(cause)) => assert_eq!(
                    cause,
                    IllFormedError::MismatchedEnd {
                        expected: "root".into(),
                        found: "root \t\r\n".into(),
                    }
                ),
                x => panic!("Expected `Err(IllFormed(_))`, but got `{:?}`", x),
            }
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }
    }

    #[test]
    fn true_() {
        let mut reader = Reader::from_str("<root></root \t\r\n>");
        reader.trim_markup_names_in_closing_tags(true);

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::new("root"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::End(BytesEnd::new("root"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }
}

const XML: &str = " \t\r\n\
<!doctype root \t\r\n> \t\r\n\
<root \t\r\n> \t\r\n\
    <empty \t\r\n/> \t\r\n\
    text \t\r\n\
    <!-- comment \t\r\n--> \t\r\n\
    <![CDATA[ \t\r\ncdata \t\r\n]]> \t\r\n\
    <?pi \t\r\n?> \t\r\n\
</root> \t\r\n";

mod trim_text {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn false_() {
        let mut reader = Reader::from_str(XML);
        reader.trim_text(false);

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new(" \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::DocType(BytesText::new("root \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new(" \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::from_content("root \t\r\n", 4))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new(" \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Empty(BytesStart::from_content("empty \t\r\n", 5))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new(" \t\r\ntext \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Comment(BytesText::new(" comment \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new(" \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::CData(BytesCData::new(" \t\r\ncdata \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new(" \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::PI(BytesText::new("pi \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new(" \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::End(BytesEnd::new("root"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new(" \t\r\n"))
        );

        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }

    #[test]
    fn true_() {
        let mut reader = Reader::from_str(XML);
        reader.trim_text(true);

        assert_eq!(
            reader.read_event().unwrap(),
            Event::DocType(BytesText::new("root \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::from_content("root \t\r\n", 4))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Empty(BytesStart::from_content("empty \t\r\n", 5))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new("text"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Comment(BytesText::new(" comment \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::CData(BytesCData::new(" \t\r\ncdata \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::PI(BytesText::new("pi \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::End(BytesEnd::new("root"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }
}

mod trim_text_end {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn false_() {
        let mut reader = Reader::from_str(XML);
        reader.trim_text_end(false);

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new(" \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::DocType(BytesText::new("root \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new(" \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::from_content("root \t\r\n", 4))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new(" \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Empty(BytesStart::from_content("empty \t\r\n", 5))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new(" \t\r\ntext \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Comment(BytesText::new(" comment \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new(" \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::CData(BytesCData::new(" \t\r\ncdata \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new(" \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::PI(BytesText::new("pi \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new(" \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::End(BytesEnd::new("root"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new(" \t\r\n"))
        );

        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }

    // TODO: Enable test after rewriting parser
    #[test]
    #[ignore = "currently it is hard to fix incorrect behavior, but this will much easy after parser rewrite"]
    fn true_() {
        let mut reader = Reader::from_str(XML);
        reader.trim_text_end(true);

        assert_eq!(
            reader.read_event().unwrap(),
            Event::DocType(BytesText::new("root \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::from_content("root \t\r\n", 4))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Empty(BytesStart::from_content("empty \t\r\n", 5))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::new(" \t\r\ntext"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Comment(BytesText::new(" comment \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::CData(BytesCData::new(" \t\r\ncdata \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::PI(BytesText::new("pi \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::End(BytesEnd::new("root"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }
}

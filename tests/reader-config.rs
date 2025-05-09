//! Contains tests for config options of a parser.
//!
//! Each module has a name of a corresponding option and functions inside performs
//! testing of various option values.
//!
//! Please keep tests sorted (exceptions are allowed if options are tightly related).

use quick_xml::errors::{Error, IllFormedError};
use quick_xml::events::{BytesCData, BytesEnd, BytesPI, BytesRef, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;

mod allow_dangling_amp {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn false_() {
        let mut reader = Reader::from_str("&&&lt;&");
        reader.config_mut().allow_dangling_amp = false;

        match reader.read_event() {
            Err(Error::IllFormed(cause)) => {
                assert_eq!(cause, IllFormedError::UnclosedReference);
            }
            x => panic!("Expected `Err(Syntax(_))`, but got `{:?}`", x),
        }
        assert_eq!(reader.error_position()..reader.buffer_position(), 0..1);

        match reader.read_event() {
            Err(Error::IllFormed(cause)) => {
                assert_eq!(cause, IllFormedError::UnclosedReference);
            }
            x => panic!("Expected `Err(Syntax(_))`, but got `{:?}`", x),
        }
        assert_eq!(reader.error_position()..reader.buffer_position(), 1..2);

        assert_eq!(
            reader.read_event().unwrap(),
            Event::GeneralRef(BytesRef::new("lt"))
        );
        match reader.read_event() {
            Err(Error::IllFormed(cause)) => {
                assert_eq!(cause, IllFormedError::UnclosedReference);
            }
            x => panic!("Expected `Err(Syntax(_))`, but got `{:?}`", x),
        }
        assert_eq!(reader.error_position()..reader.buffer_position(), 6..7);

        assert_eq!(reader.read_event().unwrap(), Event::Eof);
        assert_eq!(reader.error_position()..reader.buffer_position(), 6..7);
    }

    #[test]
    fn true_() {
        let mut reader = Reader::from_str("&&&lt;&");
        reader.config_mut().allow_dangling_amp = true;

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::from_escaped("&"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::from_escaped("&"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::GeneralRef(BytesRef::new("lt"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::from_escaped("&"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }
}

mod allow_unmatched_ends {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn false_() {
        let mut reader = Reader::from_str("<tag></tag></unmatched>");
        reader.config_mut().allow_unmatched_ends = false;

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::new("tag"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::End(BytesEnd::new("tag"))
        );
        match reader.read_event() {
            Err(Error::IllFormed(cause)) => {
                assert_eq!(cause, IllFormedError::UnmatchedEndTag("unmatched".into()));
            }
            x => panic!("Expected `Err(IllFormed(_))`, but got `{:?}`", x),
        }
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }

    #[test]
    fn true_() {
        let mut reader = Reader::from_str("<tag></tag></unmatched>");
        reader.config_mut().allow_unmatched_ends = true;

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::new("tag"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::End(BytesEnd::new("tag"))
        );
        // #770: We want to allow this
        assert_eq!(
            reader.read_event().unwrap(),
            Event::End(BytesEnd::new("unmatched"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }
}

mod check_comments {
    use super::*;

    mod false_ {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn empty() {
            let mut reader = Reader::from_str("<!----><tag/>");
            reader.config_mut().check_comments = false;

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Comment(BytesText::from_escaped(""))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Empty(BytesStart::new("tag"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        #[test]
        fn normal() {
            let mut reader = Reader::from_str("<!-- comment --><tag/>");
            reader.config_mut().check_comments = false;

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Comment(BytesText::from_escaped(" comment "))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Empty(BytesStart::new("tag"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        #[test]
        fn dashes_inside() {
            let mut reader = Reader::from_str("<!-- comment -- --><tag/>");
            reader.config_mut().check_comments = false;

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Comment(BytesText::from_escaped(" comment -- "))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Empty(BytesStart::new("tag"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        #[test]
        fn three_dashes_in_the_end() {
            let mut reader = Reader::from_str("<!-- comment ---><tag/>");
            reader.config_mut().check_comments = false;

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Comment(BytesText::from_escaped(" comment -"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Empty(BytesStart::new("tag"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        #[test]
        fn comment_is_gt() {
            let mut reader = Reader::from_str("<!-->--><tag/>");
            reader.config_mut().check_comments = false;

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Comment(BytesText::from_escaped(">"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Empty(BytesStart::new("tag"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        #[test]
        fn comment_is_dash_gt() {
            let mut reader = Reader::from_str("<!--->--><tag/>");
            reader.config_mut().check_comments = false;

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Comment(BytesText::from_escaped("->"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Empty(BytesStart::new("tag"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }
    }

    mod true_ {
        use super::*;
        use pretty_assertions::assert_eq;

        /// XML grammar allows `<!---->`. The simplified adapted part of full grammar
        /// can be tried online at https://peggyjs.org/online:
        ///
        /// ```pegjs
        /// comment = '<!--' $(char / ('-' char))* '-->'
        /// char = [^-]i
        /// ```
        ///
        /// The original grammar: https://www.w3.org/TR/xml11/#sec-comments
        #[test]
        fn empty() {
            let mut reader = Reader::from_str("<!----><tag/>");
            reader.config_mut().check_comments = true;

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Comment(BytesText::from_escaped(""))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Empty(BytesStart::new("tag"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        #[test]
        fn normal() {
            let mut reader = Reader::from_str("<!-- comment --><tag/>");
            reader.config_mut().check_comments = true;

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Comment(BytesText::from_escaped(" comment "))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Empty(BytesStart::new("tag"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        #[test]
        fn dashes_inside() {
            let mut reader = Reader::from_str("<!-- comment -- --><tag/>");
            reader.config_mut().check_comments = true;

            match reader.read_event() {
                Err(Error::IllFormed(cause)) => {
                    assert_eq!(cause, IllFormedError::DoubleHyphenInComment)
                }
                x => panic!("Expected `Err(IllFormed(_))`, but got `{:?}`", x),
            }
            // #513: We want to continue parsing after the error
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Empty(BytesStart::new("tag"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        #[test]
        fn three_dashes_in_the_end() {
            let mut reader = Reader::from_str("<!-- comment ---><tag/>");
            reader.config_mut().check_comments = true;

            match reader.read_event() {
                Err(Error::IllFormed(cause)) => {
                    assert_eq!(cause, IllFormedError::DoubleHyphenInComment)
                }
                x => panic!("Expected `Err(IllFormed(_))`, but got `{:?}`", x),
            }
            // #513: We want to continue parsing after the error
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Empty(BytesStart::new("tag"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        #[test]
        fn comment_is_gt() {
            let mut reader = Reader::from_str("<!-->--><tag/>");
            reader.config_mut().check_comments = true;

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Comment(BytesText::from_escaped(">"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Empty(BytesStart::new("tag"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        #[test]
        fn comment_is_dash_gt() {
            let mut reader = Reader::from_str("<!--->--><tag/>");
            reader.config_mut().check_comments = true;

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Comment(BytesText::from_escaped("->"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Empty(BytesStart::new("tag"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }
    }
}

mod check_end_names {
    use super::*;

    mod false_ {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn matched_tags() {
            let mut reader = Reader::from_str("<tag><tag></tag></tag>");
            reader.config_mut().check_end_names = false;

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Start(BytesStart::new("tag"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Start(BytesStart::new("tag"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::End(BytesEnd::new("tag"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::End(BytesEnd::new("tag"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        #[test]
        fn mismatched_tags() {
            let mut reader = Reader::from_str("<tag><tag></mismatched></tag>");
            reader.config_mut().check_end_names = false;

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Start(BytesStart::new("tag"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Start(BytesStart::new("tag"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::End(BytesEnd::new("mismatched"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::End(BytesEnd::new("tag"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }
    }

    mod true_ {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn matched_tags() {
            let mut reader = Reader::from_str("<tag><tag></tag></tag>");
            reader.config_mut().check_end_names = false;

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Start(BytesStart::new("tag"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Start(BytesStart::new("tag"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::End(BytesEnd::new("tag"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::End(BytesEnd::new("tag"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        #[test]
        fn mismatched_tags() {
            let mut reader = Reader::from_str("<tag><tag></mismatched></tag>");
            reader.config_mut().check_end_names = true;

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Start(BytesStart::new("tag"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Start(BytesStart::new("tag"))
            );
            match reader.read_event() {
                Err(Error::IllFormed(cause)) => assert_eq!(
                    cause,
                    IllFormedError::MismatchedEndTag {
                        expected: "tag".into(),
                        found: "mismatched".into(),
                    }
                ),
                x => panic!("Expected `Err(IllFormed(_))`, but got `{:?}`", x),
            }
            // #513: We want to continue parsing after the error
            assert_eq!(
                reader.read_event().unwrap(),
                Event::End(BytesEnd::new("tag"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }
    }
}

mod expand_empty_elements {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Self-closed elements should be reported as one `Empty` event
    #[test]
    fn false_() {
        let mut reader = Reader::from_str("<root/>");
        reader.config_mut().expand_empty_elements = false;

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
        reader.config_mut().expand_empty_elements = true;

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
            reader.config_mut().trim_markup_names_in_closing_tags = false;
            // We need to disable checks, otherwise the error will be returned when read end
            reader.config_mut().check_end_names = false;

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
            reader.config_mut().trim_markup_names_in_closing_tags = false;
            reader.config_mut().check_end_names = true;

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Start(BytesStart::new("root"))
            );
            match reader.read_event() {
                Err(Error::IllFormed(cause)) => assert_eq!(
                    cause,
                    IllFormedError::MismatchedEndTag {
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
        reader.config_mut().trim_markup_names_in_closing_tags = true;

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
<!DOCTYPE root \t\r\n> \t\r\n\
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
        reader.config_mut().trim_text(false);

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
            Event::Comment(BytesText::from_escaped(" comment \t\r\n"))
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
            Event::PI(BytesPI::new("pi \t\r\n"))
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
        reader.config_mut().trim_text(true);

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
            Event::Comment(BytesText::from_escaped(" comment \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::CData(BytesCData::new(" \t\r\ncdata \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::PI(BytesPI::new("pi \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::End(BytesEnd::new("root"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }
}

mod trim_text_start {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn false_() {
        let mut reader = Reader::from_str(XML);
        reader.config_mut().trim_text_start = false;

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
            Event::PI(BytesPI::new("pi \t\r\n"))
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
        reader.config_mut().trim_text_start = true;

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
            Event::Text(BytesText::new("text \t\r\n"))
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
            Event::PI(BytesPI::new("pi \t\r\n"))
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
        reader.config_mut().trim_text_end = false;

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
            Event::Comment(BytesText::from_escaped(" comment \t\r\n"))
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
            Event::PI(BytesPI::new("pi \t\r\n"))
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
        reader.config_mut().trim_text_end = true;

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
            Event::Comment(BytesText::from_escaped(" comment \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::CData(BytesCData::new(" \t\r\ncdata \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::PI(BytesPI::new("pi \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::End(BytesEnd::new("root"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }
}

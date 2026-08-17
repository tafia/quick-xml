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

// NOTE: These tests currently do NOT apply XML end-of-line normalization (XML 1.0 Section 2.11).
// Per the spec, `\r\n` must be normalized to `\n` before any other processing, which
// means every `\r\n` in this constant should become `\n` in the parsed output. That
// normalization applies universally: text content, element/attribute whitespace, comments,
// PIs, CDATA, and DOCTYPE. All assertions below that expect `\r\n` in their output are
// therefore incorrect with respect to a spec-compliant parser.
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
    fn trim_text_false() {
        let mut reader = Reader::from_str(XML);
        reader.config_mut().trim_text(false);
        assert_eq!(reader.config().trim_text_start, false);
        assert_eq!(reader.config().trim_text_end, false);

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::from_escaped(" \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::DocType(BytesText::from_escaped("root \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::from_escaped(" \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::from_content("root \t\r\n", 4))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::from_escaped(" \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Empty(BytesStart::from_content("empty \t\r\n", 5))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::from_escaped(" \t\r\ntext \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Comment(BytesText::from_escaped(" comment \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::from_escaped(" \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::CData(BytesCData::new(" \t\r\ncdata \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::from_escaped(" \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::PI(BytesPI::new("pi \t\r\n"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::from_escaped(" \t\r\n"))
        );

        assert_eq!(
            reader.read_event().unwrap(),
            Event::End(BytesEnd::new("root"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::Text(BytesText::from_escaped(" \t\r\n"))
        );

        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }

    // NOTE: Documents CURRENT behavior, which is not spec-correct (see
    // https://github.com/tafia/quick-xml/issues/984 and `mod correctness` below).
    // Two known divergences from correct XML handling:
    //  - Whitespace-only text adjacent to comments, CDATA, and PIs is suppressed,
    //    but that whitespace is text content, not inter-element indentation, so a
    //    correct parser preserves it.
    //  - The mixed-content node around `text` is trimmed on both edges; a correct
    //    parser preserves the whole node and only drops whitespace-only nodes that
    //    sit between distinct elements. But this is effectively a different feature
    //    than what currently exists.
    #[test]
    fn trim_text_true() {
        let mut reader = Reader::from_str(XML);
        reader.config_mut().trim_text(true);
        assert_eq!(reader.config().trim_text_start, true);
        assert_eq!(reader.config().trim_text_end, true);

        assert_eq!(
            reader.read_event().unwrap(),
            Event::DocType(BytesText::from_escaped("root \t\r\n"))
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

    // NOTE: Documents CURRENT (spec-incorrect) behavior, same divergences as
    // `trim_text_true`: leading whitespace is stripped from the mixed-content
    // `text` node and from whitespace-only nodes adjacent to comments/CDATA/PIs,
    // all of which is text content a correct parser preserves. See #984 and
    // `mod correctness`.
    #[test]
    fn trim_text_start() {
        let mut reader = Reader::from_str(XML);
        reader.config_mut().trim_text_start = true;
        assert_eq!(reader.config().trim_text_end, false);

        assert_eq!(
            reader.read_event().unwrap(),
            Event::DocType(BytesText::from_escaped("root \t\r\n"))
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
            Event::Text(BytesText::from_escaped("text \t\r\n"))
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

    // TODO: Enable test after rewriting parser
    // NOTE: The expected values below still do not reflect fully-correct behavior:
    // the mixed-content node around `text` should be preserved whole, and
    // whitespace-only text adjacent to comments/CDATA/PIs should be kept rather
    // than suppressed. See #984 and `mod correctness`.
    #[test]
    #[ignore = "fails due to https://github.com/tafia/quick-xml/issues/984"]
    fn trim_text_end() {
        let mut reader = Reader::from_str(XML);
        reader.config_mut().trim_text_end = true;
        assert_eq!(reader.config().trim_text_start, false);

        assert_eq!(
            reader.read_event().unwrap(),
            Event::DocType(BytesText::from_escaped("root \t\r\n"))
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
            Event::Text(BytesText::from_escaped(" \t\r\ntext"))
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

    // trim_text should only suppress whitespace-only text nodes that represent
    // inter-element indentation (pretty-printing whitespace between start/end/empty
    // elements). It must NOT trim whitespace that is adjacent to "invisible" markup
    // (comments, PIs, CDATA) or entity references, because that whitespace is part
    // of the text content.
    //
    // The fix for https://github.com/tafia/quick-xml/issues/984 should try to fix
    // these tests also.
    mod correctness {
        use super::*;
        use pretty_assertions::assert_eq;

        // Whitespace before and after entity references must not be trimmed.
        // The parser cannot know what a reference expands to as `&amp;` is content,
        // so ` &amp;` must preserve the leading space.
        #[test]
        #[ignore = "broken pending trimming rework / removal"]
        fn trim_text_preserves_whitespace_around_entity_ref() {
            let mut reader = Reader::from_str("<root> &amp; </root>");
            reader.config_mut().trim_text(true);

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Start(BytesStart::new("root"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Text(BytesText::from_escaped(" "))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::GeneralRef(BytesRef::new("amp"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Text(BytesText::from_escaped(" "))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::End(BytesEnd::new("root"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        // Whitespace around comments must not be trimmed from text content. Comments are
        // "invisible", they don't break text. The logical text content of
        // `text <!-- comment --> more` is `text  more` with both spaces preserved.
        #[test]
        #[ignore = "broken pending trimming rework / removal"]
        fn trim_text_preserves_whitespace_around_comment() {
            let mut reader = Reader::from_str("<root>text <!-- comment --> more</root>");
            reader.config_mut().trim_text(true);

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Start(BytesStart::new("root"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Text(BytesText::from_escaped("text "))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Comment(BytesText::from_escaped(" comment "))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Text(BytesText::from_escaped(" more"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::End(BytesEnd::new("root"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        // Same as comments - whitespace around PIs is text content.
        #[test]
        #[ignore = "broken pending trimming rework / removal"]
        fn trim_text_preserves_whitespace_around_pi() {
            let mut reader = Reader::from_str("<root>text <?pi target?> more</root>");
            reader.config_mut().trim_text(true);

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Start(BytesStart::new("root"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Text(BytesText::from_escaped("text "))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::PI(BytesPI::new("pi target"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Text(BytesText::from_escaped(" more"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::End(BytesEnd::new("root"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        // Same as comments - whitespace around CDATA is text content.
        #[test]
        #[ignore = "broken pending trimming rework / removal"]
        fn trim_text_preserves_whitespace_around_cdata() {
            let mut reader = Reader::from_str("<root>text <![CDATA[data]]> more</root>");
            reader.config_mut().trim_text(true);

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Start(BytesStart::new("root"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Text(BytesText::from_escaped("text "))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::CData(BytesCData::new("data"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Text(BytesText::from_escaped(" more"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::End(BytesEnd::new("root"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        // Whitespace-only text between elements IS indentation and should be trimmed.
        // Verify trim still works for the intended case.
        #[test]
        fn trim_text_removes_inter_element_whitespace() {
            let mut reader = Reader::from_str("<root>\n  <child/>\n</root>");
            reader.config_mut().trim_text(true);

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Start(BytesStart::new("root"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Empty(BytesStart::new("child"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::End(BytesEnd::new("root"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        // Mixed content: whitespace flanking real text is part of the character
        // data, not inter-element indentation, so it must be preserved even
        // directly against the enclosing element's tags. Only whitespace-only
        // nodes between distinct elements are indentation (see
        // `trim_text_removes_inter_element_whitespace`).
        // NOTE: This behavior would effectively encode a different feature from
        // what currently exists / is possible with `trim_text_start` & `trim_text_end`
        #[test]
        #[ignore = "broken pending trimming rework / removal"]
        fn trim_text_preserves_whitespace_around_text_content() {
            let mut reader = Reader::from_str("<root> text </root>");
            reader.config_mut().trim_text(true);

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Start(BytesStart::new("root"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Text(BytesText::from_escaped(" text "))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::End(BytesEnd::new("root"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }

        // Whitespace-only content of a leaf element is that element's entire text
        // content, not indentation between distinct elements, so it is preserved.
        // NOTE: this is a deliberate content-fidelity choice. The whitespace here
        // is indistinguishable from pretty-printing, but `<root>`/`</root>` are the
        // same element's tags (not two distinct elements), so nothing marks it as
        // ignorable and we keep it. Revisit if the #984 rework decides otherwise.
        #[test]
        #[ignore = "broken pending trimming rework / removal"]
        fn trim_text_preserves_whitespace_only_leaf_content() {
            let mut reader = Reader::from_str("<root> </root>");
            reader.config_mut().trim_text(true);

            assert_eq!(
                reader.read_event().unwrap(),
                Event::Start(BytesStart::new("root"))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::Text(BytesText::from_escaped(" "))
            );
            assert_eq!(
                reader.read_event().unwrap(),
                Event::End(BytesEnd::new("root"))
            );
            assert_eq!(reader.read_event().unwrap(), Event::Eof);
        }
    }
}

//! Contains tests that produces errors during parsing XML.

use quick_xml::errors::{Error, SyntaxError};
use quick_xml::events::{BytesCData, BytesDecl, BytesEnd, BytesPI, BytesStart, BytesText, Event};
use quick_xml::reader::{NsReader, Reader};

// For event_ok and syntax_err macros
mod helpers;

mod syntax {
    use super::*;

    mod tag {
        use super::*;

        syntax_err!(unclosed1(".<")   => 1, SyntaxError::UnclosedTag);
        syntax_err!(unclosed2(".</")  => SyntaxError::UnclosedTag);
        syntax_err!(unclosed3(".<x")  => SyntaxError::UnclosedTag);
        syntax_err!(unclosed4(".< ")  => SyntaxError::UnclosedTag);
        syntax_err!(unclosed5(".<\t") => SyntaxError::UnclosedTag);
        syntax_err!(unclosed6(".<\r") => SyntaxError::UnclosedTag);
        syntax_err!(unclosed7(".<\n") => SyntaxError::UnclosedTag);
        syntax_err!(unclosed8(".< \t\r\nx") => SyntaxError::UnclosedTag);

        /// Closed tags can be tested only in pair with open tags, because otherwise
        /// `IllFormedError::UnmatchedEndTag` will be raised
        mod normal {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn borrowed() {
                let mut reader = Reader::from_str("<></>");
                assert_eq!(
                    reader.read_event().unwrap(),
                    Event::Start(BytesStart::new(""))
                );
                assert_eq!(reader.read_event().unwrap(), Event::End(BytesEnd::new("")));
            }

            #[test]
            fn buffered() {
                let mut buf = Vec::new();
                let mut reader = Reader::from_str("<></>");
                assert_eq!(
                    reader.read_event_into(&mut buf).unwrap(),
                    Event::Start(BytesStart::new(""))
                );
                assert_eq!(
                    reader.read_event_into(&mut buf).unwrap(),
                    Event::End(BytesEnd::new(""))
                );
            }

            #[cfg(feature = "async-tokio")]
            #[tokio::test]
            async fn async_tokio() {
                let mut buf = Vec::new();
                let mut reader = Reader::from_str("<></>");
                assert_eq!(
                    reader.read_event_into_async(&mut buf).await.unwrap(),
                    Event::Start(BytesStart::new(""))
                );
                assert_eq!(
                    reader.read_event_into_async(&mut buf).await.unwrap(),
                    Event::End(BytesEnd::new(""))
                );
            }
        }

        // TODO: errors reported at start of a tag, but is should be reported at start of attribute value
        mod attribute {
            use super::*;

            syntax_err!(single_quoted(".<tag a='")  => SyntaxError::UnclosedSingleQuotedAttributeValue);
            syntax_err!(double_quoted(".<tag a=\"") => SyntaxError::UnclosedDoubleQuotedAttributeValue);
        }
    }

    // Incorrect after-bang symbol is detected early, so buffer_position() stay at `!`
    syntax_err!(unclosed_bang1(".<!")   => 1, SyntaxError::InvalidBangMarkup);
    syntax_err!(unclosed_bang2(".<!>")  => 1, SyntaxError::InvalidBangMarkup);
    syntax_err!(unclosed_bang3(".<!a")  => 1, SyntaxError::InvalidBangMarkup);
    syntax_err!(unclosed_bang4(".<!a>") => 1, SyntaxError::InvalidBangMarkup);

    /// https://www.w3.org/TR/xml11/#NT-Comment
    mod comment {
        use super::*;

        syntax_err!(unclosed01(".<!-")    => SyntaxError::UnclosedComment);
        syntax_err!(unclosed02(".<!--")   => SyntaxError::UnclosedComment);
        syntax_err!(unclosed03(".<!->")   => SyntaxError::UnclosedComment);
        syntax_err!(unclosed04(".<!-a")   => SyntaxError::UnclosedComment);
        syntax_err!(unclosed05(".<!---")  => SyntaxError::UnclosedComment);
        syntax_err!(unclosed06(".<!-->")  => SyntaxError::UnclosedComment);
        syntax_err!(unclosed07(".<!--b")  => SyntaxError::UnclosedComment);
        syntax_err!(unclosed08(".<!----") => SyntaxError::UnclosedComment);
        syntax_err!(unclosed09(".<!--->") => SyntaxError::UnclosedComment);
        syntax_err!(unclosed10(".<!---c") => SyntaxError::UnclosedComment);

        event_ok!(normal1("<!---->")     => 7: Event::Comment(BytesText::new("")));
        event_ok!(normal2("<!---->rest") => 7: Event::Comment(BytesText::new("")));
    }

    /// https://www.w3.org/TR/xml11/#NT-CDSect
    mod cdata {
        use super::*;

        syntax_err!(unclosed01(".<![")         => SyntaxError::UnclosedCData);
        syntax_err!(unclosed02(".<![C")        => SyntaxError::UnclosedCData);
        syntax_err!(unclosed03(".<![a")        => SyntaxError::UnclosedCData);
        syntax_err!(unclosed04(".<![>")        => SyntaxError::UnclosedCData);
        syntax_err!(unclosed05(".<![CD")       => SyntaxError::UnclosedCData);
        syntax_err!(unclosed06(".<![Cb")       => SyntaxError::UnclosedCData);
        syntax_err!(unclosed07(".<![C>")       => SyntaxError::UnclosedCData);
        syntax_err!(unclosed08(".<![CDA")      => SyntaxError::UnclosedCData);
        syntax_err!(unclosed09(".<![CDc")      => SyntaxError::UnclosedCData);
        syntax_err!(unclosed10(".<![CD>")      => SyntaxError::UnclosedCData);
        syntax_err!(unclosed11(".<![CDAT")     => SyntaxError::UnclosedCData);
        syntax_err!(unclosed12(".<![CDAd")     => SyntaxError::UnclosedCData);
        syntax_err!(unclosed13(".<![CDA>")     => SyntaxError::UnclosedCData);
        syntax_err!(unclosed14(".<![CDATA")    => SyntaxError::UnclosedCData);
        syntax_err!(unclosed15(".<![CDATe")    => SyntaxError::UnclosedCData);
        syntax_err!(unclosed16(".<![CDAT>")    => SyntaxError::UnclosedCData);
        syntax_err!(unclosed17(".<![CDATA[")   => SyntaxError::UnclosedCData);
        syntax_err!(unclosed18(".<![CDATAf")   => SyntaxError::UnclosedCData);
        syntax_err!(unclosed19(".<![CDATA>")   => SyntaxError::UnclosedCData);
        syntax_err!(unclosed20(".<![CDATA[]")  => SyntaxError::UnclosedCData);
        syntax_err!(unclosed21(".<![CDATA[g")  => SyntaxError::UnclosedCData);
        syntax_err!(unclosed22(".<![CDATA[>")  => SyntaxError::UnclosedCData);
        syntax_err!(unclosed23(".<![CDATA[]]") => SyntaxError::UnclosedCData);
        syntax_err!(unclosed24(".<![CDATA[]h") => SyntaxError::UnclosedCData);
        syntax_err!(unclosed25(".<![CDATA[]>") => SyntaxError::UnclosedCData);

        syntax_err!(lowercase(".<![cdata[]]>") => SyntaxError::UnclosedCData);

        event_ok!(normal1("<![CDATA[]]>")     => 12: Event::CData(BytesCData::new("")));
        event_ok!(normal2("<![CDATA[]]>rest") => 12: Event::CData(BytesCData::new("")));
    }

    /// According to the grammar, only upper-case letters allowed for DOCTYPE writing.
    ///
    /// https://www.w3.org/TR/xml11/#NT-doctypedecl
    mod doctype {
        use super::*;

        syntax_err!(unclosed01(".<!D")         => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed02(".<!DO")        => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed03(".<!Da")        => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed04(".<!D>")        => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed05(".<!DOC")       => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed06(".<!DOb")       => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed07(".<!DO>")       => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed08(".<!DOCT")      => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed09(".<!DOCc")      => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed10(".<!DOC>")      => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed11(".<!DOCTY")     => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed12(".<!DOCTd")     => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed13(".<!DOCT>")     => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed14(".<!DOCTYP")    => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed15(".<!DOCTYe")    => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed16(".<!DOCTY>")    => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed17(".<!DOCTYPE")   => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed18(".<!DOCTYPf")   => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed19(".<!DOCTYP>")   => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed20(".<!DOCTYPE ")  => SyntaxError::UnclosedDoctype);
        syntax_err!(unclosed21(".<!DOCTYPEg")  => SyntaxError::UnclosedDoctype);
        // <!DOCTYPE> results in IllFormed(MissingDoctypeName), checked below
        syntax_err!(unclosed22(".<!DOCTYPE e") => SyntaxError::UnclosedDoctype);

        // According to the grammar, XML declaration MUST contain at least one space
        // and an element name, but we do not consider this as a _syntax_ error.
        event_ok!(normal1("<!DOCTYPE e>")     => 12: Event::DocType(BytesText::new("e")));
        event_ok!(normal2("<!DOCTYPE e>rest") => 12: Event::DocType(BytesText::new("e")));
    }

    /// https://www.w3.org/TR/xml11/#NT-PI
    mod pi {
        use super::*;

        syntax_err!(unclosed01(".<?")   => SyntaxError::UnclosedPI);
        syntax_err!(unclosed02(".<??")  => SyntaxError::UnclosedPI);
        syntax_err!(unclosed03(".<?>")  => SyntaxError::UnclosedPI);
        syntax_err!(unclosed04(".<?<")  => SyntaxError::UnclosedPI);
        syntax_err!(unclosed05(".<?&")  => SyntaxError::UnclosedPI);
        syntax_err!(unclosed06(".<?p")  => SyntaxError::UnclosedPI);
        syntax_err!(unclosed07(".<? ")  => SyntaxError::UnclosedPI);
        syntax_err!(unclosed08(".<?\t") => SyntaxError::UnclosedPI);
        syntax_err!(unclosed09(".<?\r") => SyntaxError::UnclosedPI);
        syntax_err!(unclosed10(".<?\n") => SyntaxError::UnclosedPI);

        // According to the grammar, processing instruction MUST contain a non-empty
        // target name, but we do not consider this as a _syntax_ error.
        event_ok!(normal_empty1("<??>")        => 4: Event::PI(BytesPI::new("")));
        event_ok!(normal_empty2("<??>rest")    => 4: Event::PI(BytesPI::new("")));
        event_ok!(normal_xmlx1("<?xmlx?>")     => 8: Event::PI(BytesPI::new("xmlx")));
        event_ok!(normal_xmlx2("<?xmlx?>rest") => 8: Event::PI(BytesPI::new("xmlx")));
    }

    /// https://www.w3.org/TR/xml11/#NT-prolog
    mod decl {
        use super::*;

        syntax_err!(unclosed1(".<?x")     => SyntaxError::UnclosedPI);
        syntax_err!(unclosed2(".<?xm")    => SyntaxError::UnclosedPI);
        syntax_err!(unclosed3(".<?xml")   => SyntaxError::UnclosedXmlDecl);
        syntax_err!(unclosed4(".<?xml?")  => SyntaxError::UnclosedXmlDecl);
        syntax_err!(unclosed5(".<?xml ")  => SyntaxError::UnclosedXmlDecl);
        syntax_err!(unclosed6(".<?xml\t") => SyntaxError::UnclosedXmlDecl);
        syntax_err!(unclosed7(".<?xml\r") => SyntaxError::UnclosedXmlDecl);
        syntax_err!(unclosed8(".<?xml\n") => SyntaxError::UnclosedXmlDecl);
        // "xmls" is a PI target, not an XML declaration
        syntax_err!(unclosed9(".<?xmls")  => SyntaxError::UnclosedPI);

        // According to the grammar, XML declaration MUST contain at least one space
        // and `version` attribute, but we do not consider this as a _syntax_ error.
        event_ok!(normal1("<?xml?>")       => 7: Event::Decl(BytesDecl::from_start(BytesStart::new("xml"))));
        event_ok!(normal2("<?xml ?>")      => 8: Event::Decl(BytesDecl::from_start(BytesStart::from_content("xml ", 3))));
        event_ok!(normal3("<?xml\t?>")     => 8: Event::Decl(BytesDecl::from_start(BytesStart::from_content("xml\t", 3))));
        event_ok!(normal4("<?xml\r?>")     => 8: Event::Decl(BytesDecl::from_start(BytesStart::from_content("xml\r", 3))));
        event_ok!(normal5("<?xml\n?>")     => 8: Event::Decl(BytesDecl::from_start(BytesStart::from_content("xml\n", 3))));
        event_ok!(normal6("<?xml\n?>rest") => 8: Event::Decl(BytesDecl::from_start(BytesStart::from_content("xml\n", 3))));
    }

    /// Tests for UTF-16 encoded XML declarations.
    /// FIXME: Add support for UTF-8/ASCII incompatible encodings (UTF-16)
    mod decl_utf16 {
        use super::*;
        use pretty_assertions::assert_eq;

        /// UTF-16 LE encoded `<?xml ` (with BOM)
        /// BOM (FF FE) + '<' (3C 00) + '?' (3F 00) + 'x' (78 00) + 'm' (6D 00) + 'l' (6C 00) + ' ' (20 00)
        const UTF16_LE_XML_DECL: &[u8] = &[
            0xFF, 0xFE, // BOM
            0x3C, 0x00, // <
            0x3F, 0x00, // ?
            0x78, 0x00, // x
            0x6D, 0x00, // m
            0x6C, 0x00, // l
            0x20, 0x00, // space
        ];

        /// UTF-16 BE encoded `<?xml ` (with BOM)
        /// BOM (FE FF) + '<' (00 3C) + '?' (00 3F) + 'x' (00 78) + 'm' (00 6D) + 'l' (00 6C) + ' ' (00 20)
        const UTF16_BE_XML_DECL: &[u8] = &[
            0xFE, 0xFF, // BOM
            0x00, 0x3C, // <
            0x00, 0x3F, // ?
            0x00, 0x78, // x
            0x00, 0x6D, // m
            0x00, 0x6C, // l
            0x00, 0x20, // space
        ];

        #[test]
        #[ignore = "UTF-16 support not yet implemented for XML declaration detection"]
        fn utf16_le_unclosed_xml_decl() {
            let mut reader = Reader::from_reader(UTF16_LE_XML_DECL);
            match reader.read_event() {
                Err(Error::Syntax(cause)) => {
                    assert_eq!(cause, SyntaxError::UnclosedXmlDecl);
                }
                x => panic!("Expected `Err(Syntax(UnclosedXmlDecl))`, but got {:?}", x),
            }
        }

        #[test]
        #[ignore = "UTF-16 support not yet implemented for XML declaration detection"]
        fn utf16_be_unclosed_xml_decl() {
            let mut reader = Reader::from_reader(UTF16_BE_XML_DECL);
            match reader.read_event() {
                Err(Error::Syntax(cause)) => {
                    assert_eq!(cause, SyntaxError::UnclosedXmlDecl);
                }
                x => panic!("Expected `Err(Syntax(UnclosedXmlDecl))`, but got {:?}", x),
            }
        }
    }
}

mod ill_formed {
    use super::*;
    use quick_xml::errors::IllFormedError;

    macro_rules! err {
        ($test:ident($xml:literal) => $pos:literal : $cause:expr) => {
            mod $test {
                use super::*;

                mod reader {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[test]
                    fn borrowed() {
                        let xml = concat!($xml, "<x/>");
                        let mut reader = Reader::from_str(xml);
                        reader.config_mut().enable_all_checks(true);
                        match reader.read_event() {
                            Err(Error::IllFormed(cause)) => assert_eq!(
                                (cause, reader.error_position(), reader.buffer_position()),
                                ($cause, $pos, $xml.len() as u64),
                            ),
                            x => panic!("Expected `Err(IllFormed(_))`, but got {:?}", x),
                        }
                        assert_eq!(
                            reader.read_event().expect(
                                "parsing should be possible to continue after `Error::IllFormed`"
                            ),
                            Event::Empty(BytesStart::new("x"))
                        );
                        assert_eq!(
                            reader.buffer_position(),
                            xml.len() as u64,
                            ".buffer_position() is incorrect in the end"
                        );
                    }

                    #[test]
                    fn buffered() {
                        let xml = concat!($xml, "<x/>");
                        let mut buf = Vec::new();
                        let mut reader = Reader::from_str(xml);
                        reader.config_mut().enable_all_checks(true);
                        match reader.read_event_into(&mut buf) {
                            Err(Error::IllFormed(cause)) => assert_eq!(
                                (cause, reader.error_position(), reader.buffer_position()),
                                ($cause, $pos, $xml.len() as u64),
                            ),
                            x => panic!("Expected `Err(IllFormed(_))`, but got {:?}", x),
                        }
                        assert_eq!(
                            reader.read_event_into(&mut buf).expect(
                                "parsing should be possible to continue after `Error::IllFormed`"
                            ),
                            Event::Empty(BytesStart::new("x"))
                        );
                        assert_eq!(
                            reader.buffer_position(),
                            xml.len() as u64,
                            ".buffer_position() is incorrect in the end"
                        );
                    }

                    #[cfg(feature = "async-tokio")]
                    #[tokio::test]
                    async fn async_tokio() {
                        let xml = concat!($xml, "<x/>");
                        let mut buf = Vec::new();
                        let mut reader = Reader::from_str(xml);
                        reader.config_mut().enable_all_checks(true);
                        match reader.read_event_into_async(&mut buf).await {
                            Err(Error::IllFormed(cause)) => assert_eq!(
                                (cause, reader.error_position(), reader.buffer_position()),
                                ($cause, $pos, $xml.len() as u64),
                            ),
                            x => panic!("Expected `Err(IllFormed(_))`, but got {:?}", x),
                        }
                        assert_eq!(
                            reader.read_event_into_async(&mut buf).await.expect(
                                "parsing should be possible to continue after `Error::IllFormed`"
                            ),
                            Event::Empty(BytesStart::new("x"))
                        );
                        assert_eq!(
                            reader.buffer_position(),
                            xml.len() as u64,
                            ".buffer_position() is incorrect in the end"
                        );
                    }
                }

                mod ns_reader {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[test]
                    fn borrowed() {
                        let xml = concat!($xml, "<x/>");
                        let mut reader = NsReader::from_str(xml);
                        reader.config_mut().enable_all_checks(true);
                        match reader.read_resolved_event() {
                            Err(Error::IllFormed(cause)) => assert_eq!(
                                (cause, reader.error_position(), reader.buffer_position()),
                                ($cause, $pos, $xml.len() as u64),
                            ),
                            x => panic!("Expected `Err(IllFormed(_))`, but got {:?}", x),
                        }
                        assert_eq!(
                            reader
                                .read_resolved_event()
                                .expect(
                                    "parsing should be possible to continue after `Error::IllFormed`"
                                )
                                .1,
                            Event::Empty(BytesStart::new("x"))
                        );
                        assert_eq!(
                            reader.buffer_position(),
                            xml.len() as u64,
                            ".buffer_position() is incorrect in the end"
                        );
                    }

                    #[test]
                    fn buffered() {
                        let xml = concat!($xml, "<x/>");
                        let mut buf = Vec::new();
                        let mut reader = NsReader::from_str(xml);
                        reader.config_mut().enable_all_checks(true);
                        match reader.read_resolved_event_into(&mut buf) {
                            Err(Error::IllFormed(cause)) => assert_eq!(
                                (cause, reader.error_position(), reader.buffer_position()),
                                ($cause, $pos, $xml.len() as u64),
                            ),
                            x => panic!("Expected `Err(IllFormed(_))`, but got {:?}", x),
                        }
                        assert_eq!(
                            reader
                                .read_resolved_event_into(&mut buf)
                                .expect(
                                    "parsing should be possible to continue after `Error::IllFormed`"
                                )
                                .1,
                            Event::Empty(BytesStart::new("x"))
                        );
                        assert_eq!(
                            reader.buffer_position(),
                            xml.len() as u64,
                            ".buffer_position() is incorrect in the end"
                        );
                    }

                    #[cfg(feature = "async-tokio")]
                    #[tokio::test]
                    async fn async_tokio() {
                        let xml = concat!($xml, "<x/>");
                        let mut buf = Vec::new();
                        let mut reader = NsReader::from_str(xml);
                        reader.config_mut().enable_all_checks(true);
                        match reader.read_resolved_event_into_async(&mut buf).await {
                            Err(Error::IllFormed(cause)) => assert_eq!(
                                (cause, reader.error_position(), reader.buffer_position()),
                                ($cause, $pos, $xml.len() as u64),
                            ),
                            x => panic!("Expected `Err(IllFormed(_))`, but got {:?}", x),
                        }
                        assert_eq!(
                            reader
                                .read_resolved_event_into_async(&mut buf)
                                .await
                                .expect(
                                    "parsing should be possible to continue after `Error::IllFormed`"
                                )
                                .1,
                            Event::Empty(BytesStart::new("x"))
                        );
                        assert_eq!(
                            reader.buffer_position(),
                            xml.len() as u64,
                            ".buffer_position() is incorrect in the end"
                        );
                    }
                }
            }
        };
    }

    /// Performs 3 reads, the first and third ones should be successful
    macro_rules! err2 {
        ($test:ident($xml:literal) => $pos:literal : $cause:expr) => {
            mod $test {
                use super::*;

                mod reader {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[test]
                    fn borrowed() {
                        let xml = concat!($xml, "<x/>");
                        let mut reader = Reader::from_str(xml);
                        reader.config_mut().enable_all_checks(true);
                        reader.read_event().expect("first .read_event()");
                        match reader.read_event() {
                            Err(Error::IllFormed(cause)) => assert_eq!(
                                (cause, reader.error_position(), reader.buffer_position()),
                                ($cause, $pos, $xml.len() as u64),
                            ),
                            x => panic!("Expected `Err(IllFormed(_))`, but got {:?}", x),
                        }
                        assert_eq!(
                            reader.read_event().expect(
                                "parsing should be possible to continue after `Error::IllFormed`"
                            ),
                            Event::Empty(BytesStart::new("x"))
                        );
                        assert_eq!(
                            reader.buffer_position(),
                            xml.len() as u64,
                            ".buffer_position() is incorrect in the end"
                        );
                    }

                    #[test]
                    fn buffered() {
                        let xml = concat!($xml, "<x/>");
                        let mut buf = Vec::new();
                        let mut reader = Reader::from_str(xml);
                        reader.config_mut().enable_all_checks(true);
                        reader
                            .read_event_into(&mut buf)
                            .expect("first .read_event_into()");
                        match reader.read_event_into(&mut buf) {
                            Err(Error::IllFormed(cause)) => assert_eq!(
                                (cause, reader.error_position(), reader.buffer_position()),
                                ($cause, $pos, $xml.len() as u64),
                            ),
                            x => panic!("Expected `Err(IllFormed(_))`, but got {:?}", x),
                        }
                        assert_eq!(
                            reader.read_event_into(&mut buf).expect(
                                "parsing should be possible to continue after `Error::IllFormed`"
                            ),
                            Event::Empty(BytesStart::new("x"))
                        );
                        assert_eq!(
                            reader.buffer_position(),
                            xml.len() as u64,
                            ".buffer_position() is incorrect in the end"
                        );
                    }

                    #[cfg(feature = "async-tokio")]
                    #[tokio::test]
                    async fn async_tokio() {
                        let xml = concat!($xml, "<x/>");
                        let mut buf = Vec::new();
                        let mut reader = Reader::from_str(xml);
                        reader.config_mut().enable_all_checks(true);
                        reader
                            .read_event_into_async(&mut buf)
                            .await
                            .expect("first .read_event_into_async()");
                        match reader.read_event_into_async(&mut buf).await {
                            Err(Error::IllFormed(cause)) => assert_eq!(
                                (cause, reader.error_position(), reader.buffer_position()),
                                ($cause, $pos, $xml.len() as u64),
                            ),
                            x => panic!("Expected `Err(IllFormed(_))`, but got {:?}", x),
                        }
                        assert_eq!(
                            reader.read_event_into_async(&mut buf).await.expect(
                                "parsing should be possible to continue after `Error::IllFormed`"
                            ),
                            Event::Empty(BytesStart::new("x"))
                        );
                        assert_eq!(
                            reader.buffer_position(),
                            xml.len() as u64,
                            ".buffer_position() is incorrect in the end"
                        );
                    }
                }

                mod ns_reader {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[test]
                    fn borrowed() {
                        let xml = concat!($xml, "<x/>");
                        let mut reader = NsReader::from_str(xml);
                        reader.config_mut().enable_all_checks(true);
                        reader
                            .read_resolved_event()
                            .expect("first .read_resolved_event()");
                        match reader.read_resolved_event() {
                            Err(Error::IllFormed(cause)) => assert_eq!(
                                (cause, reader.error_position(), reader.buffer_position()),
                                ($cause, $pos, $xml.len() as u64),
                            ),
                            x => panic!("Expected `Err(IllFormed(_))`, but got {:?}", x),
                        }
                        assert_eq!(
                            reader
                                .read_resolved_event()
                                .expect(
                                    "parsing should be possible to continue after `Error::IllFormed`"
                                )
                                .1,
                            Event::Empty(BytesStart::new("x"))
                        );
                        assert_eq!(
                            reader.buffer_position(),
                            xml.len() as u64,
                            ".buffer_position() is incorrect in the end"
                        );
                    }

                    #[test]
                    fn buffered() {
                        let xml = concat!($xml, "<x/>");
                        let mut buf = Vec::new();
                        let mut reader = NsReader::from_str(xml);
                        reader.config_mut().enable_all_checks(true);
                        reader
                            .read_resolved_event_into(&mut buf)
                            .expect("first .read_resolved_event_into()");
                        match reader.read_resolved_event_into(&mut buf) {
                            Err(Error::IllFormed(cause)) => assert_eq!(
                                (cause, reader.error_position(), reader.buffer_position()),
                                ($cause, $pos, $xml.len() as u64),
                            ),
                            x => panic!("Expected `Err(IllFormed(_))`, but got {:?}", x),
                        }
                        assert_eq!(
                            reader
                                .read_resolved_event_into(&mut buf)
                                .expect(
                                    "parsing should be possible to continue after `Error::IllFormed`"
                                )
                                .1,
                            Event::Empty(BytesStart::new("x"))
                        );
                        assert_eq!(
                            reader.buffer_position(),
                            xml.len() as u64,
                            ".buffer_position() is incorrect in the end"
                        );
                    }

                    #[cfg(feature = "async-tokio")]
                    #[tokio::test]
                    async fn async_tokio() {
                        let xml = concat!($xml, "<x/>");
                        let mut buf = Vec::new();
                        let mut reader = NsReader::from_str(xml);
                        reader.config_mut().enable_all_checks(true);
                        reader
                            .read_resolved_event_into_async(&mut buf)
                            .await
                            .expect("first .read_resolved_event_into_async()");
                        match reader.read_resolved_event_into_async(&mut buf).await {
                            Err(Error::IllFormed(cause)) => assert_eq!(
                                (cause, reader.error_position(), reader.buffer_position()),
                                ($cause, $pos, $xml.len() as u64),
                            ),
                            x => panic!("Expected `Err(IllFormed(_))`, but got {:?}", x),
                        }
                        assert_eq!(
                            reader
                                .read_resolved_event_into_async(&mut buf)
                                .await
                                .expect(
                                    "parsing should be possible to continue after `Error::IllFormed`"
                                )
                                .1,
                            Event::Empty(BytesStart::new("x"))
                        );
                        assert_eq!(
                            reader.buffer_position(),
                            xml.len() as u64,
                            ".buffer_position() is incorrect in the end"
                        );
                    }
                }
            }
        };
    }

    // IllFormedError::MissingDeclVersion is generated lazily when you call `BytesDecl::version()`

    err!(missing_doctype_name1("<!DOCTYPE>") => 9: IllFormedError::MissingDoctypeName);
    //                                   ^= 9
    err!(missing_doctype_name2("<!DOCTYPE \t\r\n>") => 13: IllFormedError::MissingDoctypeName);
    //                                          ^= 13
    event_ok!(missing_doctype_name3("<!DOCTYPE \t\r\nx>") => 15: Event::DocType(BytesText::new("x")));

    err2!(unmatched_end_tag1(".</>") => 1: IllFormedError::UnmatchedEndTag("".to_string()));
    err2!(unmatched_end_tag2(".</end>") => 1: IllFormedError::UnmatchedEndTag("end".to_string()));
    err2!(unmatched_end_tag3(".</end >") => 1: IllFormedError::UnmatchedEndTag("end".to_string()));

    event_ok!(mismatched_end_tag1("<start></start>") => 7: Event::Start(BytesStart::new("start")));
    err2!(mismatched_end_tag2("<start></>") => 7: IllFormedError::MismatchedEndTag {
        //                            ^= 7
        expected: "start".to_string(),
        found: "".to_string(),
    });
    err2!(mismatched_end_tag3("<start></end>") => 7: IllFormedError::MismatchedEndTag {
        //                            ^= 7
        expected: "start".to_string(),
        found: "end".to_string(),
    });
    err2!(mismatched_end_tag4("<start></end >") => 7: IllFormedError::MismatchedEndTag {
        //                            ^= 7
        expected: "start".to_string(),
        found: "end".to_string(),
    });

    event_ok!(double_hyphen_in_comment1("<!---->") => 7: Event::Comment(BytesText::new("")));
    err!(double_hyphen_in_comment2("<!----->") => 4: IllFormedError::DoubleHyphenInComment);
    //                                  ^= 4
    err!(double_hyphen_in_comment3("<!-- --->") => 5: IllFormedError::DoubleHyphenInComment);
    //                                   ^= 5
    err!(double_hyphen_in_comment4("<!-- -- -->") => 5: IllFormedError::DoubleHyphenInComment);
    //                                   ^= 5

    mod reference {
        use super::*;
        use quick_xml::events::BytesRef;

        err2!(unclosed1(".&")        => 1: IllFormedError::UnclosedReference);
        err2!(unclosed2(".&x")       => 1: IllFormedError::UnclosedReference);
        err2!(unclosed_num(".&#")    => 1: IllFormedError::UnclosedReference);
        err2!(unclosed_dec(".&#2")   => 1: IllFormedError::UnclosedReference);
        err2!(unclosed_hex1(".&#x")  => 1: IllFormedError::UnclosedReference);
        err2!(unclosed_hex2(".&#xF") => 1: IllFormedError::UnclosedReference);

        // We do not check correctness of references during parsing
        event_ok!(empty("&;")   =>      2: Event::GeneralRef(BytesRef::new("")));
        event_ok!(normal1("&x;") =>     3: Event::GeneralRef(BytesRef::new("x")));
        event_ok!(normal2("&x;rest") => 3: Event::GeneralRef(BytesRef::new("x")));
        event_ok!(num("&#;")    =>      3: Event::GeneralRef(BytesRef::new("#")));
        event_ok!(dec("&#2;")   =>      4: Event::GeneralRef(BytesRef::new("#2")));
        event_ok!(hex1("&#x;")  =>      4: Event::GeneralRef(BytesRef::new("#x")));
        event_ok!(hex2("&#xF;") =>      5: Event::GeneralRef(BytesRef::new("#xF")));

        // XML specification explicitly allowed any number of leading zeroes
        event_ok!(long_dec("&#00000000000000000000000000000000000000032;")  => 44: Event::GeneralRef(BytesRef::new("#00000000000000000000000000000000000000032")));
        event_ok!(long_hex("&#x00000000000000000000000000000000000000020;") => 45: Event::GeneralRef(BytesRef::new("#x00000000000000000000000000000000000000020")));
    }
}

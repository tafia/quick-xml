//! Contains tests that produces errors during parsing XML.

use quick_xml::errors::{Error, SyntaxError};
use quick_xml::events::{BytesCData, BytesDecl, BytesEnd, BytesPI, BytesStart, BytesText, Event};
use quick_xml::reader::{NsReader, Reader};

macro_rules! ok {
    ($test:ident($xml:literal) => $event:expr) => {
        mod $test {
            use super::*;

            mod reader {
                use super::*;
                use pretty_assertions::assert_eq;

                #[test]
                fn borrowed() {
                    let mut reader = Reader::from_str($xml);
                    reader.config_mut().enable_all_checks(true);
                    assert_eq!(reader.read_event().unwrap(), $event);
                }

                #[test]
                fn buffered() {
                    let mut buf = Vec::new();
                    let mut reader = Reader::from_str($xml);
                    reader.config_mut().enable_all_checks(true);
                    assert_eq!(reader.read_event_into(&mut buf).unwrap(), $event);
                }

                #[cfg(feature = "async-tokio")]
                #[tokio::test]
                async fn async_tokio() {
                    let mut buf = Vec::new();
                    let mut reader = Reader::from_str($xml);
                    reader.config_mut().enable_all_checks(true);
                    assert_eq!(
                        reader.read_event_into_async(&mut buf).await.unwrap(),
                        $event
                    );
                }
            }

            mod ns_reader {
                use super::*;
                use pretty_assertions::assert_eq;

                #[test]
                fn borrowed() {
                    let mut reader = NsReader::from_str($xml);
                    reader.config_mut().enable_all_checks(true);
                    assert_eq!(reader.read_resolved_event().unwrap().1, $event);
                }

                #[test]
                fn buffered() {
                    let mut buf = Vec::new();
                    let mut reader = NsReader::from_str($xml);
                    reader.config_mut().enable_all_checks(true);
                    assert_eq!(reader.read_resolved_event_into(&mut buf).unwrap().1, $event);
                }

                #[cfg(feature = "async-tokio")]
                #[tokio::test]
                async fn async_tokio() {
                    let mut buf = Vec::new();
                    let mut reader = NsReader::from_str($xml);
                    reader.config_mut().enable_all_checks(true);
                    assert_eq!(
                        reader
                            .read_resolved_event_into_async(&mut buf)
                            .await
                            .unwrap()
                            .1,
                        $event
                    );
                }
            }
        }
    };
}

mod syntax {
    use super::*;

    macro_rules! err {
        ($test:ident($xml:literal) => $pos:expr, $cause:expr) => {
            mod $test {
                use super::*;

                mod reader {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[test]
                    fn borrowed() {
                        let mut reader = Reader::from_str($xml);
                        match reader.read_event() {
                            Err(Error::Syntax(cause)) => assert_eq!(
                                (cause, reader.error_position(), reader.buffer_position()),
                                ($cause, 0, $pos),
                            ),
                            x => panic!("Expected `Err(Syntax(_))`, but got {:?}", x),
                        }
                        assert_eq!(
                            reader
                                .read_event()
                                .expect("parser should return `Event::Eof` after error"),
                            Event::Eof
                        );
                    }

                    #[test]
                    fn buffered() {
                        let mut buf = Vec::new();
                        let mut reader = Reader::from_str($xml);
                        match reader.read_event_into(&mut buf) {
                            Err(Error::Syntax(cause)) => assert_eq!(
                                (cause, reader.error_position(), reader.buffer_position()),
                                ($cause, 0, $pos),
                            ),
                            x => panic!("Expected `Err(Syntax(_))`, but got {:?}", x),
                        }
                        assert_eq!(
                            reader
                                .read_event_into(&mut buf)
                                .expect("parser should return `Event::Eof` after error"),
                            Event::Eof
                        );
                    }

                    #[cfg(feature = "async-tokio")]
                    #[tokio::test]
                    async fn async_tokio() {
                        let mut buf = Vec::new();
                        let mut reader = Reader::from_str($xml);
                        match reader.read_event_into_async(&mut buf).await {
                            Err(Error::Syntax(cause)) => assert_eq!(
                                (cause, reader.error_position(), reader.buffer_position()),
                                ($cause, 0, $pos),
                            ),
                            x => panic!("Expected `Err(Syntax(_))`, but got {:?}", x),
                        }
                        assert_eq!(
                            reader
                                .read_event_into_async(&mut buf)
                                .await
                                .expect("parser should return `Event::Eof` after error"),
                            Event::Eof
                        );
                    }
                }

                mod ns_reader {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[test]
                    fn borrowed() {
                        let mut reader = NsReader::from_str($xml);
                        match reader.read_resolved_event() {
                            Err(Error::Syntax(cause)) => assert_eq!(
                                (cause, reader.error_position(), reader.buffer_position()),
                                ($cause, 0, $pos),
                            ),
                            x => panic!("Expected `Err(Syntax(_))`, but got {:?}", x),
                        }
                        assert_eq!(
                            reader
                                .read_resolved_event()
                                .expect("parser should return `Event::Eof` after error")
                                .1,
                            Event::Eof
                        );
                    }

                    #[test]
                    fn buffered() {
                        let mut buf = Vec::new();
                        let mut reader = NsReader::from_str($xml);
                        match reader.read_resolved_event_into(&mut buf) {
                            Err(Error::Syntax(cause)) => assert_eq!(
                                (cause, reader.error_position(), reader.buffer_position()),
                                ($cause, 0, $pos),
                            ),
                            x => panic!("Expected `Err(Syntax(_))`, but got {:?}", x),
                        }
                        assert_eq!(
                            reader
                                .read_resolved_event_into(&mut buf)
                                .expect("parser should return `Event::Eof` after error")
                                .1,
                            Event::Eof
                        );
                    }

                    #[cfg(feature = "async-tokio")]
                    #[tokio::test]
                    async fn async_tokio() {
                        let mut buf = Vec::new();
                        let mut reader = NsReader::from_str($xml);
                        match reader.read_resolved_event_into_async(&mut buf).await {
                            Err(Error::Syntax(cause)) => assert_eq!(
                                (cause, reader.error_position(), reader.buffer_position()),
                                ($cause, 0, $pos),
                            ),
                            x => panic!("Expected `Err(Syntax(_))`, but got {:?}", x),
                        }
                        assert_eq!(
                            reader
                                .read_resolved_event_into_async(&mut buf)
                                .await
                                .expect("parser should return `Event::Eof` after error")
                                .1,
                            Event::Eof
                        );
                    }
                }
            }
        };
        ($test:ident($xml:literal) => $cause:expr) => {
            err!($test($xml) => $xml.len() as u64, $cause);
        };
    }

    mod tag {
        use super::*;

        err!(unclosed1("<")   => SyntaxError::UnclosedTag);
        err!(unclosed2("</")  => SyntaxError::UnclosedTag);
        err!(unclosed3("<x")  => SyntaxError::UnclosedTag);
        err!(unclosed4("< ")  => SyntaxError::UnclosedTag);
        err!(unclosed5("<\t") => SyntaxError::UnclosedTag);
        err!(unclosed6("<\r") => SyntaxError::UnclosedTag);
        err!(unclosed7("<\n") => SyntaxError::UnclosedTag);
        err!(unclosed8("< \t\r\nx") => SyntaxError::UnclosedTag);

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
    }

    // Incorrect after-bang symbol is detected early, so buffer_position() stay at `!`
    err!(unclosed_bang1("<!")   => 1, SyntaxError::InvalidBangMarkup);
    err!(unclosed_bang2("<!>")  => 1, SyntaxError::InvalidBangMarkup);
    err!(unclosed_bang3("<!a")  => 1, SyntaxError::InvalidBangMarkup);
    err!(unclosed_bang4("<!a>") => 1, SyntaxError::InvalidBangMarkup);

    /// https://www.w3.org/TR/xml11/#NT-Comment
    mod comment {
        use super::*;

        err!(unclosed01("<!-")    => SyntaxError::UnclosedComment);
        err!(unclosed02("<!--")   => SyntaxError::UnclosedComment);
        err!(unclosed03("<!->")   => SyntaxError::UnclosedComment);
        err!(unclosed04("<!-a")   => SyntaxError::UnclosedComment);
        err!(unclosed05("<!---")  => SyntaxError::UnclosedComment);
        err!(unclosed06("<!-->")  => SyntaxError::UnclosedComment);
        err!(unclosed07("<!--b")  => SyntaxError::UnclosedComment);
        err!(unclosed08("<!----") => SyntaxError::UnclosedComment);
        err!(unclosed09("<!--->") => SyntaxError::UnclosedComment);
        err!(unclosed10("<!---c") => SyntaxError::UnclosedComment);

        ok!(normal("<!---->") => Event::Comment(BytesText::new("")));
    }

    /// https://www.w3.org/TR/xml11/#NT-CDSect
    mod cdata {
        use super::*;

        err!(unclosed01("<![")         => SyntaxError::UnclosedCData);
        err!(unclosed02("<![C")        => SyntaxError::UnclosedCData);
        err!(unclosed03("<![a")        => SyntaxError::UnclosedCData);
        err!(unclosed04("<![>")        => SyntaxError::UnclosedCData);
        err!(unclosed05("<![CD")       => SyntaxError::UnclosedCData);
        err!(unclosed06("<![Cb")       => SyntaxError::UnclosedCData);
        err!(unclosed07("<![C>")       => SyntaxError::UnclosedCData);
        err!(unclosed08("<![CDA")      => SyntaxError::UnclosedCData);
        err!(unclosed09("<![CDc")      => SyntaxError::UnclosedCData);
        err!(unclosed10("<![CD>")      => SyntaxError::UnclosedCData);
        err!(unclosed11("<![CDAT")     => SyntaxError::UnclosedCData);
        err!(unclosed12("<![CDAd")     => SyntaxError::UnclosedCData);
        err!(unclosed13("<![CDA>")     => SyntaxError::UnclosedCData);
        err!(unclosed14("<![CDATA")    => SyntaxError::UnclosedCData);
        err!(unclosed15("<![CDATe")    => SyntaxError::UnclosedCData);
        err!(unclosed16("<![CDAT>")    => SyntaxError::UnclosedCData);
        err!(unclosed17("<![CDATA[")   => SyntaxError::UnclosedCData);
        err!(unclosed18("<![CDATAf")   => SyntaxError::UnclosedCData);
        err!(unclosed19("<![CDATA>")   => SyntaxError::UnclosedCData);
        err!(unclosed20("<![CDATA[]")  => SyntaxError::UnclosedCData);
        err!(unclosed21("<![CDATA[g")  => SyntaxError::UnclosedCData);
        err!(unclosed22("<![CDATA[>")  => SyntaxError::UnclosedCData);
        err!(unclosed23("<![CDATA[]]") => SyntaxError::UnclosedCData);
        err!(unclosed24("<![CDATA[]h") => SyntaxError::UnclosedCData);
        err!(unclosed25("<![CDATA[]>") => SyntaxError::UnclosedCData);

        ok!(normal("<![CDATA[]]>") => Event::CData(BytesCData::new("")));
    }

    /// According to the grammar, only upper-case letters allowed for DOCTYPE writing.
    ///
    /// https://www.w3.org/TR/xml11/#NT-doctypedecl
    mod doctype {
        use super::*;

        err!(unclosed01("<!D")         => SyntaxError::UnclosedDoctype);
        err!(unclosed02("<!DO")        => SyntaxError::UnclosedDoctype);
        err!(unclosed03("<!Da")        => SyntaxError::UnclosedDoctype);
        err!(unclosed04("<!D>")        => SyntaxError::UnclosedDoctype);
        err!(unclosed05("<!DOC")       => SyntaxError::UnclosedDoctype);
        err!(unclosed06("<!DOb")       => SyntaxError::UnclosedDoctype);
        err!(unclosed07("<!DO>")       => SyntaxError::UnclosedDoctype);
        err!(unclosed08("<!DOCT")      => SyntaxError::UnclosedDoctype);
        err!(unclosed09("<!DOCc")      => SyntaxError::UnclosedDoctype);
        err!(unclosed10("<!DOC>")      => SyntaxError::UnclosedDoctype);
        err!(unclosed11("<!DOCTY")     => SyntaxError::UnclosedDoctype);
        err!(unclosed12("<!DOCTd")     => SyntaxError::UnclosedDoctype);
        err!(unclosed13("<!DOCT>")     => SyntaxError::UnclosedDoctype);
        err!(unclosed14("<!DOCTYP")    => SyntaxError::UnclosedDoctype);
        err!(unclosed15("<!DOCTYe")    => SyntaxError::UnclosedDoctype);
        err!(unclosed16("<!DOCTY>")    => SyntaxError::UnclosedDoctype);
        err!(unclosed17("<!DOCTYPE")   => SyntaxError::UnclosedDoctype);
        err!(unclosed18("<!DOCTYPf")   => SyntaxError::UnclosedDoctype);
        err!(unclosed19("<!DOCTYP>")   => SyntaxError::UnclosedDoctype);
        err!(unclosed20("<!DOCTYPE ")  => SyntaxError::UnclosedDoctype);
        err!(unclosed21("<!DOCTYPEg")  => SyntaxError::UnclosedDoctype);
        // <!DOCTYPE> results in IllFormed(MissingDoctypeName), checked below
        err!(unclosed22("<!DOCTYPE e") => SyntaxError::UnclosedDoctype);

        // According to the grammar, XML declaration MUST contain at least one space
        // and an element name, but we do not consider this as a _syntax_ error.
        ok!(normal("<!DOCTYPE e>") => Event::DocType(BytesText::new("e")));
    }

    /// https://www.w3.org/TR/xml11/#NT-PI
    mod pi {
        use super::*;

        err!(unclosed1("<?")    => SyntaxError::UnclosedPIOrXmlDecl);
        err!(unclosed2("<??")   => SyntaxError::UnclosedPIOrXmlDecl);
        err!(unclosed3("<?>")   => SyntaxError::UnclosedPIOrXmlDecl);
        err!(unclosed4("<?<")   => SyntaxError::UnclosedPIOrXmlDecl);
        err!(unclosed5("<?&")   => SyntaxError::UnclosedPIOrXmlDecl);
        err!(unclosed6("<?p")   => SyntaxError::UnclosedPIOrXmlDecl);
        err!(unclosed7("<? ")   => SyntaxError::UnclosedPIOrXmlDecl);
        err!(unclosed8("<?\t")  => SyntaxError::UnclosedPIOrXmlDecl);
        err!(unclosed9("<?\r")  => SyntaxError::UnclosedPIOrXmlDecl);
        err!(unclosed10("<?\n") => SyntaxError::UnclosedPIOrXmlDecl);

        // According to the grammar, processing instruction MUST contain a non-empty
        // target name, but we do not consider this as a _syntax_ error.
        ok!(normal_empty("<??>")    => Event::PI(BytesPI::new("")));
        ok!(normal_xmlx("<?xmlx?>") => Event::PI(BytesPI::new("xmlx")));
    }

    /// https://www.w3.org/TR/xml11/#NT-prolog
    mod decl {
        use super::*;

        err!(unclosed1("<?x")    => SyntaxError::UnclosedPIOrXmlDecl);
        err!(unclosed2("<?xm")   => SyntaxError::UnclosedPIOrXmlDecl);
        err!(unclosed3("<?xml")  => SyntaxError::UnclosedPIOrXmlDecl);
        err!(unclosed4("<?xml?") => SyntaxError::UnclosedPIOrXmlDecl);

        // According to the grammar, XML declaration MUST contain at least one space
        // and `version` attribute, but we do not consider this as a _syntax_ error.
        ok!(normal1("<?xml?>")   => Event::Decl(BytesDecl::from_start(BytesStart::new("xml"))));
        ok!(normal2("<?xml ?>")  => Event::Decl(BytesDecl::from_start(BytesStart::from_content("xml ", 3))));
        ok!(normal3("<?xml\t?>") => Event::Decl(BytesDecl::from_start(BytesStart::from_content("xml\t", 3))));
        ok!(normal4("<?xml\r?>") => Event::Decl(BytesDecl::from_start(BytesStart::from_content("xml\r", 3))));
        ok!(normal5("<?xml\n?>") => Event::Decl(BytesDecl::from_start(BytesStart::from_content("xml\n", 3))));
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
    ok!(missing_doctype_name3("<!DOCTYPE \t\r\nx>") => Event::DocType(BytesText::new("x")));

    err!(unmatched_end_tag1("</>") => 0: IllFormedError::UnmatchedEndTag("".to_string()));
    err!(unmatched_end_tag2("</end>") => 0: IllFormedError::UnmatchedEndTag("end".to_string()));
    err!(unmatched_end_tag3("</end >") => 0: IllFormedError::UnmatchedEndTag("end".to_string()));

    ok!(mismatched_end_tag1("<start></start>") => Event::Start(BytesStart::new("start")));
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

    ok!(double_hyphen_in_comment1("<!---->") => Event::Comment(BytesText::new("")));
    err!(double_hyphen_in_comment2("<!----->") => 4: IllFormedError::DoubleHyphenInComment);
    //                                  ^= 4
    err!(double_hyphen_in_comment3("<!-- --->") => 5: IllFormedError::DoubleHyphenInComment);
    //                                   ^= 5
    err!(double_hyphen_in_comment4("<!-- -- -->") => 5: IllFormedError::DoubleHyphenInComment);
    //                                   ^= 5
}

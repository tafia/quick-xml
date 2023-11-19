//! Contains tests that produces errors during parsing XML.

use quick_xml::errors::{Error, SyntaxError};
use quick_xml::events::{BytesCData, BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::{NsReader, Reader};

macro_rules! ok {
    ($test:ident($xml:literal) => $event:expr) => {
        mod $test {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn borrowed() {
                let mut reader = Reader::from_str($xml);
                reader.config_mut().enable_all_checks(true);
                assert_eq!(reader.read_event().unwrap(), $event);

                let mut reader = NsReader::from_str($xml);
                reader.config_mut().enable_all_checks(true);
                assert_eq!(reader.read_resolved_event().unwrap().1, $event);
            }

            #[test]
            fn buffered() {
                let mut buf = Vec::new();
                let mut reader = Reader::from_str($xml);
                reader.config_mut().enable_all_checks(true);
                assert_eq!(reader.read_event_into(&mut buf).unwrap(), $event);

                let mut reader = NsReader::from_str($xml);
                reader.config_mut().enable_all_checks(true);
                assert_eq!(reader.read_resolved_event_into(&mut buf).unwrap().1, $event);
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
    };
}

mod syntax {
    use super::*;

    macro_rules! err {
        ($test:ident($xml:literal) => $cause:expr) => {
            mod $test {
                use super::*;
                use pretty_assertions::assert_eq;

                #[test]
                fn borrowed() {
                    let mut reader = Reader::from_str($xml);
                    match reader.read_event() {
                        Err(Error::Syntax(cause)) => {
                            assert_eq!(cause, $cause);
                            assert_eq!(reader.buffer_position(), 0);
                        }
                        x => panic!("Expected `Err(Syntax(_))`, but got {:?}", x),
                    }
                    assert_eq!(
                        reader
                            .read_event()
                            .expect("parser should return `Event::Eof` after error"),
                        Event::Eof
                    );

                    let mut reader = NsReader::from_str($xml);
                    match reader.read_resolved_event() {
                        Err(Error::Syntax(cause)) => {
                            assert_eq!(cause, $cause);
                            assert_eq!(reader.buffer_position(), 0);
                        }
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
                    let mut reader = Reader::from_str($xml);
                    match reader.read_event_into(&mut buf) {
                        Err(Error::Syntax(cause)) => {
                            assert_eq!(cause, $cause);
                            assert_eq!(reader.buffer_position(), 0);
                        }
                        x => panic!("Expected `Err(Syntax(_))`, but got {:?}", x),
                    }
                    assert_eq!(
                        reader
                            .read_event_into(&mut buf)
                            .expect("parser should return `Event::Eof` after error"),
                        Event::Eof
                    );

                    let mut reader = NsReader::from_str($xml);
                    match reader.read_resolved_event_into(&mut buf) {
                        Err(Error::Syntax(cause)) => {
                            assert_eq!(cause, $cause);
                            assert_eq!(reader.buffer_position(), 0);
                        }
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
                    let mut reader = Reader::from_str($xml);
                    match reader.read_event_into_async(&mut buf).await {
                        Err(Error::Syntax(cause)) => {
                            assert_eq!(cause, $cause);
                            assert_eq!(reader.buffer_position(), 0);
                        }
                        x => panic!("Expected `Err(Syntax(_))`, but got {:?}", x),
                    }
                    assert_eq!(
                        reader
                            .read_event_into_async(&mut buf)
                            .await
                            .expect("parser should return `Event::Eof` after error"),
                        Event::Eof
                    );

                    let mut reader = NsReader::from_str($xml);
                    match reader.read_resolved_event_into_async(&mut buf).await {
                        Err(Error::Syntax(cause)) => {
                            assert_eq!(cause, $cause);
                            assert_eq!(reader.buffer_position(), 0);
                        }
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

        /// Closed tags can be tested only in pair with open tags, because otherwise
        /// `IllFormedError::UnmatchedEnd` will be raised
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

    err!(unclosed_bang1("<!")  => SyntaxError::InvalidBangMarkup);
    err!(unclosed_bang2("<!>") => SyntaxError::InvalidBangMarkup);

    /// https://www.w3.org/TR/xml11/#NT-Comment
    mod comment {
        use super::*;

        err!(unclosed1("<!-")    => SyntaxError::UnclosedComment);
        err!(unclosed2("<!--")   => SyntaxError::UnclosedComment);
        err!(unclosed3("<!->")   => SyntaxError::UnclosedComment);
        err!(unclosed4("<!---")  => SyntaxError::UnclosedComment);
        err!(unclosed5("<!-->")  => SyntaxError::UnclosedComment);
        err!(unclosed6("<!----") => SyntaxError::UnclosedComment);
        err!(unclosed7("<!--->") => SyntaxError::UnclosedComment);

        ok!(normal("<!---->") => Event::Comment(BytesText::new("")));
    }

    /// https://www.w3.org/TR/xml11/#NT-CDSect
    mod cdata {
        use super::*;

        err!(unclosed1("<![")         => SyntaxError::UnclosedCData);
        err!(unclosed2("<![C")        => SyntaxError::UnclosedCData);
        err!(unclosed3("<![CD")       => SyntaxError::UnclosedCData);
        err!(unclosed4("<![CDA")      => SyntaxError::UnclosedCData);
        err!(unclosed5("<![CDAT")     => SyntaxError::UnclosedCData);
        err!(unclosed6("<![CDATA")    => SyntaxError::UnclosedCData);
        err!(unclosed7("<![CDATA[")   => SyntaxError::UnclosedCData);
        err!(unclosed8("<![CDATA[]")  => SyntaxError::UnclosedCData);
        err!(unclosed9("<![CDATA[]]") => SyntaxError::UnclosedCData);

        ok!(normal("<![CDATA[]]>") => Event::CData(BytesCData::new("")));
    }

    /// According to the grammar, only upper-case letters allowed for DOCTYPE writing.
    ///
    /// https://www.w3.org/TR/xml11/#NT-doctypedecl
    mod doctype {
        use super::*;

        err!(unclosed1("<!D")         => SyntaxError::UnclosedDoctype);
        err!(unclosed2("<!DO")        => SyntaxError::UnclosedDoctype);
        err!(unclosed3("<!DOC")       => SyntaxError::UnclosedDoctype);
        err!(unclosed4("<!DOCT")      => SyntaxError::UnclosedDoctype);
        err!(unclosed5("<!DOCTY")     => SyntaxError::UnclosedDoctype);
        err!(unclosed6("<!DOCTYP")    => SyntaxError::UnclosedDoctype);
        err!(unclosed7("<!DOCTYPE")   => SyntaxError::UnclosedDoctype);
        err!(unclosed8("<!DOCTYPE ")  => SyntaxError::UnclosedDoctype);
        err!(unclosed9("<!DOCTYPE e") => SyntaxError::UnclosedDoctype);

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
        ok!(normal_empty("<??>")    => Event::PI(BytesText::new("")));
        ok!(normal_xmlx("<?xmlx?>") => Event::PI(BytesText::new("xmlx")));
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
                use pretty_assertions::assert_eq;

                #[test]
                fn borrowed() {
                    let mut reader = Reader::from_str(concat!($xml, "<x/>"));
                    reader.config_mut().enable_all_checks(true);
                    match reader.read_event() {
                        Err(Error::IllFormed(cause)) => {
                            assert_eq!(cause, $cause);
                            assert_eq!(reader.buffer_position(), $pos);
                        }
                        x => panic!("Expected `Err(IllFormed(_))`, but got {:?}", x),
                    }
                    assert_eq!(
                        reader.read_event().expect(
                            "parsing should be possible to continue after `Error::IllFormed`"
                        ),
                        Event::Empty(BytesStart::new("x"))
                    );

                    let mut reader = NsReader::from_str(concat!($xml, "<x/>"));
                    reader.config_mut().enable_all_checks(true);
                    match reader.read_resolved_event() {
                        Err(Error::IllFormed(cause)) => {
                            assert_eq!(cause, $cause);
                            assert_eq!(reader.buffer_position(), $pos);
                        }
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
                }

                #[test]
                fn buffered() {
                    let mut buf = Vec::new();
                    let mut reader = Reader::from_str(concat!($xml, "<x/>"));
                    reader.config_mut().enable_all_checks(true);
                    match reader.read_event_into(&mut buf) {
                        Err(Error::IllFormed(cause)) => {
                            assert_eq!(cause, $cause);
                            assert_eq!(reader.buffer_position(), $pos);
                        }
                        x => panic!("Expected `Err(IllFormed(_))`, but got {:?}", x),
                    }
                    assert_eq!(
                        reader.read_event_into(&mut buf).expect(
                            "parsing should be possible to continue after `Error::IllFormed`"
                        ),
                        Event::Empty(BytesStart::new("x"))
                    );

                    let mut reader = NsReader::from_str(concat!($xml, "<x/>"));
                    reader.config_mut().enable_all_checks(true);
                    match reader.read_resolved_event_into(&mut buf) {
                        Err(Error::IllFormed(cause)) => {
                            assert_eq!(cause, $cause);
                            assert_eq!(reader.buffer_position(), $pos);
                        }
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
                }

                #[cfg(feature = "async-tokio")]
                #[tokio::test]
                async fn async_tokio() {
                    let mut buf = Vec::new();
                    let mut reader = Reader::from_str(concat!($xml, "<x/>"));
                    reader.config_mut().enable_all_checks(true);
                    match reader.read_event_into_async(&mut buf).await {
                        Err(Error::IllFormed(cause)) => {
                            assert_eq!(cause, $cause);
                            assert_eq!(reader.buffer_position(), $pos);
                        }
                        x => panic!("Expected `Err(IllFormed(_))`, but got {:?}", x),
                    }
                    assert_eq!(
                        reader.read_event_into_async(&mut buf).await.expect(
                            "parsing should be possible to continue after `Error::IllFormed`"
                        ),
                        Event::Empty(BytesStart::new("x"))
                    );

                    let mut reader = NsReader::from_str(concat!($xml, "<x/>"));
                    reader.config_mut().enable_all_checks(true);
                    match reader.read_resolved_event_into_async(&mut buf).await {
                        Err(Error::IllFormed(cause)) => {
                            assert_eq!(cause, $cause);
                            assert_eq!(reader.buffer_position(), $pos);
                        }
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
                }
            }
        };
    }

    /// Performs 3 reads, the first and third ones should be successful
    macro_rules! err2 {
        ($test:ident($xml:literal) => $pos:literal : $cause:expr) => {
            mod $test {
                use super::*;
                use pretty_assertions::assert_eq;

                #[test]
                fn borrowed() {
                    let mut reader = Reader::from_str(concat!($xml, "<x/>"));
                    reader.config_mut().enable_all_checks(true);
                    reader.read_event().expect("first .read_event()");
                    match reader.read_event() {
                        Err(Error::IllFormed(cause)) => {
                            assert_eq!(cause, $cause);
                            assert_eq!(reader.buffer_position(), $pos);
                        }
                        x => panic!("Expected `Err(IllFormed(_))`, but got {:?}", x),
                    }
                    assert_eq!(
                        reader.read_event().expect(
                            "parsing should be possible to continue after `Error::IllFormed`"
                        ),
                        Event::Empty(BytesStart::new("x"))
                    );

                    let mut reader = NsReader::from_str(concat!($xml, "<x/>"));
                    reader.config_mut().enable_all_checks(true);
                    reader.read_event().expect("first .read_resolved_event()");
                    match reader.read_resolved_event() {
                        Err(Error::IllFormed(cause)) => {
                            assert_eq!(cause, $cause);
                            assert_eq!(reader.buffer_position(), $pos);
                        }
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
                }

                #[test]
                fn buffered() {
                    let mut buf = Vec::new();
                    let mut reader = Reader::from_str(concat!($xml, "<x/>"));
                    reader.config_mut().enable_all_checks(true);
                    reader
                        .read_event_into(&mut buf)
                        .expect("first .read_event_into()");
                    match reader.read_event_into(&mut buf) {
                        Err(Error::IllFormed(cause)) => {
                            assert_eq!(cause, $cause);
                            assert_eq!(reader.buffer_position(), $pos);
                        }
                        x => panic!("Expected `Err(IllFormed(_))`, but got {:?}", x),
                    }
                    assert_eq!(
                        reader.read_event_into(&mut buf).expect(
                            "parsing should be possible to continue after `Error::IllFormed`"
                        ),
                        Event::Empty(BytesStart::new("x"))
                    );

                    let mut reader = NsReader::from_str(concat!($xml, "<x/>"));
                    reader.config_mut().enable_all_checks(true);
                    reader
                        .read_resolved_event_into(&mut buf)
                        .expect("first .read_resolved_event_into()");
                    match reader.read_resolved_event_into(&mut buf) {
                        Err(Error::IllFormed(cause)) => {
                            assert_eq!(cause, $cause);
                            assert_eq!(reader.buffer_position(), $pos);
                        }
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
                }

                #[cfg(feature = "async-tokio")]
                #[tokio::test]
                async fn async_tokio() {
                    let mut buf = Vec::new();
                    let mut reader = Reader::from_str(concat!($xml, "<x/>"));
                    reader.config_mut().enable_all_checks(true);
                    reader
                        .read_event_into_async(&mut buf)
                        .await
                        .expect("first .read_event_into_async()");
                    match reader.read_event_into_async(&mut buf).await {
                        Err(Error::IllFormed(cause)) => {
                            assert_eq!(cause, $cause);
                            assert_eq!(reader.buffer_position(), $pos);
                        }
                        x => panic!("Expected `Err(IllFormed(_))`, but got {:?}", x),
                    }
                    assert_eq!(
                        reader.read_event_into_async(&mut buf).await.expect(
                            "parsing should be possible to continue after `Error::IllFormed`"
                        ),
                        Event::Empty(BytesStart::new("x"))
                    );

                    let mut reader = NsReader::from_str(concat!($xml, "<x/>"));
                    reader.config_mut().enable_all_checks(true);
                    reader
                        .read_resolved_event_into_async(&mut buf)
                        .await
                        .expect("first .read_resolved_event_into_async()");
                    match reader.read_resolved_event_into_async(&mut buf).await {
                        Err(Error::IllFormed(cause)) => {
                            assert_eq!(cause, $cause);
                            assert_eq!(reader.buffer_position(), $pos);
                        }
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
                }
            }
        };
    }

    // IllFormedError::MissedVersion is generated lazily when you call `BytesDecl::version()`

    err!(missed_doctype_name1("<!DOCTYPE>") => 9: IllFormedError::MissedDoctypeName);
    //                                  ^= 9
    err!(missed_doctype_name2("<!DOCTYPE \t\r\n>") => 13: IllFormedError::MissedDoctypeName);
    //                                         ^= 13
    ok!(missed_doctype_name3("<!DOCTYPE \t\r\nx>") => Event::DocType(BytesText::new("x")));

    err!(unmatched_end1("</>") => 0: IllFormedError::UnmatchedEnd("".to_string()));
    err!(unmatched_end2("</end>") => 0: IllFormedError::UnmatchedEnd("end".to_string()));
    err!(unmatched_end3("</end >") => 0: IllFormedError::UnmatchedEnd("end".to_string()));

    ok!(mismatched_end1("<start></start>") => Event::Start(BytesStart::new("start")));
    err2!(mismatched_end2("<start></>") => 7: IllFormedError::MismatchedEnd {
        //                        ^= 7
        expected: "start".to_string(),
        found: "".to_string(),
    });
    err2!(mismatched_end3("<start></end>") => 7: IllFormedError::MismatchedEnd {
        //                        ^= 7
        expected: "start".to_string(),
        found: "end".to_string(),
    });
    err2!(mismatched_end4("<start></end >") => 7: IllFormedError::MismatchedEnd {
        //                        ^= 7
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

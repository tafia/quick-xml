//! Utility functions for integration tests

/// Tests for https://github.com/tafia/quick-xml/issues/469
/// Exported to reuse in `async-tokio` tests.
#[macro_export]
macro_rules! small_buffers_tests {
    (
        #[$test:meta]
        $read_event:ident: $BufReader:ty
        $(, $async:ident, $await:ident)?
    ) => {
        mod small_buffers {
            use quick_xml::events::{BytesCData, BytesDecl, BytesPI, BytesStart, BytesText, Event};
            use quick_xml::reader::Reader;
            use pretty_assertions::assert_eq;

            #[$test]
            $($async)? fn decl() {
                let xml = "<?xml ?>";
                //         ^^^^^^^ data that fit into buffer
                let size = xml.match_indices("?>").next().unwrap().0 + 1;
                let br = <$BufReader>::with_capacity(size, xml.as_bytes());
                let mut reader = Reader::from_reader(br);
                let mut buf = Vec::new();

                assert_eq!(
                    reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                    Event::Decl(BytesDecl::from_start(BytesStart::from_content("xml ", 3)))
                );
                assert_eq!(
                    reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                    Event::Eof
                );
            }

            #[$test]
            $($async)? fn pi() {
                let xml = "<?pi?>";
                //         ^^^^^ data that fit into buffer
                let size = xml.match_indices("?>").next().unwrap().0 + 1;
                let br = <$BufReader>::with_capacity(size, xml.as_bytes());
                let mut reader = Reader::from_reader(br);
                let mut buf = Vec::new();

                assert_eq!(
                    reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                    Event::PI(BytesPI::new("pi"))
                );
                assert_eq!(
                    reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                    Event::Eof
                );
            }

            #[$test]
            $($async)? fn empty() {
                let xml = "<empty/>";
                //         ^^^^^^^ data that fit into buffer
                let size = xml.match_indices("/>").next().unwrap().0 + 1;
                let br = <$BufReader>::with_capacity(size, xml.as_bytes());
                let mut reader = Reader::from_reader(br);
                let mut buf = Vec::new();

                assert_eq!(
                    reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                    Event::Empty(BytesStart::new("empty"))
                );
                assert_eq!(
                    reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                    Event::Eof
                );
            }

            #[$test]
            $($async)? fn cdata1() {
                let xml = "<![CDATA[cdata]]>";
                //         ^^^^^^^^^^^^^^^ data that fit into buffer
                let size = xml.match_indices("]]>").next().unwrap().0 + 1;
                let br = <$BufReader>::with_capacity(size, xml.as_bytes());
                let mut reader = Reader::from_reader(br);
                let mut buf = Vec::new();

                assert_eq!(
                    reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                    Event::CData(BytesCData::new("cdata"))
                );
                assert_eq!(
                    reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                    Event::Eof
                );
            }

            #[$test]
            $($async)? fn cdata2() {
                let xml = "<![CDATA[cdata]]>";
                //         ^^^^^^^^^^^^^^^^ data that fit into buffer
                let size = xml.match_indices("]]>").next().unwrap().0 + 2;
                let br = <$BufReader>::with_capacity(size, xml.as_bytes());
                let mut reader = Reader::from_reader(br);
                let mut buf = Vec::new();

                assert_eq!(
                    reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                    Event::CData(BytesCData::new("cdata"))
                );
                assert_eq!(
                    reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                    Event::Eof
                );
            }

            #[$test]
            $($async)? fn comment1() {
                let xml = "<!--comment-->";
                //         ^^^^^^^^^^^^ data that fit into buffer
                let size = xml.match_indices("-->").next().unwrap().0 + 1;
                let br = <$BufReader>::with_capacity(size, xml.as_bytes());
                let mut reader = Reader::from_reader(br);
                let mut buf = Vec::new();

                assert_eq!(
                    reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                    Event::Comment(BytesText::new("comment"))
                );
                assert_eq!(
                    reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                    Event::Eof
                );
            }

            #[$test]
            $($async)? fn comment2() {
                let xml = "<!--comment-->";
                //         ^^^^^^^^^^^^^ data that fit into buffer
                let size = xml.match_indices("-->").next().unwrap().0 + 2;
                let br = <$BufReader>::with_capacity(size, xml.as_bytes());
                let mut reader = Reader::from_reader(br);
                let mut buf = Vec::new();

                assert_eq!(
                    reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                    Event::Comment(BytesText::new("comment"))
                );
                assert_eq!(
                    reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                    Event::Eof
                );
            }
        }
    };
}

/// Tests for reader-errors and reader-dtd.
#[macro_export]
macro_rules! event_ok {
    ($test:ident($xml:literal) => $pos:literal : $event:expr) => {
        mod $test {
            use super::*;

            mod reader {
                use super::*;
                use pretty_assertions::assert_eq;

                #[test]
                fn borrowed() {
                    let mut reader = Reader::from_str($xml);
                    reader.config_mut().enable_all_checks(true);
                    assert_eq!(
                        (reader.read_event().unwrap(), reader.buffer_position()),
                        ($event, $pos)
                    );
                }

                #[test]
                fn buffered() {
                    let mut buf = Vec::new();
                    let mut reader = Reader::from_str($xml);
                    reader.config_mut().enable_all_checks(true);
                    assert_eq!(
                        (
                            reader.read_event_into(&mut buf).unwrap(),
                            reader.buffer_position()
                        ),
                        ($event, $pos)
                    );
                }

                #[cfg(feature = "async-tokio")]
                #[tokio::test]
                async fn async_tokio() {
                    let mut buf = Vec::new();
                    let mut reader = Reader::from_str($xml);
                    reader.config_mut().enable_all_checks(true);
                    assert_eq!(
                        (
                            reader.read_event_into_async(&mut buf).await.unwrap(),
                            reader.buffer_position()
                        ),
                        ($event, $pos)
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
                    assert_eq!(
                        (
                            reader.read_resolved_event().unwrap().1,
                            reader.buffer_position()
                        ),
                        ($event, $pos)
                    );
                }

                #[test]
                fn buffered() {
                    let mut buf = Vec::new();
                    let mut reader = NsReader::from_str($xml);
                    reader.config_mut().enable_all_checks(true);
                    assert_eq!(
                        (
                            reader.read_resolved_event_into(&mut buf).unwrap().1,
                            reader.buffer_position()
                        ),
                        ($event, $pos)
                    );
                }

                #[cfg(feature = "async-tokio")]
                #[tokio::test]
                async fn async_tokio() {
                    let mut buf = Vec::new();
                    let mut reader = NsReader::from_str($xml);
                    reader.config_mut().enable_all_checks(true);
                    assert_eq!(
                        (
                            reader
                                .read_resolved_event_into_async(&mut buf)
                                .await
                                .unwrap()
                                .1,
                            reader.buffer_position()
                        ),
                        ($event, $pos)
                    );
                }
            }
        }
    };
}

/// Tests for reader-errors and reader-dtd.
#[macro_export]
macro_rules! syntax_err {
    ($test:ident($xml:literal) => $pos:expr, $cause:expr) => {
        mod $test {
            use super::*;

            mod reader {
                use super::*;
                use pretty_assertions::assert_eq;

                #[test]
                fn borrowed() {
                    let mut reader = Reader::from_str($xml);
                    assert_eq!(
                        reader
                            .read_event()
                            .expect("parser should return `Event::Text`"),
                        Event::Text(BytesText::new("."))
                    );
                    match reader.read_event() {
                        Err(Error::Syntax(cause)) => assert_eq!(
                            (cause, reader.error_position(), reader.buffer_position()),
                            ($cause, 1, $pos),
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
                    assert_eq!(
                        reader
                            .read_event_into(&mut buf)
                            .expect("parser should return `Event::Text`"),
                        Event::Text(BytesText::new("."))
                    );
                    match reader.read_event_into(&mut buf) {
                        Err(Error::Syntax(cause)) => assert_eq!(
                            (cause, reader.error_position(), reader.buffer_position()),
                            ($cause, 1, $pos),
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
                    assert_eq!(
                        reader
                            .read_event_into_async(&mut buf)
                            .await
                            .expect("parser should return `Event::Text`"),
                        Event::Text(BytesText::new("."))
                    );
                    match reader.read_event_into_async(&mut buf).await {
                        Err(Error::Syntax(cause)) => assert_eq!(
                            (cause, reader.error_position(), reader.buffer_position()),
                            ($cause, 1, $pos),
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
                    assert_eq!(
                        reader
                            .read_resolved_event()
                            .expect("parser should return `Event::Text`")
                            .1,
                        Event::Text(BytesText::new("."))
                    );
                    match reader.read_resolved_event() {
                        Err(Error::Syntax(cause)) => assert_eq!(
                            (cause, reader.error_position(), reader.buffer_position()),
                            ($cause, 1, $pos),
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
                    assert_eq!(
                        reader
                            .read_resolved_event_into(&mut buf)
                            .expect("parser should return `Event::Text`")
                            .1,
                        Event::Text(BytesText::new("."))
                    );
                    match reader.read_resolved_event_into(&mut buf) {
                        Err(Error::Syntax(cause)) => assert_eq!(
                            (cause, reader.error_position(), reader.buffer_position()),
                            ($cause, 1, $pos),
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
                    assert_eq!(
                        reader
                            .read_resolved_event_into_async(&mut buf)
                            .await
                            .expect("parser should return `Event::Text`")
                            .1,
                        Event::Text(BytesText::new("."))
                    );
                    match reader.read_resolved_event_into_async(&mut buf).await {
                        Err(Error::Syntax(cause)) => assert_eq!(
                            (cause, reader.error_position(), reader.buffer_position()),
                            ($cause, 1, $pos),
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
        syntax_err!($test($xml) => $xml.len() as u64, $cause);
    };
}

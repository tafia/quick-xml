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

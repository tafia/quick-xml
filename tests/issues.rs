//! Regression tests found in various issues
//!
//! Name each test as `issue<GH number>`

use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use quick_xml::Error;

/// Regression test for https://github.com/tafia/quick-xml/issues/514
mod issue514 {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Check that there is no unexpected error
    #[test]
    fn no_mismatch() {
        let mut reader = Reader::from_str("<some-tag><html>...</html></some-tag>");

        let outer_start = BytesStart::new("some-tag");
        let outer_end = outer_start.to_end().into_owned();

        let html_start = BytesStart::new("html");
        let html_end = html_start.to_end().into_owned();

        assert_eq!(reader.read_event().unwrap(), Event::Start(outer_start));
        assert_eq!(reader.read_event().unwrap(), Event::Start(html_start));

        reader.check_end_names(false);

        assert_eq!(reader.read_text(html_end.name()).unwrap(), "...");

        reader.check_end_names(true);

        assert_eq!(reader.read_event().unwrap(), Event::End(outer_end));
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }

    /// Canary check that legitimate error is reported
    #[test]
    fn mismatch() {
        let mut reader = Reader::from_str("<some-tag><html>...</html></other-tag>");

        let outer_start = BytesStart::new("some-tag");

        let html_start = BytesStart::new("html");
        let html_end = html_start.to_end().into_owned();

        assert_eq!(reader.read_event().unwrap(), Event::Start(outer_start));
        assert_eq!(reader.read_event().unwrap(), Event::Start(html_start));

        reader.check_end_names(false);

        assert_eq!(reader.read_text(html_end.name()).unwrap(), "...");

        reader.check_end_names(true);

        match reader.read_event() {
            Err(Error::EndEventMismatch { expected, found }) => {
                assert_eq!(expected, "some-tag");
                assert_eq!(found, "other-tag");
            }
            x => panic!(
                r#"Expected `Err(EndEventMismatch("some-tag", "other-tag")))`, but found {:?}"#,
                x
            ),
        }
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }
}

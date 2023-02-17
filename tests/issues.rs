//! Regression tests found in various issues.
//!
//! Name each module / test as `issue<GH number>` and keep sorted by issue number

use std::sync::mpsc;

use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use quick_xml::reader::Reader;
use quick_xml::Error;

/// Regression test for https://github.com/tafia/quick-xml/issues/115
#[test]
fn issue115() {
    let mut r = Reader::from_str("<tag1 attr1='line 1\nline 2'></tag1>");
    match r.read_event() {
        Ok(Event::Start(e)) if e.name() == QName(b"tag1") => {
            let v = e.attributes().map(|a| a.unwrap().value).collect::<Vec<_>>();
            assert_eq!(v[0].clone().into_owned(), b"line 1\nline 2");
        }
        _ => (),
    }
}

/// Regression test for https://github.com/tafia/quick-xml/issues/360
#[test]
fn issue360() {
    let (tx, rx) = mpsc::channel::<Event>();

    std::thread::spawn(move || {
        let mut r = Reader::from_str("<tag1 attr1='line 1\nline 2'></tag1>");
        loop {
            let event = r.read_event().unwrap();
            if event == Event::Eof {
                tx.send(event).unwrap();
                break;
            } else {
                tx.send(event).unwrap();
            }
        }
    });
    for event in rx.iter() {
        println!("{:?}", event);
    }
}

/// Regression test for https://github.com/tafia/quick-xml/issues/514
mod issue514 {
    use super::*;
    use pretty_assertions::assert_eq;

    #[cfg(not(feature = "span"))]
    fn read_event<'a>(reader: &'a mut Reader<&[u8]>) -> Event<'a> {
        reader.read_event().unwrap()
    }

    // We do not test correctness of spans here so just clear them
    #[cfg(feature = "span")]
    fn read_event<'a>(reader: &'a mut Reader<&[u8]>) -> Event<'a> {
        use quick_xml::events::Spanned;
        use quick_xml::reader::Span;

        reader.read_event().unwrap().with_span(Span::default())
    }

    /// Check that there is no unexpected error
    #[test]
    fn no_mismatch() {
        let mut reader = Reader::from_str("<some-tag><html>...</html></some-tag>");

        let outer_start = BytesStart::new("some-tag");
        let outer_end = outer_start.to_end().into_owned();

        let html_start = BytesStart::new("html");
        let html_end = html_start.to_end().into_owned();

        assert_eq!(read_event(&mut reader), Event::Start(outer_start));
        assert_eq!(read_event(&mut reader), Event::Start(html_start));

        reader.check_end_names(false);

        assert_eq!(reader.read_text(html_end.name()).unwrap(), "...");

        reader.check_end_names(true);

        assert_eq!(read_event(&mut reader), Event::End(outer_end));
        assert_eq!(read_event(&mut reader), Event::Eof);
    }

    /// Canary check that legitimate error is reported
    #[test]
    fn mismatch() {
        let mut reader = Reader::from_str("<some-tag><html>...</html></other-tag>");

        let outer_start = BytesStart::new("some-tag");

        let html_start = BytesStart::new("html");
        let html_end = html_start.to_end().into_owned();

        assert_eq!(read_event(&mut reader), Event::Start(outer_start));
        assert_eq!(read_event(&mut reader), Event::Start(html_start));

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

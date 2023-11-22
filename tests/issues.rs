//! Regression tests found in various issues.
//!
//! Name each module / test as `issue<GH number>` and keep sorted by issue number

use std::sync::mpsc;

use quick_xml::errors::{Error, IllFormedError, SyntaxError};
use quick_xml::events::{BytesDecl, BytesStart, BytesText, Event};
use quick_xml::name::QName;
use quick_xml::reader::Reader;

/// Regression test for https://github.com/tafia/quick-xml/issues/94
#[test]
fn issue94() {
    let data = br#"<Run>
<!B>
</Run>"#;
    let mut reader = Reader::from_reader(&data[..]);
    reader.config_mut().trim_text(true);
    loop {
        match reader.read_event() {
            Ok(Event::Eof) | Err(..) => break,
            _ => (),
        }
    }
}

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

/// Regression test for https://github.com/tafia/quick-xml/issues/299
#[test]
fn issue299() -> Result<(), Error> {
    let xml = r#"
<?xml version="1.0" encoding="utf8"?>
<MICEX_DOC xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <SECURITY SecurityId="PLZL" ISIN="RU000A0JNAA8" SecShortName="Short Name" PriceType="CASH">
    <RECORDS RecNo="1" TradeNo="1111" TradeDate="2021-07-08" TradeTime="15:00:00" BuySell="S" SettleCode="Y1Dt" Decimals="3" Price="13057.034" Quantity="766" Value="10001688.29" AccInt="0" Amount="10001688.29" Balance="766" TrdAccId="X0011" ClientDetails="2222" CPFirmId="3333" CPFirmShortName="Firm Short Name" Price2="13057.034" RepoPart="2" ReportTime="16:53:27" SettleTime="17:47:06" ClientCode="4444" DueDate="2021-07-09" EarlySettleStatus="N" RepoRate="5.45" RateType="FIX"/>
  </SECURITY>
</MICEX_DOC>
"#;
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event()? {
            Event::Start(e) | Event::Empty(e) => {
                let attr_count = match e.name().as_ref() {
                    b"MICEX_DOC" => 1,
                    b"SECURITY" => 4,
                    b"RECORDS" => 26,
                    _ => unreachable!(),
                };
                assert_eq!(
                    attr_count,
                    e.attributes().filter(Result::is_ok).count(),
                    "mismatch att count on '{:?}'",
                    reader.decoder().decode(e.name().as_ref())
                );
            }
            Event::Eof => break,
            _ => (),
        }
    }
    Ok(())
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

        reader.config_mut().check_end_names = false;

        assert_eq!(reader.read_text(html_end.name()).unwrap(), "...");

        reader.config_mut().check_end_names = true;

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

        reader.config_mut().check_end_names = false;

        assert_eq!(reader.read_text(html_end.name()).unwrap(), "...");

        reader.config_mut().check_end_names = true;

        match reader.read_event() {
            Err(Error::IllFormed(cause)) => assert_eq!(
                cause,
                IllFormedError::MismatchedEndTag {
                    expected: "some-tag".into(),
                    found: "other-tag".into(),
                }
            ),
            x => panic!("Expected `Err(IllFormed(_))`, but got `{:?}`", x),
        }
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }
}

/// Regression test for https://github.com/tafia/quick-xml/issues/604
mod issue604 {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn short() {
        let data = b"<?xml version=\"1.0\"?><!-->";
        let mut reader = Reader::from_reader(data.as_slice());
        let mut buf = Vec::new();
        assert_eq!(
            reader.read_event_into(&mut buf).unwrap(),
            Event::Decl(BytesDecl::new("1.0", None, None))
        );
        match reader.read_event_into(&mut buf) {
            Err(Error::Syntax(SyntaxError::UnclosedComment)) => {}
            x => panic!("Expected `Err(Syntax(UnclosedComment))`, but got `{:?}`", x),
        }
        assert_eq!(reader.read_event_into(&mut buf).unwrap(), Event::Eof);
    }

    #[test]
    fn long() {
        let data = b"<?xml version=\"1.0\"?><!--->";
        let mut reader = Reader::from_reader(data.as_slice());
        let mut buf = Vec::new();
        assert_eq!(
            reader.read_event_into(&mut buf).unwrap(),
            Event::Decl(BytesDecl::new("1.0", None, None))
        );
        match reader.read_event_into(&mut buf) {
            Err(Error::Syntax(SyntaxError::UnclosedComment)) => {}
            x => panic!("Expected `Err(Syntax(UnclosedComment))`, but got `{:?}`", x),
        }
        assert_eq!(reader.read_event_into(&mut buf).unwrap(), Event::Eof);
    }

    /// According to the grammar, `>` is allowed just in start of comment.
    /// See https://www.w3.org/TR/xml11/#sec-comments
    #[test]
    fn short_valid() {
        let data = b"<?xml version=\"1.0\"?><!-->-->";
        let mut reader = Reader::from_reader(data.as_slice());
        let mut buf = Vec::new();
        assert_eq!(
            reader.read_event_into(&mut buf).unwrap(),
            Event::Decl(BytesDecl::new("1.0", None, None))
        );
        assert_eq!(
            reader.read_event_into(&mut buf).unwrap(),
            Event::Comment(BytesText::from_escaped(">"))
        );
        assert_eq!(reader.read_event_into(&mut buf).unwrap(), Event::Eof);
    }

    /// According to the grammar, `->` is allowed just in start of comment.
    /// See https://www.w3.org/TR/xml11/#sec-comments
    #[test]
    fn long_valid() {
        let data = b"<?xml version=\"1.0\"?><!--->-->";
        let mut reader = Reader::from_reader(data.as_slice());
        let mut buf = Vec::new();
        assert_eq!(
            reader.read_event_into(&mut buf).unwrap(),
            Event::Decl(BytesDecl::new("1.0", None, None))
        );
        assert_eq!(
            reader.read_event_into(&mut buf).unwrap(),
            Event::Comment(BytesText::from_escaped("->"))
        );
        assert_eq!(reader.read_event_into(&mut buf).unwrap(), Event::Eof);
    }
}

/// Regression test for https://github.com/tafia/quick-xml/issues/622
#[test]
fn issue622() {
    let mut reader = Reader::from_str("><");
    reader.config_mut().trim_text(true);

    assert_eq!(
        reader.read_event().unwrap(),
        Event::Text(BytesText::from_escaped(">"))
    );
    match reader.read_event() {
        Err(Error::Syntax(cause)) => assert_eq!(cause, SyntaxError::UnclosedTag),
        x => panic!("Expected `Err(Syntax(_))`, but got `{:?}`", x),
    }
}

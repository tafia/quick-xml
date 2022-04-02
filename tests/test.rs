use quick_xml::events::attributes::Attribute;
use quick_xml::events::Event::*;
use quick_xml::name::QName;
use quick_xml::reader::Reader;
use quick_xml::Error;
use std::borrow::Cow;

use pretty_assertions::assert_eq;

#[test]
fn test_sample() {
    let src = include_str!("documents/sample_rss.xml");
    let mut r = Reader::from_str(src);
    let mut count = 0;
    loop {
        match r.read_event().unwrap() {
            Start(_) => count += 1,
            Decl(e) => println!("{:?}", e.version()),
            Eof => break,
            _ => (),
        }
    }
    println!("{}", count);
}

#[test]
fn test_attributes_empty() {
    let src = "<a att1='a' att2='b'/>";
    let mut r = Reader::from_str(src);
    r.trim_text(true).expand_empty_elements(false);
    match r.read_event() {
        Ok(Empty(e)) => {
            let mut attrs = e.attributes();
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"att1"),
                    value: Cow::Borrowed(b"a"),
                }))
            );
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"att2"),
                    value: Cow::Borrowed(b"b"),
                }))
            );
            assert_eq!(attrs.next(), None);
        }
        e => panic!("Expecting Empty event, got {:?}", e),
    }
}

#[test]
fn test_attribute_equal() {
    let src = "<a att1=\"a=b\"/>";
    let mut r = Reader::from_str(src);
    r.trim_text(true).expand_empty_elements(false);
    match r.read_event() {
        Ok(Empty(e)) => {
            let mut attrs = e.attributes();
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"att1"),
                    value: Cow::Borrowed(b"a=b"),
                }))
            );
            assert_eq!(attrs.next(), None);
        }
        e => panic!("Expecting Empty event, got {:?}", e),
    }
}

#[test]
fn test_comment_starting_with_gt() {
    let src = "<a /><!-->-->";
    let mut r = Reader::from_str(src);
    r.trim_text(true).expand_empty_elements(false);
    loop {
        match r.read_event() {
            Ok(Comment(e)) => {
                assert_eq!(e.as_ref(), b">");
                break;
            }
            Ok(Eof) => panic!("Expecting Comment"),
            _ => (),
        }
    }
}

#[test]
fn test_issue94() {
    let data = br#"<Run>
<!B>
</Run>"#;
    let mut reader = Reader::from_reader(&data[..]);
    reader.trim_text(true);
    loop {
        match reader.read_event() {
            Ok(Eof) | Err(..) => break,
            _ => (),
        }
    }
}

#[test]
fn test_no_trim() {
    let mut reader = Reader::from_str(" <tag> text </tag> ");

    assert!(matches!(reader.read_event().unwrap(), Text(_)));
    assert!(matches!(reader.read_event().unwrap(), Start(_)));
    assert!(matches!(reader.read_event().unwrap(), Text(_)));
    assert!(matches!(reader.read_event().unwrap(), End(_)));
    assert!(matches!(reader.read_event().unwrap(), Text(_)));
}

#[test]
fn test_trim_end() {
    let mut reader = Reader::from_str(" <tag> text </tag> ");
    reader.trim_text_end(true);

    assert!(matches!(reader.read_event().unwrap(), Text(_)));
    assert!(matches!(reader.read_event().unwrap(), Start(_)));
    assert!(matches!(reader.read_event().unwrap(), Text(_)));
    assert!(matches!(reader.read_event().unwrap(), End(_)));
}

#[test]
fn test_trim() {
    let mut reader = Reader::from_str(" <tag> text </tag> ");
    reader.trim_text(true);

    assert!(matches!(reader.read_event().unwrap(), Start(_)));
    assert!(matches!(reader.read_event().unwrap(), Text(_)));
    assert!(matches!(reader.read_event().unwrap(), End(_)));
}

#[test]
fn test_clone_reader() {
    let mut reader = Reader::from_str("<tag>text</tag>");
    reader.trim_text(true);

    assert!(matches!(reader.read_event().unwrap(), Start(_)));

    let mut cloned = reader.clone();

    assert!(matches!(reader.read_event().unwrap(), Text(_)));
    assert!(matches!(reader.read_event().unwrap(), End(_)));

    assert!(matches!(cloned.read_event().unwrap(), Text(_)));
    assert!(matches!(cloned.read_event().unwrap(), End(_)));
}

#[test]
fn test_issue299() -> Result<(), Error> {
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
            Start(e) | Empty(e) => {
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
            Eof => break,
            _ => (),
        }
    }
    Ok(())
}

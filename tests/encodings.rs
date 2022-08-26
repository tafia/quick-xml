#[allow(unused_imports)]
use quick_xml::events::Event;
#[allow(unused_imports)]
use quick_xml::Reader;

#[cfg(feature = "encoding")]
mod decode {
    use encoding_rs::{UTF_16BE, UTF_16LE, UTF_8};
    use pretty_assertions::assert_eq;
    use quick_xml::encoding::*;
    use std::borrow::Cow;

    static UTF16BE_TEXT_WITH_BOM: &[u8] = include_bytes!("./documents/utf16be.xml");
    static UTF16LE_TEXT_WITH_BOM: &[u8] = include_bytes!("./documents/utf16le.xml");
    static UTF8_TEXT_WITH_BOM: &[u8] = include_bytes!("./documents/utf8.xml");

    static UTF8_TEXT: &str = r#"<?xml version="1.0"?>
<project name="project-name">
</project>
"#;

    #[test]
    fn test_removes_bom() {
        // No BOM
        assert_eq!(
            decode_with_bom_removal(UTF8_TEXT.as_bytes()).unwrap(),
            Cow::Borrowed(UTF8_TEXT)
        );
        // BOM
        assert_eq!(
            decode_with_bom_removal(UTF8_TEXT_WITH_BOM).unwrap(),
            Cow::Borrowed(UTF8_TEXT)
        );
        assert_eq!(
            decode_with_bom_removal(UTF16BE_TEXT_WITH_BOM).unwrap(),
            Cow::Borrowed(UTF8_TEXT).into_owned()
        );
        assert_eq!(
            decode_with_bom_removal(UTF16LE_TEXT_WITH_BOM).unwrap(),
            Cow::Borrowed(UTF8_TEXT).into_owned()
        );
    }

    #[test]
    fn test_detect_encoding() {
        // No BOM
        assert_eq!(detect_encoding(UTF8_TEXT.as_bytes()), Some(UTF_8));
        // BOM
        assert_eq!(detect_encoding(UTF8_TEXT_WITH_BOM), Some(UTF_8));
        assert_eq!(detect_encoding(UTF16BE_TEXT_WITH_BOM), Some(UTF_16BE));
        assert_eq!(detect_encoding(UTF16LE_TEXT_WITH_BOM), Some(UTF_16LE));
    }

    #[test]
    fn test_decode_with_bom_removal() {
        // No BOM
        assert_eq!(
            decode_with_bom_removal(UTF8_TEXT.as_bytes()).unwrap(),
            UTF8_TEXT
        );
        // BOM
        assert_eq!(
            decode_with_bom_removal(UTF8_TEXT_WITH_BOM).unwrap(),
            UTF8_TEXT
        );
        assert_eq!(
            decode_with_bom_removal(UTF16BE_TEXT_WITH_BOM).unwrap(),
            UTF8_TEXT
        );
        assert_eq!(
            decode_with_bom_removal(UTF16LE_TEXT_WITH_BOM).unwrap(),
            UTF8_TEXT
        );
    }
}

#[test]
#[cfg(feature = "encoding")]
fn test_koi8_r_encoding() {
    let src = include_bytes!("documents/opennews_all.rss").as_ref();
    let mut buf = vec![];
    let mut r = Reader::from_reader(src);
    r.trim_text(true).expand_empty_elements(false);
    loop {
        match r.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => {
                e.unescape().unwrap();
            }
            Ok(Event::Eof) => break,
            _ => (),
        }
    }
}

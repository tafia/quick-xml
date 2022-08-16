use quick_xml::events::Event;
use quick_xml::Reader;

mod decode {
    use encoding_rs::{UTF_16BE, UTF_16LE, UTF_8};
    use pretty_assertions::assert_eq;
    use quick_xml::encoding::*;

    static UTF16BE_TEXT_WITH_BOM: &[u8] = include_bytes!("documents/encoding/utf16be-bom.xml");
    static UTF16LE_TEXT_WITH_BOM: &[u8] = include_bytes!("documents/encoding/utf16le-bom.xml");
    static UTF8_TEXT_WITH_BOM: &[u8] = include_bytes!("documents/encoding/utf8-bom.xml");

    static UTF8_TEXT: &str = r#"<?xml version="1.0"?>
<project name="project-name">
</project>
"#;

    #[test]
    fn test_detect_encoding() {
        // No BOM
        assert_eq!(detect_encoding(UTF8_TEXT.as_bytes()), Some(UTF_8));
        // BOM
        assert_eq!(detect_encoding(UTF8_TEXT_WITH_BOM), Some(UTF_8));
        assert_eq!(detect_encoding(UTF16BE_TEXT_WITH_BOM), Some(UTF_16BE));
        assert_eq!(detect_encoding(UTF16LE_TEXT_WITH_BOM), Some(UTF_16LE));
    }
}

#[test]
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

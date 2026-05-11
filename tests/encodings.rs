use encoding_rs::{UTF_16BE, UTF_16LE, UTF_8, WINDOWS_1251};
use pretty_assertions::assert_eq;
use quick_xml::events::Event::*;
use quick_xml::reader::Reader;

static RSS_DOC: &[u8] = include_bytes!("documents/opennews_all.rss");
static UTF16BE_TEXT_WITH_BOM: &[u8] = include_bytes!("documents/encoding/utf16be-bom.xml");
static UTF16LE_TEXT_WITH_BOM: &[u8] = include_bytes!("documents/encoding/utf16le-bom.xml");
static UTF8_TEXT_WITH_BOM: &[u8] = include_bytes!("documents/encoding/utf8-bom.xml");
static UTF8_TEXT: &[u8] = include_bytes!("documents/encoding/utf8.xml");

mod xml_decoding_reader {
    use super::*;
    use quick_xml::encoding::{DecodingReader, EncodingError};
    use quick_xml::errors::Error;

    /// Read events until an error occurs, panicking if EOF is reached first.
    fn read_until_error(data: &[u8]) -> Error {
        let mut buf = Vec::new();
        let mut r = Reader::from_reader(DecodingReader::new(data));
        r.config_mut().trim_text(true);
        loop {
            match r.read_event_into(&mut buf) {
                Ok(Eof) => panic!("Expected encoding error, got EOF"),
                Ok(_) => continue,
                Err(e) => break e,
            }
        }
    }

    /// Invalid UTF-8 (no BOM, so treated as UTF-8) must produce an error.
    #[test]
    fn invalid_utf8_is_rejected() {
        // "<a>" followed by invalid byte 0xFF
        let result = read_until_error(&[0x3C, 0x61, 0x3E, 0xFF]);
        assert!(
            matches!(result, Error::Encoding(EncodingError::Other(_))),
            "Expected EncodingError::Other, got: {:?}",
            result,
        );
    }

    /// UTF-16 LE with valid XML followed by an unpaired low surrogate (0xDC00).
    #[test]
    fn invalid_utf16le_is_rejected() {
        let result = read_until_error(&[
            0xFF, 0xFE, // UTF-16 LE BOM
            0x3C, 0x00, // '<'
            0x61, 0x00, // 'a'
            0x3E, 0x00, // '>'
            0x00, 0xDC, // unpaired low surrogate
        ]);
        assert!(
            matches!(result, Error::Encoding(EncodingError::Other(_))),
            "Expected EncodingError::Other, got: {:?}",
            result,
        );
    }

    /// UTF-16 BE with valid XML followed by an unpaired low surrogate (0xDC00).
    #[test]
    fn invalid_utf16be_is_rejected() {
        let result = read_until_error(&[
            0xFE, 0xFF, // UTF-16 BE BOM
            0x00, 0x3C, // '<'
            0x00, 0x61, // 'a'
            0x00, 0x3E, // '>'
            0xDC, 0x00, // unpaired low surrogate
        ]);
        assert!(
            matches!(result, Error::Encoding(EncodingError::Other(_))),
            "Expected EncodingError::Other, got: {:?}",
            result,
        );
    }

    /// An odd trailing byte in UTF-16 is malformed and must produce an error.
    #[test]
    fn truncated_utf16_at_eof() {
        let result = read_until_error(&[
            0xFF, 0xFE, // UTF-16 LE BOM
            0x3C, 0x00, // '<'
            0x61, 0x00, // 'a'
            0x3E, 0x00, // '>'
            0x48, 0x00, // 'H'
            0x65, // truncated code unit
        ]);
        assert!(
            matches!(result, Error::Encoding(EncodingError::Other(_))),
            "Expected EncodingError::Other, got: {:?}",
            result,
        );
    }

    // UTF-8 / 16 "happy paths" already well-tested in encoding.rs

    #[test]
    fn test_koi8_r_encoding() {
        let mut buf = vec![];
        let mut r = Reader::from_reader(DecodingReader::new(RSS_DOC));
        r.config_mut().trim_text(true);
        loop {
            match r.read_event_into(&mut buf) {
                Ok(Text(e)) => {
                    e.xml10_content();
                }
                Ok(Eof) => break,
                _ => (),
            }
        }
    }

    macro_rules! check_decoding_reader {
        ($test:ident, $enc:ident, $file:literal) => {
            #[test]
            fn $test() {
                let mut r = Reader::from_reader(DecodingReader::new(
                    include_bytes!(concat!("documents/encoding/", $file, ".xml")).as_ref(),
                ));

                let mut buf = Vec::new();
                loop {
                    match r.read_event_into(&mut buf).unwrap() {
                        Eof => break,
                        Decl(e) => {
                            if let Some(encoding) = e.encoder() {
                                r.get_mut().set_encoding(encoding);
                            }
                        }
                        _ => {}
                    }
                    assert_eq!(r.get_ref().encoding(), $enc);
                    buf.clear();
                }
            }
        };
    }

    mod decode_using_declaration {
        use super::*;
        use encoding_rs::*;
        use pretty_assertions::assert_eq;

        // Without BOM
        check_decoding_reader!(utf8, UTF_8, "utf8");
        check_decoding_reader!(utf16be, UTF_16BE, "utf16be");
        check_decoding_reader!(utf16le, UTF_16LE, "utf16le");

        // With BOM
        check_decoding_reader!(utf8_bom, UTF_8, "utf8-bom");
        check_decoding_reader!(utf16be_bom, UTF_16BE, "utf16be-bom");
        check_decoding_reader!(utf16le_bom, UTF_16LE, "utf16le-bom");

        // legacy multi-byte encodings
        check_decoding_reader!(big5, BIG5, "Big5");
        check_decoding_reader!(euc_jp, EUC_JP, "EUC-JP");
        check_decoding_reader!(iso_2022_jp, ISO_2022_JP, "ISO-2022-JP");
        check_decoding_reader!(euc_kr, EUC_KR, "EUC-KR");
        check_decoding_reader!(gb18030, GB18030, "gb18030");
        check_decoding_reader!(gbk, GBK, "GBK");
        check_decoding_reader!(shift_jis, SHIFT_JIS, "Shift_JIS");

        // legacy single-byte encodings
        check_decoding_reader!(ibm866, IBM866, "IBM866");
        check_decoding_reader!(iso_8859_2, ISO_8859_2, "ISO-8859-2");
        check_decoding_reader!(iso_8859_3, ISO_8859_3, "ISO-8859-3");
        check_decoding_reader!(iso_8859_4, ISO_8859_4, "ISO-8859-4");
        check_decoding_reader!(iso_8859_5, ISO_8859_5, "ISO-8859-5");
        check_decoding_reader!(iso_8859_6, ISO_8859_6, "ISO-8859-6");
        check_decoding_reader!(iso_8859_7, ISO_8859_7, "ISO-8859-7");
        check_decoding_reader!(iso_8859_8, ISO_8859_8, "ISO-8859-8");
        check_decoding_reader!(iso_8859_8_i, ISO_8859_8_I, "ISO-8859-8-I");
        check_decoding_reader!(iso_8859_10, ISO_8859_10, "ISO-8859-10");
        check_decoding_reader!(iso_8859_13, ISO_8859_13, "ISO-8859-13");
        check_decoding_reader!(iso_8859_14, ISO_8859_14, "ISO-8859-14");
        check_decoding_reader!(iso_8859_15, ISO_8859_15, "ISO-8859-15");
        check_decoding_reader!(iso_8859_16, ISO_8859_16, "ISO-8859-16");
        check_decoding_reader!(koi8_r, KOI8_R, "KOI8-R");
        check_decoding_reader!(koi8_u, KOI8_U, "KOI8-U");
        check_decoding_reader!(macintosh, MACINTOSH, "macintosh");
        check_decoding_reader!(windows_874, WINDOWS_874, "windows-874");
        check_decoding_reader!(windows_1250, WINDOWS_1250, "windows-1250");
        check_decoding_reader!(windows_1251, WINDOWS_1251, "windows-1251");
        check_decoding_reader!(windows_1252, WINDOWS_1252, "windows-1252");
        check_decoding_reader!(windows_1253, WINDOWS_1253, "windows-1253");
        check_decoding_reader!(windows_1254, WINDOWS_1254, "windows-1254");
        check_decoding_reader!(windows_1255, WINDOWS_1255, "windows-1255");
        check_decoding_reader!(windows_1256, WINDOWS_1256, "windows-1256");
        check_decoding_reader!(windows_1257, WINDOWS_1257, "windows-1257");
        check_decoding_reader!(windows_1258, WINDOWS_1258, "windows-1258");
        check_decoding_reader!(x_mac_cyrillic, X_MAC_CYRILLIC, "x-mac-cyrillic");
        check_decoding_reader!(x_user_defined, X_USER_DEFINED, "x-user-defined");
    }
}

/// Tests for the post-parse decoding approach
mod legacy_decoding {
    use super::*;

    #[test]
    fn koi8_r() {
        let mut buf = Vec::new();
        let mut r = Reader::from_reader(RSS_DOC);
        r.config_mut().trim_text(true);
        loop {
            match r.read_event_into(&mut buf) {
                Ok(Text(e)) => {
                    e.xml10_content();
                }
                Ok(Eof) => break,
                _ => (),
            }
        }
    }
}

#[test]
fn test_detect_encoding() {
    use quick_xml::encoding::detect_encoding;

    // No BOM
    let detected = detect_encoding(UTF8_TEXT).unwrap();
    assert_eq!(detected.encoding(), UTF_8);
    assert_eq!(detected.bom_len(), 0);

    // BOM
    let detected = detect_encoding(UTF8_TEXT_WITH_BOM).unwrap();
    assert_eq!(detected.encoding(), UTF_8);
    assert_eq!(detected.bom_len(), 3);

    let detected = detect_encoding(UTF16BE_TEXT_WITH_BOM).unwrap();
    assert_eq!(detected.encoding(), UTF_16BE);
    assert_eq!(detected.bom_len(), 2);

    let detected = detect_encoding(UTF16LE_TEXT_WITH_BOM).unwrap();
    assert_eq!(detected.encoding(), UTF_16LE);
    assert_eq!(detected.bom_len(), 2);
}

/// Test data generated by helper project `test-gen`, which requires checkout of
/// an `encoding` submodule
mod detect {
    use super::*;
    use encoding_rs::*;
    use pretty_assertions::assert_eq;

    macro_rules! assert_matches {
        ($number:literal : $left:expr, $pattern:pat_param) => {{
            let event = $left;
            if !matches!(event, $pattern) {
                assert_eq!(
                    format!("{:#?}", event),
                    stringify!($pattern),
                    concat!("Message ", stringify!($number), " is incorrect")
                );
            }
        }};
    }
    macro_rules! check_detection {
        ($test:ident, $enc:ident, $file:literal) => {
            #[test]
            fn $test() {
                use quick_xml::errors::Error;

                let mut r = Reader::from_reader(
                    include_bytes!(concat!("documents/encoding/", $file, ".xml")).as_ref(),
                );
                assert_eq!(r.decoder().encoding(), UTF_8);

                let mut buf = Vec::new();
                // XML declaration with encoding (pure ASCII)
                assert_matches!(1: r.read_event_into(&mut buf).unwrap(), Decl(_));
                assert_eq!(r.decoder().encoding(), $enc);
                assert_matches!(2: r.read_event_into(&mut buf).unwrap(), Text(_)); // spaces
                buf.clear();

                // Comment with information that this is generated file (pure ASCII)
                assert_matches!(3: r.read_event_into(&mut buf).unwrap(), Comment(_));
                assert_eq!(r.decoder().encoding(), $enc);
                assert_matches!(4: r.read_event_into(&mut buf).unwrap(), Text(_)); // spaces
                buf.clear();

                // The start tag contains non-UTF-8 attribute values.
                // Without DecodingReader, reading non-UTF-8 content must produce an encoding error.
                let err = r.read_event_into(&mut buf).unwrap_err();
                assert!(
                    matches!(err, Error::Encoding(_)),
                    "Expected Error::Encoding, got: {:?}",
                    err,
                );
            }
        };
    }
    macro_rules! detect_test {
        ($test:ident, $enc:ident, $file:literal $($break:stmt)?) => {
            #[test]
            fn $test() {
                let mut r = Reader::from_reader(
                    include_bytes!(concat!("documents/encoding/", $file, ".xml")).as_ref(),
                );
                assert_eq!(r.decoder().encoding(), UTF_8);

                let mut buf = Vec::new();
                loop {
                    match dbg!(r.read_event_into(&mut buf).unwrap()) {
                        Eof => break,
                        _ => {}
                    }
                    assert_eq!(r.decoder().encoding(), $enc);
                    buf.clear();
                    $($break)?
                }
            }
        };
    }

    // Without BOM
    detect_test!(utf8, UTF_8, "utf8");
    detect_test!(utf16be, UTF_16BE, "utf16be" break);
    detect_test!(utf16le, UTF_16LE, "utf16le" break);

    // With BOM
    detect_test!(utf8_bom, UTF_8, "utf8-bom");
    detect_test!(utf16be_bom, UTF_16BE, "utf16be-bom" break);
    detect_test!(utf16le_bom, UTF_16LE, "utf16le-bom" break);

    // legacy multi-byte encodings (7)
    check_detection!(big5, BIG5, "Big5");
    check_detection!(euc_jp, EUC_JP, "EUC-JP");
    check_detection!(euc_kr, EUC_KR, "EUC-KR");
    check_detection!(gb18030, GB18030, "gb18030");
    check_detection!(gbk, GBK, "GBK");
    // XML in this encoding cannot be parsed successfully without DecodingReader,
    // because encoding is stateful and the same byte may have different meaning
    // depending on the previous bytes in the stream.
    // We only read the first event to ensure, that encoding detected correctly
    detect_test!(iso_2022_jp, ISO_2022_JP, "ISO-2022-JP" break);
    check_detection!(shift_jis, SHIFT_JIS, "Shift_JIS");

    // legacy single-byte encodings (19)
    check_detection!(ibm866, IBM866, "IBM866");
    check_detection!(iso_8859_2, ISO_8859_2, "ISO-8859-2");
    check_detection!(iso_8859_3, ISO_8859_3, "ISO-8859-3");
    check_detection!(iso_8859_4, ISO_8859_4, "ISO-8859-4");
    check_detection!(iso_8859_5, ISO_8859_5, "ISO-8859-5");
    check_detection!(iso_8859_6, ISO_8859_6, "ISO-8859-6");
    check_detection!(iso_8859_7, ISO_8859_7, "ISO-8859-7");
    check_detection!(iso_8859_8, ISO_8859_8, "ISO-8859-8");
    check_detection!(iso_8859_8_i, ISO_8859_8_I, "ISO-8859-8-I");
    check_detection!(iso_8859_10, ISO_8859_10, "ISO-8859-10");
    check_detection!(iso_8859_13, ISO_8859_13, "ISO-8859-13");
    check_detection!(iso_8859_14, ISO_8859_14, "ISO-8859-14");
    check_detection!(iso_8859_15, ISO_8859_15, "ISO-8859-15");
    check_detection!(iso_8859_16, ISO_8859_16, "ISO-8859-16");
    check_detection!(koi8_r, KOI8_R, "KOI8-R");
    check_detection!(koi8_u, KOI8_U, "KOI8-U");
    check_detection!(macintosh, MACINTOSH, "macintosh");
    check_detection!(windows_874, WINDOWS_874, "windows-874");
    check_detection!(windows_1250, WINDOWS_1250, "windows-1250");
    check_detection!(windows_1251, WINDOWS_1251, "windows-1251");
    check_detection!(windows_1252, WINDOWS_1252, "windows-1252");
    check_detection!(windows_1253, WINDOWS_1253, "windows-1253");
    check_detection!(windows_1254, WINDOWS_1254, "windows-1254");
    check_detection!(windows_1255, WINDOWS_1255, "windows-1255");
    check_detection!(windows_1256, WINDOWS_1256, "windows-1256");
    check_detection!(windows_1257, WINDOWS_1257, "windows-1257");
    check_detection!(windows_1258, WINDOWS_1258, "windows-1258");
    check_detection!(x_mac_cyrillic, X_MAC_CYRILLIC, "x-mac-cyrillic");
    check_detection!(x_user_defined, X_USER_DEFINED, "x-user-defined");
}

/// Checks that encoding is detected by BOM and changed after XML declaration
/// BOM indicates UTF-16LE, but XML declares windows-1251
#[test]
fn bom_overridden_by_declaration() {
    let mut reader = Reader::from_reader(b"\xFF\xFE<?xml encoding='windows-1251'?>".as_ref());
    let mut buf = Vec::new();

    assert_eq!(reader.decoder().encoding(), UTF_8);
    assert!(matches!(reader.read_event_into(&mut buf).unwrap(), Decl(_)));
    assert_eq!(reader.decoder().encoding(), WINDOWS_1251);

    assert_eq!(reader.read_event_into(&mut buf).unwrap(), Eof);
}

/// Checks that encoding is changed by XML declaration, but only once
#[test]
fn only_one_declaration_changes_encoding() {
    let mut reader =
        Reader::from_reader(b"<?xml encoding='UTF-16'?><?xml encoding='windows-1251'?>".as_ref());
    let mut buf = Vec::new();

    assert_eq!(reader.decoder().encoding(), UTF_8);
    assert!(matches!(reader.read_event_into(&mut buf).unwrap(), Decl(_)));
    assert_eq!(reader.decoder().encoding(), UTF_16LE);

    assert!(matches!(reader.read_event_into(&mut buf).unwrap(), Decl(_)));
    assert_eq!(reader.decoder().encoding(), UTF_16LE);

    assert_eq!(reader.read_event_into(&mut buf).unwrap(), Eof);
}

/// Checks that XML declaration cannot change the encoding from UTF-8 if
/// a `Reader` was created using `from_str` method
#[test]
fn str_always_has_utf8() {
    let mut reader = Reader::from_str("<?xml encoding='UTF-16'?>");

    assert_eq!(reader.decoder().encoding(), UTF_8);
    reader.read_event().unwrap();
    assert_eq!(reader.decoder().encoding(), UTF_8);

    assert_eq!(reader.read_event().unwrap(), Eof);
}

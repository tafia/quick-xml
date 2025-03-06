use quick_xml::events::{
    BytesCData, BytesDecl, BytesEnd, BytesPI, BytesRef, BytesStart, BytesText, Event::*,
};
use quick_xml::reader::Reader;

use pretty_assertions::assert_eq;

mod character_reference {
    use super::*;

    mod dec {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn decl() {
            for i in 0..=0x10FFFF {
                let input = format!("<?xml version=\"&{i};\"?>");
                let mut reader = Reader::from_str(&input);

                assert_eq!(
                    reader.read_event().unwrap(),
                    Decl(BytesDecl::new(&format!("&{i};"), None, None)),
                    "Character reference {i}=0x{i:x}: {input}"
                );
            }
        }

        #[test]
        fn pi() {
            for i in 0..=0x10FFFF {
                let input = format!("<?&{i};?>");
                let mut reader = Reader::from_str(&input);

                assert_eq!(
                    reader.read_event().unwrap(),
                    PI(BytesPI::new(&format!("&{i};"))),
                    "Character reference {i}=0x{i:x}: {input}"
                );
            }
        }

        #[test]
        fn doctype() {
            for i in 0..=0x10FFFF {
                let input = format!("<!DOCTYPE &{i};>");
                let mut reader = Reader::from_str(&input);

                assert_eq!(
                    reader.read_event().unwrap(),
                    DocType(BytesText::from_escaped(&format!("&{i};"))),
                    "Character reference {i}=0x{i:x}: {input}"
                );
            }
        }

        #[test]
        fn comment() {
            for i in 0..=0x10FFFF {
                let input = format!("<!--&{i};-->");
                let mut reader = Reader::from_str(&input);

                assert_eq!(
                    reader.read_event().unwrap(),
                    Comment(BytesText::from_escaped(&format!("&{i};"))),
                    "Character reference {i}=0x{i:x}: {input}"
                );
            }
        }

        #[test]
        fn cdata() {
            for i in 0..=0x10FFFF {
                let input = format!("<![CDATA[&{i};]]>");
                let mut reader = Reader::from_str(&input);

                assert_eq!(
                    reader.read_event().unwrap(),
                    CData(BytesCData::new(format!("&{i};"))),
                    "Character reference {i}=0x{i:x}: {input}"
                );
            }
        }

        #[test]
        fn text() {
            for i in 0..=0x10FFFF {
                let input = format!("&{i};");
                let mut reader = Reader::from_str(&input);

                assert_eq!(
                    reader.read_event().unwrap(),
                    GeneralRef(BytesRef::new(format!("{i}"))),
                    "Character reference {i}=0x{i:x}: {input}"
                );
            }
        }

        #[test]
        fn empty() {
            for i in 0u32..=0x10FFFF {
                let input = format!("<&{i}; &{i};='&{i};' &{i};=\"&{i};\" &{i};=&{i};/>");
                let mut reader = Reader::from_str(&input);

                let name_len = format!("&{i};").len();
                assert_eq!(
                    reader.read_event().unwrap(),
                    Empty(BytesStart::from_content(
                        format!("&{i}; &{i};='&{i};' &{i};=\"&{i};\" &{i};=&{i};"),
                        name_len
                    )),
                    "Character reference {i}=0x{i:x}: {input}"
                );
            }
        }

        #[test]
        fn start() {
            for i in 0..=0x10FFFF {
                let input = format!("<&{i}; &{i};='&{i};' &{i};=\"&{i};\" &{i};=&{i};>");
                let mut reader = Reader::from_str(&input);

                let name_len = format!("&{i};").len();
                assert_eq!(
                    reader.read_event().unwrap(),
                    Start(BytesStart::from_content(
                        format!("&{i}; &{i};='&{i};' &{i};=\"&{i};\" &{i};=&{i};"),
                        name_len
                    )),
                    "Character reference {i}=0x{i:x}: {input}"
                );
            }
        }

        #[test]
        fn end() {
            for i in 0..=0x10FFFF {
                let input = format!("<></&{i};>");
                let mut reader = Reader::from_str(&input);
                reader.config_mut().check_end_names = false;

                // Skip <>
                reader.read_event().unwrap();
                assert_eq!(
                    reader.read_event().unwrap(),
                    End(BytesEnd::new(format!("&{i};"))),
                    "Character reference {i}=0x{i:x}: {input}"
                );
            }
        }
    }

    mod hex {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn decl() {
            for i in 0..=0x10FFFF {
                let input = format!("<?xml version=\"&#{i:x};\"?>");
                let mut reader = Reader::from_str(&input);

                assert_eq!(
                    reader.read_event().unwrap(),
                    Decl(BytesDecl::new(&format!("&#{i:x};"), None, None)),
                    "Character reference {i}=0x{i:x}: {input}"
                );
            }
        }

        #[test]
        fn pi() {
            for i in 0..=0x10FFFF {
                let input = format!("<?&#{i:x};?>");
                let mut reader = Reader::from_str(&input);

                assert_eq!(
                    reader.read_event().unwrap(),
                    PI(BytesPI::new(&format!("&#{i:x};"))),
                    "Character reference {i}=0x{i:x}: {input}"
                );
            }
        }

        #[test]
        fn doctype() {
            for i in 0..=0x10FFFF {
                let input = format!("<!DOCTYPE &#{i:x};>");
                let mut reader = Reader::from_str(&input);

                assert_eq!(
                    reader.read_event().unwrap(),
                    DocType(BytesText::from_escaped(&format!("&#{i:x};"))),
                    "Character reference {i}=0x{i:x}: {input}"
                );
            }
        }

        #[test]
        fn comment() {
            for i in 0..=0x10FFFF {
                let input = format!("<!--&#{i:x};-->");
                let mut reader = Reader::from_str(&input);

                assert_eq!(
                    reader.read_event().unwrap(),
                    Comment(BytesText::from_escaped(&format!("&#{i:x};"))),
                    "Character reference {i}=0x{i:x}: {input}"
                );
            }
        }

        #[test]
        fn cdata() {
            for i in 0..=0x10FFFF {
                let input = format!("<![CDATA[&#{i:x};]]>");
                let mut reader = Reader::from_str(&input);

                assert_eq!(
                    reader.read_event().unwrap(),
                    CData(BytesCData::new(format!("&#{i:x};"))),
                    "Character reference {i}=0x{i:x}: {input}"
                );
            }
        }

        #[test]
        fn text() {
            for i in 0..=0x10FFFF {
                let input = format!("&#{i:x};");
                let mut reader = Reader::from_str(&input);

                assert_eq!(
                    reader.read_event().unwrap(),
                    GeneralRef(BytesRef::new(format!("#{i:x}"))),
                    "Character reference {i}=0x{i:x}: {input}"
                );
            }
        }

        #[test]
        fn empty() {
            for i in 0u32..=0x10FFFF {
                let input = format!(
                    "<&#{i:x}; &#{i:x};='&#{i:x};' &#{i:x};=\"&#{i:x};\" &#{i:x};=&#{i:x};/>"
                );
                let mut reader = Reader::from_str(&input);

                let name_len = format!("&#{i:x};").len();
                assert_eq!(
                    reader.read_event().unwrap(),
                    Empty(BytesStart::from_content(
                        format!(
                            "&#{i:x}; &#{i:x};='&#{i:x};' &#{i:x};=\"&#{i:x};\" &#{i:x};=&#{i:x};"
                        ),
                        name_len
                    )),
                    "Character reference {i}=0x{i:x}: {input}"
                );
            }
        }

        #[test]
        fn start() {
            for i in 0..=0x10FFFF {
                let input = format!(
                    "<&#{i:x}; &#{i:x};='&#{i:x};' &#{i:x};=\"&#{i:x};\" &#{i:x};=&#{i:x};>"
                );
                let mut reader = Reader::from_str(&input);

                let name_len = format!("&#{i:x};").len();
                assert_eq!(
                    reader.read_event().unwrap(),
                    Start(BytesStart::from_content(
                        format!(
                            "&#{i:x}; &#{i:x};='&#{i:x};' &#{i:x};=\"&#{i:x};\" &#{i:x};=&#{i:x};"
                        ),
                        name_len
                    )),
                    "Character reference {i}=0x{i:x}: {input}"
                );
            }
        }

        #[test]
        fn end() {
            for i in 0..=0x10FFFF {
                let input = format!("<></&#{i:x};>");
                let mut reader = Reader::from_str(&input);
                reader.config_mut().check_end_names = false;

                // Skip <>
                reader.read_event().unwrap();
                assert_eq!(
                    reader.read_event().unwrap(),
                    End(BytesEnd::new(format!("&#{i:x};"))),
                    "Character reference {i}=0x{i:x}: {input}"
                );
            }
        }
    }
}

mod general_entity_reference {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn decl() {
        let mut reader = Reader::from_str("<?xml version=\"&entity;\"?>");

        assert_eq!(
            reader.read_event().unwrap(),
            Decl(BytesDecl::new("&entity;", None, None)),
        );
    }

    #[test]
    fn pi() {
        let mut reader = Reader::from_str("<?&entity;?>");

        assert_eq!(reader.read_event().unwrap(), PI(BytesPI::new("&entity;")));
    }

    #[test]
    fn doctype() {
        let mut reader = Reader::from_str("<!DOCTYPE &entity;>");

        assert_eq!(
            reader.read_event().unwrap(),
            DocType(BytesText::from_escaped("&entity;")),
        );
    }

    #[test]
    fn comment() {
        let mut reader = Reader::from_str("<!--&entity;-->");

        assert_eq!(
            reader.read_event().unwrap(),
            Comment(BytesText::from_escaped("&entity;")),
        );
    }

    #[test]
    fn cdata() {
        let mut reader = Reader::from_str("<![CDATA[&entity;]]>");

        assert_eq!(
            reader.read_event().unwrap(),
            CData(BytesCData::new("&entity;")),
        );
    }

    #[test]
    fn text() {
        let mut reader = Reader::from_str("&entity;");

        assert_eq!(
            reader.read_event().unwrap(),
            GeneralRef(BytesRef::new("entity")),
        );
    }

    #[test]
    fn empty() {
        let mut reader = Reader::from_str(
            "<&entity; &entity;='&entity;' &entity;=\"&entity;\" &entity;=&entity;/>",
        );

        let name_len = "&entity;".len();
        assert_eq!(
            reader.read_event().unwrap(),
            Empty(BytesStart::from_content(
                "&entity; &entity;='&entity;' &entity;=\"&entity;\" &entity;=&entity;",
                name_len
            )),
        );
    }

    #[test]
    fn start() {
        let mut reader = Reader::from_str(
            "<&entity; &entity;='&entity;' &entity;=\"&entity;\" &entity;=&entity;>",
        );

        let name_len = "&entity;".len();
        assert_eq!(
            reader.read_event().unwrap(),
            Start(BytesStart::from_content(
                "&entity; &entity;='&entity;' &entity;=\"&entity;\" &entity;=&entity;",
                name_len
            )),
        );
    }

    #[test]
    fn end() {
        let mut reader = Reader::from_str("<></&entity;>");
        reader.config_mut().check_end_names = false;

        // Skip <>
        reader.read_event().unwrap();
        assert_eq!(reader.read_event().unwrap(), End(BytesEnd::new("&entity;")));
    }
}

/// _Parameter entity references_ are references to entities recognized within DTD.
/// That references recognized [only] inside DTD (`<!DOCTYPE>` declaration) and have a
/// form `%name;` (percent sign, name, semicolon).
///
/// Parameter entities are so-called _parsed entities_, i.e. the content of this
/// reference is a part of DTD and MUST follow DTD grammar after all substitutions.
/// That also means that DTD could be self-modified.
///
/// In those tests, however, parameter entity references are not recognized.
///
/// [only]: https://www.w3.org/TR/xml11/#indtd
mod parameter_entity_reference {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn decl() {
        let mut reader = Reader::from_str("<?xml version=\"%param;\"?>");

        assert_eq!(
            reader.read_event().unwrap(),
            Decl(BytesDecl::new("%param;", None, None)),
        );
    }

    #[test]
    fn pi() {
        let mut reader = Reader::from_str("<?%param;?>");

        assert_eq!(reader.read_event().unwrap(), PI(BytesPI::new("%param;")));
    }

    /// Because we do not parse DTD, we do not recognize parameter reference here yet.
    /// TODO: Recognize parameter entity references when DTD parsing will be implemented
    #[test]
    fn doctype() {
        let mut reader = Reader::from_str("<!DOCTYPE %param;>");

        assert_eq!(
            reader.read_event().unwrap(),
            DocType(BytesText::from_escaped("%param;")),
        );
    }

    /// Comments can be part of DTD, but parameter entity references does not recognized within them.
    ///
    /// See: <https://www.w3.org/TR/xml11/#sec-comments>
    #[test]
    fn comment() {
        let mut reader = Reader::from_str("<!--%param;-->");

        assert_eq!(
            reader.read_event().unwrap(),
            Comment(BytesText::from_escaped("%param;")),
        );
    }

    #[test]
    fn cdata() {
        let mut reader = Reader::from_str("<![CDATA[%param;]]>");

        assert_eq!(
            reader.read_event().unwrap(),
            CData(BytesCData::new("%param;")),
        );
    }

    #[test]
    fn text() {
        let mut reader = Reader::from_str("%param;");

        assert_eq!(
            reader.read_event().unwrap(),
            Text(BytesText::from_escaped("%param;")),
        );
    }

    #[test]
    fn empty() {
        let mut reader =
            Reader::from_str("<%param; %param;='%param;' %param;=\"%param;\" %param;=%param;/>");

        let name_len = "%param;".len();
        assert_eq!(
            reader.read_event().unwrap(),
            Empty(BytesStart::from_content(
                "%param; %param;='%param;' %param;=\"%param;\" %param;=%param;",
                name_len
            )),
        );
    }

    #[test]
    fn start() {
        let mut reader =
            Reader::from_str("<%param; %param;='%param;' %param;=\"%param;\" %param;=%param;>");

        let name_len = "%param;".len();
        assert_eq!(
            reader.read_event().unwrap(),
            Start(BytesStart::from_content(
                "%param; %param;='%param;' %param;=\"%param;\" %param;=%param;",
                name_len
            )),
        );
    }

    #[test]
    fn end() {
        let mut reader = Reader::from_str("<></%param;>");
        reader.config_mut().check_end_names = false;

        // Skip <>
        reader.read_event().unwrap();
        assert_eq!(reader.read_event().unwrap(), End(BytesEnd::new("%param;")));
    }
}

#[test]
fn mixed_text() {
    let input = "text with &lt;&amp;'&#32;' or '&#x20;'";
    let mut r = Reader::from_str(input);

    assert_eq!(
        r.read_event().unwrap(),
        Text(BytesText::from_escaped("text with "))
    );
    assert_eq!(r.read_event().unwrap(), GeneralRef(BytesRef::new("lt")));
    assert_eq!(r.read_event().unwrap(), GeneralRef(BytesRef::new("amp")));
    assert_eq!(r.read_event().unwrap(), Text(BytesText::from_escaped("'")));
    assert_eq!(r.read_event().unwrap(), GeneralRef(BytesRef::new("#32")));
    assert_eq!(
        r.read_event().unwrap(),
        Text(BytesText::from_escaped("' or '"))
    );
    assert_eq!(r.read_event().unwrap(), GeneralRef(BytesRef::new("#x20")));
    assert_eq!(r.read_event().unwrap(), Text(BytesText::from_escaped("'")));
    assert_eq!(r.read_event().unwrap(), Eof);
}

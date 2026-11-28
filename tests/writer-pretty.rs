use quick_xml::events::{BytesComment, BytesEnd, BytesRef, BytesStart, BytesText, Event};
use quick_xml::writer::PrettyWriter;

mod lxml_reference_cases {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn element_only_indented() {
        let mut writer = PrettyWriter::new(Vec::new());

        writer
            .write_event(Event::Start(BytesStart::new("root")))
            .unwrap();
        writer
            .write_event(Event::Empty(BytesStart::new("a")))
            .unwrap();
        writer
            .write_event(Event::Empty(BytesStart::new("b")))
            .unwrap();
        writer
            .write_event(Event::End(BytesEnd::new("root")))
            .unwrap();

        let result = writer.into_inner().unwrap();
        assert_eq!(
            std::str::from_utf8(&result).unwrap(),
            "<root>\n  <a/>\n  <b/>\n</root>"
        );
    }

    #[test]
    fn comment_then_text_inline() {
        let mut writer = PrettyWriter::new(Vec::new());

        writer
            .write_event(Event::Start(BytesStart::new("tag1")))
            .unwrap();
        writer
            .write_event(Event::Start(BytesStart::new("tag2")))
            .unwrap();
        writer
            .write_event(Event::Comment(BytesComment::new("c").unwrap()))
            .unwrap();
        writer
            .write_event(Event::Text(BytesText::new("Test")))
            .unwrap();
        writer
            .write_event(Event::End(BytesEnd::new("tag2")))
            .unwrap();
        writer
            .write_event(Event::End(BytesEnd::new("tag1")))
            .unwrap();

        let result = writer.into_inner().unwrap();
        // tag2 has text child, so it stays inline, but tag1 is element-only
        assert_eq!(
            std::str::from_utf8(&result).unwrap(),
            "<tag1>\n  <tag2><!--c-->Test</tag2>\n</tag1>"
        );
    }

    #[test]
    fn element_then_text_inline() {
        let mut writer = PrettyWriter::new(Vec::new());

        writer
            .write_event(Event::Start(BytesStart::new("a")))
            .unwrap();
        writer
            .write_event(Event::Empty(BytesStart::new("b")))
            .unwrap();
        writer
            .write_event(Event::Text(BytesText::new("text")))
            .unwrap();
        writer.write_event(Event::End(BytesEnd::new("a"))).unwrap();

        let result = writer.into_inner().unwrap();
        assert_eq!(std::str::from_utf8(&result).unwrap(), "<a><b/>text</a>");
    }

    #[test]
    fn text_then_element_inline() {
        let mut writer = PrettyWriter::new(Vec::new());

        writer
            .write_event(Event::Start(BytesStart::new("a")))
            .unwrap();
        writer
            .write_event(Event::Text(BytesText::new("text")))
            .unwrap();
        writer
            .write_event(Event::Empty(BytesStart::new("b")))
            .unwrap();
        writer.write_event(Event::End(BytesEnd::new("a"))).unwrap();

        let result = writer.into_inner().unwrap();
        assert_eq!(std::str::from_utf8(&result).unwrap(), "<a>text<b/></a>");
    }

    #[test]
    fn comment_only_indented() {
        let mut writer = PrettyWriter::new(Vec::new());

        writer
            .write_event(Event::Start(BytesStart::new("a")))
            .unwrap();
        writer
            .write_event(Event::Comment(BytesComment::new("c").unwrap()))
            .unwrap();
        writer.write_event(Event::End(BytesEnd::new("a"))).unwrap();

        let result = writer.into_inner().unwrap();
        assert_eq!(
            std::str::from_utf8(&result).unwrap(),
            "<a>\n  <!--c-->\n</a>"
        );
    }

    #[test]
    fn general_ref_inline() {
        let mut writer = PrettyWriter::new(Vec::new());

        writer
            .write_event(Event::Start(BytesStart::new("p")))
            .unwrap();
        writer
            .write_event(Event::Text(BytesText::new("Hello ")))
            .unwrap();
        writer
            .write_event(Event::GeneralRef(BytesRef::new("amp")))
            .unwrap();
        writer
            .write_event(Event::Text(BytesText::new(" world")))
            .unwrap();
        writer.write_event(Event::End(BytesEnd::new("p"))).unwrap();

        let result = writer.into_inner().unwrap();
        assert_eq!(
            std::str::from_utf8(&result).unwrap(),
            "<p>Hello &amp; world</p>"
        );
    }

    #[test]
    fn text_only_inline() {
        let mut writer = PrettyWriter::new(Vec::new());

        writer
            .write_event(Event::Start(BytesStart::new("a")))
            .unwrap();
        writer
            .write_event(Event::Text(BytesText::new("Test")))
            .unwrap();
        writer.write_event(Event::End(BytesEnd::new("a"))).unwrap();

        let result = writer.into_inner().unwrap();
        assert_eq!(std::str::from_utf8(&result).unwrap(), "<a>Test</a>");
    }

    #[test]
    fn nested_mixed_content() {
        let mut writer = PrettyWriter::new(Vec::new());

        writer
            .write_event(Event::Start(BytesStart::new("doc")))
            .unwrap();
        writer
            .write_event(Event::Start(BytesStart::new("p")))
            .unwrap();
        writer
            .write_event(Event::Text(BytesText::new("a ")))
            .unwrap();
        writer
            .write_event(Event::Start(BytesStart::new("b")))
            .unwrap();
        writer
            .write_event(Event::Text(BytesText::new("bold")))
            .unwrap();
        writer.write_event(Event::End(BytesEnd::new("b"))).unwrap();
        writer
            .write_event(Event::Text(BytesText::new(" c")))
            .unwrap();
        writer.write_event(Event::End(BytesEnd::new("p"))).unwrap();
        writer
            .write_event(Event::Start(BytesStart::new("p")))
            .unwrap();
        writer
            .write_event(Event::Text(BytesText::new("plain")))
            .unwrap();
        writer.write_event(Event::End(BytesEnd::new("p"))).unwrap();
        writer
            .write_event(Event::End(BytesEnd::new("doc")))
            .unwrap();

        let result = writer.into_inner().unwrap();
        // Both <p> elements have text children, so they're inline
        // doc is element-only, so indented
        assert_eq!(
            std::str::from_utf8(&result).unwrap(),
            "<doc>\n  <p>a <b>bold</b> c</p>\n  <p>plain</p>\n</doc>"
        );
    }
}

mod edge_cases {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn deeply_nested_pure_elements() {
        let mut writer = PrettyWriter::new(Vec::new());

        // Create 5 levels of nesting with no text
        for i in 0..5 {
            writer
                .write_event(Event::Start(BytesStart::new(format!("level{}", i))))
                .unwrap();
        }
        for i in (0..5).rev() {
            writer
                .write_event(Event::End(BytesEnd::new(format!("level{}", i))))
                .unwrap();
        }

        let result = writer.into_inner().unwrap();
        // The innermost empty element (level4) stays compact
        let expected = "<level0>\n  <level1>\n    <level2>\n      <level3>\n        <level4></level4>\n      </level3>\n    </level2>\n  </level1>\n</level0>";
        assert_eq!(std::str::from_utf8(&result).unwrap(), expected);
    }

    #[test]
    fn cdata_treated_like_text() {
        let mut writer = PrettyWriter::new(Vec::new());

        writer
            .write_event(Event::Start(BytesStart::new("a")))
            .unwrap();
        writer
            .write_event(Event::CData(quick_xml::events::BytesCData::new("data")))
            .unwrap();
        writer.write_event(Event::End(BytesEnd::new("a"))).unwrap();

        let result = writer.into_inner().unwrap();
        // CData is text-like, so element stays inline
        assert_eq!(
            std::str::from_utf8(&result).unwrap(),
            "<a><![CDATA[data]]></a>"
        );
    }

    #[test]
    fn multiple_text_nodes() {
        let mut writer = PrettyWriter::new(Vec::new());

        writer
            .write_event(Event::Start(BytesStart::new("a")))
            .unwrap();
        writer
            .write_event(Event::Text(BytesText::new("text1")))
            .unwrap();
        writer
            .write_event(Event::Empty(BytesStart::new("b")))
            .unwrap();
        writer
            .write_event(Event::Text(BytesText::new("text2")))
            .unwrap();
        writer.write_event(Event::End(BytesEnd::new("a"))).unwrap();

        let result = writer.into_inner().unwrap();
        assert_eq!(
            std::str::from_utf8(&result).unwrap(),
            "<a>text1<b/>text2</a>"
        );
    }

    #[test]
    fn whitespace_only_text_inline() {
        let mut writer = PrettyWriter::new(Vec::new());

        writer
            .write_event(Event::Start(BytesStart::new("a")))
            .unwrap();
        writer
            .write_event(Event::Text(BytesText::new("   ")))
            .unwrap();
        writer.write_event(Event::End(BytesEnd::new("a"))).unwrap();

        let result = writer.into_inner().unwrap();
        // Whitespace-only text is still text, so inline
        assert_eq!(std::str::from_utf8(&result).unwrap(), "<a>   </a>");
    }

    #[test]
    fn pi_in_element_indented() {
        let mut writer = PrettyWriter::new(Vec::new());

        writer
            .write_event(Event::Start(BytesStart::new("a")))
            .unwrap();
        writer
            .write_event(Event::PI(quick_xml::events::BytesPI::new("target")))
            .unwrap();
        writer.write_event(Event::End(BytesEnd::new("a"))).unwrap();

        let result = writer.into_inner().unwrap();
        // PI is block content, not text
        assert_eq!(
            std::str::from_utf8(&result).unwrap(),
            "<a>\n  <?target?>\n</a>"
        );
    }

    #[test]
    fn empty_element_only() {
        let mut writer = PrettyWriter::new(Vec::new());

        writer
            .write_event(Event::Start(BytesStart::new("root")))
            .unwrap();
        writer
            .write_event(Event::Empty(BytesStart::new("a")))
            .unwrap();
        writer
            .write_event(Event::Empty(BytesStart::new("b")))
            .unwrap();
        writer
            .write_event(Event::Empty(BytesStart::new("c")))
            .unwrap();
        writer
            .write_event(Event::End(BytesEnd::new("root")))
            .unwrap();

        let result = writer.into_inner().unwrap();
        assert_eq!(
            std::str::from_utf8(&result).unwrap(),
            "<root>\n  <a/>\n  <b/>\n  <c/>\n</root>"
        );
    }

    #[test]
    fn general_ref_only_child_inline() {
        let mut writer = PrettyWriter::new(Vec::new());

        writer
            .write_event(Event::Start(BytesStart::new("a")))
            .unwrap();
        writer
            .write_event(Event::GeneralRef(BytesRef::new("amp")))
            .unwrap();
        writer.write_event(Event::End(BytesEnd::new("a"))).unwrap();

        let result = writer.into_inner().unwrap();
        assert_eq!(std::str::from_utf8(&result).unwrap(), "<a>&amp;</a>");
    }
}

mod api {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn config_space_before_slash() {
        let mut writer = PrettyWriter::new(Vec::new());
        writer.config_mut().add_space_before_slash_in_empty_elements = true;

        writer
            .write_event(Event::Empty(BytesStart::new("x")))
            .unwrap();

        let result = writer.into_inner().unwrap();
        assert_eq!(std::str::from_utf8(&result).unwrap(), "<x />");
    }

    #[test]
    fn unclosed_element_error() {
        let mut writer = PrettyWriter::new(Vec::new());

        writer
            .write_event(Event::Start(BytesStart::new("root")))
            .unwrap();
        writer
            .write_event(Event::Start(BytesStart::new("child")))
            .unwrap();
        // Missing End for child
        writer
            .write_event(Event::End(BytesEnd::new("root")))
            .unwrap();

        let err = writer.flush();
        assert!(err.is_err());
        let err_msg = err.unwrap_err().to_string();
        assert!(err_msg.contains("Unclosed elements"));
    }

    #[test]
    fn end_without_start_error() {
        let mut writer = PrettyWriter::new(Vec::new());

        let err = writer.write_event(Event::End(BytesEnd::new("root")));
        assert!(err.is_err());
        let err_msg = err.unwrap_err().to_string();
        assert!(err_msg.contains("End tag without matching start"));
    }

    #[test]
    fn multiple_flush_calls() {
        let mut writer = PrettyWriter::new(Vec::new());

        // First document
        writer
            .write_event(Event::Start(BytesStart::new("a")))
            .unwrap();
        writer.write_event(Event::End(BytesEnd::new("a"))).unwrap();
        writer.flush().unwrap();

        // Second document
        writer
            .write_event(Event::Start(BytesStart::new("b")))
            .unwrap();
        writer.write_event(Event::End(BytesEnd::new("b"))).unwrap();
        writer.flush().unwrap();

        let result = std::str::from_utf8(writer.get_ref()).unwrap();
        // Empty elements stay compact (no semantic reason to split)
        assert_eq!(result, "<a></a><b></b>");
    }

    #[test]
    fn empty_document() {
        let writer = PrettyWriter::new(Vec::new());
        let result = writer.into_inner().unwrap();
        assert_eq!(std::str::from_utf8(&result).unwrap(), "");
    }

    #[test]
    fn with_declaration() {
        let mut writer = PrettyWriter::new(Vec::new());

        writer
            .write_event(Event::Decl(quick_xml::events::BytesDecl::new(
                "1.0", None, None,
            )))
            .unwrap();
        writer
            .write_event(Event::Start(BytesStart::new("root")))
            .unwrap();
        writer
            .write_event(Event::End(BytesEnd::new("root")))
            .unwrap();

        let result = writer.into_inner().unwrap();
        // Empty root element stays compact
        assert_eq!(
            std::str::from_utf8(&result).unwrap(),
            "<?xml version=\"1.0\"?><root></root>"
        );
    }
}

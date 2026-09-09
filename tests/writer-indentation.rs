use quick_xml::events::{BytesStart, BytesText, Event};
use quick_xml::writer::Writer;

use pretty_assertions::assert_eq;

#[test]
fn self_closed() {
    let mut buffer = Vec::new();
    let mut writer = Writer::new_with_indent(&mut buffer, b' ', 4);

    let tag = BytesStart::new("self-closed")
        .with_attributes(vec![("attr1", "value1"), ("attr2", "value2")]);
    writer
        .write_event(Event::Empty(tag))
        .expect("write tag failed");

    assert_eq!(
        std::str::from_utf8(&buffer).unwrap(),
        r#"<self-closed attr1="value1" attr2="value2"/>"#
    );
}

#[test]
fn empty_paired() {
    let mut buffer = Vec::new();
    let mut writer = Writer::new_with_indent(&mut buffer, b' ', 4);

    let start =
        BytesStart::new("paired").with_attributes(vec![("attr1", "value1"), ("attr2", "value2")]);
    let end = start.to_end();
    writer
        .write_event(Event::Start(start.clone()))
        .expect("write start tag failed");
    writer
        .write_event(Event::End(end))
        .expect("write end tag failed");

    assert_eq!(
        std::str::from_utf8(&buffer).unwrap(),
        r#"<paired attr1="value1" attr2="value2">
</paired>"#
    );
}

#[test]
fn paired_with_inner() {
    let mut buffer = Vec::new();
    let mut writer = Writer::new_with_indent(&mut buffer, b' ', 4);

    let start =
        BytesStart::new("paired").with_attributes(vec![("attr1", "value1"), ("attr2", "value2")]);
    let end = start.to_end();
    let inner = BytesStart::new("inner");

    writer
        .write_event(Event::Start(start.clone()))
        .expect("write start tag failed");
    writer
        .write_event(Event::Empty(inner))
        .expect("write inner tag failed");
    writer
        .write_event(Event::End(end))
        .expect("write end tag failed");

    assert_eq!(
        std::str::from_utf8(&buffer).unwrap(),
        r#"<paired attr1="value1" attr2="value2">
    <inner/>
</paired>"#
    );
}

#[test]
fn paired_with_text() {
    let mut buffer = Vec::new();
    let mut writer = Writer::new_with_indent(&mut buffer, b' ', 4);

    let start =
        BytesStart::new("paired").with_attributes(vec![("attr1", "value1"), ("attr2", "value2")]);
    let end = start.to_end();
    let text = BytesText::new("text");

    writer
        .write_event(Event::Start(start.clone()))
        .expect("write start tag failed");
    writer
        .write_event(Event::Text(text))
        .expect("write text failed");
    writer
        .write_event(Event::End(end))
        .expect("write end tag failed");

    assert_eq!(
        std::str::from_utf8(&buffer).unwrap(),
        r#"<paired attr1="value1" attr2="value2">text</paired>"#
    );
}

#[test]
fn mixed_content() {
    let mut buffer = Vec::new();
    let mut writer = Writer::new_with_indent(&mut buffer, b' ', 4);

    let start =
        BytesStart::new("paired").with_attributes(vec![("attr1", "value1"), ("attr2", "value2")]);
    let end = start.to_end();
    let text = BytesText::new("text");
    let inner = BytesStart::new("inner");

    writer
        .write_event(Event::Start(start.clone()))
        .expect("write start tag failed");
    writer
        .write_event(Event::Text(text))
        .expect("write text failed");
    writer
        .write_event(Event::Empty(inner))
        .expect("write inner tag failed");
    writer
        .write_event(Event::End(end))
        .expect("write end tag failed");

    assert_eq!(
        std::str::from_utf8(&buffer).unwrap(),
        r#"<paired attr1="value1" attr2="value2">text<inner/>
</paired>"#
    );
}

#[test]
fn nested() {
    let mut buffer = Vec::new();
    let mut writer = Writer::new_with_indent(&mut buffer, b' ', 4);

    let start =
        BytesStart::new("paired").with_attributes(vec![("attr1", "value1"), ("attr2", "value2")]);
    let end = start.to_end();
    let inner = BytesStart::new("inner");

    writer
        .write_event(Event::Start(start.clone()))
        .expect("write start 1 tag failed");
    writer
        .write_event(Event::Start(start.clone()))
        .expect("write start 2 tag failed");
    writer
        .write_event(Event::Empty(inner))
        .expect("write inner tag failed");
    writer
        .write_event(Event::End(end.clone()))
        .expect("write end tag 2 failed");
    writer
        .write_event(Event::End(end))
        .expect("write end tag 1 failed");

    assert_eq!(
        std::str::from_utf8(&buffer).unwrap(),
        r#"<paired attr1="value1" attr2="value2">
    <paired attr1="value1" attr2="value2">
        <inner/>
    </paired>
</paired>"#
    );
}

#[cfg(feature = "serialize")]
#[test]
fn serializable() {
    use serde::Serialize;

    #[derive(Serialize)]
    struct Foo {
        #[serde(rename = "@attribute")]
        attribute: &'static str,

        element: Bar,
        list: Vec<&'static str>,

        #[serde(rename = "$text")]
        text: &'static str,

        val: String,
    }

    #[derive(Serialize)]
    struct Bar {
        baz: usize,
        bat: usize,
    }

    let mut buffer = Vec::new();
    let mut writer = Writer::new_with_indent(&mut buffer, b' ', 4);

    let content = Foo {
        attribute: "attribute",
        element: Bar { baz: 42, bat: 43 },
        list: vec!["first element", "second element"],
        text: "text",
        val: "foo".to_owned(),
    };

    let start =
        BytesStart::new("paired").with_attributes(vec![("attr1", "value1"), ("attr2", "value2")]);
    let end = start.to_end();

    writer
        .write_event(Event::Start(start.clone()))
        .expect("write start tag failed");
    writer
        .write_serializable("foo_element", &content)
        .expect("write serializable inner contents failed");
    writer
        .write_event(Event::End(end))
        .expect("write end tag failed");

    assert_eq!(
        std::str::from_utf8(&buffer).unwrap(),
        r#"<paired attr1="value1" attr2="value2">
    <foo_element attribute="attribute">
        <element>
            <baz>42</baz>
            <bat>43</bat>
        </element>
        <list>first element</list>
        <list>second element</list>text<val>foo</val>
    </foo_element>
</paired>"#
    );
}

#[test]
fn element_writer_empty() {
    let mut buffer = Vec::new();
    let mut writer = Writer::new_with_indent(&mut buffer, b' ', 4);

    writer
        .create_element("empty")
        .with_attribute(("attr1", "value1"))
        .with_attribute(("attr2", "value2"))
        .write_empty()
        .expect("failure");

    assert_eq!(
        std::str::from_utf8(&buffer).unwrap(),
        r#"<empty attr1="value1" attr2="value2"/>"#
    );
}

#[test]
fn element_writer_text() {
    let mut buffer = Vec::new();
    let mut writer = Writer::new_with_indent(&mut buffer, b' ', 4);

    writer
        .create_element("paired")
        .with_attribute(("attr1", "value1"))
        .with_attribute(("attr2", "value2"))
        .write_text_content(BytesText::new("text"))
        .expect("failure");

    assert_eq!(
        std::str::from_utf8(&buffer).unwrap(),
        r#"<paired attr1="value1" attr2="value2">text</paired>"#
    );
}

#[test]
fn element_writer_nested() {
    let mut buffer = Vec::new();
    let mut writer = Writer::new_with_indent(&mut buffer, b' ', 4);

    writer
        .create_element("outer")
        .with_attribute(("attr1", "value1"))
        .with_attribute(("attr2", "value2"))
        .write_inner_content(|writer| {
            let fruits = ["apple", "orange", "banana"];
            for (quant, item) in fruits.iter().enumerate() {
                writer
                    .create_element("fruit")
                    .with_attribute(("quantity", quant.to_string().as_str()))
                    .write_text_content(BytesText::new(item))?;
            }
            writer
                .create_element("inner")
                .write_inner_content(|writer| {
                    writer.create_element("empty").write_empty().map(|_| ())
                })?;

            Ok(())
        })
        .expect("failure");

    assert_eq!(
        std::str::from_utf8(&buffer).unwrap(),
        r#"<outer attr1="value1" attr2="value2">
    <fruit quantity="0">apple</fruit>
    <fruit quantity="1">orange</fruit>
    <fruit quantity="2">banana</fruit>
    <inner>
        <empty/>
    </inner>
</outer>"#
    );
}

mod in_attributes {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn newline_first() {
        let mut buffer = Vec::new();
        let mut writer = Writer::new_with_indent(&mut buffer, b'_', 1);

        writer
            .create_element("element")
            .new_line()
            .with_attribute(("first", "1"))
            .with_attribute(("second", "2"))
            .new_line()
            .with_attribute(("third", "3"))
            .with_attribute(("fourth", "4"))
            .write_empty()
            .expect("write tag failed");

        assert_eq!(
            std::str::from_utf8(&buffer).unwrap(),
            "<element\
                \n_first=\"1\" second=\"2\"\
                \n_third=\"3\" fourth=\"4\"/>"
        );
    }

    #[test]
    fn newline_inside() {
        let mut buffer = Vec::new();
        let mut writer = Writer::new_with_indent(&mut buffer, b'_', 1);

        writer
            .create_element("element")
            .with_attribute(("first", "1"))
            .with_attribute(("second", "2"))
            .new_line()
            .with_attribute(("third", "3"))
            .with_attribute(("fourth", "4"))
            .write_empty()
            .expect("write tag failed");

        assert_eq!(
            std::str::from_utf8(&buffer).unwrap(),
            "<element first=\"1\" second=\"2\"\
            \n         third=\"3\" fourth=\"4\"/>"
        );
    }

    #[test]
    fn newline_last() {
        let mut buffer = Vec::new();
        let mut writer = Writer::new_with_indent(&mut buffer, b'_', 1);

        writer
            .create_element("element")
            .new_line()
            .with_attribute(("first", "1"))
            .with_attribute(("second", "2"))
            .new_line()
            .with_attribute(("third", "3"))
            .with_attribute(("fourth", "4"))
            .new_line()
            .write_empty()
            .expect("write tag failed");

        writer
            .create_element("element")
            .with_attribute(("first", "1"))
            .with_attribute(("second", "2"))
            .new_line()
            .with_attribute(("third", "3"))
            .with_attribute(("fourth", "4"))
            .new_line()
            .write_empty()
            .expect("write tag failed");

        assert_eq!(
            std::str::from_utf8(&buffer).unwrap(),
            "<element\
                \n_first=\"1\" second=\"2\"\
                \n_third=\"3\" fourth=\"4\"\
            \n/>\
            \n<element first=\"1\" second=\"2\"\
            \n         third=\"3\" fourth=\"4\"\
            \n/>"
        );
    }

    #[test]
    fn newline_twice() {
        let mut buffer = Vec::new();
        let mut writer = Writer::new_with_indent(&mut buffer, b'_', 1);

        writer
            .create_element("element")
            .new_line()
            .new_line()
            .write_empty()
            .expect("write tag failed");

        writer
            .create_element("element")
            .with_attribute(("first", "1"))
            .new_line()
            .new_line()
            .with_attribute(("second", "2"))
            .write_empty()
            .expect("write tag failed");

        assert_eq!(
            std::str::from_utf8(&buffer).unwrap(),
            r#"<element

/>
<element first="1"

         second="2"/>"#
        );
    }

    #[test]
    fn without_indent() {
        let mut buffer = Vec::new();
        let mut writer = Writer::new(&mut buffer);

        writer
            .create_element("element")
            .new_line()
            .new_line()
            .write_empty()
            .expect("write tag failed");

        writer
            .create_element("element")
            .with_attribute(("first", "1"))
            .new_line()
            .new_line()
            .with_attribute(("second", "2"))
            .write_empty()
            .expect("write tag failed");

        assert_eq!(
            std::str::from_utf8(&buffer).unwrap(),
            r#"<element/><element first="1" second="2"/>"#
        );
    }

    #[test]
    fn long_element_name() {
        let mut buffer = Vec::new();
        let mut writer = Writer::new_with_indent(&mut buffer, b't', 1);

        writer
            .create_element(String::from("x").repeat(128).as_str())
            .with_attribute(("first", "1"))
            .new_line()
            .with_attribute(("second", "2"))
            .write_empty()
            .expect("Problem with indentation reference");
    }
}

mod in_attributes_multi {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn newline_first() {
        let mut buffer = Vec::new();
        let mut writer = Writer::new_with_indent(&mut buffer, b'_', 1);

        writer
            .create_element("element")
            .new_line()
            .with_attributes([("first", "1"), ("second", "2")])
            .new_line()
            .with_attributes([("third", "3"), ("fourth", "4")])
            .write_empty()
            .expect("write tag failed");

        assert_eq!(
            std::str::from_utf8(&buffer).unwrap(),
            "<element\
                \n_first=\"1\" second=\"2\"\
                \n_third=\"3\" fourth=\"4\"/>"
        );
    }

    #[test]
    fn newline_inside() {
        let mut buffer = Vec::new();
        let mut writer = Writer::new_with_indent(&mut buffer, b'_', 1);

        writer
            .create_element("element")
            .with_attributes([("first", "1"), ("second", "2")])
            .new_line()
            .with_attributes([("third", "3"), ("fourth", "4")])
            .write_empty()
            .expect("write tag failed");

        assert_eq!(
            std::str::from_utf8(&buffer).unwrap(),
            r#"<element first="1" second="2"
         third="3" fourth="4"/>"#
        );
    }

    #[test]
    fn newline_last() {
        let mut buffer = Vec::new();
        let mut writer = Writer::new_with_indent(&mut buffer, b'_', 1);

        writer
            .create_element("element")
            .new_line()
            .with_attributes([("first", "1"), ("second", "2")])
            .new_line()
            .with_attributes([("third", "3"), ("fourth", "4")])
            .new_line()
            .write_empty()
            .expect("write tag failed");

        writer
            .create_element("element")
            .with_attributes([("first", "1"), ("second", "2")])
            .new_line()
            .with_attributes([("third", "3"), ("fourth", "4")])
            .new_line()
            .write_empty()
            .expect("write tag failed");

        assert_eq!(
            std::str::from_utf8(&buffer).unwrap(),
            "<element\
                \n_first=\"1\" second=\"2\"\
                \n_third=\"3\" fourth=\"4\"\
            \n/>\
            \n<element first=\"1\" second=\"2\"\
            \n         third=\"3\" fourth=\"4\"\
            \n/>"
        );
    }
}

/// `Writer::new_with_indent` converts the indent byte with `char::from`, so bytes
/// above 0x7F select a character that is two bytes long in UTF-8.
mod multi_byte_indent_char {
    use super::*;
    use pretty_assertions::assert_eq;

    /// NO-BREAK SPACE is `char::from(0xA0)` and occupies two bytes in UTF-8.
    const NBSP: char = '\u{a0}';

    #[test]
    fn nested() {
        let mut buffer = Vec::new();
        let mut writer = Writer::new_with_indent(&mut buffer, 0xA0, 2);

        let start = BytesStart::new("paired");
        let end = start.to_end().into_owned();

        writer
            .write_event(Event::Start(start.clone()))
            .expect("write start 1 tag failed");
        writer
            .write_event(Event::Start(start))
            .expect("write start 2 tag failed");
        writer
            .write_event(Event::Empty(BytesStart::new("inner")))
            .expect("write inner tag failed");
        writer
            .write_event(Event::End(end.clone()))
            .expect("write end tag 2 failed");
        writer
            .write_event(Event::End(end))
            .expect("write end tag 1 failed");

        let i = NBSP.to_string().repeat(2);
        assert_eq!(
            std::str::from_utf8(&buffer).unwrap(),
            format!(
                "<paired>\n{i}<paired>\n{i}{i}<inner/>\n{i}</paired>\n</paired>",
                i = i
            )
        );
    }

    #[test]
    fn in_attributes() {
        let mut buffer = Vec::new();
        let mut writer = Writer::new_with_indent(&mut buffer, 0xA0, 2);

        writer
            .create_element("element")
            .new_line()
            .with_attribute(("first", "1"))
            .write_empty()
            .expect("write tag failed");

        assert_eq!(
            std::str::from_utf8(&buffer).unwrap(),
            format!("<element\n{i}first=\"1\"/>", i = NBSP.to_string().repeat(2))
        );
    }

    /// An odd `indent_size` makes the byte length of one level an odd number,
    /// which used to land in the middle of a two byte character and panic.
    #[test]
    fn odd_indent_size() {
        let mut buffer = Vec::new();
        let mut writer = Writer::new_with_indent(&mut buffer, 0xA0, 1);

        let start = BytesStart::new("paired");
        writer
            .write_event(Event::Start(start.clone()))
            .expect("write start tag failed");
        writer
            .write_event(Event::Empty(BytesStart::new("inner")))
            .expect("write inner tag failed");
        writer
            .write_event(Event::End(start.to_end()))
            .expect("write end tag failed");

        assert_eq!(
            std::str::from_utf8(&buffer).unwrap(),
            format!("<paired>\n{i}<inner/>\n</paired>", i = NBSP)
        );
    }

    /// Attribute indent is added on top of the current element indent, so the
    /// conversion must apply to the added level only, not to the sum.
    #[test]
    fn in_attributes_nested() {
        let mut buffer = Vec::new();
        let mut writer = Writer::new_with_indent(&mut buffer, 0xA0, 2);

        let outer = BytesStart::new("outer");
        writer
            .write_event(Event::Start(outer.clone()))
            .expect("write outer tag failed");
        writer
            .create_element("element")
            .new_line()
            .with_attribute(("first", "1"))
            .write_empty()
            .expect("write tag failed");
        writer
            .write_event(Event::End(outer.to_end()))
            .expect("write end tag failed");

        let i = NBSP.to_string();
        assert_eq!(
            std::str::from_utf8(&buffer).unwrap(),
            format!(
                "<outer>\n{i2}<element\n{i4}first=\"1\"/>\n</outer>",
                i2 = i.repeat(2),
                i4 = i.repeat(4)
            )
        );
    }

    /// ASCII indents must keep working exactly as before.
    #[test]
    fn ascii_is_unchanged() {
        let mut buffer = Vec::new();
        let mut writer = Writer::new_with_indent(&mut buffer, b' ', 2);

        let start = BytesStart::new("paired");
        writer
            .write_event(Event::Start(start.clone()))
            .expect("write start tag failed");
        writer
            .write_event(Event::Empty(BytesStart::new("inner")))
            .expect("write inner tag failed");
        writer
            .write_event(Event::End(start.to_end()))
            .expect("write end tag failed");

        assert_eq!(
            std::str::from_utf8(&buffer).unwrap(),
            "<paired>\n  <inner/>\n</paired>"
        );
    }

    /// `Serializer::indent` takes a `char`, so it must not be truncated to a byte.
    #[cfg(feature = "serialize")]
    #[test]
    fn serializer_indent_char_is_not_truncated() {
        use quick_xml::se::Serializer;
        use serde::Serialize;

        #[derive(Serialize)]
        struct Response {
            message: &'static str,
        }

        // IDEOGRAPHIC SPACE: three bytes in UTF-8, `as u8` truncates it to NUL.
        let mut output = String::new();
        let mut serializer = Serializer::with_root(&mut output, Some("response")).unwrap();
        serializer.indent('\u{3000}', 2);
        Response { message: "Success" }
            .serialize(serializer)
            .unwrap();

        assert_eq!(
            output,
            "<response>\n\u{3000}\u{3000}<message>Success</message>\n</response>"
        );
    }
}

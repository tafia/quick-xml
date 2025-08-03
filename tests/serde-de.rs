use quick_xml::de::Deserializer;
use quick_xml::utils::{ByteBuf, Bytes};
use quick_xml::DeError;

use pretty_assertions::assert_eq;

use serde::de::IgnoredAny;
use serde::Deserialize;

mod serde_helpers;
use serde_helpers::from_str;

/// Tests for deserializing into specially named field `$text` which represent
/// textual content of an XML element
mod text {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Test for https://github.com/tafia/quick-xml/issues/231
    #[test]
    fn implicit() {
        use serde_value::Value;

        let item: Value = from_str(r#"<root>content</root>"#).unwrap();

        assert_eq!(
            item,
            Value::Map(
                vec![(
                    Value::String("$text".into()),
                    Value::String("content".into())
                )]
                .into_iter()
                .collect()
            )
        );
    }

    #[test]
    fn explicit() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Item {
            #[serde(rename = "$text")]
            content: String,
        }

        let item: Item = from_str(r#"<root>content</root>"#).unwrap();

        assert_eq!(
            item,
            Item {
                content: "content".into()
            }
        );
    }

    #[test]
    fn without() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Item;

        let _: Item = from_str(r#"<root>content</root>"#).unwrap();
    }
}

/// Tests calling `deserialize_ignored_any`
#[test]
fn ignored_any() {
    let err = from_str::<IgnoredAny>("");
    match err {
        Err(DeError::UnexpectedEof) => {}
        x => panic!("Expected `Err(UnexpectedEof)`, but got `{:?}`", x),
    }

    from_str::<IgnoredAny>(r#"<empty/>"#).unwrap();
    from_str::<IgnoredAny>(r#"<with-attributes key="value"/>"#).unwrap();
    from_str::<IgnoredAny>(r#"<nested>text</nested>"#).unwrap();
    from_str::<IgnoredAny>(r#"<nested><![CDATA[cdata]]></nested>"#).unwrap();
    from_str::<IgnoredAny>(r#"<nested><nested/></nested>"#).unwrap();
}

/// Tests for trivial XML documents: empty or contains only primitive type
/// on a top level; all of them should be considered invalid
mod trivial {
    use super::*;

    #[rustfmt::skip] // excess spaces used for readability
    macro_rules! eof {
        ($name:ident: $type:ty = $value:expr) => {
            #[test]
            fn $name() {
                match from_str::<$type>($value) {
                    Err(DeError::UnexpectedEof) => {}
                    x => panic!(
                        r#"Expected `Err(UnexpectedEof)`, but got `{:?}`"#,
                        x
                    ),
                }
            }
        };
        ($value:expr) => {
            eof!(i8_:    i8    = $value);
            eof!(i16_:   i16   = $value);
            eof!(i32_:   i32   = $value);
            eof!(i64_:   i64   = $value);
            eof!(isize_: isize = $value);

            eof!(u8_:    u8    = $value);
            eof!(u16_:   u16   = $value);
            eof!(u32_:   u32   = $value);
            eof!(u64_:   u64   = $value);
            eof!(usize_: usize = $value);

            eof!(u128_: u128 = $value);
            eof!(i128_: i128 = $value);

            eof!(f32_: f32 = $value);
            eof!(f64_: f64 = $value);

            eof!(false_: bool = $value);
            eof!(true_: bool = $value);
            eof!(char_: char = $value);

            eof!(string: String = $value);

            /// XML does not able to store binary data
            #[test]
            fn byte_buf() {
                match from_str::<ByteBuf>($value) {
                    Err(DeError::UnexpectedEof) => {}
                    x => panic!(
                        r#"Expected `Err(UnexpectedEof)`, but got `{:?}`"#,
                        x
                    ),
                }
            }

            /// XML does not able to store binary data
            #[test]
            fn bytes() {
                match from_str::<Bytes>($value) {
                    Err(DeError::UnexpectedEof) => {}
                    x => panic!(
                        r#"Expected `Err(UnexpectedEof)`, but got `{:?}`"#,
                        x
                    ),
                }
            }

            #[test]
            fn unit() {
                match from_str::<()>($value) {
                    Err(DeError::UnexpectedEof) => {}
                    x => panic!(
                        r#"Expected `Err(UnexpectedEof)`, but got `{:?}`"#,
                        x
                    ),
                }
            }
        };
    }

    /// Empty document should considered invalid no matter what type we try to deserialize
    mod empty_doc {
        use super::*;

        eof!("");
    }

    /// Document that contains only comment should be handled as if it is empty
    mod only_comment {
        use super::*;

        eof!("<!--comment-->");
    }

    macro_rules! in_struct {
        // string, char and bool have some specifics in some tests
        (string: $type:ty = $value:expr, $expected:literal) => {
            in_struct!(
                string: $type = $value, $expected.into(), concat!(" \n\t", $expected, " \n\t").into();

                /// Try to deserialize type from a root tag without data or with only spaces
                #[test]
                fn wrapped_empty() {
                    let item: $type = from_str("<root/>").unwrap();
                    let expected: $type = "".into();
                    assert_eq!(item, expected);

                    let item: $type = from_str("<root> \r\n\t</root>").unwrap();
                    // \r\n normalized to \n
                    let expected: $type = " \n\t".into();
                    assert_eq!(item, expected);
                }

                #[test]
                fn field_empty() {
                    let item: Field<$type> = from_str("<root><value/></root>").unwrap();
                    assert_eq!(item, Field { value: "".into() });

                    let item: Field<$type> = from_str("<root><value> \r\n\t</value></root>").unwrap();
                    // \r\n normalized to \n
                    assert_eq!(item, Field { value: " \n\t".into() });
                }
            );
        };
        (char_: $type:ty = $value:expr, $expected:expr) => {
            in_struct!(
                char_: $type = $value, $expected, $expected;

                /// Try to deserialize type from a root tag without data or with only spaces
                #[test]
                fn wrapped_empty() {
                    match from_str::<$type>("<root/>") {
                        Err(DeError::Custom(msg)) => assert_eq!(msg, r#"invalid value: string "", expected a character"#),
                        x => panic!("Expected `Err(Custom(_))`, but got `{:?}`", x),
                    }

                    match from_str::<$type>("<root> \r\n\t</root>") {
                        // \r\n normalized to \n
                        Err(DeError::Custom(msg)) => assert_eq!(msg, r#"invalid value: string " \n\t", expected a character"#),
                        x => panic!("Expected `Err(Custom(_))`, but got `{:?}`", x),
                    }
                }

                #[test]
                fn field_empty() {
                    match from_str::<Field<$type>>("<root><value/></root>") {
                        Err(DeError::Custom(msg)) => assert_eq!(msg, r#"invalid value: string "", expected a character"#),
                        x => panic!("Expected `Err(Custom(_))`, but got `{:?}`", x),
                    }

                    match from_str::<Field<$type>>("<root><value> \r\n\t</value></root>") {
                        // \r\n normalized to \n
                        Err(DeError::Custom(msg)) => assert_eq!(msg, r#"invalid value: string " \n\t", expected a character"#),
                        x => panic!("Expected `Err(Custom(_))`, but got `{:?}`", x),
                    }
                }
            );
        };
        ($name:ident: bool = $value:expr, $expected:expr) => {
            in_struct!(
                $name: bool = $value, $expected, $expected;

                /// Try to deserialize type from a root tag without data or with only spaces
                #[test]
                fn wrapped_empty() {
                    match from_str::<bool>("<root/>") {
                        Err(DeError::Custom(msg)) => assert_eq!(msg, r#"invalid type: string "", expected a boolean"#),
                        x => panic!("Expected `Err(Custom(_))`, but got `{:?}`", x),
                    }

                    match from_str::<bool>("<root> \r\n\t</root>") {
                        // \r\n normalized to \n
                        Err(DeError::Custom(msg)) => assert_eq!(msg, r#"invalid type: string " \n\t", expected a boolean"#),
                        x => panic!("Expected `Err(Custom(_))`, but got `{:?}`", x),
                    }
                }

                #[test]
                fn field_empty() {
                    match from_str::<Field<bool>>("<root><value/></root>") {
                        Err(DeError::Custom(msg)) => assert_eq!(msg, r#"invalid type: string "", expected a boolean"#),
                        x => panic!("Expected `Err(Custom(_))`, but got `{:?}`", x),
                    }

                    match from_str::<Field<bool>>("<root><value> \r\n\t</value></root>") {
                        // \r\n normalized to \n
                        Err(DeError::Custom(msg)) => assert_eq!(msg, r#"invalid type: string " \n\t", expected a boolean"#),
                        x => panic!("Expected `Err(Custom(_))`, but got `{:?}`", x),
                    }
                }
            );
        };
        ($name:ident: $type:ty = $value:expr, $expected:expr) => {
            in_struct!(
                $name: $type = $value, $expected, $expected;

                /// Try to deserialize type from a root tag without data or with only spaces
                #[test]
                fn wrapped_empty() {
                    match from_str::<$type>("<root/>") {
                        Err(DeError::Custom(msg)) => assert_eq!(msg, concat!(r#"invalid type: string "", expected "#, stringify!($type))),
                        x => panic!("Expected `Err(Custom(_))`, but got `{:?}`", x),
                    }

                    match from_str::<$type>("<root> \r\n\t</root>") {
                        // \r\n normalized to \n
                        Err(DeError::Custom(msg)) => assert_eq!(msg, concat!(r#"invalid type: string " \n\t", expected "#, stringify!($type))),
                        x => panic!("Expected `Err(Custom(_))`, but got `{:?}`", x),
                    }
                }

                #[test]
                fn field_empty() {
                    match from_str::<Field<$type>>("<root><value/></root>") {
                        Err(DeError::Custom(msg)) => assert_eq!(msg, concat!(r#"invalid type: string "", expected "#, stringify!($type))),
                        x => panic!("Expected `Err(Custom(_))`, but got `{:?}`", x),
                    }

                    match from_str::<Field<$type>>("<root><value> \r\n\t</value></root>") {
                        // \r\n normalized to \n
                        Err(DeError::Custom(msg)) => assert_eq!(msg, concat!(r#"invalid type: string " \n\t", expected "#, stringify!($type))),
                        x => panic!("Expected `Err(Custom(_))`, but got `{:?}`", x),
                    }
                }
            );
        };
        ($name:ident: $type:ty = $value:expr, $expected:expr, $expected_with_indent:expr; $($specific_tests:item)*) => {
            mod $name {
                use super::*;
                use pretty_assertions::assert_eq;

                $($specific_tests)*

                /// Try to deserialize type wrapped in a tag, optionally surround with spaces
                #[test]
                fn wrapped() {
                    let item: $type = from_str(&format!("<root>{}</root>", $value)).unwrap();
                    let expected: $type = $expected;
                    assert_eq!(item, expected);

                    let item: $type =
                        from_str(&format!("<root> \r\n\t{} \r\n\t</root>", $value)).unwrap();
                    let expected: $type = $expected_with_indent;
                    assert_eq!(item, expected);
                }

                /// Try to deserialize type wrapped in two tags
                #[test]
                fn nested() {
                    match from_str::<$type>(&format!("<root><nested>{}</nested></root>", $value)) {
                        // Expected unexpected start element `<nested>`
                        Err(DeError::UnexpectedStart(tag)) => assert_eq!(tag, b"nested"),
                        x => panic!(
                            r#"Expected `Err(UnexpectedStart("nested"))`, but got `{:?}`"#,
                            x
                        ),
                    }

                    match from_str::<$type>(&format!(
                        "<root> \r\n\t<nested>{}</nested> \r\n\t</root>",
                        $value
                    )) {
                        // Expected unexpected start element `<nested>`
                        Err(DeError::UnexpectedStart(tag)) => assert_eq!(tag, b"nested"),
                        x => panic!(
                            r#"Expected `Err(UnexpectedStart("nested"))`, but got `{:?}`"#,
                            x
                        ),
                    }
                }

                /// Try to deserialize type wrapped in a tag with another tag after content
                #[test]
                fn tag_after() {
                    match from_str::<$type>(&format!("<root>{}<something-else/></root>", $value)) {
                        // Expected unexpected start element `<something-else>`
                        Err(DeError::UnexpectedStart(tag)) => assert_eq!(tag, b"something-else"),
                        x => panic!(
                            r#"Expected `Err(UnexpectedStart("something-else"))`, but got `{:?}`"#,
                            x
                        ),
                    }

                    match from_str::<$type>(&format!(
                        "<root> \r\n\t{} \r\n\t<something-else/></root>",
                        $value
                    )) {
                        // Expected unexpected start element `<something-else>`
                        Err(DeError::UnexpectedStart(tag)) => assert_eq!(tag, b"something-else"),
                        x => panic!(
                            r#"Expected `Err(UnexpectedStart("something-else"))`, but got `{:?}`"#,
                            x
                        ),
                    }
                }

                /// Try to deserialize type wrapped in a tag with another tag before content
                #[test]
                fn tag_before() {
                    match from_str::<$type>(&format!("<root><something-else/>{}</root>", $value)) {
                        // Expected unexpected start element `<something-else>`
                        Err(DeError::UnexpectedStart(tag)) => assert_eq!(tag, b"something-else"),
                        x => panic!(
                            r#"Expected `Err(UnexpectedStart("something-else"))`, but got `{:?}`"#,
                            x
                        ),
                    }

                    match from_str::<$type>(&format!(
                        "<root><something-else/> \r\n\t{} \r\n\t</root>",
                        $value
                    )) {
                        // Expected unexpected start element `<something-else>`
                        Err(DeError::UnexpectedStart(tag)) => assert_eq!(tag, b"something-else"),
                        x => panic!(
                            r#"Expected `Err(UnexpectedStart("something-else"))`, but got `{:?}`"#,
                            x
                        ),
                    }
                }

                /// Try to deserialize type which is a field of a struct
                #[test]
                fn field() {
                    let item: Field<$type> =
                        from_str(&format!("<root><value>{}</value></root>", $value)).unwrap();
                    assert_eq!(item, Field { value: $expected });

                    let item: Field<$type> =
                        from_str(&format!("<root><value> \r\n\t{} \r\n\t</value></root>", $value)).unwrap();
                    assert_eq!(item, Field { value: $expected_with_indent });
                }

                #[test]
                fn field_nested() {
                    match from_str::<Field<$type>>(&format!(
                        "<root><value><nested>{}</nested></value></root>",
                        $value
                    )) {
                        // Expected unexpected start element `<nested>`
                        Err(DeError::UnexpectedStart(tag)) => assert_eq!(tag, b"nested"),
                        x => panic!(
                            r#"Expected `Err(UnexpectedStart("nested"))`, but got `{:?}`"#,
                            x
                        ),
                    }

                    match from_str::<Field<$type>>(&format!(
                        "<root><value> \r\n\t<nested>{}</nested> \r\n\t</value></root>",
                        $value
                    )) {
                        // Expected unexpected start element `<nested>`
                        Err(DeError::UnexpectedStart(tag)) => assert_eq!(tag, b"nested"),
                        x => panic!(
                            r#"Expected `Err(UnexpectedStart("nested"))`, but got `{:?}`"#,
                            x
                        ),
                    }
                }

                #[test]
                fn field_tag_after() {
                    match from_str::<Field<$type>>(&format!(
                        "<root><value>{}<something-else/></value></root>",
                        $value
                    )) {
                        // Expected unexpected start element `<something-else>`
                        Err(DeError::UnexpectedStart(tag)) => assert_eq!(tag, b"something-else"),
                        x => panic!(
                            r#"Expected `Err(UnexpectedStart("something-else"))`, but got `{:?}`"#,
                            x
                        ),
                    }

                    match from_str::<Field<$type>>(&format!(
                        "<root><value> \r\n\t{} \r\n\t<something-else/></value></root>",
                        $value
                    )) {
                        // Expected unexpected start element `<something-else>`
                        Err(DeError::UnexpectedStart(tag)) => assert_eq!(tag, b"something-else"),
                        x => panic!(
                            r#"Expected `Err(UnexpectedStart("something-else"))`, but got `{:?}`"#,
                            x
                        ),
                    }
                }

                #[test]
                fn field_tag_before() {
                    match from_str::<Field<$type>>(&format!(
                        "<root><value><something-else/>{}</value></root>",
                        $value
                    )) {
                        // Expected unexpected start element `<something-else>`
                        Err(DeError::UnexpectedStart(tag)) => assert_eq!(tag, b"something-else"),
                        x => panic!(
                            r#"Expected `Err(UnexpectedStart("something-else"))`, but got `{:?}`"#,
                            x
                        ),
                    }

                    match from_str::<Field<$type>>(&format!(
                        "<root><value><something-else/> \r\n\t{} \r\n\t</value></root>",
                        $value
                    )) {
                        // Expected unexpected start element `<something-else>`
                        Err(DeError::UnexpectedStart(tag)) => assert_eq!(tag, b"something-else"),
                        x => panic!(
                            r#"Expected `Err(UnexpectedStart("something-else"))`, but got `{:?}`"#,
                            x
                        ),
                    }
                }

                #[test]
                fn text_empty() {
                    match from_str::<Trivial<$type>>("<root/>") {
                        Err(DeError::Custom(msg)) => assert_eq!(msg, "missing field `$text`"),
                        x => panic!("Expected `Err(Custom(_))`, but got `{:?}`", x),
                    }

                    match from_str::<Trivial<$type>>("<root> \r\n\t</root>") {
                        Err(DeError::Custom(msg)) => assert_eq!(msg, "missing field `$text`"),
                        x => panic!("Expected `Err(Custom(_))`, but got `{:?}`", x),
                    }
                }

                /// Tests deserialization from top-level tag content: `<root>...content...</root>`
                #[test]
                fn text() {
                    let item: Trivial<$type> =
                        from_str(&format!("<root>{}</root>", $value)).unwrap();
                    assert_eq!(item, Trivial { value: $expected });

                    let item: Trivial<$type> =
                        from_str(&format!("<root> \r\n\t{} \r\n\t</root>", $value)).unwrap();
                    assert_eq!(item, Trivial { value: $expected_with_indent });
                }

                #[test]
                fn text_nested() {
                    match from_str::<Trivial<$type>>(&format!(
                        "<root><nested>{}</nested></root>",
                        $value
                    )) {
                        // Expected unexpected start element `<nested>`
                        Err(DeError::Custom(reason)) => assert_eq!(reason, "missing field `$text`"),
                        x => panic!(
                            r#"Expected `Err(Custom("missing field `$text`"))`, but got `{:?}`"#,
                            x
                        ),
                    }

                    match from_str::<Trivial<$type>>(&format!(
                        "<root> \r\n\t<nested>{}</nested> \r\n\t</root>",
                        $value
                    )) {
                        // Expected unexpected start element `<nested>`
                        Err(DeError::Custom(reason)) => assert_eq!(reason, "missing field `$text`"),
                        x => panic!(
                            r#"Expected `Err(Custom("missing field `$text`"))`, but got `{:?}`"#,
                            x
                        ),
                    }
                }

                #[test]
                fn text_tag_after() {
                    // Unlike `wrapped` test, here we have a struct that is serialized to XML with
                    // an implicit field `$text` and some other field "something-else" which not interested
                    // for us in the Trivial structure. If you want the same behavior as for naked primitive,
                    // use `$value` field which would consume all data, unless a dedicated field would present
                    let item: Trivial<$type> =
                        from_str(&format!("<root>{}<something-else/></root>", $value)).unwrap();
                    assert_eq!(item, Trivial { value: $expected });

                    let item: Trivial<$type> =
                        from_str(&format!("<root> \r\n\t{} \r\n\t<something-else/></root>", $value)).unwrap();
                    assert_eq!(item, Trivial { value: $expected_with_indent });
                }

                #[test]
                fn text_tag_before() {
                    let item: Trivial<$type> =
                        from_str(&format!("<root><something-else/>{}</root>", $value)).unwrap();
                    assert_eq!(item, Trivial { value: $expected });

                    let item: Trivial<$type> =
                        from_str(&format!("<root><something-else/> \r\n\t{} \r\n\t</root>", $value)).unwrap();
                    assert_eq!(item, Trivial { value: $expected_with_indent });
                }
            }
        };
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Field<T> {
        value: T,
    }

    /// Well-formed XML must have a single tag at the root level.
    /// Any XML tag can be modeled as a struct, and content of this tag are modeled as
    /// fields of this struct.
    ///
    /// Because we want to get access to unnamed content of the tag (usually, this internal
    /// XML node called `$text`) we use a rename to a special name `$text`
    #[derive(Debug, Deserialize, PartialEq)]
    struct Trivial<T> {
        #[serde(rename = "$text")]
        value: T,
    }

    /// Tests deserialization from text content in a tag
    #[rustfmt::skip] // tests formatted in a table
    mod text {
        use super::*;
        use pretty_assertions::assert_eq;

        in_struct!(i8_:    i8    = "-42", -42i8);
        in_struct!(i16_:   i16   = "-4200", -4200i16);
        in_struct!(i32_:   i32   = "-42000000", -42000000i32);
        in_struct!(i64_:   i64   = "-42000000000000", -42000000000000i64);
        in_struct!(isize_: isize = "-42000000", -42000000isize);

        in_struct!(u8_:    u8    = "42", 42u8);
        in_struct!(u16_:   u16   = "4200", 4200u16);
        in_struct!(u32_:   u32   = "42000000", 42000000u32);
        in_struct!(u64_:   u64   = "42000000000000", 42000000000000u64);
        in_struct!(usize_: usize = "42000000", 42000000usize);

        in_struct!(u128_: u128 = "420000000000000000000000000000", 420000000000000000000000000000u128);
        in_struct!(i128_: i128 = "-420000000000000000000000000000", -420000000000000000000000000000i128);

        in_struct!(f32_: f32 = "4.2", 4.2f32);
        in_struct!(f64_: f64 = "4.2", 4.2f64);

        in_struct!(false_: bool = "false", false);
        in_struct!(true_: bool = "true", true);
        in_struct!(char_: char = "r", 'r');

        in_struct!(string: String = "escaped&#x20;string", "escaped string");

        /// XML does not able to store binary data
        #[test]
        fn byte_buf() {
            match from_str::<Trivial<ByteBuf>>("<root>escaped&#x20;byte_buf</root>") {
                Err(DeError::Custom(msg)) => {
                    assert_eq!(msg, "invalid type: string \"escaped byte_buf\", expected byte data")
                }
                x => panic!(
                    r#"Expected `Err(Custom("invalid type: string \"escaped byte_buf\", expected byte data"))`, but got `{:?}`"#,
                    x
                ),
            }
        }

        /// XML does not able to store binary data
        #[test]
        fn bytes() {
            match from_str::<Trivial<Bytes>>("<root>escaped&#x20;byte_buf</root>") {
                Err(DeError::Custom(msg)) => {
                    assert_eq!(msg, "invalid type: string \"escaped byte_buf\", expected borrowed bytes")
                }
                x => panic!(
                    r#"Expected `Err(Custom("invalid type: string \"escaped byte_buf\", expected borrowed bytes"))`, but got `{:?}`"#,
                    x
                ),
            }
        }
    }

    /// Tests deserialization from CDATA content in a tag.
    /// CDATA handling similar to text handling except that strings does not unescapes
    #[rustfmt::skip] // tests formatted in a table
    mod cdata {
        use super::*;
        use pretty_assertions::assert_eq;

        in_struct!(i8_:    i8    = "<![CDATA[-42]]>", -42i8);
        in_struct!(i16_:   i16   = "<![CDATA[-4200]]>", -4200i16);
        in_struct!(i32_:   i32   = "<![CDATA[-42000000]]>", -42000000i32);
        in_struct!(i64_:   i64   = "<![CDATA[-42000000000000]]>", -42000000000000i64);
        in_struct!(isize_: isize = "<![CDATA[-42000000]]>", -42000000isize);

        in_struct!(u8_:    u8    = "<![CDATA[42]]>", 42u8);
        in_struct!(u16_:   u16   = "<![CDATA[4200]]>", 4200u16);
        in_struct!(u32_:   u32   = "<![CDATA[42000000]]>", 42000000u32);
        in_struct!(u64_:   u64   = "<![CDATA[42000000000000]]>", 42000000000000u64);
        in_struct!(usize_: usize = "<![CDATA[42000000]]>", 42000000usize);

        in_struct!(u128_: u128 = "<![CDATA[420000000000000000000000000000]]>", 420000000000000000000000000000u128);
        in_struct!(i128_: i128 = "<![CDATA[-420000000000000000000000000000]]>", -420000000000000000000000000000i128);

        in_struct!(f32_: f32 = "<![CDATA[4.2]]>", 4.2f32);
        in_struct!(f64_: f64 = "<![CDATA[4.2]]>", 4.2f64);

        in_struct!(false_: bool = "<![CDATA[false]]>", false);
        in_struct!(true_: bool = "<![CDATA[true]]>", true);
        in_struct!(char_: char = "<![CDATA[r]]>", 'r');

        // Escape sequences does not processed inside CDATA section
        in_struct!(string: String = "<![CDATA[escaped&#x20;string]]>", "escaped&#x20;string");

        /// XML does not able to store binary data
        #[test]
        fn byte_buf() {
            match from_str::<Trivial<ByteBuf>>("<root><![CDATA[escaped&#x20;byte_buf]]></root>") {
                Err(DeError::Custom(msg)) => {
                    assert_eq!(msg, "invalid type: string \"escaped&#x20;byte_buf\", expected byte data")
                }
                x => panic!(
                    r#"Expected `Err(Custom("invalid type: string \"escaped&#x20;byte_buf\", expected byte data"))`, but got `{:?}`"#,
                    x
                ),
            }
        }

        /// XML does not able to store binary data
        #[test]
        fn bytes() {
            match from_str::<Trivial<Bytes>>("<root><![CDATA[escaped&#x20;byte_buf]]></root>") {
                Err(DeError::Custom(msg)) => {
                    assert_eq!(msg, "invalid type: string \"escaped&#x20;byte_buf\", expected borrowed bytes")
                }
                x => panic!(
                    r#"Expected `Err(Custom("invalid type: string \"escaped&#x20;byte_buf\", expected borrowed bytes"))`, but got `{:?}`"#,
                    x
                ),
            }
        }
    }
}

mod unit {
    use super::*;
    use pretty_assertions::assert_eq;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Unit;

    #[test]
    fn simple() {
        let data: Unit = from_str("<root/>").unwrap();
        assert_eq!(data, Unit);
    }

    #[test]
    fn excess_attribute() {
        let data: Unit = from_str(r#"<root excess="attribute"/>"#).unwrap();
        assert_eq!(data, Unit);
    }

    #[test]
    fn excess_element() {
        let data: Unit = from_str(r#"<root><excess>element</excess></root>"#).unwrap();
        assert_eq!(data, Unit);
    }

    #[test]
    fn excess_text() {
        let data: Unit = from_str(r#"<root>excess text</root>"#).unwrap();
        assert_eq!(data, Unit);
    }

    #[test]
    fn excess_cdata() {
        let data: Unit = from_str(r#"<root><![CDATA[excess CDATA]]></root>"#).unwrap();
        assert_eq!(data, Unit);
    }
}

mod newtype {
    use super::*;
    use pretty_assertions::assert_eq;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Newtype(bool);

    #[test]
    fn simple() {
        let data: Newtype = from_str("<root>true</root>").unwrap();
        assert_eq!(data, Newtype(true));
    }

    #[test]
    fn excess_attribute() {
        let data: Newtype = from_str(r#"<root excess="attribute">true</root>"#).unwrap();
        assert_eq!(data, Newtype(true));
    }
}

mod tuple {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn simple() {
        let data: (f32, String) = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            "<root>42</root><root>answer</root>",
        )
        .unwrap();
        assert_eq!(data, (42.0, "answer".into()));
    }

    #[test]
    fn excess_attribute() {
        let data: (f32, String) = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root excess="attribute">42</root><root>answer</root>"#,
        )
        .unwrap();
        assert_eq!(data, (42.0, "answer".into()));
    }
}

mod tuple_struct {
    use super::*;
    use pretty_assertions::assert_eq;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Tuple(f32, String);

    #[test]
    fn simple() {
        let data: Tuple = from_str("<root>42</root><root>answer</root>").unwrap();
        assert_eq!(data, Tuple(42.0, "answer".into()));
    }

    #[test]
    fn excess_attribute() {
        let data: Tuple = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root excess="attribute">42</root><root>answer</root>"#,
        )
        .unwrap();
        assert_eq!(data, Tuple(42.0, "answer".into()));
    }
}

// seq tests are so big, so it in the separate file serde-de-seq.rs to speed-up compilation

macro_rules! maplike_errors {
    ($type:ty, $list:ty) => {
        maplike_errors!($type, $type, $list);
    };
    (
        $attributes:ty,
        $mixed:ty,
        $list:ty
    ) => {
        mod non_closed {
            use super::*;
            use pretty_assertions::assert_eq;
            use quick_xml::errors::{Error, IllFormedError};

            /// For struct we expect that error about not closed tag appears
            /// earlier than error about missing fields
            #[test]
            fn missing_field() {
                let data = from_str::<$mixed>(r#"<root>"#);

                match data {
                    Err(DeError::InvalidXml(Error::IllFormed(cause))) => {
                        assert_eq!(cause, IllFormedError::MissingEndTag("root".into()))
                    }
                    x => panic!(
                        "Expected `Err(InvalidXml(IllFormed(_)))`, but got `{:?}`",
                        x
                    ),
                }
            }

            #[test]
            fn attributes() {
                let data = from_str::<$attributes>(r#"<root float="42" string="answer">"#);

                match data {
                    Err(DeError::InvalidXml(Error::IllFormed(cause))) => {
                        assert_eq!(cause, IllFormedError::MissingEndTag("root".into()))
                    }
                    x => panic!(
                        "Expected `Err(InvalidXml(IllFormed(_)))`, but got `{:?}`",
                        x
                    ),
                }
            }

            #[test]
            fn elements_root() {
                let data = from_str::<$mixed>(r#"<root float="42"><string>answer</string>"#);

                match data {
                    Err(DeError::InvalidXml(Error::IllFormed(cause))) => {
                        assert_eq!(cause, IllFormedError::MissingEndTag("root".into()))
                    }
                    x => panic!(
                        "Expected `Err(InvalidXml(IllFormed(_)))`, but got `{:?}`",
                        x
                    ),
                }

                // Reaches `DeEvent::Eof` in `MapValueSeqAccess::next_element_seed`
                let data = from_str::<$list>(r#"<root><item>1</item><item>2</item>"#);

                match data {
                    Err(DeError::InvalidXml(Error::IllFormed(cause))) => {
                        assert_eq!(cause, IllFormedError::MissingEndTag("root".into()))
                    }
                    x => panic!(
                        "Expected `Err(InvalidXml(IllFormed(_)))`, but got `{:?}`",
                        x
                    ),
                }
            }

            #[test]
            fn elements_child() {
                // Reaches `Deserializer::read_text` when text is not present
                let data = from_str::<$mixed>(r#"<root float="42"><string>"#);

                match data {
                    Err(DeError::InvalidXml(Error::IllFormed(cause))) => {
                        assert_eq!(cause, IllFormedError::MissingEndTag("string".into()))
                    }
                    x => panic!(
                        "Expected `Err(InvalidXml(IllFormed(_)))`, but got `{:?}`",
                        x
                    ),
                }

                // Reaches `Deserializer::read_text` when text is present
                let data = from_str::<$mixed>(r#"<root float="42"><string>answer"#);

                match data {
                    Err(DeError::InvalidXml(Error::IllFormed(cause))) => {
                        assert_eq!(cause, IllFormedError::MissingEndTag("string".into()))
                    }
                    x => panic!(
                        "Expected `Err(InvalidXml(IllFormed(_)))`, but got `{:?}`",
                        x
                    ),
                }
            }
        }

        mod mismatched_end {
            use super::*;
            use pretty_assertions::assert_eq;
            use quick_xml::errors::{Error, IllFormedError};

            /// For struct we expect that error about mismatched tag appears
            /// earlier than error about missing fields
            #[test]
            fn missing_field() {
                let data = from_str::<$mixed>(r#"<root></mismatched>"#);

                match data {
                    Err(DeError::InvalidXml(Error::IllFormed(cause))) => assert_eq!(
                        cause,
                        IllFormedError::MismatchedEndTag {
                            expected: "root".into(),
                            found: "mismatched".into(),
                        }
                    ),
                    x => panic!(
                        "Expected `Err(InvalidXml(IllFormed(_)))`, but got `{:?}`",
                        x
                    ),
                }
            }

            #[test]
            fn attributes() {
                let data = from_str::<$attributes>(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root float="42" string="answer"></mismatched>"#,
                );

                match data {
                    Err(DeError::InvalidXml(Error::IllFormed(cause))) => assert_eq!(
                        cause,
                        IllFormedError::MismatchedEndTag {
                            expected: "root".into(),
                            found: "mismatched".into(),
                        }
                    ),
                    x => panic!(
                        "Expected `Err(InvalidXml(IllFormed(_)))`, but got `{:?}`",
                        x
                    ),
                }
            }

            #[test]
            fn elements_root() {
                let data = from_str::<$mixed>(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root float="42"><string>answer</string></mismatched>"#,
                );

                match data {
                    Err(DeError::InvalidXml(Error::IllFormed(cause))) => assert_eq!(
                        cause,
                        IllFormedError::MismatchedEndTag {
                            expected: "root".into(),
                            found: "mismatched".into(),
                        }
                    ),
                    x => panic!(
                        "Expected `Err(InvalidXml(IllFormed(_)))`, but got `{:?}`",
                        x
                    ),
                }
            }

            #[test]
            fn elements_child() {
                let data = from_str::<$mixed>(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root float="42"><string>answer</mismatched></root>"#,
                );

                match data {
                    Err(DeError::InvalidXml(Error::IllFormed(cause))) => assert_eq!(
                        cause,
                        IllFormedError::MismatchedEndTag {
                            expected: "string".into(),
                            found: "mismatched".into(),
                        }
                    ),
                    x => panic!(
                        "Expected `Err(InvalidXml(IllFormed(_)))`, but got `{:?}`",
                        x
                    ),
                }
            }
        }

        mod incomplete_tag {
            use super::*;
            use quick_xml::errors::{Error, SyntaxError};

            #[test]
            fn attributes() {
                let err = from_str::<$attributes>(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root float="42" string="answer""#,
                )
                .unwrap_err();
                match err {
                    // TODO: add error position to errors and check it here
                    // Related: https://github.com/tafia/quick-xml/issues/625
                    DeError::InvalidXml(Error::Syntax(SyntaxError::UnclosedTag)) => (),
                    _ => panic!(
                        "Expected `Err(InvalidXml(Syntax(UnclosedTag)))`, but got `{:?}`",
                        err
                    ),
                }
            }

            #[test]
            fn elements_root() {
                let err = from_str::<$mixed>(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root float="42"><string>answer</string><root"#,
                )
                .unwrap_err();
                match err {
                    // TODO: add error position to errors and check it here
                    // Related: https://github.com/tafia/quick-xml/issues/625
                    DeError::InvalidXml(Error::Syntax(SyntaxError::UnclosedTag)) => (),
                    _ => panic!(
                        "Expected `Err(InvalidXml(Syntax(UnclosedTag)))`, but got `{:?}`",
                        err
                    ),
                }
            }

            #[test]
            fn elements_child() {
                let err = from_str::<$mixed>(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root float="42"><string>answer</string"#,
                )
                .unwrap_err();
                match err {
                    // TODO: add error position to errors and check it here
                    // Related: https://github.com/tafia/quick-xml/issues/625
                    DeError::InvalidXml(Error::Syntax(SyntaxError::UnclosedTag)) => (),
                    _ => panic!(
                        "Expected `Err(InvalidXml(Syntax(UnclosedTag)))`, but got `{:?}`",
                        err
                    ),
                }
            }
        }
    };
}

mod map {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;
    use std::iter::FromIterator;

    #[test]
    fn elements() {
        let data: HashMap<(), ()> = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root><float>42</float><string>answer</string></root>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            HashMap::from_iter([((), ()), ((), ()),].iter().cloned())
        );
    }

    #[test]
    fn attributes() {
        let data: HashMap<(), ()> = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root float="42" string="answer"/>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            HashMap::from_iter([((), ()), ((), ()),].iter().cloned())
        );
    }

    #[test]
    fn attribute_and_element() {
        let data: HashMap<(), ()> = from_str(
            r#"
            <root float="42">
                <string>answer</string>
            </root>
            "#,
        )
        .unwrap();

        assert_eq!(
            data,
            HashMap::from_iter([((), ()), ((), ()),].iter().cloned())
        );
    }

    maplike_errors!(HashMap<(), ()>, HashMap<(), Vec<u32>>);
}

mod struct_ {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Type where all struct fields represented by elements
    #[derive(Debug, Deserialize, PartialEq)]
    struct Elements {
        float: f64,
        string: String,
    }

    /// Type where all struct fields represented by attributes
    #[derive(Debug, Deserialize, PartialEq)]
    struct Attributes {
        #[serde(rename = "@float")]
        float: f64,
        #[serde(rename = "@string")]
        string: String,
    }

    /// Type where one field represented by an attribute and one by an element
    #[derive(Debug, Deserialize, PartialEq)]
    struct Mixed {
        #[serde(rename = "@float")]
        float: f64,
        string: String,
    }

    /// Type where one field represented by list type
    #[derive(Debug, Deserialize, PartialEq)]
    struct List {
        item: Vec<f64>,
    }

    #[test]
    fn elements() {
        let data: Elements = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root><float>42</float><string>answer</string></root>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Elements {
                float: 42.0,
                string: "answer".into()
            }
        );
    }

    #[test]
    fn excess_elements() {
        let data: Elements = from_str(
            r#"
            <root>
                <before/>
                <float>42</float>
                <in-the-middle/>
                <string>answer</string>
                <after/>
            </root>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Elements {
                float: 42.0,
                string: "answer".into()
            }
        );
    }

    #[test]
    fn attributes() {
        let data: Attributes = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root float="42" string="answer"/>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Attributes {
                float: 42.0,
                string: "answer".into()
            }
        );
    }

    #[test]
    fn excess_attributes() {
        let data: Attributes = from_str(
            r#"<root before="1" float="42" in-the-middle="2" string="answer" after="3"/>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Attributes {
                float: 42.0,
                string: "answer".into()
            }
        );
    }

    #[test]
    fn attribute_and_element() {
        let data: Mixed = from_str(
            r#"
            <root float="42">
                <string>answer</string>
            </root>
        "#,
        )
        .unwrap();

        assert_eq!(
            data,
            Mixed {
                float: 42.0,
                string: "answer".into()
            }
        );
    }

    #[test]
    fn namespaces() {
        let data: Elements = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root xmlns:namespace="http://name.space"><namespace:float>42</namespace:float><string>answer</string></root>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Elements {
                float: 42.0,
                string: "answer".into()
            }
        );
    }

    /// Checks that excess data before the struct correctly handled.
    /// Any data not allowed before the struct
    mod excess_data_before {
        use super::*;
        use pretty_assertions::assert_eq;

        /// Space-only text events does not treated as data
        #[test]
        fn text_spaces_only() {
            let data: Elements = from_str(
                // Comment for prevent unnecessary formatting - we use the same style in all tests
                " \t\n\r<root><float>42</float><string>answer</string></root>",
            )
            .unwrap();
            assert_eq!(
                data,
                Elements {
                    float: 42.0,
                    string: "answer".into()
                }
            );
        }

        /// Text events with non-space characters are not allowed
        #[test]
        fn text_non_spaces() {
            match from_str::<Elements>(
                "\nexcess text\t<root><float>42</float><string>answer</string></root>",
            ) {
                Err(DeError::Custom(reason)) => assert_eq!(
                    reason,
                    "invalid type: string \"\\nexcess text\\t\", expected struct Elements",
                ),
                x => panic!(
                    r#"Expected `Err(Custom("invalid type: string \"\\nexcess text\\t\", expected struct Elements"))`, but got `{:?}`"#,
                    x
                ),
            };
        }

        /// CDATA events are not allowed
        #[test]
        fn cdata() {
            match from_str::<Elements>(
                "<![CDATA[excess cdata]]><root><float>42</float><string>answer</string></root>",
            ) {
                Err(DeError::Custom(reason)) => assert_eq!(
                    reason,
                    "invalid type: string \"excess cdata\", expected struct Elements",
                ),
                x => panic!(
                    r#"Expected `Err(Custom("invalid type: string \"excess cdata\", expected struct Elements"))`, but got `{:?}`"#,
                    x
                ),
            };
        }

        /// Comments are ignored, so they are allowed
        #[test]
        fn comment() {
            let data: Elements = from_str(
                // Comment for prevent unnecessary formatting - we use the same style in all tests
                "<!--comment--><root><float>42</float><string>answer</string></root>",
            )
            .unwrap();
            assert_eq!(
                data,
                Elements {
                    float: 42.0,
                    string: "answer".into()
                }
            );
        }

        /// Processing instructions are ignored, so they are allowed
        #[test]
        fn pi() {
            let data: Elements = from_str(
                // Comment for prevent unnecessary formatting - we use the same style in all tests
                "<?pi?><root><float>42</float><string>answer</string></root>",
            )
            .unwrap();
            assert_eq!(
                data,
                Elements {
                    float: 42.0,
                    string: "answer".into()
                }
            );
        }
    }

    maplike_errors!(Attributes, Mixed, List);
}

mod nested_struct {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn elements() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Struct {
            nested: Nested,
            string: String,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Nested {
            float: f32,
        }

        let data: Struct = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root><string>answer</string><nested><float>42</float></nested></root>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Struct {
                nested: Nested { float: 42.0 },
                string: "answer".into()
            }
        );
    }

    #[test]
    fn attributes() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Struct {
            nested: Nested,
            #[serde(rename = "@string")]
            string: String,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Nested {
            #[serde(rename = "@float")]
            float: f32,
        }

        let data: Struct = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root string="answer"><nested float="42"/></root>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Struct {
                nested: Nested { float: 42.0 },
                string: "answer".into()
            }
        );
    }
}

mod flatten_struct {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
    fn elements() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Struct {
            #[serde(flatten)]
            nested: Nested,
            string: String,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Nested {
            //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
            float: String,
        }

        let data: Struct = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root><float>42</float><string>answer</string></root>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Struct {
                nested: Nested { float: "42".into() },
                string: "answer".into()
            }
        );
    }

    #[test]
    fn attributes() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Struct {
            #[serde(flatten)]
            nested: Nested,
            #[serde(rename = "@string")]
            string: String,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Nested {
            //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
            #[serde(rename = "@float")]
            float: String,
        }

        let data: Struct = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root float="42" string="answer"/>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Struct {
                nested: Nested { float: "42".into() },
                string: "answer".into()
            }
        );
    }
}

// enum tests are so big, so it in the separate file serde-de-seq.rs to speed-up compilation

/// https://www.w3schools.com/xml/el_list.asp
mod xml_schema_lists {
    use super::*;

    macro_rules! list {
        ($name:ident: $type:ty = $xml:literal => $result:expr) => {
            #[test]
            fn $name() {
                let data: List<$type> = from_str($xml).unwrap();

                assert_eq!(data, List { list: $result });
            }
        };
    }

    macro_rules! err {
        ($name:ident: $type:ty = $xml:literal => $kind:ident($err:literal)) => {
            #[test]
            fn $name() {
                let err = from_str::<List<$type>>($xml).unwrap_err();

                match err {
                    DeError::$kind(e) => assert_eq!(e, $err),
                    _ => panic!(
                        "Expected `Err({}({}))`, but got `{:?}`",
                        stringify!($kind),
                        $err,
                        err
                    ),
                }
            }
        };
    }

    /// Checks that sequences can be deserialized from an XML attribute content
    /// according to the `xs:list` XML Schema type
    mod attribute {
        use super::*;
        use pretty_assertions::assert_eq;

        #[derive(Debug, Deserialize, PartialEq)]
        struct List<T> {
            #[serde(rename = "@list")]
            list: Vec<T>,
        }

        list!(i8_:  i8  = r#"<root list="1 -2  3"/>"# => vec![1, -2, 3]);
        list!(i16_: i16 = r#"<root list="1 -2  3"/>"# => vec![1, -2, 3]);
        list!(i32_: i32 = r#"<root list="1 -2  3"/>"# => vec![1, -2, 3]);
        list!(i64_: i64 = r#"<root list="1 -2  3"/>"# => vec![1, -2, 3]);

        list!(u8_:  u8  = r#"<root list="1 2  3"/>"# => vec![1, 2, 3]);
        list!(u16_: u16 = r#"<root list="1 2  3"/>"# => vec![1, 2, 3]);
        list!(u32_: u32 = r#"<root list="1 2  3"/>"# => vec![1, 2, 3]);
        list!(u64_: u64 = r#"<root list="1 2  3"/>"# => vec![1, 2, 3]);

        list!(i128_: i128 = r#"<root list="1 -2  3"/>"# => vec![1, -2, 3]);
        list!(u128_: u128 = r#"<root list="1 2  3"/>"# => vec![1, 2, 3]);

        list!(f32_: f32 = r#"<root list="1.23 -4.56  7.89"/>"# => vec![1.23, -4.56, 7.89]);
        list!(f64_: f64 = r#"<root list="1.23 -4.56  7.89"/>"# => vec![1.23, -4.56, 7.89]);

        list!(bool_: bool = r#"<root list="true false  true"/>"# => vec![true, false, true]);
        list!(char_: char = r#"<root list="4 2  j"/>"# => vec!['4', '2', 'j']);

        list!(string: String = r#"<root list="first second  third&#x20;3"/>"# => vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
            "3".to_string(),
        ]);
        err!(byte_buf: ByteBuf = r#"<root list="first second  third&#x20;3"/>"#
            => Custom("invalid type: string \"first\", expected byte data"));

        list!(unit: () = r#"<root list="1 second  false"/>"# => vec![(), (), ()]);
    }

    /// Checks that sequences can be deserialized from an XML text content
    /// according to the `xs:list` XML Schema type
    mod element {
        use super::*;

        #[derive(Debug, Deserialize, PartialEq)]
        struct List<T> {
            // Give it a special name that means text content of the XML node
            #[serde(rename = "$text")]
            list: Vec<T>,
        }

        mod text {
            use super::*;
            use pretty_assertions::assert_eq;

            list!(i8_:  i8  = "<root>1 -2  3</root>" => vec![1, -2, 3]);
            list!(i16_: i16 = "<root>1 -2  3</root>" => vec![1, -2, 3]);
            list!(i32_: i32 = "<root>1 -2  3</root>" => vec![1, -2, 3]);
            list!(i64_: i64 = "<root>1 -2  3</root>" => vec![1, -2, 3]);

            list!(u8_:  u8  = "<root>1 2  3</root>" => vec![1, 2, 3]);
            list!(u16_: u16 = "<root>1 2  3</root>" => vec![1, 2, 3]);
            list!(u32_: u32 = "<root>1 2  3</root>" => vec![1, 2, 3]);
            list!(u64_: u64 = "<root>1 2  3</root>" => vec![1, 2, 3]);

            list!(i128_: i128 = "<root>1 -2  3</root>" => vec![1, -2, 3]);
            list!(u128_: u128 = "<root>1 2  3</root>" => vec![1, 2, 3]);

            list!(f32_: f32 = "<root>1.23 -4.56  7.89</root>" => vec![1.23, -4.56, 7.89]);
            list!(f64_: f64 = "<root>1.23 -4.56  7.89</root>" => vec![1.23, -4.56, 7.89]);

            list!(bool_: bool = "<root>true false  true</root>" => vec![true, false, true]);
            list!(char_: char = "<root>4 2  j</root>" => vec!['4', '2', 'j']);

            // Expanding of entity references happens before list parsing
            // This is confirmed by XmlBeans (mature Java library) as well
            list!(string: String = "<root>first second  third&#x20;3</root>" => vec![
                "first".to_string(),
                "second".to_string(),
                "third".to_string(),
                "3".to_string(),
            ]);
            err!(byte_buf: ByteBuf = "<root>first second  third&#x20;3</root>"
                => Custom("invalid type: string \"first\", expected byte data"));

            list!(unit: () = "<root>1 second  false</root>" => vec![(), (), ()]);
        }

        mod cdata {
            use super::*;
            use pretty_assertions::assert_eq;

            list!(i8_:  i8  = "<root><![CDATA[1 -2  3]]></root>" => vec![1, -2, 3]);
            list!(i16_: i16 = "<root><![CDATA[1 -2  3]]></root>" => vec![1, -2, 3]);
            list!(i32_: i32 = "<root><![CDATA[1 -2  3]]></root>" => vec![1, -2, 3]);
            list!(i64_: i64 = "<root><![CDATA[1 -2  3]]></root>" => vec![1, -2, 3]);

            list!(u8_:  u8  = "<root><![CDATA[1 2  3]]></root>" => vec![1, 2, 3]);
            list!(u16_: u16 = "<root><![CDATA[1 2  3]]></root>" => vec![1, 2, 3]);
            list!(u32_: u32 = "<root><![CDATA[1 2  3]]></root>" => vec![1, 2, 3]);
            list!(u64_: u64 = "<root><![CDATA[1 2  3]]></root>" => vec![1, 2, 3]);

            list!(i128_: i128 = "<root><![CDATA[1 -2  3]]></root>" => vec![1, -2, 3]);
            list!(u128_: u128 = "<root><![CDATA[1 2  3]]></root>" => vec![1, 2, 3]);

            list!(f32_: f32 = "<root><![CDATA[1.23 -4.56  7.89]]></root>" => vec![1.23, -4.56, 7.89]);
            list!(f64_: f64 = "<root><![CDATA[1.23 -4.56  7.89]]></root>" => vec![1.23, -4.56, 7.89]);

            list!(bool_: bool = "<root><![CDATA[true false  true]]></root>" => vec![true, false, true]);
            list!(char_: char = "<root><![CDATA[4 2  j]]></root>" => vec!['4', '2', 'j']);

            // Cannot get whitespace in the value in any way if CDATA used:
            // - literal spaces means list item delimiters
            // - escaped sequences are not decoded in CDATA
            list!(string: String = "<root><![CDATA[first second  third&#x20;3]]></root>" => vec![
                "first".to_string(),
                "second".to_string(),
                "third&#x20;3".to_string(),
            ]);
            err!(byte_buf: ByteBuf = "<root>first second  third&#x20;3</root>"
                => Custom("invalid type: string \"first\", expected byte data"));

            list!(unit: () = "<root>1 second  false</root>" => vec![(), (), ()]);
        }
    }
}

/// Test for https://github.com/tafia/quick-xml/issues/324
#[test]
fn from_str_should_ignore_encoding() {
    let xml = r#"
        <?xml version="1.0" encoding="windows-1252" ?>
        <A a="€" />
    "#;

    #[derive(Debug, PartialEq, Deserialize)]
    struct A {
        #[serde(rename = "@a")]
        a: String,
    }

    let a: A = from_str(xml).unwrap();
    assert_eq!(
        a,
        A {
            a: "€".to_string()
        }
    );
}

/// Checks that deserializer is able to borrow data from the input
mod borrow {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::BTreeMap;
    use std::iter::FromIterator;

    /// Struct that should borrow input to be able to deserialize successfully.
    /// serde implicitly borrow `&str` and `&[u8]` even without `#[serde(borrow)]`
    #[derive(Debug, Deserialize, PartialEq)]
    struct BorrowedElement<'a> {
        string: &'a str,
    }

    /// Struct that should borrow input to be able to deserialize successfully.
    /// serde implicitly borrow `&str` and `&[u8]` even without `#[serde(borrow)]`
    #[derive(Debug, Deserialize, PartialEq)]
    struct BorrowedAttribute<'a> {
        #[serde(rename = "@string")]
        string: &'a str,
    }

    /// Deserialization of all XML's in that module expected to pass because
    /// unescaping is not required, so deserialized `Borrowed*` types can hold
    /// references to the original buffer with an XML text
    mod non_escaped {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn top_level() {
            let data: &str = from_str(r#"<root>without escape sequences</root>"#).unwrap();
            assert_eq!(data, "without escape sequences",);
        }

        #[test]
        fn element() {
            let data: BorrowedElement = from_str(
                r#"
                <root>
                    <string>without escape sequences</string>
                </root>"#,
            )
            .unwrap();
            assert_eq!(
                data,
                BorrowedElement {
                    string: "without escape sequences",
                }
            );
        }

        #[test]
        fn attribute() {
            let data: BorrowedAttribute =
                from_str(r#"<root string="without escape sequences"/>"#).unwrap();
            assert_eq!(
                data,
                BorrowedAttribute {
                    string: "without escape sequences",
                }
            );
        }
    }

    /// Deserialization of all XML's in that module expected to fail because
    /// values requires unescaping that will lead to allocating an internal
    /// buffer by deserializer, but the `Borrowed*` types couldn't take ownership
    /// on it.
    ///
    /// The same behavior demonstrates the `serde_json` crate
    mod escaped {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn top_level() {
            match from_str::<&str>(r#"<root>with escape sequence: &lt;</root>"#) {
                Err(DeError::Custom(reason)) => assert_eq!(
                    reason,
                    "invalid type: string \"with escape sequence: <\", expected a borrowed string"
                ),
                e => panic!(
                    r#"Expected `Err(Custom("invalid type: string \"with escape sequence: <\", expected a borrowed string"))`, but got `{:?}`"#,
                    e
                ),
            }
        }

        #[test]
        fn element() {
            match from_str::<BorrowedElement>(
                r#"
                <root>
                    <string>with escape sequence: &lt;</string>
                </root>"#,
            ) {
                Err(DeError::Custom(reason)) => assert_eq!(
                    reason,
                    "invalid type: string \"with escape sequence: <\", expected a borrowed string"
                ),
                e => panic!(
                    r#"Expected `Err(Custom("invalid type: string \"with escape sequence: <\", expected a borrowed string"))`, but got `{:?}`"#,
                    e
                ),
            }
        }

        #[test]
        fn attribute() {
            match from_str::<BorrowedAttribute>(r#"<root string="with &quot;escape&quot; sequences"/>"#) {
                Err(DeError::Custom(reason)) => assert_eq!(
                    reason,
                    "invalid type: string \"with \\\"escape\\\" sequences\", expected a borrowed string"
                ),
                e => panic!(
                    r#"Expected `Err(Custom("invalid type: string \"with \"escape\" sequences\", expected a borrowed string"))`, but got `{:?}`"#,
                    e
                ),
            }
        }
    }

    #[test]
    fn element_name() {
        let data: BTreeMap<&str, &str> = from_str(
            r#"
            <root>
                <element>element content</element>
                text content
            </root>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            BTreeMap::from_iter([
                // Comment to prevent formatting in one line
                ("element", "element content"),
                ("$text", "\n                text content\n            "),
            ])
        );
    }
}

/// Test for entity resolver
mod resolve {
    use super::*;
    use pretty_assertions::assert_eq;
    use quick_xml::de::EntityResolver;
    use quick_xml::events::BytesText;
    use std::collections::BTreeMap;
    use std::convert::Infallible;
    use std::iter::FromIterator;

    struct TestEntityResolver {
        capture_called: bool,
    }

    impl EntityResolver for TestEntityResolver {
        type Error = Infallible;

        fn capture(&mut self, doctype: BytesText) -> Result<(), Self::Error> {
            self.capture_called = true;

            assert_eq!(doctype.as_ref(), br#"dict[ <!ENTITY unc "unclassified"> ]"#);

            Ok(())
        }

        fn resolve(&self, entity: &str) -> Option<&str> {
            assert!(
                self.capture_called,
                "`EntityResolver::capture` should be called before `EntityResolver::resolve`"
            );
            match entity {
                "t1" => Some("test_one"),
                "t2" => Some("test_two"),
                _ => None,
            }
        }
    }

    #[test]
    fn resolve_custom_entity() {
        let resolver = TestEntityResolver {
            capture_called: false,
        };
        let mut de = Deserializer::with_resolver(
            br#"
            <!DOCTYPE dict[ <!ENTITY unc "unclassified"> ]>

            <root>
                <entity_one>&t1;</entity_one>
                <entity_two>&t2;</entity_two>
                <entity_three>non-entity</entity_three>
            </root>
            "#
            .as_ref(),
            resolver,
        );

        let data: BTreeMap<String, String> = BTreeMap::deserialize(&mut de).unwrap();
        assert_eq!(
            data,
            BTreeMap::from_iter([
                (String::from("entity_one"), String::from("test_one")),
                (String::from("entity_two"), String::from("test_two")),
                (String::from("entity_three"), String::from("non-entity")),
            ])
        );
    }
}

/// Tests for https://github.com/tafia/quick-xml/pull/603.
///
/// According to <https://www.w3.org/TR/xml11/#NT-prolog> comments,
/// processing instructions and spaces are possible after XML declaration or DTD.
/// Their existence should not break deserializing
///
/// ```text
/// [22] prolog ::= XMLDecl Misc* (doctypedecl Misc*)?
/// [27] Misc   ::= Comment | PI | S
/// ```
mod xml_prolog {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;

    #[test]
    fn spaces() {
        assert_eq!(
            from_str::<HashMap<(), ()>>(
                r#"
        <?xml version="1.1"?>

        <!DOCTYPE dict>

        <doc>
        </doc>
        "#
            )
            .unwrap(),
            HashMap::new()
        );
    }

    #[test]
    fn comments() {
        assert_eq!(
            from_str::<HashMap<(), ()>>(
                r#"
        <?xml version="1.1"?>
        <!-- comment between xml declaration and doctype -->
        <!-- another comment -->
        <!DOCTYPE dict>
        <!-- comment between doctype and root element -->
        <!-- another comment -->
        <doc>
        </doc>
        "#,
            )
            .unwrap(),
            HashMap::new()
        );
    }

    #[test]
    fn pi() {
        assert_eq!(
            from_str::<HashMap<(), ()>>(
                r#"
        <?xml version="1.1"?>
        <?pi?>
        <?another pi?>
        <!DOCTYPE dict>
        <?pi?>
        <?another pi?>
        <doc>
        </doc>
        "#,
            )
            .unwrap(),
            HashMap::new()
        );
    }
}

/// Tests for https://github.com/tafia/quick-xml/pull/937.
///
/// Checks that correct EOL normalization rules is applied to the texts.
mod xml_version {
    use super::*;
    use pretty_assertions::assert_eq;
    use quick_xml::errors::{Error, IllFormedError};

    #[derive(Debug, Deserialize, PartialEq)]
    struct Root {
        #[serde(rename = "@attribute")]
        attribute: String,

        #[serde(rename = "$text")]
        text: String,
    }

    #[test]
    fn v1_0_implicit() {
        assert_eq!(
            from_str::<Root>(
                "\
                <root attribute='\r\n,\n,\r,\r\u{0085},\u{0085},\u{2028}'>\r\n,\n,\r,\r\u{0085},\u{0085},\u{2028}</root>\
                "
            )
            .unwrap(),
            Root {
                attribute: " , , , \u{0085},\u{0085},\u{2028}".to_string(),
                text: "\n,\n,\n,\n\u{0085},\u{0085},\u{2028}".to_string(),
            }
        );
    }

    #[test]
    fn v1_0_explicit() {
        assert_eq!(
            from_str::<Root>(
                "\
                <?xml version='1.0'?>\
                <root attribute='\r\n,\n,\r,\r\u{0085},\u{0085},\u{2028}'>\r\n,\n,\r,\r\u{0085},\u{0085},\u{2028}</root>\
                "
            )
            .unwrap(),
            Root {
                attribute: " , , , \u{0085},\u{0085},\u{2028}".to_string(),
                text: "\n,\n,\n,\n\u{0085},\u{0085},\u{2028}".to_string(),
            }
        );
    }

    #[test]
    fn v1_1() {
        assert_eq!(
            from_str::<Root>(
                "\
                <?xml version='1.1'?>\
                <root attribute='\r\n,\n,\r,\r\u{0085},\u{0085},\u{2028}'>\r\n,\n,\r,\r\u{0085},\u{0085},\u{2028}</root>\
                "
            )
            .unwrap(),
            Root {
                attribute: " , , , , , ".to_string(),
                text: "\n,\n,\n,\n,\n,\n".to_string(),
            }
        );
    }

    #[test]
    fn unknown() {
        match from_str::<Root>(
            "\
                <?xml version='1.2'?>\
                <root attribute='\r\n,\n,\r,\r\u{0085},\u{0085},\u{2028}'>\r\n,\n,\r,\r\u{0085},\u{0085},\u{2028}</root>\
                ",
        ) {
            Err(DeError::InvalidXml(Error::IllFormed(cause))) => {
                assert_eq!(cause, IllFormedError::UnknownVersion,)
            }
            x => panic!("Expected `Err(InvalidXml(IllFormed(_)))`, but got {:?}", x),
        }
    }
}

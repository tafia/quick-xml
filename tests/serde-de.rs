use quick_xml::de::Deserializer;
use quick_xml::utils::{ByteBuf, Bytes};
use quick_xml::DeError;

use pretty_assertions::assert_eq;

use serde::de::IgnoredAny;
use serde::serde_if_integer128;
use serde::Deserialize;

/// Deserialize an instance of type T from a string of XML text.
/// If deserialization was succeeded checks that all XML events was consumed
fn from_str<'de, T>(s: &'de str) -> Result<T, DeError>
where
    T: Deserialize<'de>,
{
    // Log XML that we try to deserialize to see it in the failed tests output
    dbg!(s);
    let mut de = Deserializer::from_str(s);
    let result = T::deserialize(&mut de);

    // If type was deserialized, the whole XML document should be consumed
    if let Ok(_) = result {
        match <()>::deserialize(&mut de) {
            Err(DeError::UnexpectedEof) => (),
            e => panic!("Expected end `UnexpectedEof`, but got {:?}", e),
        }
    }

    result
}

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
        other => panic!("Expected `UnexpectedEof`, found {:?}", other),
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
                    Err(DeError::UnexpectedEof) => (),
                    x => panic!(
                        r#"Expected `Err(DeError::UnexpectedEof)`, but got `{:?}`"#,
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

            serde_if_integer128! {
                eof!(u128_: u128 = $value);
                eof!(i128_: i128 = $value);
            }

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
                    Err(DeError::Unsupported(msg)) => {
                        assert_eq!(msg, "binary data content is not supported by XML format")
                    }
                    x => panic!(
                        r#"Expected `Err(DeError::Unsupported("binary data content is not supported by XML format"))`, but got `{:?}`"#,
                        x
                    ),
                }
            }

            /// XML does not able to store binary data
            #[test]
            fn bytes() {
                match from_str::<Bytes>($value) {
                    Err(DeError::Unsupported(msg)) => {
                        assert_eq!(msg, "binary data content is not supported by XML format")
                    }
                    x => panic!(
                        r#"Expected `Err(DeError::Unsupported("binary data content is not supported by XML format"))`, but got `{:?}`"#,
                        x
                    ),
                }
            }

            #[test]
            fn unit() {
                match from_str::<()>($value) {
                    Err(DeError::UnexpectedEof) => (),
                    x => panic!(
                        r#"Expected `Err(DeError::UnexpectedEof)`, but got `{:?}`"#,
                        x
                    ),
                }
            }
        };
    }

    /// Empty document should considered invalid no matter what type we try to deserialize
    mod empty_doc {
        use super::*;
        use pretty_assertions::assert_eq;

        eof!("");
    }

    /// Document that contains only comment should be handled as if it is empty
    mod only_comment {
        use super::*;
        use pretty_assertions::assert_eq;

        eof!("<!--comment-->");
    }

    /// Tests deserialization from top-level tag content: `<root>...content...</root>`
    mod struct_ {
        use super::*;

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

        macro_rules! in_struct {
            ($name:ident: $type:ty = $value:expr, $expected:expr) => {
                #[test]
                fn $name() {
                    let item: Trivial<$type> = from_str($value).unwrap();

                    assert_eq!(item, Trivial { value: $expected });

                    match from_str::<Trivial<$type>>(&format!("<outer>{}</outer>", $value)) {
                        // Expected unexpected start element `<root>`
                        Err(DeError::Custom(reason)) => assert_eq!(reason, "missing field `$text`"),
                        x => panic!(
                            r#"Expected `Err(DeError::Custom("missing field `$text`"))`, but got `{:?}`"#,
                            x
                        ),
                    }
                }
            };
        }

        /// Tests deserialization from text content in a tag
        #[rustfmt::skip] // tests formatted in a table
        mod text {
            use super::*;
            use pretty_assertions::assert_eq;

            in_struct!(i8_:    i8    = "<root>-42</root>", -42i8);
            in_struct!(i16_:   i16   = "<root>-4200</root>", -4200i16);
            in_struct!(i32_:   i32   = "<root>-42000000</root>", -42000000i32);
            in_struct!(i64_:   i64   = "<root>-42000000000000</root>", -42000000000000i64);
            in_struct!(isize_: isize = "<root>-42000000000000</root>", -42000000000000isize);

            in_struct!(u8_:    u8    = "<root>42</root>", 42u8);
            in_struct!(u16_:   u16   = "<root>4200</root>", 4200u16);
            in_struct!(u32_:   u32   = "<root>42000000</root>", 42000000u32);
            in_struct!(u64_:   u64   = "<root>42000000000000</root>", 42000000000000u64);
            in_struct!(usize_: usize = "<root>42000000000000</root>", 42000000000000usize);

            serde_if_integer128! {
                in_struct!(u128_: u128 = "<root>420000000000000000000000000000</root>", 420000000000000000000000000000u128);
                in_struct!(i128_: i128 = "<root>-420000000000000000000000000000</root>", -420000000000000000000000000000i128);
            }

            in_struct!(f32_: f32 = "<root>4.2</root>", 4.2f32);
            in_struct!(f64_: f64 = "<root>4.2</root>", 4.2f64);

            in_struct!(false_: bool = "<root>false</root>", false);
            in_struct!(true_: bool = "<root>true</root>", true);
            in_struct!(char_: char = "<root>r</root>", 'r');

            in_struct!(string: String = "<root>escaped&#x20;string</root>", "escaped string".into());

            /// XML does not able to store binary data
            #[test]
            fn byte_buf() {
                match from_str::<Trivial<ByteBuf>>("<root>escaped&#x20;byte_buf</root>") {
                    Err(DeError::Unsupported(msg)) => {
                        assert_eq!(msg, "binary data content is not supported by XML format")
                    }
                    x => panic!(
                        r#"Expected `Err(DeError::Unsupported("binary data content is not supported by XML format"))`, but got `{:?}`"#,
                        x
                    ),
                }
            }

            /// XML does not able to store binary data
            #[test]
            fn bytes() {
                match from_str::<Trivial<Bytes>>("<root>escaped&#x20;byte_buf</root>") {
                    Err(DeError::Unsupported(msg)) => {
                        assert_eq!(msg, "binary data content is not supported by XML format")
                    }
                    x => panic!(
                        r#"Expected `Err(DeError::Unsupported("binary data content is not supported by XML format"))`, but got `{:?}`"#,
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

            in_struct!(i8_:    i8    = "<root><![CDATA[-42]]></root>", -42i8);
            in_struct!(i16_:   i16   = "<root><![CDATA[-4200]]></root>", -4200i16);
            in_struct!(i32_:   i32   = "<root><![CDATA[-42000000]]></root>", -42000000i32);
            in_struct!(i64_:   i64   = "<root><![CDATA[-42000000000000]]></root>", -42000000000000i64);
            in_struct!(isize_: isize = "<root><![CDATA[-42000000000000]]></root>", -42000000000000isize);

            in_struct!(u8_:    u8    = "<root><![CDATA[42]]></root>", 42u8);
            in_struct!(u16_:   u16   = "<root><![CDATA[4200]]></root>", 4200u16);
            in_struct!(u32_:   u32   = "<root><![CDATA[42000000]]></root>", 42000000u32);
            in_struct!(u64_:   u64   = "<root><![CDATA[42000000000000]]></root>", 42000000000000u64);
            in_struct!(usize_: usize = "<root><![CDATA[42000000000000]]></root>", 42000000000000usize);

            serde_if_integer128! {
                in_struct!(u128_: u128 = "<root><![CDATA[420000000000000000000000000000]]></root>", 420000000000000000000000000000u128);
                in_struct!(i128_: i128 = "<root><![CDATA[-420000000000000000000000000000]]></root>", -420000000000000000000000000000i128);
            }

            in_struct!(f32_: f32 = "<root><![CDATA[4.2]]></root>", 4.2f32);
            in_struct!(f64_: f64 = "<root><![CDATA[4.2]]></root>", 4.2f64);

            in_struct!(false_: bool = "<root><![CDATA[false]]></root>", false);
            in_struct!(true_: bool = "<root><![CDATA[true]]></root>", true);
            in_struct!(char_: char = "<root><![CDATA[r]]></root>", 'r');

            // Escape sequences does not processed inside CDATA section
            in_struct!(string: String = "<root><![CDATA[escaped&#x20;string]]></root>", "escaped&#x20;string".into());

            /// XML does not able to store binary data
            #[test]
            fn byte_buf() {
                match from_str::<Trivial<ByteBuf>>("<root><![CDATA[escaped&#x20;byte_buf]]></root>") {
                    Err(DeError::Unsupported(msg)) => {
                        assert_eq!(msg, "binary data content is not supported by XML format")
                    }
                    x => panic!(
                        r#"Expected `Err(DeError::Unsupported("binary data content is not supported by XML format"))`, but got `{:?}`"#,
                        x
                    ),
                }
            }

            /// XML does not able to store binary data
            #[test]
            fn bytes() {
                match from_str::<Trivial<Bytes>>("<root><![CDATA[escaped&#x20;byte_buf]]></root>") {
                    Err(DeError::Unsupported(msg)) => {
                        assert_eq!(msg, "binary data content is not supported by XML format")
                    }
                    x => panic!(
                        r#"Expected `Err(DeError::Unsupported("binary data content is not supported by XML format"))`, but got `{:?}`"#,
                        x
                    ),
                }
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

mod seq {
    use super::*;

    /// Check that top-level sequences can be deserialized from the multi-root XML documents
    mod top_level {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn simple() {
            from_str::<[(); 3]>("<root/><root>42</root><root>answer</root>").unwrap();

            let data: Vec<()> = from_str("<root/><root>42</root><root>answer</root>").unwrap();
            assert_eq!(data, vec![(), (), ()]);
        }

        /// Special case: empty sequence
        #[test]
        fn empty() {
            from_str::<[(); 0]>("").unwrap();

            let data: Vec<()> = from_str("").unwrap();
            assert_eq!(data, vec![]);
        }

        /// Special case: one-element sequence
        #[test]
        fn one_element() {
            from_str::<[(); 1]>("<root/>").unwrap();
            from_str::<[(); 1]>("<root>42</root>").unwrap();
            from_str::<[(); 1]>("text").unwrap();
            from_str::<[(); 1]>("<![CDATA[cdata]]>").unwrap();

            let data: Vec<()> = from_str("<root/>").unwrap();
            assert_eq!(data, vec![()]);

            let data: Vec<()> = from_str("<root>42</root>").unwrap();
            assert_eq!(data, vec![()]);

            let data: Vec<()> = from_str("text").unwrap();
            assert_eq!(data, vec![()]);

            let data: Vec<()> = from_str("<![CDATA[cdata]]>").unwrap();
            assert_eq!(data, vec![()]);
        }

        #[test]
        fn excess_attribute() {
            from_str::<[(); 3]>(r#"<root/><root excess="attribute">42</root><root>answer</root>"#)
                .unwrap();

            let data: Vec<()> =
                from_str(r#"<root/><root excess="attribute">42</root><root>answer</root>"#)
                    .unwrap();
            assert_eq!(data, vec![(), (), ()]);
        }

        #[test]
        fn mixed_content() {
            from_str::<[(); 3]>(
                r#"
                <element/>
                text
                <![CDATA[cdata]]>
                "#,
            )
            .unwrap();

            let data: Vec<()> = from_str(
                r#"
                <element/>
                text
                <![CDATA[cdata]]>
                "#,
            )
            .unwrap();
            assert_eq!(data, vec![(), (), ()]);
        }

        /// This test ensures that composition of deserializer building blocks plays well
        #[test]
        fn list_of_struct() {
            #[derive(Debug, PartialEq, Default, Deserialize)]
            #[serde(default)]
            struct Struct {
                #[serde(rename = "@attribute")]
                attribute: Option<String>,
                element: Option<String>,
            }

            let data: Vec<Struct> = from_str(
                r#"
                <struct/>
                <struct attribute="value"/>
                <struct>
                    <element>value</element>
                </struct>
                <struct attribute="value">
                    <element>value</element>
                </struct>
                "#,
            )
            .unwrap();
            assert_eq!(
                data,
                vec![
                    Struct {
                        attribute: None,
                        element: None,
                    },
                    Struct {
                        attribute: Some("value".to_string()),
                        element: None,
                    },
                    Struct {
                        attribute: None,
                        element: Some("value".to_string()),
                    },
                    Struct {
                        attribute: Some("value".to_string()),
                        element: Some("value".to_string()),
                    },
                ]
            );
        }

        /// Test for https://github.com/tafia/quick-xml/issues/500
        #[test]
        fn list_of_enum() {
            #[derive(Debug, PartialEq, Deserialize)]
            enum Enum {
                One,
                Two,
            }

            let data: Vec<Enum> = from_str(
                r#"
                <One/>
                <Two/>
                <One/>
                "#,
            )
            .unwrap();
            assert_eq!(data, vec![Enum::One, Enum::Two, Enum::One]);
        }
    }

    /// Tests where each sequence item have an identical name in an XML.
    /// That explicitly means that `enum`s as list elements are not supported
    /// in that case, because enum requires different tags.
    ///
    /// (by `enums` we mean [externally tagged enums] is serde terminology)
    ///
    /// [externally tagged enums]: https://serde.rs/enum-representations.html#externally-tagged
    mod fixed_name {
        use super::*;

        /// This module contains tests where size of the list have a compile-time size
        mod fixed_size {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                item: [(); 3],
            }

            /// Simple case: count of elements matches expected size of sequence,
            /// each element has the same name. Successful deserialization expected
            #[test]
            fn simple() {
                from_str::<List>(
                    r#"
                    <root>
                        <item/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();
            }

            /// Special case: empty sequence
            #[test]
            #[ignore = "it is impossible to distinguish between missed field and empty list: use `Option<>` or #[serde(default)]"]
            fn empty() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    item: [(); 0],
                }

                from_str::<List>(r#"<root></root>"#).unwrap();
                from_str::<List>(r#"<root/>"#).unwrap();
            }

            /// Special case: one-element sequence
            #[test]
            fn one_element() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    item: [(); 1],
                }

                from_str::<List>(
                    r#"
                    <root>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();
            }

            /// Fever elements than expected size of sequence, each element has
            /// the same name. Failure expected
            #[test]
            fn fever_elements() {
                let data = from_str::<List>(
                    r#"
                    <root>
                        <item/>
                        <item/>
                    </root>
                    "#,
                );

                match data {
                    Err(DeError::Custom(e)) => {
                        assert_eq!(e, "invalid length 2, expected an array of length 3")
                    }
                    e => panic!(
                        r#"Expected `Err(Custom("invalid length 2, expected an array of length 3"))`, but found {:?}"#,
                        e
                    ),
                }
            }

            /// More elements than expected size of sequence, each element has
            /// the same name. Failure expected. If you wish to ignore excess
            /// elements, use the special type, that consume as much elements
            /// as possible, but ignores excess elements
            #[test]
            fn excess_elements() {
                let data = from_str::<List>(
                    r#"
                    <root>
                        <item/>
                        <item/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                );

                match data {
                    Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `item`"),
                    e => panic!(
                        r#"Expected `Err(Custom("duplicate field `item`"))`, but found {:?}"#,
                        e
                    ),
                }
            }

            /// Mixed content assumes, that some elements will have an internal
            /// name `$text` or `$value`, so, unless field named the same, it is expected
            /// to fail
            #[test]
            fn mixed_content() {
                let data = from_str::<List>(
                    r#"
                    <root>
                        <element/>
                        text
                        <![CDATA[cdata]]>
                    </root>
                    "#,
                );

                match data {
                    Err(DeError::Custom(e)) => assert_eq!(e, "missing field `item`"),
                    e => panic!(
                        r#"Expected `Err(Custom("missing field `item`"))`, but found {:?}"#,
                        e
                    ),
                }
            }

            /// In those tests sequence should be deserialized from an XML
            /// with additional elements that is not defined in the struct.
            /// That fields should be skipped during deserialization
            mod unknown_items {
                use super::*;
                #[cfg(not(feature = "overlapped-lists"))]
                use pretty_assertions::assert_eq;

                #[test]
                fn before() {
                    from_str::<List>(
                        r#"
                        <root>
                            <unknown/>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                #[test]
                fn after() {
                    from_str::<List>(
                        r#"
                        <root>
                            <item/>
                            <item/>
                            <item/>
                            <unknown/>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<List>(
                        r#"
                        <root>
                            <item/>
                            <unknown/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    data.unwrap();

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                            e
                        ),
                    }
                }

                /// Test for https://github.com/tafia/quick-xml/issues/435
                #[test]
                fn overlapped_with_nested_list() {
                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        outer: [List; 3],
                    }

                    let data = from_str::<Root>(
                        r#"
                        <root>
                          <outer><item/><item/><item/></outer>
                          <unknown/>
                          <outer><item/><item/><item/></outer>
                          <outer><item/><item/><item/></outer>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    data.unwrap();

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests non-sequential field is defined in the struct
            /// before sequential, so it will be deserialized before the list.
            /// That struct should be deserialized from an XML where these
            /// fields comes in an arbitrary order
            mod field_before_list {
                use super::*;
                #[cfg(not(feature = "overlapped-lists"))]
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    node: (),
                    item: [(); 3],
                }

                #[test]
                fn before() {
                    from_str::<Root>(
                        r#"
                        <root>
                            <node/>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                #[test]
                fn after() {
                    from_str::<Root>(
                        r#"
                        <root>
                            <item/>
                            <item/>
                            <item/>
                            <node/>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <item/>
                            <node/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    data.unwrap();

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                            e
                        ),
                    }
                }

                /// Test for https://github.com/tafia/quick-xml/issues/435
                #[test]
                fn overlapped_with_nested_list() {
                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        node: (),
                        outer: [List; 3],
                    }

                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <outer><item/><item/><item/></outer>
                            <node/>
                            <outer><item/><item/><item/></outer>
                            <outer><item/><item/><item/></outer>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    data.unwrap();

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests non-sequential field is defined in the struct
            /// after sequential, so it will be deserialized after the list.
            /// That struct should be deserialized from an XML where these
            /// fields comes in an arbitrary order
            mod field_after_list {
                use super::*;
                #[cfg(not(feature = "overlapped-lists"))]
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    item: [(); 3],
                    node: (),
                }

                #[test]
                fn before() {
                    from_str::<Root>(
                        r#"
                        <root>
                            <node/>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                #[test]
                fn after() {
                    from_str::<Root>(
                        r#"
                        <root>
                            <item/>
                            <item/>
                            <item/>
                            <node/>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <item/>
                            <node/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    data.unwrap();

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                            e
                        ),
                    }
                }

                /// Test for https://github.com/tafia/quick-xml/issues/435
                #[test]
                fn overlapped_with_nested_list() {
                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        outer: [List; 3],
                        node: (),
                    }

                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <outer><item/><item/><item/></outer>
                            <node/>
                            <outer><item/><item/><item/></outer>
                            <outer><item/><item/><item/></outer>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    data.unwrap();

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests two lists are deserialized simultaneously.
            /// Lists should be deserialized even when them overlaps
            mod two_lists {
                use super::*;
                #[cfg(not(feature = "overlapped-lists"))]
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Pair {
                    item: [(); 3],
                    element: [(); 2],
                }

                #[test]
                fn splitted() {
                    from_str::<Pair>(
                        r#"
                        <root>
                            <element/>
                            <element/>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Pair>(
                        r#"
                        <root>
                            <item/>
                            <element/>
                            <item/>
                            <element/>
                            <item/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    data.unwrap();

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                            e
                        ),
                    }
                }

                /// Test for https://github.com/tafia/quick-xml/issues/435
                #[test]
                fn overlapped_with_nested_list() {
                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Pair {
                        outer: [List; 3],
                        element: [(); 2],
                    }

                    let data = from_str::<Pair>(
                        r#"
                        <root>
                            <outer><item/><item/><item/></outer>
                            <element/>
                            <outer><item/><item/><item/></outer>
                            <element/>
                            <outer><item/><item/><item/></outer>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    data.unwrap();

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// Deserialization of primitives slightly differs from deserialization
            /// of complex types, so need to check this separately
            #[test]
            fn primitives() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    item: [usize; 3],
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <item>41</item>
                        <item>42</item>
                        <item>43</item>
                    </root>
                    "#,
                )
                .unwrap();
                assert_eq!(data, List { item: [41, 42, 43] });

                from_str::<List>(
                    r#"
                    <root>
                        <item>41</item>
                        <item><item>42</item></item>
                        <item>43</item>
                    </root>
                    "#,
                )
                .unwrap_err();
            }

            /// This test ensures that composition of deserializer building blocks
            /// plays well
            #[test]
            fn list_of_struct() {
                #[derive(Debug, PartialEq, Default, Deserialize)]
                #[serde(default)]
                struct Struct {
                    #[serde(rename = "@attribute")]
                    attribute: Option<String>,
                    element: Option<String>,
                }

                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    item: [Struct; 4],
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <item/>
                        <item attribute="value"/>
                        <item>
                            <element>value</element>
                        </item>
                        <item attribute="value">
                            <element>value</element>
                        </item>
                    </root>
                    "#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    List {
                        item: [
                            Struct {
                                attribute: None,
                                element: None,
                            },
                            Struct {
                                attribute: Some("value".to_string()),
                                element: None,
                            },
                            Struct {
                                attribute: None,
                                element: Some("value".to_string()),
                            },
                            Struct {
                                attribute: Some("value".to_string()),
                                element: Some("value".to_string()),
                            },
                        ],
                    }
                );
            }

            /// Checks that sequences represented by elements can contain sequences,
            /// represented by [`xs:list`s](https://www.w3schools.com/xml/el_list.asp)
            mod xs_list {
                use super::*;
                use pretty_assertions::assert_eq;

                /// Special case: zero elements
                #[test]
                fn zero() {
                    #[derive(Debug, Deserialize, PartialEq)]
                    struct List {
                        /// Outer list mapped to elements, inner -- to `xs:list`.
                        ///
                        /// `#[serde(default)]` is required to correctly deserialize
                        /// empty sequence, because without elements the field
                        /// also is missing and derived `Deserialize` implementation
                        /// would complain about that unless field is marked as
                        /// `default`.
                        #[serde(default)]
                        item: [Vec<String>; 0],
                    }

                    let data: List = from_str(
                        r#"
                        <root>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(data, List { item: [] });
                }

                /// Special case: one element
                #[test]
                fn one() {
                    #[derive(Debug, Deserialize, PartialEq)]
                    struct List {
                        /// Outer list mapped to elements, inner -- to `xs:list`.
                        ///
                        /// `#[serde(default)]` is not required, because correct
                        /// XML will always contains at least 1 element.
                        item: [Vec<String>; 1],
                    }

                    let data: List = from_str(
                        r#"
                        <root>
                            <item>first list</item>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        List {
                            item: [vec!["first".to_string(), "list".to_string()]]
                        }
                    );
                }

                /// Special case: outer list is always mapped to an elements sequence,
                /// not to an `xs:list`
                #[test]
                fn element() {
                    #[derive(Debug, Deserialize, PartialEq)]
                    struct List {
                        /// Outer list mapped to elements, inner -- to `xs:list`.
                        ///
                        /// `#[serde(default)]` is not required, because correct
                        /// XML will always contains at least 1 element.
                        item: [String; 1],
                    }

                    let data: List = from_str(
                        r#"
                        <root>
                            <item>first item</item>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        List {
                            item: ["first item".to_string()]
                        }
                    );
                }

                /// This tests demonstrates, that for `$value` field (`list`) actual
                /// name of XML element (`item`) does not matter. That allows list
                /// item to be an enum, where tag name determines enum variant
                #[test]
                fn many() {
                    #[derive(Debug, Deserialize, PartialEq)]
                    struct List {
                        /// Outer list mapped to elements, inner -- to `xs:list`.
                        ///
                        /// `#[serde(default)]` is not required, because correct
                        /// XML will always contains at least 1 element.
                        item: [Vec<String>; 2],
                    }

                    let data: List = from_str(
                        r#"
                        <root>
                            <item>first list</item>
                            <item>second list</item>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        List {
                            item: [
                                vec!["first".to_string(), "list".to_string()],
                                vec!["second".to_string(), "list".to_string()],
                            ]
                        }
                    );
                }
            }
        }

        /// This module contains tests where size of the list have an unspecified size
        mod variable_size {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                item: Vec<()>,
            }

            /// Simple case: count of elements matches expected size of sequence,
            /// each element has the same name. Successful deserialization expected
            #[test]
            fn simple() {
                let data: List = from_str(
                    r#"
                    <root>
                        <item/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: vec![(), (), ()],
                    }
                );
            }

            /// Special case: empty sequence
            #[test]
            #[ignore = "it is impossible to distinguish between missed field and empty list: use `Option<>` or #[serde(default)]"]
            fn empty() {
                let data: List = from_str(r#"<root></root>"#).unwrap();
                assert_eq!(data, List { item: vec![] });

                let data: List = from_str(r#"<root/>"#).unwrap();
                assert_eq!(data, List { item: vec![] });
            }

            /// Special case: one-element sequence
            #[test]
            fn one_element() {
                let data: List = from_str(
                    r#"
                    <root>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(data, List { item: vec![()] });
            }

            /// Mixed content assumes, that some elements will have an internal
            /// name `$text` or `$value`, so, unless field named the same, it is expected
            /// to fail
            #[test]
            fn mixed_content() {
                let data = from_str::<List>(
                    r#"
                    <root>
                        <element/>
                        text
                        <![CDATA[cdata]]>
                    </root>
                    "#,
                );

                match data {
                    Err(DeError::Custom(e)) => assert_eq!(e, "missing field `item`"),
                    e => panic!(
                        r#"Expected `Err(Custom("missing field `item`"))`, but found {:?}"#,
                        e
                    ),
                }
            }

            /// In those tests sequence should be deserialized from the XML
            /// with additional elements that is not defined in the struct.
            /// That fields should be skipped during deserialization
            mod unknown_items {
                use super::*;
                use pretty_assertions::assert_eq;

                #[test]
                fn before() {
                    let data: List = from_str(
                        r#"
                        <root>
                            <unknown/>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        List {
                            item: vec![(), (), ()],
                        }
                    );
                }

                #[test]
                fn after() {
                    let data: List = from_str(
                        r#"
                        <root>
                            <item/>
                            <item/>
                            <item/>
                            <unknown/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        List {
                            item: vec![(), (), ()],
                        }
                    );
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<List>(
                        r#"
                        <root>
                            <item/>
                            <unknown/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        List {
                            item: vec![(), (), ()],
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `item`"),
                        e => panic!(
                            r#"Expected Err(Custom("duplicate field `item`")), got {:?}"#,
                            e
                        ),
                    }
                }

                /// Test for https://github.com/tafia/quick-xml/issues/435
                #[test]
                fn overlapped_with_nested_list() {
                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        outer: Vec<List>,
                    }

                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <outer><item/></outer>
                            <unknown/>
                            <outer><item/></outer>
                            <outer><item/></outer>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            outer: vec![
                                List { item: vec![()] },
                                List { item: vec![()] },
                                List { item: vec![()] },
                            ],
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `outer`"),
                        e => panic!(
                            r#"Expected Err(Custom("duplicate field `outer`")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests non-sequential field is defined in the struct
            /// before sequential, so it will be deserialized before the list.
            /// That struct should be deserialized from the XML where these
            /// fields comes in an arbitrary order
            mod field_before_list {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Default, Deserialize)]
                struct Root {
                    node: (),
                    item: Vec<()>,
                }

                #[test]
                fn before() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <node/>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            node: (),
                            item: vec![(), (), ()],
                        }
                    );
                }

                #[test]
                fn after() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <item/>
                            <item/>
                            <item/>
                            <node/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            node: (),
                            item: vec![(), (), ()],
                        }
                    );
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <item/>
                            <node/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            node: (),
                            item: vec![(), (), ()],
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "duplicate field `item`")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("duplicate field `item`")), got {:?}"#,
                            e
                        ),
                    }
                }

                /// Test for https://github.com/tafia/quick-xml/issues/435
                #[test]
                fn overlapped_with_nested_list() {
                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        node: (),
                        outer: Vec<List>,
                    }

                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <outer><item/></outer>
                            <node/>
                            <outer><item/></outer>
                            <outer><item/></outer>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            node: (),
                            outer: vec![
                                List { item: vec![()] },
                                List { item: vec![()] },
                                List { item: vec![()] },
                            ],
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `outer`"),
                        e => panic!(
                            r#"Expected Err(Custom("duplicate field `outer`")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests non-sequential field is defined in the struct
            /// after sequential, so it will be deserialized after the list.
            /// That struct should be deserialized from the XML where these
            /// fields comes in an arbitrary order
            mod field_after_list {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Default, Deserialize)]
                struct Root {
                    item: Vec<()>,
                    node: (),
                }

                #[test]
                fn before() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <node/>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            item: vec![(), (), ()],
                            node: (),
                        }
                    );
                }

                #[test]
                fn after() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <item/>
                            <item/>
                            <item/>
                            <node/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            item: vec![(), (), ()],
                            node: (),
                        }
                    );
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <item/>
                            <node/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            item: vec![(), (), ()],
                            node: (),
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "duplicate field `item`")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("duplicate field `item`")), got {:?}"#,
                            e
                        ),
                    }
                }

                /// Test for https://github.com/tafia/quick-xml/issues/435
                #[test]
                fn overlapped_with_nested_list() {
                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        outer: Vec<List>,
                        node: (),
                    }

                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <outer><item/></outer>
                            <node/>
                            <outer><item/></outer>
                            <outer><item/></outer>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            outer: vec![
                                List { item: vec![()] },
                                List { item: vec![()] },
                                List { item: vec![()] },
                            ],
                            node: (),
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `outer`"),
                        e => panic!(
                            r#"Expected Err(Custom("duplicate field `outer`")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests two lists are deserialized simultaneously.
            /// Lists should be deserialized even when them overlaps
            mod two_lists {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Pair {
                    item: Vec<()>,
                    element: Vec<()>,
                }

                #[test]
                fn splitted() {
                    let data: Pair = from_str(
                        r#"
                        <root>
                            <element/>
                            <element/>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Pair {
                            item: vec![(), (), ()],
                            element: vec![(), ()],
                        }
                    );
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Pair>(
                        r#"
                        <root>
                            <item/>
                            <element/>
                            <item/>
                            <element/>
                            <item/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Pair {
                            item: vec![(), (), ()],
                            element: vec![(), ()],
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `item`"),
                        e => panic!(
                            r#"Expected Err(Custom("duplicate field `item`")), got {:?}"#,
                            e
                        ),
                    }
                }

                #[test]
                fn overlapped_with_nested_list() {
                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Pair {
                        outer: Vec<List>,
                        element: Vec<()>,
                    }

                    let data = from_str::<Pair>(
                        r#"
                        <root>
                            <outer><item/></outer>
                            <element/>
                            <outer><item/></outer>
                            <outer><item/></outer>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Pair {
                            outer: vec![
                                List { item: vec![()] },
                                List { item: vec![()] },
                                List { item: vec![()] },
                            ],
                            element: vec![()],
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `outer`"),
                        e => panic!(
                            r#"Expected Err(Custom("duplicate field `outer`")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// Deserialization of primitives slightly differs from deserialization
            /// of complex types, so need to check this separately
            #[test]
            fn primitives() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    item: Vec<usize>,
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <item>41</item>
                        <item>42</item>
                        <item>43</item>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: vec![41, 42, 43],
                    }
                );

                from_str::<List>(
                    r#"
                    <root>
                        <item>41</item>
                        <item><item>42</item></item>
                        <item>43</item>
                    </root>
                    "#,
                )
                .unwrap_err();
            }

            /// This test ensures that composition of deserializer building blocks
            /// plays well
            #[test]
            fn list_of_struct() {
                #[derive(Debug, PartialEq, Default, Deserialize)]
                #[serde(default)]
                struct Struct {
                    #[serde(rename = "@attribute")]
                    attribute: Option<String>,
                    element: Option<String>,
                }

                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    item: Vec<Struct>,
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <item/>
                        <item attribute="value"/>
                        <item>
                            <element>value</element>
                        </item>
                        <item attribute="value">
                            <element>value</element>
                        </item>
                    </root>
                    "#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    List {
                        item: vec![
                            Struct {
                                attribute: None,
                                element: None,
                            },
                            Struct {
                                attribute: Some("value".to_string()),
                                element: None,
                            },
                            Struct {
                                attribute: None,
                                element: Some("value".to_string()),
                            },
                            Struct {
                                attribute: Some("value".to_string()),
                                element: Some("value".to_string()),
                            },
                        ],
                    }
                );
            }

            /// Checks that sequences represented by elements can contain sequences,
            /// represented by `xs:list`s
            mod xs_list {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, Deserialize, PartialEq)]
                struct List {
                    /// Outer list mapped to elements, inner -- to `xs:list`.
                    ///
                    /// `#[serde(default)]` is required to correctly deserialize
                    /// empty sequence, because without elements the field
                    /// also is missing and derived `Deserialize` implementation
                    /// would complain about that unless field is marked as
                    /// `default`.
                    #[serde(default)]
                    item: Vec<Vec<String>>,
                }

                /// Special case: zero elements
                #[test]
                fn zero() {
                    let data: List = from_str(
                        r#"
                        <root>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(data, List { item: vec![] });
                }

                /// Special case: one element
                #[test]
                fn one() {
                    let data: List = from_str(
                        r#"
                        <root>
                            <item>first list</item>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        List {
                            item: vec![vec!["first".to_string(), "list".to_string()]]
                        }
                    );
                }

                /// Special case: outer list is always mapped to an elements sequence,
                /// not to an `xs:list`
                #[test]
                fn element() {
                    #[derive(Debug, Deserialize, PartialEq)]
                    struct List {
                        /// List mapped to elements, inner -- to `xs:list`.
                        ///
                        /// `#[serde(default)]` is not required, because correct
                        /// XML will always contains at least 1 element.
                        item: Vec<String>,
                    }

                    let data: List = from_str(
                        r#"
                        <root>
                            <item>first item</item>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        List {
                            item: vec!["first item".to_string()]
                        }
                    );
                }

                /// This tests demonstrates, that for `$value` field (`list`) actual
                /// name of XML element (`item`) does not matter. That allows list
                /// item to be an enum, where tag name determines enum variant
                #[test]
                fn many() {
                    let data: List = from_str(
                        r#"
                        <root>
                            <item>first list</item>
                            <item>second list</item>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        List {
                            item: vec![
                                vec!["first".to_string(), "list".to_string()],
                                vec!["second".to_string(), "list".to_string()],
                            ]
                        }
                    );
                }
            }
        }
    }

    /// Check that sequences inside element can be deserialized.
    /// In terms of serde this is a sequence flatten into the struct:
    ///
    /// ```ignore
    /// struct Root {
    ///   #[serde(flatten)]
    ///   items: Vec<T>,
    /// }
    /// ```
    /// except that fact that this is not supported nowadays
    /// (https://github.com/serde-rs/serde/issues/1905)
    ///
    /// Because this is very frequently used pattern in the XML, quick-xml
    /// have a workaround for this. If a field will have a special name `$value`
    /// then any `xs:element`s in the `xs:sequence` / `xs:all`, except that
    /// which name matches the struct name, will be associated with this field:
    ///
    /// ```ignore
    /// struct Root {
    ///   field: U,
    ///   #[serde(rename = "$value")]
    ///   items: Vec<Enum>,
    /// }
    /// ```
    /// In this example `<field>` tag will be associated with a `field` field,
    /// but all other tags will be associated with an `items` field. Disadvantages
    /// of this approach that you can have only one field, but usually you don't
    /// want more
    mod variable_name {
        use super::*;
        use serde::de::{Deserializer, EnumAccess, VariantAccess, Visitor};
        use std::fmt::{self, Formatter};

        // NOTE: Derive could be possible once https://github.com/serde-rs/serde/issues/2126 is resolved
        macro_rules! impl_deserialize_choice {
            ($name:ident : $(($field:ident, $field_name:literal)),*) => {
                impl<'de> Deserialize<'de> for $name {
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: Deserializer<'de>,
                    {
                        #[derive(Deserialize)]
                        #[serde(field_identifier)]
                        #[serde(rename_all = "kebab-case")]
                        enum Tag {
                            $($field,)*
                            Other(String),
                        }

                        struct EnumVisitor;
                        impl<'de> Visitor<'de> for EnumVisitor {
                            type Value = $name;

                            fn expecting(&self, f: &mut Formatter) -> fmt::Result {
                                f.write_str("enum ")?;
                                f.write_str(stringify!($name))
                            }

                            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
                            where
                                A: EnumAccess<'de>,
                            {
                                match data.variant()? {
                                    $(
                                        (Tag::$field, variant) => variant.unit_variant().map(|_| $name::$field),
                                    )*
                                    (Tag::Other(t), v) => v.unit_variant().map(|_| $name::Other(t)),
                                }
                            }
                        }

                        const VARIANTS: &'static [&'static str] = &[
                            $($field_name,)*
                            "<any other tag>"
                        ];
                        deserializer.deserialize_enum(stringify!($name), VARIANTS, EnumVisitor)
                    }
                }
            };
        }

        /// Type that can be deserialized from `<one>`, `<two>`, or any other element
        #[derive(Debug, PartialEq)]
        enum Choice {
            One,
            Two,
            /// Any other tag name except `One` or `Two`, name of tag stored inside variant
            Other(String),
        }
        impl_deserialize_choice!(Choice: (One, "one"), (Two, "two"));

        /// Type that can be deserialized from `<first>`, `<second>`, or any other element
        #[derive(Debug, PartialEq)]
        enum Choice2 {
            First,
            Second,
            /// Any other tag name except `First` or `Second`, name of tag stored inside variant
            Other(String),
        }
        impl_deserialize_choice!(Choice2: (First, "first"), (Second, "second"));

        /// Type that can be deserialized from `<one>`, `<two>`, or any other element.
        /// Used for `primitives` tests
        #[derive(Debug, PartialEq, Deserialize)]
        #[serde(rename_all = "kebab-case")]
        enum Choice3 {
            One(usize),
            Two(String),
            #[serde(other)]
            Other,
        }

        #[derive(Debug, PartialEq, Deserialize)]
        #[serde(rename_all = "kebab-case")]
        enum Choice4 {
            One {
                inner: [(); 1],
            },
            Two {
                inner: [(); 1],
            },
            #[serde(other)]
            Other,
        }

        /// This module contains tests where size of the list have a compile-time size
        mod fixed_size {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                #[serde(rename = "$value")]
                item: [Choice; 3],
            }

            /// Simple case: count of elements matches expected size of sequence,
            /// each element has the same name. Successful deserialization expected
            #[test]
            fn simple() {
                let data: List = from_str(
                    r#"
                    <root>
                        <one/>
                        <two/>
                        <three/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                    }
                );
            }

            /// Special case: empty sequence
            #[test]
            #[ignore = "it is impossible to distinguish between missed field and empty list: use `Option<>` or #[serde(default)]"]
            fn empty() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    #[serde(rename = "$value")]
                    item: [Choice; 0],
                }

                from_str::<List>(r#"<root></root>"#).unwrap();
                from_str::<List>(r#"<root/>"#).unwrap();
            }

            /// Special case: one-element sequence
            #[test]
            fn one_element() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    #[serde(rename = "$value")]
                    item: [Choice; 1],
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <one/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: [Choice::One],
                    }
                );
            }

            /// Fever elements than expected size of sequence, each element has
            /// the same name. Failure expected
            #[test]
            fn fever_elements() {
                from_str::<List>(
                    r#"
                    <root>
                        <one/>
                        <two/>
                    </root>
                    "#,
                )
                .unwrap_err();
            }

            /// More elements than expected size of sequence, each element has
            /// the same name. Failure expected. If you wish to ignore excess
            /// elements, use the special type, that consume as much elements
            /// as possible, but ignores excess elements
            #[test]
            fn excess_elements() {
                from_str::<List>(
                    r#"
                    <root>
                        <one/>
                        <two/>
                        <three/>
                        <four/>
                    </root>
                    "#,
                )
                .unwrap_err();
            }

            #[test]
            fn mixed_content() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    #[serde(rename = "$value")]
                    item: [(); 3],
                }

                from_str::<List>(
                    r#"
                    <root>
                        <element/>
                        text
                        <![CDATA[cdata]]>
                    </root>
                    "#,
                )
                .unwrap();
            }

            // There cannot be unknown items, because any tag name is accepted

            /// In those tests non-sequential field is defined in the struct
            /// before sequential, so it will be deserialized before the list.
            /// That struct should be deserialized from the XML where these
            /// fields comes in an arbitrary order
            mod field_before_list {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    node: (),
                    #[serde(rename = "$value")]
                    item: [Choice; 3],
                }

                #[test]
                fn before() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <node/>
                            <one/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            node: (),
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                        }
                    );
                }

                #[test]
                fn after() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <one/>
                            <two/>
                            <three/>
                            <node/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            node: (),
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                        }
                    );
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <one/>
                            <node/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            node: (),
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                            e
                        ),
                    }
                }

                /// Test for https://github.com/tafia/quick-xml/issues/435
                #[test]
                fn overlapped_with_nested_list() {
                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        node: (),
                        #[serde(rename = "$value")]
                        item: [Choice4; 3],
                    }

                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <one><inner/></one>
                            <node/>
                            <two><inner/></two>
                            <three><inner/></three>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            node: (),
                            item: [
                                Choice4::One { inner: [()] },
                                Choice4::Two { inner: [()] },
                                Choice4::Other,
                            ],
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests non-sequential field is defined in the struct
            /// after sequential, so it will be deserialized after the list.
            /// That struct should be deserialized from the XML where these
            /// fields comes in an arbitrary order
            mod field_after_list {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    #[serde(rename = "$value")]
                    item: [Choice; 3],
                    node: (),
                }

                #[test]
                fn before() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <node/>
                            <one/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                            node: (),
                        }
                    );
                }

                #[test]
                fn after() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <one/>
                            <two/>
                            <three/>
                            <node/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                            node: (),
                        }
                    );
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <one/>
                            <node/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                            node: (),
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                            e
                        ),
                    }
                }

                /// Test for https://github.com/tafia/quick-xml/issues/435
                #[test]
                fn overlapped_with_nested_list() {
                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        #[serde(rename = "$value")]
                        item: [Choice4; 3],
                        node: (),
                    }

                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <one><inner/></one>
                            <node/>
                            <two><inner/></two>
                            <three><inner/></three>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            item: [
                                Choice4::One { inner: [()] },
                                Choice4::Two { inner: [()] },
                                Choice4::Other,
                            ],
                            node: (),
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests two lists are deserialized simultaneously.
            /// Lists should be deserialized even when them overlaps
            mod two_lists {
                use super::*;

                /// A field with a variable-name items defined before a field with a fixed-name
                /// items
                mod choice_and_fixed {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Pair {
                        #[serde(rename = "$value")]
                        item: [Choice; 3],
                        element: [(); 2],
                    }

                    /// A list with fixed-name elements located before a list with variable-name
                    /// elements in an XML
                    #[test]
                    fn fixed_before() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <element/>
                                <element/>
                                <one/>
                                <two/>
                                <three/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [(), ()],
                            }
                        );
                    }

                    /// A list with fixed-name elements located after a list with variable-name
                    /// elements in an XML
                    #[test]
                    fn fixed_after() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <one/>
                                <two/>
                                <three/>
                                <element/>
                                <element/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [(), ()],
                            }
                        );
                    }

                    mod overlapped {
                        use super::*;
                        use pretty_assertions::assert_eq;

                        #[derive(Debug, PartialEq, Deserialize)]
                        struct Root {
                            #[serde(rename = "$value")]
                            item: [Choice4; 3],
                            element: [(); 2],
                        }

                        /// A list with fixed-name elements are mixed with a list with variable-name
                        /// elements in an XML, and the first element is a fixed-name one
                        #[test]
                        fn fixed_before() {
                            let data = from_str::<Pair>(
                                r#"
                                <root>
                                    <element/>
                                    <one/>
                                    <two/>
                                    <element/>
                                    <three/>
                                </root>
                                "#,
                            );

                            #[cfg(feature = "overlapped-lists")]
                            assert_eq!(
                                data.unwrap(),
                                Pair {
                                    item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                    element: [(), ()],
                                }
                            );

                            #[cfg(not(feature = "overlapped-lists"))]
                            match data {
                                Err(DeError::Custom(e)) => {
                                    assert_eq!(e, "invalid length 1, expected an array of length 2")
                                }
                                e => panic!(
                                    r#"Expected Err(Custom("invalid length 1, expected an array of length 2")), got {:?}"#,
                                    e
                                ),
                            }
                        }

                        /// A list with fixed-name elements are mixed with a list with variable-name
                        /// elements in an XML, and the first element is a variable-name one
                        #[test]
                        fn fixed_after() {
                            let data = from_str::<Pair>(
                                r#"
                                <root>
                                    <one/>
                                    <element/>
                                    <two/>
                                    <three/>
                                    <element/>
                                </root>
                                "#,
                            );

                            #[cfg(feature = "overlapped-lists")]
                            assert_eq!(
                                data.unwrap(),
                                Pair {
                                    item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                    element: [(), ()],
                                }
                            );

                            #[cfg(not(feature = "overlapped-lists"))]
                            match data {
                                Err(DeError::Custom(e)) => {
                                    assert_eq!(e, "invalid length 1, expected an array of length 3")
                                }
                                e => panic!(
                                    r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                                    e
                                ),
                            }
                        }

                        /// Test for https://github.com/tafia/quick-xml/issues/435
                        #[test]
                        fn with_nested_list_fixed_before() {
                            let data = from_str::<Root>(
                                r#"
                                <root>
                                    <element/>
                                    <one><inner/></one>
                                    <two><inner/></two>
                                    <element/>
                                    <three><inner/></three>
                                </root>
                                "#,
                            );

                            #[cfg(feature = "overlapped-lists")]
                            assert_eq!(
                                data.unwrap(),
                                Root {
                                    item: [
                                        Choice4::One { inner: [()] },
                                        Choice4::Two { inner: [()] },
                                        Choice4::Other,
                                    ],
                                    element: [(); 2],
                                }
                            );

                            #[cfg(not(feature = "overlapped-lists"))]
                            match data {
                                Err(DeError::Custom(e)) => {
                                    assert_eq!(e, "invalid length 1, expected an array of length 2")
                                }
                                e => panic!(
                                    r#"Expected Err(Custom("invalid length 1, expected an array of length 2")), got {:?}"#,
                                    e
                                ),
                            }
                        }

                        /// Test for https://github.com/tafia/quick-xml/issues/435
                        #[test]
                        fn with_nested_list_fixed_after() {
                            let data = from_str::<Root>(
                                r#"
                                <root>
                                    <one><inner/></one>
                                    <element/>
                                    <two><inner/></two>
                                    <three><inner/></three>
                                    <element/>
                                </root>
                                "#,
                            );

                            #[cfg(feature = "overlapped-lists")]
                            assert_eq!(
                                data.unwrap(),
                                Root {
                                    item: [
                                        Choice4::One { inner: [()] },
                                        Choice4::Two { inner: [()] },
                                        Choice4::Other,
                                    ],
                                    element: [(); 2],
                                }
                            );

                            #[cfg(not(feature = "overlapped-lists"))]
                            match data {
                                Err(DeError::Custom(e)) => {
                                    assert_eq!(e, "invalid length 1, expected an array of length 3")
                                }
                                e => panic!(
                                    r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                                    e
                                ),
                            }
                        }
                    }
                }

                /// A field with a variable-name items defined after a field with a fixed-name
                /// items
                mod fixed_and_choice {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Pair {
                        element: [(); 2],
                        #[serde(rename = "$value")]
                        item: [Choice; 3],
                    }

                    /// A list with fixed-name elements located before a list with variable-name
                    /// elements in an XML
                    #[test]
                    fn fixed_before() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <element/>
                                <element/>
                                <one/>
                                <two/>
                                <three/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [(), ()],
                            }
                        );
                    }

                    /// A list with fixed-name elements located after a list with variable-name
                    /// elements in an XML
                    #[test]
                    fn fixed_after() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <one/>
                                <two/>
                                <three/>
                                <element/>
                                <element/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [(), ()],
                            }
                        );
                    }

                    mod overlapped {
                        use super::*;
                        use pretty_assertions::assert_eq;

                        #[derive(Debug, PartialEq, Deserialize)]
                        struct Root {
                            element: [(); 2],
                            #[serde(rename = "$value")]
                            item: [Choice4; 3],
                        }

                        /// A list with fixed-name elements are mixed with a list with variable-name
                        /// elements in an XML, and the first element is a fixed-name one
                        #[test]
                        fn fixed_before() {
                            let data = from_str::<Pair>(
                                r#"
                                <root>
                                    <element/>
                                    <one/>
                                    <two/>
                                    <element/>
                                    <three/>
                                </root>
                                "#,
                            );

                            #[cfg(feature = "overlapped-lists")]
                            assert_eq!(
                                data.unwrap(),
                                Pair {
                                    item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                    element: [(), ()],
                                }
                            );

                            #[cfg(not(feature = "overlapped-lists"))]
                            match data {
                                Err(DeError::Custom(e)) => {
                                    assert_eq!(e, "invalid length 1, expected an array of length 2")
                                }
                                e => panic!(
                                    r#"Expected Err(Custom("invalid length 1, expected an array of length 2")), got {:?}"#,
                                    e
                                ),
                            }
                        }

                        /// A list with fixed-name elements are mixed with a list with variable-name
                        /// elements in an XML, and the first element is a variable-name one
                        #[test]
                        fn fixed_after() {
                            let data = from_str::<Pair>(
                                r#"
                                <root>
                                    <one/>
                                    <element/>
                                    <two/>
                                    <three/>
                                    <element/>
                                </root>
                                "#,
                            );

                            #[cfg(feature = "overlapped-lists")]
                            assert_eq!(
                                data.unwrap(),
                                Pair {
                                    item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                    element: [(), ()],
                                }
                            );

                            #[cfg(not(feature = "overlapped-lists"))]
                            match data {
                                Err(DeError::Custom(e)) => {
                                    assert_eq!(e, "invalid length 1, expected an array of length 3")
                                }
                                e => panic!(
                                    r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                                    e
                                ),
                            }
                        }

                        /// Test for https://github.com/tafia/quick-xml/issues/435
                        #[test]
                        fn with_nested_list_fixed_before() {
                            let data = from_str::<Root>(
                                r#"
                                <root>
                                    <element/>
                                    <one><inner/></one>
                                    <two><inner/></two>
                                    <element/>
                                    <three><inner/></three>
                                </root>
                                "#,
                            );

                            #[cfg(feature = "overlapped-lists")]
                            assert_eq!(
                                data.unwrap(),
                                Root {
                                    element: [(); 2],
                                    item: [
                                        Choice4::One { inner: [()] },
                                        Choice4::Two { inner: [()] },
                                        Choice4::Other,
                                    ],
                                }
                            );

                            #[cfg(not(feature = "overlapped-lists"))]
                            match data {
                                Err(DeError::Custom(e)) => {
                                    assert_eq!(e, "invalid length 1, expected an array of length 2")
                                }
                                e => panic!(
                                    r#"Expected Err(Custom("invalid length 1, expected an array of length 2")), got {:?}"#,
                                    e
                                ),
                            }
                        }

                        /// Test for https://github.com/tafia/quick-xml/issues/435
                        #[test]
                        fn with_nested_list_fixed_after() {
                            let data = from_str::<Root>(
                                r#"
                                <root>
                                    <one><inner/></one>
                                    <element/>
                                    <two><inner/></two>
                                    <three><inner/></three>
                                    <element/>
                                </root>
                                "#,
                            );

                            #[cfg(feature = "overlapped-lists")]
                            assert_eq!(
                                data.unwrap(),
                                Root {
                                    element: [(); 2],
                                    item: [
                                        Choice4::One { inner: [()] },
                                        Choice4::Two { inner: [()] },
                                        Choice4::Other,
                                    ],
                                }
                            );

                            #[cfg(not(feature = "overlapped-lists"))]
                            match data {
                                Err(DeError::Custom(e)) => {
                                    assert_eq!(e, "invalid length 1, expected an array of length 3")
                                }
                                e => panic!(
                                    r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                                    e
                                ),
                            }
                        }
                    }
                }

                /// Tests are ignored, but exists to show a problem.
                /// May be it will be solved in the future
                mod choice_and_choice {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Pair {
                        #[serde(rename = "$value")]
                        item: [Choice; 3],
                        // Actually, we cannot rename both fields to `$value`, which is now
                        // required to indicate, that field accepts elements with any name
                        #[serde(rename = "$value")]
                        element: [Choice2; 2],
                    }

                    #[test]
                    #[ignore = "There is no way to associate XML elements with `item` or `element` without extra knowledge from type"]
                    fn splitted() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <first/>
                                <second/>
                                <one/>
                                <two/>
                                <three/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [Choice2::First, Choice2::Second],
                            }
                        );
                    }

                    #[test]
                    #[ignore = "There is no way to associate XML elements with `item` or `element` without extra knowledge from type"]
                    fn overlapped() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <one/>
                                <first/>
                                <two/>
                                <second/>
                                <three/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [Choice2::First, Choice2::Second],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "invalid length 1, expected an array of length 3")
                            }
                            e => panic!(
                                r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                                e
                            ),
                        }
                    }
                }
            }

            /// Deserialization of primitives slightly differs from deserialization
            /// of complex types, so need to check this separately
            #[test]
            fn primitives() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    #[serde(rename = "$value")]
                    item: [Choice3; 3],
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <one>41</one>
                        <two>42</two>
                        <three>43</three>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: [
                            Choice3::One(41),
                            Choice3::Two("42".to_string()),
                            Choice3::Other,
                        ],
                    }
                );

                from_str::<List>(
                    r#"
                    <root>
                        <one>41</one>
                        <two><item>42</item></two>
                        <three>43</three>
                    </root>
                    "#,
                )
                .unwrap_err();
            }

            /// Checks that sequences represented by elements can contain sequences,
            /// represented by `xs:list`s
            mod xs_list {
                use super::*;
                use pretty_assertions::assert_eq;

                /// Special case: zero elements
                #[test]
                fn zero() {
                    #[derive(Debug, Deserialize, PartialEq)]
                    struct List {
                        /// Outer list mapped to elements, inner -- to `xs:list`.
                        ///
                        /// `#[serde(default)]` is required to correctly deserialize
                        /// empty sequence, because without elements the field
                        /// also is missing and derived `Deserialize` implementation
                        /// would complain about that unless field is marked as
                        /// `default`.
                        #[serde(default)]
                        #[serde(rename = "$value")]
                        element: [Vec<String>; 0],
                    }

                    let data: List = from_str(
                        r#"
                        <root>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(data, List { element: [] });
                }

                /// Special case: one element
                #[test]
                fn one() {
                    #[derive(Debug, Deserialize, PartialEq)]
                    struct List {
                        /// Outer list mapped to elements, inner -- to `xs:list`.
                        ///
                        /// `#[serde(default)]` is not required, because correct
                        /// XML will always contains at least 1 element.
                        #[serde(rename = "$value")]
                        element: [Vec<String>; 1],
                    }

                    let data: List = from_str(
                        r#"
                        <root>
                            <item>first list</item>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        List {
                            element: [vec!["first".to_string(), "list".to_string()]]
                        }
                    );
                }

                /// Special case: outer list is always mapped to an elements sequence,
                /// not to an `xs:list`
                #[test]
                fn element() {
                    #[derive(Debug, Deserialize, PartialEq)]
                    struct List {
                        /// List mapped to elements, String -- to `xs:list`.
                        ///
                        /// `#[serde(default)]` is not required, because correct
                        /// XML will always contains at least 1 element.
                        #[serde(rename = "$value")]
                        element: [String; 1],
                    }

                    let data: List = from_str(
                        r#"
                        <root>
                            <item>first item</item>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        List {
                            element: ["first item".to_string()]
                        }
                    );
                }

                #[test]
                fn many() {
                    #[derive(Debug, Deserialize, PartialEq)]
                    struct List {
                        /// Outer list mapped to elements, inner -- to `xs:list`
                        #[serde(rename = "$value")]
                        element: [Vec<String>; 2],
                    }

                    let data: List = from_str(
                        r#"
                        <root>
                            <item>first list</item>
                            <item>second list</item>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        List {
                            element: [
                                vec!["first".to_string(), "list".to_string()],
                                vec!["second".to_string(), "list".to_string()],
                            ]
                        }
                    );
                }
            }
        }

        /// This module contains tests where size of the list have an unspecified size
        mod variable_size {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                #[serde(rename = "$value")]
                item: Vec<Choice>,
            }

            /// Simple case: count of elements matches expected size of sequence,
            /// each element has the same name. Successful deserialization expected
            #[test]
            fn simple() {
                let data: List = from_str(
                    r#"
                    <root>
                        <one/>
                        <two/>
                        <three/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                    }
                );
            }

            /// Special case: empty sequence
            #[test]
            #[ignore = "it is impossible to distinguish between missed field and empty list: use `Option<>` or #[serde(default)]"]
            fn empty() {
                let data = from_str::<List>(r#"<root></root>"#).unwrap();
                assert_eq!(data, List { item: vec![] });

                let data = from_str::<List>(r#"<root/>"#).unwrap();
                assert_eq!(data, List { item: vec![] });
            }

            /// Special case: one-element sequence
            #[test]
            fn one_element() {
                let data: List = from_str(
                    r#"
                    <root>
                        <one/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: vec![Choice::One],
                    }
                );
            }

            #[test]
            fn mixed_content() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    #[serde(rename = "$value")]
                    item: Vec<()>,
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <element/>
                        text
                        <![CDATA[cdata]]>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: vec![(), (), ()],
                    }
                );
            }

            // There cannot be unknown items, because any tag name is accepted

            /// In those tests non-sequential field is defined in the struct
            /// before sequential, so it will be deserialized before the list.
            /// That struct should be deserialized from the XML where these
            /// fields comes in an arbitrary order
            mod field_before_list {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    node: (),
                    #[serde(rename = "$value")]
                    item: Vec<Choice>,
                }

                #[test]
                fn before() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <node/>
                            <one/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            node: (),
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                        }
                    );
                }

                #[test]
                fn after() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <one/>
                            <two/>
                            <three/>
                            <node/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            node: (),
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                        }
                    );
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <one/>
                            <node/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            node: (),
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `$value`"),
                        e => panic!(
                            r#"Expected Err(Custom("duplicate field `$value`")), got {:?}"#,
                            e
                        ),
                    }
                }

                /// Test for https://github.com/tafia/quick-xml/issues/435
                #[test]
                fn overlapped_with_nested_list() {
                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        node: (),
                        #[serde(rename = "$value")]
                        item: Vec<Choice4>,
                    }

                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <one><inner/></one>
                            <node/>
                            <two><inner/></two>
                            <three><inner/></three>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            node: (),
                            item: vec![
                                Choice4::One { inner: [()] },
                                Choice4::Two { inner: [()] },
                                Choice4::Other,
                            ],
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "duplicate field `$value`")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("duplicate field `$value`")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests non-sequential field is defined in the struct
            /// after sequential, so it will be deserialized after the list.
            /// That struct should be deserialized from the XML where these
            /// fields comes in an arbitrary order
            mod field_after_list {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    #[serde(rename = "$value")]
                    item: Vec<Choice>,
                    node: (),
                }

                #[test]
                fn before() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <node/>
                            <one/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            node: (),
                        }
                    );
                }

                #[test]
                fn after() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <one/>
                            <two/>
                            <three/>
                            <node/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            node: (),
                        }
                    );
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <one/>
                            <node/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            node: (),
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `$value`"),
                        e => panic!(
                            r#"Expected Err(Custom("duplicate field `$value`")), got {:?}"#,
                            e
                        ),
                    }
                }

                /// Test for https://github.com/tafia/quick-xml/issues/435
                #[test]
                fn overlapped_with_nested_list() {
                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        #[serde(rename = "$value")]
                        item: Vec<Choice4>,
                        node: (),
                    }

                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <one><inner/></one>
                            <node/>
                            <two><inner/></two>
                            <three><inner/></three>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            item: vec![
                                Choice4::One { inner: [()] },
                                Choice4::Two { inner: [()] },
                                Choice4::Other,
                            ],
                            node: (),
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "duplicate field `$value`")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("duplicate field `$value`")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests two lists are deserialized simultaneously.
            /// Lists should be deserialized even when them overlaps
            mod two_lists {
                use super::*;

                /// A field with a variable-name items defined before a field with a fixed-name
                /// items
                mod choice_and_fixed {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Pair {
                        #[serde(rename = "$value")]
                        item: Vec<Choice>,
                        element: Vec<()>,
                    }

                    /// A list with fixed-name elements located before a list with variable-name
                    /// elements in an XML
                    #[test]
                    fn fixed_before() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <element/>
                                <element/>
                                <one/>
                                <two/>
                                <three/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: vec![(), ()],
                            }
                        );
                    }

                    /// A list with fixed-name elements located after a list with variable-name
                    /// elements in an XML
                    #[test]
                    fn fixed_after() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <one/>
                                <two/>
                                <three/>
                                <element/>
                                <element/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: vec![(), ()],
                            }
                        );
                    }

                    mod overlapped {
                        use super::*;
                        use pretty_assertions::assert_eq;

                        #[derive(Debug, PartialEq, Deserialize)]
                        struct Root {
                            #[serde(rename = "$value")]
                            item: Vec<Choice4>,
                            element: Vec<()>,
                        }

                        /// A list with fixed-name elements are mixed with a list with variable-name
                        /// elements in an XML, and the first element is a fixed-name one
                        #[test]
                        fn fixed_before() {
                            let data = from_str::<Pair>(
                                r#"
                                <root>
                                    <element/>
                                    <one/>
                                    <two/>
                                    <element/>
                                    <three/>
                                </root>
                                "#,
                            );

                            #[cfg(feature = "overlapped-lists")]
                            assert_eq!(
                                data.unwrap(),
                                Pair {
                                    item: vec![
                                        Choice::One,
                                        Choice::Two,
                                        Choice::Other("three".into())
                                    ],
                                    element: vec![(), ()],
                                }
                            );

                            #[cfg(not(feature = "overlapped-lists"))]
                            match data {
                                Err(DeError::Custom(e)) => {
                                    assert_eq!(e, "duplicate field `element`")
                                }
                                e => panic!(
                                    r#"Expected Err(Custom("duplicate field `element`")), got {:?}"#,
                                    e
                                ),
                            }
                        }

                        /// A list with fixed-name elements are mixed with a list with variable-name
                        /// elements in an XML, and the first element is a variable-name one
                        #[test]
                        fn fixed_after() {
                            let data = from_str::<Pair>(
                                r#"
                                <root>
                                    <one/>
                                    <element/>
                                    <two/>
                                    <three/>
                                    <element/>
                                </root>
                                "#,
                            );

                            #[cfg(feature = "overlapped-lists")]
                            assert_eq!(
                                data.unwrap(),
                                Pair {
                                    item: vec![
                                        Choice::One,
                                        Choice::Two,
                                        Choice::Other("three".into())
                                    ],
                                    element: vec![(), ()],
                                }
                            );

                            #[cfg(not(feature = "overlapped-lists"))]
                            match data {
                                Err(DeError::Custom(e)) => {
                                    assert_eq!(e, "duplicate field `$value`")
                                }
                                e => panic!(
                                    r#"Expected Err(Custom("duplicate field `$value`")), got {:?}"#,
                                    e
                                ),
                            }
                        }

                        /// Test for https://github.com/tafia/quick-xml/issues/435
                        #[test]
                        fn with_nested_list_fixed_before() {
                            let data = from_str::<Root>(
                                r#"
                                <root>
                                    <element/>
                                    <one><inner/></one>
                                    <two><inner/></two>
                                    <element/>
                                    <three><inner/></three>
                                </root>
                                "#,
                            );

                            #[cfg(feature = "overlapped-lists")]
                            assert_eq!(
                                data.unwrap(),
                                Root {
                                    item: vec![
                                        Choice4::One { inner: [()] },
                                        Choice4::Two { inner: [()] },
                                        Choice4::Other,
                                    ],
                                    element: vec![(); 2],
                                }
                            );

                            #[cfg(not(feature = "overlapped-lists"))]
                            match data {
                                Err(DeError::Custom(e)) => {
                                    assert_eq!(e, "duplicate field `element`")
                                }
                                e => panic!(
                                    r#"Expected Err(Custom("duplicate field `element`")), got {:?}"#,
                                    e
                                ),
                            }
                        }

                        /// Test for https://github.com/tafia/quick-xml/issues/435
                        #[test]
                        fn with_nested_list_fixed_after() {
                            let data = from_str::<Root>(
                                r#"
                                <root>
                                    <one><inner/></one>
                                    <element/>
                                    <two><inner/></two>
                                    <three><inner/></three>
                                    <element/>
                                </root>
                                "#,
                            );

                            #[cfg(feature = "overlapped-lists")]
                            assert_eq!(
                                data.unwrap(),
                                Root {
                                    item: vec![
                                        Choice4::One { inner: [()] },
                                        Choice4::Two { inner: [()] },
                                        Choice4::Other,
                                    ],
                                    element: vec![(); 2],
                                }
                            );

                            #[cfg(not(feature = "overlapped-lists"))]
                            match data {
                                Err(DeError::Custom(e)) => {
                                    assert_eq!(e, "duplicate field `$value`")
                                }
                                e => panic!(
                                    r#"Expected Err(Custom("duplicate field `$value`")), got {:?}"#,
                                    e
                                ),
                            }
                        }
                    }
                }

                /// A field with a variable-name items defined after a field with a fixed-name
                /// items
                mod fixed_and_choice {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Pair {
                        element: Vec<()>,
                        #[serde(rename = "$value")]
                        item: Vec<Choice>,
                    }

                    /// A list with fixed-name elements located before a list with variable-name
                    /// elements in an XML
                    #[test]
                    fn fixed_before() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <element/>
                                <element/>
                                <one/>
                                <two/>
                                <three/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                element: vec![(), ()],
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            }
                        );
                    }

                    /// A list with fixed-name elements located after a list with variable-name
                    /// elements in an XML
                    #[test]
                    fn fixed_after() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <one/>
                                <two/>
                                <three/>
                                <element/>
                                <element/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                element: vec![(), ()],
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            }
                        );
                    }

                    mod overlapped {
                        use super::*;
                        use pretty_assertions::assert_eq;

                        #[derive(Debug, PartialEq, Deserialize)]
                        struct Root {
                            element: Vec<()>,
                            #[serde(rename = "$value")]
                            item: Vec<Choice4>,
                        }

                        /// A list with fixed-name elements are mixed with a list with variable-name
                        /// elements in an XML, and the first element is a fixed-name one
                        #[test]
                        fn fixed_before() {
                            let data = from_str::<Pair>(
                                r#"
                                <root>
                                    <element/>
                                    <one/>
                                    <two/>
                                    <element/>
                                    <three/>
                                </root>
                                "#,
                            );

                            #[cfg(feature = "overlapped-lists")]
                            assert_eq!(
                                data.unwrap(),
                                Pair {
                                    element: vec![(), ()],
                                    item: vec![
                                        Choice::One,
                                        Choice::Two,
                                        Choice::Other("three".into())
                                    ],
                                }
                            );

                            #[cfg(not(feature = "overlapped-lists"))]
                            match data {
                                Err(DeError::Custom(e)) => {
                                    assert_eq!(e, "duplicate field `element`")
                                }
                                e => panic!(
                                    r#"Expected Err(Custom("duplicate field `element`")), got {:?}"#,
                                    e
                                ),
                            }
                        }

                        /// A list with fixed-name elements are mixed with a list with variable-name
                        /// elements in an XML, and the first element is a variable-name one
                        #[test]
                        fn fixed_after() {
                            let data = from_str::<Pair>(
                                r#"
                                <root>
                                    <one/>
                                    <element/>
                                    <two/>
                                    <three/>
                                    <element/>
                                </root>
                                "#,
                            );

                            #[cfg(feature = "overlapped-lists")]
                            assert_eq!(
                                data.unwrap(),
                                Pair {
                                    element: vec![(), ()],
                                    item: vec![
                                        Choice::One,
                                        Choice::Two,
                                        Choice::Other("three".into())
                                    ],
                                }
                            );

                            #[cfg(not(feature = "overlapped-lists"))]
                            match data {
                                Err(DeError::Custom(e)) => {
                                    assert_eq!(e, "duplicate field `$value`")
                                }
                                e => panic!(
                                    r#"Expected Err(Custom("duplicate field `$value`")), got {:?}"#,
                                    e
                                ),
                            }
                        }

                        /// Test for https://github.com/tafia/quick-xml/issues/435
                        #[test]
                        fn with_nested_list_fixed_before() {
                            let data = from_str::<Root>(
                                r#"
                                <root>
                                    <element/>
                                    <one><inner/></one>
                                    <two><inner/></two>
                                    <element/>
                                    <three><inner/></three>
                                </root>
                                "#,
                            );

                            #[cfg(feature = "overlapped-lists")]
                            assert_eq!(
                                data.unwrap(),
                                Root {
                                    element: vec![(); 2],
                                    item: vec![
                                        Choice4::One { inner: [()] },
                                        Choice4::Two { inner: [()] },
                                        Choice4::Other,
                                    ],
                                }
                            );

                            #[cfg(not(feature = "overlapped-lists"))]
                            match data {
                                Err(DeError::Custom(e)) => {
                                    assert_eq!(e, "duplicate field `element`")
                                }
                                e => panic!(
                                    r#"Expected Err(Custom("duplicate field `element`")), got {:?}"#,
                                    e
                                ),
                            }
                        }

                        /// Test for https://github.com/tafia/quick-xml/issues/435
                        #[test]
                        fn with_nested_list_fixed_after() {
                            let data = from_str::<Root>(
                                r#"
                                <root>
                                    <one><inner/></one>
                                    <element/>
                                    <two><inner/></two>
                                    <three><inner/></three>
                                    <element/>
                                </root>
                                "#,
                            );

                            #[cfg(feature = "overlapped-lists")]
                            assert_eq!(
                                data.unwrap(),
                                Root {
                                    element: vec![(); 2],
                                    item: vec![
                                        Choice4::One { inner: [()] },
                                        Choice4::Two { inner: [()] },
                                        Choice4::Other,
                                    ],
                                }
                            );

                            #[cfg(not(feature = "overlapped-lists"))]
                            match data {
                                Err(DeError::Custom(e)) => {
                                    assert_eq!(e, "duplicate field `$value`")
                                }
                                e => panic!(
                                    r#"Expected Err(Custom("duplicate field `$value`")), got {:?}"#,
                                    e
                                ),
                            }
                        }
                    }
                }

                /// Tests are ignored, but exists to show a problem.
                /// May be it will be solved in the future
                mod choice_and_choice {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Pair {
                        #[serde(rename = "$value")]
                        item: Vec<Choice>,
                        // Actually, we cannot rename both fields to `$value`, which is now
                        // required to indicate, that field accepts elements with any name
                        #[serde(rename = "$value")]
                        element: Vec<Choice2>,
                    }

                    #[test]
                    #[ignore = "There is no way to associate XML elements with `item` or `element` without extra knowledge from type"]
                    fn splitted() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <first/>
                                <second/>
                                <one/>
                                <two/>
                                <three/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: vec![Choice2::First, Choice2::Second],
                            }
                        );
                    }

                    #[test]
                    #[ignore = "There is no way to associate XML elements with `item` or `element` without extra knowledge from type"]
                    fn overlapped() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <one/>
                                <first/>
                                <two/>
                                <second/>
                                <three/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: vec![Choice2::First, Choice2::Second],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "invalid length 1, expected an array of length 3")
                            }
                            e => panic!(
                                r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                                e
                            ),
                        }
                    }
                }
            }

            /// Deserialization of primitives slightly differs from deserialization
            /// of complex types, so need to check this separately
            #[test]
            fn primitives() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    #[serde(rename = "$value")]
                    item: Vec<Choice3>,
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <one>41</one>
                        <two>42</two>
                        <three>43</three>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: vec![
                            Choice3::One(41),
                            Choice3::Two("42".to_string()),
                            Choice3::Other,
                        ],
                    }
                );

                from_str::<List>(
                    r#"
                    <root>
                        <one>41</one>
                        <two><item>42</item></two>
                        <three>43</three>
                    </root>
                    "#,
                )
                .unwrap_err();
            }

            /// Checks that sequences represented by elements can contain sequences,
            /// represented by `xs:list`s
            mod xs_list {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, Deserialize, PartialEq)]
                struct List {
                    /// Outer list mapped to elements, inner -- to `xs:list`.
                    ///
                    /// `#[serde(default)]` is required to correctly deserialize
                    /// empty sequence, because without elements the field
                    /// also is missing and derived `Deserialize` implementation
                    /// would complain about that unless field is marked as
                    /// `default`.
                    #[serde(default)]
                    #[serde(rename = "$value")]
                    element: Vec<Vec<String>>,
                }

                /// Special case: zero elements
                #[test]
                fn zero() {
                    let data: List = from_str(
                        r#"
                        <root>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(data, List { element: vec![] });
                }

                /// Special case: one element
                #[test]
                fn one() {
                    let data: List = from_str(
                        r#"
                        <root>
                            <item>first list</item>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        List {
                            element: vec![vec!["first".to_string(), "list".to_string()]]
                        }
                    );
                }

                /// Special case: outer list is always mapped to an elements sequence,
                /// not to an `xs:list`
                #[test]
                fn element() {
                    #[derive(Debug, Deserialize, PartialEq)]
                    struct List {
                        /// Outer list mapped to elements.
                        #[serde(rename = "$value")]
                        element: Vec<String>,
                    }

                    let data: List = from_str(
                        r#"
                        <root>
                            <item>first item</item>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        List {
                            element: vec!["first item".to_string()]
                        }
                    );
                }

                /// This tests demonstrates, that for `$value` field (`list`) actual
                /// name of XML element (`item`) does not matter. That allows list
                /// item to be an enum, where tag name determines enum variant
                #[test]
                fn many() {
                    let data: List = from_str(
                        r#"
                        <root>
                            <item>first list</item>
                            <item>second list</item>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        List {
                            element: vec![
                                vec!["first".to_string(), "list".to_string()],
                                vec!["second".to_string(), "list".to_string()],
                            ]
                        }
                    );
                }
            }
        }
    }
}

macro_rules! maplike_errors {
    ($type:ty) => {
        maplike_errors!($type, $type);
    };
    (
        $attributes:ty,
        $mixed:ty
    ) => {
        mod non_closed {
            use super::*;

            /// For struct we expect that error about not closed tag appears
            /// earlier than error about missing fields
            #[test]
            fn missing_field() {
                let data = from_str::<$mixed>(r#"<root>"#);

                match data {
                    Err(DeError::UnexpectedEof) => (),
                    _ => panic!("Expected `UnexpectedEof`, found {:?}", data),
                }
            }

            #[test]
            fn attributes() {
                let data = from_str::<$attributes>(r#"<root float="42" string="answer">"#);

                match data {
                    Err(DeError::UnexpectedEof) => (),
                    _ => panic!("Expected `UnexpectedEof`, found {:?}", data),
                }
            }

            #[test]
            fn elements_root() {
                let data = from_str::<$mixed>(r#"<root float="42"><string>answer</string>"#);

                match data {
                    Err(DeError::UnexpectedEof) => (),
                    _ => panic!("Expected `UnexpectedEof`, found {:?}", data),
                }
            }

            #[test]
            fn elements_child() {
                let data = from_str::<$mixed>(r#"<root float="42"><string>answer"#);

                match data {
                    Err(DeError::UnexpectedEof) => (),
                    _ => panic!("Expected `UnexpectedEof`, found {:?}", data),
                }
            }
        }

        mod mismatched_end {
            use super::*;
            use quick_xml::Error::EndEventMismatch;

            /// For struct we expect that error about mismatched tag appears
            /// earlier than error about missing fields
            #[test]
            fn missing_field() {
                let data = from_str::<$mixed>(r#"<root></mismatched>"#);

                match data {
                    Err(DeError::InvalidXml(EndEventMismatch { .. })) => (),
                    _ => panic!("Expected `InvalidXml(EndEventMismatch)`, found {:?}", data),
                }
            }

            #[test]
            fn attributes() {
                let data = from_str::<$attributes>(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root float="42" string="answer"></mismatched>"#,
                );

                match data {
                    Err(DeError::InvalidXml(EndEventMismatch { .. })) => (),
                    _ => panic!("Expected `InvalidXml(EndEventMismatch)`, found {:?}", data),
                }
            }

            #[test]
            fn elements_root() {
                let data = from_str::<$mixed>(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root float="42"><string>answer</string></mismatched>"#,
                );

                match data {
                    Err(DeError::InvalidXml(EndEventMismatch { .. })) => (),
                    _ => panic!("Expected `InvalidXml(EndEventMismatch)`, found {:?}", data),
                }
            }

            #[test]
            fn elements_child() {
                let data = from_str::<$mixed>(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root float="42"><string>answer</mismatched></root>"#,
                );

                match data {
                    Err(DeError::InvalidXml(EndEventMismatch { .. })) => (),
                    _ => panic!("Expected `InvalidXml(EndEventMismatch)`, found {:?}", data),
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

    maplike_errors!(HashMap<(), ()>);
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
                Err(DeError::ExpectedStart) => (),
                x => panic!("Expected Err(ExpectedStart), but got {:?}", x),
            };
        }

        /// CDATA events are not allowed
        #[test]
        fn cdata() {
            match from_str::<Elements>(
                "<![CDATA[excess cdata]]><root><float>42</float><string>answer</string></root>",
            ) {
                Err(DeError::ExpectedStart) => (),
                x => panic!("Expected Err(ExpectedStart), but got {:?}", x),
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

    maplike_errors!(Attributes, Mixed);
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

mod enum_ {
    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Nested {
        //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
        float: String,
    }

    /// Type where all struct fields represented by attributes
    #[derive(Debug, Deserialize, PartialEq)]
    struct NestedAttr {
        //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
        #[serde(rename = "@float")]
        float: String,
    }

    mod externally_tagged {
        use super::*;
        use pretty_assertions::assert_eq;

        /// Type where all fields of struct variants represented by elements
        #[derive(Debug, Deserialize, PartialEq)]
        enum Node {
            Unit,
            Newtype(bool),
            //TODO: serde bug https://github.com/serde-rs/serde/issues/1904
            // Tuple(f64, String),
            Struct {
                float: f64,
                string: String,
            },
            Holder {
                nested: Nested,
                string: String,
            },
            Flatten {
                #[serde(flatten)]
                nested: Nested,
                string: String,
            },
        }

        /// Type where all fields of struct variants represented by attributes
        #[derive(Debug, Deserialize, PartialEq)]
        enum NodeAttr {
            Struct {
                #[serde(rename = "@float")]
                float: f64,
                #[serde(rename = "@string")]
                string: String,
            },
            Holder {
                nested: NestedAttr,
                #[serde(rename = "@string")]
                string: String,
            },
            Flatten {
                #[serde(flatten)]
                nested: NestedAttr,
                #[serde(rename = "@string")]
                string: String,
            },
        }

        /// Workaround for serde bug https://github.com/serde-rs/serde/issues/1904
        #[derive(Debug, Deserialize, PartialEq)]
        enum Workaround {
            Tuple(f64, String),
        }

        #[test]
        fn unit() {
            let data: Node = from_str("<Unit/>").unwrap();
            assert_eq!(data, Node::Unit);
        }

        #[test]
        fn newtype() {
            let data: Node = from_str("<Newtype>true</Newtype>").unwrap();
            assert_eq!(data, Node::Newtype(true));
        }

        #[test]
        fn tuple_struct() {
            let data: Workaround = from_str("<Tuple>42</Tuple><Tuple>answer</Tuple>").unwrap();
            assert_eq!(data, Workaround::Tuple(42.0, "answer".into()));
        }

        mod struct_ {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn elements() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<Struct><float>42</float><string>answer</string></Struct>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Struct {
                        float: 42.0,
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: NodeAttr = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<Struct float="42" string="answer"/>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    NodeAttr::Struct {
                        float: 42.0,
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn namespaces() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<namespace:Struct xmlns:namespace="http://name.space"><float>42</float><string>answer</string></namespace:Struct>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Struct {
                        float: 42.0,
                        string: "answer".into()
                    }
                );
            }
        }

        mod nested_struct {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn elements() {
                let data: Node = from_str(
                    r#"<Holder><string>answer</string><nested><float>42</float></nested></Holder>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Holder {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: NodeAttr = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<Holder string="answer"><nested float="42"/></Holder>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    NodeAttr::Holder {
                        nested: NestedAttr { float: "42".into() },
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
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<Flatten><float>42</float><string>answer</string></Flatten>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Flatten {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: NodeAttr = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<Flatten float="42" string="answer"/>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    NodeAttr::Flatten {
                        nested: NestedAttr { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }
        }
    }

    mod internally_tagged {
        use super::*;

        /// Type where all fields of struct variants and a tag represented by elements
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(tag = "tag")]
        enum Node {
            Unit,
            /// Primitives (such as `bool`) are not supported by serde in the internally tagged mode
            Newtype(NewtypeContent),
            // Tuple(f64, String),// Tuples are not supported in the internally tagged mode
            Struct {
                //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
                float: String,
                string: String,
            },
            Holder {
                nested: Nested,
                string: String,
            },
            Flatten {
                #[serde(flatten)]
                nested: Nested,
                string: String,
            },
        }

        /// Type where all fields of struct variants and a tag represented by attributes
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(tag = "@tag")]
        enum NodeAttr {
            Unit,
            /// Primitives (such as `bool`) are not supported by serde in the internally tagged mode
            Newtype(NewtypeContent),
            // Tuple(f64, String),// Tuples are not supported in the internally tagged mode
            Struct {
                //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
                #[serde(rename = "@float")]
                float: String,
                #[serde(rename = "@string")]
                string: String,
            },
            Holder {
                nested: NestedAttr,
                #[serde(rename = "@string")]
                string: String,
            },
            Flatten {
                #[serde(flatten)]
                nested: NestedAttr,
                #[serde(rename = "@string")]
                string: String,
            },
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct NewtypeContent {
            value: bool,
        }

        mod unit {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn elements() {
                let data: Node = from_str(r#"<root><tag>Unit</tag></root>"#).unwrap();
                assert_eq!(data, Node::Unit);
            }

            #[test]
            fn attributes() {
                let data: NodeAttr = from_str(r#"<root tag="Unit"/>"#).unwrap();
                assert_eq!(data, NodeAttr::Unit);
            }
        }

        mod newtype {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn elements() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root><tag>Newtype</tag><value>true</value></root>"#,
                )
                .unwrap();
                assert_eq!(data, Node::Newtype(NewtypeContent { value: true }));
            }

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn attributes() {
                let data: NodeAttr = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Newtype" value="true"/>"#,
                )
                .unwrap();
                assert_eq!(data, NodeAttr::Newtype(NewtypeContent { value: true }));
            }
        }

        mod struct_ {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn elements() {
                let data: Node = from_str(
                    r#"<root><tag>Struct</tag><float>42</float><string>answer</string></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Struct {
                        float: "42".into(),
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: NodeAttr = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Struct" float="42" string="answer"/>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    NodeAttr::Struct {
                        float: "42".into(),
                        string: "answer".into()
                    }
                );
            }
        }

        mod nested_struct {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn elements() {
                let data: Node = from_str(
                    r#"<root><tag>Holder</tag><string>answer</string><nested><float>42</float></nested></root>"#,
                ).unwrap();
                assert_eq!(
                    data,
                    Node::Holder {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: NodeAttr = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Holder" string="answer"><nested float="42"/></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    NodeAttr::Holder {
                        nested: NestedAttr { float: "42".into() },
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
                let data: Node = from_str(
                    r#"<root><tag>Flatten</tag><float>42</float><string>answer</string></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Flatten {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: NodeAttr = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Flatten" float="42" string="answer"/>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    NodeAttr::Flatten {
                        nested: NestedAttr { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }
        }
    }

    mod adjacently_tagged {
        use super::*;

        /// Type where all fields of struct variants, tag and content fields
        /// represented by elements
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(tag = "tag", content = "content")]
        enum Node {
            Unit,
            Newtype(bool),
            //TODO: serde bug https://github.com/serde-rs/serde/issues/1904
            // Tuple(f64, String),
            Struct {
                float: f64,
                string: String,
            },
            Holder {
                nested: Nested,
                string: String,
            },
            Flatten {
                #[serde(flatten)]
                nested: Nested,
                string: String,
            },
        }

        /// Type where all fields of struct variants, tag and content fields
        /// represented by attributes
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(tag = "@tag", content = "@content")]
        enum NodeAttrSimple {
            Unit,
            Newtype(bool),
        }

        /// Type where all fields of struct variants and a tag represented by attributes
        /// content cannot be represented by attribute because this is a complex struct
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(tag = "@tag", content = "content")]
        enum NodeAttrComplex {
            //TODO: serde bug https://github.com/serde-rs/serde/issues/1904
            // Tuple(f64, String),
            Struct {
                #[serde(rename = "@float")]
                float: f64,
                #[serde(rename = "@string")]
                string: String,
            },
            Holder {
                nested: NestedAttr,
                #[serde(rename = "@string")]
                string: String,
            },
            Flatten {
                #[serde(flatten)]
                nested: NestedAttr,
                #[serde(rename = "@string")]
                string: String,
            },
        }

        /// Workaround for serde bug https://github.com/serde-rs/serde/issues/1904
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(tag = "tag", content = "content")]
        enum Workaround {
            Tuple(f64, String),
        }

        /// Workaround for serde bug https://github.com/serde-rs/serde/issues/1904
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(tag = "@tag", content = "@content")]
        enum WorkaroundAttr {
            Tuple(f64, String),
        }

        mod unit {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn elements() {
                let data: Node = from_str(r#"<root><tag>Unit</tag></root>"#).unwrap();
                assert_eq!(data, Node::Unit);
            }

            #[test]
            fn attributes() {
                let data: NodeAttrSimple = from_str(r#"<root tag="Unit"/>"#).unwrap();
                assert_eq!(data, NodeAttrSimple::Unit);
            }
        }

        mod newtype {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn elements() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root><tag>Newtype</tag><content>true</content></root>"#,
                )
                .unwrap();
                assert_eq!(data, Node::Newtype(true));
            }

            #[test]
            fn attributes() {
                let data: NodeAttrSimple = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Newtype" content="true"/>"#,
                )
                .unwrap();
                assert_eq!(data, NodeAttrSimple::Newtype(true));
            }
        }

        mod tuple_struct {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn elements() {
                let data: Workaround = from_str(
                    r#"<root><tag>Tuple</tag><content>42</content><content>answer</content></root>"#,
                ).unwrap();
                assert_eq!(data, Workaround::Tuple(42.0, "answer".into()));
            }

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn attributes() {
                let data: WorkaroundAttr = from_str(
                    // We cannot have two attributes with the same name, so both values stored in one attribute
                    r#"<root tag="Tuple" content="42 answer"/>"#,
                )
                .unwrap();
                assert_eq!(data, WorkaroundAttr::Tuple(42.0, "answer".into()));
            }
        }

        mod struct_ {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn elements() {
                let data: Node = from_str(
                    r#"<root><tag>Struct</tag><content><float>42</float><string>answer</string></content></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Struct {
                        float: 42.0,
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: NodeAttrComplex = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Struct"><content float="42" string="answer"/></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    NodeAttrComplex::Struct {
                        float: 42.0,
                        string: "answer".into()
                    }
                );
            }
        }

        mod nested_struct {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn elements() {
                let data: Node = from_str(
                    r#"<root>
                        <tag>Holder</tag>
                        <content>
                            <string>answer</string>
                            <nested>
                                <float>42</float>
                            </nested>
                        </content>
                    </root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Holder {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: NodeAttrComplex = from_str(
                    r#"<root tag="Holder"><content string="answer"><nested float="42"/></content></root>"#,
                ).unwrap();
                assert_eq!(
                    data,
                    NodeAttrComplex::Holder {
                        nested: NestedAttr { float: "42".into() },
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
                let data: Node = from_str(
                    r#"<root><tag>Flatten</tag><content><float>42</float><string>answer</string></content></root>"#,
                ).unwrap();
                assert_eq!(
                    data,
                    Node::Flatten {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: NodeAttrComplex = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Flatten"><content float="42" string="answer"/></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    NodeAttrComplex::Flatten {
                        nested: NestedAttr { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }
        }
    }

    mod untagged {
        use super::*;
        use pretty_assertions::assert_eq;

        /// Type where all fields of struct variants represented by elements
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(untagged)]
        enum Node {
            Unit,
            Newtype(bool),
            // serde bug https://github.com/serde-rs/serde/issues/1904
            // Tuple(f64, String),
            Struct {
                float: f64,
                string: String,
            },
            Holder {
                nested: Nested,
                string: String,
            },
            Flatten {
                #[serde(flatten)]
                nested: Nested,
                // Can't use "string" as name because in that case this variant
                // will have no difference from `Struct` variant
                string2: String,
            },
        }

        /// Type where all fields of struct variants represented by attributes
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(untagged)]
        enum NodeAttr {
            // serde bug https://github.com/serde-rs/serde/issues/1904
            // Tuple(f64, String),
            Struct {
                #[serde(rename = "@float")]
                float: f64,
                #[serde(rename = "@string")]
                string: String,
            },
            Holder {
                nested: NestedAttr,
                #[serde(rename = "@string")]
                string: String,
            },
            Flatten {
                #[serde(flatten)]
                nested: NestedAttr,
                // Can't use "string" as name because in that case this variant
                // will have no difference from `Struct` variant
                #[serde(rename = "@string2")]
                string2: String,
            },
        }

        /// Workaround for serde bug https://github.com/serde-rs/serde/issues/1904
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(untagged)]
        enum Workaround {
            Tuple(f64, String),
        }

        #[test]
        #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
        fn unit() {
            // Unit variant consists just from the tag, and because tags
            // are not written, nothing is written
            let data: Node = from_str("").unwrap();
            assert_eq!(data, Node::Unit);
        }

        #[test]
        #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
        fn newtype() {
            let data: Node = from_str("true").unwrap();
            assert_eq!(data, Node::Newtype(true));
        }

        #[test]
        #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
        fn tuple_struct() {
            let data: Workaround = from_str("<root>42</root><root>answer</root>").unwrap();
            assert_eq!(data, Workaround::Tuple(42.0, "answer".into()));
        }

        mod struct_ {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn elements() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root><float>42</float><string>answer</string></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Struct {
                        float: 42.0,
                        string: "answer".into()
                    }
                );
            }

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn attributes() {
                let data: NodeAttr = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root float="42" string="answer"/>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    NodeAttr::Struct {
                        float: 42.0,
                        string: "answer".into()
                    }
                );
            }
        }

        mod nested_struct {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn elements() {
                let data: Node = from_str(
                    r#"<root><string>answer</string><nested><float>42</float></nested></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Holder {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: NodeAttr = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root string="answer"><nested float="42"/></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    NodeAttr::Holder {
                        nested: NestedAttr { float: "42".into() },
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
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root><float>42</float><string2>answer</string2></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Flatten {
                        nested: Nested { float: "42".into() },
                        string2: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: NodeAttr = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root float="42" string2="answer"/>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    NodeAttr::Flatten {
                        nested: NestedAttr { float: "42".into() },
                        string2: "answer".into()
                    }
                );
            }
        }
    }
}

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
                        "Expected `{}({})`, found `{:?}`",
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

        serde_if_integer128! {
            list!(i128_: i128 = r#"<root list="1 -2  3"/>"# => vec![1, -2, 3]);
            list!(u128_: u128 = r#"<root list="1 2  3"/>"# => vec![1, 2, 3]);
        }

        list!(f32_: f32 = r#"<root list="1.23 -4.56  7.89"/>"# => vec![1.23, -4.56, 7.89]);
        list!(f64_: f64 = r#"<root list="1.23 -4.56  7.89"/>"# => vec![1.23, -4.56, 7.89]);

        list!(bool_: bool = r#"<root list="true false  true"/>"# => vec![true, false, true]);
        list!(char_: char = r#"<root list="4 2  j"/>"# => vec!['4', '2', 'j']);

        list!(string: String = r#"<root list="first second  third&#x20;3"/>"# => vec![
            "first".to_string(),
            "second".to_string(),
            "third 3".to_string(),
        ]);
        err!(byte_buf: ByteBuf = r#"<root list="first second  third&#x20;3"/>"#
                => Unsupported("byte arrays are not supported as `xs:list` items"));

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

            serde_if_integer128! {
                list!(i128_: i128 = "<root>1 -2  3</root>" => vec![1, -2, 3]);
                list!(u128_: u128 = "<root>1 2  3</root>" => vec![1, 2, 3]);
            }

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
                => Unsupported("byte arrays are not supported as `xs:list` items"));

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

            serde_if_integer128! {
                list!(i128_: i128 = "<root><![CDATA[1 -2  3]]></root>" => vec![1, -2, 3]);
                list!(u128_: u128 = "<root><![CDATA[1 2  3]]></root>" => vec![1, 2, 3]);
            }

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
                => Unsupported("byte arrays are not supported as `xs:list` items"));

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
            match from_str::<&str>(
                r#"<root>with escape sequence: &lt;</root>"#,
            ) {
                Err(DeError::Custom(reason)) => assert_eq!(
                    reason,
                    "invalid type: string \"with escape sequence: <\", expected a borrowed string"
                ),
                e => panic!(
                    "Expected `Err(Custom(invalid type: string \"with escape sequence: <\", expected a borrowed string))`, but found {:?}",
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
                    "Expected `Err(Custom(invalid type: string \"with escape sequence: <\", expected a borrowed string))`, but found {:?}",
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
                    "Expected `Err(Custom(invalid type: string \"with \"escape\" sequences\", expected a borrowed string))`, but found {:?}",
                    e
                ),
            }
        }
    }
}

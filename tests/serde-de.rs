use fast_xml::de::Deserializer;
use fast_xml::utils::ByteBuf;
use fast_xml::DeError;

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

#[test]
fn string_borrow() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct BorrowedText<'a> {
        #[serde(rename = "$value")]
        text: &'a str,
    }

    let borrowed_item: BorrowedText = from_str("<text>Hello world</text>").unwrap();

    assert_eq!(borrowed_item.text, "Hello world");
}

#[derive(Debug, Deserialize, PartialEq)]
struct Item {
    name: String,
    source: String,
}

#[test]
fn multiple_roots_attributes() {
    let item: Vec<Item> = from_str(
        r#"
            <item name="hello1" source="world1.rs" />
            <item name="hello2" source="world2.rs" />
        "#,
    )
    .unwrap();
    assert_eq!(
        item,
        vec![
            Item {
                name: "hello1".to_string(),
                source: "world1.rs".to_string(),
            },
            Item {
                name: "hello2".to_string(),
                source: "world2.rs".to_string(),
            },
        ]
    );
}

#[test]
fn nested_collection() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Project {
        name: String,

        #[serde(rename = "item", default)]
        items: Vec<Item>,
    }

    let project: Project = from_str(
        r#"
        <project name="my_project">
            <item name="hello1" source="world1.rs" />
            <item name="hello2" source="world2.rs" />
        </project>
        "#,
    )
    .unwrap();
    assert_eq!(
        project,
        Project {
            name: "my_project".to_string(),
            items: vec![
                Item {
                    name: "hello1".to_string(),
                    source: "world1.rs".to_string(),
                },
                Item {
                    name: "hello2".to_string(),
                    source: "world2.rs".to_string(),
                },
            ],
        }
    );
}

#[test]
fn collection_of_enums() {
    #[derive(Debug, Deserialize, PartialEq)]
    enum MyEnum {
        A(String),
        B { name: String, flag: bool },
        C,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct MyEnums {
        // TODO: This should be #[serde(flatten)], but right now serde don't support flattening of sequences
        // See https://github.com/serde-rs/serde/issues/1905
        #[serde(rename = "$value")]
        items: Vec<MyEnum>,
    }

    let s = r#"
    <enums>
        <A>test</A>
        <B name="hello" flag="t" />
        <C />
    </enums>
    "#;

    let project: MyEnums = from_str(s).unwrap();

    assert_eq!(
        project,
        MyEnums {
            items: vec![
                MyEnum::A("test".to_string()),
                MyEnum::B {
                    name: "hello".to_string(),
                    flag: true,
                },
                MyEnum::C,
            ],
        }
    );
}

#[test]
fn deserialize_bytes() {
    let item: ByteBuf = from_str(r#"<item>bytes</item>"#).unwrap();

    assert_eq!(item, ByteBuf(b"bytes".to_vec()));
}

/// Test for https://github.com/tafia/quick-xml/issues/231
#[test]
fn implicit_value() {
    use serde_value::Value;

    let item: Value = from_str(r#"<root>content</root>"#).unwrap();

    assert_eq!(
        item,
        Value::Map(
            vec![(
                Value::String("$value".into()),
                Value::String("content".into())
            )]
            .into_iter()
            .collect()
        )
    );
}

#[test]
fn explicit_value() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Item {
        #[serde(rename = "$value")]
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
fn without_value() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Item;

    let item: Item = from_str(r#"<root>content</root>"#).unwrap();

    assert_eq!(item, Item);
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
                let item = from_str::<$type>($value).unwrap_err();

                match item {
                    DeError::UnexpectedEof => (),
                    _ => panic!("Expected `UnexpectedEof`, found {:?}", item),
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
            eof!(byte_buf: ByteBuf = $value);

            #[test]
            fn unit() {
                let item = from_str::<()>($value).unwrap_err();

                match item {
                    DeError::UnexpectedEof => (),
                    _ => panic!("Expected `UnexpectedEof`, found {:?}", item),
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

    /// Tests deserialization from top-level tag content: `<root>...content...</root>`
    mod struct_ {
        use super::*;

        /// Well-formed XML must have a single tag at the root level.
        /// Any XML tag can be modeled as a struct, and content of this tag are modeled as
        /// fields of this struct.
        ///
        /// Because we want to get access to unnamed content of the tag (usually, this internal
        /// XML node called `#text`) we use a rename to a special name `$value`
        #[derive(Debug, Deserialize, PartialEq)]
        struct Trivial<T> {
            #[serde(rename = "$value")]
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
                        Err(DeError::UnexpectedStart(tag)) => assert_eq!(tag, b"root"),
                        x => panic!(
                            r#"Expected `Err(DeError::UnexpectedStart("root"))`, but got `{:?}`"#,
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

            in_struct!(string:   String  = "<root>escaped&#x20;string</root>", "escaped string".into());
            // Byte buffers gives access to the raw data from the input, so never treated as escaped
            // TODO: It is a bit unusual and it would be better completely forbid deserialization
            // into bytes, because XML cannot store any bytes natively. User should use some sort
            // of encoding to a string, for example, hex or base64
            in_struct!(byte_buf: ByteBuf = "<root>escaped&#x20;byte_buf</root>", ByteBuf(r"escaped&#x20;byte_buf".into()));
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
            in_struct!(string:   String  = "<root><![CDATA[escaped&#x20;string]]></root>", "escaped&#x20;string".into());
            in_struct!(byte_buf: ByteBuf = "<root><![CDATA[escaped&#x20;byte_buf]]></root>", ByteBuf(r"escaped&#x20;byte_buf".into()));
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

macro_rules! maplike_errors {
    ($type:ty) => {
        mod non_closed {
            use super::*;

            #[test]
            fn attributes() {
                let data = from_str::<$type>(r#"<root float="42" string="answer">"#);

                match data {
                    Err(DeError::UnexpectedEof) => (),
                    _ => panic!("Expected `UnexpectedEof`, found {:?}", data),
                }
            }

            #[test]
            fn elements_root() {
                let data = from_str::<$type>(r#"<root float="42"><string>answer</string>"#);

                match data {
                    Err(DeError::UnexpectedEof) => (),
                    _ => panic!("Expected `UnexpectedEof`, found {:?}", data),
                }
            }

            #[test]
            fn elements_child() {
                let data = from_str::<$type>(r#"<root float="42"><string>answer"#);

                match data {
                    Err(DeError::UnexpectedEof) => (),
                    _ => panic!("Expected `UnexpectedEof`, found {:?}", data),
                }
            }
        }

        mod mismatched_end {
            use super::*;
            use fast_xml::Error::EndEventMismatch;

            #[test]
            fn attributes() {
                let data = from_str::<$type>(
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
                let data = from_str::<$type>(
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
                let data = from_str::<$type>(
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

    #[derive(Debug, Deserialize, PartialEq)]
    struct Struct {
        float: f64,
        string: String,
    }

    #[test]
    fn elements() {
        let data: Struct = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root><float>42</float><string>answer</string></root>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Struct {
                float: 42.0,
                string: "answer".into()
            }
        );
    }

    #[test]
    fn excess_elements() {
        let data: Struct = from_str(
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
            Struct {
                float: 42.0,
                string: "answer".into()
            }
        );
    }

    #[test]
    fn attributes() {
        let data: Struct = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root float="42" string="answer"/>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Struct {
                float: 42.0,
                string: "answer".into()
            }
        );
    }

    #[test]
    fn excess_attributes() {
        let data: Struct = from_str(
            r#"<root before="1" float="42" in-the-middle="2" string="answer" after="3"/>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Struct {
                float: 42.0,
                string: "answer".into()
            }
        );
    }

    #[test]
    fn attribute_and_element() {
        let data: Struct = from_str(
            r#"
            <root float="42">
                <string>answer</string>
            </root>
        "#,
        )
        .unwrap();

        assert_eq!(
            data,
            Struct {
                float: 42.0,
                string: "answer".into()
            }
        );
    }

    maplike_errors!(Struct);
}

mod nested_struct {
    use super::*;
    use pretty_assertions::assert_eq;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Struct {
        nested: Nested,
        string: String,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Nested {
        float: f32,
    }

    #[test]
    fn elements() {
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

    #[test]
    #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
    fn elements() {
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

    mod externally_tagged {
        use super::*;
        use pretty_assertions::assert_eq;

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

        #[derive(Debug, Deserialize, PartialEq)]
        struct Nested {
            //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
            float: String,
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
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<Struct float="42" string="answer"/>"#,
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
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<Holder string="answer"><nested float="42"/></Holder>"#,
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
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<Flatten float="42" string="answer"/>"#,
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
        }
    }

    mod internally_tagged {
        use super::*;

        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(tag = "tag")]
        enum Node {
            Unit,
            /// Primitives (such as `bool`) are not supported by serde in the internally tagged mode
            Newtype(NewtypeContent),
            // Tuple(f64, String),// Tuples are not supported in the internally tagged mode
            //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
            Struct {
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

        #[derive(Debug, Deserialize, PartialEq)]
        struct NewtypeContent {
            value: bool,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Nested {
            //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
            float: String,
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
                let data: Node = from_str(r#"<root tag="Unit"/>"#).unwrap();
                assert_eq!(data, Node::Unit);
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
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Newtype" value="true"/>"#,
                )
                .unwrap();
                assert_eq!(data, Node::Newtype(NewtypeContent { value: true }));
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
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Struct" float="42" string="answer"/>"#,
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
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Holder" string="answer"><nested float="42"/></root>"#,
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
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Flatten" float="42" string="answer"/>"#,
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
        }
    }

    mod adjacently_tagged {
        use super::*;

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

        #[derive(Debug, Deserialize, PartialEq)]
        struct Nested {
            //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
            float: String,
        }

        /// Workaround for serde bug https://github.com/serde-rs/serde/issues/1904
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(tag = "tag", content = "content")]
        enum Workaround {
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
                let data: Node = from_str(r#"<root tag="Unit"/>"#).unwrap();
                assert_eq!(data, Node::Unit);
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
                let data: Node = from_str(r#"<root tag="Newtype" content="true"/>"#).unwrap();
                assert_eq!(data, Node::Newtype(true));
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
                let data: Workaround = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Tuple" content="42"><content>answer</content></root>"#,
                )
                .unwrap();
                assert_eq!(data, Workaround::Tuple(42.0, "answer".into()));
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
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Struct"><content float="42" string="answer"/></root>"#,
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
                let data: Node = from_str(
                    r#"<root tag="Holder"><content string="answer"><nested float="42"/></content></root>"#,
                ).unwrap();
                assert_eq!(
                    data,
                    Node::Holder {
                        nested: Nested { float: "42".into() },
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
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Flatten"><content float="42" string="answer"/></root>"#,
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
        }
    }

    mod untagged {
        use super::*;
        use pretty_assertions::assert_eq;

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

        #[derive(Debug, Deserialize, PartialEq)]
        struct Nested {
            //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
            float: String,
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
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root float="42" string="answer"/>"#,
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
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root string="answer"><nested float="42"/></root>"#,
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
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root float="42" string2="answer"/>"#,
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
        }
    }
}

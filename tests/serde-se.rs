use quick_xml::de::from_str;
use quick_xml::se::Serializer;
use quick_xml::utils::Bytes;
use quick_xml::DeError;

use serde::{serde_if_integer128, Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Unit;

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Newtype(bool);

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Tuple(f32, &'static str);

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Struct {
    float: f64,
    string: &'static str,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct NestedStruct {
    nested: Nested,
    string: &'static str,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct FlattenStruct {
    #[serde(flatten)]
    nested: Nested,
    string: &'static str,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Nested {
    float: f64,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Empty {}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Text {
    #[serde(rename = "$text")]
    float: f64,
    string: &'static str,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
enum ExternallyTagged {
    Unit,
    Newtype(bool),
    Tuple(f64, &'static str),
    Struct {
        float: f64,
        string: &'static str,
    },
    Holder {
        nested: Nested,
        string: &'static str,
    },
    /// `float` field serialized as textual content instead of a tag
    Text {
        #[serde(rename = "$text")]
        float: f64,
        string: &'static str,
    },
    Empty {},
}

/// Having both `#[serde(flatten)]` and `'static` fields in one struct leads to
/// incorrect code generation when deriving `Deserialize`.
///
/// TODO: Merge into main enum after fixing <https://github.com/serde-rs/serde/issues/2371>
///
/// Anyway, deserialization of that type in roundtrip suffers from
/// <https://github.com/serde-rs/serde/issues/1183>
#[derive(Debug, PartialEq, Deserialize, Serialize)]
enum ExternallyTaggedWorkaround {
    Flatten {
        #[serde(flatten)]
        nested: Nested,
        string: &'static str,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "tag")]
enum InternallyTagged {
    Unit,
    /// Primitives (such as `bool`) are not supported by the serde in the internally tagged mode
    Newtype(Nested),
    // Tuple(f64, &'static str),// Tuples are not supported in the internally tagged mode
    Struct {
        float: f64,
        string: &'static str,
    },
    Holder {
        nested: Nested,
        string: &'static str,
    },
    /// `float` field serialized as textual content instead of a tag
    Text {
        #[serde(rename = "$text")]
        float: f64,
        string: &'static str,
    },
    Empty {},
}

/// Having both `#[serde(flatten)]` and `'static` fields in one struct leads to
/// incorrect code generation when deriving `Deserialize`.
///
/// TODO: Merge into main enum after fixing <https://github.com/serde-rs/serde/issues/2371>
///
/// Anyway, deserialization of that type in roundtrip suffers from
/// <https://github.com/serde-rs/serde/issues/1183>
#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "tag")]
enum InternallyTaggedWorkaround {
    Flatten {
        #[serde(flatten)]
        nested: Nested,
        string: &'static str,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "tag", content = "content")]
enum AdjacentlyTagged {
    Unit,
    Newtype(bool),
    Tuple(f64, &'static str),
    Struct {
        float: f64,
        string: &'static str,
    },
    Holder {
        nested: Nested,
        string: &'static str,
    },
    /// `float` field serialized as textual content instead of a tag
    Text {
        #[serde(rename = "$text")]
        float: f64,
        string: &'static str,
    },
    Empty {},
}

/// Having both `#[serde(flatten)]` and `'static` fields in one struct leads to
/// incorrect code generation when deriving `Deserialize`.
///
/// TODO: Merge into main enum after fixing <https://github.com/serde-rs/serde/issues/2371>
///
/// Anyway, deserialization of that type in roundtrip suffers from
/// <https://github.com/serde-rs/serde/issues/1183>
#[derive(Serialize)]
#[serde(tag = "tag", content = "content")]
enum AdjacentlyTaggedWorkaround {
    Flatten {
        #[serde(flatten)]
        nested: Nested,
        string: &'static str,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
enum Untagged {
    Unit,
    Newtype(bool),
    Tuple(f64, &'static str),
    Struct {
        float: f64,
        string: &'static str,
    },
    Holder {
        nested: Nested,
        string: &'static str,
    },
    /// `float` field serialized as textual content instead of a tag
    Text {
        #[serde(rename = "$text")]
        float: f64,
        string: &'static str,
    },
    Empty {},
}

/// Having both `#[serde(flatten)]` and `'static` fields in one struct leads to
/// incorrect code generation when deriving `Deserialize`.
///
/// TODO: Merge into main enum after fixing <https://github.com/serde-rs/serde/issues/2371>
///
/// Anyway, deserialization of that type in roundtrip suffers from
/// <https://github.com/serde-rs/serde/issues/1183>
#[derive(Serialize)]
#[serde(untagged)]
enum UntaggedWorkaround {
    Flatten {
        #[serde(flatten)]
        nested: Nested,
        string: &'static str,
    },
}

mod without_root {
    use super::*;
    use pretty_assertions::assert_eq;

    macro_rules! serialize_as {
        ($name:ident: $data:expr => $expected:expr) => {
            #[test]
            fn $name() {
                serialize_as!(@ $data => $expected);

                // Roundtrip to ensure that serializer corresponds to deserializer
                assert_eq!(
                    $data,
                    from_str($expected).expect("deserialization roundtrip"),
                    "deserialization roundtrip",
                );
            }
        };
        (@ $data:expr => $expected:expr) => {
            let mut buffer = String::new();
            let ser = Serializer::new(&mut buffer);

            $data.serialize(ser).unwrap();
            assert_eq!(buffer, $expected);
        };
    }
    macro_rules! serialize_as_only {
        ($name:ident: $data:expr => $expected:literal) => {
            #[test]
            fn $name() {
                serialize_as!(@ $data => $expected);
            }
        };
    }

    /// Checks that attempt to serialize given `$data` results to a
    /// serialization error `$kind` with `$reason`
    macro_rules! err {
        ($name:ident: $data:expr => $kind:ident($reason:literal), $buffer:literal) => {
            #[test]
            fn $name() {
                let mut buffer = String::new();
                let ser = Serializer::new(&mut buffer);

                match $data.serialize(ser) {
                    Err(DeError::$kind(e)) => assert_eq!(e, $reason),
                    e => panic!(
                        "Expected `{}({})`, found `{:?}`",
                        stringify!($kind),
                        $reason,
                        e
                    ),
                }
                assert_eq!(buffer, $buffer);
            }
        };
        ($name:ident: $data:expr => $kind:ident($reason:literal)) => {
            err!($name: $data => $kind($reason), "");
        };
    }

    err!(false_: false => Unsupported("cannot serialize `bool` without defined root tag"));
    err!(true_:  true  => Unsupported("cannot serialize `bool` without defined root tag"));

    err!(i8_:    -42i8                => Unsupported("cannot serialize `i8` without defined root tag"));
    err!(i16_:   -4200i16             => Unsupported("cannot serialize `i16` without defined root tag"));
    err!(i32_:   -42000000i32         => Unsupported("cannot serialize `i32` without defined root tag"));
    err!(i64_:   -42000000000000i64   => Unsupported("cannot serialize `i64` without defined root tag"));
    err!(isize_: -42000000000000isize => Unsupported("cannot serialize `i64` without defined root tag"));

    err!(u8_:    42u8                => Unsupported("cannot serialize `u8` without defined root tag"));
    err!(u16_:   4200u16             => Unsupported("cannot serialize `u16` without defined root tag"));
    err!(u32_:   42000000u32         => Unsupported("cannot serialize `u32` without defined root tag"));
    err!(u64_:   42000000000000u64   => Unsupported("cannot serialize `u64` without defined root tag"));
    err!(usize_: 42000000000000usize => Unsupported("cannot serialize `u64` without defined root tag"));

    serde_if_integer128! {
        err!(i128_: -420000000000000000000000000000i128 => Unsupported("cannot serialize `i128` without defined root tag"));
        err!(u128_:  420000000000000000000000000000u128 => Unsupported("cannot serialize `u128` without defined root tag"));
    }

    err!(f32_: 4.2f32 => Unsupported("cannot serialize `f32` without defined root tag"));
    err!(f64_: 4.2f64 => Unsupported("cannot serialize `f64` without defined root tag"));

    err!(char_non_escaped: 'h'  => Unsupported("cannot serialize `char` without defined root tag"));
    err!(char_lt:          '<'  => Unsupported("cannot serialize `char` without defined root tag"));
    err!(char_gt:          '>'  => Unsupported("cannot serialize `char` without defined root tag"));
    err!(char_amp:         '&'  => Unsupported("cannot serialize `char` without defined root tag"));
    err!(char_apos:        '\'' => Unsupported("cannot serialize `char` without defined root tag"));
    err!(char_quot:        '"'  => Unsupported("cannot serialize `char` without defined root tag"));
    err!(char_space:       ' '  => Unsupported("cannot serialize `char` without defined root tag"));

    err!(str_non_escaped: "non-escaped string" => Unsupported("cannot serialize `&str` without defined root tag"));
    err!(str_escaped:  "<\"escaped & string'>" => Unsupported("cannot serialize `&str` without defined root tag"));

    err!(bytes: Bytes(b"<\"escaped & bytes'>") => Unsupported("cannot serialize `&[u8]` without defined root tag"));

    serialize_as!(option_none: Option::<Unit>::None => "");
    serialize_as!(option_some: Some(Unit) => "<Unit/>");

    err!(unit: () => Unsupported("cannot serialize `()` without defined root tag"));
    serialize_as!(unit_struct: Unit => "<Unit/>");

    serialize_as!(newtype: Newtype(true) => "<Newtype>true</Newtype>");

    err!(seq: vec![1, 2, 3] => Unsupported("cannot serialize sequence without defined root tag"));
    err!(tuple:
        ("<\"&'>", "with\t\r\n spaces", 3usize)
        => Unsupported("cannot serialize unnamed tuple without defined root tag"));
    serialize_as!(tuple_struct:
        Tuple(42.0, "answer")
        => "<Tuple>42</Tuple>\
            <Tuple>answer</Tuple>");

    err!(map:
        BTreeMap::from([("$text", 1), ("_2", 3)])
        => Unsupported("cannot serialize map without defined root tag"));
    serialize_as!(struct_:
        Struct {
            float: 42.0,
            string: "answer"
        }
        => "<Struct>\
                <float>42</float>\
                <string>answer</string>\
            </Struct>");
    serialize_as!(nested_struct:
        NestedStruct {
            nested: Nested { float: 42.0 },
            string: "answer",
        }
        => "<NestedStruct>\
                <nested>\
                    <float>42</float>\
                </nested>\
                <string>answer</string>\
            </NestedStruct>");
    // serde serializes flatten structs as maps, and we do not support
    // serialization of maps without root tag
    err!(flatten_struct:
        FlattenStruct {
            nested: Nested { float: 42.0 },
            string: "answer",
        }
        => Unsupported("cannot serialize map without defined root tag"));
    serialize_as!(empty_struct:
        Empty {}
        => "<Empty/>");
    serialize_as!(text:
        Text {
            float: 42.0,
            string: "answer",
        }
        => "<Text>\
                42\
                <string>answer</string>\
            </Text>");

    mod enum_ {
        use super::*;

        /// Enum representation:
        ///
        /// |Kind   |Top-level and in `$value` field          |
        /// |-------|-----------------------------------------|
        /// |Unit   |`<Unit/>`                                |
        /// |Newtype|`<Newtype>42</Newtype>`                  |
        /// |Tuple  |`<Tuple>42</Tuple><Tuple>answer</Tuple>` |
        /// |Struct |`<Struct><q>42</q><a>answer</a></Struct>`|
        mod externally_tagged {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Deserialize, Serialize)]
            struct Root<T> {
                field: T,
            }

            serialize_as!(unit:
                ExternallyTagged::Unit
                => "<Unit/>");
            serialize_as!(newtype:
                ExternallyTagged::Newtype(true)
                => "<Newtype>true</Newtype>");
            serialize_as!(tuple:
                ExternallyTagged::Tuple(42.0, "answer")
                => "<Tuple>42</Tuple>\
                    <Tuple>answer</Tuple>");
            serialize_as!(struct_:
                ExternallyTagged::Struct {
                    float: 42.0,
                    string: "answer"
                }
                => "<Struct>\
                        <float>42</float>\
                        <string>answer</string>\
                    </Struct>");
            serialize_as!(nested_struct:
                ExternallyTagged::Holder {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => "<Holder>\
                        <nested>\
                            <float>42</float>\
                        </nested>\
                        <string>answer</string>\
                    </Holder>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(flatten_struct:
                ExternallyTaggedWorkaround::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => "<Flatten>\
                        <float>42</float>\
                        <string>answer</string>\
                    </Flatten>");
            serialize_as!(empty_struct:
                ExternallyTagged::Empty {}
                => "<Empty/>");
            serialize_as!(text:
                ExternallyTagged::Text {
                    float: 42.0,
                    string: "answer"
                }
                => "<Text>\
                        42\
                        <string>answer</string>\
                    </Text>");

            /// Test serialization of the specially named variant `$text`.
            ///
            /// Enum representation:
            ///
            /// |Kind   |Top-level, in `$text` and `$value` fields|
            /// |-------|-----------------------------------------|
            /// |Unit   |_(empty)_                                |
            /// |Newtype|`42`                                     |
            /// |Tuple  |`42 answer`                              |
            /// |Struct |Err(Unsupported)                         |
            mod text_variant {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize, Serialize)]
                enum Unit {
                    #[serde(rename = "$text")]
                    Text,
                }
                #[derive(Debug, PartialEq, Deserialize, Serialize)]
                enum Newtype<'a> {
                    #[serde(rename = "$text")]
                    Text(&'a str),
                }
                #[derive(Debug, PartialEq, Deserialize, Serialize)]
                enum Tuple {
                    #[serde(rename = "$text")]
                    Text(f64, String),
                }
                #[derive(Debug, PartialEq, Deserialize, Serialize)]
                enum Struct<'a> {
                    #[serde(rename = "$text")]
                    Text { float: f64, string: &'a str },
                }

                // It is unknown how to exactly serialize unit to a text
                err!(unit: Unit::Text => Unsupported("cannot serialize enum unit variant `Unit::$text` as text content value"));
                serialize_as!(newtype: Newtype::Text("newtype text") => "newtype text");
                // Tuple variant serialized as an `xs:list`
                serialize_as!(tuple: Tuple::Text(4.2, "newtype-text".into()) => "4.2 newtype-text");
                // Note, that spaces in strings, even escaped, would represent
                // the list item delimiters. Non-symmetric serialization follows
                // tradition: the XmlBeans Java library have the same behavior.
                // See also <https://stackoverflow.com/questions/45494204/escape-space-in-xml-xslist>
                serialize_as_only!(tuple_with_spaces: Tuple::Text(4.2, "newtype text".into()) => "4.2 newtype&#32;text");
                // Struct variant cannot be directly serialized to a text
                err!(struct_:
                    Struct::Text {
                        float: 4.2,
                        string: "newtype text",
                    }
                    => Unsupported("cannot serialize enum struct variant `Struct::$text` as text content value"));
                /// Tests the enum type that is type of field of a struct.
                /// The tests above does not cover those variants, because we use
                /// different serializers for enums on top level and which represents
                /// a field.
                ///
                /// According to general rules for structs, we should write `<field>` tag
                /// for a `field` field. Because value of that field is an enum, we should
                /// somehow know what the variant was written in order to deserialize it,
                /// but `$text` variant say us that we should write enum content using
                /// `xs:simpleType` serialization.
                ///
                /// Enum representation:
                ///
                /// |Kind   |In normal field `field`|
                /// |-------|-----------------------|
                /// |Unit   |`<field/>`             |
                /// |Newtype|Err(Unsupported) [^1]  |
                /// |Tuple  |Err(Unsupported) [^1]  |
                /// |Struct |Err(Unsupported)       |
                ///
                /// [^1]: Unfortunately, cannot be represented, because the possible representation
                ///       (`<field>42</field>` and `<field>42 answer</field>`) will clash with
                ///       representation of normal unit variant in normal field
                mod normal_field {
                    use super::*;
                    use super::{Newtype, Struct, Tuple, Unit};
                    use pretty_assertions::assert_eq;

                    // `Root::field` contains text content, and because text content is empty,
                    // `<field/>` is written
                    serialize_as!(unit: Root { field: Unit::Text } => "<Root><field/></Root>");
                    err!(newtype:
                        Root { field: Newtype::Text("newtype text") }
                        => Unsupported("cannot serialize enum newtype variant `Newtype::$text`"),
                        "<Root");
                    err!(tuple:
                        Root { field: Tuple::Text(42.0, "tuple-text".into()) }
                        => Unsupported("cannot serialize enum tuple variant `Tuple::$text`"),
                        "<Root");
                    err!(struct_:
                        Root { field: Struct::Text {
                            float: 42.0,
                            string: "answer"
                        }}
                        => Unsupported("cannot serialize enum struct variant `Struct::$text` as text content value"),
                        "<Root");
                }

                /// The same tests as in `normal_field`, but struct field renamed to `$value`.
                ///
                /// `$value` fields means, that struct field name won't be written, but instead
                /// the whole representation of a field is dependent on its type. Its type is
                /// an enum which variant tag is not written because variant content represents
                /// text (that is what `$text` variant means). So the enum variant would be
                /// serialized as a text, and because struct field itself not written, we get
                /// the text wrapped in the `<Root>` tags.
                ///
                /// Enum representation:
                ///
                /// |Kind   |Top-level and in `$value` field|
                /// |-------|-------------------------------|
                /// |Unit   |_(empty)_                      |
                /// |Newtype|`42`                           |
                /// |Tuple  |`42 answer`                    |
                /// |Struct |Err(Unsupported)               |
                mod value_field {
                    use super::*;
                    use super::{Newtype, Struct, Tuple, Unit};
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize, Serialize)]
                    struct Root<T> {
                        #[serde(rename = "$value")]
                        field: T,
                    }

                    // Without #[serde(default)] on a field we cannot deserialize value
                    // back, because there is no signs in the XML that `field` was written.
                    // If we write the usual enum, then variant name would be written as
                    // a tag, but because variant is a `$text`, nothing is written
                    serialize_as_only!(unit: Root { field: Unit::Text } => "<Root/>");
                    serialize_as!(newtype:
                        Root { field: Newtype::Text("newtype text") }
                        => "<Root>newtype text</Root>");
                    serialize_as!(tuple:
                        Root { field: Tuple::Text(42.0, "tuple-text".into()) }
                        => "<Root>42 tuple-text</Root>");
                    // Note, that spaces in strings, even escaped, would represent
                    // the list item delimiters. Non-symmetric serialization follows
                    // tradition: the XmlBeans Java library have the same behavior.
                    // See also <https://stackoverflow.com/questions/45494204/escape-space-in-xml-xslist>
                    serialize_as_only!(tuple_with_spaces:
                        Root { field: Tuple::Text(42.0, "tuple text".into()) }
                        => "<Root>42 tuple&#32;text</Root>");
                    err!(struct_:
                        Root { field: Struct::Text {
                            float: 42.0,
                            string: "answer"
                        }}
                        => Unsupported("cannot serialize `$text` struct variant of `Struct` enum"),
                        "<Root");
                }

                /// The same tests as in `normal_field`, but struct field renamed to `$text`.
                ///
                /// Enum representation:
                ///
                /// |Kind   |In `$text` field     |
                /// |-------|---------------------|
                /// |Unit   |_(empty)_            |
                /// |Newtype|Err(Unsupported) [^1]|
                /// |Tuple  |Err(Unsupported) [^1]|
                /// |Struct |Err(Unsupported)     |
                ///
                /// [^1]: Unfortunately, cannot be represented, because the possible
                ///       representation (`42` and `42 answer`) will clash with
                ///       representation of normal unit variant in normal field
                mod text_field {
                    use super::*;
                    use super::{Newtype, Struct, Tuple, Unit};
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize, Serialize)]
                    struct Root<T> {
                        #[serde(rename = "$text")]
                        field: T,
                    }

                    // Without #[serde(default)] on a field we cannot deserialize value
                    // back, because there is no signs in the XML that `field` was written.
                    // If we write the usual enum, then variant name would be written as
                    // a tag, but because variant is a `$text`, nothing is written
                    serialize_as_only!(unit: Root { field: Unit::Text } => "<Root/>");
                    err!(newtype:
                        Root { field: Newtype::Text("newtype text") }
                        => Unsupported("cannot serialize enum newtype variant `Newtype::$text` as an attribute or text content value"),
                        "<Root");
                    err!(tuple:
                        Root { field: Tuple::Text(42.0, "tuple-text".into()) }
                        => Unsupported("cannot serialize enum tuple variant `Tuple::$text` as an attribute or text content value"),
                        "<Root");
                    err!(struct_:
                        Root { field: Struct::Text {
                            float: 42.0,
                            string: "answer"
                        }}
                        => Unsupported("cannot serialize enum struct variant `Struct::$text` as an attribute or text content value"),
                        "<Root");
                }
            }

            /// Tests the enum type that is type of field of a struct.
            /// The tests above does not cover those variants, because we use
            /// different serializers for enums on top level and which represents
            /// a field.
            ///
            /// Deserialization is not possible because we cannot choose with what
            /// field we should associate the XML node that we see. To do that we
            /// mark field by a special name `$value` ([`VALUE_KEY`]) and that is
            /// tested in the `value_field` module.
            ///
            /// Enum representation:
            ///
            /// |Kind   |In normal field      |
            /// |-------|---------------------|
            /// |Unit   |`<field>Unit</field>`|
            /// |Newtype|Err(Unsupported)     |
            /// |Tuple  |Err(Unsupported)     |
            /// |Struct |Err(Unsupported)     |
            mod normal_field {
                use super::*;
                use pretty_assertions::assert_eq;

                serialize_as!(unit:
                    Root { field: ExternallyTagged::Unit }
                    => "<Root>\
                            <field>Unit</field>\
                        </Root>");
                err!(newtype:
                    Root { field: ExternallyTagged::Newtype(true) }
                    => Unsupported("cannot serialize enum newtype variant `ExternallyTagged::Newtype`"),
                    "<Root");
                err!(tuple:
                    Root { field: ExternallyTagged::Tuple(42.0, "answer") }
                    => Unsupported("cannot serialize enum tuple variant `ExternallyTagged::Tuple`"),
                    "<Root");
                err!(struct_:
                    Root { field: ExternallyTagged::Struct {
                        float: 42.0,
                        string: "answer"
                    }}
                    => Unsupported("cannot serialize enum struct variant `ExternallyTagged::Struct`"),
                    "<Root");
                err!(empty_struct:
                    Root { field: ExternallyTagged::Empty {} }
                    => Unsupported("cannot serialize enum struct variant `ExternallyTagged::Empty`"),
                    "<Root");
            }

            /// The same tests as in `normal_field`, but enum at the second nesting
            /// level.
            mod normal_field2 {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize, Serialize)]
                struct Root<T> {
                    field: T,
                }

                #[derive(Debug, PartialEq, Deserialize, Serialize)]
                struct Inner<T> {
                    inner: T,
                }

                serialize_as!(unit:
                    Root { field: Inner { inner: ExternallyTagged::Unit } }
                    => "<Root>\
                            <field>\
                                <inner>Unit</inner>\
                            </field>\
                        </Root>");
                err!(newtype:
                    Root { field: Inner { inner: ExternallyTagged::Newtype(true) } }
                    => Unsupported("cannot serialize enum newtype variant `ExternallyTagged::Newtype`"),
                    "<Root");
                err!(tuple:
                    Root { field: Inner { inner: ExternallyTagged::Tuple(42.0, "answer") } }
                    => Unsupported("cannot serialize enum tuple variant `ExternallyTagged::Tuple`"),
                    "<Root");
                err!(struct_:
                    Root { field: Inner { inner: ExternallyTagged::Struct {
                        float: 42.0,
                        string: "answer"
                    }}}
                    => Unsupported("cannot serialize enum struct variant `ExternallyTagged::Struct`"),
                    "<Root");
                err!(empty_struct:
                    Root { field: Inner { inner: ExternallyTagged::Empty {} } }
                    => Unsupported("cannot serialize enum struct variant `ExternallyTagged::Empty`"),
                    "<Root");
            }

            /// The same tests as in `normal_field`, but enum field renamed to `$value`.
            ///
            /// Enum representation:
            ///
            /// |Kind   |Top-level and in `$value` field          |
            /// |-------|-----------------------------------------|
            /// |Unit   |`<Unit/>`                                |
            /// |Newtype|`<Newtype>42</Newtype>`                  |
            /// |Tuple  |`<Tuple>42</Tuple><Tuple>answer</Tuple>` |
            /// |Struct |`<Struct><q>42</q><a>answer</a></Struct>`|
            mod value_field {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize, Serialize)]
                struct Root<T> {
                    #[serde(rename = "$value")]
                    field: T,
                }

                serialize_as!(unit:
                    Root { field: ExternallyTagged::Unit }
                    => "<Root>\
                            <Unit/>\
                        </Root>");
                serialize_as!(newtype:
                    Root { field: ExternallyTagged::Newtype(true) }
                    => "<Root>\
                            <Newtype>true</Newtype>\
                        </Root>");
                serialize_as!(tuple:
                    Root { field: ExternallyTagged::Tuple(42.0, "answer") }
                    => "<Root>\
                            <Tuple>42</Tuple>\
                            <Tuple>answer</Tuple>\
                        </Root>");
                serialize_as!(struct_:
                    Root { field: ExternallyTagged::Struct {
                        float: 42.0,
                        string: "answer"
                    }}
                    => "<Root>\
                            <Struct>\
                                <float>42</float>\
                                <string>answer</string>\
                            </Struct>\
                        </Root>");
                serialize_as!(nested_struct:
                    Root { field: ExternallyTagged::Holder {
                        nested: Nested { float: 42.0 },
                        string: "answer",
                    }}
                    => "<Root>\
                            <Holder>\
                                <nested>\
                                    <float>42</float>\
                                </nested>\
                                <string>answer</string>\
                            </Holder>\
                        </Root>");
                // NOTE: Cannot be deserialized in roundtrip due to
                // https://github.com/serde-rs/serde/issues/1183
                serialize_as_only!(flatten_struct:
                    Root { field: ExternallyTaggedWorkaround::Flatten {
                        nested: Nested { float: 42.0 },
                        string: "answer",
                    }}
                    => "<Root>\
                            <Flatten>\
                                <float>42</float>\
                                <string>answer</string>\
                            </Flatten>\
                        </Root>");
                serialize_as!(empty_struct:
                    Root { field: ExternallyTagged::Empty {} }
                    => "<Root>\
                            <Empty/>\
                        </Root>");
                serialize_as!(text:
                    Root { field: ExternallyTagged::Text {
                        float: 42.0,
                        string: "answer"
                    }}
                    => "<Root>\
                            <Text>\
                                42\
                                <string>answer</string>\
                            </Text>\
                        </Root>");
            }

            /// The same tests as in `normal_field2`, but enum field renamed to `$value`.
            mod value_field2 {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize, Serialize)]
                struct Inner<T> {
                    #[serde(rename = "$value")]
                    inner: T,
                }

                serialize_as!(unit:
                    Root { field: Inner { inner: ExternallyTagged::Unit } }
                    => "<Root>\
                            <field>\
                                <Unit/>\
                            </field>\
                        </Root>");
                serialize_as!(newtype:
                    Root { field: Inner { inner: ExternallyTagged::Newtype(true) } }
                    => "<Root>\
                            <field>\
                                <Newtype>true</Newtype>\
                            </field>\
                        </Root>");
                serialize_as!(tuple:
                    Root { field: Inner { inner: ExternallyTagged::Tuple(42.0, "answer") } }
                    => "<Root>\
                            <field>\
                                <Tuple>42</Tuple>\
                                <Tuple>answer</Tuple>\
                            </field>\
                        </Root>");
                serialize_as!(struct_:
                    Root { field: Inner { inner: ExternallyTagged::Struct {
                        float: 42.0,
                        string: "answer"
                    }}}
                    => "<Root>\
                            <field>\
                                <Struct>\
                                    <float>42</float>\
                                    <string>answer</string>\
                                </Struct>\
                            </field>\
                        </Root>");
                serialize_as!(nested_struct:
                    Root { field: Inner { inner: ExternallyTagged::Holder {
                        nested: Nested { float: 42.0 },
                        string: "answer",
                    }}}
                    => "<Root>\
                            <field>\
                                <Holder>\
                                    <nested>\
                                        <float>42</float>\
                                    </nested>\
                                    <string>answer</string>\
                                </Holder>\
                            </field>\
                        </Root>");
                // NOTE: Cannot be deserialized in roundtrip due to
                // https://github.com/serde-rs/serde/issues/1183
                serialize_as_only!(flatten_struct:
                    Root { field: Inner { inner: ExternallyTaggedWorkaround::Flatten {
                        nested: Nested { float: 42.0 },
                        string: "answer",
                    }}}
                    => "<Root>\
                            <field>\
                                <Flatten>\
                                    <float>42</float>\
                                    <string>answer</string>\
                                </Flatten>\
                            </field>\
                        </Root>");
                serialize_as!(empty_struct:
                    Root { field: Inner { inner: ExternallyTagged::Empty {} } }
                    => "<Root>\
                            <field>\
                                <Empty/>\
                            </field>\
                        </Root>");
                serialize_as!(text:
                    Root { field: Inner { inner: ExternallyTagged::Text {
                        float: 42.0,
                        string: "answer"
                    }}}
                    => "<Root>\
                            <field>\
                                <Text>\
                                    42\
                                    <string>answer</string>\
                                </Text>\
                            </field>\
                        </Root>");
            }

            /// The same tests as in `normal_field`, but enum field renamed to `$text`.
            ///
            /// Text representation of enum is possible only for unit variants.
            ///
            /// Enum representation:
            ///
            /// |Kind   |In `$text` field|
            /// |-------|----------------|
            /// |Unit   |`Unit`          |
            /// |Newtype|Err(Unsupported)|
            /// |Tuple  |Err(Unsupported)|
            /// |Struct |Err(Unsupported)|
            mod text_field {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize, Serialize)]
                struct Root<T> {
                    #[serde(rename = "$text")]
                    field: T,
                }

                serialize_as!(unit:
                    Root { field: ExternallyTagged::Unit }
                    => "<Root>Unit</Root>");
                err!(newtype:
                    Root { field: ExternallyTagged::Newtype(true) }
                    => Unsupported("cannot serialize enum newtype variant `ExternallyTagged::Newtype` as an attribute or text content value"),
                    "<Root");
                err!(tuple:
                    Root { field: ExternallyTagged::Tuple(42.0, "answer") }
                    => Unsupported("cannot serialize enum tuple variant `ExternallyTagged::Tuple` as an attribute or text content value"),
                    "<Root");
                err!(struct_:
                    Root { field: ExternallyTagged::Struct {
                        float: 42.0,
                        string: "answer"
                    }}
                    => Unsupported("cannot serialize enum struct variant `ExternallyTagged::Struct` as an attribute or text content value"),
                    "<Root");
                err!(nested_struct:
                    Root { field: ExternallyTagged::Holder {
                        nested: Nested { float: 42.0 },
                        string: "answer",
                    }}
                    => Unsupported("cannot serialize enum struct variant `ExternallyTagged::Holder` as an attribute or text content value"),
                    "<Root");
                err!(flatten_struct:
                    Root { field: ExternallyTaggedWorkaround::Flatten {
                        nested: Nested { float: 42.0 },
                        string: "answer",
                    }}
                    // Flatten enum struct variants represented as newtype variants containing maps
                    => Unsupported("cannot serialize enum newtype variant `ExternallyTaggedWorkaround::Flatten` as an attribute or text content value"),
                    "<Root");
                err!(empty_struct:
                    Root { field: ExternallyTagged::Empty {} }
                    => Unsupported("cannot serialize enum struct variant `ExternallyTagged::Empty` as an attribute or text content value"),
                    "<Root");
                err!(text:
                    Root { field: ExternallyTagged::Text {
                        float: 42.0,
                        string: "answer"
                    }}
                    => Unsupported("cannot serialize enum struct variant `ExternallyTagged::Text` as an attribute or text content value"),
                    "<Root");
            }

            /// The same tests as in `normal_field2`, but enum field renamed to `$text`.
            ///
            /// Text representation of enum is possible only for unit variants.
            mod text_field2 {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize, Serialize)]
                struct Inner<T> {
                    #[serde(rename = "$text")]
                    inner: T,
                }

                serialize_as!(unit:
                    Root { field: Inner { inner: ExternallyTagged::Unit } }
                    => "<Root><field>Unit</field></Root>");
                err!(newtype:
                    Root { field: Inner { inner: ExternallyTagged::Newtype(true) } }
                    => Unsupported("cannot serialize enum newtype variant `ExternallyTagged::Newtype` as an attribute or text content value"),
                    "<Root");
                err!(tuple:
                    Root { field: Inner { inner: ExternallyTagged::Tuple(42.0, "answer") } }
                    => Unsupported("cannot serialize enum tuple variant `ExternallyTagged::Tuple` as an attribute or text content value"),
                    "<Root");
                err!(struct_:
                    Root { field: Inner { inner: ExternallyTagged::Struct {
                        float: 42.0,
                        string: "answer"
                    }}}
                    => Unsupported("cannot serialize enum struct variant `ExternallyTagged::Struct` as an attribute or text content value"),
                    "<Root");
                err!(nested_struct:
                    Root { field: Inner { inner: ExternallyTagged::Holder {
                        nested: Nested { float: 42.0 },
                        string: "answer",
                    }}}
                    => Unsupported("cannot serialize enum struct variant `ExternallyTagged::Holder` as an attribute or text content value"),
                    "<Root");
                err!(flatten_struct:
                    Root { field: Inner { inner: ExternallyTaggedWorkaround::Flatten {
                        nested: Nested { float: 42.0 },
                        string: "answer",
                    }}}
                    // Flatten enum struct variants represented as newtype variants containing maps
                    => Unsupported("cannot serialize enum newtype variant `ExternallyTaggedWorkaround::Flatten` as an attribute or text content value"),
                    "<Root");
                err!(empty_struct:
                    Root { field: Inner { inner: ExternallyTagged::Empty {} } }
                    => Unsupported("cannot serialize enum struct variant `ExternallyTagged::Empty` as an attribute or text content value"),
                    "<Root");
                err!(text:
                    Root { field: Inner { inner: ExternallyTagged::Text {
                        float: 42.0,
                        string: "answer"
                    }}}
                    => Unsupported("cannot serialize enum struct variant `ExternallyTagged::Text` as an attribute or text content value"),
                    "<Root");
            }
        }

        /// Name `$text` has no special meaning in internally tagged enums
        mod internally_tagged {
            use super::*;
            use pretty_assertions::assert_eq;

            serialize_as!(unit:
                InternallyTagged::Unit
                => "<InternallyTagged>\
                        <tag>Unit</tag>\
                    </InternallyTagged>");
            // serde serializes internally tagged newtype structs by delegating
            // serialization to the inner type and augmenting it with a tag
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(newtype:
                InternallyTagged::Newtype(Nested { float: 4.2 })
                => "<Nested>\
                        <tag>Newtype</tag>\
                        <float>4.2</float>\
                    </Nested>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(struct_:
                InternallyTagged::Struct {
                    float: 42.0,
                    string: "answer"
                }
                => "<InternallyTagged>\
                        <tag>Struct</tag>\
                        <float>42</float>\
                        <string>answer</string>\
                    </InternallyTagged>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(nested_struct:
                InternallyTagged::Holder {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => "<InternallyTagged>\
                        <tag>Holder</tag>\
                        <nested>\
                            <float>42</float>\
                        </nested>\
                        <string>answer</string>\
                    </InternallyTagged>");
            // serde serializes flatten structs as maps, and we do not support
            // serialization of maps without root tag
            err!(flatten_struct:
                InternallyTaggedWorkaround::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => Unsupported("cannot serialize map without defined root tag"));
            serialize_as!(empty_struct:
                InternallyTagged::Empty {}
                => "<InternallyTagged>\
                        <tag>Empty</tag>\
                    </InternallyTagged>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(text:
                InternallyTagged::Text {
                    float: 42.0,
                    string: "answer"
                }
                => "<InternallyTagged>\
                        <tag>Text</tag>\
                        42\
                        <string>answer</string>\
                    </InternallyTagged>");
        }

        /// Name `$text` has no special meaning in adjacently tagged enums
        mod adjacently_tagged {
            use super::*;
            use pretty_assertions::assert_eq;

            serialize_as!(unit:
                AdjacentlyTagged::Unit
                => "<AdjacentlyTagged>\
                        <tag>Unit</tag>\
                    </AdjacentlyTagged>");
            serialize_as!(newtype:
                AdjacentlyTagged::Newtype(true)
                => "<AdjacentlyTagged>\
                        <tag>Newtype</tag>\
                        <content>true</content>\
                    </AdjacentlyTagged>");
            serialize_as!(tuple:
                AdjacentlyTagged::Tuple(42.0, "answer")
                => "<AdjacentlyTagged>\
                        <tag>Tuple</tag>\
                        <content>42</content>\
                        <content>answer</content>\
                    </AdjacentlyTagged>");
            serialize_as!(struct_:
                AdjacentlyTagged::Struct {
                    float: 42.0,
                    string: "answer",
                }
                => "<AdjacentlyTagged>\
                        <tag>Struct</tag>\
                        <content>\
                            <float>42</float>\
                            <string>answer</string>\
                        </content>\
                    </AdjacentlyTagged>");
            serialize_as!(nested_struct:
                AdjacentlyTagged::Holder {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => "<AdjacentlyTagged>\
                        <tag>Holder</tag>\
                        <content>\
                            <nested>\
                                <float>42</float>\
                            </nested>\
                            <string>answer</string>\
                        </content>\
                    </AdjacentlyTagged>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(flatten_struct:
                AdjacentlyTaggedWorkaround::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => "<AdjacentlyTaggedWorkaround>\
                        <tag>Flatten</tag>\
                        <content>\
                            <float>42</float>\
                            <string>answer</string>\
                        </content>\
                    </AdjacentlyTaggedWorkaround>");
            serialize_as!(empty_struct:
                AdjacentlyTagged::Empty {}
                => "<AdjacentlyTagged>\
                        <tag>Empty</tag>\
                        <content/>\
                    </AdjacentlyTagged>");
            serialize_as!(text:
                AdjacentlyTagged::Text {
                    float: 42.0,
                    string: "answer",
                }
                => "<AdjacentlyTagged>\
                        <tag>Text</tag>\
                        <content>\
                            42\
                            <string>answer</string>\
                        </content>\
                    </AdjacentlyTagged>");
        }

        /// Name `$text` has no special meaning in untagged enums
        mod untagged {
            use super::*;
            use pretty_assertions::assert_eq;

            // Until https://github.com/serde-rs/serde/pull/2288 will be merged,
            // some results can be confusing
            err!(unit: Untagged::Unit
                => Unsupported("cannot serialize `()` without defined root tag"));
            err!(newtype: Untagged::Newtype(true)
                => Unsupported("cannot serialize `bool` without defined root tag"));
            err!(tuple: Untagged::Tuple(42.0, "answer")
                => Unsupported("cannot serialize unnamed tuple without defined root tag"));
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(struct_:
                Untagged::Struct {
                    float: 42.0,
                    string: "answer",
                }
                => "<Untagged>\
                        <float>42</float>\
                        <string>answer</string>\
                    </Untagged>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(nested_struct:
                Untagged::Holder {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => "<Untagged>\
                        <nested>\
                            <float>42</float>\
                        </nested>\
                        <string>answer</string>\
                    </Untagged>");
            // serde serializes flatten structs as maps, and we do not support
            // serialization of maps without root tag
            err!(flatten_struct:
                UntaggedWorkaround::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => Unsupported("cannot serialize map without defined root tag"));
            serialize_as!(empty_struct:
                Untagged::Empty {}
                => "<Untagged/>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(text:
                Untagged::Text {
                    float: 42.0,
                    string: "answer"
                }
                => "<Untagged>\
                        42\
                        <string>answer</string>\
                    </Untagged>");
        }
    }

    /// Do not run roundtrip in those tests because the results the same as without indentation
    mod with_indent {
        use super::*;
        use pretty_assertions::assert_eq;

        macro_rules! serialize_as {
            ($name:ident: $data:expr => $expected:literal) => {
                #[test]
                fn $name() {
                    let mut buffer = String::new();
                    let mut ser = Serializer::new(&mut buffer);
                    ser.indent(' ', 2);

                    $data.serialize(ser).unwrap();
                    assert_eq!(buffer, $expected);
                }
            };
        }

        /// Checks that attempt to serialize given `$data` results to a
        /// serialization error `$kind` with `$reason`
        macro_rules! err {
            ($name:ident: $data:expr => $kind:ident($reason:literal)) => {
                #[test]
                fn $name() {
                    let mut buffer = String::new();
                    let ser = Serializer::new(&mut buffer);

                    match $data.serialize(ser) {
                        Err(DeError::$kind(e)) => assert_eq!(e, $reason),
                        e => panic!(
                            "Expected `{}({})`, found `{:?}`",
                            stringify!($kind),
                            $reason,
                            e
                        ),
                    }
                    assert_eq!(buffer, "");
                }
            };
        }

        err!(false_: false => Unsupported("cannot serialize `bool` without defined root tag"));
        err!(true_:  true  => Unsupported("cannot serialize `bool` without defined root tag"));

        err!(i8_:    -42i8                => Unsupported("cannot serialize `i8` without defined root tag"));
        err!(i16_:   -4200i16             => Unsupported("cannot serialize `i16` without defined root tag"));
        err!(i32_:   -42000000i32         => Unsupported("cannot serialize `i32` without defined root tag"));
        err!(i64_:   -42000000000000i64   => Unsupported("cannot serialize `i64` without defined root tag"));
        err!(isize_: -42000000000000isize => Unsupported("cannot serialize `i64` without defined root tag"));

        err!(u8_:    42u8                => Unsupported("cannot serialize `u8` without defined root tag"));
        err!(u16_:   4200u16             => Unsupported("cannot serialize `u16` without defined root tag"));
        err!(u32_:   42000000u32         => Unsupported("cannot serialize `u32` without defined root tag"));
        err!(u64_:   42000000000000u64   => Unsupported("cannot serialize `u64` without defined root tag"));
        err!(usize_: 42000000000000usize => Unsupported("cannot serialize `u64` without defined root tag"));

        serde_if_integer128! {
            err!(i128_: -420000000000000000000000000000i128 => Unsupported("cannot serialize `i128` without defined root tag"));
            err!(u128_:  420000000000000000000000000000u128 => Unsupported("cannot serialize `u128` without defined root tag"));
        }

        err!(f32_: 4.2f32 => Unsupported("cannot serialize `f32` without defined root tag"));
        err!(f64_: 4.2f64 => Unsupported("cannot serialize `f64` without defined root tag"));

        err!(char_non_escaped: 'h'  => Unsupported("cannot serialize `char` without defined root tag"));
        err!(char_lt:          '<'  => Unsupported("cannot serialize `char` without defined root tag"));
        err!(char_gt:          '>'  => Unsupported("cannot serialize `char` without defined root tag"));
        err!(char_amp:         '&'  => Unsupported("cannot serialize `char` without defined root tag"));
        err!(char_apos:        '\'' => Unsupported("cannot serialize `char` without defined root tag"));
        err!(char_quot:        '"'  => Unsupported("cannot serialize `char` without defined root tag"));

        err!(str_non_escaped: "non-escaped string" => Unsupported("cannot serialize `&str` without defined root tag"));
        err!(str_escaped:  "<\"escaped & string'>" => Unsupported("cannot serialize `&str` without defined root tag"));

        err!(bytes: Bytes(b"<\"escaped & bytes'>") => Unsupported("cannot serialize `&[u8]` without defined root tag"));

        serialize_as!(option_none: Option::<Unit>::None => "");
        serialize_as!(option_some: Some(Unit) => "<Unit/>");

        err!(unit: () => Unsupported("cannot serialize `()` without defined root tag"));
        serialize_as!(unit_struct: Unit => "<Unit/>");

        serialize_as!(newtype: Newtype(true) => "<Newtype>true</Newtype>");

        err!(seq: vec![1, 2, 3] => Unsupported("cannot serialize sequence without defined root tag"));
        err!(tuple:
            ("<\"&'>", "with\t\r\n spaces", 3usize)
            => Unsupported("cannot serialize unnamed tuple without defined root tag"));
        serialize_as!(tuple_struct:
            Tuple(42.0, "answer")
            => "<Tuple>42</Tuple>\n\
                <Tuple>answer</Tuple>");

        err!(map:
            BTreeMap::from([("$text", 1), ("_2", 3)])
            => Unsupported("cannot serialize map without defined root tag"));
        serialize_as!(struct_:
            Struct {
                float: 42.0,
                string: "answer"
            }
            => "<Struct>\n  \
                    <float>42</float>\n  \
                    <string>answer</string>\n\
                </Struct>");
        serialize_as!(nested_struct:
            NestedStruct {
                nested: Nested { float: 42.0 },
                string: "answer",
            }
            => "<NestedStruct>\n  \
                    <nested>\n    \
                        <float>42</float>\n  \
                    </nested>\n  \
                    <string>answer</string>\n\
                </NestedStruct>");
        // serde serializes flatten structs as maps, and we do not support
        // serialization of maps without root tag
        err!(flatten_struct:
            FlattenStruct {
                nested: Nested { float: 42.0 },
                string: "answer",
            }
            => Unsupported("cannot serialize map without defined root tag"));
        serialize_as!(empty_struct:
            Empty {}
            => "<Empty/>");
        serialize_as!(text:
            Text {
                float: 42.0,
                string: "answer"
            }
            => "<Text>\n  \
                    42\n  \
                    <string>answer</string>\n\
                </Text>");

        mod enum_ {
            use super::*;

            mod externally_tagged {
                use super::*;
                use pretty_assertions::assert_eq;

                serialize_as!(unit:
                    ExternallyTagged::Unit
                    => "<Unit/>");
                serialize_as!(newtype:
                    ExternallyTagged::Newtype(true)
                    => "<Newtype>true</Newtype>");
                serialize_as!(tuple:
                    ExternallyTagged::Tuple(42.0, "answer")
                    => "<Tuple>42</Tuple>\n\
                        <Tuple>answer</Tuple>");
                serialize_as!(struct_:
                    ExternallyTagged::Struct {
                        float: 42.0,
                        string: "answer"
                    }
                    => "<Struct>\n  \
                            <float>42</float>\n  \
                            <string>answer</string>\n\
                        </Struct>");
                serialize_as!(nested_struct:
                    ExternallyTagged::Holder {
                        nested: Nested { float: 42.0 },
                        string: "answer",
                    }
                    => "<Holder>\n  \
                            <nested>\n    \
                                <float>42</float>\n  \
                            </nested>\n  \
                            <string>answer</string>\n\
                        </Holder>");
                serialize_as!(flatten_struct:
                    ExternallyTaggedWorkaround::Flatten {
                        nested: Nested { float: 42.0 },
                        string: "answer",
                    }
                    => "<Flatten>\n  \
                            <float>42</float>\n  \
                            <string>answer</string>\n\
                        </Flatten>");
                serialize_as!(empty_struct:
                    ExternallyTagged::Empty {}
                    => "<Empty/>");
                serialize_as!(text:
                    ExternallyTagged::Text {
                        float: 42.0,
                        string: "answer"
                    }
                    => "<Text>\n  \
                            42\n  \
                            <string>answer</string>\n\
                        </Text>");

                /// Test serialization of the specially named variant `$text`
                mod text {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize, Serialize)]
                    enum Unit {
                        #[serde(rename = "$text")]
                        Text,
                    }
                    #[derive(Debug, PartialEq, Deserialize, Serialize)]
                    enum Newtype<'a> {
                        #[serde(rename = "$text")]
                        Text(&'a str),
                    }
                    #[derive(Debug, PartialEq, Deserialize, Serialize)]
                    enum Tuple<'a> {
                        #[serde(rename = "$text")]
                        Text(f64, &'a str),
                    }
                    #[derive(Debug, PartialEq, Deserialize, Serialize)]
                    enum Struct<'a> {
                        #[serde(rename = "$text")]
                        Text { float: f64, string: &'a str },
                    }

                    // It is unknown how to exactly serialize unit to a text
                    err!(unit: Unit::Text => Unsupported("cannot serialize enum unit variant `Unit::$text` as text content value"));
                    serialize_as!(newtype: Newtype::Text("newtype text") => "newtype text");
                    // Tuple variant serialized as an `xs:list`
                    serialize_as!(tuple: Tuple::Text(4.2, "newtype text") => "4.2 newtype&#32;text");
                    // Struct variant cannot be directly serialized to a text
                    err!(struct_:
                        Struct::Text {
                            float: 4.2,
                            string: "newtype text",
                        }
                        => Unsupported("cannot serialize enum struct variant `Struct::$text` as text content value"));
                }
            }

            /// Name `$text` has no special meaning in untagged enums
            mod internally_tagged {
                use super::*;
                use pretty_assertions::assert_eq;

                serialize_as!(unit:
                    InternallyTagged::Unit
                    => "<InternallyTagged>\n  \
                            <tag>Unit</tag>\n\
                        </InternallyTagged>");
                // serde serializes internally tagged newtype structs by delegating
                // serialization to the inner type and augmenting it with a tag
                serialize_as!(newtype:
                    InternallyTagged::Newtype(Nested { float: 42.0 })
                    => "<Nested>\n  \
                            <tag>Newtype</tag>\n  \
                            <float>42</float>\n\
                        </Nested>");
                serialize_as!(struct_:
                    InternallyTagged::Struct {
                        float: 42.0,
                        string: "answer"
                    }
                    => "<InternallyTagged>\n  \
                            <tag>Struct</tag>\n  \
                            <float>42</float>\n  \
                            <string>answer</string>\n\
                        </InternallyTagged>");
                serialize_as!(nested_struct:
                    InternallyTagged::Holder {
                        nested: Nested { float: 42.0 },
                        string: "answer",
                    }
                    => "<InternallyTagged>\n  \
                            <tag>Holder</tag>\n  \
                            <nested>\n    \
                                <float>42</float>\n  \
                            </nested>\n  \
                            <string>answer</string>\n\
                        </InternallyTagged>");
                // serde serializes flatten structs as maps, and we do not support
                // serialization of maps without root tag
                err!(flatten_struct:
                    InternallyTaggedWorkaround::Flatten {
                        nested: Nested { float: 42.0 },
                        string: "answer",
                    }
                    => Unsupported("cannot serialize map without defined root tag"));
                serialize_as!(empty_struct:
                    InternallyTagged::Empty {}
                    => "<InternallyTagged>\n  \
                            <tag>Empty</tag>\n\
                        </InternallyTagged>");
                serialize_as!(text:
                    InternallyTagged::Text {
                        float: 42.0,
                        string: "answer"
                    }
                    => "<InternallyTagged>\n  \
                            <tag>Text</tag>\n  \
                            42\n  \
                            <string>answer</string>\n\
                        </InternallyTagged>");
            }

            /// Name `$text` has no special meaning in adjacently tagged enums
            mod adjacently_tagged {
                use super::*;
                use pretty_assertions::assert_eq;

                serialize_as!(unit:
                    AdjacentlyTagged::Unit
                    => "<AdjacentlyTagged>\n  \
                            <tag>Unit</tag>\n\
                        </AdjacentlyTagged>");
                serialize_as!(newtype:
                    AdjacentlyTagged::Newtype(true)
                    => "<AdjacentlyTagged>\n  \
                            <tag>Newtype</tag>\n  \
                            <content>true</content>\n\
                        </AdjacentlyTagged>");
                serialize_as!(tuple:
                    AdjacentlyTagged::Tuple(42.0, "answer")
                    => "<AdjacentlyTagged>\n  \
                            <tag>Tuple</tag>\n  \
                            <content>42</content>\n  \
                            <content>answer</content>\n\
                        </AdjacentlyTagged>");
                serialize_as!(struct_:
                    AdjacentlyTagged::Struct {
                        float: 42.0,
                        string: "answer"
                    }
                    => "<AdjacentlyTagged>\n  \
                            <tag>Struct</tag>\n  \
                            <content>\n    \
                                <float>42</float>\n    \
                                <string>answer</string>\n  \
                            </content>\n\
                        </AdjacentlyTagged>");
                serialize_as!(nested_struct:
                    AdjacentlyTagged::Holder {
                        nested: Nested { float: 42.0 },
                        string: "answer",
                    }
                    => "<AdjacentlyTagged>\n  \
                            <tag>Holder</tag>\n  \
                            <content>\n    \
                                <nested>\n      \
                                    <float>42</float>\n    \
                                </nested>\n    \
                                <string>answer</string>\n  \
                            </content>\n\
                        </AdjacentlyTagged>");
                serialize_as!(flatten_struct:
                    AdjacentlyTaggedWorkaround::Flatten {
                        nested: Nested { float: 42.0 },
                        string: "answer",
                    }
                    => "<AdjacentlyTaggedWorkaround>\n  \
                            <tag>Flatten</tag>\n  \
                            <content>\n    \
                                <float>42</float>\n    \
                                <string>answer</string>\n  \
                            </content>\n\
                        </AdjacentlyTaggedWorkaround>");
                serialize_as!(empty_struct:
                    AdjacentlyTagged::Empty {}
                    => "<AdjacentlyTagged>\n  \
                            <tag>Empty</tag>\n  \
                            <content/>\n\
                        </AdjacentlyTagged>");
                serialize_as!(text:
                    AdjacentlyTagged::Text {
                        float: 42.0,
                        string: "answer"
                    }
                    => "<AdjacentlyTagged>\n  \
                            <tag>Text</tag>\n  \
                            <content>\n    \
                                42\n    \
                                <string>answer</string>\n  \
                            </content>\n\
                        </AdjacentlyTagged>");
            }

            /// Name `$text` has no special meaning in untagged enums
            mod untagged {
                use super::*;
                use pretty_assertions::assert_eq;

                err!(unit: Untagged::Unit
                    => Unsupported("cannot serialize `()` without defined root tag"));
                err!(newtype: Untagged::Newtype(true)
                    => Unsupported("cannot serialize `bool` without defined root tag"));
                err!(tuple: Untagged::Tuple(42.0, "answer")
                    => Unsupported("cannot serialize unnamed tuple without defined root tag"));
                serialize_as!(struct_:
                    Untagged::Struct {
                        float: 42.0,
                        string: "answer",
                    }
                    => "<Untagged>\n  \
                            <float>42</float>\n  \
                            <string>answer</string>\n\
                        </Untagged>");
                serialize_as!(nested_struct:
                    Untagged::Holder {
                        nested: Nested { float: 42.0 },
                        string: "answer",
                    }
                    => "<Untagged>\n  \
                            <nested>\n    \
                                <float>42</float>\n  \
                            </nested>\n  \
                            <string>answer</string>\n\
                        </Untagged>");
                err!(flatten_struct:
                    UntaggedWorkaround::Flatten {
                        nested: Nested { float: 42.0 },
                        string: "answer",
                    }
                    => Unsupported("cannot serialize map without defined root tag"));
                serialize_as!(empty_struct:
                    Untagged::Empty {}
                    => "<Untagged/>");
                serialize_as!(text:
                    Untagged::Text {
                        float: 42.0,
                        string: "answer"
                    }
                    => "<Untagged>\n  \
                            42\n  \
                            <string>answer</string>\n\
                        </Untagged>");
            }
        }
    }
}

mod with_root {
    use super::*;
    use pretty_assertions::assert_eq;

    macro_rules! serialize_as {
        ($name:ident: $data:expr => $expected:literal) => {
            #[test]
            fn $name() {
                serialize_as!(@ $data => $expected);

                // Roundtrip to ensure that serializer corresponds to deserializer
                assert_eq!(
                    $data,
                    from_str($expected).expect("deserialization roundtrip"),
                    "deserialization roundtrip",
                );
            }
        };
        ($name:ident: $data:expr ; $ty:ty => $expected:literal) => {
            #[test]
            fn $name() {
                serialize_as!(@ $data => $expected);

                // Roundtrip to ensure that serializer corresponds to deserializer
                assert_eq!(
                    $data,
                    from_str::<'_, $ty>($expected).expect("deserialization roundtrip"),
                    "deserialization roundtrip",
                );
            }
        };
        (@ $data:expr => $expected:literal) => {
            let mut buffer = String::new();
            let ser = Serializer::with_root(&mut buffer, Some("root")).unwrap();

            $data.serialize(ser).unwrap();
            assert_eq!(buffer, $expected);
        };
    }
    macro_rules! serialize_as_only {
        ($name:ident: $data:expr => $expected:literal) => {
            #[test]
            fn $name() {
                serialize_as!(@ $data => $expected);
            }
        };
    }

    /// Checks that attempt to serialize given `$data` results to a
    /// serialization error `$kind` with `$reason`
    macro_rules! err {
        ($name:ident: $data:expr => $kind:ident($reason:literal)) => {
            #[test]
            fn $name() {
                let mut buffer = String::new();
                let ser = Serializer::with_root(&mut buffer, Some("root")).unwrap();

                match $data.serialize(ser) {
                    Err(DeError::$kind(e)) => assert_eq!(e, $reason),
                    e => panic!(
                        "Expected `{}({})`, found `{:?}`",
                        stringify!($kind),
                        $reason,
                        e
                    ),
                }
                // We can write something before fail
                // assert_eq!(buffer, "");
            }
        };
    }

    serialize_as!(false_: false => "<root>false</root>");
    serialize_as!(true_:  true  => "<root>true</root>");

    serialize_as!(i8_:    -42i8                => "<root>-42</root>");
    serialize_as!(i16_:   -4200i16             => "<root>-4200</root>");
    serialize_as!(i32_:   -42000000i32         => "<root>-42000000</root>");
    serialize_as!(i64_:   -42000000000000i64   => "<root>-42000000000000</root>");
    serialize_as!(isize_: -42000000000000isize => "<root>-42000000000000</root>");

    serialize_as!(u8_:    42u8                => "<root>42</root>");
    serialize_as!(u16_:   4200u16             => "<root>4200</root>");
    serialize_as!(u32_:   42000000u32         => "<root>42000000</root>");
    serialize_as!(u64_:   42000000000000u64   => "<root>42000000000000</root>");
    serialize_as!(usize_: 42000000000000usize => "<root>42000000000000</root>");

    serde_if_integer128! {
        serialize_as!(i128_: -420000000000000000000000000000i128 => "<root>-420000000000000000000000000000</root>");
        serialize_as!(u128_:  420000000000000000000000000000u128 => "<root>420000000000000000000000000000</root>");
    }

    serialize_as!(f32_: 4.2f32 => "<root>4.2</root>");
    serialize_as!(f64_: 4.2f64 => "<root>4.2</root>");

    serialize_as!(char_non_escaped: 'h'  => "<root>h</root>");
    serialize_as!(char_lt:          '<'  => "<root>&lt;</root>");
    serialize_as!(char_gt:          '>'  => "<root>&gt;</root>");
    serialize_as!(char_amp:         '&'  => "<root>&amp;</root>");
    serialize_as!(char_apos:        '\'' => "<root>&apos;</root>");
    serialize_as!(char_quot:        '"'  => "<root>&quot;</root>");
    // FIXME: Probably we should trim only for specified types when deserialize
    serialize_as_only!(char_space:       ' '  => "<root> </root>");

    serialize_as!(str_non_escaped: "non-escaped string"; &str => "<root>non-escaped string</root>");
    serialize_as!(str_escaped: "<\"escaped & string'>"; String => "<root>&lt;&quot;escaped &amp; string&apos;&gt;</root>");

    err!(bytes: Bytes(b"<\"escaped & bytes'>") => Unsupported("`serialize_bytes` not supported yet"));

    serialize_as!(option_none: Option::<&str>::None => "");
    serialize_as!(option_some: Some("non-escaped string") => "<root>non-escaped string</root>");

    serialize_as!(unit:
        ()
        => "<root/>");
    serialize_as!(unit_struct:
        Unit
        => "<root/>");

    serialize_as!(newtype:
        Newtype(true)
        => "<root>true</root>");

    serialize_as!(seq:
        vec![1, 2, 3]; Vec<usize>
        => "<root>1</root>\
            <root>2</root>\
            <root>3</root>");
    serialize_as!(tuple:
        // Use to_string() to get owned type that is required for deserialization
        ("<\"&'>".to_string(), "with\t\r\n spaces", 3usize)
        => "<root>&lt;&quot;&amp;&apos;&gt;</root>\
            <root>with\t\r\n spaces</root>\
            <root>3</root>");
    serialize_as!(tuple_struct:
        Tuple(42.0, "answer")
        => "<root>42</root>\
            <root>answer</root>");

    serialize_as!(map:
        BTreeMap::from([("$text", 1), ("_2", 3)])
        => "<root>\
                1\
                <_2>3</_2>\
            </root>");
    serialize_as!(struct_:
        Struct {
            float: 42.0,
            string: "answer"
        }
        => "<root>\
                <float>42</float>\
                <string>answer</string>\
            </root>");
    serialize_as!(nested_struct:
        NestedStruct {
            nested: Nested { float: 42.0 },
            string: "answer",
        }
        => "<root>\
                <nested>\
                    <float>42</float>\
                </nested>\
                <string>answer</string>\
            </root>");
    // NOTE: Cannot be deserialized in roundtrip due to
    // https://github.com/serde-rs/serde/issues/1183
    serialize_as_only!(flatten_struct:
        FlattenStruct {
            nested: Nested { float: 42.0 },
            string: "answer",
        }
        => "<root>\
                <float>42</float>\
                <string>answer</string>\
            </root>");
    serialize_as!(empty_struct:
        Empty {}
        => "<root/>");
    serialize_as!(text:
        Text {
            float: 42.0,
            string: "answer"
        }
        => "<root>\
                42\
                <string>answer</string>\
            </root>");

    mod enum_ {
        use super::*;

        mod externally_tagged {
            use super::*;
            use pretty_assertions::assert_eq;

            serialize_as!(unit:
                ExternallyTagged::Unit
                => "<Unit/>");
            serialize_as!(newtype:
                ExternallyTagged::Newtype(true)
                => "<Newtype>true</Newtype>");
            serialize_as!(tuple:
                ExternallyTagged::Tuple(42.0, "answer")
                => "<Tuple>42</Tuple>\
                    <Tuple>answer</Tuple>");
            serialize_as!(struct_:
                ExternallyTagged::Struct {
                    float: 42.0,
                    string: "answer",
                }
                => "<Struct>\
                        <float>42</float>\
                        <string>answer</string>\
                    </Struct>");
            serialize_as!(nested_struct:
                ExternallyTagged::Holder {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => "<Holder>\
                        <nested>\
                            <float>42</float>\
                        </nested>\
                        <string>answer</string>\
                    </Holder>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(flatten_struct:
                ExternallyTaggedWorkaround::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => "<Flatten>\
                        <float>42</float>\
                        <string>answer</string>\
                    </Flatten>");
            serialize_as!(empty_struct:
                ExternallyTagged::Empty {}
                => "<Empty/>");
            serialize_as!(text:
                ExternallyTagged::Text {
                    float: 42.0,
                    string: "answer"
                }
                => "<Text>\
                        42\
                        <string>answer</string>\
                    </Text>");

            /// Test serialization of the specially named variant `$text`
            mod text {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize, Serialize)]
                enum Unit {
                    #[serde(rename = "$text")]
                    Text,
                }
                #[derive(Debug, PartialEq, Deserialize, Serialize)]
                enum Newtype<'a> {
                    #[serde(rename = "$text")]
                    Text(&'a str),
                }
                #[derive(Debug, PartialEq, Deserialize, Serialize)]
                enum Tuple {
                    #[serde(rename = "$text")]
                    Text(f64, String),
                }
                #[derive(Debug, PartialEq, Deserialize, Serialize)]
                enum Struct<'a> {
                    #[serde(rename = "$text")]
                    Text { float: f64, string: &'a str },
                }

                // It is unknown how to exactly serialize unit to a text
                err!(unit: Unit::Text => Unsupported("cannot serialize enum unit variant `Unit::$text` as text content value"));
                serialize_as!(newtype: Newtype::Text("newtype text") => "newtype text");
                // Tuple variant serialized as an `xs:list`
                serialize_as!(tuple: Tuple::Text(4.2, "newtype-text".into()) => "4.2 newtype-text");
                // Note, that spaces in strings, even escaped, would represent
                // the list item delimiters. Non-symmetric serialization follows
                // tradition: the XmlBeans Java library have the same behavior.
                // See also <https://stackoverflow.com/questions/45494204/escape-space-in-xml-xslist>
                serialize_as_only!(tuple_with_spaces: Tuple::Text(4.2, "newtype text".into()) => "4.2 newtype&#32;text");
                // Struct variant cannot be directly serialized to a text
                err!(struct_:
                    Struct::Text {
                        float: 4.2,
                        string: "newtype text",
                    }
                    => Unsupported("cannot serialize enum struct variant `Struct::$text` as text content value"));
            }
        }

        /// Name `$text` has no special meaning in adjacently tagged enums
        mod internally_tagged {
            use super::*;
            use pretty_assertions::assert_eq;

            serialize_as!(unit:
                InternallyTagged::Unit
                => "<root>\
                        <tag>Unit</tag>\
                    </root>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(newtype:
                InternallyTagged::Newtype(Nested { float: 4.2 })
                => "<root>\
                        <tag>Newtype</tag>\
                        <float>4.2</float>\
                    </root>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(struct_:
                InternallyTagged::Struct {
                    float: 42.0,
                    string: "answer",
                }
                => "<root>\
                        <tag>Struct</tag>\
                        <float>42</float>\
                        <string>answer</string>\
                    </root>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(nested_struct:
                InternallyTagged::Holder {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => "<root>\
                        <tag>Holder</tag>\
                        <nested>\
                            <float>42</float>\
                        </nested>\
                        <string>answer</string>\
                    </root>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(flatten_struct:
                InternallyTaggedWorkaround::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => "<root>\
                        <tag>Flatten</tag>\
                        <float>42</float>\
                        <string>answer</string>\
                    </root>");
            serialize_as!(empty_struct:
                InternallyTagged::Empty {}
                => "<root>\
                        <tag>Empty</tag>\
                    </root>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(text:
                InternallyTagged::Text {
                    float: 42.0,
                    string: "answer"
                }
                => "<root>\
                        <tag>Text</tag>\
                        42\
                        <string>answer</string>\
                    </root>");
        }

        /// Name `$text` has no special meaning in adjacently tagged enums
        mod adjacently_tagged {
            use super::*;
            use pretty_assertions::assert_eq;

            serialize_as!(unit:
                AdjacentlyTagged::Unit
                => "<root>\
                        <tag>Unit</tag>\
                    </root>");
            serialize_as!(newtype:
                AdjacentlyTagged::Newtype(true)
                => "<root>\
                        <tag>Newtype</tag>\
                        <content>true</content>\
                    </root>");
            serialize_as!(tuple:
                AdjacentlyTagged::Tuple(42.0, "answer")
                => "<root>\
                        <tag>Tuple</tag>\
                        <content>42</content>\
                        <content>answer</content>\
                    </root>");
            serialize_as!(struct_:
                AdjacentlyTagged::Struct {
                    float: 42.0,
                    string: "answer",
                }
                => "<root>\
                        <tag>Struct</tag>\
                        <content>\
                            <float>42</float>\
                            <string>answer</string>\
                        </content>\
                    </root>");
            serialize_as!(nested_struct:
                AdjacentlyTagged::Holder {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => "<root>\
                        <tag>Holder</tag>\
                        <content>\
                            <nested>\
                                <float>42</float>\
                            </nested>\
                            <string>answer</string>\
                        </content>\
                    </root>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(flatten_struct:
                AdjacentlyTaggedWorkaround::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => "<root>\
                        <tag>Flatten</tag>\
                        <content>\
                            <float>42</float>\
                            <string>answer</string>\
                        </content>\
                    </root>");
            serialize_as!(empty_struct:
                AdjacentlyTagged::Empty {}
                => "<root>\
                        <tag>Empty</tag>\
                        <content/>\
                    </root>");
            serialize_as!(text:
                AdjacentlyTagged::Text {
                    float: 42.0,
                    string: "answer",
                }
                => "<root>\
                        <tag>Text</tag>\
                        <content>\
                            42\
                            <string>answer</string>\
                        </content>\
                    </root>");
        }

        /// Name `$text` has no special meaning in untagged enums
        mod untagged {
            use super::*;
            use pretty_assertions::assert_eq;

            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(unit:
                Untagged::Unit
                => "<root/>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(newtype:
                Untagged::Newtype(true)
                => "<root>true</root>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(tuple:
                Untagged::Tuple(42.0, "answer")
                => "<root>42</root>\
                    <root>answer</root>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(struct_:
                Untagged::Struct {
                    float: 42.0,
                    string: "answer",
                }
                => "<root>\
                        <float>42</float>\
                        <string>answer</string>\
                    </root>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(nested_struct:
                Untagged::Holder {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => "<root>\
                        <nested>\
                            <float>42</float>\
                        </nested>\
                        <string>answer</string>\
                    </root>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(flatten_struct:
                UntaggedWorkaround::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => "<root>\
                        <float>42</float>\
                        <string>answer</string>\
                    </root>");
            serialize_as!(empty_struct:
                Untagged::Empty {}
                => "<root/>");
            // NOTE: Cannot be deserialized in roundtrip due to
            // https://github.com/serde-rs/serde/issues/1183
            serialize_as_only!(text:
                Untagged::Text {
                    float: 42.0,
                    string: "answer"
                }
                => "<root>\
                        42\
                        <string>answer</string>\
                    </root>");
        }
    }
}

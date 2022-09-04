use quick_xml::se::Serializer;
use quick_xml::utils::Bytes;
use quick_xml::DeError;

use serde::{serde_if_integer128, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize)]
struct Unit;

#[derive(Serialize)]
struct Newtype(bool);

#[derive(Serialize)]
struct Tuple(f32, &'static str);

#[derive(Serialize)]
struct Struct {
    float: f64,
    string: &'static str,
}

#[derive(Serialize)]
struct NestedStruct {
    nested: Nested,
    string: &'static str,
}

#[derive(Serialize)]
struct FlattenStruct {
    #[serde(flatten)]
    nested: Nested,
    string: &'static str,
}

#[derive(Serialize)]
struct Nested {
    float: f64,
}

#[derive(Serialize)]
struct Empty {}

#[derive(Serialize)]
struct Text {
    #[serde(rename = "$text")]
    float: f64,
    string: &'static str,
}

#[derive(Serialize)]
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
    Flatten {
        #[serde(flatten)]
        nested: Nested,
        string: &'static str,
    },
    Empty {},
    Text {
        #[serde(rename = "$text")]
        float: f64,
        string: &'static str,
    },
}

#[derive(Serialize)]
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
    Flatten {
        #[serde(flatten)]
        nested: Nested,
        string: &'static str,
    },
    Empty {},
    Text {
        #[serde(rename = "$text")]
        float: f64,
        string: &'static str,
    },
}

#[derive(Serialize)]
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
    Flatten {
        #[serde(flatten)]
        nested: Nested,
        string: &'static str,
    },
    Empty {},
    Text {
        #[serde(rename = "$text")]
        float: f64,
        string: &'static str,
    },
}

#[derive(Serialize)]
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
    Flatten {
        #[serde(flatten)]
        nested: Nested,
        string: &'static str,
    },
    Empty {},
    Text {
        #[serde(rename = "$text")]
        float: f64,
        string: &'static str,
    },
}

mod without_root {
    use super::*;
    use pretty_assertions::assert_eq;

    macro_rules! serialize_as {
        ($name:ident: $data:expr => $expected:literal) => {
            #[test]
            fn $name() {
                let ser = Serializer::new(String::new());

                assert_eq!($data.serialize(ser).unwrap(), $expected);
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

        mod externally_tagged {
            use super::*;
            use pretty_assertions::assert_eq;

            serialize_as!(unit:
                ExternallyTagged::Unit
                => "<Unit/>");
            serialize_as!(newtype:
                ExternallyTagged::Newtype(true)
                => "<Newtype>true</Newtype>");
            serialize_as!(tuple_struct:
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
            serialize_as!(flatten_struct:
                ExternallyTagged::Flatten {
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
        }

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
            serialize_as!(newtype:
                InternallyTagged::Newtype(Nested { float: 4.2 })
                => "<Nested>\
                        <tag>Newtype</tag>\
                        <float>4.2</float>\
                    </Nested>");
            serialize_as!(struct_:
                InternallyTagged::Struct {
                    float: 42.0,
                    string: "answer"
                }
                => "<InternallyTagged>\
                        <tag>Struct</tag>\
                        <float>42</float>\
                        <string>answer</string>\
                    </InternallyTagged>");
            serialize_as!(nested_struct:
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
                InternallyTagged::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => Unsupported("cannot serialize map without defined root tag"));
            serialize_as!(empty_struct:
                InternallyTagged::Empty {}
                => "<InternallyTagged>\
                        <tag>Empty</tag>\
                    </InternallyTagged>");
            serialize_as!(text:
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
            serialize_as!(tuple_struct:
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
            serialize_as!(flatten_struct:
                AdjacentlyTagged::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => "<AdjacentlyTagged>\
                        <tag>Flatten</tag>\
                        <content>\
                            <float>42</float>\
                            <string>answer</string>\
                        </content>\
                    </AdjacentlyTagged>");
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

        mod untagged {
            use super::*;
            use pretty_assertions::assert_eq;

            // Until https://github.com/serde-rs/serde/pull/2288 will be merged,
            // some results can be confusing
            err!(unit: Untagged::Unit
                => Unsupported("cannot serialize `()` without defined root tag"));
            err!(newtype: Untagged::Newtype(true)
                => Unsupported("cannot serialize `bool` without defined root tag"));
            err!(tuple_struct: Untagged::Tuple(42.0, "answer")
                => Unsupported("cannot serialize unnamed tuple without defined root tag"));
            serialize_as!(struct_:
                Untagged::Struct {
                    float: 42.0,
                    string: "answer",
                }
                => "<Untagged>\
                        <float>42</float>\
                        <string>answer</string>\
                    </Untagged>");
            serialize_as!(nested_struct:
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
            err!(flatten_struct:
                Untagged::Flatten {
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
                => "<Untagged>\
                        42\
                        <string>answer</string>\
                    </Untagged>");
        }
    }

    mod with_indent {
        use super::*;
        use pretty_assertions::assert_eq;

        macro_rules! serialize_as {
            ($name:ident: $data:expr => $expected:literal) => {
                #[test]
                fn $name() {
                    let mut ser = Serializer::new(String::new());
                    ser.indent(' ', 2);

                    assert_eq!($data.serialize(ser).unwrap(), $expected);
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
                serialize_as!(tuple_struct:
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
                    ExternallyTagged::Flatten {
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
            }

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
                    InternallyTagged::Flatten {
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
                serialize_as!(tuple_struct:
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
                    AdjacentlyTagged::Flatten {
                        nested: Nested { float: 42.0 },
                        string: "answer",
                    }
                    => "<AdjacentlyTagged>\n  \
                            <tag>Flatten</tag>\n  \
                            <content>\n    \
                                <float>42</float>\n    \
                                <string>answer</string>\n  \
                            </content>\n\
                        </AdjacentlyTagged>");
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

            mod untagged {
                use super::*;
                use pretty_assertions::assert_eq;

                err!(unit: Untagged::Unit
                    => Unsupported("cannot serialize `()` without defined root tag"));
                err!(newtype: Untagged::Newtype(true)
                    => Unsupported("cannot serialize `bool` without defined root tag"));
                err!(tuple_struct: Untagged::Tuple(42.0, "answer")
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
                    Untagged::Flatten {
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
                let ser = Serializer::with_root(String::new(), Some("root")).unwrap();

                assert_eq!($data.serialize(ser).unwrap(), $expected);
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
    serialize_as!(char_space:       ' '  => "<root> </root>");

    serialize_as!(str_non_escaped: "non-escaped string" => "<root>non-escaped string</root>");
    serialize_as!(str_escaped:  "<\"escaped & string'>" => "<root>&lt;&quot;escaped &amp; string&apos;&gt;</root>");

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
        vec![1, 2, 3]
        => "<root>1</root>\
            <root>2</root>\
            <root>3</root>");
    serialize_as!(tuple:
        ("<\"&'>", "with\t\r\n spaces", 3usize)
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
    serialize_as!(flatten_struct:
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
            serialize_as!(tuple_struct:
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
            serialize_as!(flatten_struct:
                ExternallyTagged::Flatten {
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
        }

        mod internally_tagged {
            use super::*;
            use pretty_assertions::assert_eq;

            serialize_as!(unit:
                InternallyTagged::Unit
                => "<root>\
                        <tag>Unit</tag>\
                    </root>");
            serialize_as!(newtype:
                InternallyTagged::Newtype(Nested { float: 4.2 })
                => "<root>\
                        <tag>Newtype</tag>\
                        <float>4.2</float>\
                    </root>");
            serialize_as!(struct_:
                InternallyTagged::Struct {
                    float: 42.0,
                    string: "answer",
                }
                => "<root>\
                        <tag>Struct</tag>\
                        <float>42</float>\
                        <string>answer</string>\
                    </root>");
            serialize_as!(nested_struct:
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
            serialize_as!(flatten_struct:
                InternallyTagged::Flatten {
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
            serialize_as!(text:
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
            serialize_as!(tuple_struct:
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
            serialize_as!(flatten_struct:
                AdjacentlyTagged::Flatten {
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

        mod untagged {
            use super::*;
            use pretty_assertions::assert_eq;

            serialize_as!(unit:
                Untagged::Unit
                => "<root/>");
            serialize_as!(newtype:
                Untagged::Newtype(true)
                => "<root>true</root>");
            serialize_as!(tuple_struct:
                Untagged::Tuple(42.0, "answer")
                => "<root>42</root>\
                    <root>answer</root>");
            serialize_as!(struct_:
                Untagged::Struct {
                    float: 42.0,
                    string: "answer",
                }
                => "<root>\
                        <float>42</float>\
                        <string>answer</string>\
                    </root>");
            serialize_as!(nested_struct:
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
            serialize_as!(flatten_struct:
                Untagged::Flatten {
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
            serialize_as!(text:
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

use quick_xml::se::{to_string, Serializer};
use quick_xml::writer::Writer;

use pretty_assertions::assert_eq;

use serde::{Serialize, Serializer as SerSerializer};

#[test]
fn serialize_bool() {
    let inputs = [(true, "true"), (false, "false")];

    for (src, should_be) in &inputs {
        let mut buffer = Vec::new();
        let mut ser = Serializer::new(&mut buffer);
        ser.serialize_bool(*src).unwrap();

        assert_eq!(String::from_utf8(buffer).unwrap(), *should_be);
    }
}

#[test]
fn serialize_struct() {
    #[derive(Serialize)]
    struct Person {
        name: String,
        age: u32,
    }

    let bob = Person {
        name: "Bob".to_string(),
        age: 42,
    };

    let mut buffer = Vec::new();
    let mut ser = Serializer::new(&mut buffer);
    bob.serialize(&mut ser).unwrap();

    assert_eq!(
        String::from_utf8(buffer).unwrap(),
        "<Person name=\"Bob\" age=\"42\"/>"
    );
}

#[test]
fn serialize_struct_value_number() {
    #[derive(Serialize)]
    struct Person {
        name: String,
        #[serde(rename = "$value")]
        age: u32,
    }

    let bob = Person {
        name: "Bob".to_string(),
        age: 42,
    };
    assert_eq!(to_string(&bob).unwrap(), "<Person name=\"Bob\">42</Person>");
}

#[test]
fn serialize_struct_value_string() {
    #[derive(Serialize)]
    struct Person {
        name: String,
        #[serde(rename = "$value")]
        age: String,
    }

    let bob = Person {
        name: "Bob".to_string(),
        age: "42".to_string(),
    };
    assert_eq!(to_string(&bob).unwrap(), "<Person name=\"Bob\">42</Person>");
}

#[test]
fn serialize_enum() {
    #[derive(Serialize)]
    #[allow(dead_code)]
    enum Node {
        Boolean(bool),
        Number(f64),
        String(String),
    }

    let mut buffer = Vec::new();
    let mut ser = Serializer::new(&mut buffer);
    let node = Node::Boolean(true);
    node.serialize(&mut ser).unwrap();

    assert_eq!(
        String::from_utf8(buffer).unwrap(),
        "<Boolean>true</Boolean>"
    );
}

#[test]
#[ignore]
fn serialize_a_list() {
    let inputs = vec![1, 2, 3, 4];

    let mut buffer = Vec::new();
    let mut ser = Serializer::new(&mut buffer);
    inputs.serialize(&mut ser).unwrap();

    println!("{}", String::from_utf8(buffer).unwrap());
    panic!();
}

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
struct Value {
    #[serde(rename = "$value")]
    float: f64,
    string: &'static str,
}

#[derive(Serialize)]
enum ExternallyTagged {
    Unit,
    #[serde(rename = "$primitive=PrimitiveUnit")]
    PrimitiveUnit,
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
    Value {
        #[serde(rename = "$value")]
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
    Value {
        #[serde(rename = "$value")]
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
    Value {
        #[serde(rename = "$value")]
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
    Value {
        #[serde(rename = "$value")]
        float: f64,
        string: &'static str,
    },
}

mod with_root {
    use super::*;
    use pretty_assertions::assert_eq;

    macro_rules! serialize_as {
        ($name:ident: $data:expr => $expected:literal) => {
            #[test]
            fn $name() {
                let mut buffer = Vec::new();
                let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

                $data.serialize(&mut ser).unwrap();
                assert_eq!(String::from_utf8(buffer).unwrap(), $expected);
            }
        };
    }

    serialize_as!(unit:
        Unit
        => "<root/>");
    serialize_as!(newtype:
        Newtype(true)
        => "<root>true</root>");
    serialize_as!(tuple:
        (42.0, "answer")
        => "<root>42</root><root>answer</root>");
    serialize_as!(tuple_struct:
        Tuple(42.0, "answer")
        => "<root>42</root><root>answer</root>");
    serialize_as!(struct_:
        Struct {
            float: 42.0,
            string: "answer"
        }
        => r#"<root float="42" string="answer"/>"#);
    serialize_as!(nested_struct:
        NestedStruct {
            nested: Nested { float: 42.0 },
            string: "answer",
        }
        => r#"<root string="answer"><nested float="42"/></root>"#);
    serialize_as!(flatten_struct:
        FlattenStruct {
            nested: Nested { float: 42.0 },
            string: "answer",
        }
        => r#"<root><float>42</float><string>answer</string></root>"#);
    serialize_as!(empty_struct:
        Empty {}
        => "<root/>");
    serialize_as!(value:
        Value {
            float: 42.0,
            string: "answer"
        }
        => r#"<root string="answer">42</root>"#);

    mod enum_ {
        use super::*;

        mod externally_tagged {
            use super::*;
            use pretty_assertions::assert_eq;

            serialize_as!(unit:
                ExternallyTagged::Unit
                => "<Unit/>");
            serialize_as!(primitive_unit:
                ExternallyTagged::PrimitiveUnit
                => "PrimitiveUnit");
            serialize_as!(newtype:
                ExternallyTagged::Newtype(true)
                => "<Newtype>true</Newtype>");
            serialize_as!(tuple_struct:
                ExternallyTagged::Tuple(42.0, "answer")
                => "<Tuple>42</Tuple><Tuple>answer</Tuple>");
            serialize_as!(struct_:
                ExternallyTagged::Struct {
                    float: 42.0,
                    string: "answer",
                }
                => r#"<Struct float="42" string="answer"/>"#);
            serialize_as!(nested_struct:
                ExternallyTagged::Holder {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => r#"<Holder string="answer"><nested float="42"/></Holder>"#);
            serialize_as!(flatten_struct:
                ExternallyTagged::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => r#"<Flatten><float>42</float><string>answer</string></Flatten>"#);
            serialize_as!(empty_struct:
                ExternallyTagged::Empty {}
                => "<Empty/>");
            serialize_as!(value:
                ExternallyTagged::Value {
                    float: 42.0,
                    string: "answer"
                }
                => r#"<Value string="answer">42</Value>"#);
        }

        mod internally_tagged {
            use super::*;
            use pretty_assertions::assert_eq;

            serialize_as!(unit:
                InternallyTagged::Unit
                => r#"<root tag="Unit"/>"#);
            serialize_as!(newtype:
                InternallyTagged::Newtype(Nested { float: 4.2 })
                => r#"<root tag="Newtype" float="4.2"/>"#);
            serialize_as!(struct_:
                InternallyTagged::Struct {
                    float: 42.0,
                    string: "answer",
                }
                => r#"<root tag="Struct" float="42" string="answer"/>"#);
            serialize_as!(nested_struct:
                InternallyTagged::Holder {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => r#"<root tag="Holder" string="answer"><nested float="42"/></root>"#);
            serialize_as!(flatten_struct:
                InternallyTagged::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => r#"<root><tag>Flatten</tag><float>42</float><string>answer</string></root>"#);
            serialize_as!(empty_struct:
                InternallyTagged::Empty {}
                => r#"<root tag="Empty"/>"#);
            serialize_as!(value:
                InternallyTagged::Value {
                    float: 42.0,
                    string: "answer"
                }
                => r#"<root tag="Value" string="answer">42</root>"#);
        }

        mod adjacently_tagged {
            use super::*;
            use pretty_assertions::assert_eq;

            serialize_as!(unit:
                AdjacentlyTagged::Unit
                => r#"<root tag="Unit"/>"#);
            serialize_as!(newtype:
                AdjacentlyTagged::Newtype(true)
                => r#"<root tag="Newtype" content="true"/>"#);
            serialize_as!(tuple_struct:
                AdjacentlyTagged::Tuple(42.0, "answer")
                => r#"<root tag="Tuple"><content>42</content><content>answer</content></root>"#);
            serialize_as!(struct_:
                AdjacentlyTagged::Struct {
                    float: 42.0,
                    string: "answer",
                }
                => r#"<root tag="Struct"><content float="42" string="answer"/></root>"#);
            serialize_as!(nested_struct:
                AdjacentlyTagged::Holder {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => r#"<root tag="Holder"><content string="answer"><nested float="42"/></content></root>"#);
            serialize_as!(flatten_struct:
                AdjacentlyTagged::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => r#"<root tag="Flatten"><content><float>42</float><string>answer</string></content></root>"#);
            serialize_as!(empty_struct:
                AdjacentlyTagged::Empty {}
                => r#"<root tag="Empty"><content/></root>"#);
            serialize_as!(value:
                AdjacentlyTagged::Value {
                    float: 42.0,
                    string: "answer",
                }
                => r#"<root tag="Value"><content string="answer">42</content></root>"#);
        }

        mod untagged {
            use super::*;
            use pretty_assertions::assert_eq;

            serialize_as!(unit:
                Untagged::Unit
                // Unit variant consists just from the tag, and because tags
                // are not written in untagged mode, nothing is written
                => "");
            serialize_as!(newtype:
                Untagged::Newtype(true)
                => "true");
            serialize_as!(tuple_struct:
                Untagged::Tuple(42.0, "answer")
                => "<root>42</root><root>answer</root>");
            serialize_as!(struct_:
                Untagged::Struct {
                    float: 42.0,
                    string: "answer",
                }
                => r#"<root float="42" string="answer"/>"#);
            serialize_as!(nested_struct:
                Untagged::Holder {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => r#"<root string="answer"><nested float="42"/></root>"#);
            serialize_as!(flatten_struct:
                Untagged::Flatten {
                    nested: Nested { float: 42.0 },
                    string: "answer",
                }
                => r#"<root><float>42</float><string>answer</string></root>"#);
            serialize_as!(empty_struct:
                Untagged::Empty {}
                => "<root/>");
            serialize_as!(value:
                Untagged::Value {
                    float: 42.0,
                    string: "answer"
                }
                => r#"<root string="answer">42</root>"#);
        }
    }
}

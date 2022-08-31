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

#[test]
fn unit() {
    #[derive(Serialize)]
    struct Unit;

    let mut buffer = Vec::new();
    let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

    let data = Unit;
    data.serialize(&mut ser).unwrap();
    assert_eq!(String::from_utf8(buffer).unwrap(), "<root/>");
}

#[test]
fn newtype() {
    #[derive(Serialize)]
    struct Newtype(bool);

    let mut buffer = Vec::new();
    let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

    let data = Newtype(true);
    data.serialize(&mut ser).unwrap();
    assert_eq!(String::from_utf8(buffer).unwrap(), "<root>true</root>");
}

#[test]
fn tuple() {
    let mut buffer = Vec::new();
    let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

    let data = (42.0, "answer");
    data.serialize(&mut ser).unwrap();
    assert_eq!(
        String::from_utf8(buffer).unwrap(),
        "<root>42</root><root>answer</root>"
    );
}

#[test]
fn tuple_struct() {
    #[derive(Serialize)]
    struct Tuple(f32, &'static str);

    let mut buffer = Vec::new();
    let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

    let data = Tuple(42.0, "answer");
    data.serialize(&mut ser).unwrap();
    assert_eq!(
        String::from_utf8(buffer).unwrap(),
        "<root>42</root><root>answer</root>"
    );
}

#[test]
fn struct_() {
    #[derive(Serialize)]
    struct Struct {
        float: f64,
        string: String,
    }

    let mut buffer = Vec::new();
    let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

    let node = Struct {
        float: 42.0,
        string: "answer".to_string(),
    };
    node.serialize(&mut ser).unwrap();
    assert_eq!(
        String::from_utf8(buffer).unwrap(),
        r#"<root float="42" string="answer"/>"#
    );
}

#[test]
fn nested_struct() {
    #[derive(Serialize)]
    struct Struct {
        nested: Nested,
        string: String,
    }

    #[derive(Serialize)]
    struct Nested {
        float: f64,
    }

    let mut buffer = Vec::new();
    let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

    let node = Struct {
        nested: Nested { float: 42.0 },
        string: "answer".to_string(),
    };
    node.serialize(&mut ser).unwrap();
    assert_eq!(
        String::from_utf8(buffer).unwrap(),
        r#"<root string="answer"><nested float="42"/></root>"#
    );
}

#[test]
fn flatten_struct() {
    #[derive(Serialize)]
    struct Struct {
        #[serde(flatten)]
        nested: Nested,
        string: String,
    }

    #[derive(Serialize)]
    struct Nested {
        float: f64,
    }

    let mut buffer = Vec::new();
    let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

    let node = Struct {
        nested: Nested { float: 42.0 },
        string: "answer".to_string(),
    };
    node.serialize(&mut ser).unwrap();
    assert_eq!(
        String::from_utf8(buffer).unwrap(),
        r#"<root><float>42</float><string>answer</string></root>"#
    );
}

mod enum_ {
    use super::*;

    #[derive(Serialize)]
    struct Nested {
        float: f64,
    }

    mod externally_tagged {
        use super::*;
        use pretty_assertions::assert_eq;

        #[derive(Serialize)]
        enum Node {
            Unit,
            #[serde(rename = "$primitive=PrimitiveUnit")]
            PrimitiveUnit,
            Newtype(bool),
            Tuple(f64, String),
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

        #[test]
        fn unit() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Unit;
            node.serialize(&mut ser).unwrap();
            assert_eq!(String::from_utf8(buffer).unwrap(), "<Unit/>");
        }

        #[test]
        fn primitive_unit() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::PrimitiveUnit;
            node.serialize(&mut ser).unwrap();
            assert_eq!(String::from_utf8(buffer).unwrap(), "PrimitiveUnit");
        }

        #[test]
        fn newtype() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Newtype(true);
            node.serialize(&mut ser).unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                "<Newtype>true</Newtype>"
            );
        }

        #[test]
        fn struct_() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Struct {
                float: 42.0,
                string: "answer".to_string(),
            };
            node.serialize(&mut ser).unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                r#"<Struct float="42" string="answer"/>"#
            );
        }

        #[test]
        fn tuple_struct() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Tuple(42.0, "answer".to_string());
            node.serialize(&mut ser).unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                "<Tuple>42</Tuple><Tuple>answer</Tuple>"
            );
        }

        #[test]
        fn nested_struct() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Holder {
                nested: Nested { float: 42.0 },
                string: "answer".to_string(),
            };
            node.serialize(&mut ser).unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                r#"<Holder string="answer"><nested float="42"/></Holder>"#
            );
        }

        #[test]
        fn flatten_struct() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Flatten {
                nested: Nested { float: 42.0 },
                string: "answer".to_string(),
            };
            node.serialize(&mut ser).unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                r#"<Flatten><float>42</float><string>answer</string></Flatten>"#
            );
        }
    }

    mod internally_tagged {
        use super::*;
        use pretty_assertions::assert_eq;

        #[derive(Serialize)]
        #[serde(tag = "tag")]
        enum Node {
            Unit,
            /// Primitives (such as `bool`) are not supported by the serde in the internally tagged mode
            Newtype(NewtypeContent),
            // Tuple(f64, String),// Tuples are not supported in the internally tagged mode
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

        #[derive(Serialize)]
        struct NewtypeContent {
            value: bool,
        }

        #[test]
        fn unit() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Unit;
            node.serialize(&mut ser).unwrap();
            assert_eq!(String::from_utf8(buffer).unwrap(), r#"<root tag="Unit"/>"#);
        }

        #[test]
        fn newtype() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Newtype(NewtypeContent { value: true });
            node.serialize(&mut ser).unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                r#"<root tag="Newtype" value="true"/>"#
            );
        }

        #[test]
        fn struct_() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Struct {
                float: 42.0,
                string: "answer".to_string(),
            };
            node.serialize(&mut ser).unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                r#"<root tag="Struct" float="42" string="answer"/>"#
            );
        }

        #[test]
        fn nested_struct() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Holder {
                nested: Nested { float: 42.0 },
                string: "answer".to_string(),
            };
            node.serialize(&mut ser).unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                r#"<root tag="Holder" string="answer"><nested float="42"/></root>"#
            );
        }

        #[test]
        fn flatten_struct() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Flatten {
                nested: Nested { float: 42.0 },
                string: "answer".to_string(),
            };
            node.serialize(&mut ser).unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                r#"<root><tag>Flatten</tag><float>42</float><string>answer</string></root>"#
            );
        }
    }

    mod adjacently_tagged {
        use super::*;
        use pretty_assertions::assert_eq;

        #[derive(Serialize)]
        #[serde(tag = "tag", content = "content")]
        enum Node {
            Unit,
            Newtype(bool),
            Tuple(f64, String),
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

        #[test]
        fn unit() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Unit;
            node.serialize(&mut ser).unwrap();
            assert_eq!(String::from_utf8(buffer).unwrap(), r#"<root tag="Unit"/>"#);
        }

        #[test]
        fn newtype() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Newtype(true);
            node.serialize(&mut ser).unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                r#"<root tag="Newtype" content="true"/>"#
            );
        }

        #[test]
        fn tuple_struct() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Tuple(42.0, "answer".to_string());
            node.serialize(&mut ser).unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                r#"<root tag="Tuple"><content>42</content><content>answer</content></root>"#
            );
        }

        #[test]
        fn struct_() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Struct {
                float: 42.0,
                string: "answer".to_string(),
            };
            node.serialize(&mut ser).unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                r#"<root tag="Struct"><content float="42" string="answer"/></root>"#
            );
        }

        #[test]
        fn nested_struct() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Holder {
                nested: Nested { float: 42.0 },
                string: "answer".to_string(),
            };
            node.serialize(&mut ser).unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                r#"<root tag="Holder"><content string="answer"><nested float="42"/></content></root>"#
            );
        }

        #[test]
        fn flatten_struct() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Flatten {
                nested: Nested { float: 42.0 },
                string: "answer".to_string(),
            };
            node.serialize(&mut ser).unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                r#"<root tag="Flatten"><content><float>42</float><string>answer</string></content></root>"#
            );
        }
    }

    mod untagged {
        use super::*;
        use pretty_assertions::assert_eq;

        #[derive(Serialize)]
        #[serde(untagged)]
        enum Node {
            Unit,
            Newtype(bool),
            Tuple(f64, String),
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

        #[test]
        fn unit() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Unit;
            node.serialize(&mut ser).unwrap();
            // Unit variant consists just from the tag, and because tags
            // are not written in untagged mode, nothing is written
            assert_eq!(String::from_utf8(buffer).unwrap(), "");
        }

        #[test]
        fn newtype() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Newtype(true);
            node.serialize(&mut ser).unwrap();
            assert_eq!(String::from_utf8(buffer).unwrap(), "true");
        }

        #[test]
        fn tuple_struct() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Tuple(42.0, "answer".to_string());
            node.serialize(&mut ser).unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                "<root>42</root><root>answer</root>"
            );
        }

        #[test]
        fn struct_() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Struct {
                float: 42.0,
                string: "answer".to_string(),
            };
            node.serialize(&mut ser).unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                r#"<root float="42" string="answer"/>"#
            );
        }

        #[test]
        fn nested_struct() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Holder {
                nested: Nested { float: 42.0 },
                string: "answer".to_string(),
            };
            node.serialize(&mut ser).unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                r#"<root string="answer"><nested float="42"/></root>"#
            );
        }

        #[test]
        fn flatten_struct() {
            let mut buffer = Vec::new();
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));

            let node = Node::Flatten {
                nested: Nested { float: 42.0 },
                string: "answer".to_string(),
            };
            node.serialize(&mut ser).unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                r#"<root><float>42</float><string>answer</string></root>"#
            );
        }
    }
}

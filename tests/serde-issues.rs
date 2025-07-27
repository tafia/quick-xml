//! Regression tests found in various issues with serde integration.
//!
//! Name each module / test as `issue<GH number>` and keep sorted by issue number

use pretty_assertions::assert_eq;
use quick_xml::de::{from_reader, from_str};
use quick_xml::se::{to_string, to_string_with_root, Serializer};
use serde::de::{Deserializer, IgnoredAny};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::BufReader;

/// Regression tests for https://github.com/tafia/quick-xml/issues/252.
mod issue252 {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn attributes() {
        #[derive(Debug, Deserialize, Serialize, PartialEq)]
        struct OptionalAttributes {
            #[serde(rename = "@a")]
            a: Option<&'static str>,

            #[serde(rename = "@b")]
            #[serde(skip_serializing_if = "Option::is_none")]
            b: Option<&'static str>,
        }

        // Writing `a=""` for a `None` we reflects serde_json behavior which also
        // writes `a: null` for `None`, and reflect they deserialization asymmetry
        let xml = r#"<OptionalAttributes a=""/>"#;
        assert_eq!(
            to_string(&OptionalAttributes { a: None, b: None }).unwrap(),
            xml
        );
        assert_eq!(
            from_str::<OptionalAttributes>(xml).unwrap(),
            OptionalAttributes {
                a: Some(""),
                b: None
            }
        );

        let value = OptionalAttributes {
            a: Some(""),
            b: Some(""),
        };
        let xml = r#"<OptionalAttributes a="" b=""/>"#;
        assert_eq!(to_string(&value).unwrap(), xml);
        assert_eq!(from_str::<OptionalAttributes>(xml).unwrap(), value);

        let value = OptionalAttributes {
            a: Some("a"),
            b: Some("b"),
        };
        let xml = r#"<OptionalAttributes a="a" b="b"/>"#;
        assert_eq!(to_string(&value).unwrap(), xml);
        assert_eq!(from_str::<OptionalAttributes>(xml).unwrap(), value);
    }

    #[test]
    fn elements() {
        #[derive(Debug, Deserialize, Serialize, PartialEq)]
        struct OptionalElements {
            a: Option<&'static str>,

            #[serde(skip_serializing_if = "Option::is_none")]
            b: Option<&'static str>,
        }

        // Writing `<a/>` for a `None` we reflects serde_json behavior which also
        // writes `a: null` for `None`, and reflect they deserialization asymmetry
        let xml = "<OptionalElements><a/></OptionalElements>";
        assert_eq!(
            to_string(&OptionalElements { a: None, b: None }).unwrap(),
            xml
        );
        assert_eq!(
            from_str::<OptionalElements>(xml).unwrap(),
            OptionalElements {
                a: Some(""),
                b: None
            }
        );

        let value = OptionalElements {
            a: Some(""),
            b: Some(""),
        };
        let xml = "<OptionalElements><a/><b/></OptionalElements>";
        assert_eq!(to_string(&value).unwrap(), xml);
        assert_eq!(from_str::<OptionalElements>(xml).unwrap(), value);

        let value = OptionalElements {
            a: Some("a"),
            b: Some("b"),
        };
        let xml = "<OptionalElements><a>a</a><b>b</b></OptionalElements>";
        assert_eq!(to_string(&value).unwrap(), xml);
        assert_eq!(from_str::<OptionalElements>(xml).unwrap(), value);
    }
}

/// Regression test for https://github.com/tafia/quick-xml/issues/343.
#[test]
fn issue343() {
    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct Users {
        users: HashMap<String, User>,
    }
    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct Max(u16);

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct User {
        max: Max,
    }

    let xml = "<Users>\
                        <users>\
                            <roger>\
                                <max>10</max>\
                            </roger>\
                        </users>\
                    </Users>";
    let users: Users = from_str(xml).unwrap();

    assert_eq!(
        users,
        Users {
            users: HashMap::from([("roger".to_string(), User { max: Max(10) })]),
        }
    );
    assert_eq!(to_string(&users).unwrap(), xml);
}

/// Regression test for https://github.com/tafia/quick-xml/issues/349.
#[test]
fn issue349() {
    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct Entity {
        id: Id,
    }
    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct Id {
        #[serde(rename = "$value")]
        content: Enum,
    }
    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    #[serde(rename_all = "kebab-case")]
    enum Enum {
        A(String),
        B(String),
    }

    assert_eq!(
        from_str::<Entity>("<entity><id><a>Id</a></id></entity>").unwrap(),
        Entity {
            id: Id {
                content: Enum::A("Id".to_string()),
            }
        }
    );
}

/// Regression test for https://github.com/tafia/quick-xml/issues/352.
#[test]
fn issue352() {
    use std::borrow::Cow;

    #[derive(Deserialize)]
    struct Root<'a> {
        #[serde(borrow)]
        #[serde(rename = "@attribute")]
        attribute: Cow<'a, str>,
    }

    let r: Root = from_str("<Root attribute='borrowed value'></Root>").unwrap();

    assert!(matches!(r.attribute, Cow::Borrowed(_)));
}

/// Regression test for https://github.com/tafia/quick-xml/issues/429.
#[test]
fn issue429() {
    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    enum State {
        A,
        B,
        C,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct StateOuter {
        #[serde(rename = "$text")]
        state: State,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    pub struct Root {
        state: StateOuter,
    }

    assert_eq!(
        from_str::<Root>("<root><state>B</state></root>").unwrap(),
        Root {
            state: StateOuter { state: State::B }
        }
    );

    assert_eq!(
        to_string(&Root {
            state: StateOuter { state: State::B }
        })
        .unwrap(),
        "<Root><state>B</state></Root>"
    );
}

/// Regression test for https://github.com/tafia/quick-xml/issues/497.
#[test]
fn issue497() {
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct Player {
        #[serde(skip_serializing_if = "Option::is_none")]
        spawn_forced: Option<bool>,
    }
    let data = Player { spawn_forced: None };

    let deserialize_buffer = to_string(&data).unwrap();
    dbg!(&deserialize_buffer);

    let p: Player = from_reader(deserialize_buffer.as_bytes()).unwrap();
    assert_eq!(p, data);
}

/// Regression test for https://github.com/tafia/quick-xml/issues/500.
#[test]
fn issue500() {
    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct TagOne {}

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct TagTwo {}

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    enum Tag {
        TagOne(TagOne),
        TagTwo(TagTwo),
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct Root {
        #[serde(rename = "$value", default)]
        data: Vec<Tag>,
    }

    let data: Root = from_str(
        "\
        <root>\
            <TagOne></TagOne>\
            <TagTwo></TagTwo>\
            <TagOne></TagOne>\
        </root>\
    ",
    )
    .unwrap();

    assert_eq!(
        data,
        Root {
            data: vec![
                Tag::TagOne(TagOne {}),
                Tag::TagTwo(TagTwo {}),
                Tag::TagOne(TagOne {}),
            ],
        }
    );

    let data: Vec<Tag> = from_str(
        "\
        <TagOne></TagOne>\
        <TagTwo></TagTwo>\
        <TagOne></TagOne>\
    ",
    )
    .unwrap();

    assert_eq!(
        data,
        vec![
            Tag::TagOne(TagOne {}),
            Tag::TagTwo(TagTwo {}),
            Tag::TagOne(TagOne {}),
        ]
    );
}

/// Regression test for https://github.com/tafia/quick-xml/issues/510.
#[test]
fn issue510() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(rename = "ENTRY")]
    struct Entry {
        #[serde(rename = "CUE_V2")]
        cues: Option<Vec<Cue>>,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    // #[serde_with::serde_as]
    struct Cue {
        #[serde(rename = "@NAME")]
        name: String,
    }

    let data: Entry = from_str(
        "\
        <ENTRY>\
            <CUE_V2 NAME='foo'></CUE_V2>\
            <CUE_V2 NAME='bar'></CUE_V2>\
        </ENTRY>\
    ",
    )
    .unwrap();

    assert_eq!(
        data,
        Entry {
            cues: Some(vec![
                Cue {
                    name: "foo".to_string(),
                },
                Cue {
                    name: "bar".to_string(),
                },
            ]),
        }
    );
}

/// Regression test for https://github.com/tafia/quick-xml/issues/533.
#[test]
fn issue533() {
    #[derive(Deserialize)]
    struct X {}

    let xml = "<!DOCTYPE X [<!-- -->]><X></X>";
    let reader = BufReader::with_capacity(20, xml.as_bytes());
    let _: X = from_reader(reader).unwrap();
}

/// Regression test for https://github.com/tafia/quick-xml/issues/537.
///
/// This test checks that special `xmlns:xxx` attributes uses full name of
/// an attribute (xmlns:xxx) as a field name instead of just local name of
/// an attribute (xxx)
mod issue537 {
    use super::*;
    use pretty_assertions::assert_eq;

    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    struct Bindings<'a> {
        /// Default namespace binding
        #[serde(rename = "@xmlns")]
        xmlns: &'a str,

        /// Named namespace binding
        #[serde(rename = "@xmlns:named")]
        xmlns_named: &'a str,

        /// Usual attribute
        #[serde(rename = "@attribute")]
        attribute: &'a str,
    }

    #[test]
    fn de() {
        assert_eq!(
            from_str::<Bindings>(
                r#"<Bindings xmlns="default" xmlns:named="named" attribute="attribute"/>"#
            )
            .unwrap(),
            Bindings {
                xmlns: "default",
                xmlns_named: "named",
                attribute: "attribute",
            }
        );
    }

    #[test]
    fn se() {
        assert_eq!(
            to_string(&Bindings {
                xmlns: "default",
                xmlns_named: "named",
                attribute: "attribute",
            })
            .unwrap(),
            r#"<Bindings xmlns="default" xmlns:named="named" attribute="attribute"/>"#
        );
    }
}

/// Regression test for https://github.com/tafia/quick-xml/issues/540.
#[test]
fn issue540() {
    #[derive(Serialize)]
    pub enum Enum {
        Variant {},
    }

    #[derive(Serialize)]
    pub struct Struct {
        #[serde(flatten)]
        flatten: Enum,
    }

    assert_eq!(
        to_string_with_root(
            "root",
            &Struct {
                flatten: Enum::Variant {},
            }
        )
        .unwrap(),
        "<root><Variant/></root>"
    );
}

/// Regression test for https://github.com/tafia/quick-xml/issues/567.
#[test]
fn issue567() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Root {
        #[serde(rename = "$value")]
        items: Vec<Enum>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    enum Enum {
        List(Vec<()>),
    }

    assert_eq!(
        from_str::<Root>("<root><List/></root>").unwrap(),
        Root {
            items: vec![Enum::List(vec![])],
        }
    );
}

/// Regression test for https://github.com/tafia/quick-xml/issues/580.
#[test]
fn issue580() {
    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct Seq {
        #[serde(rename = "$value")]
        items: Vec<Wrapper>,
    }

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct Wrapper(#[serde(deserialize_with = "Item::parse")] Item);

    #[derive(Debug, PartialEq, Eq)]
    struct Item;
    impl Item {
        fn parse<'de, D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            // We should consume something from the deserializer, otherwise this
            // leads to infinity loop
            IgnoredAny::deserialize(deserializer)?;
            Ok(Item)
        }
    }

    assert_eq!(
        from_str::<Seq>(
            r#"
        <Seq>
            <One/>
            <Two/>
        </Seq>"#
        )
        .unwrap(),
        Seq {
            items: vec![Wrapper(Item), Wrapper(Item)],
        }
    );
}

/// Regression test for https://github.com/tafia/quick-xml/issues/655
#[test]
fn issue655() {
    #[derive(Deserialize, Serialize, Debug)]
    pub struct TextureCoordinates {
        #[serde(rename = "@dimension")]
        pub dimension: String,
        #[serde(rename = "@channel")]
        pub channel: String,
        #[serde(rename = "$value")]
        pub elements: String,
    }
    #[derive(Deserialize, Serialize, Debug)]
    #[serde(rename_all = "PascalCase")]
    pub struct VertexBuffer {
        pub positions: String,
        pub normals: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        pub texture_coordinates: Option<TextureCoordinates>,
    }

    let mut buffer = String::new();
    let mut ser = Serializer::with_root(&mut buffer, None).unwrap();
    ser.indent(' ', 2);
    ser.expand_empty_elements(true);

    let obj = VertexBuffer {
        positions: "319.066 -881.28705 7.71589".into(),
        normals: "-0.0195154 -0.21420999 0.976593".into(),
        texture_coordinates: Some(TextureCoordinates {
            dimension: "2D".into(),
            channel: "0".into(),
            elements: "752494 0.201033,0.773967 0.201033".into(),
        }),
    };
    obj.serialize(ser).unwrap();
    assert_eq!(
        buffer,
        "\
<VertexBuffer>
  <Positions>319.066 -881.28705 7.71589</Positions>
  <Normals>-0.0195154 -0.21420999 0.976593</Normals>
  <TextureCoordinates dimension=\"2D\" channel=\"0\">752494 0.201033,0.773967 0.201033</TextureCoordinates>
</VertexBuffer>"
    );
}

/// Regression test for https://github.com/tafia/quick-xml/issues/674
#[test]
fn issue674() {
    #[derive(Debug, Deserialize)]
    struct Any {
        #[serde(rename = "@list")]
        list: Vec<Item>,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    enum Item {
        Foo,
        Bar,
    }

    let any: Any = from_str("<any list='Foo\tBar' />").unwrap();
    assert_eq!(any.list, [Item::Foo, Item::Bar]);
}

/// Regression test for https://github.com/tafia/quick-xml/issues/683.
#[test]
fn issue683() {
    #[derive(Deserialize, Debug, PartialEq)]
    enum ScheduleLocation {
        #[serde(rename = "DT")]
        Destination,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    #[allow(non_snake_case)]
    struct Schedule {
        cancelReason: Option<u32>,
        #[serde(rename = "$value")]
        locations: Vec<ScheduleLocation>,
    }
    let xml = r#"
        <schedule xmlns:ns2="http://www.thalesgroup.com/rtti/PushPort/Schedules/v3">
            <ns2:DT/>
            <ns2:cancelReason>918</ns2:cancelReason>
        </schedule>"#;
    let result = quick_xml::de::from_str::<Schedule>(xml);
    dbg!(&result);
    assert_eq!(
        result.unwrap(),
        Schedule {
            cancelReason: Some(918),
            locations: vec![ScheduleLocation::Destination],
        }
    );
}

/// Regression test for https://github.com/tafia/quick-xml/issues/841.
/// `xml:`-attributes should be able to roundtrip serialization - deserialization
#[test]
fn issue841() {
    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    struct ElementWithXmlLang {
        #[serde(rename = "$text")]
        content: String,
        #[serde(rename = "@xml:lang")]
        language: String,
    }

    let value = ElementWithXmlLang {
        content: "content".to_string(),
        language: "en-US".to_string(),
    };
    let sr = to_string(&value).unwrap();
    let ds: ElementWithXmlLang = from_str(&sr).unwrap();
    assert_eq!(ds, value);
}

/// Regression test for https://github.com/tafia/quick-xml/issues/868.
#[test]
fn issue868() {
    #[derive(Debug, PartialEq, Deserialize)]
    #[serde(rename = "root")]
    pub struct Root {
        #[serde(rename = "key")]
        pub key: Key,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    #[serde(rename = "key")]
    pub struct Key {
        #[serde(rename = "value")]
        pub values: Option<Vec<Value>>,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    #[serde(rename = "Value")]
    pub struct Value {
        #[serde(rename = "@id")]
        pub id: String,

        #[serde(rename = "$text")]
        pub text: Option<String>,
    }
    let xml = r#"
        <?xml version="1.0" encoding="utf-8"?>
        <root>
            <key>
                <value id="1">text1</value>
                <value id="2">text2</value>
                <value id="3">text3</value>
                <value id="4">text4</value>d
                <value id="5">text5</value>
                <value id="6">text6</value>
            </key>
        </root>"#;
    let data = quick_xml::de::from_str::<Root>(xml);
    #[cfg(feature = "overlapped-lists")]
    assert_eq!(
        data.unwrap(),
        Root {
            key: Key {
                values: Some(vec![
                    Value {
                        id: "1".to_string(),
                        text: Some("text1".to_string())
                    },
                    Value {
                        id: "2".to_string(),
                        text: Some("text2".to_string())
                    },
                    Value {
                        id: "3".to_string(),
                        text: Some("text3".to_string())
                    },
                    Value {
                        id: "4".to_string(),
                        text: Some("text4".to_string())
                    },
                    Value {
                        id: "5".to_string(),
                        text: Some("text5".to_string())
                    },
                    Value {
                        id: "6".to_string(),
                        text: Some("text6".to_string())
                    },
                ]),
            },
        }
    );
    #[cfg(not(feature = "overlapped-lists"))]
    match data {
        Err(quick_xml::DeError::Custom(e)) => assert_eq!(e, "duplicate field `value`"),
        e => panic!(
            r#"Expected `Err(Custom("duplicate field `value`"))`, but got `{:?}`"#,
            e
        ),
    }
}

/// Regression test for https://github.com/tafia/quick-xml/pull/888.
#[cfg(feature = "encoding")]
#[test]
fn issue888() {
    #[derive(Debug, PartialEq, Deserialize)]
    struct Root {
        #[serde(rename = "@list")]
        list: Vec<String>,
    }

    let xml = r#"
        <?xml version="1.0" encoding="windows-1251"?>
        <root list="текст требующий декодирования"/>"#;

    let (xml, enc, _) = dbg!(encoding_rs::WINDOWS_1251.encode(xml));
    assert_eq!(
        enc,
        encoding_rs::WINDOWS_1251,
        "windows-1251 should be used for the test"
    );

    let data: Root = quick_xml::de::from_reader(xml.as_ref()).unwrap();
    assert_eq!(
        data,
        Root {
            list: vec![
                // Translation from Russian:
                "текст".to_string(),         // text
                "требующий".to_string(),     // that-requires
                "декодирования".to_string(), // decoding
            ],
        }
    );
}

/// Regression test for https://github.com/tafia/quick-xml/issues/928.
#[cfg(feature = "serde-types")]
#[test]
fn issue928() {
    use quick_xml::impl_deserialize_for_internally_tagged_enum;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Root {
        #[serde(rename = "action")]
        actions: Vec<Action>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(rename_all = "kebab-case")]
    enum Element {
        Node {
            #[serde(rename = "@id")]
            id: u64,
            #[serde(rename = "tag")]
            tags: Option<Vec<Tag>>,
        },
        Way,
        #[serde(other)]
        Other,
    }

    #[derive(Debug, Deserialize, Clone, PartialEq)]
    struct Tag {
        #[serde(rename = "@k")]
        k: String,
        #[serde(rename = "@v")]
        v: String,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct ElementHolder {
        #[serde(rename = "$value")]
        e: Element,
    }

    #[derive(Debug, PartialEq)]
    enum Action {
        Create(ElementHolder),
        Modify {
            old: ElementHolder,
            new: ElementHolder,
        },
        Delete {
            old: ElementHolder,
            new: ElementHolder,
        },
    }

    impl_deserialize_for_internally_tagged_enum! {
        Action, "@type": ["$value", "old", "new"],
        ("create" => Create(ElementHolder)),
        ("modify" => Modify {
            old: ElementHolder,
            new: ElementHolder
        }),
        ("delete" => Delete {
            old: ElementHolder,
            new: ElementHolder
        })
    }

    assert_eq!(
        from_str::<Root>(
            r#"
<root>
  <action type="create">
    <node id="123">
      <tag k="key" v="value"/>
    </node>
  </action>
  <action type="modify">
    <old>
      <node id="456">
        <tag k="key" v="old_value"/>
      </node>
    </old>
    <new>
      <node id="456">
        <tag k="key" v="new_value"/>
      </node>
    </new>
  </action>
  <action type="modify">
    <old>
      <way />
    </old>
    <new>
      <way />
    </new>
  </action>
</root>
     "#
        )
        .unwrap(),
        Root {
            actions: vec![
                Action::Create(ElementHolder {
                    e: Element::Node {
                        id: 123,
                        tags: Some(vec![Tag {
                            k: "key".to_string(),
                            v: "value".to_string(),
                        }]),
                    },
                }),
                Action::Modify {
                    old: ElementHolder {
                        e: Element::Node {
                            id: 456,
                            tags: Some(vec![Tag {
                                k: "key".to_string(),
                                v: "old_value".to_string(),
                            }]),
                        },
                    },
                    new: ElementHolder {
                        e: Element::Node {
                            id: 456,
                            tags: Some(vec![Tag {
                                k: "key".to_string(),
                                v: "new_value".to_string(),
                            }]),
                        },
                    },
                },
                Action::Modify {
                    old: ElementHolder { e: Element::Way },
                    new: ElementHolder { e: Element::Way },
                },
            ],
        },
    );
}

//! Regression tests found in various issues with serde integration.
//!
//! Name each module / test as `issue<GH number>` and keep sorted by issue number

use pretty_assertions::assert_eq;
use quick_xml::de::from_str;
use quick_xml::se::{to_string, to_string_with_root};
use serde::de::{Deserializer, IgnoredAny};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

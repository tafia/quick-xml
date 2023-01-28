//! Regression tests found in various issues with serde integration.
//!
//! Name each module / test as `issue<GH number>` and keep sorted by issue number

use pretty_assertions::assert_eq;
use quick_xml::de::from_str;
use quick_xml::se::to_string;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Regression tests for https://github.com/tafia/quick-xml/issues/252.
mod issue252 {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn attributes() {
        #[derive(Serialize, Debug, PartialEq)]
        struct OptionalAttributes {
            #[serde(rename = "@a")]
            a: Option<&'static str>,

            #[serde(rename = "@b")]
            #[serde(skip_serializing_if = "Option::is_none")]
            b: Option<&'static str>,
        }

        assert_eq!(
            to_string(&OptionalAttributes { a: None, b: None }).unwrap(),
            r#"<OptionalAttributes a=""/>"#
        );
        assert_eq!(
            to_string(&OptionalAttributes {
                a: Some(""),
                b: Some("")
            })
            .unwrap(),
            r#"<OptionalAttributes a="" b=""/>"#
        );
        assert_eq!(
            to_string(&OptionalAttributes {
                a: Some("a"),
                b: Some("b")
            })
            .unwrap(),
            r#"<OptionalAttributes a="a" b="b"/>"#
        );
    }

    #[test]
    fn elements() {
        #[derive(Serialize, Debug, PartialEq)]
        struct OptionalElements {
            a: Option<&'static str>,

            #[serde(skip_serializing_if = "Option::is_none")]
            b: Option<&'static str>,
        }

        assert_eq!(
            to_string(&OptionalElements { a: None, b: None }).unwrap(),
            r#"<OptionalElements><a/></OptionalElements>"#
        );
        assert_eq!(
            to_string(&OptionalElements {
                a: Some(""),
                b: Some("")
            })
            .unwrap(),
            r#"<OptionalElements><a/><b/></OptionalElements>"#
        );
        assert_eq!(
            to_string(&OptionalElements {
                a: Some("a"),
                b: Some("b")
            })
            .unwrap(),
            r#"<OptionalElements><a>a</a><b>b</b></OptionalElements>"#
        );
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

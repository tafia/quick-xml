//! Regression tests found in various issues with serde integration.
//!
//! Name each module / test as `issue<GH number>` and keep sorted by issue number

use quick_xml::de::from_str;
use quick_xml::se::to_string;
use serde::{Deserialize, Serialize};

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

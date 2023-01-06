//! Regression tests found in various issues with serde integration.
//!
//! Name each module / test as `issue<GH number>` and keep sorted by issue number

use quick_xml::de::from_str;
use quick_xml::se::to_string;
use serde::{Deserialize, Serialize};

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

use std::borrow::Cow;
use std::convert::Infallible;

use quick_xml::de::Deserializer;
use quick_xml::events::BytesText;
use quick_xml::reader::{EntityResolver, EntityResolverFactory, ReplacementText};

use pretty_assertions::assert_eq;
use serde::Deserialize;

#[derive(Clone, Copy)]
struct TestEntityResolver {
    capture_called: bool,
}

impl<'i> EntityResolverFactory<'i> for TestEntityResolver {
    type CaptureError = Infallible;
    type Resolver = Self;

    fn new_resolver(&mut self) -> Self::Resolver {
        *self
    }
}

impl<'i> EntityResolver<'i> for TestEntityResolver {
    type CaptureError = Infallible;

    fn capture(&mut self, _doctype: BytesText) -> Result<(), Self::CaptureError> {
        self.capture_called = true;
        Ok(())
    }

    fn resolve<'e>(&self, entity: &str) -> Option<ReplacementText<'i, 'e>> {
        assert!(
            self.capture_called,
            "`EntityResolver::capture` should be called before `EntityResolver::resolve(\"{}\")`",
            entity,
        );

        match dbg!(entity) {
            "text" => Some(ReplacementText::Internal(Cow::Borrowed(
                b"&#x20;<![CDATA[second text]]>&#32;",
            ))),
            _ => Some(ReplacementText::Internal(Cow::Borrowed(
                b"
                <child1 attribute = '&lt;attribute value&gt;'>&text;</child1>
                <child2/>
            ",
            ))),
        }
    }
}

#[derive(Debug, PartialEq, Deserialize)]
struct Root {
    child1: Child1,
    child2: (),
}

#[derive(Debug, PartialEq, Deserialize)]
struct Child1 {
    #[serde(rename = "@attribute")]
    attribute: String,

    #[serde(rename = "$text")]
    text: String,
}

#[test]
fn entities() {
    let mut de = Deserializer::from_str_with_resolver(
        "
        <!DOCTYPE root>
        <root>&entity;</root>
        ",
        TestEntityResolver {
            capture_called: false,
        },
    );

    let data = Root::deserialize(&mut de).unwrap();

    de.check_eof_reached();
    assert_eq!(
        data,
        Root {
            child1: Child1 {
                attribute: "<attribute value>".to_string(),
                text: " second text ".to_string(),
            },
            child2: (),
        }
    );
}

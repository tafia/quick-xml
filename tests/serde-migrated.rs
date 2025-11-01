use std::fmt::Debug;

use quick_xml::de::from_str;
use serde::{de, ser};
use serde::{Deserialize, Serialize};

use pretty_assertions::assert_eq;

#[track_caller]
fn test_parse_ok<'a, T: std::fmt::Debug>(errors: &[(&'a str, T)])
where
    T: PartialEq + Debug + ser::Serialize + for<'de> de::Deserialize<'de>,
{
    for (i, &(s, ref value)) in errors.iter().enumerate() {
        match from_str::<T>(s) {
            Ok(v) => assert_eq!(
                v, *value,
                "{} error, expected: {:?}, found: {:?}",
                i, value, v
            ),
            Err(e) => panic!("{} error, expected {:?}, found error {}", i, value, e),
        }

        // // Make sure we can deserialize into an `Element`.
        // let xml_value: Element = from_str(s).unwrap();

        // // Make sure we can deserialize from an `Element`.
        // let v: T = from_value(xml_value.clone()).unwrap();
        // assert_eq!(v, *value);
    }
}

#[test]
fn test_namespaces() {
    #[derive(PartialEq, Serialize, Deserialize, Debug)]
    struct Envelope {
        subject: String,
    }
    let s = r#"
    <?xml version="1.0" encoding="UTF-8"?>
    <gesmes:Envelope xmlns:gesmes="http://www.gesmes.org/xml/2002-08-01" xmlns="http://www.ecb.int/vocabulary/2002-08-01/eurofxref">
        <gesmes:subject>Reference rates</gesmes:subject>
    </gesmes:Envelope>"#;
    test_parse_ok(&[(
        s,
        Envelope {
            subject: "Reference rates".to_string(),
        },
    )]);
}

#[test]
#[ignore] // FIXME
fn test_forwarded_namespace() {
    #[derive(PartialEq, Serialize, Deserialize, Debug)]
    struct Graphml {
        #[serde(rename = "xsi:schemaLocation")]
        schema_location: String,
    }
    let s = r#"
    <?xml version="1.0" encoding="UTF-8"?>
    <graphml xmlns="http://graphml.graphdrawing.org/xmlns"
        xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
        xsi:schemaLocation="http://graphml.graphdrawing.org/xmlns
        http://graphml.graphdrawing.org/xmlns/1.0/graphml.xsd">
    </graphml>"#;
    test_parse_ok(&[(
        s,
        Graphml {
            schema_location: "http://graphml.graphdrawing.org/xmlns
        http://graphml.graphdrawing.org/xmlns/1.0/graphml.xsd"
                .to_string(),
        },
    )]);
}

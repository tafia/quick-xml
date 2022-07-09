use std::fmt::Debug;

use quick_xml::de::from_str;
use serde::{de, ser};
use serde::{Deserialize, Serialize};

use pretty_assertions::assert_eq;

#[derive(PartialEq, Debug, Serialize, Deserialize)]
struct Simple {
    a: (),
    b: usize,
    c: String,
    d: Option<String>,
}

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

#[track_caller]
fn test_parse_err<'a, T>(errors: &[&'a str])
where
    T: PartialEq + Debug + ser::Serialize + for<'de> de::Deserialize<'de>,
{
    for &s in errors {
        assert!(from_str::<T>(s).is_err());
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

#[test]
fn test_parse_string() {
    test_parse_ok(&[
        (
            "<bla>This is a String</bla>",
            "This is a String".to_string(),
        ),
        ("<bla></bla>", "".to_string()),
        ("<bla>     </bla>", "".to_string()),
        ("<bla>&lt;boom/&gt;</bla>", "<boom/>".to_string()),
        ("<bla>&#9835;</bla>", "♫".to_string()),
        ("<bla>&#x266B;</bla>", "♫".to_string()),
        //(
        //    "<bla>♫<![CDATA[<cookies/>]]>♫</bla>",
        //    "♫<cookies/>♫".to_string(),
        //),
    ]);
}

#[test]
#[ignore] // FIXME
fn test_parse_string_not_trim() {
    test_parse_ok(&[("<bla>     </bla>", "     ".to_string())]);
}

#[test]
#[ignore] // FIXME
fn test_parse_enum() {
    use Animal::*;

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum Animal {
        Dog,
        Frog(String),
        Ant(Simple),
        Cat { age: usize, name: String },
    }

    test_parse_ok(&[
        ("<Animal xsi:type=\"Dog\"/>", Dog),
        (
            "<Animal xsi:type=\"Frog\">Quak</Animal>",
            Frog("Quak".to_string()),
        ),
        (
            "<Animal xsi:type=\"Ant\"><a/><c>bla</c><b>15</b><d>Foo</d></Animal>",
            Ant(Simple {
                a: (),
                b: 15,
                c: "bla".to_string(),
                d: Some("Foo".to_string()),
            }),
        ),
        (
            "<Animal xsi:type=\"Ant\"><a/><c>bla</c><b>15</b></Animal>",
            Ant(Simple {
                a: (),
                b: 15,
                c: "bla".to_string(),
                d: None,
            }),
        ),
        (
            "<Animal xsi:type=\"Cat\"><age>42</age><name>Shere Khan</name></Animal>",
            Cat {
                age: 42,
                name: "Shere Khan".to_string(),
            },
        ),
    ]);

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct Helper {
        x: Animal,
    }

    test_parse_ok(&[
        ("<Helper><x xsi:type=\"Dog\"/></Helper>", Helper { x: Dog }),
        (
            "<Helper><x xsi:type=\"Frog\">Quak</Animal></Helper>",
            Helper {
                x: Frog("Quak".to_string()),
            },
        ),
        (
            "<Helper><x xsi:type=\"Cat\">
                <age>42</age>
                <name>Shere Khan</name>
            </x></Helper>",
            Helper {
                x: Cat {
                    age: 42,
                    name: "Shere Khan".to_string(),
                },
            },
        ),
    ]);
}

#[test]
fn test_option() {
    test_parse_ok(&[
        ("<a/>", Some("".to_string())),
        ("<a></a>", Some("".to_string())),
        ("<a> </a>", Some("".to_string())),
        ("<a>42</a>", Some("42".to_string())),
    ]);
}

#[test]
#[ignore] // FIXME
fn test_option_not_trim() {
    test_parse_ok(&[("<a> </a>", Some(" ".to_string()))]);
}

#[test]
fn test_parse_unfinished() {
    test_parse_err::<Simple>(&["<Simple>
            <c>abc</c>
            <a/>
            <b>2</b>
            <d/>"]);
}

#[test]
fn test_things_qc_found() {
    test_parse_err::<u32>(&["<\u{0}:/"]);
}

#![cfg(feature = "serialize")]

extern crate quick_xml;
extern crate serde;

use std::fmt::Debug;

use quick_xml::de::from_str;
use serde::{de, ser};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Serialize, Deserialize)]
enum Animal {
    Dog,
    Frog(String),
    Ant(Simple),
    Cat { age: usize, name: String },
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
struct Simple {
    a: (),
    b: usize,
    c: String,
    d: Option<String>,
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
struct Inner {
    a: (),
    b: (usize, String, i8),
    c: Vec<String>,
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
struct Outer {
    inner: Option<Inner>,
}

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

fn test_parse_err<'a, T>(errors: &[&'a str])
where
    T: PartialEq + Debug + ser::Serialize + for<'de> de::Deserialize<'de>,
{
    for &s in errors {
        assert!(from_str::<T>(s).is_err());
    }
}

fn test_parse_invalid<'a, T>(errors: &[&'a str])
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
fn test_doctype() {
    #[derive(PartialEq, Serialize, Deserialize, Debug)]
    struct Envelope {
        subject: String,
    }

    test_parse_ok(&[
        (
            r#"
            <?xml version="1.0" encoding="UTF-8"?>
            <!DOCTYPE Envelope>
            <Envelope>
            <subject>Reference rates</subject>
            </Envelope>"#,
            Envelope {
                subject: "Reference rates".to_string(),
            },
        ),
        (
            r#"
            <?xml version="1.0" encoding="UTF-8"?>
            <!DOCTYPE Envelope[]>
            <Envelope>
            <subject>Reference rates</subject>
            </Envelope>"#,
            Envelope {
                subject: "Reference rates".to_string(),
            },
        ),
        (
            r#"
            <?xml version="1.0" encoding="UTF-8"?>
            <!DOCTYPE Envelope [
                <!ELEMENT subject (#PCDATA)>
            ] >
            <Envelope>
            <subject>Reference rates</subject>
            </Envelope>"#,
            Envelope {
                subject: "Reference rates".to_string(),
            },
        ),
    ]);
}

// pass in quick-xml because there is no proper doctype support
//
//
// #[test]
// fn test_doctype_fail() {
//     #[derive(PartialEq, Serialize, Deserialize, Debug)]
//     struct Envelope {
//         subject: String,
//     }
//
//     test_parse_err::<Envelope>(&[
//         r#"
//             <?xml version="1.0" encoding="UTF-8"?>
//             <!DOCTYPE Envelope [
//                 <!ELEMENT subject (#PCDATA)>
//             >
//             <Envelope>
//             <subject>Reference rates</subject>
//             </Envelope>"#,
//         r#"
//             <?xml version="1.0" encoding="UTF-8"?>
//             <Envelope>
//             <subject>Reference rates</subject>
//             <!DOCTYPE Envelope [
//                 <!ELEMENT subject (#PCDATA)>
//             ]>
//             </Envelope>"#,
//     ])
// }

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
    use self::Animal::*;

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
fn test_parse_i64() {
    test_parse_ok(&[
        ("<bla>0</bla>", 0),
        ("<bla>-2</bla>", -2),
        ("<bla>-1234</bla>", -1234),
        ("<bla> -1234 </bla>", -1234),
    ]);
}

#[test]
fn test_parse_u64() {
    test_parse_ok(&[
        ("<bla>0</bla>", 0),
        ("<bla>1234</bla>", 1234),
        ("<bla> 1234 </bla>", 1234),
    ]);
}

#[test]
fn test_parse_bool() {
    test_parse_ok(&[
        ("<bla>true</bla>", true),
        ("<bla>false</bla>", false),
        ("<bla> true </bla>", true),
        ("<bla> false </bla>", false),
        ("<bla>1</bla>", true),
        ("<bla>0</bla>", false),
    ]);

    test_parse_invalid::<bool>(&["<bla>verum</bla>"]);
}

#[test]
fn test_parse_unit() {
    test_parse_ok(&[("<bla/>", ())]);
}

#[test]
fn test_parse_f64() {
    test_parse_ok(&[
        ("<bla>3.0</bla>", 3.0f64),
        ("<bla>3.1</bla>", 3.1),
        ("<bla>-1.2</bla>", -1.2),
        ("<bla>0.4</bla>", 0.4),
        ("<bla>0.4e5</bla>", 0.4e5),
        ("<bla>0.4e15</bla>", 0.4e15),
        ("<bla>0.4e-01</bla>", 0.4e-01), // precision troubles
        ("<bla> 0.4e-01 </bla>", 0.4e-01),
    ]);
}

#[test]
fn test_parse_struct() {
    test_parse_ok(&[
        (
            "<Simple>
                <c>abc</c>
                <a/>
                <b>2</b>
            </Simple>",
            Simple {
                a: (),
                b: 2,
                c: "abc".to_string(),
                d: None,
            },
        ),
        (
            "<Simple><!-- this is a comment -->
                <c>abc</c>
                <a/>
                <b>2</b>
            </Simple>",
            Simple {
                a: (),
                b: 2,
                c: "abc".to_string(),
                d: None,
            },
        ),
        (
            "<Simple d=\"Foo\"><!-- this is a comment -->
                <c>abc</c>
                <a/>
                <b>2</b>
            </Simple>",
            Simple {
                a: (),
                b: 2,
                c: "abc".to_string(),
                d: Some("Foo".to_string()),
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
fn test_amoskvin() {
    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Root {
        foo: Vec<Foo>,
    }

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Foo {
        a: String,
        b: Option<String>,
    }
    test_parse_ok(&[(
        "
<root>
<foo>
 <a>Hello</a>
 <b>World</b>
</foo>
<foo>
 <a>Hi</a>
</foo>
</root>",
        Root {
            foo: vec![
                Foo {
                    a: "Hello".to_string(),
                    b: Some("World".to_string()),
                },
                Foo {
                    a: "Hi".to_string(),
                    b: None,
                },
            ],
        },
    )]);
}

#[test]
#[ignore] // FIXME
fn test_nicolai86() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct TheSender {
        name: String,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct CurrencyCube {
        currency: String,
        rate: String,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    #[allow(non_snake_case)]
    struct InnerCube {
        Cube: Vec<CurrencyCube>,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    #[allow(non_snake_case)]
    struct OuterCube {
        Cube: Vec<InnerCube>,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    #[allow(non_snake_case)]
    struct Envelope {
        subject: String,
        Sender: TheSender,
        Cube: OuterCube,
    }
    test_parse_ok(&[
        (
            r#"
            <?xml version="1.0" encoding="UTF-8"?>
            <gesmes:Envelope xmlns:gesmes="http://www.gesmes.org/xml/2002-08-01" xmlns="http://www.ecb.int/vocabulary/2002-08-01/eurofxref">
                <gesmes:subject>Reference rates</gesmes:subject>
                <gesmes:Sender>
                    <gesmes:name>European Central Bank</gesmes:name>
                </gesmes:Sender>
                <Cube> </Cube>
            </gesmes:Envelope>"#,
            Envelope {
                subject: "Reference rates".to_string(),
                Sender: TheSender {
                    name: "European Central Bank".to_string(),
                },
                Cube: OuterCube { Cube: vec![] },
            },
        ),
        (
            r#"
            <?xml version="1.0" encoding="UTF-8"?>
            <gesmes:Envelope xmlns:gesmes="http://www.gesmes.org/xml/2002-08-01" xmlns="http://www.ecb.int/vocabulary/2002-08-01/eurofxref">
                <gesmes:subject>Reference rates</gesmes:subject>
                <gesmes:Sender>
                    <gesmes:name>European Central Bank</gesmes:name>
                </gesmes:Sender>
                <Cube><Cube>
                    <Cube currency='GBP' rate='0.81725'/>
                    <Cube currency='Latinum' rate='999999'/>
                </Cube></Cube>
            </gesmes:Envelope>"#,
            Envelope {
                subject: "Reference rates".to_string(),
                Sender: TheSender {
                    name: "European Central Bank".to_string(),
                },
                Cube: OuterCube {
                    Cube: vec![InnerCube {
                        Cube: vec![
                            CurrencyCube {
                                currency: "GBP".to_string(),
                                rate: "0.81725".to_string(),
                            },
                            CurrencyCube {
                                currency: "Latinum".to_string(),
                                rate: "999999".to_string(),
                            },
                        ],
                    }],
                },
            },
        ),
    ]);
}

#[test]
fn test_hugo_duncan2() {
    let s = r#"
    <?xml version="1.0" encoding="UTF-8"?>
    <DescribeVpcsResponse xmlns="http://ec2.amazonaws.com/doc/2014-10-01/">
        <requestId>8d521e9a-509e-4ef6-bbb7-9f1ac0d49cd1</requestId>
        <vpcSet>
            <item>
                <vpcId>vpc-ba0d18d8</vpcId>
                <state>available</state>
            </item>
        </vpcSet>
    </DescribeVpcsResponse>"#;
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[allow(non_snake_case)]
    struct VpcSet {
        vpcId: String,
        state: String,
    }

    #[derive(PartialEq, Debug, Serialize)]
    struct ItemVec<T>(Vec<T>);

    impl<'de, T: de::Deserialize<'de>> de::Deserialize<'de> for ItemVec<T> {
        fn deserialize<D>(deserializer: D) -> Result<ItemVec<T>, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            #[derive(PartialEq, Debug, Serialize, Deserialize)]
            struct Helper<U> {
                item: Vec<U>,
            }
            let h: Helper<_> = de::Deserialize::deserialize(deserializer)?;
            Ok(ItemVec(h.item))
        }
    }
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[allow(non_snake_case)]
    struct DescribeVpcsResponse {
        requestId: String,
        vpcSet: ItemVec<VpcSet>,
    }
    test_parse_ok(&[(
        s,
        DescribeVpcsResponse {
            requestId: "8d521e9a-509e-4ef6-bbb7-9f1ac0d49cd1".to_string(),
            vpcSet: ItemVec(vec![VpcSet {
                vpcId: "vpc-ba0d18d8".to_string(),
                state: "available".to_string(),
            }]),
        },
    )]);
}

#[test]
fn test_hugo_duncan() {
    let s = "
        <?xml version=\"1.0\" encoding=\"UTF-8\"?>
        <DescribeInstancesResponse xmlns=\"http://ec2.amazonaws.com/doc/2014-10-01/\">
            <requestId>9474f558-10a5-42e8-84d1-f9ee181fe943</requestId>
            <reservationSet/>
        </DescribeInstancesResponse>
    ";
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[allow(non_snake_case)]
    struct DescribeInstancesResponse {
        requestId: String,
        reservationSet: (),
    }
    test_parse_ok(&[(
        s,
        DescribeInstancesResponse {
            requestId: "9474f558-10a5-42e8-84d1-f9ee181fe943".to_string(),
            reservationSet: (),
        },
    )]);
}

#[test]
fn test_parse_xml_value() {
    #[derive(Eq, Debug, PartialEq, Deserialize, Serialize)]
    struct Test {
        #[serde(rename = "$value")]
        myval: String,
    }
    test_parse_ok(&[(
        "<Test>abc</Test>",
        Test {
            myval: "abc".to_string(),
        },
    )]);
}

#[test]
#[ignore] // FIXME
fn test_parse_complexstruct() {
    test_parse_ok(&[
        (
            "<Outer>
                <inner>
                    <b>2</b>
                    <b>boom</b>
                    <b>88</b>
                </inner>
            </Outer>",
            Outer {
                inner: Some(Inner {
                    a: (),
                    b: (2, "boom".to_string(), 88),
                    c: vec![],
                }),
            },
        ),
        (
            "<Outer>
                <inner>
                    <c>abc</c>
                    <c>xyz</c>
                    <a/>
                    <b>2</b>
                    <b>boom</b>
                    <b>88</b>
                </inner>
            </Outer>",
            Outer {
                inner: Some(Inner {
                    a: (),
                    b: (2, "boom".to_string(), 88),
                    c: vec!["abc".to_string(), "xyz".to_string()],
                }),
            },
        ),
        ("<Outer/>", Outer { inner: None }),
    ]);
}

#[test]
fn test_parse_attributes() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        a1: String,
        #[serde(rename = "$value")]
        a2: i32,
    }

    test_parse_ok(&[(
        r#"<A a1="What is the answer to the ultimate question?">42</A>"#,
        A {
            a1: "What is the answer to the ultimate question?".to_string(),
            a2: 42,
        },
    )]);

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct B {
        b1: String,
        b2: i32,
    }

    test_parse_ok(&[(
        r#"<B b1="What is the answer to the ultimate question?" b2="42"/>"#,
        B {
            b1: "What is the answer to the ultimate question?".to_string(),
            b2: 42,
        },
    )]);

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct C {
        c1: B,
    }

    test_parse_ok(&[
        (
            r#"<C><c1 b1="What is the answer to the ultimate question?" b2="42"/></C>"#,
            C {
                c1: B {
                    b1: "What is the answer to the ultimate question?".to_string(),
                    b2: 42,
                },
            },
        ),
        (
            r#"<C><c1 b1="What is the answer to the ultimate question?" b2="42"/> </C>"#,
            C {
                c1: B {
                    b1: "What is the answer to the ultimate question?".to_string(),
                    b2: 42,
                },
            },
        ),
        (
            r#"<C>  <c1 b1="What is the answer to the ultimate question?" b2="42">
        </c1> </C>"#,
            C {
                c1: B {
                    b1: "What is the answer to the ultimate question?".to_string(),
                    b2: 42,
                },
            },
        ),
    ]);

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct D {
        d1: Option<A>,
    }
    test_parse_ok(&[(
        r#"<D><d1 a1="What is the answer to the ultimate question?">42</d1></D>"#,
        D {
            d1: Some(A {
                a1: "What is the answer to the ultimate question?".to_string(),
                a2: 42,
            }),
        },
    )]);
}

#[test]
#[ignore] // FIXME
fn test_parse_hierarchies() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        a1: String,
        a2: (String, String),
    }
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct B {
        b1: A,
        b2: (A, A),
    }
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct C {
        c1: B,
        c2: Vec<B>,
    }

    test_parse_ok(&[
        (
            "<C><c1>
            <b1>
                <a1>No</a1>
                <a2>Maybe</a2>
                <a2>Yes</a2>
            </b1>
            <b2>
                <a1>Red</a1>
                <a2>Green</a2>
                <a2>Blue</a2>
            </b2>
            <b2>
                <a1>London</a1>
                <a2>Berlin</a2>
                <a2>Paris</a2>
            </b2>
        </c1></C>",
            C {
                c1: B {
                    b1: A {
                        a1: "No".to_string(),
                        a2: ("Maybe".to_string(), "Yes".to_string()),
                    },
                    b2: (
                        A {
                            a1: "Red".to_string(),
                            a2: ("Green".to_string(), "Blue".to_string()),
                        },
                        A {
                            a1: "London".to_string(),
                            a2: ("Berlin".to_string(), "Paris".to_string()),
                        },
                    ),
                },
                c2: vec![],
            },
        ),
        (
            "<C><c1>
            <b2>
                <a2>Green</a2>
                <a2>Blue</a2>
                <a1>Red</a1>
            </b2>
            <b2>
                <a2>Berlin</a2>
                <a2>Paris</a2>
                <a1>London</a1>
            </b2>
            <b1>
                <a2>Maybe</a2>
                <a2>Yes</a2>
                <a1>No</a1>
            </b1>
        </c1></C>",
            C {
                c1: B {
                    b1: A {
                        a1: "No".to_string(),
                        a2: ("Maybe".to_string(), "Yes".to_string()),
                    },
                    b2: (
                        A {
                            a1: "Red".to_string(),
                            a2: ("Green".to_string(), "Blue".to_string()),
                        },
                        A {
                            a1: "London".to_string(),
                            a2: ("Berlin".to_string(), "Paris".to_string()),
                        },
                    ),
                },
                c2: vec![],
            },
        ),
    ]);
}

#[test]
fn unknown_field() {
    #[derive(Deserialize, Debug, PartialEq, Eq, Serialize)]
    struct A {
        other: Vec<Other>,
    }

    #[derive(Deserialize, Debug, PartialEq, Eq, Serialize)]
    struct Other {
        d: i32,
    }
    test_parse_ok(&[(
        "<a>
               <b>
                 <c>5</c>
               </b>
               <other>
                 <d>6</d>
               </other>
            </a>",
        A {
            other: vec![Other { d: 6 }],
        },
    )]);
}

// #[test]
// fn eoz() {
//     use std::io::Read;
//     let mut file = std::fs::File::open("Report_test.2.xml").unwrap();
//     let mut s = String::new();
//     file.read_to_string(&mut s).unwrap();

//     let _xml_value: Element = from_str(&s).unwrap();
// }

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

#[test]
fn futile() {
    #[derive(Eq, PartialEq, Debug, Serialize, Deserialize)]
    struct Object {
        id: u8,
        name: String,
        x: u8,
        y: u8,
        width: u8,
        height: u8,
        ellipse: Option<()>,
    }

    test_parse_ok(&[
        (
            r###"
            <object id="11" name="testEllipse" x="102" y="38" width="21" height="14">
              <ellipse/>
            </object>
            "###,
            Object {
                id: 11,
                name: "testEllipse".to_owned(),
                x: 102,
                y: 38,
                width: 21,
                height: 14,
                ellipse: Some(()),
            },
        ),
        (
            r###"
            <object id="11" name="testEllipse" x="102" y="38" width="21" height="14">
            </object>
            "###,
            Object {
                id: 11,
                name: "testEllipse".to_owned(),
                x: 102,
                y: 38,
                width: 21,
                height: 14,
                ellipse: None,
            },
        ),
    ]);
}

#[test]
fn futile2() {
    #[derive(Eq, PartialEq, Debug, Serialize, Deserialize)]
    struct Null;

    #[derive(Eq, PartialEq, Debug, Serialize, Deserialize)]
    struct Object {
        field: Option<Null>,
    }

    #[derive(Eq, PartialEq, Debug, Serialize, Deserialize)]
    struct Stuff {
        stuff_field: Option<Object>,
    }

    test_parse_ok(&[
        (
            r###"
            <object>
              <field/>
            </object>
            "###,
            Object { field: Some(Null) },
        ),
        (
            r###"
            <object>
            </object>
            "###,
            Object { field: None },
        ),
    ]);

    test_parse_ok(&[
        (
            r###"
            <object>
              <stuff_field/>
            </object>
            "###,
            Stuff {
                stuff_field: Some(Object { field: None }),
            },
        ),
        (
            r###"
            <object>
              <stuff_field>
                <field/>
              </stuff_field>
            </object>
            "###,
            Stuff {
                stuff_field: Some(Object { field: Some(Null) }),
            },
        ),
        (
            r###"
            <object>
            </object>
            "###,
            Stuff { stuff_field: None },
        ),
        (
            r###"
            <object/>
            "###,
            Stuff { stuff_field: None },
        ),
    ]);
}

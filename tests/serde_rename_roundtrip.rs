#![cfg(feature = "serialize")]

extern crate quick_xml;
extern crate serde;

use quick_xml::{de::from_str, se::to_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Nested {
    #[serde(rename="A")]
    a: ItemA,
    #[serde(rename="B")]
    b: ItemB,
    #[serde(rename="C")]
    c: Option<ItemC>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Wrapper {
    #[serde(rename="ItA")]
    A(ItemA),
    #[serde(rename="ItB")]
    B(ItemB),
    #[serde(rename="Nd")]
    Node(Node),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ItemA {
    #[serde(rename="Nm")]
    name: String,
    #[serde(rename="Src")]
    source: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ItemB {
    #[serde(rename="Cnt")]
    cnt: usize,
    #[serde(rename="Nodes")]
    nodes: Nodes,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ItemC {
    #[serde(rename="Wrp")]
    inner: Wrapper,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Node {
    Boolean(bool),
    Identifier { value: String, index: u32 },
    EOF,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Nodes {
    #[serde(rename = "$value")]
    items: Vec<Node>,
}

#[test]
fn basic_struct() {
    let src = r#"<ItemA><Nm>Banana</Nm><Src>Store</Src></ItemA>"#;
    let should_be = ItemA {
        name: "Banana".to_string(),
        source: "Store".to_string(),
    };

    let v: ItemA = from_str(src).unwrap();
    assert_eq!(v, should_be);

    let reserialized_item = to_string(&v).unwrap();
    assert_eq!(src, reserialized_item);
}

#[test]
fn nested_struct() {
    let src = r#"<Nested><A><Nm>Banana</Nm><Src>Store</Src></A><B><Cnt>2</Cnt><Nodes><Boolean>false</Boolean><EOF/></Nodes></B></Nested>"#;
    let should_be = Nested {
        a: ItemA {
            name: "Banana".to_string(),
            source: "Store".to_string(),
        },
        b: ItemB {
            cnt: 2,
            nodes: Nodes {
                items: vec![
                    Node::Boolean(false),
                    Node::EOF,
                ]
            }
        },
        c: None,
    };

    let ser_item = to_string(&should_be).unwrap();
    println!("Serialized: {}", ser_item);

    let v: Nested = from_str(src).unwrap();
    assert_eq!(v, should_be);

    let reserialized_item = to_string(&v).unwrap();
    assert_eq!(src, reserialized_item);
}

#[test]
fn nested_enum() {
    let src = r#"<Nested><A><Nm>Banana</Nm><Src>Store</Src></A><B><Cnt>2</Cnt><Nodes><Boolean>false</Boolean><EOF /></Nodes></B><C><ItA><Nm>Apple</Nm><Src>Orchard</Src></ItA></C></Nested>"#;
    let should_be = Nested {
        a: ItemA {
            name: "Banana".to_string(),
            source: "Store".to_string(),
        },
        b: ItemB {
            cnt: 2,
            nodes: Nodes {
                items: vec![
                    Node::Boolean(false),
                    Node::EOF,
                ]
            }
        },
        c: Some(ItemC {
            inner: Wrapper::A(ItemA {
                name: "Apple".to_string(),
                source: "Orchard".to_string(),
            })
        })
    };

    let ser_item = to_string(&should_be).unwrap();
    println!("Serialized: {}", ser_item);

    let v: Nested = from_str(src).unwrap();
    assert_eq!(v, should_be);

    let reserialized_item = to_string(&v).unwrap();
    assert_eq!(src, reserialized_item);
}

#[test]
fn wrapped_struct() {
    let src = r#"<ItA><Nm>Banana</Nm><Src>Store</Src></ItA>"#;
    let should_be = Wrapper::A(ItemA {
        name: "Banana".to_string(),
        source: "Store".to_string(),
    });

    let v: Wrapper = from_str(src).unwrap();
    assert_eq!(v, should_be);

    let reserialized_item = to_string(&v).unwrap();
    assert_eq!(src, reserialized_item);
}

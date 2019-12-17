#![cfg(feature = "serialize")]

extern crate quick_xml;
extern crate serde;

use quick_xml::{de::from_str, se::to_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Item {
    name: String,
    source: String,
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
    let src = r#"<Item><name>Banana</name><source>Store</source></Item>"#;
    let should_be = Item {
        name: "Banana".to_string(),
        source: "Store".to_string(),
    };

    let item: Item = from_str(src).unwrap();
    assert_eq!(item, should_be);

    let reserialized_item = to_string(&item).unwrap();
    assert_eq!(src, reserialized_item);
}

#[test]
#[ignore]
fn round_trip_list_of_enums() {
    // Construct some inputs
    let nodes = Nodes {
        items: vec![
            Node::Boolean(true),
            Node::Identifier {
                value: "foo".to_string(),
                index: 5,
            },
            Node::EOF,
        ],
    };

    let should_be = r#"
    <Nodes>
        <Boolean>
            true
        </Boolean>
        <Identifier>
            <value>foo</value>
            <index>5</index>
        </Identifier>
        <EOF />
    </Nodes>"#;

    let serialized_nodes = to_string(&nodes).unwrap();
    assert_eq!(serialized_nodes, should_be);

    // Then turn it back into a `Nodes` struct and make sure it's the same
    // as the original
    let deserialized_nodes: Nodes = from_str(serialized_nodes.as_str()).unwrap();
    assert_eq!(deserialized_nodes, nodes);
}

#[test]
fn no_contiguous_fields() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Xml {
        #[serde(rename = "$value")]
        fields: Vec<Field>,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum Field {
        #[serde(rename = "field1")]
        Field1 { name: String },
        #[serde(rename = "field2")]
        Field2 { name: String },
    }

    let source = r#"
<Xml>
    <field1 name='a'/>
    <field2 name='b'/>
    <field1 name='a'/>
</Xml>
"#;

    let xml: Xml = ::quick_xml::de::from_str(source).unwrap();
    assert_eq!(
        xml,
        Xml {
            fields: vec![
                Field::Field1 {
                    name: "a".to_string()
                },
                Field::Field2 {
                    name: "b".to_string()
                },
                Field::Field1 {
                    name: "a".to_string()
                },
            ],
        }
    );

    // TODO: impl Serialize for struct variants
    // let serialized = to_string(&xml).unwrap();
    // assert_eq!(serialized, source);
}

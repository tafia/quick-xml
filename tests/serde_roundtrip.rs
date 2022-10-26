use quick_xml::{de::from_str, se::to_string};
use serde::{Deserialize, Serialize};

use pretty_assertions::assert_eq;

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

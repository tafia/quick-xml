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
    let src = "<Item name=\"Banana\" source=\"Store\"/>";
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

#[test]
fn escapes_in_cdata() {
    #[derive(Debug, Deserialize, PartialEq)]
    pub struct Protocols {
        protocol: Vec<Protocol>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    pub struct Protocol {
        pub name: String,
        pub irp: String,
    }

    // this is from https://github.com/bengtmartensson/IrpTransmogrifier/blob/master/src/main/resources/IrpProtocols.xml
    // no copyright claimed
    let source = r###"<?xml version="1.0" encoding="UTF-8" standalone="no"?>
    <?xml-stylesheet type="text/xsl" href="IrpProtocols2html.xsl"?>

    <irp:protocols xmlns="http://www.w3.org/1999/xhtml"
                   xmlns:rm="https://sourceforge.net/projects/controlremote/files/RemoteMaster"
                   xmlns:xi="http://www.w3.org/2001/XInclude"
                   xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
                   version="2020-09-10"
                   xsi:schemaLocation="http://www.harctoolbox.org/irp-protocols http://www.harctoolbox.org/schemas/irp-protocols.xsd"
                   xmlns:irp="http://www.harctoolbox.org/irp-protocols">
        <irp:protocol name="Amino">
            <irp:irp>
                <![CDATA[{37.3k,268,msb}<-1,1|1,-1>(T=1,(7,-6,3,D:4,1:1,T:1,1:2,0:8,F:8,15:4,C:4,-79m,T=0)+){C =(D:4+4*T+9+F:4+F:4:4+15)&15} [D:0..15,F:0..255]]]>
            </irp:irp>
        </irp:protocol>
    </irp:protocols>"###;

    let protocols: Protocols = from_str(&source).expect("unexpected xml");

    assert_eq!(
        protocols.protocol[0].irp,
        r#"{37.3k,268,msb}<-1,1|1,-1>(T=1,(7,-6,3,D:4,1:1,T:1,1:2,0:8,F:8,15:4,C:4,-79m,T=0)+){C =(D:4+4*T+9+F:4+F:4:4+15)&15} [D:0..15,F:0..255]"#
    );
}

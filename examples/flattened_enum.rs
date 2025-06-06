//! This example demonstrates how to deserialize and serialize enum nodes using an intermediate
//! custom deserializer and seralizer.
//! The `elem` node can either be a `Foo` or a `Bar` node, depending on the `type`.
//! The `type` attribute is used to determine which variant to deserialize.
//! This is a workaround for [serde's issue](https://github.com/serde-rs/serde/issues/1905)
//!
//! note: to use serde, the feature needs to be enabled
//! run example with:
//!    cargo run --example flattened_enum --features="serialize"

use std::fmt;

use quick_xml::de::from_str;
use quick_xml::se::to_string_with_root;
use serde::de::value::MapAccessDeserializer;
use serde::de::{Error, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Model {
    elem: Vec<Elem>,
}

#[derive(Debug, PartialEq)]
enum Elem {
    Foo(Foo),
    Bar(Bar),
}

impl<'de> Deserialize<'de> for Elem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ElemVisitor;

        impl<'de> Visitor<'de> for ElemVisitor {
            type Value = Elem;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an object with a `type` field")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Elem, A::Error>
            where
                A: MapAccess<'de>,
            {
                if let Some((key, value)) = map.next_entry::<String, String>()? {
                    return match key.as_str() {
                        "@type" => match value.as_str() {
                            "foo" => {
                                let f = Foo::deserialize(MapAccessDeserializer::new(map))?;
                                Ok(Elem::Foo(f))
                            }
                            "bar" => {
                                let f = Bar::deserialize(MapAccessDeserializer::new(map))?;
                                Ok(Elem::Bar(f))
                            }
                            t => Err(Error::custom(format!("unknown type attribute `{t}`"))),
                        },
                        a => Err(Error::custom(format!(
                            "expected attribute `type`, but found `{a}`"
                        ))),
                    };
                }
                Err(Error::custom("expected `type` attribute"))
            }
        }
        deserializer.deserialize_map(ElemVisitor)
    }
}

impl Serialize for Elem {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self {
            Elem::Foo(f) => {
                let mut state = serializer.serialize_map(Some(3))?;
                state.serialize_entry("@type", "foo")?;
                state.serialize_entry("a", &f.a)?;
                state.serialize_entry("subfoo", &f.subfoo)?;
                state.end()
            }
            Elem::Bar(b) => {
                let mut state = serializer.serialize_map(Some(2))?;
                state.serialize_entry("@type", "bar")?;
                state.serialize_entry("b", &b.b)?;
                state.end()
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Foo {
    a: String,
    subfoo: SubFoo,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct SubFoo {
    a1: String,
    a2: String,
    a3: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Bar {
    b: String,
}

fn main() {
    let x = r#"
<model>
    <elem type="foo">
        <a>1</a>
        <subfoo>
            <a1>2</a1>
            <a2>42</a2>
            <a3>1337</a3>
        </subfoo>
    </elem>
    <elem type="bar">
        <b>22</b>
    </elem>
</model>
"#;

    let model: Model = from_str(&x).unwrap();
    println!("{:?}", model);
    // Model { elem: [Foo(Foo { a: "1", subfoo: SubFoo { a1: "2", a2: "42", a3: "1337" } }), Bar(Bar { b: "22" })] }

    let x = to_string_with_root("model", &model).unwrap();
    println!("{}", x);
    // <model><elem type="foo"><a>1</a><subfoo><a1>2</a1><a2>42</a2><a3>1337</a3></subfoo></elem><elem type="bar"><b>22</b></elem></model>
}

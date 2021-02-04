//! Module to handle custom serde `Serializer`

mod var;

use self::var::{Map, Seq, Struct, Tuple};
use crate::{
    errors::serialize::DeError,
    events::{BytesEnd, BytesStart, BytesText, Event},
    writer::Writer,
};
use serde::ser::{self, Serialize};
use serde::serde_if_integer128;
use std::io::Write;

/// Serialize struct into a `Write`r
pub fn to_writer<W: Write, S: Serialize>(writer: W, value: &S) -> Result<(), DeError> {
    let mut serializer = Serializer::new(writer);
    value.serialize(&mut serializer)
}

/// Serialize struct into a `String`
pub fn to_string<S: Serialize>(value: &S) -> Result<String, DeError> {
    let mut writer = Vec::new();
    to_writer(&mut writer, value)?;
    let s = String::from_utf8(writer).map_err(|e| crate::errors::Error::Utf8(e.utf8_error()))?;
    Ok(s)
}

/// A Serializer
pub struct Serializer<'r, W: Write> {
    writer: Writer<W>,
    /// Name of the root tag. If not specified, deduced from the structure name
    root_tag: Option<&'r str>,
}

impl<'r, W: Write> Serializer<'r, W> {
    /// Creates a new `Serializer` that uses struct name as a root tag name.
    ///
    /// Note, that attempt to serialize a non-struct (including unit structs
    /// and newtype structs) will end up to an error. Use `with_root` to create
    /// serializer with explicitly defined root element name
    pub fn new(writer: W) -> Self {
        Self::with_root(Writer::new(writer), None)
    }

    /// Creates a new `Serializer` that uses specified root tag name
    ///
    /// # Examples
    ///
    /// When serializing a primitive type, only its representation will be written:
    ///
    /// ```edition2018
    /// # use serde::Serialize;
    /// use quick_xml::Writer;
    /// # use quick_xml::se::Serializer;
    ///
    /// let mut buffer = Vec::new();
    /// let mut writer = Writer::new_with_indent(&mut buffer, b' ', 2);
    /// let mut ser = Serializer::with_root(writer, Some("root"));
    ///
    /// "node".serialize(&mut ser).unwrap();
    /// assert_eq!(String::from_utf8(buffer).unwrap(), "node");
    /// ```
    ///
    /// When serializing a struct, newtype struct, unit struct or tuple `root_tag`
    /// is used as tag name of root(s) element(s):
    ///
    /// ```edition2018
    /// # use serde::Serialize;
    /// use quick_xml::Writer;
    /// use quick_xml::se::Serializer;
    ///
    /// #[derive(Debug, PartialEq, Serialize)]
    /// struct Struct {
    ///     question: String,
    ///     answer: u32,
    /// }
    ///
    /// let mut buffer = Vec::new();
    /// let mut writer = Writer::new_with_indent(&mut buffer, b' ', 2);
    /// let mut ser = Serializer::with_root(writer, Some("root"));
    ///
    /// Struct {
    ///     question: "The Ultimate Question of Life, the Universe, and Everything".into(),
    ///     answer: 42,
    /// }.serialize(&mut ser).unwrap();
    /// assert_eq!(
    ///     String::from_utf8(buffer.clone()).unwrap(),
    ///     r#"<root question="The Ultimate Question of Life, the Universe, and Everything" answer="42"/>"#
    /// );
    /// ```
    pub fn with_root(writer: Writer<W>, root_tag: Option<&'r str>) -> Self {
        Self { writer, root_tag }
    }

    fn write_primitive<P: std::fmt::Display>(
        &mut self,
        value: P,
        escaped: bool,
    ) -> Result<(), DeError> {
        let value = value.to_string().into_bytes();
        let event = if escaped {
            BytesText::from_escaped(value)
        } else {
            BytesText::from_plain(&value)
        };
        self.writer.write_event(Event::Text(event))?;
        Ok(())
    }

    /// Writes self-closed tag `<tag_name/>` into inner writer
    fn write_self_closed(&mut self, tag_name: &str) -> Result<(), DeError> {
        self.writer
            .write_event(Event::Empty(BytesStart::borrowed_name(tag_name.as_bytes())))?;
        Ok(())
    }

    /// Writes a serialized `value` surrounded by `<tag_name>...</tag_name>`
    fn write_paired<T: ?Sized + Serialize>(
        &mut self,
        tag_name: &str,
        value: &T,
    ) -> Result<(), DeError> {
        self.writer
            .write_event(Event::Start(BytesStart::borrowed_name(tag_name.as_bytes())))?;
        value.serialize(&mut *self)?;
        self.writer
            .write_event(Event::End(BytesEnd::borrowed(tag_name.as_bytes())))?;
        Ok(())
    }
}

impl<'r, 'w, W: Write> ser::Serializer for &'w mut Serializer<'r, W> {
    type Ok = ();
    type Error = DeError;

    type SerializeSeq = Seq<'r, 'w, W>;
    type SerializeTuple = Tuple<'r, 'w, W>;
    type SerializeTupleStruct = Tuple<'r, 'w, W>;
    type SerializeTupleVariant = Tuple<'r, 'w, W>;
    type SerializeMap = Map<'r, 'w, W>;
    type SerializeStruct = Struct<'r, 'w, W>;
    type SerializeStructVariant = Struct<'r, 'w, W>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, DeError> {
        self.write_primitive(if v { "true" } else { "false" }, true)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    serde_if_integer128! {
        fn serialize_i128(self, v: i128) -> Result<Self::Ok, DeError> {
            self.write_primitive(v, true)
        }

        fn serialize_u128(self, v: u128) -> Result<Self::Ok, DeError> {
            self.write_primitive(v, true)
        }
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, false)
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok, DeError> {
        self.write_primitive(value, false)
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<Self::Ok, DeError> {
        // TODO: I imagine you'd want to use base64 here.
        // Not sure how to roundtrip effectively though...
        Err(DeError::Unsupported("serialize_bytes"))
    }

    fn serialize_none(self) -> Result<Self::Ok, DeError> {
        Ok(())
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, DeError> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, DeError> {
        self.serialize_none()
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, DeError> {
        self.write_self_closed(self.root_tag.unwrap_or(name))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, DeError> {
        self.write_self_closed(variant)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, DeError> {
        self.write_paired(self.root_tag.unwrap_or(name), value)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, DeError> {
        // Flatten structs in enums are serialized as newtype struct variant + map.
        // As serialize_map should write `root_tag` for ordinal maps (because it's
        // only way for maps), and for enums this method already written a tag name
        // (`variant`), we need to clear root tag before writing content and restore
        // it after
        let root = self.root_tag.take();
        let result = self.write_paired(variant, value);
        self.root_tag = root;
        result
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, DeError> {
        Ok(Seq::new(self))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, DeError> {
        let tag = match self.root_tag {
            Some(tag) => tag,
            None => {
                return Err(DeError::Custom(
                    "root tag name must be specified when serialize unnamed tuple".into(),
                ))
            }
        };
        Ok(Tuple::new(self, tag))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, DeError> {
        Ok(Tuple::new(self, self.root_tag.unwrap_or(name)))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, DeError> {
        Ok(Tuple::new(self, variant))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, DeError> {
        if let Some(tag) = self.root_tag {
            // TODO: Write self-closed tag if map is empty
            self.writer
                .write_event(Event::Start(BytesStart::borrowed_name(tag.as_bytes())))?;
        }
        Ok(Map::new(self))
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, DeError> {
        Ok(Struct::new(self, self.root_tag.unwrap_or(name)))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, DeError> {
        Ok(Struct::new(self, variant))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::ser::SerializeMap;
    use serde::{Serialize, Serializer as SerSerializer};

    #[test]
    fn test_serialize_bool() {
        let inputs = vec![(true, "true"), (false, "false")];

        for (src, should_be) in inputs {
            let mut buffer = Vec::new();

            {
                let mut ser = Serializer::new(&mut buffer);
                ser.serialize_bool(src).unwrap();
            }

            let got = String::from_utf8(buffer).unwrap();
            assert_eq!(got, should_be);
        }
    }

    #[test]
    fn test_serialize_struct() {
        #[derive(Serialize)]
        struct Person {
            name: String,
            age: u32,
        }

        let bob = Person {
            name: "Bob".to_string(),
            age: 42,
        };
        let should_be = "<Person name=\"Bob\" age=\"42\"/>";
        let mut buffer = Vec::new();

        {
            let mut ser = Serializer::new(&mut buffer);
            bob.serialize(&mut ser).unwrap();
        }

        let got = String::from_utf8(buffer).unwrap();
        assert_eq!(got, should_be);
    }

    #[test]
    fn test_serialize_struct_value_number() {
        #[derive(Serialize)]
        struct Person {
            name: String,
            #[serde(rename = "$value")]
            age: u32,
        }

        let bob = Person {
            name: "Bob".to_string(),
            age: 42,
        };
        let should_be = "<Person name=\"Bob\">42</Person>";
        let got = to_string(&bob).unwrap();
        assert_eq!(got, should_be);
    }

    #[test]
    fn test_serialize_struct_value_string() {
        #[derive(Serialize)]
        struct Person {
            name: String,
            #[serde(rename = "$value")]
            age: String,
        }

        let bob = Person {
            name: "Bob".to_string(),
            age: "42".to_string(),
        };
        let should_be = "<Person name=\"Bob\">42</Person>";
        let got = to_string(&bob).unwrap();
        assert_eq!(got, should_be);
    }

    #[test]
    fn test_serialize_map_entries() {
        let should_be = "<name>Bob</name><age>5</age>";
        let mut buffer = Vec::new();

        {
            let mut ser = Serializer::new(&mut buffer);
            let mut map = Map::new(&mut ser);
            map.serialize_entry("name", "Bob").unwrap();
            map.serialize_entry("age", "5").unwrap();
        }

        let got = String::from_utf8(buffer).unwrap();
        assert_eq!(got, should_be);
    }

    #[test]
    fn test_serialize_enum() {
        #[derive(Serialize)]
        #[allow(dead_code)]
        enum Node {
            Boolean(bool),
            Number(f64),
            String(String),
        }

        let mut buffer = Vec::new();
        let should_be = "<Boolean>true</Boolean>";

        {
            let mut ser = Serializer::new(&mut buffer);
            let node = Node::Boolean(true);
            node.serialize(&mut ser).unwrap();
        }

        let got = String::from_utf8(buffer).unwrap();
        assert_eq!(got, should_be);
    }

    #[test]
    #[ignore]
    fn serialize_a_list() {
        let inputs = vec![1, 2, 3, 4];

        let mut buffer = Vec::new();

        {
            let mut ser = Serializer::new(&mut buffer);
            inputs.serialize(&mut ser).unwrap();
        }

        let got = String::from_utf8(buffer).unwrap();
        println!("{}", got);
        panic!();
    }

    #[test]
    fn unit() {
        #[derive(Serialize)]
        struct Unit;

        let data = Unit;
        let should_be = "<root/>";
        let mut buffer = Vec::new();

        {
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));
            data.serialize(&mut ser).unwrap();
        }

        let got = String::from_utf8(buffer).unwrap();
        assert_eq!(got, should_be);
    }

    #[test]
    fn newtype() {
        #[derive(Serialize)]
        struct Newtype(bool);

        let data = Newtype(true);
        let should_be = "<root>true</root>";
        let mut buffer = Vec::new();

        {
            let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));
            data.serialize(&mut ser).unwrap();
        }

        let got = String::from_utf8(buffer).unwrap();
        assert_eq!(got, should_be);
    }

    #[test]
    fn tuple() {
        let data = (42.0, "answer");
        let should_be = "<root>42</root><root>answer</root>";
        let mut buffer = Vec::new();

        {
            let mut ser =
                Serializer::with_root(Writer::new_with_indent(&mut buffer, b' ', 4), Some("root"));
            data.serialize(&mut ser).unwrap();
        }

        let got = String::from_utf8(buffer).unwrap();
        assert_eq!(got, should_be);
    }

    #[test]
    fn tuple_struct() {
        #[derive(Serialize)]
        struct Tuple(f32, &'static str);

        let data = Tuple(42.0, "answer");
        let should_be = "<root>42</root><root>answer</root>";
        let mut buffer = Vec::new();

        {
            let mut ser =
                Serializer::with_root(Writer::new_with_indent(&mut buffer, b' ', 4), Some("root"));
            data.serialize(&mut ser).unwrap();
        }

        let got = String::from_utf8(buffer).unwrap();
        assert_eq!(got, should_be);
    }

    #[test]
    fn struct_() {
        #[derive(Serialize)]
        struct Struct {
            float: f64,
            string: String,
        }

        let mut buffer = Vec::new();
        let should_be = r#"<root float="42" string="answer"/>"#;

        {
            let mut ser =
                Serializer::with_root(Writer::new_with_indent(&mut buffer, b' ', 4), Some("root"));
            let node = Struct {
                float: 42.0,
                string: "answer".to_string(),
            };
            node.serialize(&mut ser).unwrap();
        }

        let got = String::from_utf8(buffer).unwrap();
        assert_eq!(got, should_be);
    }

    #[test]
    fn nested_struct() {
        #[derive(Serialize)]
        struct Struct {
            nested: Nested,
            string: String,
        }

        #[derive(Serialize)]
        struct Nested {
            float: f64,
        }

        let mut buffer = Vec::new();
        let should_be = r#"<root string="answer"><nested float="42"/>
</root>"#;

        {
            let mut ser =
                Serializer::with_root(Writer::new_with_indent(&mut buffer, b' ', 4), Some("root"));
            let node = Struct {
                nested: Nested { float: 42.0 },
                string: "answer".to_string(),
            };
            node.serialize(&mut ser).unwrap();
        }

        let got = String::from_utf8(buffer).unwrap();
        assert_eq!(got, should_be);
    }

    #[test]
    fn flatten_struct() {
        #[derive(Serialize)]
        struct Struct {
            #[serde(flatten)]
            nested: Nested,
            string: String,
        }

        #[derive(Serialize)]
        struct Nested {
            float: f64,
        }

        let mut buffer = Vec::new();
        let should_be = r#"<root><float>42</float><string>answer</string></root>"#;

        {
            let mut ser =
                Serializer::with_root(Writer::new_with_indent(&mut buffer, b' ', 4), Some("root"));
            let node = Struct {
                nested: Nested { float: 42.0 },
                string: "answer".to_string(),
            };
            node.serialize(&mut ser).unwrap();
        }

        let got = String::from_utf8(buffer).unwrap();
        assert_eq!(got, should_be);
    }

    mod enum_ {
        use super::*;

        #[derive(Serialize)]
        struct Nested {
            float: f64,
        }

        mod externally_tagged {
            use super::*;

            #[derive(Serialize)]
            enum Node {
                Unit,
                Newtype(bool),
                Tuple(f64, String),
                Struct {
                    float: f64,
                    string: String,
                },
                Holder {
                    nested: Nested,
                    string: String,
                },
                Flatten {
                    #[serde(flatten)]
                    nested: Nested,
                    string: String,
                },
            }

            #[test]
            fn unit() {
                let mut buffer = Vec::new();
                let should_be = "<Unit/>";

                {
                    let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));
                    let node = Node::Unit;
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn newtype() {
                let mut buffer = Vec::new();
                let should_be = "<Newtype>true</Newtype>";

                {
                    let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));
                    let node = Node::Newtype(true);
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn struct_() {
                let mut buffer = Vec::new();
                let should_be = r#"<Struct float="42" string="answer"/>"#;

                {
                    let mut ser = Serializer::with_root(
                        Writer::new_with_indent(&mut buffer, b' ', 4),
                        Some("root"),
                    );
                    let node = Node::Struct {
                        float: 42.0,
                        string: "answer".to_string(),
                    };
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn tuple_struct() {
                let mut buffer = Vec::new();
                let should_be = "<Tuple>42</Tuple><Tuple>answer</Tuple>";

                {
                    let mut ser = Serializer::with_root(
                        Writer::new_with_indent(&mut buffer, b' ', 4),
                        Some("root"),
                    );
                    let node = Node::Tuple(42.0, "answer".to_string());
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn nested_struct() {
                let mut buffer = Vec::new();
                let should_be = r#"<Holder string="answer"><nested float="42"/>
</Holder>"#;

                {
                    let mut ser = Serializer::with_root(
                        Writer::new_with_indent(&mut buffer, b' ', 4),
                        Some("root"),
                    );
                    let node = Node::Holder {
                        nested: Nested { float: 42.0 },
                        string: "answer".to_string(),
                    };
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn flatten_struct() {
                let mut buffer = Vec::new();
                let should_be = r#"<Flatten><float>42</float><string>answer</string></Flatten>"#;

                {
                    let mut ser = Serializer::with_root(
                        Writer::new_with_indent(&mut buffer, b' ', 4),
                        Some("root"),
                    );
                    let node = Node::Flatten {
                        nested: Nested { float: 42.0 },
                        string: "answer".to_string(),
                    };
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }
        }

        mod internally_tagged {
            use super::*;

            #[derive(Serialize)]
            #[serde(tag = "tag")]
            enum Node {
                Unit,
                /// Primitives (such as `bool`) are not supported by the serde in the internally tagged mode
                Newtype(NewtypeContent),
                // Tuple(f64, String),// Tuples are not supported in the internally tagged mode
                Struct {
                    float: f64,
                    string: String,
                },
                Holder {
                    nested: Nested,
                    string: String,
                },
                Flatten {
                    #[serde(flatten)]
                    nested: Nested,
                    string: String,
                },
            }

            #[derive(Serialize)]
            struct NewtypeContent {
                value: bool,
            }

            #[test]
            fn unit() {
                let mut buffer = Vec::new();
                let should_be = r#"<root tag="Unit"/>"#;

                {
                    let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));
                    let node = Node::Unit;
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn newtype() {
                let mut buffer = Vec::new();
                let should_be = r#"<root tag="Newtype" value="true"/>"#;

                {
                    let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));
                    let node = Node::Newtype(NewtypeContent { value: true });
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn struct_() {
                let mut buffer = Vec::new();
                let should_be = r#"<root tag="Struct" float="42" string="answer"/>"#;

                {
                    let mut ser = Serializer::with_root(
                        Writer::new_with_indent(&mut buffer, b' ', 4),
                        Some("root"),
                    );
                    let node = Node::Struct {
                        float: 42.0,
                        string: "answer".to_string(),
                    };
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn nested_struct() {
                let mut buffer = Vec::new();
                let should_be = r#"<root tag="Holder" string="answer"><nested float="42"/>
</root>"#;

                {
                    let mut ser = Serializer::with_root(
                        Writer::new_with_indent(&mut buffer, b' ', 4),
                        Some("root"),
                    );
                    let node = Node::Holder {
                        nested: Nested { float: 42.0 },
                        string: "answer".to_string(),
                    };
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn flatten_struct() {
                let mut buffer = Vec::new();
                let should_be =
                    r#"<root><tag>Flatten</tag><float>42</float><string>answer</string></root>"#;

                {
                    let mut ser = Serializer::with_root(
                        Writer::new_with_indent(&mut buffer, b' ', 4),
                        Some("root"),
                    );
                    let node = Node::Flatten {
                        nested: Nested { float: 42.0 },
                        string: "answer".to_string(),
                    };
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }
        }

        mod adjacently_tagged {
            use super::*;

            #[derive(Serialize)]
            #[serde(tag = "tag", content = "content")]
            enum Node {
                Unit,
                Newtype(bool),
                Tuple(f64, String),
                Struct {
                    float: f64,
                    string: String,
                },
                Holder {
                    nested: Nested,
                    string: String,
                },
                Flatten {
                    #[serde(flatten)]
                    nested: Nested,
                    string: String,
                },
            }

            #[test]
            fn unit() {
                let mut buffer = Vec::new();
                let should_be = r#"<root tag="Unit"/>"#;

                {
                    let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));
                    let node = Node::Unit;
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn newtype() {
                let mut buffer = Vec::new();
                let should_be = r#"<root tag="Newtype" content="true"/>"#;

                {
                    let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));
                    let node = Node::Newtype(true);
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn tuple_struct() {
                let mut buffer = Vec::new();
                let should_be = r#"<root tag="Tuple"><content>42</content><content>answer</content>
</root>"#;

                {
                    let mut ser = Serializer::with_root(
                        Writer::new_with_indent(&mut buffer, b' ', 4),
                        Some("root"),
                    );
                    let node = Node::Tuple(42.0, "answer".to_string());
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn struct_() {
                let mut buffer = Vec::new();
                let should_be = r#"<root tag="Struct"><content float="42" string="answer"/>
</root>"#;

                {
                    let mut ser = Serializer::with_root(
                        Writer::new_with_indent(&mut buffer, b' ', 4),
                        Some("root"),
                    );
                    let node = Node::Struct {
                        float: 42.0,
                        string: "answer".to_string(),
                    };
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn nested_struct() {
                let mut buffer = Vec::new();
                let should_be = r#"<root tag="Holder"><content string="answer"><nested float="42"/></content>
</root>"#;

                {
                    let mut ser = Serializer::with_root(
                        Writer::new_with_indent(&mut buffer, b' ', 4),
                        Some("root"),
                    );
                    let node = Node::Holder {
                        nested: Nested { float: 42.0 },
                        string: "answer".to_string(),
                    };
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn flatten_struct() {
                let mut buffer = Vec::new();
                let should_be = r#"<root tag="Flatten"><content><float>42</float><string>answer</string></content>
</root>"#;

                {
                    let mut ser = Serializer::with_root(
                        Writer::new_with_indent(&mut buffer, b' ', 4),
                        Some("root"),
                    );
                    let node = Node::Flatten {
                        nested: Nested { float: 42.0 },
                        string: "answer".to_string(),
                    };
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }
        }

        mod untagged {
            use super::*;

            #[derive(Serialize)]
            #[serde(untagged)]
            enum Node {
                Unit,
                Newtype(bool),
                Tuple(f64, String),
                Struct {
                    float: f64,
                    string: String,
                },
                Holder {
                    nested: Nested,
                    string: String,
                },
                Flatten {
                    #[serde(flatten)]
                    nested: Nested,
                    string: String,
                },
            }

            #[test]
            fn unit() {
                let mut buffer = Vec::new();
                // Unit variant consists just from the tag, and because tags
                // are not written in untagged mode, nothing is written
                let should_be = "";

                {
                    let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));
                    let node = Node::Unit;
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn newtype() {
                let mut buffer = Vec::new();
                let should_be = "true";

                {
                    let mut ser = Serializer::with_root(Writer::new(&mut buffer), Some("root"));
                    let node = Node::Newtype(true);
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn tuple_struct() {
                let mut buffer = Vec::new();
                let should_be = "<root>42</root><root>answer</root>";

                {
                    let mut ser = Serializer::with_root(
                        Writer::new_with_indent(&mut buffer, b' ', 4),
                        Some("root"),
                    );
                    let node = Node::Tuple(42.0, "answer".to_string());
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn struct_() {
                let mut buffer = Vec::new();
                let should_be = r#"<root float="42" string="answer"/>"#;

                {
                    let mut ser = Serializer::with_root(
                        Writer::new_with_indent(&mut buffer, b' ', 4),
                        Some("root"),
                    );
                    let node = Node::Struct {
                        float: 42.0,
                        string: "answer".to_string(),
                    };
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn nested_struct() {
                let mut buffer = Vec::new();
                let should_be = r#"<root string="answer"><nested float="42"/>
</root>"#;

                {
                    let mut ser = Serializer::with_root(
                        Writer::new_with_indent(&mut buffer, b' ', 4),
                        Some("root"),
                    );
                    let node = Node::Holder {
                        nested: Nested { float: 42.0 },
                        string: "answer".to_string(),
                    };
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }

            #[test]
            fn flatten_struct() {
                let mut buffer = Vec::new();
                let should_be = r#"<root><float>42</float><string>answer</string></root>"#;

                {
                    let mut ser = Serializer::with_root(
                        Writer::new_with_indent(&mut buffer, b' ', 4),
                        Some("root"),
                    );
                    let node = Node::Flatten {
                        nested: Nested { float: 42.0 },
                        string: "answer".to_string(),
                    };
                    node.serialize(&mut ser).unwrap();
                }

                let got = String::from_utf8(buffer).unwrap();
                assert_eq!(got, should_be);
            }
        }
    }
}

use std::fmt::Write;

use serde::{
    ser::{Impossible, SerializeStruct},
    Serialize, Serializer,
};

use super::{simple_type::QuoteTarget, QuoteLevel, SeError, SimpleTypeSerializer, WriteResult};

/// Writes everything it sees as attributes. Useful
/// for appending attributes from "flattened" structs
pub struct AppendAttributesSerializer<'w, W: Write> {
    pub writer: &'w mut W,
    pub level: QuoteLevel,
}

impl<'w, W: Write> AppendAttributesSerializer<'w, W> {
    #[inline]
    fn unsupported<T>(&self, typ: &'static str) -> Result<T, SeError> {
        Err(SeError::Unsupported(format!(
            "cannot serialize {typ} as a mapping of XML attributes. Only structs are supported for $attributes fields"
        ).into()))
    }
}

impl<'w, W: Write> Serializer for AppendAttributesSerializer<'w, W> {
    type Ok = WriteResult;
    type Error = SeError;

    type SerializeSeq = Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeMap = Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = StructAttrsOnly<'w, W>;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;

    fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
        self.unsupported("string")
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.unsupported("bytes")
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.unsupported("none")
    }

    fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.unsupported("some")
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.unsupported("unit")
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.unsupported("unit struct")
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.unsupported("unit variant")
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.unsupported("newtype struct")
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.unsupported("newtype variant")
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.unsupported("sequence")
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.unsupported("tuple")
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.unsupported("tuple struct")
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.unsupported("tuple variant")
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(SeError::Unsupported(
            "maps are not yet supported by $attributes fields".into(),
        ))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(Self::SerializeStruct {
            writer: self.writer,
            level: self.level,
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.unsupported("struct variant")
    }

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        self.unsupported("bool")
    }

    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        self.unsupported("i8")
    }

    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        self.unsupported("i16")
    }

    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        self.unsupported("i32")
    }

    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        self.unsupported("i64")
    }

    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        self.unsupported("u8")
    }

    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        self.unsupported("u16")
    }

    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        self.unsupported("u32")
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        self.unsupported("u64")
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        self.unsupported("f32")
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        self.unsupported("f64")
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        self.unsupported("char")
    }
}

pub struct StructAttrsOnly<'w, W: Write> {
    writer: &'w mut W,
    level: QuoteLevel,
}

impl<'w, W: Write> SerializeStruct for StructAttrsOnly<'w, W> {
    type Ok = WriteResult;
    type Error = SeError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        //TODO: Customization point: each attribute on new line
        self.writer.write_char(' ')?;

        match key.strip_prefix('@') {
            Some(stripped) => self.writer.write_str(stripped),
            None => self.writer.write_str(key),
        }?;

        self.writer.write_char('=')?;

        //TODO: Customization point: preferred quote style
        self.writer.write_char('"')?;
        value.serialize(SimpleTypeSerializer {
            writer: &mut self.writer,
            target: QuoteTarget::DoubleQAttr,
            level: self.level,
        })?;
        self.writer.write_char('"')?;

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(WriteResult::Nothing)
    }
}

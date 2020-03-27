use crate::{
    errors::{serialize::DeError, Error},
    se::Serializer,
};
use serde::ser::{self, Serialize};
use std::io::Write;

/// Determine if this is a basic type that should be an attribute
fn is_attr_type(type_name: &str) -> bool {
    match type_name {
        "bool" | "char" | "option" | "()" => true,
        "u8" | "u16" | "u32" | "u64" | "u128" => true,
        "i8" | "i16" | "i32" | "i64" | "i128" => true,
        "f32" | "f64" => true,
        "[u8]" | "str" => true,
        "alloc::string::String" => true,
        _ => false
    }
}


/// An implementation of `SerializeMap` for serializing to XML.
pub struct Map<'w, W>
where
    W: 'w + Write,
{
    parent: &'w mut Serializer<W>,
}

impl<'w, W> Map<'w, W>
where
    W: 'w + Write,
{
    /// Create a new Map
    pub fn new(parent: &'w mut Serializer<W>) -> Map<'w, W> {
        Map { parent }
    }
}

impl<'w, W> ser::SerializeMap for Map<'w, W>
where
    W: 'w + Write,
{
    type Ok = ();
    type Error = DeError;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, _: &T) -> Result<(), DeError> {
        Err(DeError::Unsupported(
            "impossible to serialize the key on its own, please use serialize_entry()",
        ))
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), DeError> {
        value.serialize(&mut *self.parent)
    }

    fn end(self) -> Result<Self::Ok, DeError> {
        Ok(())
    }

    fn serialize_entry<K: ?Sized + Serialize, V: ?Sized + Serialize>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), DeError> {
        // TODO: Is it possible to ensure our key is never a composite type?
        // Anything which isn't a "primitive" would lead to malformed XML here...
        write!(self.parent.writer.inner(), "<").map_err(Error::Io)?;
        key.serialize(&mut *self.parent)?;
        write!(self.parent.writer.inner(), ">").map_err(Error::Io)?;

        value.serialize(&mut *self.parent)?;

        write!(self.parent.writer.inner(), "</").map_err(Error::Io)?;
        key.serialize(&mut *self.parent)?;
        write!(self.parent.writer.inner(), ">").map_err(Error::Io)?;
        Ok(())
    }
}

/// An implementation of `SerializeStruct` for serializing to XML.
pub struct Struct<'w, W>
where
    W: 'w + Write,
{
    parent: &'w mut Serializer<W>,
    name: &'w str,
    attrs: u32,
    needs_close: bool,
}

impl<'w, W> Struct<'w, W>
    where
        W: 'w + Write,
{
    /// Create a new `Struct`
    pub fn new(parent: &'w mut Serializer<W>, name: &'w str) -> Struct<'w, W> {
        Struct { parent, name, attrs: 0, needs_close: true }
    }
}

impl<'w, W> ser::SerializeStruct for Struct<'w, W>
    where
        W: 'w + Write,
{
    type Ok = ();
    type Error = DeError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), DeError> {
        let type_name = std::any::type_name::<T>();
        let is_attr = is_attr_type(type_name);
        //println!{"type_name {}", type_name};
        if is_attr {
            write!(self.parent.writer.inner(), " ").map_err(Error::Io)?;
            key.serialize(&mut *self.parent)?;
            write!(self.parent.writer.inner(), "=").map_err(Error::Io)?;

            write!(self.parent.writer.inner(), r#"""#).map_err(Error::Io)?;
            value.serialize(&mut *self.parent)?;
            write!(self.parent.writer.inner(), r#"""#).map_err(Error::Io)?;
            self.attrs += 1;
        } else {
            if self.needs_close {
                write!(self.parent.writer.inner(), ">").map_err(Error::Io)?;
                self.needs_close = false;
            }
            value.serialize(&mut *self.parent)?;
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, DeError> {
        if self.needs_close {
            write!(self.parent.writer.inner(), ">").map_err(Error::Io)?;
        }
        write!(self.parent.writer.inner(), "</").map_err(Error::Io)?;
        self.name.serialize(&mut *self.parent)?;
        write!(self.parent.writer.inner(), ">").map_err(Error::Io)?;
        Ok(())
    }
}

/// An implementation of `SerializeSeq' for serializing to XML.
pub struct Seq<'w, W>
where
    W: 'w + Write,
{
    parent: &'w mut Serializer<W>,
}

impl<'w, W> Seq<'w, W>
where
    W: 'w + Write,
{
    /// Create a new `Seq`
    pub fn new(parent: &'w mut Serializer<W>) -> Seq<'w, W> {
        Seq { parent }
    }
}

impl<'w, W> ser::SerializeSeq for Seq<'w, W>
where
    W: 'w + Write,
{
    type Ok = ();
    type Error = DeError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        value.serialize(&mut *self.parent)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

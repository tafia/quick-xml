use crate::{
    errors::{serialize::DeError, Error},
    events::{BytesStart, Event},
    se::Serializer,
};
use serde::ser::{self, Serialize};
use std::io::Write;

/// An implementation of `SerializeMap` for serializing to XML.
pub struct Map<'r, 'w, W>
where
    W: 'w + Write,
{
    parent: &'w mut Serializer<'r, W>,
}

impl<'r, 'w, W> Map<'r, 'w, W>
where
    W: 'w + Write,
{
    /// Create a new Map
    pub fn new(parent: &'w mut Serializer<'r, W>) -> Self {
        Map { parent }
    }
}

impl<'r, 'w, W> ser::SerializeMap for Map<'r, 'w, W>
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
pub struct Struct<'r, 'w, W>
where
    W: 'w + Write,
{
    parent: &'w mut Serializer<'r, W>,
    /// Buffer for holding fields, serialized as attributes. Doesn't allocate
    /// if there are no fields represented as attributes
    attrs: BytesStart<'w>,
    /// Buffer for holding fields, serialized as elements
    children: Vec<u8>,
    /// Buffer for serializing one field. Cleared after serialize each field
    buffer: Vec<u8>,
}

impl<'r, 'w, W> Struct<'r, 'w, W>
where
    W: 'w + Write,
{
    /// Create a new `Struct`
    pub fn new(parent: &'w mut Serializer<'r, W>, name: &'r str) -> Self {
        let name = name.as_bytes();
        Struct {
            parent,
            attrs: BytesStart::borrowed_name(name),
            children: Vec::new(),
            buffer: Vec::new(),
        }
    }
}

impl<'r, 'w, W> ser::SerializeStruct for Struct<'r, 'w, W>
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
        let mut serializer = Serializer::new(&mut self.buffer);
        value.serialize(&mut serializer)?;

        if !self.buffer.is_empty() {
            if self.buffer[0] == b'<' {
                write!(&mut self.children, "<{}>", key).map_err(Error::Io)?;
                // Drains buffer, moves it to children
                self.children.append(&mut self.buffer);
                write!(&mut self.children, "</{}>", key).map_err(Error::Io)?;
            } else {
                self.attrs.push_attribute((key.as_bytes(), self.buffer.as_ref()));
                self.buffer.clear();
            }
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, DeError> {
        if self.children.is_empty() {
            self.parent.writer.write_event(Event::Empty(self.attrs))?;
        } else {
            self.parent.writer.write_event(Event::Start(self.attrs.to_borrowed()))?;
            self.parent.writer.write(&self.children)?;
            self.parent.writer.write_event(Event::End(self.attrs.to_end()))?;
        }
        Ok(())
    }
}

/// An implementation of `SerializeSeq' for serializing to XML.
pub struct Seq<'r, 'w, W>
where
    W: 'w + Write,
{
    parent: &'w mut Serializer<'r, W>,
}

impl<'r, 'w, W> Seq<'r, 'w, W>
where
    W: 'w + Write,
{
    /// Create a new `Seq`
    pub fn new(parent: &'w mut Serializer<'r, W>) -> Self {
        Seq { parent }
    }
}

impl<'r, 'w, W> ser::SerializeSeq for Seq<'r, 'w, W>
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

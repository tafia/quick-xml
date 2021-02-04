use crate::{
    errors::{serialize::DeError, Error},
    events::{BytesEnd, BytesStart, Event},
    se::Serializer,
    writer::Writer,
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
        if let Some(tag) = self.parent.root_tag {
            self.parent
                .writer
                .write_event(Event::End(BytesEnd::borrowed(tag.as_bytes())))?;
        }
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
        // TODO: Inherit indentation state from self.parent.writer
        let writer = Writer::new(&mut self.buffer);
        let mut serializer = Serializer::with_root(writer, Some(key));
        value.serialize(&mut serializer)?;

        if !self.buffer.is_empty() {
            if self.buffer[0] == b'<' || key == "$value" {
                // Drains buffer, moves it to children
                self.children.append(&mut self.buffer);
            } else {
                self.attrs
                    .push_attribute((key.as_bytes(), self.buffer.as_ref()));
                self.buffer.clear();
            }
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, DeError> {
        if self.children.is_empty() {
            self.parent.writer.write_event(Event::Empty(self.attrs))?;
        } else {
            self.parent
                .writer
                .write_event(Event::Start(self.attrs.to_borrowed()))?;
            self.parent.writer.write(&self.children)?;
            self.parent
                .writer
                .write_event(Event::End(self.attrs.to_end()))?;
        }
        Ok(())
    }
}

impl<'r, 'w, W> ser::SerializeStructVariant for Struct<'r, 'w, W>
where
    W: 'w + Write,
{
    type Ok = ();
    type Error = DeError;

    #[inline]
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        <Self as ser::SerializeStruct>::serialize_field(self, key, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as ser::SerializeStruct>::end(self)
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

/// An implementation of `SerializeTuple`, `SerializeTupleStruct` and
/// `SerializeTupleVariant` for serializing to XML.
pub struct Tuple<'r, 'w, W>
where
    W: 'w + Write,
{
    parent: &'w mut Serializer<'r, W>,
    /// Possible qualified name of XML tag surrounding each element
    name: &'r str,
}

impl<'r, 'w, W> Tuple<'r, 'w, W>
where
    W: 'w + Write,
{
    /// Create a new `Tuple`
    pub fn new(parent: &'w mut Serializer<'r, W>, name: &'r str) -> Self {
        Tuple { parent, name }
    }
}

impl<'r, 'w, W> ser::SerializeTuple for Tuple<'r, 'w, W>
where
    W: 'w + Write,
{
    type Ok = ();
    type Error = DeError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        write!(self.parent.writer.inner(), "<{}>", self.name).map_err(Error::Io)?;
        value.serialize(&mut *self.parent)?;
        write!(self.parent.writer.inner(), "</{}>", self.name).map_err(Error::Io)?;
        Ok(())
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'r, 'w, W> ser::SerializeTupleStruct for Tuple<'r, 'w, W>
where
    W: 'w + Write,
{
    type Ok = ();
    type Error = DeError;

    #[inline]
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        <Self as ser::SerializeTuple>::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as ser::SerializeTuple>::end(self)
    }
}

impl<'r, 'w, W> ser::SerializeTupleVariant for Tuple<'r, 'w, W>
where
    W: 'w + Write,
{
    type Ok = ();
    type Error = DeError;

    #[inline]
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        <Self as ser::SerializeTuple>::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as ser::SerializeTuple>::end(self)
    }
}

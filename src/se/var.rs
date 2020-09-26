use crate::{
    errors::{serialize::DeError, Error},
    events::{BytesEnd, Event},
    se::Serializer,
};
use serde::ser::{self, Serialize};
use std::io::Write;

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
    pub(crate) attrs: Vec<u8>,
    children: Vec<u8>,
    buffer: Vec<u8>,
}

impl<'w, W> Struct<'w, W>
where
    W: 'w + Write,
{
    /// Create a new `Struct`
    pub fn new(parent: &'w mut Serializer<W>, name: &'w str) -> Struct<'w, W> {
        Struct {
            parent,
            name,
            attrs: Vec::new(),
            children: Vec::new(),
            buffer: Vec::new(),
        }
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
        let mut serializer = Serializer::new(&mut self.buffer);
        value.serialize(&mut serializer)?;

        if !self.buffer.is_empty() {
            if self.buffer[0] == b'<' {
                write!(&mut self.children, "<{}>", key).map_err(Error::Io)?;
                self.children.extend(&self.buffer);
                write!(&mut self.children, "</{}>", key).map_err(Error::Io)?;
            } else {
                write!(&mut self.attrs, " {}=\"", key).map_err(Error::Io)?;
                self.attrs.extend(&self.buffer);
                write!(&mut self.attrs, "\"").map_err(Error::Io)?;
            }

            self.buffer.clear();
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, DeError> {
        self.parent.writer.write(&self.attrs)?;
        self.parent.writer.write(">".as_bytes())?;
        self.parent.writer.write(&self.children)?;
        self.parent
            .writer
            .write_event(Event::End(BytesEnd::borrowed(self.name.as_bytes())))?;
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

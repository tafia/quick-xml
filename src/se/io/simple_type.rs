use super::super::simple_type::*;
use super::*;

/// Implements writing primitives to the underlying writer.
/// Implementor must provide `write_str(self, &str) -> Result<(), DeError>` method
macro_rules! write_primitive {
    ($method:ident ( $ty:ty )) => {
        fn $method(mut self, value: $ty) -> Result<Self::Ok, Self::Error> {
            self.write_str(&value.to_string())?;
            Ok(self.writer)
        }
    };
    () => {
        fn serialize_bool(mut self, value: bool) -> Result<Self::Ok, Self::Error> {
            self.write_str(if value { "true" } else { "false" })?;
            Ok(self.writer)
        }

        write_primitive!(serialize_i8(i8));
        write_primitive!(serialize_i16(i16));
        write_primitive!(serialize_i32(i32));
        write_primitive!(serialize_i64(i64));

        write_primitive!(serialize_u8(u8));
        write_primitive!(serialize_u16(u16));
        write_primitive!(serialize_u32(u32));
        write_primitive!(serialize_u64(u64));

        serde_if_integer128! {
            write_primitive!(serialize_i128(i128));
            write_primitive!(serialize_u128(u128));
        }

        write_primitive!(serialize_f32(f32));
        write_primitive!(serialize_f64(f64));

        fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
            self.serialize_str(&value.to_string())
        }

        fn serialize_bytes(mut self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
            self.writer.write(&value)?;
            Ok(self.writer)
        }

        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            Ok(self.writer)
        }

        fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
            value.serialize(self)
        }

        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            self.serialize_str(variant)
        }

        fn serialize_newtype_struct<T: ?Sized + Serialize>(
            self,
            _name: &'static str,
            value: &T,
        ) -> Result<Self::Ok, Self::Error> {
            value.serialize(self)
        }
    };
}

/// A serializer for a values representing XSD [simple types], which used in:
/// - attribute values (`<... ...="value" ...>`)
/// - text content (`<...>text</...>`)
/// - CDATA content (`<...><![CDATA[cdata]]></...>`)
///
/// [simple types]: https://www.w3.org/TR/xmlschema11-1/#Simple_Type_Definition
pub struct SimpleTypeSerializer<'i, W: Write> {
    /// Writer to which this serializer writes content
    pub writer: W,
    /// Target for which element is serializing. Affects additional characters to escape.
    pub target: QuoteTarget,
    /// Defines which XML characters need to be escaped
    pub level: QuoteLevel,
    /// Indent that should be written before the content if content is not an empty string
    pub(crate) indent: Indent<'i>,
}

impl<'i, W: Write> SimpleTypeSerializer<'i, W> {
    fn write_str(&mut self, value: &str) -> Result<(), DeError> {
        self.indent.write_io_indent(&mut self.writer)?;
        Ok(self.writer.write_str(value)?)
    }
}

impl<'i, W: Write> ser::Serializer for SimpleTypeSerializer<'i, W> {
    type Ok = W;
    type Error = DeError;

    type SerializeSeq = SimpleSeq<'i, W>;
    type SerializeTuple = SimpleSeq<'i, W>;
    type SerializeTupleStruct = SimpleSeq<'i, W>;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeMap = Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

    write_primitive!();

    fn serialize_str(mut self, value: &str) -> Result<Self::Ok, Self::Error> {
        if !value.is_empty() {
            self.write_str(&super::simple_type::escape_list(
                value,
                self.target,
                self.level,
            ))?;
        }
        Ok(self.writer)
    }

    /// Does not write anything
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.writer)
    }

    /// Does not write anything
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(self.writer)
    }

    /// We cannot store both a variant discriminant and a variant value,
    /// so serialization of enum newtype variant returns `Err(Unsupported)`
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, DeError> {
        Err(DeError::Unsupported(
            format!("cannot serialize enum newtype variant `{}::{}` as an attribute or text content value", name, variant).into(),
        ))
    }

    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SimpleSeq {
            writer: self.writer,
            target: self.target,
            level: self.level,
            indent: self.indent,
            is_empty: true,
        })
    }

    #[inline]
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(None)
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(None)
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(DeError::Unsupported(
            format!("cannot serialize enum tuple variant `{}::{}` as an attribute or text content value", name, variant).into(),
        ))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(DeError::Unsupported(
            "cannot serialize map as an attribute or text content value".into(),
        ))
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(DeError::Unsupported(
            format!(
                "cannot serialize struct `{}` as an attribute or text content value",
                name
            )
            .into(),
        ))
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(DeError::Unsupported(
            format!("cannot serialize enum struct variant `{}::{}` as an attribute or text content value", name, variant).into(),
        ))
    }
}

/// Serializer for a sequence of atomic values delimited by space
pub struct SimpleSeq<'i, W: Write> {
    writer: W,
    target: QuoteTarget,
    level: QuoteLevel,
    /// Indent that should be written before the content if content is not an empty string
    indent: Indent<'i>,
    /// If `true`, nothing was written yet to the `writer`
    is_empty: bool,
}

impl<'i, W: Write> SerializeSeq for SimpleSeq<'i, W> {
    type Ok = W;
    type Error = DeError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        // Write indent for the first element and delimiter for others
        let indent = if self.is_empty {
            Some(self.indent.borrow())
        } else {
            None
        };
        if value.serialize(super::simple_type::AtomicSerializer {
            writer: FmtWriter::ref_cast_mut(&mut self.writer),
            target: self.target,
            level: self.level,
            indent,
        })? {
            self.is_empty = false;
        }
        Ok(())
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.writer)
    }
}

impl<'i, W: Write> SerializeTuple for SimpleSeq<'i, W> {
    type Ok = W;
    type Error = DeError;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

impl<'i, W: Write> SerializeTupleStruct for SimpleSeq<'i, W> {
    type Ok = W;
    type Error = DeError;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

impl<'i, W: Write> SerializeTupleVariant for SimpleSeq<'i, W> {
    type Ok = W;
    type Error = DeError;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

// ////////////////////////////////////////////////////////////////////////////////////////////////////

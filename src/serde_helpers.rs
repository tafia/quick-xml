//! Provides helper functions to glue an XML with a serde content model.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Provides helper functions to serialization and deserialization of types
/// (usually enums) as a text content of an element and intended to use with
/// [`#[serde(with = "...")]`][with], [`#[serde(deserialize_with = "...")]`][de-with]
/// and [`#[serde(serialize_with = "...")]`][se-with].
///
/// When you serialize unit variants of enums, they are serialized as an empty
/// elements, like `<Unit/>`. At the same time, when enum consist only from unit
/// variants, it is frequently needed to serialize them as string content of an
/// element, like `<field>Unit</field>`. To make this possible use this module.
///
/// ```
/// # use pretty_assertions::assert_eq;
/// use quick_xml::de::from_str;
/// use quick_xml::se::to_string;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, PartialEq, Debug)]
/// enum SomeEnum {
///     // Default implementation serializes enum as an `<EnumValue/>` element
///     EnumValue,
/// # /*
///     ...
/// # */
/// }
///
/// #[derive(Serialize, Deserialize, PartialEq, Debug)]
/// #[serde(rename = "some-container")]
/// struct SomeContainer {
///     #[serde(with = "quick_xml::serde_helpers::text_content")]
///     field: SomeEnum,
/// }
///
/// let container = SomeContainer {
///     field: SomeEnum::EnumValue,
/// };
/// let xml = "\
///     <some-container>\
///         <field>EnumValue</field>\
///     </some-container>";
///
/// assert_eq!(to_string(&container).unwrap(), xml);
/// assert_eq!(from_str::<SomeContainer>(xml).unwrap(), container);
/// ```
///
/// Using of this module is equivalent to replacing `field`'s type to this:
///
/// ```
/// # use serde::{Deserialize, Serialize};
/// # type SomeEnum = ();
/// #[derive(Serialize, Deserialize)]
/// struct Field {
///     // Use a special name `$text` to map field to the text content
///     #[serde(rename = "$text")]
///     content: SomeEnum,
/// }
///
/// #[derive(Serialize, Deserialize)]
/// #[serde(rename = "some-container")]
/// struct SomeContainer {
///     field: Field,
/// }
/// ```
/// Read about the meaning of a special [`$text`] field.
///
/// [with]: https://serde.rs/field-attrs.html#with
/// [de-with]: https://serde.rs/field-attrs.html#deserialize_with
/// [se-with]: https://serde.rs/field-attrs.html#serialize_with
/// [`$text`]: ../../de/index.html#text
pub mod text_content {
    use super::*;

    /// Serializes `value` as an XSD [simple type]. Intended to use with
    /// `#[serde(serialize_with = "...")]`. See example at [`text_content`]
    /// module level.
    ///
    /// [simple type]: https://www.w3.org/TR/xmlschema11-1/#Simple_Type_Definition
    pub fn serialize<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        #[derive(Serialize)]
        struct Field<'a, T> {
            #[serde(rename = "$text")]
            value: &'a T,
        }
        Field { value }.serialize(serializer)
    }

    /// Deserializes XSD's [simple type]. Intended to use with
    /// `#[serde(deserialize_with = "...")]`. See example at [`text_content`]
    /// module level.
    ///
    /// [simple type]: https://www.w3.org/TR/xmlschema11-1/#Simple_Type_Definition
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        #[derive(Deserialize)]
        struct Field<T> {
            #[serde(rename = "$text")]
            value: T,
        }
        Ok(Field::deserialize(deserializer)?.value)
    }
}

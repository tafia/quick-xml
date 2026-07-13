use std::borrow::Cow;
use std::convert::Infallible;
use std::error::Error;
use std::fmt;
use std::io::BufRead;

use crate::events::BytesText;
use crate::utils::Bytes;

/// [Replacement text] of the resolved entity reference (`&...;`).
///
/// [Replacement text]: https://www.w3.org/TR/xml11/#dt-repltext
pub enum ReplacementText<'i, 'e> {
    /// Referenced entity inside the same document in the internal DTD.
    Internal(Cow<'i, [u8]>),
    /// Referenced entity inside the other document which will be read from
    /// the specified source.
    External(Box<dyn BufRead + 'e>),
}
impl<'i, 'e> fmt::Debug for ReplacementText<'i, 'e> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Internal(e) => Bytes(e).fmt(f),
            Self::External(e) => write!(f, "<dyn BufRead at {:p}>", &e),
        }
    }
}

/// Used to create entity resolver for each physical document (storage unit or an _[entity]_)
/// that would be parsed by the reader.
///
/// [entity]: https://www.w3.org/TR/xml11/#sec-documents
pub trait EntityResolverFactory<'i> {
    /// The error type that represents DTD parse error.
    type CaptureError: Error + 'static;
    /// Type that holds state for each entity, for example, for each file, which
    /// forms the whole logical structure of the XML document.
    type Resolver: EntityResolver<'i, CaptureError = Self::CaptureError>;

    /// Creates state for the new entity parser.
    fn new_resolver(&mut self) -> Self::Resolver;
}

/// Used to resolve unknown [general entities] (`&...;`) while parsing.
///
/// Note, that this trait is not used to resolve _[parameter entities]_ (`%...;`), they are resolved
/// inside implementation of this trait. Parameter entities cannot be used outside of the `<!DOCTYPE >`
/// declaration, so no need to resolve them in the document.
///
/// # Example
///
/// That example is taken from the XML specification. Suppose that we have the following DTD:
/// ```xml
/// <!ENTITY % pub    "&#xc9;ditions Gallimard" >
/// <!ENTITY   rights "All rights reserved" >
/// <!ENTITY   book   "&#xA9; 1947 %pub;. &rights;" >
/// ```
/// Here we have two defined _internal general entities_ (`rights` and `book`), which may be used
/// everything in the document below their definition point (including the DOCTYPE declaration) and
/// one _parameter entity_ (`pub`), which may be used only inside DOCTYPE declaration below it
/// definition point. The literal values and replacement texts for those entities are:
///
/// |Entity|Literal value                |Replacement text
/// |------|-----------------------------|------------------------------------
/// |pub   |`&#xc9;ditions Gallimard`    |`Éditions Gallimard`
/// |rights|`All rights reserved`        |`All rights reserved`
/// |book  |`&#xA9; 1947 %pub;. &rights;`|`© 1947 Éditions Gallimard. &rights;`
///
/// Implementation of the `EntityResolver` must return the _replacement text_ from the
/// [resolve](Self::resolve) method. To follow XML specification, that means, that the
/// following must be done over the text that was captured in the [capture](Self::capture) method:
/// - EOLs must be normalized according to the XML version  for which this resolver was created
/// - any parameter entity references should be resolved: they should be replaced by their's
///   replacement text
/// - any character references should be expanded into the corresponding characters
/// - any references to the other general entities (`&...;`) should be left as is
///
/// If the implementation will not parse DTD and just provide values for the general entity
/// references (which usually custom resolvers will do), then just know, that any returned
/// text will be considered as a replacement text as required by the XML specification.
/// One consequence of this: if you want to have literal `<` and `&` characters in the text,
/// you should use escape form of them, either as character reference or as entity reference.
/// Otherwise they will be considered as part of the markup.
///
/// [general entities]: https://www.w3.org/TR/xml11/#gen-entity
/// [parameter entities]: https://www.w3.org/TR/xml11/#dt-PE
pub trait EntityResolver<'i> {
    /// The error type that represents DTD parse error.
    type CaptureError: Error + 'static;

    /// Called on contents of [`Event::DocType`] to capture declared entities.
    /// Can be called multiple times, for each parsed `<!DOCTYPE >` declaration.
    ///
    /// [`Event::DocType`]: crate::reader::Event::DocType
    fn capture(&mut self, doctype: BytesText<'i>) -> Result<(), Self::CaptureError>;

    /// Called when an entity needs to be resolved. Returns entity's [replacement text].
    ///
    /// `None` is returned if a suitable value can not be found.
    /// In that case an [`Error::UnrecognizedGeneralEntity`] will be returned by a reader.
    ///
    /// [replacement text]: https://www.w3.org/TR/xml11/#dt-repltext
    /// [`Error::UnrecognizedGeneralEntity`]: crate::errors::Error::UnrecognizedGeneralEntity
    fn resolve<'e>(&self, entity: &str) -> Option<ReplacementText<'i, 'e>>;
}

/// An [`EntityResolver`] that resolves only predefined entities, as defined in [specification]:
///
/// | Entity | Resolution
/// |--------|------------
/// |`&lt;`  | `&#60;` (note: not `<`)
/// |`&gt;`  | `>`
/// |`&amp;` | `&#38;` (note: not `&`)
/// |`&apos;`| `'`
/// |`&quot;`| `"`
///
/// This is the default resolver for reader and deserializer.
///
/// [specification]: https://www.w3.org/TR/xml11/#sec-predefined-ent
#[derive(Default, Debug, Copy, Clone)]
pub struct PredefinedEntityResolver;

impl<'i> EntityResolverFactory<'i> for PredefinedEntityResolver {
    type CaptureError = Infallible;
    type Resolver = Self;

    #[inline]
    fn new_resolver(&mut self) -> Self::Resolver {
        *self
    }
}

impl<'i> EntityResolver<'i> for PredefinedEntityResolver {
    type CaptureError = Infallible;

    #[inline]
    fn capture(&mut self, _doctype: BytesText<'i>) -> Result<(), Self::CaptureError> {
        Ok(())
    }

    #[inline]
    fn resolve<'e>(&self, entity: &str) -> Option<ReplacementText<'i, 'e>> {
        let replacement_text = match entity {
            "lt" => "&#60;",
            "gt" => ">",
            "amp" => "&#38;",
            "apos" => "'",
            "quot" => "\"",
            _ => return None,
        };
        Some(ReplacementText::Internal(Cow::Borrowed(
            replacement_text.as_bytes(),
        )))
    }
}

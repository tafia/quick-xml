use crate::events::{BytesCData, BytesEnd, BytesPI, BytesStart, BytesText};

/// Event emitted by [`Reader::read_event`].
///
/// # Lifetime
///
/// The `'i` lifetime of this struct is the lifetime of data that may be borrowed
/// from the XML input (when reader of the main document reads from `&[u8]` or `&str`).
///
/// [`Reader::read_event`]: crate::reader::Reader::read_event
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum Event<'i> {
    /// Empty element tag (with attributes) `<tag attr="value" />`.
    Empty(BytesStart<'i>),
    /// Start tag (with attributes) `<tag attr="value">`.
    Start(BytesStart<'i>),
    /// End tag `</tag>`.
    End(BytesEnd<'i>),
    /// Character data between `Start` and `End` element.
    Text(BytesText<'i>),
    /// CData `<![CDATA[...]]>`.
    CData(BytesCData<'i>),
    /// Processing instruction `<?...?>`.
    PI(BytesPI<'i>),
    /// End of XML document.
    Eof,
}

impl<'i> Event<'i> {
    /// Ensures that all data is owned to extend the object's lifetime if necessary.
    #[inline]
    pub fn into_owned(self) -> Event<'static> {
        match self {
            Self::Empty(e) => Event::Empty(e.into_owned()),
            Self::Start(e) => Event::Start(e.into_owned()),
            Self::End(e) => Event::End(e.into_owned()),
            Self::Text(e) => Event::Text(e.into_owned()),
            Self::CData(e) => Event::CData(e.into_owned()),
            Self::PI(e) => Event::PI(e.into_owned()),
            Self::Eof => Event::Eof,
        }
    }
}

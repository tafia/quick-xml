//! Error management module

#![allow(missing_docs)]

/// The error type used by this crate.
#[derive(Display, Debug)]
pub enum Error {
    #[display(fmt = "I/O error: {}", "_0")]
    Io(::std::io::Error),

    #[display(fmt = "UTF8 error: {}", "_0")]
    Utf8(::std::str::Utf8Error),

    #[display(fmt = "Unexpected EOF during reading {}.", "_0")]
    UnexpectedEof(String),

    #[display(fmt = "Expecting </{}> found </{}>", expected, found)]
    EndEventMismatch { expected: String, found: String },

    #[display(fmt = "Unexpected token '{}'", "_0")]
    UnexpectedToken(String),

    #[display(fmt = "Only Comment, CDATA and DOCTYPE nodes can start with a '!'")]
    UnexpectedBang,

    #[display(fmt = "Cannot read text, expecting Event::Text")]
    TextNotFound,

    #[display(
        fmt = "XmlDecl must start with 'version' attribute, found {:?}",
        "_0"
    )]
    XmlDeclWithoutVersion(Option<String>),

    #[display(
        fmt = "error while parsing attribute at position {}: Attribute key cannot contain quote.",
        "_0"
    )]
    NameWithQuote(usize),

    #[display(
        fmt = "error while parsing attribute at position {}: Attribute key must be directly followed by = or space",
        "_0"
    )]
    NoEqAfterName(usize),

    #[display(
        fmt = "error while parsing attribute at position {}: Attribute value must start with a quote.",
        "_0"
    )]
    UnquotedValue(usize),

    #[display(
        fmt = "error while parsing attribute at position {}: Duplicate attribute at position {} and {}",
        "_0", "_1", "_0"
    )]
    DuplicatedAttribute(usize, usize),

    #[display(fmt = "{}", "_0")]
    EscapeError(::escape::EscapeError),
}

impl From<::std::io::Error> for Error {
    /// Creates a new `Error::Io` from the given error
    #[inline]
    fn from(error: ::std::io::Error) -> Error {
        Error::Io(error)
    }
}

impl From<::std::str::Utf8Error> for Error {
    /// Creates a new `Error::Utf8` from the given error
    #[inline]
    fn from(error: ::std::str::Utf8Error) -> Error {
        Error::Utf8(error)
    }
}

/// A specialized `Result` type where the error is hard-wired to [`Error`].
///
/// [`Error`]: enum.Error.html
pub type Result<T> = ::std::result::Result<T, Error>;

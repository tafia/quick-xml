//! Error management module

#![allow(missing_docs)]

/// The error type used by this crate.
#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "I/O error")]
    Io(#[cause] ::std::io::Error),

    #[fail(display = "UTF8 error")]
    Utf8(#[cause] ::std::str::Utf8Error),

    #[fail(display = "Unexpected EOF during reading {}.", _0)]
    UnexpectedEof(String),

    #[fail(display = "Expecting </{}> found </{}>", expected, found)]
    EndEventMismatch { expected: String, found: String },

    #[fail(display = "Unexpected token '{}'", _0)]
    UnexpectedToken(String),

    #[fail(display = "Only Comment, CDATA and DOCTYPE nodes can start with a '!'")]
    UnexpectedBang,

    #[fail(display = "Cannot read text, expecting Event::Text")]
    TextNotFound,

    #[fail(
        display = "XmlDecl must start with 'version' attribute, found {:?}",
        _0
    )]
    XmlDeclWithoutVersion(Option<String>),

    #[fail(
        display = "error while parsing attribute at position {}: Attribute key cannot contain quote.",
        _0
    )]
    NameWithQuote(usize),

    #[fail(
        display = "error while parsing attribute at position {}: Attribute key must be directly followed by = or space",
        _0
    )]
    NoEqAfterName(usize),

    #[fail(
        display = "error while parsing attribute at position {}: Attribute value must start with a quote.",
        _0
    )]
    UnquotedValue(usize),

    #[fail(
        display = "error while parsing attribute at position {}: Duplicate attribute at position {} and {}",
        _0,
        _1,
        _0
    )]
    DuplicatedAttribute(usize, usize),

    #[fail(display = "{}", _0)]
    EscapeError(#[cause] ::escape::EscapeError),
}

/// A specialized `Result` type where the error is hard-wired to [`Error`].
///
/// [`Error`]: enum.Error.html
pub type Result<T> = ::std::result::Result<T, Error>;

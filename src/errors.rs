//! Error management module

/// The error type used by this crate.
#[cfg_attr(feature = "failure", derive(Fail))]
#[derive(Debug)]
pub enum Error {
    /// IO error
    Io(#[cfg_attr(feature = "failure", cause)] ::std::io::Error),
    /// Utf8 error
    Utf8(#[cfg_attr(feature = "failure", cause)] ::std::str::Utf8Error),
    /// Unexpected End of File
    UnexpectedEof(String),
    /// End event mismatch
    EndEventMismatch {
        /// Expected end event
        expected: String,
        /// Found end event
        found: String,
    },
    /// Unexpected token
    UnexpectedToken(String),
    /// Unexpected <!>
    UnexpectedBang,
    /// Text not found, expected `Event::Text`
    TextNotFound,
    /// `Event::XmlDecl` must start with *version* attribute
    XmlDeclWithoutVersion(Option<String>),
    /// Attribute Name contains quote
    NameWithQuote(usize),
    /// Attribute key not followed by with `=`
    NoEqAfterName(usize),
    /// Attribute value not quoted
    UnquotedValue(usize),
    /// Duplicate attribute
    DuplicatedAttribute(usize, usize),
    /// Escape error
    EscapeError(#[cfg_attr(feature = "failure", cause)] ::escape::EscapeError),
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

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::Utf8(e) => write!(f, "UTF8 error: {}", e),
            Error::UnexpectedEof(e) => write!(f, "Unexpected EOF during reading {}.", e),
            Error::EndEventMismatch { expected, found } => {
                write!(f, "Expecting </{}> found </{}>", expected, found)
            }
            Error::UnexpectedToken(e) => write!(f, "Unexpected token '{}'", e),
            Error::UnexpectedBang => write!(
                f,
                "Only Comment, CDATA and DOCTYPE nodes can start with a '!'"
            ),
            Error::TextNotFound => write!(f, "Cannot read text, expecting Event::Text"),
            Error::XmlDeclWithoutVersion(e) => write!(
                f,
                "XmlDecl must start with 'version' attribute, found {:?}",
                e
            ),
            Error::NameWithQuote(e) => write!(
                f,
                "error while parsing attribute at position {}: \
                 Attribute key cannot contain quote.",
                e
            ),
            Error::NoEqAfterName(e) => write!(
                f,
                "error while parsing attribute at position {}: \
                 Attribute key must be directly followed by = or space",
                e
            ),
            Error::UnquotedValue(e) => write!(
                f,
                "error while parsing attribute at position {}: \
                 Attribute value must start with a quote.",
                e
            ),
            Error::DuplicatedAttribute(pos1, pos2) => write!(
                f,
                "error while parsing attribute at position {0}: \
                 Duplicate attribute at position {1} and {0}",
                pos1, pos2
            ),
            Error::EscapeError(e) => write!(f, "{}", e),
        }
    }
}

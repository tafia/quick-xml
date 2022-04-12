//! Error management module

use crate::escape::EscapeError;
use crate::events::attributes::AttrError;
use std::str::Utf8Error;

/// The error type used by this crate.
#[derive(Debug)]
pub enum Error {
    /// IO error
    Io(::std::io::Error),
    /// Utf8 error
    Utf8(Utf8Error),
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
    UnexpectedBang(u8),
    /// Text not found, expected `Event::Text`
    TextNotFound,
    /// `Event::XmlDecl` must start with *version* attribute
    XmlDeclWithoutVersion(Option<String>),
    /// Attribute parsing error
    InvalidAttr(AttrError),
    /// Escape error
    EscapeError(EscapeError),
}

impl From<::std::io::Error> for Error {
    /// Creates a new `Error::Io` from the given error
    #[inline]
    fn from(error: ::std::io::Error) -> Error {
        Error::Io(error)
    }
}

impl From<Utf8Error> for Error {
    /// Creates a new `Error::Utf8` from the given error
    #[inline]
    fn from(error: Utf8Error) -> Error {
        Error::Utf8(error)
    }
}

impl From<EscapeError> for Error {
    /// Creates a new `Error::EscapeError` from the given error
    #[inline]
    fn from(error: EscapeError) -> Error {
        Error::EscapeError(error)
    }
}

impl From<AttrError> for Error {
    #[inline]
    fn from(error: AttrError) -> Self {
        Error::InvalidAttr(error)
    }
}

/// A specialized `Result` type where the error is hard-wired to [`Error`].
///
/// [`Error`]: enum.Error.html
pub type Result<T> = std::result::Result<T, Error>;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::Utf8(e) => write!(f, "UTF8 error: {}", e),
            Error::UnexpectedEof(e) => write!(f, "Unexpected EOF during reading {}", e),
            Error::EndEventMismatch { expected, found } => {
                write!(f, "Expecting </{}> found </{}>", expected, found)
            }
            Error::UnexpectedToken(e) => write!(f, "Unexpected token '{}'", e),
            Error::UnexpectedBang(b) => write!(
                f,
                "Only Comment (`--`), CDATA (`[CDATA[`) and DOCTYPE (`DOCTYPE`) nodes can start with a '!', but symbol `{}` found",
                *b as char
            ),
            Error::TextNotFound => write!(f, "Cannot read text, expecting Event::Text"),
            Error::XmlDeclWithoutVersion(e) => write!(
                f,
                "XmlDecl must start with 'version' attribute, found {:?}",
                e
            ),
            Error::InvalidAttr(e) => write!(f, "error while parsing attribute: {}", e),
            Error::EscapeError(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Utf8(e) => Some(e),
            Error::InvalidAttr(e) => Some(e),
            Error::EscapeError(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(feature = "serialize")]
pub mod serialize {
    //! A module to handle serde (de)serialization errors

    use super::*;
    use std::fmt;
    use std::num::{ParseFloatError, ParseIntError};

    /// (De)serialization error
    #[derive(Debug)]
    pub enum DeError {
        /// Serde custom error
        Custom(String),
        /// Cannot parse to integer
        Int(ParseIntError),
        /// Cannot parse to float
        Float(ParseFloatError),
        /// Xml parsing error
        Xml(Error),
        /// Unexpected end of attributes.
        ///
        /// Usually this indicates an error in the `Deserialize` implementation when read map:
        /// `MapAccess::next_value[_seed]` was called before `MapAccess::next_key[_seed]`
        EndOfAttributes,
        /// Unexpected end of file
        Eof,
        /// Invalid value for a boolean
        InvalidBoolean(String),
        /// Invalid event for Enum
        InvalidEnum(crate::events::Event<'static>),
        /// Expecting Text event
        Text,
        /// Expecting Start event
        Start,
        /// Expecting End event
        End,
        /// Unsupported operation
        Unsupported(&'static str),
    }

    impl fmt::Display for DeError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                DeError::Custom(s) => write!(f, "{}", s),
                DeError::Xml(e) => write!(f, "{}", e),
                DeError::Int(e) => write!(f, "{}", e),
                DeError::Float(e) => write!(f, "{}", e),
                DeError::EndOfAttributes => write!(f, "Unexpected end of attributes"),
                DeError::Eof => write!(f, "Unexpected `Event::Eof`"),
                DeError::InvalidBoolean(v) => write!(f, "Invalid boolean value '{}'", v),
                DeError::InvalidEnum(e) => write!(
                    f,
                    "Invalid event for Enum, expecting Text or Start, got: {:?}",
                    e
                ),
                DeError::Text => write!(f, "Expecting Text event"),
                DeError::Start => write!(f, "Expecting Start event"),
                DeError::End => write!(f, "Expecting End event"),
                DeError::Unsupported(s) => write!(f, "Unsupported operation {}", s),
            }
        }
    }

    impl ::std::error::Error for DeError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                DeError::Int(e) => Some(e),
                DeError::Float(e) => Some(e),
                DeError::Xml(e) => Some(e),
                _ => None,
            }
        }
    }

    impl serde::de::Error for DeError {
        fn custom<T: fmt::Display>(msg: T) -> Self {
            DeError::Custom(msg.to_string())
        }
    }

    impl serde::ser::Error for DeError {
        fn custom<T: fmt::Display>(msg: T) -> Self {
            DeError::Custom(msg.to_string())
        }
    }

    impl From<Error> for DeError {
        fn from(e: Error) -> Self {
            DeError::Xml(e)
        }
    }

    impl From<EscapeError> for DeError {
        #[inline]
        fn from(e: EscapeError) -> Self {
            Self::Xml(e.into())
        }
    }

    impl From<ParseIntError> for DeError {
        fn from(e: ParseIntError) -> Self {
            DeError::Int(e)
        }
    }

    impl From<ParseFloatError> for DeError {
        fn from(e: ParseFloatError) -> Self {
            DeError::Float(e)
        }
    }
}

//! Error management module

use crate::escape::EscapeError;
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

/// A specialized `Result` type where the error is hard-wired to [`Error`].
///
/// [`Error`]: enum.Error.html
pub type Result<T> = std::result::Result<T, Error>;

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

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Utf8(e) => Some(e),
            Error::EscapeError(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(feature = "serialize")]
pub mod serialize {
    //! A module to handle serde (de)serialization errors

    use super::Error;
    use crate::escape::EscapeError;
    use crate::utils::write_byte_string;
    use std::fmt;
    use std::num::{ParseFloatError, ParseIntError};
    use std::string::FromUtf8Error;

    /// (De)serialization error
    #[derive(Debug)]
    pub enum DeError {
        /// Serde custom error
        Custom(String),
        /// Xml parsing error
        InvalidXml(Error),
        /// Cannot parse to integer
        InvalidInt(ParseIntError),
        /// Cannot parse to float
        InvalidFloat(ParseFloatError),
        /// Cannot parse specified value to boolean
        InvalidBoolean(String),
        /// Unexpected end of attributes.
        ///
        /// Usually this indicates an error in the `Deserialize` implementation when read map:
        /// `MapAccess::next_value[_seed]` was called before `MapAccess::next_key[_seed]`
        EndOfAttributes,
        /// Deserializer encounter a start tag with a specified name when this is
        /// not expecting. This happens when you try to deserialize a primitive
        /// value (numbers, strings, booleans) from an XML element.
        UnexpectedStart(Vec<u8>),
        /// Deserializer encounter an end tag with a specified name when this is
        /// not expecting. Usually that should not be possible, because XML reader
        /// is not able to produce such stream of events that lead to this error.
        ///
        /// If you get this error this likely indicates and error in the `quick_xml`.
        /// Please open an issue at <https://github.com/tafia/quick-xml> and attach
        /// your types and XML input.
        UnexpectedEnd(Vec<u8>),
        /// Unexpected end of file
        UnexpectedEof,
        /// This error indicates that [`deserialize_struct`] was called, but there
        /// is no any XML element in the input. That means that you try to deserialize
        /// a struct not from an XML element.
        ///
        /// [`deserialize_struct`]: serde::de::Deserializer::deserialize_struct
        ExpectedStart,
        /// Unsupported operation
        Unsupported(&'static str),
    }

    impl fmt::Display for DeError {
        fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            match self {
                DeError::Custom(s) => write!(f, "{}", s),
                DeError::InvalidXml(e) => write!(f, "{}", e),
                DeError::InvalidInt(e) => write!(f, "{}", e),
                DeError::InvalidFloat(e) => write!(f, "{}", e),
                DeError::InvalidBoolean(v) => write!(f, "Invalid boolean value '{}'", v),
                DeError::EndOfAttributes => write!(f, "Unexpected end of attributes"),
                DeError::UnexpectedStart(e) => {
                    f.write_str("Unexpected `Event::Start(")?;
                    write_byte_string(f, &e)?;
                    f.write_str(")`")
                }
                DeError::UnexpectedEnd(e) => {
                    f.write_str("Unexpected `Event::End(")?;
                    write_byte_string(f, &e)?;
                    f.write_str(")`")
                }
                DeError::UnexpectedEof => write!(f, "Unexpected `Event::Eof`"),
                DeError::ExpectedStart => write!(f, "Expecting Start event"),
                DeError::Unsupported(s) => write!(f, "Unsupported operation {}", s),
            }
        }
    }

    impl ::std::error::Error for DeError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                DeError::InvalidXml(e) => Some(e),
                DeError::InvalidInt(e) => Some(e),
                DeError::InvalidFloat(e) => Some(e),
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
            DeError::InvalidXml(e)
        }
    }

    impl From<EscapeError> for DeError {
        #[inline]
        fn from(e: EscapeError) -> Self {
            Self::InvalidXml(e.into())
        }
    }

    impl From<FromUtf8Error> for DeError {
        fn from(e: FromUtf8Error) -> Self {
            Self::InvalidXml(e.utf8_error().into())
        }
    }

    impl From<ParseIntError> for DeError {
        fn from(e: ParseIntError) -> Self {
            DeError::InvalidInt(e)
        }
    }

    impl From<ParseFloatError> for DeError {
        fn from(e: ParseFloatError) -> Self {
            DeError::InvalidFloat(e)
        }
    }
}

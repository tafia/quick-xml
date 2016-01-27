//! Error management module

use std::fmt;
use std::io;
use std::str::Utf8Error;

/// An error produced by an operation on Xml data.
#[derive(Debug)]
pub enum Error {
    /// An error originating from reading or writing to the underlying buffer.
    Io(io::Error),
    /// An error originating from finding end of line instead of a column.
    EOL,
    /// An error while converting to utf8
    Utf8(Utf8Error),
    UnexpectedQuote,
}

/// Result type
pub type Result<T> = ::std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => write!(f, "{}", err),
            Error::Utf8(ref err) => write!(f, "{}", err),
            Error::EOL => write!(f, "Trying to access column but found End Of Line"),
            Error::UnexpectedQuote => write!(f, "Unexpected quote while parsing attributes"),
        }
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(..) => "IO error",
            Error::Utf8(..) => "Error while converting to utf8",
            Error::EOL => "Trying to access column but found End Of Line",
            Error::UnexpectedQuote => "Unexpected quote while parsing attributes",
        }
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        match *self {
            Error::Io(ref err) => Some(err),
            Error::Utf8(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error { Error::Io(err) }
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Error { Error::Utf8(err) }
}

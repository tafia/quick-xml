use crate::errors::Error;
use std::fmt;

/// Deserialization error
#[derive(Debug)]
pub enum DeError {
    /// Custom error
    Custom(String),
    /// Cannot parse to integer
    Int(std::num::ParseIntError),
    /// Cannot parse to float
    Float(std::num::ParseFloatError),
    /// Xml parsing error
    Xml(Error),
    /// Unexpected end of attributes
    EndOfAttributes,
    /// Unexpected `Event::Start`
    StartEvent(String),
    /// Cannot deserialize owned event
    OwnedEvent,
    /// Unexpected of of file
    Eof,
    /// Cannot peek event
    Peek,
    /// Unexpected Start event name
    NameMismatch {
        /// Expected name
        expected: &'static str,
        /// Event name found
        found: String,
    },
    /// Invalid value for a boolean
    InvalidBoolean(String),
    /// Invalid unit value
    InvalidUnit(String),
    /// Invalid event for Enum
    InvalidEnum(crate::events::Event<'static>),
    /// Expecting Text event
    Text,
    /// Expecting Start event
    Start,
    /// Expecting End event
    End,
}

impl fmt::Display for DeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            DeError::Custom(s) => write!(f, "{}", s),
            DeError::Xml(e) => write!(f, "{}", e),
            DeError::Int(e) => write!(f, "{}", e),
            DeError::Float(e) => write!(f, "{}", e),
            DeError::EndOfAttributes => write!(f, "Unexpected end of attributes"),
            DeError::StartEvent(n) => write!(f, "Unexpected `Event::Start`: {}", n),
            DeError::OwnedEvent => write!(f, "Cannot deserialize owned event"),
            DeError::Eof => write!(f, "Unexpected `Event::Eof`"),
            DeError::Peek => write!(f, "Cannot peek event"),
            DeError::NameMismatch { expected, found } => write!(
                f,
                "Start event name mismatch:\nExpecting: {}\nFound: {}",
                expected, found
            ),
            DeError::InvalidBoolean(v) => write!(f, "Invalid boolean value '{}'", v),
            DeError::InvalidUnit(v) => {
                write!(f, "Invalid unit value '{}', expected empty string", v)
            }
            DeError::InvalidEnum(e) => write!(
                f,
                "Invalid event for Enum, expecting Text or Start, got: {:?}",
                e
            ),
            DeError::Text => write!(f, "Expecting Text event"),
            DeError::Start => write!(f, "Expecting Start event"),
            DeError::End => write!(f, "Expecting End event"),
        }
    }
}

impl ::std::error::Error for DeError {
    fn description(&self) -> &str {
        "xml deserialize error"
    }
    fn cause(&self) -> Option<&dyn (::std::error::Error)> {
        None
    }
}

impl serde::de::Error for DeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        DeError::Custom(msg.to_string())
    }
}

impl From<Error> for DeError {
    fn from(e: Error) -> Self {
        DeError::Xml(e)
    }
}

impl From<std::num::ParseIntError> for DeError {
    fn from(e: std::num::ParseIntError) -> Self {
        DeError::Int(e)
    }
}

impl From<std::num::ParseFloatError> for DeError {
    fn from(e: std::num::ParseFloatError) -> Self {
        DeError::Float(e)
    }
}

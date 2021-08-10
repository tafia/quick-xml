//! High performance XML reader/writer.
//!
//! ## Description
//!
//! quick-xml contains two modes of operation:
//!
//! A streaming API based on the [StAX] model. This is suited for larger XML documents which
//! cannot completely read into memory at once.
//!
//! The user has to expicitely _ask_ for the next XML event, similar
//! to a database cursor.
//! This is achieved by the following two structs:
//!
//! - [`Reader`]: A low level XML pull-reader where buffer allocation/clearing is left to user.
//! - [`Writer`]: A XML writer. Can be nested with readers if you want to transform XMLs.
//!
//! Especially for nested XML elements, the user must keep track _where_ (how deep) in the XML document
//! the current event is located. This is needed as the
//!
//! Furthermore, quick-xml also contains optional [Serde] support to directly serialize and deserialize from
//! structs, without having to deal with the XML events.
//!
//! ## Examples
//!
//! ### Reader
//!
//! ```rust
//! use quick_xml::Reader;
//! use quick_xml::events::Event;
//!
//! let xml = r#"<tag1 att1 = "test">
//!                 <tag2><!--Test comment-->Test</tag2>
//!                 <tag2>
//!                     Test 2
//!                 </tag2>
//!             </tag1>"#;
//!
//! let mut reader = Reader::from_str(xml);
//! reader.trim_text(true);
//!
//! let mut count = 0;
//! let mut txt = Vec::new();
//! let mut buf = Vec::new();
//!
//! // The `Reader` does not implement `Iterator` because it outputs borrowed data (`Cow`s)
//! loop {
//!     match reader.read_event(&mut buf) {
//!     // for triggering namespaced events, use this instead:
//!     // match reader.read_namespaced_event(&mut buf) {
//!         Ok(Event::Start(ref e)) => {
//!         // for namespaced:
//!         // Ok((ref namespace_value, Event::Start(ref e)))
//!             match e.name() {
//!                 b"tag1" => println!("attributes values: {:?}",
//!                                     e.attributes().map(|a| a.unwrap().value)
//!                                     .collect::<Vec<_>>()),
//!                 b"tag2" => count += 1,
//!                 _ => (),
//!             }
//!         },
//!         // unescape and decode the text event using the reader encoding
//!         Ok(Event::Text(e)) => txt.push(e.unescape_and_decode(&reader).unwrap()),
//!         Ok(Event::Eof) => break, // exits the loop when reaching end of file
//!         Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
//!         _ => (), // There are several other `Event`s we do not consider here
//!     }
//!
//!     // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
//!     buf.clear();
//! }
//! ```
//!
//! ### Writer
//!
//! ```rust
//! use quick_xml::Writer;
//! use quick_xml::events::{Event, BytesEnd, BytesStart};
//! use quick_xml::Reader;
//! use std::io::Cursor;
//! use std::iter;
//!
//! let xml = r#"<this_tag k1="v1" k2="v2"><child>text</child></this_tag>"#;
//! let mut reader = Reader::from_str(xml);
//! reader.trim_text(true);
//! let mut writer = Writer::new(Cursor::new(Vec::new()));
//! let mut buf = Vec::new();
//! loop {
//!     match reader.read_event(&mut buf) {
//!         Ok(Event::Start(ref e)) if e.name() == b"this_tag" => {
//!
//!             // crates a new element ... alternatively we could reuse `e` by calling
//!             // `e.into_owned()`
//!             let mut elem = BytesStart::owned(b"my_elem".to_vec(), "my_elem".len());
//!
//!             // collect existing attributes
//!             elem.extend_attributes(e.attributes().map(|attr| attr.unwrap()));
//!
//!             // copy existing attributes, adds a new my-key="some value" attribute
//!             elem.push_attribute(("my-key", "some value"));
//!
//!             // writes the event to the writer
//!             assert!(writer.write_event(Event::Start(elem)).is_ok());
//!         },
//!         Ok(Event::End(ref e)) if e.name() == b"this_tag" => {
//!             assert!(writer.write_event(Event::End(BytesEnd::borrowed(b"my_elem"))).is_ok());
//!         },
//!         Ok(Event::Eof) => break,
//!         Ok(e) => assert!(writer.write_event(e).is_ok()),
//!         // or using the buffer
//!         // Ok(e) => assert!(writer.write(&buf).is_ok()),
//!         Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
//!     }
//!     buf.clear();
//! }
//!
//! let result = writer.into_inner().into_inner();
//! let expected = r#"<my_elem k1="v1" k2="v2" my-key="some value"><child>text</child></my_elem>"#;
//! assert_eq!(result, expected.as_bytes());
//! ```
//!
//! # Features
//!
//! quick-xml supports 2 additional features, non activated by default:
//! - `encoding`: support non utf8 XMLs
//! - `serialize`: support serde `Serialize`/`Deserialize`
//!
//! [StAX]: https://en.wikipedia.org/wiki/StAX
//! [Serde]: https://serde.rs/
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![recursion_limit = "1024"]

#[cfg(feature = "encoding_rs")]
extern crate encoding_rs;
extern crate memchr;
#[cfg(feature = "serialize")]
extern crate serde;
#[cfg(all(test, feature = "serialize"))]
extern crate serde_value;

#[cfg(feature = "serialize")]
pub mod de;
mod errors;
mod escapei;
pub mod escape {
    //! Manage xml character escapes
    pub(crate) use crate::escapei::{do_unescape, EscapeError};
    pub use crate::escapei::{escape, partial_escape, unescape, unescape_with};
}
pub mod events;
mod reader;
#[cfg(feature = "serialize")]
pub mod se;
mod utils;
mod writer;

// reexports
#[cfg(feature = "serialize")]
pub use crate::errors::serialize::DeError;
pub use crate::errors::{Error, Result};
pub use crate::{reader::Reader, writer::Writer};

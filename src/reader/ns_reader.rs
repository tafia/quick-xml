//! A reader that manages namespace declarations found in the input and able
//! to resolve [qualified names] to [expanded names].
//!
//! [qualified names]: https://www.w3.org/TR/xml-names11/#dt-qualname
//! [expanded names]: https://www.w3.org/TR/xml-names11/#dt-expname

use std::io::BufRead;
use std::ops::{Deref, DerefMut};

use crate::errors::Result;
use crate::events::Event;
use crate::name::{LocalName, NamespaceResolver, QName, ResolveResult};
use crate::reader::Reader;

/// A low level encoding-agnostic XML event reader that performs namespace resolution.
///
/// Consumes a [`BufRead`] and streams XML `Event`s.
pub struct NsReader<R> {
    /// An XML reader
    reader: Reader<R>,
    /// Buffer that contains names of namespace prefixes (the part between `xmlns:`
    /// and an `=`) and namespace values.
    buffer: Vec<u8>,
    /// A buffer to manage namespaces
    ns_resolver: NamespaceResolver,
    /// We cannot pop data from the namespace stack until returned `Empty` or `End`
    /// event will be processed by the user, so we only mark that we should that
    /// in the next [`Self::read_namespaced_event()`] call.
    pending_pop: bool,
}

impl<R> NsReader<R> {
    #[inline]
    fn new(reader: Reader<R>) -> Self {
        Self {
            reader,
            buffer: Vec::new(),
            ns_resolver: NamespaceResolver::default(),
            pending_pop: false,
        }
    }
}

/// Getters
impl<R> NsReader<R> {
    /// Resolves a potentially qualified **event name** into (namespace name, local name).
    ///
    /// *Qualified* attribute names have the form `prefix:local-name` where the`prefix` is defined
    /// on any containing XML element via `xmlns:prefix="the:namespace:uri"`. The namespace prefix
    /// can be defined on the same element as the attribute in question.
    ///
    /// *Unqualified* event inherits the current *default namespace*.
    ///
    /// # Lifetimes
    ///
    /// - `'n`: lifetime of an element name
    #[inline]
    pub fn event_namespace<'n>(&self, name: QName<'n>) -> (ResolveResult, LocalName<'n>) {
        self.ns_resolver.resolve(name, &self.buffer, true)
    }

    /// Resolves a potentially qualified **attribute name** into (namespace name, local name).
    ///
    /// *Qualified* attribute names have the form `prefix:local-name` where the`prefix` is defined
    /// on any containing XML element via `xmlns:prefix="the:namespace:uri"`. The namespace prefix
    /// can be defined on the same element as the attribute in question.
    ///
    /// *Unqualified* attribute names do *not* inherit the current *default namespace*.
    ///
    /// # Lifetimes
    ///
    /// - `'n`: lifetime of an attribute
    #[inline]
    pub fn attribute_namespace<'n>(&self, name: QName<'n>) -> (ResolveResult, LocalName<'n>) {
        self.ns_resolver.resolve(name, &self.buffer, false)
    }
}

impl<R: BufRead> NsReader<R> {
    /// Reads the next event and resolves its namespace (if applicable).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::str::from_utf8;
    /// use quick_xml::NsReader;
    /// use quick_xml::events::Event;
    /// use quick_xml::name::ResolveResult::*;
    ///
    /// let xml = r#"<x:tag1 xmlns:x="www.xxxx" xmlns:y="www.yyyy" att1 = "test">
    ///                 <y:tag2><!--Test comment-->Test</y:tag2>
    ///                 <y:tag2>Test 2</y:tag2>
    ///             </x:tag1>"#;
    /// let mut reader = NsReader::from_str(xml);
    /// reader.trim_text(true);
    /// let mut count = 0;
    /// let mut buf = Vec::new();
    /// let mut txt = Vec::new();
    /// loop {
    ///     match reader.read_namespaced_event(&mut buf) {
    ///         Ok((Bound(ns), Event::Start(e))) => {
    ///             count += 1;
    ///             match (ns.as_ref(), e.local_name().as_ref()) {
    ///                 (b"www.xxxx", b"tag1") => (),
    ///                 (b"www.yyyy", b"tag2") => (),
    ///                 (ns, n) => panic!("Namespace and local name mismatch"),
    ///             }
    ///             println!("Resolved namespace: {:?}", ns);
    ///         }
    ///         Ok((Unbound, Event::Start(_))) => {
    ///             panic!("Element not in any namespace")
    ///         },
    ///         Ok((Unknown(p), Event::Start(_))) => {
    ///             panic!("Undeclared namespace prefix {:?}", String::from_utf8(p))
    ///         }
    ///         Ok((_, Event::Text(e))) => {
    ///             txt.push(e.decode_and_unescape(&reader).unwrap().into_owned())
    ///         },
    ///         Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
    ///         Ok((_, Event::Eof)) => break,
    ///         _ => (),
    ///     }
    ///     buf.clear();
    /// }
    /// println!("Found {} start events", count);
    /// println!("Text events: {:?}", txt);
    /// ```
    pub fn read_namespaced_event<'b>(
        &mut self,
        buf: &'b mut Vec<u8>,
    ) -> Result<(ResolveResult, Event<'b>)> {
        if self.pending_pop {
            self.ns_resolver.pop(&mut self.buffer);
        }
        self.pending_pop = false;
        match self.reader.read_event_into(buf) {
            Ok(Event::Eof) => Ok((ResolveResult::Unbound, Event::Eof)),
            Ok(Event::Start(e)) => {
                self.ns_resolver.push(&e, &mut self.buffer);
                Ok((
                    self.ns_resolver.find(e.name(), &mut self.buffer),
                    Event::Start(e),
                ))
            }
            Ok(Event::Empty(e)) => {
                // For empty elements we need to 'artificially' keep the namespace scope on the
                // stack until the next `next()` call occurs.
                // Otherwise the caller has no chance to use `resolve` in the context of the
                // namespace declarations that are 'in scope' for the empty element alone.
                // Ex: <img rdf:nodeID="abc" xmlns:rdf="urn:the-rdf-uri" />
                self.ns_resolver.push(&e, &mut self.buffer);
                // notify next `read_namespaced_event()` invocation that it needs to pop this
                // namespace scope
                self.pending_pop = true;
                Ok((
                    self.ns_resolver.find(e.name(), &mut self.buffer),
                    Event::Empty(e),
                ))
            }
            Ok(Event::End(e)) => {
                // notify next `read_namespaced_event()` invocation that it needs to pop this
                // namespace scope
                self.pending_pop = true;
                Ok((
                    self.ns_resolver.find(e.name(), &mut self.buffer),
                    Event::End(e),
                ))
            }
            Ok(e) => Ok((ResolveResult::Unbound, e)),
            Err(e) => Err(e),
        }
    }
}

impl<'i> NsReader<&'i [u8]> {
    /// Creates an XML reader from a string slice.
    #[inline]
    pub fn from_str(s: &'i str) -> Self {
        Self::new(Reader::from_str(s))
    }

    /// Creates an XML reader from a slice of bytes.
    #[inline]
    pub fn from_bytes(bytes: &'i [u8]) -> Self {
        Self::new(Reader::from_bytes(bytes))
    }
}

impl<R> Deref for NsReader<R> {
    type Target = Reader<R>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}

impl<R> DerefMut for NsReader<R> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reader
    }
}

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
use crate::reader::{Reader, XmlSource};

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
    /// in the next [`Self::read_event_impl()`] call.
    pending_pop: bool,
}

/// Private methods
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

    fn read_event_impl<'i, B>(&mut self, buf: B) -> Result<Event<'i>>
    where
        R: XmlSource<'i, B>,
    {
        if self.pending_pop {
            self.ns_resolver.pop(&mut self.buffer);
            self.pending_pop = false;
        }
        match self.reader.read_event_impl(buf) {
            Ok(Event::Start(e)) => {
                self.ns_resolver.push(&e, &mut self.buffer);
                Ok(Event::Start(e))
            }
            Ok(Event::Empty(e)) => {
                self.ns_resolver.push(&e, &mut self.buffer);
                // notify next `read_event_impl()` invocation that it needs to pop this
                // namespace scope
                self.pending_pop = true;
                Ok(Event::Empty(e))
            }
            Ok(Event::End(e)) => {
                // notify next `read_event_impl()` invocation that it needs to pop this
                // namespace scope
                self.pending_pop = true;
                Ok(Event::End(e))
            }
            e => e,
        }
    }

    fn read_resolved_event_impl<'i, B>(&mut self, buf: B) -> Result<(ResolveResult, Event<'i>)>
    where
        R: XmlSource<'i, B>,
    {
        match self.read_event_impl(buf) {
            Ok(Event::Start(e)) => Ok((
                self.ns_resolver.find(e.name(), &mut self.buffer),
                Event::Start(e),
            )),
            Ok(Event::Empty(e)) => Ok((
                self.ns_resolver.find(e.name(), &mut self.buffer),
                Event::Empty(e),
            )),
            Ok(Event::End(e)) => Ok((
                self.ns_resolver.find(e.name(), &mut self.buffer),
                Event::End(e),
            )),
            Ok(e) => Ok((ResolveResult::Unbound, e)),
            Err(e) => Err(e),
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
    /// Reads the next event into given buffer.
    ///
    /// This method manages namespaces but doesn't resolve them automatically.
    /// You should call [`event_namespace()`] if you want to get a namespace.
    ///
    /// You also can use [`read_resolved_event_into()`] instead if you want to resolve
    /// namespace as soon as you get an event.
    ///
    /// # Examples
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// use quick_xml::NsReader;
    /// use quick_xml::events::Event;
    /// use quick_xml::name::{Namespace, ResolveResult::*};
    ///
    /// let mut reader = NsReader::from_str(r#"
    ///     <x:tag1 xmlns:x="www.xxxx" xmlns:y="www.yyyy" att1 = "test">
    ///        <y:tag2><!--Test comment-->Test</y:tag2>
    ///        <y:tag2>Test 2</y:tag2>
    ///     </x:tag1>
    /// "#);
    /// reader.trim_text(true);
    ///
    /// let mut count = 0;
    /// let mut buf = Vec::new();
    /// let mut txt = Vec::new();
    /// loop {
    ///     match reader.read_event_into(&mut buf).unwrap() {
    ///         Event::Start(e) => {
    ///             count += 1;
    ///             let (ns, local) = reader.event_namespace(e.name());
    ///             match local.as_ref() {
    ///                 b"tag1" => assert_eq!(ns, Bound(Namespace(b"www.xxxx"))),
    ///                 b"tag2" => assert_eq!(ns, Bound(Namespace(b"www.yyyy"))),
    ///                 _ => unreachable!(),
    ///             }
    ///         }
    ///         Event::Text(e) => {
    ///             txt.push(e.decode_and_unescape(&reader).unwrap().into_owned())
    ///         }
    ///         Event::Eof => break,
    ///         _ => (),
    ///     }
    ///     buf.clear();
    /// }
    /// assert_eq!(count, 3);
    /// assert_eq!(txt, vec!["Test".to_string(), "Test 2".to_string()]);
    /// ```
    ///
    /// [`event_namespace()`]: Self::event_namespace
    /// [`read_resolved_event_into()`]: Self::read_resolved_event_into
    #[inline]
    pub fn read_event_into<'b>(&mut self, buf: &'b mut Vec<u8>) -> Result<Event<'b>> {
        self.read_event_impl(buf)
    }

    /// Reads the next event into given buffer and resolves its namespace (if applicable).
    ///
    /// Namespace is resolved only for [`Start`], [`Empty`] and [`End`] events.
    /// For all other events the concept of namespace is not defined, so
    /// a [`ResolveResult::Unbound`] is returned.
    ///
    /// If you are not interested in namespaces, you can use [`read_event_into()`]
    /// which will not automatically resolve namespaces for you.
    ///
    /// # Examples
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// use quick_xml::NsReader;
    /// use quick_xml::events::Event;
    /// use quick_xml::name::{Namespace, QName, ResolveResult::*};
    ///
    /// let mut reader = NsReader::from_str(r#"
    ///     <x:tag1 xmlns:x="www.xxxx" xmlns:y="www.yyyy" att1 = "test">
    ///        <y:tag2><!--Test comment-->Test</y:tag2>
    ///        <y:tag2>Test 2</y:tag2>
    ///     </x:tag1>
    /// "#);
    /// reader.trim_text(true);
    ///
    /// let mut count = 0;
    /// let mut buf = Vec::new();
    /// let mut txt = Vec::new();
    /// loop {
    ///     match reader.read_resolved_event_into(&mut buf).unwrap() {
    ///         (Bound(Namespace(b"www.xxxx")), Event::Start(e)) => {
    ///             count += 1;
    ///             assert_eq!(e.local_name(), QName(b"tag1").into());
    ///         }
    ///         (Bound(Namespace(b"www.yyyy")), Event::Start(e)) => {
    ///             count += 1;
    ///             assert_eq!(e.local_name(), QName(b"tag2").into());
    ///         }
    ///         (_, Event::Start(_)) => unreachable!(),
    ///
    ///         (_, Event::Text(e)) => {
    ///             txt.push(e.decode_and_unescape(&reader).unwrap().into_owned())
    ///         }
    ///         (_, Event::Eof) => break,
    ///         _ => (),
    ///     }
    ///     buf.clear();
    /// }
    /// assert_eq!(count, 3);
    /// assert_eq!(txt, vec!["Test".to_string(), "Test 2".to_string()]);
    /// ```
    ///
    /// [`Start`]: Event::Start
    /// [`Empty`]: Event::Empty
    /// [`End`]: Event::End
    /// [`read_event_into()`]: Self::read_event_into
    #[inline]
    pub fn read_resolved_event_into<'b>(
        &mut self,
        buf: &'b mut Vec<u8>,
    ) -> Result<(ResolveResult, Event<'b>)> {
        self.read_resolved_event_impl(buf)
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

    /// Reads the next event, borrow its content from the input buffer.
    ///
    /// This method manages namespaces but doesn't resolve them automatically.
    /// You should call [`event_namespace()`] if you want to get a namespace.
    ///
    /// You also can use [`read_resolved_event()`] instead if you want to resolve namespace
    /// as soon as you get an event.
    ///
    /// # Examples
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// use quick_xml::NsReader;
    /// use quick_xml::events::Event;
    /// use quick_xml::name::{Namespace, ResolveResult::*};
    ///
    /// let mut reader = NsReader::from_str(r#"
    ///     <x:tag1 xmlns:x="www.xxxx" xmlns:y="www.yyyy" att1 = "test">
    ///        <y:tag2><!--Test comment-->Test</y:tag2>
    ///        <y:tag2>Test 2</y:tag2>
    ///     </x:tag1>
    /// "#);
    /// reader.trim_text(true);
    ///
    /// let mut count = 0;
    /// let mut txt = Vec::new();
    /// loop {
    ///     match reader.read_event().unwrap() {
    ///         Event::Start(e) => {
    ///             count += 1;
    ///             let (ns, local) = reader.event_namespace(e.name());
    ///             match local.as_ref() {
    ///                 b"tag1" => assert_eq!(ns, Bound(Namespace(b"www.xxxx"))),
    ///                 b"tag2" => assert_eq!(ns, Bound(Namespace(b"www.yyyy"))),
    ///                 _ => unreachable!(),
    ///             }
    ///         }
    ///         Event::Text(e) => {
    ///             txt.push(e.decode_and_unescape(&reader).unwrap().into_owned())
    ///         }
    ///         Event::Eof => break,
    ///         _ => (),
    ///     }
    /// }
    /// assert_eq!(count, 3);
    /// assert_eq!(txt, vec!["Test".to_string(), "Test 2".to_string()]);
    /// ```
    ///
    /// [`event_namespace()`]: Self::event_namespace
    /// [`read_resolved_event()`]: Self::read_resolved_event
    #[inline]
    pub fn read_event(&mut self) -> Result<Event<'i>> {
        self.read_event_impl(())
    }

    /// Reads the next event, borrow its content from the input buffer, and resolves
    /// its namespace (if applicable).
    ///
    /// Namespace is resolved only for [`Start`], [`Empty`] and [`End`] events.
    /// For all other events the concept of namespace is not defined, so
    /// a [`ResolveResult::Unbound`] is returned.
    ///
    /// If you are not interested in namespaces, you can use [`read_event()`]
    /// which will not automatically resolve namespaces for you.
    ///
    /// # Examples
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// use quick_xml::NsReader;
    /// use quick_xml::events::Event;
    /// use quick_xml::name::{Namespace, QName, ResolveResult::*};
    ///
    /// let mut reader = NsReader::from_str(r#"
    ///     <x:tag1 xmlns:x="www.xxxx" xmlns:y="www.yyyy" att1 = "test">
    ///        <y:tag2><!--Test comment-->Test</y:tag2>
    ///        <y:tag2>Test 2</y:tag2>
    ///     </x:tag1>
    /// "#);
    /// reader.trim_text(true);
    ///
    /// let mut count = 0;
    /// let mut txt = Vec::new();
    /// loop {
    ///     match reader.read_resolved_event().unwrap() {
    ///         (Bound(Namespace(b"www.xxxx")), Event::Start(e)) => {
    ///             count += 1;
    ///             assert_eq!(e.local_name(), QName(b"tag1").into());
    ///         }
    ///         (Bound(Namespace(b"www.yyyy")), Event::Start(e)) => {
    ///             count += 1;
    ///             assert_eq!(e.local_name(), QName(b"tag2").into());
    ///         }
    ///         (_, Event::Start(_)) => unreachable!(),
    ///
    ///         (_, Event::Text(e)) => {
    ///             txt.push(e.decode_and_unescape(&reader).unwrap().into_owned())
    ///         }
    ///         (_, Event::Eof) => break,
    ///         _ => (),
    ///     }
    /// }
    /// assert_eq!(count, 3);
    /// assert_eq!(txt, vec!["Test".to_string(), "Test 2".to_string()]);
    /// ```
    ///
    /// [`Start`]: Event::Start
    /// [`Empty`]: Event::Empty
    /// [`End`]: Event::End
    /// [`read_event()`]: Self::read_event
    #[inline]
    pub fn read_resolved_event(&mut self) -> Result<(ResolveResult, Event<'i>)> {
        self.read_resolved_event_impl(())
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

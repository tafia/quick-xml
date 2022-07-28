//! A reader that manages namespace declarations found in the input and able
//! to resolve [qualified names] to [expanded names].
//!
//! [qualified names]: https://www.w3.org/TR/xml-names11/#dt-qualname
//! [expanded names]: https://www.w3.org/TR/xml-names11/#dt-expname

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ops::Deref;
use std::path::Path;

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

/// Builder methods
impl<R> NsReader<R> {
    /// Creates a `NsReader` that reads from a reader.
    #[inline]
    pub fn from_reader(reader: R) -> Self {
        Self::new(Reader::from_reader(reader))
    }

    configure_methods!(reader);
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
    /// Consumes `NsReader` returning the underlying reader
    ///
    /// See the [`Reader::into_inner`] for examples
    #[inline]
    pub fn into_inner(self) -> R {
        self.reader.into_inner()
    }

    /// Gets a mutable reference to the underlying reader.
    pub fn get_mut(&mut self) -> &mut R {
        self.reader.get_mut()
    }

    /// Resolves a potentially qualified **element name** or **attribute name**
    /// into (namespace name, local name).
    ///
    /// *Qualified* names have the form `prefix:local-name` where the `prefix`
    /// is defined on any containing XML element via `xmlns:prefix="the:namespace:uri"`.
    /// The namespace prefix can be defined on the same element as the name in question.
    ///
    /// The method returns following results depending on the `name` shape,
    /// `attribute` flag and the presence of the default namespace:
    ///
    /// |attribute|`xmlns="..."`|QName              |ResolveResult          |LocalName
    /// |---------|-------------|-------------------|-----------------------|------------
    /// |`true`   |Not defined  |`local-name`       |[`Unbound`]            |`local-name`
    /// |`true`   |Defined      |`local-name`       |[`Unbound`]            |`local-name`
    /// |`true`   |_any_        |`prefix:local-name`|[`Bound`] / [`Unknown`]|`local-name`
    /// |`false`  |Not defined  |`local-name`       |[`Unbound`]            |`local-name`
    /// |`false`  |Defined      |`local-name`       |[`Bound`] (default)    |`local-name`
    /// |`false`  |_any_        |`prefix:local-name`|[`Bound`] / [`Unknown`]|`local-name`
    ///
    /// If you want to clearly indicate that name that you resolve is an element
    /// or an attribute name, you could use [`resolve_attribute()`] or [`resolve_element()`]
    /// methods.
    ///
    /// # Lifetimes
    ///
    /// - `'n`: lifetime of a name. Returned local name will be bound to the same
    ///   lifetime as the name in question.
    /// - returned namespace name will be bound to the reader itself
    ///
    /// [`Bound`]: ResolveResult::Bound
    /// [`Unbound`]: ResolveResult::Unbound
    /// [`Unknown`]: ResolveResult::Unknown
    /// [`resolve_attribute()`]: Self::resolve_attribute()
    /// [`resolve_element()`]: Self::resolve_element()
    #[inline]
    pub fn resolve<'n>(&self, name: QName<'n>, attribute: bool) -> (ResolveResult, LocalName<'n>) {
        self.ns_resolver.resolve(name, &self.buffer, !attribute)
    }

    /// Resolves a potentially qualified **element name** into (namespace name, local name).
    ///
    /// *Qualified* element names have the form `prefix:local-name` where the
    /// `prefix` is defined on any containing XML element via `xmlns:prefix="the:namespace:uri"`.
    /// The namespace prefix can be defined on the same element as the element
    /// in question.
    ///
    /// *Unqualified* elements inherits the current *default namespace*.
    ///
    /// The method returns following results depending on the `name` shape and
    /// the presence of the default namespace:
    ///
    /// |`xmlns="..."`|QName              |ResolveResult          |LocalName
    /// |-------------|-------------------|-----------------------|------------
    /// |Not defined  |`local-name`       |[`Unbound`]            |`local-name`
    /// |Defined      |`local-name`       |[`Bound`] (default)    |`local-name`
    /// |_any_        |`prefix:local-name`|[`Bound`] / [`Unknown`]|`local-name`
    ///
    /// # Lifetimes
    ///
    /// - `'n`: lifetime of an element name. Returned local name will be bound
    ///   to the same lifetime as the name in question.
    /// - returned namespace name will be bound to the reader itself
    ///
    /// # Examples
    ///
    /// This example shows how you can resolve qualified name into a namespace.
    /// Note, that in the code like this you do not need to do that manually,
    /// because the namespace resolution result returned by the [`read_event()`].
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// use quick_xml::events::Event;
    /// use quick_xml::name::{Namespace, QName, ResolveResult::*};
    /// use quick_xml::NsReader;
    ///
    /// let mut reader = NsReader::from_str("<tag xmlns='root namespace'/>");
    ///
    /// match reader.read_event().unwrap() {
    ///     Event::Empty(e) => assert_eq!(
    ///         reader.resolve_element(e.name()),
    ///         (Bound(Namespace(b"root namespace")), QName(b"tag").into())
    ///     ),
    ///     _ => unreachable!(),
    /// }
    /// ```
    ///
    /// [`Bound`]: ResolveResult::Bound
    /// [`Unbound`]: ResolveResult::Unbound
    /// [`Unknown`]: ResolveResult::Unknown
    /// [`read_event()`]: Self::read_event
    #[inline]
    pub fn resolve_element<'n>(&self, name: QName<'n>) -> (ResolveResult, LocalName<'n>) {
        self.ns_resolver.resolve(name, &self.buffer, true)
    }

    /// Resolves a potentially qualified **attribute name** into (namespace name, local name).
    ///
    /// *Qualified* attribute names have the form `prefix:local-name` where the
    /// `prefix` is defined on any containing XML element via `xmlns:prefix="the:namespace:uri"`.
    /// The namespace prefix can be defined on the same element as the attribute
    /// in question.
    ///
    /// *Unqualified* attribute names do *not* inherit the current *default namespace*.
    ///
    /// The method returns following results depending on the `name` shape and
    /// the presence of the default namespace:
    ///
    /// |`xmlns="..."`|QName              |ResolveResult          |LocalName
    /// |-------------|-------------------|-----------------------|------------
    /// |Not defined  |`local-name`       |[`Unbound`]            |`local-name`
    /// |Defined      |`local-name`       |[`Unbound`]            |`local-name`
    /// |_any_        |`prefix:local-name`|[`Bound`] / [`Unknown`]|`local-name`
    ///
    /// # Lifetimes
    ///
    /// - `'n`: lifetime of an attribute name. Returned local name will be bound
    ///   to the same lifetime as the name in question.
    /// - returned namespace name will be bound to the reader itself
    ///
    /// # Examples
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// use quick_xml::events::Event;
    /// use quick_xml::events::attributes::Attribute;
    /// use quick_xml::name::{Namespace, QName, ResolveResult::*};
    /// use quick_xml::NsReader;
    ///
    /// let mut reader = NsReader::from_str("
    ///     <tag one='1'
    ///          p:two='2'
    ///          xmlns='root namespace'
    ///          xmlns:p='other namespace'/>
    /// ");
    /// reader.trim_text(true);
    ///
    /// match reader.read_event().unwrap() {
    ///     Event::Empty(e) => {
    ///         let mut iter = e.attributes();
    ///
    ///         // Unlike elements, attributes without explicit namespace
    ///         // not bound to any namespace
    ///         let one = iter.next().unwrap().unwrap();
    ///         assert_eq!(
    ///             reader.resolve_attribute(one.key),
    ///             (Unbound, QName(b"one").into())
    ///         );
    ///
    ///         let two = iter.next().unwrap().unwrap();
    ///         assert_eq!(
    ///             reader.resolve_attribute(two.key),
    ///             (Bound(Namespace(b"other namespace")), QName(b"two").into())
    ///         );
    ///     }
    ///     _ => unreachable!(),
    /// }
    /// ```
    ///
    /// [`Bound`]: ResolveResult::Bound
    /// [`Unbound`]: ResolveResult::Unbound
    /// [`Unknown`]: ResolveResult::Unknown
    #[inline]
    pub fn resolve_attribute<'n>(&self, name: QName<'n>) -> (ResolveResult, LocalName<'n>) {
        self.ns_resolver.resolve(name, &self.buffer, false)
    }
}

impl<R: BufRead> NsReader<R> {
    /// Reads the next event into given buffer.
    ///
    /// This method manages namespaces but doesn't resolve them automatically.
    /// You should call [`resolve_element()`] if you want to get a namespace.
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
    ///             let (ns, local) = reader.resolve_element(e.name());
    ///             match local.as_ref() {
    ///                 b"tag1" => assert_eq!(ns, Bound(Namespace(b"www.xxxx"))),
    ///                 b"tag2" => assert_eq!(ns, Bound(Namespace(b"www.yyyy"))),
    ///                 _ => unreachable!(),
    ///             }
    ///         }
    ///         Event::Text(e) => {
    ///             txt.push(e.unescape().unwrap().into_owned())
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
    /// [`resolve_element()`]: Self::resolve_element
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
    ///             txt.push(e.unescape().unwrap().into_owned())
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

    /// Reads until end element is found using provided buffer as intermediate
    /// storage for events content. This function is supposed to be called after
    /// you already read a [`Start`] event.
    ///
    /// Manages nested cases where parent and child elements have the same name
    /// ("the same" means that their local names are the same and their prefixes
    /// resolves to the same namespace).
    ///
    /// If corresponding [`End`] event will not be found, the [`UnexpectedEof`]
    /// will be returned. In particularly, that error will be returned if you call
    /// this method without consuming the corresponding [`Start`] event first.
    ///
    /// If your reader created from a string slice or byte array slice, it is
    /// better to use [`read_to_end()`] method, because it will not copy bytes
    /// into intermediate buffer.
    ///
    /// The provided `buf` buffer will be filled only by one event content at time.
    /// Before reading of each event the buffer will be cleared. If you know an
    /// appropriate size of each event, you can preallocate the buffer to reduce
    /// number of reallocations.
    ///
    /// The `ns` and `end` parameters should contain namespace and name of the
    /// end element _in the reader encoding_. It is good practice to always get
    /// that parameters using [`BytesStart::to_end()`] method.
    ///
    /// # Namespaces
    ///
    /// Unlike [`Reader::read_to_end_into()`], this method resolves namespace
    /// prefixes, so the names that are not equals literally (for example,
    /// `a:name` and `b:name`) could be considered equal if prefixes resolved to
    /// the same namespace.
    ///
    /// # Examples
    ///
    /// This example shows, how you can skip XML content after you read the
    /// start event.
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// use quick_xml::events::{BytesStart, Event};
    /// use quick_xml::name::{Namespace, ResolveResult};
    /// use quick_xml::NsReader;
    ///
    /// let mut reader = NsReader::from_str(r#"
    ///     <outer xmlns="namespace 1">
    ///         <inner xmlns="namespace 2">
    ///             <outer></outer>
    ///         </inner>
    ///         <inner>
    ///             <inner></inner>
    ///             <inner/>
    ///             <outer></outer>
    ///             <p:outer xmlns:p="ns"></p:outer>
    ///             <outer/>
    ///         </inner>
    ///     </outer>
    /// "#);
    /// reader.trim_text(true);
    /// let mut buf = Vec::new();
    ///
    /// let ns = Namespace(b"namespace 1");
    /// let start = BytesStart::from_content(r#"outer xmlns="namespace 1""#, 5);
    /// let end   = start.to_end().into_owned();
    ///
    /// // First, we read a start event...
    /// assert_eq!(
    ///     reader.read_resolved_event_into(&mut buf).unwrap(),
    ///     (ResolveResult::Bound(ns), Event::Start(start))
    /// );
    ///
    /// //...then, we could skip all events to the corresponding end event.
    /// // This call will correctly handle nested <outer> elements.
    /// // Note, however, that this method does not handle namespaces.
    /// reader.read_to_end_into(end.name(), &mut buf).unwrap();
    ///
    /// // At the end we should get an Eof event, because we ate the whole XML
    /// assert_eq!(
    ///     reader.read_resolved_event_into(&mut buf).unwrap(),
    ///     (ResolveResult::Unbound, Event::Eof)
    /// );
    /// ```
    ///
    /// [`Start`]: Event::Start
    /// [`End`]: Event::End
    /// [`UnexpectedEof`]: crate::errors::Error::UnexpectedEof
    /// [`read_to_end()`]: Self::read_to_end
    /// [`BytesStart::to_end()`]: crate::events::BytesStart::to_end
    #[inline]
    pub fn read_to_end_into(&mut self, end: QName, buf: &mut Vec<u8>) -> Result<()> {
        // According to the https://www.w3.org/TR/xml11/#dt-etag, end name should
        // match literally the start name. See `Self::check_end_names` documentation
        self.reader.read_to_end_into(end, buf)
    }
}

impl NsReader<BufReader<File>> {
    /// Creates an XML reader from a file path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self::new(Reader::from_file(path)?))
    }
}

impl<'i> NsReader<&'i [u8]> {
    /// Creates an XML reader from a string slice.
    #[inline]
    pub fn from_str(s: &'i str) -> Self {
        Self::new(Reader::from_str(s))
    }

    /// Reads the next event, borrow its content from the input buffer.
    ///
    /// This method manages namespaces but doesn't resolve them automatically.
    /// You should call [`resolve_element()`] if you want to get a namespace.
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
    ///             let (ns, local) = reader.resolve_element(e.name());
    ///             match local.as_ref() {
    ///                 b"tag1" => assert_eq!(ns, Bound(Namespace(b"www.xxxx"))),
    ///                 b"tag2" => assert_eq!(ns, Bound(Namespace(b"www.yyyy"))),
    ///                 _ => unreachable!(),
    ///             }
    ///         }
    ///         Event::Text(e) => {
    ///             txt.push(e.unescape().unwrap().into_owned())
    ///         }
    ///         Event::Eof => break,
    ///         _ => (),
    ///     }
    /// }
    /// assert_eq!(count, 3);
    /// assert_eq!(txt, vec!["Test".to_string(), "Test 2".to_string()]);
    /// ```
    ///
    /// [`resolve_element()`]: Self::resolve_element
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
    ///             txt.push(e.unescape().unwrap().into_owned())
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

    /// Reads until end element is found. This function is supposed to be called
    /// after you already read a [`Start`] event.
    ///
    /// Manages nested cases where parent and child elements have the same name
    /// ("the same" means that their local names are the same and their prefixes
    /// resolves to the same namespace).
    ///
    /// If corresponding [`End`] event will not be found, the [`UnexpectedEof`]
    /// will be returned. In particularly, that error will be returned if you call
    /// this method without consuming the corresponding [`Start`] event first.
    ///
    /// The `end` parameter should contain name of the end element _in the reader
    /// encoding_. It is good practice to always get that parameter using
    /// [`BytesStart::to_end()`] method.
    ///
    /// # Namespaces
    ///
    /// Unlike [`Reader::read_to_end()`], this method resolves namespace
    /// prefixes, so the names that are not equals literally (for example,
    /// `a:name` and `b:name`) could be considered equal if prefixes resolved to
    /// the same namespace.
    ///
    /// # Examples
    ///
    /// This example shows, how you can skip XML content after you read the
    /// start event.
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// use quick_xml::events::{BytesStart, Event};
    /// use quick_xml::name::{Namespace, ResolveResult};
    /// use quick_xml::NsReader;
    ///
    /// let mut reader = NsReader::from_str(r#"
    ///     <outer xmlns="namespace 1">
    ///         <inner xmlns="namespace 2">
    ///             <outer></outer>
    ///         </inner>
    ///         <inner>
    ///             <inner></inner>
    ///             <inner/>
    ///             <outer></outer>
    ///             <p:outer xmlns:p="ns"></p:outer>
    ///             <outer/>
    ///         </inner>
    ///     </outer>
    /// "#);
    /// reader.trim_text(true);
    ///
    /// let ns = Namespace(b"namespace 1");
    /// let start = BytesStart::from_content(r#"outer xmlns="namespace 1""#, 5);
    /// let end   = start.to_end().into_owned();
    ///
    /// // First, we read a start event...
    /// assert_eq!(
    ///     reader.read_resolved_event().unwrap(),
    ///     (ResolveResult::Bound(ns), Event::Start(start))
    /// );
    ///
    /// //...then, we could skip all events to the corresponding end event.
    /// // This call will correctly handle nested <outer> elements.
    /// // Note, however, that this method does not handle namespaces.
    /// reader.read_to_end(end.name()).unwrap();
    ///
    /// // At the end we should get an Eof event, because we ate the whole XML
    /// assert_eq!(
    ///     reader.read_resolved_event().unwrap(),
    ///     (ResolveResult::Unbound, Event::Eof)
    /// );
    /// ```
    ///
    /// [`Start`]: Event::Start
    /// [`End`]: Event::End
    /// [`UnexpectedEof`]: crate::errors::Error::UnexpectedEof
    /// [`read_to_end()`]: Self::read_to_end
    /// [`BytesStart::to_end()`]: crate::events::BytesStart::to_end
    #[inline]
    pub fn read_to_end(&mut self, end: QName) -> Result<()> {
        // According to the https://www.w3.org/TR/xml11/#dt-etag, end name should
        // match literally the start name. See `Self::check_end_names` documentation
        self.reader.read_to_end(end)
    }
}

impl<R> Deref for NsReader<R> {
    type Target = Reader<R>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}

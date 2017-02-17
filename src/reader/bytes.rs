//! A module to handle `XmlBytesReader`

use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::borrow::Cow;
use std::ops::Range;

use error::{Error, Result, ResultPos};
use escape::unescape;
use reader::attributes::{Attributes, UnescapedAttributes};
use AsStr;

#[derive(Clone)]
enum TagState {
    Opened,
    Closed,
//     Empty,
}

/// General content of an event (aka node)
///
/// BytesElement is a wrapper over the bytes representing the node:
///
/// E.g. given a node `<name att1="a", att2="b">`, the corresponding `BytesEvent` will be
///
/// ```ignore
/// BytesEvent::Start(BytesElement {
///     buf:    b"name att1=\"a\", att2=\"b\"",
///     start:  0,
///     end:    b"name att1=\"a\", att2=\"b\"".len(),
///     name_end: b"name".len()
/// })
/// ```
///
/// For performance reasons, most of the time, no character searches but
/// `b'<'` and `b'>'` are performed:
///
/// - no attribute parsing: use lazy `Attributes` iterator only when needed
/// - no namespace awareness as it requires parsing all `Start` element attributes
/// - no utf8 conversion: prefer searching statically binary comparisons
/// then use the `as_str` or `into_string` methods
#[derive(Clone)]
pub struct BytesElement<'a> {
    /// content of the element, before any utf8 conversion
    buf: Cow<'a, [u8]>,
    /// content range, excluding text defining BytesEvent type
    content: Range<usize>,
    /// element name range
    name: Range<usize>,
}

impl<'a> BytesElement<'a> {

    /// Creates a new `BytesElement` from the given name.
    /// name is a reference that can be converted to a byte slice,
    /// such as &[u8] or &str
    pub fn new(name: &'a [u8]) -> BytesElement<'a> {
        let bytes = Cow::Borrowed(name.as_ref());
        let end = bytes.len();
        BytesElement::from_buffer(bytes, 0, end, end)
    }

    /// private function to create a new element from a buffer.
    #[inline]
    fn from_buffer(buf: Cow<'a,[u8]>, start: usize, end: usize, name_end: usize)
        -> BytesElement
    {
        BytesElement {
            buf: buf,
            content: start..end,
            name: start..name_end,
        }
    }

    /// Consumes self and adds attributes to this element from an iterator
    /// over (key, value) tuples.
    /// Key and value can be anything that implements the AsRef<[u8]> trait,
    /// like byte slices and strings.
    pub fn with_attributes<K, V, I>(mut self, attributes: I) -> Self
        where K: AsRef<[u8]>,
              V: AsRef<[u8]>,
              I: IntoIterator<Item = (K, V)>
    {
        self.extend_attributes(attributes);
        self
    }

    /// name as &[u8] (without eventual attributes)
    pub fn name(&self) -> &[u8] {
        &self.buf[self.name.clone()]
    }

    /// whole content as &[u8] (including eventual attributes)
    pub fn content(&self) -> &[u8] {
        &self.buf[self.content.clone()]
    }

    /// gets escaped content
    ///
    /// Searches for '&' into content and try to escape the coded character if possible
    /// returns Malformed error with index within element if '&' is not followed by ';'
    pub fn unescaped_content(&self) -> ResultPos<Cow<[u8]>> {
        unescape(self.content())
    }

    /// gets attributes iterator
    pub fn attributes(&self) -> Attributes {
        Attributes::new(self.content(), self.name.end)
    }

    /// gets attributes iterator whose attribute values are unescaped ('&...;' replaced
    /// by their corresponding character)
    pub fn unescaped_attributes(&self) -> UnescapedAttributes {
        self.attributes().unescaped()
    }

    /// extend the attributes of this element from an iterator over (key, value) tuples.
    /// Key and value can be anything that implements the AsRef<[u8]> trait,
    /// like byte slices and strings.
    pub fn extend_attributes<K, V, I>(&mut self, attributes: I) -> &mut BytesElement<'a>
        where K: AsRef<[u8]>,
              V: AsRef<[u8]>,
              I: IntoIterator<Item = (K, V)>
    {
        for attr in attributes {
            self.push_attribute(attr.0, attr.1);
        }
        self
    }

    /// consumes entire self (including eventual attributes!) and returns `String`
    ///
    /// useful when we need to get Text event value (which don't have attributes)
    pub fn into_string(self) -> Result<String> {
        ::std::string::String::from_utf8(self.buf.into_owned())
            .map_err(|e| Error::Utf8(e.utf8_error()))
    }
    
    /// consumes entire self (including eventual attributes!) and returns `String`
    ///
    /// useful when we need to get Text event value (which don't have attributes)
    /// and unescape XML entities
    pub fn into_unescaped_string(self) -> Result<String> {
        ::std::string::String::from_utf8(
            try!(self.unescaped_content().map_err(|(e, _)| e)).into_owned())
            .map_err(|e| Error::Utf8(e.utf8_error()))
    }

    /// Adds an attribute to this element from the given key and value.
    /// Key and value can be anything that implements the AsRef<[u8]> trait,
    /// like byte slices and strings.
    pub fn push_attribute<K, V>(&mut self, key: K, value: V)
        where K: AsRef<[u8]>,
              V: AsRef<[u8]>
    {
        let bytes = self.buf.to_mut();
        bytes.push(b' ');
        bytes.extend_from_slice(key.as_ref());
        bytes.extend_from_slice(b"=\"");
        bytes.extend_from_slice(value.as_ref());
        bytes.push(b'"');
        self.content.end = bytes.len();
    }
}

impl<'a> fmt::Debug for BytesElement<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> ::std::result::Result<(), fmt::Error> {
        write!(f,
               "BytesElement {{ buf: {:?}, name_end: {}, end: {} }}",
               self.content().as_str(),
               self.name.end,
               self.content.end)
    }
}

/// Wrapper around `BytesElement` to parse/write `XmlDecl`
///
/// Postpone element parsing only when needed.
///
/// [W3C XML 1.1 Prolog and Document Type Delcaration](http://w3.org/TR/xml11/#sec-prolog-dtd)
#[derive(Clone, Debug)]
pub struct XmlBytesDecl<'a> {
    element: BytesElement<'a>,
}

impl<'a> XmlBytesDecl<'a> {

    /// Gets xml version, including quotes (' or ")
    pub fn version(&self) -> ResultPos<&[u8]> {
        match self.element.attributes().next() {
            Some(Err(e)) => Err(e),
            Some(Ok((b"version", v))) => Ok(v),
            Some(Ok((k, _))) => {
                let m = format!("XmlDecl must start with 'version' attribute, found {:?}",
                                k.as_str());
                Err((Error::Malformed(m), 0))
            }
            None => {
                let m = "XmlDecl must start with 'version' attribute, found none".to_string();
                Err((Error::Malformed(m), 0))
            }
        }
    }

    /// Gets xml encoding, including quotes (' or ")
    pub fn encoding(&self) -> Option<ResultPos<&[u8]>> {
        for a in self.element.attributes() {
            match a {
                Err(e) => return Some(Err(e)),
                Ok((b"encoding", v)) => return Some(Ok(v)),
                _ => (),
            }
        }
        None
    }

    /// Gets xml standalone, including quotes (' or ")
    pub fn standalone(&self) -> Option<ResultPos<&[u8]>> {
        for a in self.element.attributes() {
            match a {
                Err(e) => return Some(Err(e)),
                Ok((b"standalone", v)) => return Some(Ok(v)),
                _ => (),
            }
        }
        None
    }

    /// Constructs a new `XmlDecl` from the (mandatory) _version_ (should be `1.0` or `1.1`),
    /// the optional _encoding_ (e.g., `UTF-8`) and the optional _standalone_ (`yes` or `no`)
    /// attribute.
    ///
    /// Does not escape any of its inputs. Always uses double quotes to wrap the attribute values.
    /// The caller is responsible for escaping attribute values. Shouldn't usually be relevant since
    /// the double quote character is not allowed in any of the attribute values.
    pub fn new(version: &[u8], encoding: Option<&[u8]>, standalone: Option<&[u8]>) -> XmlBytesDecl<'static> {
        // Compute length of the buffer based on supplied attributes
        // ' encoding=""'   => 12
        let encoding_attr_len = if let Some(xs) = encoding { 12 + xs.len() } else { 0 };
        // ' standalone=""' => 14
        let standalone_attr_len = if let Some(xs) = standalone { 14 + xs.len() } else { 0 };
        // 'xml version=""' => 14
        let mut buf = Vec::with_capacity(14 + encoding_attr_len + standalone_attr_len);

        buf.extend_from_slice(b"xml version=\"");
        buf.extend_from_slice(version);

        if let Some(encoding_val) = encoding {
            buf.extend_from_slice(b"\" encoding=\"");
            buf.extend_from_slice(encoding_val);
        }

        if let Some(standalone_val) = standalone {
            buf.extend_from_slice(b"\" standalone=\"");
            buf.extend_from_slice(standalone_val);
        }
        buf.push(b'"');

        let buf_len = buf.len();
        XmlBytesDecl { element: BytesElement::from_buffer(Cow::Owned(buf), 0, buf_len, 3) }
    }
}

/// BytesEvent to interprete node as they are parsed
#[derive(Clone, Debug)]
pub enum BytesEvent<'a> {
    /// Start tag (with attributes) <...>
    Start(BytesElement<'a>),
    /// End tag </...>
    End(BytesElement<'a>),
    /// Empty element tag (with attributes) <.../>
    Empty(BytesElement<'a>),
    /// Data between Start and End element
    Text(BytesElement<'a>),
    /// Comment <!-- ... -->
    Comment(BytesElement<'a>),
    /// CData <![CDATA[...]]>
    CData(BytesElement<'a>),
    /// Xml declaration <?xml ...?>
    Decl(XmlBytesDecl<'a>),
    /// Processing instruction <?...?>
    PI(BytesElement<'a>),
    /// Doctype <!DOCTYPE...>
    DocType(BytesElement<'a>),
}

impl<'a> BytesEvent<'a> {
    /// returns inner BytesElement for the event
    pub fn element(&self) -> &BytesElement<'a> {
        match *self {
            BytesEvent::Start(ref e) |
            BytesEvent::End(ref e) |
            BytesEvent::Empty(ref e) |
            BytesEvent::Text(ref e) |
            BytesEvent::Comment(ref e) |
            BytesEvent::CData(ref e) |
            BytesEvent::PI(ref e) |
            BytesEvent::DocType(ref e) => e,
            BytesEvent::Decl(ref e) => &e.element,
        }
    }
}

/// A low level Xml bytes reader
///
/// Consumes a `BufRead` and streams xml `BytesEvent`s
///
/// ```
/// use quick_xml::reader::bytes::{XmlBytesReader, BytesEvent};
///
/// let xml = r#"<tag1 att1 = "test">
///                 <tag2><!--Test comment-->Test</tag2>
///                 <tag2>Test 2</tag2>
///             </tag1>"#;
/// let mut reader = XmlBytesReader::from(xml).trim_text(true);
/// let mut count = 0;
/// let mut txt = Vec::new();
/// let mut buf = Vec::new();
/// loop {
///     match reader.next_event(&mut buf) {
///         Some(Ok(BytesEvent::Start(ref e))) => {
///             match e.name() {
///                 b"tag1" => println!("attributes values: {:?}",
///                                     e.attributes()
///                                     .map(|a| a.unwrap().1)
///                                     .collect::<Vec<_>>()),
///                 b"tag2" => count += 1,
///                 _ => (),
///             }
///         },
///         Some(Ok(BytesEvent::Text(e))) => txt.push(e.into_string()),
///         Some(Err((e, pos))) => panic!("{:?} at position {}", e, pos),
///         None => break,
///         _ => (),
///     }
/// }
/// ```
#[derive(Clone)]
pub struct XmlBytesReader<B: BufRead> {
    /// reader
    reader: B,
    /// if was error, exit next
    exit: bool,
    /// all currently Started elements which didn't have a matching
    /// End element yet
//     opened: Vec<BytesElement>,
    /// current state Open/Close
    tag_state: TagState,
    /// expand empty element into an opening and closing element
    expand_empty_elements: bool,
    /// trims Text events, skip the element if text is empty
    trim_text: bool,
    /// check if End nodes match last Start node
    with_check: bool,
    /// check if comments contains `--` (false per default)
    check_comments: bool,
    /// current buffer position, useful for debuging errors
    buf_position: usize,
}

impl<'a> ::std::convert::From<&'a str> for XmlBytesReader<&'a [u8]> {
    fn from(reader: &'a str) -> XmlBytesReader<&'a [u8]> {
        XmlBytesReader::from_reader(reader.as_bytes())
    }
}

impl<B: BufRead> XmlBytesReader<B> {
    /// Creates a XmlBytesReader from a generic BufReader
    pub fn from_reader(reader: B) -> XmlBytesReader<B> {
        XmlBytesReader {
            reader: reader,
            exit: false,
//             opened: Vec::new(),
            tag_state: TagState::Closed,
            expand_empty_elements: true,
            trim_text: false,
            with_check: true,
            buf_position: 0,
            check_comments: false,
        }
    }

//     /// Converts into a `XmlnsReader` iterator
//     pub fn namespaced(self) -> XmlnsReader<B> {
//         XmlnsReader::new(self)
//     }

    /// Change expand_empty_elements default behaviour (true per default)
    ///
    /// When set to true, all `Empty` events are expanded into an `Open` event
    /// followed by a `Close` BytesEvent.
    pub fn expand_empty_elements(mut self, val: bool) -> XmlBytesReader<B> {
        self.expand_empty_elements = val;
        self
    }

    /// Change trim_text default behaviour (false per default)
    ///
    /// When set to true, all Text events are trimed.
    /// If they are empty, no event if pushed
    pub fn trim_text(mut self, val: bool) -> XmlBytesReader<B> {
        self.trim_text = val;
        self
    }

    /// Change default with_check (true per default)
    ///
    /// When set to true, it won't check if End node match last Start node.
    /// If the xml is known to be sane (already processed etc ...)
    /// this saves extra time
    pub fn with_check(mut self, val: bool) -> XmlBytesReader<B> {
        self.with_check = val;
        self
    }

    /// Change default check_comment (false per default)
    ///
    /// When set to true, every Comment event will be checked for not containing `--`
    /// Most of the time we don't want comments at all so we don't really care about
    /// comment correctness, thus default value is false for performance reason
    pub fn check_comments(mut self, val: bool) -> XmlBytesReader<B> {
        self.check_comments = val;
        self
    }

    /// Gets the current BufRead position
    /// Useful when debugging errors
    pub fn buffer_position(&self) -> usize {
        self.buf_position
    }

    /// private function to read until '<' is found
    /// return a `Text` event
    fn read_until_open<'a, 'b>(&'a mut self, buf: &'b mut Vec<u8>) -> Option<ResultPos<BytesEvent<'b>>> {
        self.tag_state = TagState::Opened;
        let buf_start = buf.len();
        match read_until(&mut self.reader, b'<', buf) {
            Ok(0) => None,
            Ok(n) => {
                self.buf_position += n;
                let (start, len) = if self.trim_text {
                    match buf.iter().skip(buf_start).position(|&b| !is_whitespace(b)) {
                        Some(start) => {
                            (start, buf.iter().skip(buf_start)
                             .rposition(|&b| !is_whitespace(b))
                             .unwrap_or(buf.len()))
                        }
                        None => return self.next_event(buf),
                    }
                } else {
                    (buf_start, buf.len())
                };
                Some(Ok(BytesEvent::Text(BytesElement::from_buffer(Cow::Borrowed(buf), start, len, len))))
            }
            Err(e) => Some(self.error(e, 0)),
        }
    }

    /// private function to read until '>' is found
    fn read_until_close<'a, 'b>(&'a mut self, buf: &'b mut Vec<u8>) -> Option<ResultPos<BytesEvent<'b>>> {
        self.tag_state = TagState::Closed;

        // need to read 1 character to decide whether pay special attention to attribute values
        let buf_start = buf.len();
        let start;
        loop {
            // Need to contain the `self.reader.fill_buf()` in a scope lexically separate from the
            // `self.error()` call because both require `&mut self`.
            let start_result = {
                let available = match self.reader.fill_buf() {
                    Ok(n) if n.is_empty() => return None,
                    Ok(n) => Ok(n),
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => Err(e),
                };
                // `available` is a non-empty slice => we only need the first byte to decide
                available.map(|xs| xs[0])
            };

            // throw the error we couldn't throw in the block above because `self` was sill borrowed
            start = match start_result {
                Ok(s) => s,
                Err(e) => return Some(self.error(Error::Io(e), 0))
            };

            // We intentionally don't `consume()` the byte, otherwise we would have to handle things
            // like '<>' here already.
            break;
        }

        if start != b'/' && start != b'!' && start != b'?' {
            match read_elem_until(&mut self.reader, b'>', buf) {
                Ok(0) => None,
                Ok(n) => {
                    self.buf_position += n;
                    // we already *know* that we are in this case
                    Some(self.read_start(&buf[buf_start..]))
                }
                Err(e) => Some(self.error(e, 0)),
            }
        } else {
            match read_until(&mut self.reader, b'>', buf) {
                Ok(0) => None,
                Ok(n) => {
                    self.buf_position += n;
                    match start {
                        b'/' => Some(self.read_end(&buf[buf_start..])),
                        b'!' => Some(self.read_bang(buf_start, buf)),
                        b'?' => Some(self.read_question_mark(&buf[buf_start..])),
                        _ => unreachable!("We checked that `start` must be one of [/!?], \
                                            was {:?} instead.", start),
                    }
                }
                Err(e) => Some(self.error(e, 0)),
            }
        }

    }

    /// reads `BytesElement` starting with a `/`,
    /// if `self.with_check`, checks that element matches last opened element
    /// return `End` event
    fn read_end<'a, 'b>(&'a mut self, buf: &'b[u8]) -> ResultPos<BytesEvent<'b>> {
        let len = buf.len();
//         if self.with_check {
//             let e = match self.opened.pop() {
//                 Some(e) => e,
//                 None => return self.error(
//                     Error::Malformed(format!("Cannot close {:?} element, \
//                                              there is no opened element",
//                                              buf[1..].as_str())), len),
//             };
//             if &buf[1..] != e.name() {
//                 let m = format!("End event {:?} doesn't match last \
//                                 opened element {:?}, opened: {:?}",
//                                 BytesElement::from_buffer(Cow::Borrowed(buf), 1, len, len), e, &self.opened);
//                 return self.error(Error::Malformed(m), len);
//             }
//         }
        Ok(BytesEvent::End(BytesElement::from_buffer(Cow::Borrowed(buf), 1, len, len)))
    }

    /// reads `BytesElement` starting with a `!`,
    /// return `Comment`, `CData` or `DocType` event
    fn read_bang<'a, 'b>(&'a mut self, buf_start: usize, buf: &'b mut Vec<u8>) -> ResultPos<BytesEvent<'b>> {
        let len = buf.len();
        if len >= 3 && &buf[buf_start + 1..buf_start + 3] == b"--" {
            let mut len = buf.len();
            while len < 5 || &buf[len - 2..] != b"--" {
                buf.push(b'>');
                match read_until(&mut self.reader, b'>', buf) {
                    Ok(0) => return self.error(
                        Error::Malformed("Unescaped Comment event".to_string()), len),
                    Ok(n) => self.buf_position += n,
                    Err(e) => return self.error(e, 0),
                }
                len = buf.len();
            }
            if self.check_comments {
                let mut offset = len - 3;
                for w in buf[buf_start + 3..len - 1].windows(2) {
                    if &*w == b"--" {
                        return self.error(
                            Error::Malformed("Unexpected token '--'".to_string()), offset);
                    }
                    offset -= 1;
                }
            }
            Ok(BytesEvent::Comment(BytesElement::from_buffer(Cow::Borrowed(buf), buf_start + 3, len - 2, len - 2)))
        } else if len >= 8 {
            match &buf[buf_start + 1..buf_start + 8] {
                b"[CDATA[" => {
                    let mut len = buf.len();
                    while len < 10 || &buf[len - 2..] != b"]]" {
                        buf.push(b'>');
                        match read_until(&mut self.reader, b'>', buf) {
                            Ok(0) => return self.error(
                                Error::Malformed("Unescaped CDATA event".to_string()), len),
                            Ok(n) => self.buf_position += n,
                            Err(e) => return self.error(e, 0),
                        }
                        len = buf.len();
                    }
                    Ok(BytesEvent::CData(BytesElement::from_buffer(Cow::Borrowed(buf), buf_start + 8, len - 2, len - 2)))
                }
                b"DOCTYPE" => {
                    let mut count = buf.iter().skip(buf_start).filter(|&&b| b == b'<').count();
                    while count > 0 {
                        buf.push(b'>');
                        match read_until(&mut self.reader, b'>', buf) {
                            Ok(0) => return self.error(
                                Error::Malformed("Unescaped DOCTYPE node".to_string()), buf.len()),
                            Ok(n) => {
                                self.buf_position += n;
                                let start = buf.len() - n;
                                count += buf.iter().skip(start).filter(|&&b| b == b'<').count() - 1;
                            }
                            Err(e) => return self.error(e, 0),
                        }
                    }
                    let len = buf.len();
                    Ok(BytesEvent::DocType(BytesElement::from_buffer(Cow::Borrowed(buf), buf_start + 1, len, buf_start + 8)))
                }
                _ => self.error(Error::Malformed("Only Comment, CDATA and DOCTYPE nodes \
                                                 can start with a '!'".to_string()), 0),
            }
        } else {
            self.error(Error::Malformed("Only Comment, CDATA and DOCTYPE nodes can start \
                                        with a '!'".to_string()), buf.len())
        }
    }

    /// reads `BytesElement` starting with a `?`,
    /// return `Decl` or `PI` event
    fn read_question_mark<'a, 'b>(&'a mut self, buf: &'b [u8]) -> ResultPos<BytesEvent<'b>> {
        let len = buf.len();
        if len > 2 && buf[len - 1] == b'?' {
            if len > 5 && &buf[1..4] == b"xml" && is_whitespace(buf[4]) {
                Ok(BytesEvent::Decl(XmlBytesDecl { element: BytesElement::from_buffer(Cow::Borrowed(buf), 1, len - 1, 3) }))
            } else {
                Ok(BytesEvent::PI(BytesElement::from_buffer(Cow::Borrowed(buf), 1, len - 1, 3)))
            }
        } else {
            self.error(Error::Malformed("Unescaped XmlDecl event".to_string()), len)
        }
    }

//     fn close_expanded_empty(&mut self) -> Option<ResultPos<BytesEvent>> {
//         self.tag_state = TagState::Closed;
//         let e = self.opened.pop().unwrap();
//         Some(Ok(BytesEvent::End(e)))
//     }

    /// reads `BytesElement` starting with any character except `/`, `!` or ``?`
    /// return `Start` or `Empty` event
    fn read_start<'a, 'b>(&'a mut self, buf: &'b [u8]) -> ResultPos<BytesEvent<'b>> {
        // TODO: do this directly when reading bufreader ...
        let len = buf.len();
        let name_end = buf.iter().position(|&b| is_whitespace(b)).unwrap_or(len);
        if buf[len - 1] == b'/' {
            let end = if name_end < len { name_end } else { len - 1 };
            let element = BytesElement::from_buffer(Cow::Borrowed(buf), 0, len - 1, end);
//             if self.expand_empty_elements {
//                 self.tag_state = TagState::Empty;
//                 self.opened.push(element.clone());
//                 Ok(BytesEvent::Start(element))
//             } else {
                Ok(BytesEvent::Empty(element))
//             }
        } else {
            let element = BytesElement::from_buffer(Cow::Borrowed(buf), 0, len, name_end);
//             if self.with_check { self.opened.push(element.clone()); }
            Ok(BytesEvent::Start(element))
        }
    }

    /// returns `Err(Error, buf_position - offset)`
    /// sets `self.exit = true` so next call will terminate the iterator
    fn error(&mut self, e: Error, offset: usize) -> ResultPos<BytesEvent<'static>> {
        self.exit = true;
        Err((e, self.buf_position - offset))
    }

    /// reads the next `BytesEvent`
    pub fn next_event<'a, 'b>(&'a mut self, buf: &'b mut Vec<u8>) -> Option<ResultPos<BytesEvent<'b>>> {
        if self.exit {
            return None;
        }
        match self.tag_state {
            TagState::Opened => self.read_until_close(buf),
            TagState::Closed => self.read_until_open(buf),
//             TagState::Empty => self.close_expanded_empty(buf),
        }
    }
}

impl<B: BufRead> XmlBytesReader<B> {
//     /// Reads until end element is found
//     ///
//     /// Manages nested cases where parent and child elements have the same name
//     pub fn read_to_end<K: AsRef<[u8]>>(&mut self, end: K) -> ResultPos<()> {
//         let mut depth = 0;
//         let end = end.as_ref();
//         loop {
//             match self.next() {
//                 Some(Ok(BytesEvent::End(ref e))) if e.name() == end => {
//                     if depth == 0 {
//                         return Ok(());
//                     }
//                     depth -= 1;
//                 }
//                 Some(Ok(BytesEvent::Start(ref e))) if e.name() == end => depth += 1,
//                 Some(Err(e)) => return Err(e),
//                 None => {
//                     warn!("EOF instead of {:?}", from_utf8(end));
//                     return Err((Error::Unexpected(format!(
//                                     "Reached EOF, expecting {:?} end tag",
//                                     from_utf8(end))),
//                                 self.buf_position));
//                 }
//                 _ => (),
//             }
//         }
//     }
// 
//     /// Reads next event, if `BytesEvent::Text` or `BytesEvent::End`,
//     /// then returns a `String`, else returns an error
//     pub fn read_text<K: AsRef<[u8]>>(&mut self, end: K) -> ResultPos<String> {
//         match self.next() {
//             Some(Ok(BytesEvent::Text(e))) => {
//                 self.read_to_end(end)
//                     .and_then(|_| e.into_string().map_err(|e| (e, self.buf_position)))
//             }
//             Some(Ok(BytesEvent::End(ref e))) if e.name() == end.as_ref() => {
//                 Ok("".to_string())
//             },
//             Some(Err(e)) => Err(e),
//             None => {
//                 Err((Error::Unexpected("Reached EOF while reading text".to_string()),
//                      self.buf_position))
//             }
//             _ => {
//                 Err((Error::Unexpected("Cannot read text, expecting BytesEvent::Text".to_string()),
//                      self.buf_position))
//             }
//         }
//     }
// 
//     /// Reads next event, if `BytesEvent::Text` or `BytesEvent::End`,
//     /// then returns an unescaped `String`, else returns an error
//     ///
//     /// # Examples
//     /// 
//     /// ```
//     /// use quick_xml::{XmlBytesReader, BytesEvent};
//     ///
//     /// let mut xml = XmlBytesReader::from_reader(b"<a>&lt;b&gt;</a>" as &[u8]).trim_text(true);
//     /// match xml.next() {
//     ///     Some(Ok(BytesEvent::Start(ref e))) => {
//     ///         assert_eq!(&xml.read_text_unescaped(e.name()).unwrap(), "<b>");
//     ///     },
//     ///     e => panic!("Expecting Start(a), found {:?}", e),
//     /// }
//     /// ```
//     pub fn read_text_unescaped<K: AsRef<[u8]>>(&mut self, end: K) -> ResultPos<String> {
//         match self.next() {
//             Some(Ok(BytesEvent::Text(e))) => {
//                 self.read_to_end(end)
//                     .and_then(|_| e.unescaped_content())
//                     .and_then(|c| c.as_str()
//                               .map_err(|e| (e, self.buf_position))
//                               .map(|s| s.to_string()))
//             }
//             Some(Ok(BytesEvent::End(ref e))) if e.name() == end.as_ref() => {
//                 Ok("".to_string())
//             },
//             Some(Err(e)) => Err(e),
//             None => {
//                 Err((Error::Unexpected("Reached EOF while reading text".to_string()),
//                      self.buf_position))
//             }
//             _ => {
//                 Err((Error::Unexpected("Cannot read text, expecting BytesEvent::Text".to_string()),
//                      self.buf_position))
//             }
//         }
//     }
}

impl XmlBytesReader<BufReader<File>> {
    /// Creates a xml reader from a file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<XmlBytesReader<BufReader<File>>> {
        let reader = BufReader::new(try!(File::open(path)));
        Ok(XmlBytesReader::from_reader(reader))
    }
}

/// `read_until` slightly modified from rust std library
///
/// only change is that we do not write the matching character
#[inline]
fn read_until<R: BufRead>(r: &mut R, byte: u8, buf: &mut Vec<u8>) -> Result<usize> {
    let mut read = 0;
    let mut done = false;
    while !done {
        let used = {
            let available = match r.fill_buf() {
                Ok(n) if n.is_empty() => return Ok(read),
                Ok(n) => n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(Error::Io(e)),
            };

            let mut bytes = available.iter().enumerate();

            let used: usize;
            loop {
                match bytes.next() {
                    Some((i, &b)) => {
                        if b == byte {
                            buf.extend_from_slice(&available[..i]);
                            done = true;
                            used = i + 1;
                            break;
                        }
                    }
                    None => {
                        buf.extend_from_slice(available);
                        used = available.len();
                        break;
                    }
                }
            }
            used
        };
        r.consume(used);
        read += used;
    }
    Ok(read)
}

/// Derived from `read_until`, but modified to handle XML attributes using a minimal state machine.
/// [W3C Extensible Markup Language (XML) 1.1 (2006)](https://www.w3.org/TR/xml11)
///
/// Attribute values are defined as follows:
/// ```plain
/// AttValue := '"' (([^<&"]) | Reference)* '"'
///           | "'" (([^<&']) | Reference)* "'"
/// ```
/// (`Reference` is something like `&quot;`, but we don't care about escaped characters at this
/// level)
fn read_elem_until<R: BufRead>(r: &mut R, end_byte: u8, buf: &mut Vec<u8>)
                          -> Result<usize>
{
    #[derive(Debug,Clone,Copy,PartialEq,Eq)]
    enum ElemReadState {
        /// The initial state (inside element, but outside of attribute value)
        Elem,
        /// Inside a single-quoted attribute value
        SingleQ,
        /// Inside a double-quoted attribute value
        DoubleQ
    }
    let mut state = ElemReadState::Elem;
    let mut read = 0;
    let mut done = false;
    while !done {
        let used = {
            let available = match r.fill_buf() {
                Ok(n) if n.is_empty() => return Ok(read),
                Ok(n) => n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(Error::Io(e)),
            };

            let mut bytes = available.iter().enumerate();

            let used: usize;
            loop {
                match bytes.next() {
                    Some((i, &b)) => {
                        state = match (state, b) {
                            (ElemReadState::Elem, b) if b == end_byte => {
                                // only allowed to match `end_byte` while we are in state `Elem`
                                buf.extend_from_slice(&available[..i]);
                                done = true;
                                used = i + 1;
                                break;
                            },
                            (ElemReadState::Elem,  b'\'') => ElemReadState::SingleQ,
                            (ElemReadState::Elem, b'\"') => ElemReadState::DoubleQ,

                            // the only end_byte that gets us out of state 'SingleQ' is a single quote
                            (ElemReadState::SingleQ, b'\'') => ElemReadState::Elem,

                            // the only end_byte that gets us out of state 'DoubleQ' is a double quote
                            (ElemReadState::DoubleQ, b'\"') => ElemReadState::Elem,

                            // all other bytes: no state change
                            _ => state,
                        };
                    }
                    None => {
                        buf.extend_from_slice(available);
                        used = available.len();
                        break;
                    }
                }
            }
            used
        };
        r.consume(used);
        read += used;
    }
    Ok(read)
}

#[inline]
fn is_whitespace(b: u8) -> bool {
    match b {
        b' ' | b'\r' | b'\n' | b'\t' => true,
        _ => false,
    }
}

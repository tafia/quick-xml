//! A module to handle `BytesReader`

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::borrow::Cow;

use error::{Error, Result, ResultPos};
use escape::unescape;
use reader::attributes::{Attributes, UnescapedAttributes};
use AsStr;

#[derive(Clone)]
enum TagState {
    Opened,
    Closed,
    Empty,
}

/// A struct to manage `BytesEvent::Start` events
///
/// Provides in particular an iterator over attributes
#[derive(Clone, Debug)]
pub struct BytesStart<'a> {
    /// content of the element, before any utf8 conversion
    buf: Cow<'a, [u8]>,
    /// end of the element name, the name starts at that the start of `buf`
    name_len: usize
}

impl<'a> BytesStart<'a> {

    /// Creates a new `BytesStart` from the given name.
    #[inline]
    pub fn borrowed(content: &'a[u8], name_len: usize) -> BytesStart<'a> {
        BytesStart {
            buf: Cow::Borrowed(content),
            name_len: name_len,
        }
    }

    /// Creates a new `BytesStart` from the given name. Owns its content
    #[inline]
    pub fn owned(content: Vec<u8>, name_len: usize) -> BytesStart<'static> {
        BytesStart {
            buf: Cow::Owned(content),
            name_len: name_len,
        }
    }

    /// Converts the event into an Owned event
    pub fn into_owned(self) -> BytesStart<'static> {
        BytesStart {
            buf: Cow::Owned(self.buf.into_owned()),
            name_len: self.name_len,
        }
    }

    /// Consumes self and adds attributes to this element from an iterator
    /// over (key, value) tuples.
    /// Key and value can be anything that implements the AsRef<[u8]> trait,
    /// like byte slices and strings.
    pub fn with_attributes<K, V, I>(&mut self, attributes: I) -> &mut Self
        where K: AsRef<[u8]>,
              V: AsRef<[u8]>,
              I: IntoIterator<Item = (K, V)>
    {
        self.extend_attributes(attributes);
        self
    }

    /// name as &[u8] (without eventual attributes)
    pub fn name(&self) -> &[u8] {
        &self.buf[..self.name_len]
    }

    /// whole content as &[u8] (including eventual attributes)
    pub fn content(&self) -> &[u8] {
        &*self.buf
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
        Attributes::new(self.content(), self.name_len)
    }

    /// gets attributes iterator whose attribute values are unescaped ('&...;' replaced
    /// by their corresponding character)
    pub fn unescaped_attributes(&self) -> UnescapedAttributes {
        self.attributes().unescaped()
    }

    /// extend the attributes of this element from an iterator over (key, value) tuples.
    /// Key and value can be anything that implements the AsRef<[u8]> trait,
    /// like byte slices and strings.
    pub fn extend_attributes<K, V, I>(&mut self, attributes: I) -> &mut BytesStart<'a>
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
    }
}

/// Wrapper around `BytesElement` to parse/write `XmlDecl`
///
/// Postpone element parsing only when needed.
///
/// [W3C XML 1.1 Prolog and Document Type Delcaration](http://w3.org/TR/xml11/#sec-prolog-dtd)
#[derive(Clone, Debug)]
pub struct BytesDecl<'a> {
    element: BytesStart<'a>,
}

impl<'a> BytesDecl<'a> {

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
    pub fn new(version: &[u8], encoding: Option<&[u8]>, standalone: Option<&[u8]>) -> BytesDecl<'static> {
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

        BytesDecl { element: BytesStart::owned(buf, 3) }
    }
}

/// A struct to manage `BytesEvent::End` events
#[derive(Clone, Debug)]
pub struct BytesEnd<'a> {
    name: Cow<'a, [u8]>
}

impl<'a> BytesEnd<'a> {

    /// Creates a new `BytesEnd` borrowing a slice
    #[inline]
    pub fn borrowed(name: &'a [u8]) -> BytesEnd<'a> {
        BytesEnd { name: Cow::Borrowed(name) }
    }

    /// Creates a new `BytesEnd` owning its name
    #[inline]
    pub fn owned(name: Vec<u8>) -> BytesEnd<'static> {
        BytesEnd { name: Cow::Owned(name) }
    }

    /// Gets `BytesEnd` event name
    #[inline]
    pub fn name(&self) -> &[u8] {
        &*self.name
    }
}

/// A struct to manage `BytesEvent::End` events
#[derive(Clone, Debug)]
pub struct BytesText<'a> {
    content: Cow<'a, [u8]>
}

impl<'a> BytesText<'a> {

    /// Creates a new `BytesEnd` borrowing a slice
    #[inline]
    pub fn borrowed(content: &'a [u8]) -> BytesText<'a> {
        BytesText { content: Cow::Borrowed(content) }
    }

    /// Creates a new `BytesEnd` owning its name
    #[inline]
    pub fn owned(content: Vec<u8>) -> BytesText<'static> {
        BytesText { content: Cow::Owned(content) }
    }

    /// Gets `BytesEnd` event name
    #[inline]
    pub fn content(&self) -> &[u8] {
        &*self.content
    }

    /// gets escaped content
    ///
    /// Searches for '&' into content and try to escape the coded character if possible
    /// returns Malformed error with index within element if '&' is not followed by ';'
    pub fn unescaped_content(&self) -> ResultPos<Cow<[u8]>> {
        unescape(&*self.content)
    }

    /// consumes entire self (including eventual attributes!) and returns `String`
    ///
    /// useful when we need to get Text event value (which don't have attributes)
    pub fn into_string(self) -> Result<String> {
        ::std::string::String::from_utf8(self.content.into_owned())
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
}

/// BytesEvent to interprete node as they are parsed
#[derive(Clone, Debug)]
pub enum BytesEvent<'a> {
    /// Start tag (with attributes) <...>
    Start(BytesStart<'a>),
    /// End tag </...>
    End(BytesEnd<'a>),
    /// Empty element tag (with attributes) <.../>
    Empty(BytesStart<'a>),
    /// Data between Start and End element
    Text(BytesText<'a>),
    /// Comment <!-- ... -->
    Comment(BytesText<'a>),
    /// CData <![CDATA[...]]>
    CData(BytesText<'a>),
    /// Xml declaration <?xml ...?>
    Decl(BytesDecl<'a>),
    /// Processing instruction <?...?>
    PI(BytesText<'a>),
    /// Doctype <!DOCTYPE...>
    DocType(BytesText<'a>),
    /// Eof of file event
    Eof,
}

/// A low level Xml bytes reader
///
/// Consumes a `BufRead` and streams xml `BytesEvent`s
///
/// ```
/// use quick_xml::reader::bytes::{BytesReader, BytesEvent};
///
/// let xml = r#"<tag1 att1 = "test">
///                 <tag2><!--Test comment-->Test</tag2>
///                 <tag2>Test 2</tag2>
///             </tag1>"#;
/// let mut reader = BytesReader::from(xml);
/// reader.trim_text(true);
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
pub struct BytesReader<B: BufRead> {

    /// reader
    reader: B,
    /// if was error, exit next
    exit: bool,
    /// current buffer position, useful for debuging errors
    buf_position: usize,
    /// current state Open/Close
    tag_state: TagState,
    /// expand empty element into an opening and closing element
    expand_empty_elements: bool,
    /// trims Text events, skip the element if text is empty
    trim_text: bool,
    /// check if End nodes match last Start node
    check_end_names: bool,
    /// check if comments contains `--` (false per default)
    check_comments: bool,
    /// all currently Started elements which didn't have a matching
    /// End element yet
    opened_buffer: Vec<u8>,
    /// opened name start indexes
    opened_starts: Vec<usize>,

    /// a buffer of namespace ranges
    ns_slices: Vec<Namespace>,
    /// a buffer of existing namespaces
    ns_buffer: Vec<u8>,
    /// The number of open tags at the moment. We need to keep track of this to know which namespace
    /// declarations to remove when we encounter an `End` event.
    ns_nesting_level: i32,
    /// For `Empty` events keep the 'scope' of the element on the stack artificially. That way, the
    /// consumer has a chance to use `resolve` in the context of the empty element. We perform the
    /// pop as the first operation in the next `next()` call.
    ns_pending_pop: bool,
}

impl<'a> ::std::convert::From<&'a str> for BytesReader<&'a [u8]> {
    fn from(reader: &'a str) -> BytesReader<&'a [u8]> {
        BytesReader::from_reader(reader.as_bytes())
    }
}

impl<B: BufRead> BytesReader<B> {
    /// Creates a BytesReader from a generic BufReader
    pub fn from_reader(reader: B) -> BytesReader<B> {
        BytesReader {
            reader: reader,
            exit: false,
            opened_buffer: Vec::new(),
            opened_starts: Vec::new(),
            tag_state: TagState::Closed,
            expand_empty_elements: false,
            trim_text: false,
            check_end_names: true,
            buf_position: 0,
            check_comments: false,

            ns_slices: Vec::new(),
            ns_buffer: Vec::new(),
            ns_nesting_level: 0,
            ns_pending_pop: false,

        }
    }

    /// Change expand_empty_elements default behaviour (true per default)
    ///
    /// When set to true, all `Empty` events are expanded into an `Open` event
    /// followed by a `Close` BytesEvent.
    pub fn expand_empty_elements(&mut self, val: bool) -> &mut BytesReader<B> {
        self.expand_empty_elements = val;
        self
    }

    /// Change trim_text default behaviour (false per default)
    ///
    /// When set to true, all Text events are trimed.
    /// If they are empty, no event if pushed
    pub fn trim_text(&mut self, val: bool) -> &mut BytesReader<B> {
        self.trim_text = val;
        self
    }

    /// Change default check_end_names (true per default)
    ///
    /// When set to true, it won't check if End node match last Start node.
    /// If the xml is known to be sane (already processed etc ...)
    /// this saves extra time
    pub fn check_end_names(&mut self, val: bool) -> &mut BytesReader<B> {
        self.check_end_names = val;
        self
    }

    /// Change default check_comment (false per default)
    ///
    /// When set to true, every Comment event will be checked for not containing `--`
    /// Most of the time we don't want comments at all so we don't really care about
    /// comment correctness, thus default value is false for performance reason
    pub fn check_comments(&mut self, val: bool) -> &mut BytesReader<B> {
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
    fn read_until_open<'a, 'b>(&'a mut self, buf: &'b mut Vec<u8>) -> ResultPos<BytesEvent<'b>> {
        self.tag_state = TagState::Opened;
        let buf_start = buf.len();
        match read_until(&mut self.reader, b'<', buf) {
            Ok(0) => Ok(BytesEvent::Eof),
            Ok(n) => {
                self.buf_position += n;
                let (start, len) = if self.trim_text {
                    match buf.iter().skip(buf_start).position(|&b| !is_whitespace(b)) {
                        Some(start) => {
                            (start, buf.iter()
                             .rposition(|&b| !is_whitespace(b)).map(|p| p + 1)
                             .unwrap_or(buf.len()))
                        }
                        None => return self.read_event(buf),
                    }
                } else {
                    (buf_start, buf.len())
                };
                Ok(BytesEvent::Text(BytesText::borrowed(&buf[start..len])))
            }
            Err(e) => self.error(e, 0),
        }
    }

    /// private function to read until '>' is found
    fn read_until_close<'a, 'b>(&'a mut self, buf: &'b mut Vec<u8>) -> ResultPos<BytesEvent<'b>> {
        self.tag_state = TagState::Closed;

        // need to read 1 character to decide whether pay special attention to attribute values
        let buf_start = buf.len();
        let start;
        loop {
            // Need to contain the `self.reader.fill_buf()` in a scope lexically separate from the
            // `self.error()` call because both require `&mut self`.
            let start_result = {
                let available = match self.reader.fill_buf() {
                    Ok(n) if n.is_empty() => return Ok(BytesEvent::Eof),
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
                Err(e) => return self.error(Error::Io(e), 0)
            };

            // We intentionally don't `consume()` the byte, otherwise we would have to handle things
            // like '<>' here already.
            break;
        }

        if start != b'/' && start != b'!' && start != b'?' {
            match read_elem_until(&mut self.reader, b'>', buf) {
                Ok(0) => Ok(BytesEvent::Eof),
                Ok(n) => {
                    self.buf_position += n;
                    // we already *know* that we are in this case
                    self.read_start(&buf[buf_start..])
                }
                Err(e) => self.error(e, 0),
            }
        } else {
            match read_until(&mut self.reader, b'>', buf) {
                Ok(0) => Ok(BytesEvent::Eof),
                Ok(n) => {
                    self.buf_position += n;
                    match start {
                        b'/' => self.read_end(&buf[buf_start..]),
                        b'!' => self.read_bang(buf_start, buf),
                        b'?' => self.read_question_mark(&buf[buf_start..]),
                        _ => unreachable!("We checked that `start` must be one of [/!?], \
                                            was {:?} instead.", start),
                    }
                }
                Err(e) => self.error(e, 0),
            }
        }

    }

    /// reads `BytesElement` starting with a `/`,
    /// if `self.check_end_names`, checks that element matches last opened element
    /// return `End` event
    fn read_end<'a, 'b>(&'a mut self, buf: &'b[u8]) -> ResultPos<BytesEvent<'b>> {
        let len = buf.len();
        if self.check_end_names {
            match self.opened_starts.pop() {
                Some(start) => {
                    if buf[1..] != self.opened_buffer[start..] {
                        let m = format!("End event name '{:?}' doesn't match last opened element name '{:?}'",
                                        &buf[1..].as_str(), self.opened_buffer[start..].as_str());
                        return self.error(Error::Malformed(m), len);
                    }
                    self.opened_buffer.truncate(start);
                },
                None => return self.error(
                    Error::Malformed(format!("Cannot close {:?} element, \
                                             there is no opened element",
                                             buf[1..].as_str())), len),
            }
        }
        Ok(BytesEvent::End(BytesEnd::borrowed(&buf[1..])))
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
            Ok(BytesEvent::Comment(BytesText::borrowed(&buf[buf_start + 3..len - 2])))
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
                    Ok(BytesEvent::CData(BytesText::borrowed(&buf[buf_start + 8..len - 2])))
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
                    Ok(BytesEvent::DocType(BytesText::borrowed(&buf[buf_start + 8..len])))
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
                Ok(BytesEvent::Decl(BytesDecl { element: BytesStart::borrowed(&buf[1..len - 1], 3) }))
            } else {
                Ok(BytesEvent::PI(BytesText::borrowed(&buf[1..len - 1])))
            }
        } else {
            self.error(Error::Malformed("Unescaped XmlDecl event".to_string()), len)
        }
    }

    fn close_expanded_empty(&mut self) -> ResultPos<BytesEvent<'static>> {
        self.tag_state = TagState::Closed;
        let name = self.opened_buffer.split_off(self.opened_starts.pop().unwrap());
        Ok(BytesEvent::End(BytesEnd::owned(name)))
    }

    /// reads `BytesElement` starting with any character except `/`, `!` or ``?`
    /// return `Start` or `Empty` event
    fn read_start<'a, 'b>(&'a mut self, buf: &'b [u8]) -> ResultPos<BytesEvent<'b>> {
        // TODO: do this directly when reading bufreader ...
        let len = buf.len();
        let name_end = buf.iter().position(|&b| is_whitespace(b)).unwrap_or(len);
        if buf[len - 1] == b'/' {
            let end = if name_end < len { name_end } else { len - 1 };
            if self.expand_empty_elements {
                self.tag_state = TagState::Empty;
                self.opened_starts.push(self.opened_buffer.len());
                self.opened_buffer.extend(&buf[..end]);
                Ok(BytesEvent::Start(BytesStart::borrowed(&buf[..len - 1], end)))
            } else {
                Ok(BytesEvent::Empty(BytesStart::borrowed(&buf[..len - 1], end)))
            }
        } else {
            if self.check_end_names { 
                self.opened_starts.push(self.opened_buffer.len());
                self.opened_buffer.extend(&buf[..name_end]);
            }
            Ok(BytesEvent::Start(BytesStart::borrowed(&buf, name_end)))
        }
    }

    /// returns `Err(Error, buf_position - offset)`
    /// sets `self.exit = true` so next call will terminate the iterator
    fn error(&mut self, e: Error, offset: usize) -> ResultPos<BytesEvent<'static>> {
        self.exit = true;
        Err((e, self.buf_position - offset))
    }

    /// reads the next `BytesEvent`
    pub fn read_event<'a, 'b>(&'a mut self, buf: &'b mut Vec<u8>) -> ResultPos<BytesEvent<'b>> {
        if self.exit {
            return Ok(BytesEvent::Eof);
        }
        match self.tag_state {
            TagState::Opened => self.read_until_close(buf),
            TagState::Closed => self.read_until_open(buf),
            TagState::Empty => self.close_expanded_empty(),
        }
    }

    /// reads the next `BytesEvent` and converts `BytesEvent::Eof` by `None` else `Some(event)`
    pub fn next_event<'a, 'b>(&'a mut self, buf: &'b mut Vec<u8>) -> Option<ResultPos<BytesEvent<'b>>> {
        match self.read_event(buf) {
            Ok(BytesEvent::Eof) => None,
            Ok(e) => Some(Ok(e)),
            Err(e) => Some(Err(e)),
        }
    }

    /// Resolves a potentially qualified **attribute name** into (namespace name, local name).
    ///
    /// *Qualified* attribute names have the form `prefix:local-name` where the`prefix` is defined
    /// on any containing XML element via `xmlns:prefix="the:namespace:uri"`. The namespace prefix
    /// can be defined on the same element as the attribute in question.
    ///
    /// *Unqualified* attribute names do *not* inherit the current *default namespace*.
    pub fn resolve_namespace<'a, 'b>(&'a self, qname: &'b [u8]) 
        -> (Option<&'a [u8]>, &'b [u8]) 
    {
        // Unqualified attributes don't inherit the default namespace. We don't need to search the
        // namespace declaration stack for those.
        if !qname.contains(&b':') {
            return (None, qname)
        }

        match self.ns_slices.iter().rev().find(|ref n| n.matches_qualified(&self.ns_buffer, qname)) {
            // Found closest matching namespace declaration `n`. The `unwrap` is fine because
            // `is_match_attr` doesn't return default namespace declarations.
            Some(&Namespace { ref prefix, value: Some(ref value), .. }) =>
                (Some(value.get(&self.ns_buffer)),
                 &qname[prefix.as_ref().unwrap().len() + 1..]),
            Some(&Namespace { ref prefix, value: None, .. }) =>
                (None, &qname[prefix.as_ref().unwrap().len() + 1..]),
            None => (None, qname),
        }
    }

    fn find_namespace_value(&self, element_name: &[u8]) -> Option<NamespaceSlice> {
        // We pulled the qualified-vs-unqualified check out here so that it doesn't happen for each
        // namespace we are comparing against.
        if element_name.contains(&b':') {
            // qualified name
            self.ns_slices
                .iter()
                .rev() // iterate in reverse order to find the most recent one
                .find(|ref n| n.matches_qualified(&self.ns_buffer, element_name))
                .and_then(|ref n| n.value.clone())
        } else {
            // unqualified name (inherits current default namespace)
            self.ns_slices
                .iter()
                .rev() // iterate in reverse order to find the most recent one
                .find(|ref n| n.matches_unqualified_elem())
                .and_then(|ref n| n.value.clone())
        }
    }

    fn pop_empty_namespaces(&mut self) {
        self.ns_nesting_level -= 1;
        let current_level = self.ns_nesting_level;
        // from the back (most deeply nested scope), look for the first scope that is still valid
        match self.ns_slices.iter().rposition(|n| n.level <= current_level) {
            // none of the namespaces are valid, remove all of them
            None => {
                self.ns_buffer.clear();
                self.ns_slices.clear();
            }
            // drop all namespaces past the last valid namespace
            Some(last_valid_pos) => {
                self.ns_buffer.truncate(self.ns_slices[last_valid_pos].ns_buffer_end);
                self.ns_slices.truncate(last_valid_pos + 1);
            }
        }
    }

    /// add a slice to ns_buffer and return a NamespaceSlice
    fn add_namespace_slice(&mut self, name: &[u8]) -> NamespaceSlice {
        let start = self.ns_buffer.len();
        self.ns_buffer.extend_from_slice(name);
        NamespaceSlice {
            start: start,
            end: self.ns_buffer.len()
        }
    }

    fn push_new_namespaces(&mut self, e: &BytesStart) {
        // adds new namespaces for attributes starting with 'xmlns:' and for the 'xmlns'
        // (default namespace) attribute.
        for a in e.attributes().with_checks(false) {
            if let Ok((k, v)) = a {
                // Check for 'xmlns:any-prefix' and 'xmlns' at the same time:
                if k.len() >= 5 && &k[..5] == b"xmlns" && (k.len() == 5 || k[5] == b':') {
                    // We use an None prefix as the 'name' for the default namespace.
                    // That saves an allocation compared to an empty namespace name.
                    let prefix = if k.len() == 5 { None } else { Some(self.add_namespace_slice(&k[6..])) };
                    let ns_value = if v.len() == 0 { None } else { Some(self.add_namespace_slice(v)) };
                    self.ns_slices.push(Namespace {
                        prefix: prefix,
                        value: ns_value,
                        level: self.ns_nesting_level,
                        ns_buffer_end: self.ns_buffer.len(),
                    });
                }
            } else {
                break;
            }
        }
    }

    /// Reads the next event and resolve its namespace
    pub fn read_namespaced_event<'a, 'b>(&'a mut self, buf: &'b mut Vec<u8>) 
        -> ResultPos<(Option<&'a[u8]>, BytesEvent<'b>)>
    {
        if self.ns_pending_pop {
            self.ns_pending_pop = false;
            self.pop_empty_namespaces();
        }
        match self.read_event(buf) {
            Ok(BytesEvent::Eof) => Ok((None, BytesEvent::Eof)),
            Ok(BytesEvent::Start(e)) => {
                self.ns_nesting_level += 1;
                self.push_new_namespaces(&e);
                match self.find_namespace_value(e.name()) {
                    Some(ns) => Ok((Some(ns.get(&self.ns_buffer)), BytesEvent::Start(e))),
                    None => Ok((None, BytesEvent::Start(e))),
                }
            }
            Ok(BytesEvent::Empty(e)) => {
                // For empty elements we need to 'artificially' keep the namespace scope on the
                // stack until the next `next()` call occurs.
                // Otherwise the caller has no chance to use `resolve` in the context of the
                // namespace declarations that are 'in scope' for the empty element alone.
                // Ex: <img rdf:nodeID="abc" xmlns:rdf="urn:the-rdf-uri" />
                self.ns_nesting_level += 1;
                self.push_new_namespaces(&e);
                // notify next `read_namespaced_event()` invocation that it needs to pop this
                // namespace scope
                self.ns_pending_pop = true;
                match self.find_namespace_value(e.name()) {
                    Some(ns) => Ok((Some(ns.get(&self.ns_buffer)), BytesEvent::Empty(e))),
                    None => Ok((None, BytesEvent::Empty(e))),
                }
            }
            Ok(BytesEvent::End(e)) => {
                let ns = self.find_namespace_value(e.name());
                // notify next `read_namespaced_event()` invocation that it needs to pop this
                // namespace scope
                self.ns_pending_pop = true;
                match ns {
                    Some(ns) => Ok((Some(ns.get(&self.ns_buffer)), BytesEvent::End(e))),
                    None => Ok((None, BytesEvent::End(e))),
                }
            }
            Ok(e) => Ok((None, e)),
            Err(e) => Err(e),
        }
    }

}

impl<B: BufRead> BytesReader<B> {

    /// Reads until end element is found
    ///
    /// Manages nested cases where parent and child elements have the same name
    pub fn read_to_end<K: AsRef<[u8]>>(&mut self, end: K, buf: &mut Vec<u8>) -> ResultPos<()> {
        let mut depth = 0;
        let end = end.as_ref();
        loop {
            match self.read_event(buf) {
                Ok(BytesEvent::End(ref e)) if e.name() == end => {
                    if depth == 0 { return Ok(()); }
                    depth -= 1;
                }
                Ok(BytesEvent::Start(ref e)) if e.name() == end => depth += 1,
                Err(e) => return Err(e),
                Ok(BytesEvent::Eof) => {
                    warn!("EOF instead of {:?}", end.as_str());
                    return Err((Error::Unexpected(format!("Reached EOF, expecting {:?} end tag", 
                                                          end.as_str())), self.buf_position));
                }
                _ => (),
            }
            buf.clear();
        }
    }

    /// Reads next event, if `BytesEvent::Text` or `BytesEvent::End`,
    /// then returns a `String`, else returns an error
    pub fn read_text<K: AsRef<[u8]>>(&mut self, end: K, buf: &mut Vec<u8>) -> ResultPos<String> {
        let (read_end, s) = match self.read_event(buf) {
            Ok(BytesEvent::Text(e)) => {
                let s = e.into_string().map_err(|e| (e, self.buf_position))?;
                (true, s)
            }
            Ok(BytesEvent::End(ref e)) if e.name() == end.as_ref() => {
                (false, "".to_string())
            },
            Err(e) => return Err(e),
            Ok(BytesEvent::Eof) => {
                return Err((Error::Unexpected("Reached EOF while reading text".to_string()),
                     self.buf_position))
            }
            _ => {
                return Err((Error::Unexpected("Cannot read text, expecting BytesEvent::Text".to_string()),
                     self.buf_position))
            }
        };
        if read_end { self.read_to_end(end, buf)?; }
        Ok(s)
    }

    /// Reads next event, if `BytesEvent::Text` or `BytesEvent::End`,
    /// then returns an unescaped `String`, else returns an error
    ///
    /// # Examples
    /// 
    /// ```
    /// use quick_xml::reader::bytes::{BytesReader, BytesEvent};
    ///
    /// let mut xml = BytesReader::from_reader(b"<a>&lt;b&gt;</a>" as &[u8]);
    /// 
    /// xml.trim_text(true);
    /// let mut buf = Vec::new();
    /// 
    /// match xml.next_event(&mut buf) {
    ///     Some(Ok(BytesEvent::Start(ref e))) => {
    ///         assert_eq!(&xml.read_text_unescaped(e.name(), &mut Vec::new()).unwrap(), "<b>");
    ///     },
    ///     e => panic!("Expecting Start(a), found {:?}", e),
    /// }
    /// ```
    pub fn read_text_unescaped<K: AsRef<[u8]>>(&mut self, end: K, buf: &mut Vec<u8>) -> ResultPos<String> {
        let (read_end, s) = match self.read_event(buf) {
            Ok(BytesEvent::Text(e)) => {
                assert_eq!(b"&lt;b&gt;", e.content());
                (true, e.unescaped_content().and_then(|c| c.as_str()
                                                      .map_err(|e| (e, self.buf_position))
                                                      .map(|s| s.to_string())))
            }
            Ok(BytesEvent::End(ref e)) if e.name() == end.as_ref() => {
                (false, Ok("".to_string()))
            },
            Err(e) => return Err(e),
            Ok(BytesEvent::Eof) => {
                return Err((Error::Unexpected("Reached EOF while reading text".to_string()),
                     self.buf_position))
            }
            _ => {
                return Err((Error::Unexpected("Cannot read text, expecting BytesEvent::Text".to_string()),
                     self.buf_position))
            }
        };
        if read_end { self.read_to_end(end, buf)? }
        s
    }
}

impl BytesReader<BufReader<File>> {
    /// Creates a xml reader from a file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<BytesReader<BufReader<File>>> {
        let reader = BufReader::new(try!(File::open(path)));
        Ok(BytesReader::from_reader(reader))
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


/// Start and end index into a vector with prefixes and namespaces
#[derive(Clone, Debug)]
struct NamespaceSlice {
    start: usize,
    end: usize,
}

impl NamespaceSlice {
    /// The length of this slice.
    fn len(&self) -> usize {
        self.end - self.start
    }
    /// Get the concrete u8 slice that this object points to.
    fn get<'a>(&self, buffer: &'a [u8]) -> &'a [u8] {
        &buffer[self.start..self.end]
    }
}

/// A namespace declaration. Can either bind a namespace to a prefix or define the current default
/// namespace.
#[derive(Clone, Debug)]
struct Namespace {
    /// * `Some(prefix)` binds this namespace to `prefix`.
    /// * `None` defines the current default namespace.
    prefix: Option<NamespaceSlice>,
    /// The namespace name (the URI) of this namespace declaration.
    ///
    /// The XML standard specifies that an empty namespace value 'removes' a namespace declaration
    /// for the extent of its scope. For prefix declarations that's not very interesting, but it is
    /// vital for default namespace declarations. With `xmlns=""` you can revert back to the default
    /// behaviour of leaving unqualified element names unqualified.
    value: Option<NamespaceSlice>,
    /// Level of nesting at which this namespace was declared. The declaring element is included,
    /// i.e., a declaration on the document root has `level = 1`.
    /// This is used to pop the namespace when the element gets closed.
    level: i32,
    /// Position at which this namespace ends in the namespace buffer.
    ns_buffer_end: usize,
}

impl Namespace {
    /// Check whether this namespace declaration matches the **qualified element name** name.
    /// Does not take default namespaces into account. The `matches_unqualified_elem` method is
    /// responsible for unqualified element names.
    ///
    /// [W3C Namespaces in XML 1.1 (2006)](http://w3.org/TR/xml-names11/#scoping-defaulting)
    #[inline]
    fn matches_qualified(&self, ns_buffer: &[u8], name: &[u8]) -> bool {
        if let Some(ref prefix) = self.prefix {
            let len = prefix.len();
            name.len() > len && name[len] == b':' && &name[..len] == prefix.get(&ns_buffer)
        } else {
            false
        }
    }

    /// A namespace declaration matches unqualified elements if and only if it is a default
    /// namespace declaration (no prefix).
    ///
    /// [W3C Namespaces in XML 1.1 (2006)](http://w3.org/TR/xml-names11/#scoping-defaulting)
    #[inline]
    fn matches_unqualified_elem(&self) -> bool {
        self.prefix.is_none()
    }
}

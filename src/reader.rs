//! A module to handle `Reader`

use std::borrow::Cow;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::str::from_utf8;

use encoding_rs::Encoding;

use errors::{Result, ErrorKind};
use events::{Event, BytesStart, BytesEnd, BytesText, BytesDecl};
use events::attributes::Attribute;

enum TagState {
    Opened,
    Closed,
    Empty,
}

/// A low level Xml bytes reader
///
/// Consumes a `BufRead` and streams xml `Event`s
///
/// ```
/// use quick_xml::reader::Reader;
/// use quick_xml::events::Event;
///
/// let xml = r#"<tag1 att1 = "test">
///                 <tag2><!--Test comment-->Test</tag2>
///                 <tag2>Test 2</tag2>
///             </tag1>"#;
/// let mut reader = Reader::from_str(xml);
/// reader.trim_text(true);
/// let mut count = 0;
/// let mut txt = Vec::new();
/// let mut buf = Vec::new();
/// loop {
///     match reader.read_event(&mut buf) {
///         Ok(Event::Start(ref e)) => {
///             match e.name() {
///                 b"tag1" => println!("attributes values: {:?}",
///                                     e.attributes().map(|a| a.unwrap().value).collect::<Vec<_>>()),
///                 b"tag2" => count += 1,
///                 _ => (),
///             }
///         },
///         Ok(Event::Text(e)) => txt.push(e.unescape_and_decode(&reader).unwrap()),
///         Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
///         Ok(Event::Eof) => break,
///         _ => (),
///     }
/// }
/// ```
pub struct Reader<B: BufRead> {
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
    /// a buffer to manage namespaces
    ns_buffer: NamespaceBuffer,
    /// the encoding specified in the xml, defaults to utf8
    encoding: &'static Encoding,
}

impl<B: BufRead> Reader<B> {
    /// Creates a Reader from a generic BufReader
    pub fn from_reader(reader: B) -> Reader<B> {
        Reader {
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
            ns_buffer: NamespaceBuffer::default(),
            encoding: ::encoding_rs::UTF_8,
        }
    }

    /// Change expand_empty_elements default behaviour (true per default)
    ///
    /// When set to true, all `Empty` events are expanded into an `Open` event
    /// followed by a `Close` Event.
    pub fn expand_empty_elements(&mut self, val: bool) -> &mut Reader<B> {
        self.expand_empty_elements = val;
        self
    }

    /// Change trim_text default behaviour (false per default)
    ///
    /// When set to true, all Text events are trimed.
    /// If they are empty, no event if pushed
    pub fn trim_text(&mut self, val: bool) -> &mut Reader<B> {
        self.trim_text = val;
        self
    }

    /// Change default check_end_names (true per default)
    ///
    /// When set to true, it won't check if End node match last Start node.
    /// If the xml is known to be sane (already processed etc ...)
    /// this saves extra time
    pub fn check_end_names(&mut self, val: bool) -> &mut Reader<B> {
        self.check_end_names = val;
        self
    }

    /// Change default check_comment (false per default)
    ///
    /// When set to true, every Comment event will be checked for not containing `--`
    /// Most of the time we don't want comments at all so we don't really care about
    /// comment correctness, thus default value is false for performance reason
    pub fn check_comments(&mut self, val: bool) -> &mut Reader<B> {
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
    fn read_until_open<'a, 'b>(&'a mut self, buf: &'b mut Vec<u8>) -> Result<Event<'b>> {
        self.tag_state = TagState::Opened;
        let buf_start = buf.len();
        match read_until(&mut self.reader, b'<', buf) {
            Ok(0) => Ok(Event::Eof),
            Ok(n) => {
                self.buf_position += n;
                let (start, len) = if self.trim_text {
                    match buf.iter().skip(buf_start).position(|&b| !is_whitespace(b)) {
                        Some(start) => {
                            (start,
                             buf.iter()
                                 .rposition(|&b| !is_whitespace(b))
                                 .map(|p| p + 1)
                                 .unwrap_or(buf.len()))
                        }
                        None => return self.read_event(buf),
                    }
                } else {
                    (buf_start, buf.len())
                };
                Ok(Event::Text(BytesText::borrowed(&buf[start..len])))
            }
            Err(e) => return Err(e),
        }
    }

    /// private function to read until '>' is found
    fn read_until_close<'a, 'b>(&'a mut self, buf: &'b mut Vec<u8>) -> Result<Event<'b>> {
        self.tag_state = TagState::Closed;

        // need to read 1 character to decide whether pay special attention to attribute values
        let buf_start = buf.len();
        let start;
        loop {
            // Need to contain the `self.reader.fill_buf()` in a scope lexically separate from the
            // `self.error()` call because both require `&mut self`.
            let start_result = {
                let available = match self.reader.fill_buf() {
                    Ok(n) if n.is_empty() => return Ok(Event::Eof),
                    Ok(n) => Ok(n),
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => Err(e),
                };
                // `available` is a non-empty slice => we only need the first byte to decide
                available.map(|xs| xs[0])
            };

            // throw the error we couldn't throw in the block above because `self` was sill borrowed
            start = start_result?;

            // We intentionally don't `consume()` the byte, otherwise we would have to handle things
            // like '<>' here already.
            break;
        }

        if start != b'/' && start != b'!' && start != b'?' {
            match read_elem_until(&mut self.reader, b'>', buf) {
                Ok(0) => Ok(Event::Eof),
                Ok(n) => {
                    self.buf_position += n;
                    // we already *know* that we are in this case
                    self.read_start(&buf[buf_start..])
                }
                Err(e) => return Err(e),
            }
        } else {
            match read_until(&mut self.reader, b'>', buf) {
                Ok(0) => Ok(Event::Eof),
                Ok(n) => {
                    self.buf_position += n;
                    match start {
                        b'/' => self.read_end(&buf[buf_start..]),
                        b'!' => self.read_bang(buf_start, buf),
                        b'?' => self.read_question_mark(&buf[buf_start..]),
                        _ => {
                            unreachable!("We checked that `start` must be one of [/!?], was {:?} \
                                          instead.",
                                         start)
                        }
                    }
                }
                Err(e) => return Err(e),
            }
        }

    }

    /// reads `BytesElement` starting with a `/`,
    /// if `self.check_end_names`, checks that element matches last opened element
    /// return `End` event
    fn read_end<'a, 'b>(&'a mut self, buf: &'b [u8]) -> Result<Event<'b>> {
        let len = buf.len();
        if self.check_end_names {
            match self.opened_starts.pop() {
                Some(start) => {
                    if buf[1..] != self.opened_buffer[start..] {
                        self.buf_position -= len;
                        bail!(ErrorKind::EndEventMismatch(from_utf8(&self.opened_buffer[start..])
                                                              .unwrap_or("")
                                                              .to_owned(),
                                                          from_utf8(&buf[1..])
                                                              .unwrap_or("")
                                                              .to_owned()));
                    }
                    self.opened_buffer.truncate(start);
                }
                None => {
                    self.buf_position -= len;
                    bail!(ErrorKind::EndEventMismatch("".to_owned(),
                                                      from_utf8(&buf[1..])
                                                          .unwrap_or("")
                                                          .to_owned()));
                }
            }
        }
        Ok(Event::End(BytesEnd::borrowed(&buf[1..])))
    }

    /// reads `BytesElement` starting with a `!`,
    /// return `Comment`, `CData` or `DocType` event
    fn read_bang<'a, 'b>(&'a mut self,
                         buf_start: usize,
                         buf: &'b mut Vec<u8>)
                         -> Result<Event<'b>> {
        let len = buf.len();
        if len >= 3 && &buf[buf_start + 1..buf_start + 3] == b"--" {
            let mut len = buf.len();
            while len < 5 || &buf[len - 2..] != b"--" {
                buf.push(b'>');
                match read_until(&mut self.reader, b'>', buf) {
                    Ok(0) => {
                        self.buf_position -= len;
                        bail!(io_eof("Comment"))
                    }
                    Ok(n) => self.buf_position += n,
                    Err(e) => return Err(e.into()),
                }
                len = buf.len();
            }
            if self.check_comments {
                let mut offset = len - 3;
                for w in buf[buf_start + 3..len - 1].windows(2) {
                    if &*w == b"--" {
                        self.buf_position -= offset;
                        bail!("Unexpected token '--'");
                    }
                    offset -= 1;
                }
            }
            Ok(Event::Comment(BytesText::borrowed(&buf[buf_start + 3..len - 2])))
        } else if len >= 8 {
            match &buf[buf_start + 1..buf_start + 8] {
                b"[CDATA[" => {
                    let mut len = buf.len();
                    while len < 10 || &buf[len - 2..] != b"]]" {
                        buf.push(b'>');
                        match read_until(&mut self.reader, b'>', buf) {
                            Ok(0) => {
                                self.buf_position -= len;
                                bail!(io_eof("CData"));
                            }
                            Ok(n) => self.buf_position += n,
                            Err(e) => return Err(e.into()),
                        }
                        len = buf.len();
                    }
                    Ok(Event::CData(BytesText::borrowed(&buf[buf_start + 8..len - 2])))
                }
                b"DOCTYPE" => {
                    let mut count = buf.iter().skip(buf_start).filter(|&&b| b == b'<').count();
                    while count > 0 {
                        buf.push(b'>');
                        match read_until(&mut self.reader, b'>', buf) {
                            Ok(0) => {
                                self.buf_position -= buf.len();
                                bail!(io_eof("DOCTYPE"));
                            }
                            Ok(n) => {
                                self.buf_position += n;
                                let start = buf.len() - n;
                                count += buf.iter().skip(start).filter(|&&b| b == b'<').count();
                                count -= 1;
                            }
                            Err(e) => return Err(e.into()),
                        }
                    }
                    let len = buf.len();
                    Ok(Event::DocType(BytesText::borrowed(&buf[buf_start + 8..len])))
                }
                _ => bail!("Only Comment, CDATA and DOCTYPE nodes can start with a '!'"),
            }
        } else {
            self.buf_position -= buf.len();
            bail!("Only Comment, CDATA and DOCTYPE nodes can start with a '!'");
        }
    }

    /// reads `BytesElement` starting with a `?`,
    /// return `Decl` or `PI` event
    fn read_question_mark<'a, 'b>(&'a mut self, buf: &'b [u8]) -> Result<Event<'b>> {
        let len = buf.len();
        if len > 2 && buf[len - 1] == b'?' {
            if len > 5 && &buf[1..4] == b"xml" && is_whitespace(buf[4]) {
                let event = BytesDecl::from_start(BytesStart::borrowed(&buf[1..len - 1], 3));
                // Try getting encoding from the declaration event
                if let Some(enc) = event.encoder() {
                    self.encoding = enc;
                }
                Ok(Event::Decl(event))
            } else {
                Ok(Event::PI(BytesText::borrowed(&buf[1..len - 1])))
            }
        } else {
            self.buf_position -= len;
            bail!(io_eof("XmlDecl"));
        }
    }

    #[inline]
    fn close_expanded_empty(&mut self) -> Result<Event<'static>> {
        self.tag_state = TagState::Closed;
        let name = self.opened_buffer.split_off(self.opened_starts.pop().unwrap());
        Ok(Event::End(BytesEnd::owned(name)))
    }

    /// reads `BytesElement` starting with any character except `/`, `!` or ``?`
    /// return `Start` or `Empty` event
    fn read_start<'a, 'b>(&'a mut self, buf: &'b [u8]) -> Result<Event<'b>> {
        // TODO: do this directly when reading bufreader ...
        let len = buf.len();
        let name_end = buf.iter().position(|&b| is_whitespace(b)).unwrap_or(len);
        if let Some(&b'/') = buf.last() {
            let end = if name_end < len { name_end } else { len - 1 };
            if self.expand_empty_elements {
                self.tag_state = TagState::Empty;
                self.opened_starts.push(self.opened_buffer.len());
                self.opened_buffer.extend(&buf[..end]);
                Ok(Event::Start(BytesStart::borrowed(&buf[..len - 1], end)))
            } else {
                Ok(Event::Empty(BytesStart::borrowed(&buf[..len - 1], end)))
            }
        } else {
            if self.check_end_names {
                self.opened_starts.push(self.opened_buffer.len());
                self.opened_buffer.extend(&buf[..name_end]);
            }
            Ok(Event::Start(BytesStart::borrowed(&buf, name_end)))
        }
    }

    /// reads the next `Event`
    pub fn read_event<'a, 'b>(&'a mut self, buf: &'b mut Vec<u8>) -> Result<Event<'b>> {
        if self.exit {
            return Ok(Event::Eof);
        }
        let r = match self.tag_state {
            TagState::Opened => self.read_until_close(buf),
            TagState::Closed => self.read_until_open(buf),
            TagState::Empty => self.close_expanded_empty(),
        };
        if r.is_err() {
            self.exit = true;
        }
        r
    }

    /// Resolves a potentially qualified **attribute name** into (namespace name, local name).
    ///
    /// *Qualified* attribute names have the form `prefix:local-name` where the`prefix` is defined
    /// on any containing XML element via `xmlns:prefix="the:namespace:uri"`. The namespace prefix
    /// can be defined on the same element as the attribute in question.
    ///
    /// *Unqualified* attribute names do *not* inherit the current *default namespace*.
    #[inline]
    pub fn resolve_namespace<'a, 'b>(&'a self, qname: &'b [u8]) -> (Option<&'a [u8]>, &'b [u8]) {
        self.ns_buffer.resolve_namespace(qname)
    }

    /// Reads the next event and resolve its namespace
    pub fn read_namespaced_event<'a, 'b>(&'a mut self,
                                         buf: &'b mut Vec<u8>)
                                         -> Result<(Option<&'a [u8]>, Event<'b>)> {
        self.ns_buffer.pop_empty_namespaces();
        match self.read_event(buf) {
            Ok(Event::Eof) => Ok((None, Event::Eof)),
            Ok(Event::Start(e)) => {
                self.ns_buffer.push_new_namespaces(&e);
                Ok((self.ns_buffer.find_namespace_value(e.name()), Event::Start(e)))
            }
            Ok(Event::Empty(e)) => {
                // For empty elements we need to 'artificially' keep the namespace scope on the
                // stack until the next `next()` call occurs.
                // Otherwise the caller has no chance to use `resolve` in the context of the
                // namespace declarations that are 'in scope' for the empty element alone.
                // Ex: <img rdf:nodeID="abc" xmlns:rdf="urn:the-rdf-uri" />
                self.ns_buffer.push_new_namespaces(&e);
                // notify next `read_namespaced_event()` invocation that it needs to pop this
                // namespace scope
                self.ns_buffer.pending_pop = true;
                Ok((self.ns_buffer.find_namespace_value(e.name()), Event::Empty(e)))
            }
            Ok(Event::End(e)) => {
                // notify next `read_namespaced_event()` invocation that it needs to pop this
                // namespace scope
                self.ns_buffer.pending_pop = true;
                Ok((self.ns_buffer.find_namespace_value(e.name()), Event::End(e)))
            }
            Ok(e) => Ok((None, e)),
            Err(e) => Err(e),
        }
    }

    /// Decodes a slice using the xml specified encoding
    ///
    /// Decode complete input to Cow<'a, str> with BOM sniffing and with malformed sequences
    /// replaced with the REPLACEMENT CHARACTER
    ///
    /// If no encoding is specified, then defaults to UTF_8
    #[inline]
    pub fn decode<'b, 'c>(&'b self, bytes: &'c [u8]) -> Cow<'c, str> {
        self.encoding.decode(bytes).0
    }
}

impl<B: BufRead> Reader<B> {
    /// Reads until end element is found
    ///
    /// Manages nested cases where parent and child elements have the same name
    pub fn read_to_end<K: AsRef<[u8]>>(&mut self, end: K, buf: &mut Vec<u8>) -> Result<()> {
        let mut depth = 0;
        let end = end.as_ref();
        loop {
            match self.read_event(buf) {
                Ok(Event::End(ref e)) if e.name() == end => {
                    if depth == 0 {
                        return Ok(());
                    }
                    depth -= 1;
                }
                Ok(Event::Start(ref e)) if e.name() == end => depth += 1,
                Err(e) => return Err(e),
                Ok(Event::Eof) => {
                    return Err(io_eof(&format!("Expecting {:?} end", from_utf8(end))).into())
                }
                _ => (),
            }
            buf.clear();
        }
    }

    /// Reads next event, if `Event::Text` or `Event::End`,
    /// then returns an unescaped `String`, else returns an error
    ///
    /// # Examples
    ///
    /// ```
    /// use quick_xml::reader::Reader;
    /// use quick_xml::events::Event;
    ///
    /// let mut xml = Reader::from_reader(b"<a>&lt;b&gt;</a>" as &[u8]);
    ///
    /// xml.trim_text(true);
    /// let mut buf = Vec::new();
    ///
    /// match xml.read_event(&mut buf) {
    ///     Ok(Event::Start(ref e)) => {
    ///         assert_eq!(&xml.read_text(e.name(), &mut Vec::new()).unwrap(), "<b>");
    ///     },
    ///     e => panic!("Expecting Start(a), found {:?}", e),
    /// }
    /// ```
    pub fn read_text<K: AsRef<[u8]>>(&mut self, end: K, buf: &mut Vec<u8>) -> Result<String> {
        let s = match self.read_event(buf) {
            Ok(Event::Text(e)) => e.unescape_and_decode(self),
            Ok(Event::End(ref e)) if e.name() == end.as_ref() => return Ok("".to_string()),
            Err(e) => return Err(e),
            Ok(Event::Eof) => return Err(io_eof("Text").into()),
            _ => return Err("Cannot read text, expecting Event::Text".into()),
        };
        self.read_to_end(end, buf)?;
        s
    }
}

impl Reader<BufReader<File>> {
    /// Creates a xml reader from a file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Reader<BufReader<File>>> {
        let reader = BufReader::new(try!(File::open(path)));
        Ok(Reader::from_reader(reader))
    }
}

impl<'a> Reader<&'a [u8]> {
    /// Creates a xml reader from a file path
    pub fn from_str(s: &'a str) -> Reader<&'a [u8]> {
        Reader::from_reader(s.as_bytes())
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
                Err(e) => bail!(e),
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
#[inline]
fn read_elem_until<R: BufRead>(r: &mut R, end_byte: u8, buf: &mut Vec<u8>) -> Result<usize> {
    #[derive(Debug,Clone,Copy,PartialEq,Eq)]
    enum ElemReadState {
        /// The initial state (inside element, but outside of attribute value)
        Elem,
        /// Inside a single-quoted attribute value
        SingleQ,
        /// Inside a double-quoted attribute value
        DoubleQ,
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
                Err(e) => bail!(e),
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
                            }
                            (ElemReadState::Elem, b'\'') => ElemReadState::SingleQ,
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

/// A namespace declaration. Can either bind a namespace to a prefix or define the current default
/// namespace.
#[derive(Debug)]
struct Namespace {
    /// Index of the namespace in the buffer
    start: usize,
    /// Length of the prefix
    /// * if bigger than start, then binds this namespace to the corresponding slice.
    /// * else defines the current default namespace.
    prefix_len: usize,
    /// The namespace name (the URI) of this namespace declaration.
    ///
    /// The XML standard specifies that an empty namespace value 'removes' a namespace declaration
    /// for the extent of its scope. For prefix declarations that's not very interesting, but it is
    /// vital for default namespace declarations. With `xmlns=""` you can revert back to the default
    /// behaviour of leaving unqualified element names unqualified.
    value_len: usize,
    /// Level of nesting at which this namespace was declared. The declaring element is included,
    /// i.e., a declaration on the document root has `level = 1`.
    /// This is used to pop the namespace when the element gets closed.
    level: i32,
}

impl Namespace {
    /// Gets the prefix slice out of namespace buffer
    #[inline]
    fn prefix<'a, 'b>(&'a self, ns_buffer: &'b [u8]) -> &'b [u8] {
        &ns_buffer[self.start..self.start + self.prefix_len]
    }

    /// Gets the value slice out of namespace buffer
    ///
    /// Returns `None` if `value_len == 0`
    #[inline]
    fn opt_value<'a, 'b>(&'a self, ns_buffer: &'b [u8]) -> Option<&'b [u8]> {
        if self.value_len == 0 {
            None
        } else {
            Some(&ns_buffer[self.start + self.prefix_len..
                  self.start + self.prefix_len + self.value_len])
        }
    }
}

/// A namespace management buffer.
///
/// Holds all internal logic to push/pop namespaces with their levels.
#[derive(Debug, Default)]
struct NamespaceBuffer {
    /// a buffer of namespace ranges
    slices: Vec<Namespace>,
    /// a buffer of existing namespaces
    buffer: Vec<u8>,
    /// The number of open tags at the moment. We need to keep track of this to know which namespace
    /// declarations to remove when we encounter an `End` event.
    nesting_level: i32,
    /// For `Empty` events keep the 'scope' of the element on the stack artificially. That way, the
    /// consumer has a chance to use `resolve` in the context of the empty element. We perform the
    /// pop as the first operation in the next `next()` call.
    pending_pop: bool,
}

impl NamespaceBuffer {
    #[inline]
    fn find_namespace_value(&self, element_name: &[u8]) -> Option<&[u8]> {
        let ns = match element_name.iter().position(|b| *b == b':') {
            None => self.slices.iter().rev().find(|n| n.prefix_len == 0),
            Some(len) => {
                self.slices
                    .iter()
                    .rev()
                    .find(|n| n.prefix(&self.buffer) == &element_name[..len])
            }
        };
        ns.and_then(|ref n| n.opt_value(&self.buffer))
    }

    fn pop_empty_namespaces(&mut self) {
        if !self.pending_pop {
            return;
        }
        self.pending_pop = false;
        self.nesting_level -= 1;
        let current_level = self.nesting_level;
        // from the back (most deeply nested scope), look for the first scope that is still valid
        match self.slices.iter().rposition(|n| n.level <= current_level) {
            // none of the namespaces are valid, remove all of them
            None => {
                self.buffer.clear();
                self.slices.clear();
            }
            // drop all namespaces past the last valid namespace
            Some(last_valid_pos) => {
                if let Some(len) = self.slices.get(last_valid_pos + 1).map(|n| n.start) {
                    self.buffer.truncate(len);
                    self.slices.truncate(last_valid_pos + 1);
                }
            }
        }
    }

    fn push_new_namespaces(&mut self, e: &BytesStart) {
        self.nesting_level += 1;
        let level = self.nesting_level;
        // adds new namespaces for attributes starting with 'xmlns:' and for the 'xmlns'
        // (default namespace) attribute.
        for a in e.attributes().with_checks(false) {
            if let Ok(Attribute { key: k, value: v }) = a {
                if k.starts_with(b"xmlns") {
                    match k.get(5) {
                        None => {
                            let start = self.buffer.len();
                            self.buffer.extend_from_slice(v);
                            self.slices.push(Namespace {
                                start: start,
                                prefix_len: 0,
                                value_len: v.len(),
                                level: level,
                            });
                        }
                        Some(&b':') => {
                            let start = self.buffer.len();
                            self.buffer.extend_from_slice(&k[6..]);
                            self.buffer.extend_from_slice(v);
                            self.slices.push(Namespace {
                                start: start,
                                prefix_len: k.len() - 6,
                                value_len: v.len(),
                                level: level,
                            });
                        }
                        _ => break,
                    }
                }
            } else {
                break;
            }
        }
    }

    /// Resolves a potentially qualified **attribute name** into (namespace name, local name).
    ///
    /// *Qualified* attribute names have the form `prefix:local-name` where the`prefix` is defined
    /// on any containing XML element via `xmlns:prefix="the:namespace:uri"`. The namespace prefix
    /// can be defined on the same element as the attribute in question.
    ///
    /// *Unqualified* attribute names do *not* inherit the current *default namespace*.
    #[inline]
    fn resolve_namespace<'a, 'b>(&'a self, qname: &'b [u8]) -> (Option<&'a [u8]>, &'b [u8]) {
        qname.iter()
            .position(|b| *b == b':')
            .and_then(|len| {
                let (prefix, value) = qname.split_at(len);
                self.slices
                    .iter()
                    .rev()
                    .find(|n| n.prefix(&self.buffer) == prefix)
                    .map(|ns| (ns.opt_value(&self.buffer), &value[1..]))
            })
            .unwrap_or((None, qname))
    }
}

fn io_eof(msg: &str) -> ::std::io::Error {
    ::std::io::Error::new(::std::io::ErrorKind::UnexpectedEof, msg.to_string())
}

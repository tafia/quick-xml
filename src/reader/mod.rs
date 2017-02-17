//! A module to handle `XmlReader`

pub mod attributes;
pub mod namespace;

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::str::from_utf8;

use error::{Error, Result, ResultPos};
use self::namespace::XmlnsReader;
use super::{Element, Event, XmlDecl, AsStr};

#[derive(Clone)]
enum TagState {
    Opened,
    Closed,
    Empty,
}

/// A Xml reader
///
/// Consumes a `BufRead` and streams xml `Event`s
///
/// ```
/// use quick_xml::{XmlReader, Event};
///
/// let xml = r#"<tag1 att1 = "test">
///                 <tag2><!--Test comment-->Test</tag2>
///                 <tag2>Test 2</tag2>
///             </tag1>"#;
/// let reader = XmlReader::from(xml).trim_text(true);
/// let mut count = 0;
/// let mut txt = Vec::new();
/// for r in reader {
///     match r {
///         Ok(Event::Start(ref e)) => {
///             match e.name() {
///                 b"tag1" => println!("attributes values: {:?}",
///                                     e.attributes()
///                                     .map(|a| a.unwrap().1)
///                                     .collect::<Vec<_>>()),
///                 b"tag2" => count += 1,
///                 _ => (),
///             }
///         },
///         Ok(Event::Text(e)) => txt.push(e.into_string()),
///         Err((e, pos)) => panic!("{:?} at position {}", e, pos),
///         _ => (),
///     }
/// }
/// ```
#[derive(Clone)]
pub struct XmlReader<B: BufRead> {
    /// reader
    reader: B,
    /// if was error, exit next
    exit: bool,
    /// all currently Started elements which didn't have a matching
    /// End element yet
    opened: Vec<Element>,
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

impl<'a> ::std::convert::From<&'a str> for XmlReader<&'a [u8]> {
    fn from(reader: &'a str) -> XmlReader<&'a [u8]> {
        XmlReader::from_reader(reader.as_bytes())
    }
}

impl<B: BufRead> XmlReader<B> {
    /// Creates a XmlReader from a generic BufReader
    pub fn from_reader(reader: B) -> XmlReader<B> {
        XmlReader {
            reader: reader,
            exit: false,
            opened: Vec::new(),
            tag_state: TagState::Closed,
            expand_empty_elements: true,
            trim_text: false,
            with_check: true,
            buf_position: 0,
            check_comments: false,
        }
    }

    /// Converts into a `XmlnsReader` iterator
    pub fn namespaced(self) -> XmlnsReader<B> {
        XmlnsReader::new(self)
    }

    /// Change expand_empty_elements default behaviour (true per default)
    ///
    /// When set to true, all `Empty` events are expanded into an `Open` event
    /// followed by a `Close` Event.
    pub fn expand_empty_elements(mut self, val: bool) -> XmlReader<B> {
        self.expand_empty_elements = val;
        self
    }

    /// Change trim_text default behaviour (false per default)
    ///
    /// When set to true, all Text events are trimed.
    /// If they are empty, no event if pushed
    pub fn trim_text(mut self, val: bool) -> XmlReader<B> {
        self.trim_text = val;
        self
    }

    /// Change default with_check (true per default)
    ///
    /// When set to true, it won't check if End node match last Start node.
    /// If the xml is known to be sane (already processed etc ...)
    /// this saves extra time
    pub fn with_check(mut self, val: bool) -> XmlReader<B> {
        self.with_check = val;
        self
    }

    /// Change default check_comment (false per default)
    ///
    /// When set to true, every Comment event will be checked for not containing `--`
    /// Most of the time we don't want comments at all so we don't really care about
    /// comment correctness, thus default value is false for performance reason
    pub fn check_comments(mut self, val: bool) -> XmlReader<B> {
        self.check_comments = val;
        self
    }

    /// Reads until end element is found
    ///
    /// Manages nested cases where parent and child elements have the same name
    pub fn read_to_end<K: AsRef<[u8]>>(&mut self, end: K) -> ResultPos<()> {
        let mut depth = 0;
        let end = end.as_ref();
        loop {
            match self.next() {
                Some(Ok(Event::End(ref e))) if e.name() == end => {
                    if depth == 0 {
                        return Ok(());
                    }
                    depth -= 1;
                }
                Some(Ok(Event::Start(ref e))) if e.name() == end => depth += 1,
                Some(Err(e)) => return Err(e),
                None => {
                    warn!("EOF instead of {:?}", from_utf8(end));
                    return Err((Error::Unexpected(format!(
                                    "Reached EOF, expecting {:?} end tag",
                                    from_utf8(end))),
                                self.buf_position));
                }
                _ => (),
            }
        }
    }

    /// Reads next event, if `Event::Text` or `Event::End`,
    /// then returns a `String`, else returns an error
    pub fn read_text<K: AsRef<[u8]>>(&mut self, end: K) -> ResultPos<String> {
        match self.next() {
            Some(Ok(Event::Text(e))) => {
                self.read_to_end(end)
                    .and_then(|_| e.into_string().map_err(|e| (e, self.buf_position)))
            }
            Some(Ok(Event::End(ref e))) if e.name() == end.as_ref() => {
                Ok("".to_string())
            },
            Some(Err(e)) => Err(e),
            None => {
                Err((Error::Unexpected("Reached EOF while reading text".to_string()),
                     self.buf_position))
            }
            _ => {
                Err((Error::Unexpected("Cannot read text, expecting Event::Text".to_string()),
                     self.buf_position))
            }
        }
    }

    /// Reads next event, if `Event::Text` or `Event::End`,
    /// then returns an unescaped `String`, else returns an error
    ///
    /// # Examples
    /// 
    /// ```
    /// use quick_xml::{XmlReader, Event};
    ///
    /// let mut xml = XmlReader::from_reader(b"<a>&lt;b&gt;</a>" as &[u8]).trim_text(true);
    /// match xml.next() {
    ///     Some(Ok(Event::Start(ref e))) => {
    ///         assert_eq!(&xml.read_text_unescaped(e.name()).unwrap(), "<b>");
    ///     },
    ///     e => panic!("Expecting Start(a), found {:?}", e),
    /// }
    /// ```
    pub fn read_text_unescaped<K: AsRef<[u8]>>(&mut self, end: K) -> ResultPos<String> {
        match self.next() {
            Some(Ok(Event::Text(e))) => {
                self.read_to_end(end)
                    .and_then(|_| e.unescaped_content())
                    .and_then(|c| c.as_str()
                              .map_err(|e| (e, self.buf_position))
                              .map(|s| s.to_string()))
            }
            Some(Ok(Event::End(ref e))) if e.name() == end.as_ref() => {
                Ok("".to_string())
            },
            Some(Err(e)) => Err(e),
            None => {
                Err((Error::Unexpected("Reached EOF while reading text".to_string()),
                     self.buf_position))
            }
            _ => {
                Err((Error::Unexpected("Cannot read text, expecting Event::Text".to_string()),
                     self.buf_position))
            }
        }
    }

    /// Gets the current BufRead position
    /// Useful when debugging errors
    pub fn buffer_position(&self) -> usize {
        self.buf_position
    }

    /// private function to read until '<' is found
    /// return a `Text` event
    fn read_until_open(&mut self) -> Option<ResultPos<Event>> {
        self.tag_state = TagState::Opened;
        let mut buf = Vec::new();
        match read_until(&mut self.reader, b'<', &mut buf) {
            Ok(0) => None,
            Ok(n) => {
                self.buf_position += n;
                let (start, len) = if self.trim_text {
                    match buf.iter().position(|&b| !is_whitespace(b)) {
                        Some(start) => {
                            (start, buf.len() - buf.iter().rev()
                                                   .position(|&b| !is_whitespace(b))
                                                   .unwrap_or(0))
                        }
                        None => return self.next(),
                    }
                } else {
                    (0, buf.len())
                };
                Some(Ok(Event::Text(Element::from_buffer(buf, start, len, len))))
            }
            Err(e) => Some(self.error(e, 0)),
        }
    }

    /// private function to read until '>' is found
    fn read_until_close(&mut self) -> Option<ResultPos<Event>> {
        self.tag_state = TagState::Closed;

        // need to read 1 character to decide whether pay special attention to attribute values
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

        let mut buf = Vec::new();
        if start != b'/' && start != b'!' && start != b'?' {
            match read_elem_until(&mut self.reader, b'>', &mut buf) {
                Ok(0) => None,
                Ok(n) => {
                    self.buf_position += n;
                    // we already *know* that we are in this case
                    Some(self.read_start(buf))
                }
                Err(e) => Some(self.error(e, 0)),
            }
        } else {
            match read_until(&mut self.reader, b'>', &mut buf) {
                Ok(0) => None,
                Ok(n) => {
                    self.buf_position += n;
                    match start {
                        b'/' => Some(self.read_end(buf)),
                        b'!' => Some(self.read_bang(buf)),
                        b'?' => Some(self.read_question_mark(buf)),
                        _ => unreachable!("We checked that `start` must be one of [/!?], \
                                            was {:?} instead.", start),
                    }
                }
                Err(e) => Some(self.error(e, 0)),
            }
        }

    }

    /// reads `Element` starting with a `/`,
    /// if `self.with_check`, checks that element matches last opened element
    /// return `End` event
    fn read_end(&mut self, buf: Vec<u8>) -> ResultPos<Event> {
        let len = buf.len();
        if self.with_check {
            let e = match self.opened.pop() {
                Some(e) => e,
                None => return self.error(
                    Error::Malformed(format!("Cannot close {:?} element, \
                                             there is no opened element",
                                             buf[1..].as_str())), len),
            };
            if &buf[1..] != e.name() {
                let m = format!("End event {:?} doesn't match last \
                                opened element {:?}, opened: {:?}",
                                Element::from_buffer(buf, 1, len, len), e, &self.opened);
                return self.error(Error::Malformed(m), len);
            }
        }
        Ok(Event::End(Element::from_buffer(buf, 1, len, len)))
    }

    /// reads `Element` starting with a `!`,
    /// return `Comment`, `CData` or `DocType` event
    fn read_bang(&mut self, mut buf: Vec<u8>) -> ResultPos<Event> {
        let len = buf.len();
        if len >= 3 && &buf[1..3] == b"--" {
            let mut len = buf.len();
            while len < 5 || &buf[(len - 2)..] != b"--" {
                buf.push(b'>');
                match read_until(&mut self.reader, b'>', &mut buf) {
                    Ok(0) => return self.error(
                        Error::Malformed("Unescaped Comment event".to_string()), len),
                    Ok(n) => self.buf_position += n,
                    Err(e) => return self.error(e, 0),
                }
                len = buf.len();
            }
            if self.check_comments {
                let mut offset = len - 3;
                for w in buf[3..(len - 1)].windows(2) {
                    if &*w == b"--" {
                        return self.error(
                            Error::Malformed("Unexpected token '--'".to_string()), offset);
                    }
                    offset -= 1;
                }
            }
            Ok(Event::Comment(Element::from_buffer(buf, 3, len - 2, len - 2)))
        } else if len >= 8 {
            match &buf[1..8] {
                b"[CDATA[" => {
                    let mut len = buf.len();
                    while len < 10 || &buf[(len - 2)..] != b"]]" {
                        buf.push(b'>');
                        match read_until(&mut self.reader, b'>', &mut buf) {
                            Ok(0) => return self.error(
                                Error::Malformed("Unescaped CDATA event".to_string()), len),
                            Ok(n) => self.buf_position += n,
                            Err(e) => return self.error(e, 0),
                        }
                        len = buf.len();
                    }
                    Ok(Event::CData(Element::from_buffer(buf, 8, len - 2, len - 2)))
                }
                b"DOCTYPE" => {
                    let mut count = buf.iter().filter(|&&b| b == b'<').count();
                    while count > 0 {
                        buf.push(b'>');
                        match read_until(&mut self.reader, b'>', &mut buf) {
                            Ok(0) => return self.error(
                                Error::Malformed("Unescaped DOCTYPE node".to_string()), buf.len()),
                            Ok(n) => {
                                self.buf_position += n;
                                let start = buf.len() - n;
                                count += buf[start..].iter().filter(|&&b| b == b'<').count() - 1;
                            }
                            Err(e) => return self.error(e, 0),
                        }
                    }
                    let len = buf.len();
                    Ok(Event::DocType(Element::from_buffer(buf, 1, len, 8)))
                }
                _ => self.error(Error::Malformed("Only Comment, CDATA and DOCTYPE nodes \
                                                 can start with a '!'".to_string()), 0),
            }
        } else {
            self.error(Error::Malformed("Only Comment, CDATA and DOCTYPE nodes can start \
                                        with a '!'".to_string()), buf.len())
        }
    }

    /// reads `Element` starting with a `?`,
    /// return `Decl` or `PI` event
    fn read_question_mark(&mut self, buf: Vec<u8>) -> ResultPos<Event> {
        let len = buf.len();
        if len > 2 && buf[len - 1] == b'?' {
            if len > 5 && &buf[1..4] == b"xml" && is_whitespace(buf[4]) {
                Ok(Event::Decl(XmlDecl { element: Element::from_buffer(buf, 1, len - 1, 3) }))
            } else {
                Ok(Event::PI(Element::from_buffer(buf, 1, len - 1, 3)))
            }
        } else {
            self.error(Error::Malformed("Unescaped XmlDecl event".to_string()), len)
        }
    }

    fn close_expanded_empty(&mut self) -> Option<ResultPos<Event>> {
        self.tag_state = TagState::Closed;
        let e = self.opened.pop().unwrap();
        Some(Ok(Event::End(e)))
    }

    /// reads `Element` starting with any character except `/`, `!` or ``?`
    /// return `Start` or `Empty` event
    fn read_start(&mut self, buf: Vec<u8>) -> ResultPos<Event> {
        // TODO: do this directly when reading bufreader ...
        let len = buf.len();
        let name_end = buf.iter().position(|&b| is_whitespace(b)).unwrap_or(len);
        if buf[len - 1] == b'/' {
            let end = if name_end < len { name_end } else { len - 1 };
            let element = Element::from_buffer(buf, 0, len - 1, end);
            if self.expand_empty_elements {
                self.tag_state = TagState::Empty;
                self.opened.push(element.clone());
                Ok(Event::Start(element))
            } else {
                Ok(Event::Empty(element))
            }
        } else {
            let element = Element::from_buffer(buf, 0, len, name_end);
            if self.with_check { self.opened.push(element.clone()); }
            Ok(Event::Start(element))
        }
    }

    /// returns `Err(Error, buf_position - offset)`
    /// sets `self.exit = true` so next call will terminate the iterator
    fn error(&mut self, e: Error, offset: usize) -> ResultPos<Event> {
        self.exit = true;
        Err((e, self.buf_position - offset))
    }
}

impl XmlReader<BufReader<File>> {
    /// Creates a xml reader from a file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<XmlReader<BufReader<File>>> {
        let reader = BufReader::new(try!(File::open(path)));
        Ok(XmlReader::from_reader(reader))
    }
}

/// Iterator on xml returning `Event`s
impl<B: BufRead> Iterator for XmlReader<B> {
    type Item = ResultPos<Event>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exit {
            return None;
        }
        match self.tag_state {
            TagState::Opened => self.read_until_close(),
            TagState::Closed => self.read_until_open(),
            TagState::Empty => self.close_expanded_empty(),
        }
    }
}

/// `read_until` slightly modified from rust std library
///
/// only change is that we do not write the matching character
#[inline]
fn read_until<R: BufRead>(r: &mut R, byte: u8, buf: &mut Vec<u8>)
    -> Result<usize>
{
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

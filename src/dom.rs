//! A module to manage DOM documents
//!
//! This is a very simple/experimental wrapper over the pull based `Reader`
//! The idea is to provide very basic mechanism to get a particular data out of an xml file
//!
//! # Examples
//!
//! ```rust
//! use quick_xml::dom::Node;
//!
//! // loads the entire file in memory and converts it into a `Node`
//! let path = "/path/to/my/file.xml";
//! # let path = "tests/sample_rss.xml";
//! let mut root = Node::open(path).expect("cannot read file");
//!
//! // gets specific nodes following a particular path
//! {
//!     let nodes = root.select("a/b/c");
//!     for n in nodes {
//!         println!("node: name: {}, attributes count: {}, children count: {}",
//!             n.name(), n.attributes().len(), n.children().len());
//!     }
//! }
//!
//! // Now let's say we want to modify the document
//! if let Some(child) = root.children_mut().get_mut(0) {
//!     child.attributes_mut().push(("My new key".to_string(), "My new value".to_string()));
//! }
//!
//! // we're done, we can save it back to a new file
//! root.save("/dev/null").expect("cannot save file");
//! ```

use std::io::{self, BufRead};

use escape::unescape;
use events::{Event, BytesStart, BytesEnd, BytesText};
use errors::{Result, ErrorKind};
use reader::Reader;

/// A DOM `Node`
///
/// Has name, attributes and children
#[derive(Debug, Default, Clone)]
pub struct Node {
    name: String,
    attributes: Vec<(String, String)>,
    text: String,
    children: Vec<Node>,
}

impl Node {

    /// Private constructor from a `BytesStart` event
    fn from_start<B: BufRead>(start: BytesStart, reader: &Reader<B>) -> Result<Node> {
        let mut atts = Vec::new();
        for a in start.attributes() {
            let a = a?;
            atts.push((reader.decode(a.key).into_owned(), a.unescape_and_decode_value(reader)?));
        }
        Ok(Node {
            name: reader.decode(&unescape(start.name())?).into_owned(),
            attributes: atts,
            text: String::new(),
            children: Vec::new(),
        })
    }

    /// Consumes a reader and returns the root `Node`
    pub fn root<R: BufRead>(read: R) -> Result<Node> {
        let mut reader = Reader::from_reader(read);
        let mut buffer = Vec::new();
        let mut parents = Vec::new();
        let mut node = None;
        loop {
            match reader.read_event(&mut buffer)? {
                Event::Eof => bail!(ErrorKind::Io(io::Error::new(io::ErrorKind::UnexpectedEof,
                                                                 "EOF before closing event"))),
                Event::Start(start) => {
                    if let Some(e) = node {
                        parents.push(e);
                    }
                    node = Some(Node::from_start(start, &reader)?);
                }
                Event::Empty(start) => {
                    if let Some(ref mut e) = node {
                        e.children.push(Node::from_start(start, &reader)?);
                    } else {
                        return Ok(Node::from_start(start, &reader)?);
                    }
                }
                Event::Text(t) => { 
                    if let Some(ref mut e) = node {
                        e.text = t.unescape_and_decode(&reader)?;
                    }
                }
                Event::End(ref end) => {
                    match (parents.pop(), node) {
                        (Some(mut p), Some(e)) => {
                            if e.name.as_bytes() == end.name() {
                                p.children.push(e);
                                node = Some(p);
                            } else {
                                bail!(ErrorKind::EndEventMismatch(
                                        e.name, reader.decode(end.name()).into_owned()));
                            }
                        },
                        (None, Some(e)) => {
                            if e.name.as_bytes() == end.name() {
                                return Ok(e);
                            } else {
                                bail!(ErrorKind::EndEventMismatch(
                                        e.name, reader.decode(end.name()).into_owned()));
                            }
                        },
                        (_, None) => bail!(ErrorKind::EndEventMismatch(
                                "".to_string(), reader.decode(end.name()).into_owned())),
                    }
                }
                _ => (), // ignore other events
            }
            buffer.clear();
        }
    }

    /// Converts a file into a `Node`
    pub fn open<P: AsRef<::std::path::Path>>(path: P) -> Result<Node> {
        let file = ::std::fs::File::open(path)?;
        Node::root(::std::io::BufReader::new(file))
    }

    /// Creates a simple `Node` from its name
    pub fn new<S: Into<String>>(name: S) -> Node {
        Node {
            name: name.into(),
            attributes: Vec::new(),
            text: String::new(),
            children: Vec::new()
        }
    }

    /// Gets `Node` name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets mutable `Node` name
    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    /// Gets `Node` attributes (key, value)
    pub fn attributes(&self) -> &[(String, String)] {
        &self.attributes
    }

    /// Gets mutable `Node` attributes (key, value)
    pub fn attributes_mut(&mut self) -> &mut Vec<(String, String)> {
        &mut self.attributes
    }

    /// Gets `Node` text content
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get a mutable text
    pub fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    /// Gets `Node` children
    pub fn children(&self) -> &[Node] {
        &self.children
    }

    /// Gets mutable `Node` children
    pub fn children_mut(&mut self) -> &mut Vec<Node> {
        &mut self.children
    }

    /// Gets an iterator over all children nodes matching a certain path
    /// 
    /// For now, only simple node paths are supported
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quick_xml::dom::Node;
    /// 
    /// let data = r#"
    /// <a>
    ///     <b>
    ///         <c>test 1</c>
    ///         <c att1='test att'/>
    ///         <c>test 2</c>
    ///     </b>
    ///     <b>
    ///         <c>test 3</c>
    ///     </b>
    /// </a>
    /// "#;
    ///
    /// let root = Node::root(::std::io::Cursor::new(data)).unwrap();
    /// let search_path = "b/c";
    /// let select_texts = root.select(search_path).iter()
    ///     .map(|n| n.text()).collect::<Vec<_>>();
    ///
    /// assert_eq!(vec!["test 1", "", "test 2", "test 3"], select_texts);
    /// ```
    pub fn select<'a, 'b, X: Into<XPath<'b>>>(&'a self, path: X) -> Vec<&'a Node>
    {
        // TODO: use impl Trait once stabilized
        // TODO: implement more XPath syntaxes

        let xpath = path.into();
        if xpath.inner.is_empty() {
            Vec::new()
        } else {
            let paths = xpath.inner.split('/').collect::<Vec<_>>();
            let mut vec = Vec::new();
            self.extend_select_all(&mut vec, 0, &paths);
            vec
        }
    }

    fn extend_select_all<'a>(&'a self, vec: &mut Vec<&'a Node>, idx: usize, paths: &[&str]) {
        let iter = self.children.iter().filter(|c| c.name == paths[idx]);
        if idx == paths.len() - 1 {
            vec.extend(iter);
        } else {
            for ch in iter {
                ch.extend_select_all(vec, idx + 1, paths);
            }
        }
    }

    /// Saves the content of the xml into a new file
    ///
    /// Due to technical issues, the output file will be different than the input file.
    /// As a result it might be a good idea to save them in different paths
    pub fn save<P: AsRef<::std::path::Path>>(&self, dest: P) -> Result<()> {
        let file = ::std::fs::File::create(dest)?;
        let mut writer = ::writer::Writer::new(::std::io::BufWriter::new(file));
        self.write(&mut writer)
    }

    /// Writes the node and its descendants into the `Writer`
    pub fn write<W: ::std::io::Write>(&self, writer: &mut ::writer::Writer<W>) -> Result<()> {
        let mut start = BytesStart::borrowed(self.name.as_bytes(), self.name.len());
        start.with_attributes(self.attributes.iter().map(|&(ref k, ref v)| (&**k, &**v)));
        writer.write_event(Event::Start(start))?;
        if !self.text.is_empty() { 
            writer.write_event(Event::Text(BytesText::borrowed(self.text.as_bytes())))?;
        }
        for ch in &self.children {
            ch.write(writer)?;
        }
        writer.write_event(Event::End(BytesEnd::borrowed(self.name.as_bytes())))?;
        Ok(())
    }
}

/// A struct to handle XPath parameters
///
/// For the moment it is just a wrapper over `&str`
/// Used to enable future improvements
pub struct XPath<'a> {
    inner: &'a str
}

impl<'a> From<&'a str> for XPath<'a> {
    fn from(s: &'a str) -> XPath<'a> {
        XPath { inner: s }
    }
}

#[test]
fn test_select_all() {
    let data = r#"
    <a>
        <b>
            <c>test 1</c>
            <c att1='test att'/>
            <c>test 2</c>
        </b>
        <b>
            <c>test 3</c>
        </b>
    </a>
    "#;

    let root = Node::root(::std::io::Cursor::new(data)).unwrap();
    let select_texts = root.select("b/c").iter().map(|n| n.text()).collect::<Vec<_>>();

    assert_eq!(vec!["test 1", "", "test 2", "test 3"], select_texts);
}

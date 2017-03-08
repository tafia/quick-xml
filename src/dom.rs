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
//!
//! Alternatively, if you want just to get the xpath filtered nodes and ditch the rest, you 
//! can directly call the `open_xpath` method, which is faster
//!
//! ```rust
//! use quick_xml::dom::Node;
//!
//! // loads the entire file in memory and converts it into a `Node`
//! let path = "/path/to/my/file.xml";
//! # let path = "tests/sample_rss.xml";
//! let nodes = Node::open_xpath(path, "a/b/c").expect("cannot read file");
//! ```

use std::io::{BufRead, BufReader};
use std::path::Path;

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
        reader.check_end_names(false)
            .check_comments(false)
            .expand_empty_elements(false)
            .trim_text(true);
        let mut buffer = Vec::new();
        let node = Node::new("/"); // starts with the root node
        Node::from_reader(&mut reader, &mut buffer, node)
    }

    fn from_reader<R: BufRead>(reader: &mut Reader<R>, 
                               buffer: &mut Vec<u8>,
                               mut node: Node) -> Result<Node> {
        let mut parents = Vec::new();
        loop {
            match reader.read_event(buffer)? {
                Event::Eof => return Ok(node),
                Event::Start(start) => {
                    parents.push(node);
                    node = Node::from_start(start, &reader)?;
                }
                Event::Empty(start) => {
                    node.children.push(Node::from_start(start, &reader)?);
                }
                Event::Text(t) => { 
                    node.text = t.unescape_and_decode(&reader)?;
                }
                Event::End(ref end) => {
                    if node.name.as_bytes() != end.name() {
                        bail!(ErrorKind::EndEventMismatch(
                                node.name, reader.decode(end.name()).into_owned()));
                    }
                    match parents.pop() {
                        Some(mut p) => {
                            p.children.push(node);
                            node = p;
                        },
                        None => return Ok(node),
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
        Node::root(BufReader::new(file))
    }

    /// Opens a file, then outputs ONLY the nodes matching an XPath (and its descendants)
    ///
    /// This is faster than resolving the entire xml but you only have a subset of the data
    pub fn open_xpath<'a, P, X>(path: P, xpath: X) -> Result<Vec<Node>>
        where P: AsRef<Path>,
              X: Into<XPath<'a>>,
    {
        let file = ::std::fs::File::open(path)?;
        Node::from_xpath(BufReader::new(file), xpath)
    }

    /// Consumes a reader until the end and outputs ONLY the nodes matching an XPath (and its descendants)
    ///
    /// This is faster than resolving the entire xml but you only have a subset of the data
    pub fn from_xpath<'a, R, X>(read: R, xpath: X) -> Result<Vec<Node>>
        where R: BufRead,
              X: Into<XPath<'a>>,
    {
        let mut nodes = Vec::new();
        let xpath = xpath.into();

        // discard the root node if asked
        let mut idx = if let Some(&"/") = xpath.inner.first() { 1 } else { 0 };

        let mut reader = Reader::from_reader(read);
        reader.check_end_names(false)
            .check_comments(false)
            .expand_empty_elements(false)
            .trim_text(true);
        let mut buffer = Vec::new();
        loop {
            match reader.read_event(&mut buffer)? {
                Event::Eof => return Ok(nodes),
                Event::Start(start) => {
                    if start.name() == xpath.inner[idx].as_bytes() {
                        if idx == xpath.inner.len() - 1 {
                            let node = Node::from_start(start, &reader)?;
                            nodes.push(Node::from_reader(&mut reader, &mut Vec::new(), node)?);
                        } else {
                            idx += 1;
                        }
                    }
                }
                Event::Empty(start) => {
                    if start.name() == xpath.inner[idx].as_bytes() {
                        if idx == xpath.inner.len() - 1 {
                            let node = Node::from_start(start, &reader)?;
                            nodes.push(node);
                        }
                    }
                }
                Event::End(ref end) => {
                    if end.name() == xpath.inner[idx].as_bytes() {
                        idx -= 1;
                    }
                }
                _ => (), // ignore other events
            }
            buffer.clear();
        }
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
    /// let search_path = "a/b/c";
    /// let select_texts = root.select(search_path).iter()
    ///     .map(|n| n.text()).collect::<Vec<_>>();
    ///
    /// assert_eq!(vec!["test 1", "", "test 2", "test 3"], select_texts);
    /// ```
    pub fn select<'a, 'b, X: Into<XPath<'b>>>(&'a self, path: X) -> Vec<&'a Node>
    {
        // TODO: use impl Trait once stabilized

        let xpath = path.into();
        if xpath.is_empty() {
            Vec::new()
        } else {
            let idx_start = if xpath.inner[0] == "/" {
                if self.name != "/" {
                    // only the root node can return something
                    return Vec::new();
                } else {
                    1
                }
            } else {
                0
            };
            let mut vec = Vec::new();
            self.extend_select_all(&mut vec, idx_start, &xpath.inner);
            vec
        }
    }

    fn extend_select_all<'a>(&'a self, vec: &mut Vec<&'a Node>, idx: usize, paths: &[&str]) {

        // TODO: implement more XPath syntaxes

        let n = paths[idx];
        if idx == paths.len() - 1 {
            if n.is_empty() || n == "." {
                vec.extend(&self.children);
            } else {
                vec.extend(self.children.iter().filter(|c| c.name == n));
            }
        } else {
            if n.is_empty() || n == "." {
                for ch in self.children.iter() {
                    ch.extend_select_all(vec, idx + 1, paths);
                }
            } else {
                for ch in self.children.iter().filter(|c| c.name == n) {
                    ch.extend_select_all(vec, idx + 1, paths);
                }
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
    inner: Vec<&'a str>,
}

impl<'a> XPath<'a> {
    /// Is XPath empty
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<'a> From<&'a str> for XPath<'a> {
    fn from(s: &'a str) -> XPath<'a> {
        let s = s.trim();
        let (mut inner, s) = if s.starts_with('/') {
            (vec!["/"], &s[1..])
        } else {
            (vec![], s)
        };
        inner.extend(s.split('/').map(|s| s.trim()));
        XPath { inner: inner }
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
        <ignore> test </ignore>
        <ignore><this><too /></this></ignore>
        <b>
            <c>test 3</c>
        </b>
    </a>
    "#;

    let root = Node::root(::std::io::Cursor::new(data)).unwrap();
    let select_texts = root.select("a/b/c").iter().map(|n| n.text()).collect::<Vec<_>>();
    assert_eq!(vec!["test 1", "", "test 2", "test 3"], select_texts);
    let select_texts = root.select("/a/b/c").iter().map(|n| n.text()).collect::<Vec<_>>();
    assert_eq!(vec!["test 1", "", "test 2", "test 3"], select_texts);
}

#[test]
fn test_from_xpath() {
    let data = r#"
    <a>
        <b>
            <c>test 1</c>
            <c att1='test att'/>
            <c>test 2</c>
        </b>
        <ignore> test </ignore>
        <ignore><this><too /></this></ignore>
        <b>
            <c>test 3</c>
        </b>
    </a>
    "#;

    let select_nodes = Node::from_xpath(::std::io::Cursor::new(data), "a/b/c").unwrap();
    let select_texts = select_nodes.iter().map(|n| n.text()).collect::<Vec<_>>();
    assert_eq!(vec!["test 1", "", "test 2", "test 3"], select_texts);

    let select_nodes = Node::from_xpath(::std::io::Cursor::new(data), "/a/b/c").unwrap();
    let select_texts = select_nodes.iter().map(|n| n.text()).collect::<Vec<_>>();
    assert_eq!(vec!["test 1", "", "test 2", "test 3"], select_texts);
}

//! A module to manage DOM documents

use std::io::{self, BufRead};

use escape::unescape;
use events::{Event, BytesStart};
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
    /// let select_texts = root.select_all(search_path).iter()
    ///     .map(|n| n.text()).collect::<Vec<_>>();
    ///
    /// assert_eq!(vec!["test 1", "", "test 2", "test 3"], select_texts);
    /// ```
    pub fn select_all<'a, 'b, X: Into<XPath<'b>>>(&'a self, path: X) -> Vec<&'a Node>
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
    let select_texts = root.select_all("b/c").iter().map(|n| n.text()).collect::<Vec<_>>();

    assert_eq!(vec!["test 1", "", "test 2", "test 3"], select_texts);
}

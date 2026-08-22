//! Buffered pretty-printer that matches lxml/libxml2 formatting behavior.
//!
//! Unlike the streaming [`Writer`], this writer buffers events to make layout
//! decisions based on complete element content. Elements are indented only if
//! they contain no text nodes.

use std::io::{self, Write};

use crate::events::{
    BytesCData, BytesComment, BytesDecl, BytesPI, BytesRef, BytesStart, BytesText, Event,
};
use crate::writer::{Config, Indentation};

/// A node in the buffered document tree for pretty-printing.
#[derive(Debug, Clone)]
enum Node {
    /// Start tag with name, attributes, and children
    Element {
        start: BytesStart<'static>,
        children: Vec<Node>,
    },
    /// Empty/self-closing tag
    Empty(BytesStart<'static>),
    /// Text content
    Text(BytesText<'static>),
    /// Comment
    Comment(BytesComment<'static>),
    /// Processing instruction
    PI(BytesPI<'static>),
    /// CDATA section
    CData(BytesCData<'static>),
    /// Document type declaration
    DocType(BytesText<'static>),
    /// General entity reference
    GeneralRef(BytesRef<'static>),
}

/// Document root that can contain multiple top-level nodes.
#[derive(Debug)]
struct Document {
    /// Optional XML declaration
    decl: Option<BytesDecl<'static>>,
    /// Top-level nodes (elements, comments, PIs, etc.)
    nodes: Vec<Node>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            decl: None,
            nodes: Vec::new(),
        }
    }
}

/// A buffered XML writer that produces pretty-printed output matching lxml/libxml2 behavior.
///
/// Unlike the streaming [`Writer`], this writer buffers events into a tree structure
/// before writing, allowing it to make layout decisions based on complete element content.
/// The key formatting rule: **an element is indented only if none of its direct children
/// is a text node** (Text, CData, or GeneralRef).
///
/// # Layout Rules
///
/// - **Elements with text children** → fully inline
/// - **Elements with only block children** → indented
///
/// # Examples
///
/// ```
/// use quick_xml::events::{BytesComment, BytesEnd, BytesStart, BytesText, Event};
/// use quick_xml::writer::PrettyWriter;
///
/// let mut writer = PrettyWriter::new(Vec::new());
///
/// // Element-only content gets indented
/// writer.write_event(Event::Start(BytesStart::new("root"))).unwrap();
/// writer.write_event(Event::Empty(BytesStart::new("child"))).unwrap();
/// writer.write_event(Event::End(BytesEnd::new("root"))).unwrap();
///
/// let result = writer.into_inner().unwrap();
/// assert_eq!(
///     std::str::from_utf8(&result).unwrap(),
///     "<root>\n  <child/>\n</root>"
/// );
/// ```
///
/// ```
/// use quick_xml::events::{BytesComment, BytesEnd, BytesStart, BytesText, Event};
/// use quick_xml::writer::PrettyWriter;
///
/// let mut writer = PrettyWriter::new(Vec::new());
///
/// // Mixed content (text + elements) stays inline
/// writer.write_event(Event::Start(BytesStart::new("p"))).unwrap();
/// writer.write_event(Event::Text(BytesText::new("Hello "))).unwrap();
/// writer.write_event(Event::Empty(BytesStart::new("br"))).unwrap();
/// writer.write_event(Event::Text(BytesText::new("world"))).unwrap();
/// writer.write_event(Event::End(BytesEnd::new("p"))).unwrap();
///
/// let result = writer.into_inner().unwrap();
/// assert_eq!(
///     std::str::from_utf8(&result).unwrap(),
///     "<p>Hello <br/>world</p>"
/// );
/// ```
///
/// # When to Use
///
/// Use `PrettyWriter` when:
/// - You need content-preserving pretty-printing (text unchanged)
/// - You're formatting documents for human readability
/// - Working with documents under ~10MB
/// - Building tools, debuggers, or formatters
///
/// Use the streaming [`Writer`] when:
/// - Memory is constrained
/// - Processing large documents (>10MB)
/// - Speed is critical
/// - Best-effort formatting is acceptable
///
/// # Performance
///
/// This writer buffers the entire document in memory (~2-3x the raw event size)
/// and processes it in two passes, making it slower than the streaming [`Writer`].
///
/// # Comparison with Streaming Writer
///
/// ```
/// use quick_xml::events::{BytesComment, BytesEnd, BytesStart, BytesText, Event};
/// use quick_xml::writer::{PrettyWriter, Writer};
///
/// // Streaming Writer (best-effort, may alter text content)
/// let mut streaming = Writer::new_with_indent(Vec::new(), ' ', 2);
/// streaming.write_event(Event::Start(BytesStart::new("p"))).unwrap();
/// streaming.write_event(Event::Comment(BytesComment::new("comment").unwrap())).unwrap();
/// streaming.write_event(Event::Text(BytesText::new("text"))).unwrap();
/// streaming.write_event(Event::End(BytesEnd::new("p"))).unwrap();
/// let result = streaming.into_inner();
/// // Output: <p>\n  <!--comment-->text\n</p>
/// // Text content changed from "text" to "\n  text\n"
///
/// // PrettyWriter (content-preserving)
/// let mut pretty = PrettyWriter::new(Vec::new());
/// pretty.write_event(Event::Start(BytesStart::new("p"))).unwrap();
/// pretty.write_event(Event::Comment(BytesComment::new("comment").unwrap())).unwrap();
/// pretty.write_event(Event::Text(BytesText::new("text"))).unwrap();
/// pretty.write_event(Event::End(BytesEnd::new("p"))).unwrap();
/// let result = pretty.into_inner().unwrap();
/// assert_eq!(std::str::from_utf8(&result).unwrap(), "<p><!--comment-->text</p>");
/// // Text preserved exactly as "text", element stays inline
/// ```
///
/// [`Writer`]: crate::writer::Writer
#[derive(Debug)]
pub struct PrettyWriter<W> {
    /// The underlying writer for output
    writer: W,
    /// Indentation configuration
    indent: Indentation,
    /// Indent character for creating new Indentation instances
    indent_char: char,
    /// Indent size for creating new Indentation instances
    indent_size: usize,
    /// Writer configuration (space before slash, etc.)
    config: Config,
    /// Buffered document tree
    document: Document,
    /// Stack for building the tree during event collection
    element_stack: Vec<(BytesStart<'static>, Vec<Node>)>,
}

impl<W> PrettyWriter<W> {
    /// Creates a new `PrettyWriter` with 2-space indentation.
    ///
    /// # Example
    ///
    /// ```
    /// use quick_xml::writer::PrettyWriter;
    ///
    /// let writer = PrettyWriter::new(Vec::<u8>::new());
    /// ```
    pub fn new(writer: W) -> Self {
        Self::new_with_indent(writer, ' ', 2)
    }

    /// Creates a new `PrettyWriter` with custom indentation.
    ///
    /// # Example
    ///
    /// ```
    /// use quick_xml::writer::PrettyWriter;
    ///
    /// // Use tabs with size 1
    /// let writer = PrettyWriter::new_with_indent(Vec::<u8>::new(), '\t', 1);
    /// ```
    pub fn new_with_indent(writer: W, indent_char: char, indent_size: usize) -> Self {
        Self {
            writer,
            indent: Indentation::new(indent_char, indent_size),
            indent_char,
            indent_size,
            config: Config::default(),
            document: Document::default(),
            element_stack: Vec::new(),
        }
    }

    /// Returns a reference to the writer configuration.
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// Returns a mutable reference to the writer configuration.
    ///
    /// # Example
    ///
    /// ```
    /// use quick_xml::writer::PrettyWriter;
    ///
    /// let mut writer = PrettyWriter::new(Vec::<u8>::new());
    /// writer.config_mut().add_space_before_slash_in_empty_elements = true;
    /// ```
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Get a reference to the underlying writer.
    pub const fn get_ref(&self) -> &W {
        &self.writer
    }

    /// Get a mutable reference to the underlying writer.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.writer
    }
}

impl<W: Write> PrettyWriter<W> {
    /// Buffers an event for later formatting.
    ///
    /// Events are collected into a tree structure. Call [`flush()`](Self::flush)
    /// or [`into_inner()`](Self::into_inner) to trigger pretty-printed output.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - An `End` event is encountered without a matching `Start`
    ///
    /// # Example
    ///
    /// ```
    /// use quick_xml::events::{BytesEnd, BytesStart, Event};
    /// use quick_xml::writer::PrettyWriter;
    ///
    /// let mut writer = PrettyWriter::new(Vec::<u8>::new());
    /// writer.write_event(Event::Start(BytesStart::new("root"))).unwrap();
    /// writer.write_event(Event::End(BytesEnd::new("root"))).unwrap();
    /// writer.flush().unwrap();
    /// ```
    pub fn write_event<'a, E: Into<Event<'a>>>(&mut self, event: E) -> io::Result<()> {
        let event = event.into().into_owned(); // Convert to 'static

        match event {
            Event::Decl(d) => {
                // Store declaration at document level
                self.document.decl = Some(d);
            }
            Event::Start(e) => {
                // Push onto stack to collect children
                self.element_stack.push((e, Vec::new()));
            }
            Event::End(_) => {
                // Pop stack and create Element node
                if let Some((start, children)) = self.element_stack.pop() {
                    let node = Node::Element { start, children };
                    self.add_node(node)?;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "End tag without matching start",
                    ));
                }
            }
            Event::Empty(e) => {
                self.add_node(Node::Empty(e))?;
            }
            Event::Text(t) => {
                self.add_node(Node::Text(t))?;
            }
            Event::Comment(c) => {
                self.add_node(Node::Comment(c))?;
            }
            Event::CData(c) => {
                self.add_node(Node::CData(c))?;
            }
            Event::PI(p) => {
                self.add_node(Node::PI(p))?;
            }
            Event::DocType(d) => {
                self.add_node(Node::DocType(d))?;
            }
            Event::GeneralRef(r) => {
                self.add_node(Node::GeneralRef(r))?;
            }
            Event::Eof => {
                // Ignored - flush handles output
            }
        }
        Ok(())
    }

    /// Adds a node to either the current element's children or the document root.
    fn add_node(&mut self, node: Node) -> io::Result<()> {
        if let Some((_, children)) = self.element_stack.last_mut() {
            // Add to current element's children
            children.push(node);
        } else {
            // Add to document root
            self.document.nodes.push(node);
        }
        Ok(())
    }

    /// Flushes buffered content to the underlying writer with pretty-printing applied.
    ///
    /// This method can be called multiple times. After the first call, subsequent
    /// calls will only write events that were added since the last flush.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - There are unclosed elements (mismatched tags)
    /// - Writing to the underlying writer fails
    ///
    /// # Example
    ///
    /// ```
    /// use quick_xml::events::{BytesEnd, BytesStart, Event};
    /// use quick_xml::writer::PrettyWriter;
    ///
    /// let mut writer = PrettyWriter::new(Vec::<u8>::new());
    /// writer.write_event(Event::Start(BytesStart::new("root"))).unwrap();
    /// writer.write_event(Event::End(BytesEnd::new("root"))).unwrap();
    /// writer.flush().unwrap();
    ///
    /// // Can add more events and flush again
    /// writer.write_event(Event::Start(BytesStart::new("another"))).unwrap();
    /// writer.write_event(Event::End(BytesEnd::new("another"))).unwrap();
    /// writer.flush().unwrap();
    /// ```
    pub fn flush(&mut self) -> io::Result<()> {
        // Check for unclosed elements
        if !self.element_stack.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Unclosed elements in document: {} element(s) not closed",
                    self.element_stack.len()
                ),
            ));
        }

        // Write declaration if present
        if let Some(ref decl) = self.document.decl {
            self.writer.write_all(b"<?")?;
            self.writer.write_all(decl.as_bytes())?;
            self.writer.write_all(b"?>")?;
        }

        // Reset indentation state
        self.indent = Indentation::new(self.indent_char, self.indent_size);

        // Traverse and write each top-level node
        // Top-level nodes don't need indentation (they're at column 0)
        let node_count = self.document.nodes.len();
        for i in 0..node_count {
            self.write_node_by_index(i, false)?;
        }

        // Clear the document after flushing so multiple flush() calls work
        self.document.nodes.clear();
        self.document.decl = None;

        Ok(())
    }

    /// Writes a node from the document by index.
    fn write_node_by_index(&mut self, index: usize, needs_indent: bool) -> io::Result<()> {
        // Clone the node to avoid borrow checker issues
        let node = self.document.nodes[index].clone();
        self.write_node(&node, needs_indent)
    }

    /// Writes a node with pretty-printing, deciding layout based on content.
    fn write_node(&mut self, node: &Node, needs_indent: bool) -> io::Result<()> {
        match node {
            Node::Element { start, children } => {
                // Decide layout: inline if any direct child is Text/CData/GeneralRef
                let has_text_child = children.iter().any(|child| {
                    matches!(child, Node::Text(_) | Node::CData(_) | Node::GeneralRef(_))
                });

                // Add newline + indent if parent requested it
                if needs_indent {
                    self.write_newline_indent()?;
                }

                // Write start tag
                self.write_start_tag(start)?;

                if has_text_child {
                    // Inline mode: write children without indentation
                    for child in children {
                        self.write_node(child, false)?;
                    }
                } else if children.is_empty() {
                    // Empty element: keep closing tag on same line
                    // (no semantic reason to split <a></a> across lines)
                } else {
                    // Block mode: indent children and closing tag
                    self.indent.grow();
                    for child in children {
                        self.write_node(child, true)?;
                    }
                    self.indent.shrink();
                    self.write_newline_indent()?;
                }

                // Write end tag
                self.writer.write_all(b"</")?;
                self.writer.write_all(start.name().as_ref().as_bytes())?;
                self.writer.write_all(b">")?;
            }
            Node::Empty(e) => {
                if needs_indent {
                    self.write_newline_indent()?;
                }
                // Write opening bracket and tag content
                self.writer.write_all(b"<")?;
                self.writer.write_all(e.as_bytes())?;
                // Write closing with optional space before slash
                if self.config.add_space_before_slash_in_empty_elements {
                    self.writer.write_all(b" />")?;
                } else {
                    self.writer.write_all(b"/>")?;
                }
            }
            Node::Text(t) => {
                self.writer.write_all(t.as_bytes())?;
            }
            Node::Comment(c) => {
                if needs_indent {
                    self.write_newline_indent()?;
                }
                self.writer.write_all(b"<!--")?;
                self.writer.write_all(c.as_bytes())?;
                self.writer.write_all(b"-->")?;
            }
            Node::PI(p) => {
                if needs_indent {
                    self.write_newline_indent()?;
                }
                self.writer.write_all(b"<?")?;
                self.writer.write_all(p.as_bytes())?;
                self.writer.write_all(b"?>")?;
            }
            Node::CData(c) => {
                self.writer.write_all(b"<![CDATA[")?;
                self.writer.write_all(c.as_bytes())?;
                self.writer.write_all(b"]]>")?;
            }
            Node::DocType(d) => {
                if needs_indent {
                    self.write_newline_indent()?;
                }
                self.writer.write_all(b"<!DOCTYPE ")?;
                self.writer.write_all(d.as_bytes())?;
                self.writer.write_all(b">")?;
            }
            Node::GeneralRef(r) => {
                self.writer.write_all(b"&")?;
                self.writer.write_all(r.as_bytes())?;
                self.writer.write_all(b";")?;
            }
        }
        Ok(())
    }

    /// Writes a newline followed by the current indentation.
    fn write_newline_indent(&mut self) -> io::Result<()> {
        self.writer.write_all(b"\n")?;
        self.writer.write_all(self.indent.current().as_bytes())?;
        Ok(())
    }

    /// Writes the start tag with name and attributes.
    fn write_start_tag(&mut self, start: &BytesStart<'static>) -> io::Result<()> {
        self.writer.write_all(b"<")?;
        self.writer.write_all(start.as_bytes())?;
        self.writer.write_all(b">")?;
        Ok(())
    }

    /// Consumes the writer, flushes buffered content, and returns the underlying writer.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - There are unclosed elements (mismatched tags)
    /// - Writing to the underlying writer fails
    ///
    /// # Example
    ///
    /// ```
    /// use quick_xml::events::{BytesEnd, BytesStart, Event};
    /// use quick_xml::writer::PrettyWriter;
    ///
    /// let mut writer = PrettyWriter::new(Vec::<u8>::new());
    /// writer.write_event(Event::Start(BytesStart::new("root"))).unwrap();
    /// writer.write_event(Event::End(BytesEnd::new("root"))).unwrap();
    ///
    /// let result = writer.into_inner().unwrap();
    /// assert_eq!(std::str::from_utf8(&result).unwrap(), "<root></root>");
    /// ```
    pub fn into_inner(mut self) -> io::Result<W> {
        self.flush()?;
        Ok(self.writer)
    }
}

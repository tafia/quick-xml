//! A module to handle `Reader`

pub mod attributes;
pub mod namespace;
pub mod bytes;
pub mod old;

use std::io::BufRead;
use error::ResultPos;
use reader::bytes::{BytesReader, BytesEvent};

pub use reader::old::XmlReader;

/// A wrapper over `BytesReader` with its own buffer
pub struct Reader<'a, B: BufRead> {
    buffer: &'a mut Vec<u8>,
    inner: BytesReader<B>,
}

impl<'a, B: BufRead> Reader<'a, B> {
    /// Creates a Reader from a generic BufReader
    pub fn from_reader(reader: B, buffer: &'a mut Vec<u8>) -> Reader<'a, B> {
        Reader {
            buffer: buffer,
            inner: BytesReader::from_reader(reader),
        }
    }

//     /// Converts into a `XmlnsReader` iterator
//     pub fn namespaced(self) -> XmlnsReader<B> {
//         XmlnsReader::new(self)
//     }

    /// Change expand_empty_elements default behaviour (true per default)
    ///
    /// When set to true, all `Empty` events are expanded into an `Open` event
    /// followed by a `Close` Event.
    pub fn expand_empty_elements(&mut self, val: bool) -> &mut Reader<'a, B> {
        self.inner.expand_empty_elements(val);
        self
    }

    /// Change trim_text default behaviour (false per default)
    ///
    /// When set to true, all Text events are trimed.
    /// If they are empty, no event if pushed
    pub fn trim_text(&mut self, val: bool) -> &mut Reader<'a, B> {
        self.inner.trim_text(val);
        self
    }

    /// Change default with_check (true per default)
    ///
    /// When set to true, it won't check if End node match last Start node.
    /// If the xml is known to be sane (already processed etc ...)
    /// this saves extra time
    pub fn check_end_names(&mut self, val: bool) -> &mut Reader<'a, B> {
        self.inner.check_end_names(val);
        self
    }

    /// Change default check_comment (false per default)
    ///
    /// When set to true, every Comment event will be checked for not containing `--`
    /// Most of the time we don't want comments at all so we don't really care about
    /// comment correctness, thus default value is false for performance reason
    pub fn check_comments(&mut self, val: bool) -> &mut Reader<'a, B> {
        self.inner.check_comments(val);
        self
    }

    /// Reads until end element is found
    ///
    /// Manages nested cases where parent and child elements have the same name
    pub fn read_to_end<K: AsRef<[u8]>>(&mut self, end: K) -> ResultPos<()> {
        self.buffer.clear();
        self.inner.read_to_end(end, self.buffer)
    }

    /// Reads next event, if `Event::Text` or `Event::End`,
    /// then returns a `String`, else returns an error
    pub fn read_text<K: AsRef<[u8]>>(&mut self, end: K) -> ResultPos<String> {
        self.buffer.clear();
        self.inner.read_text(end, self.buffer)
    }

    /// Reads next event, if `Event::Text` or `Event::End`,
    /// then returns an unescaped `String`, else returns an error
    ///
    /// # Examples
    /// 
    /// ```
    /// use quick_xml::{Reader, Event};
    ///
    /// let mut xml = Reader::from_reader(b"<a>&lt;b&gt;</a>" as &[u8]).trim_text(true);
    /// match xml.next() {
    ///     Some(Ok(Event::Start(ref e))) => {
    ///         assert_eq!(&xml.read_text_unescaped(e.name()).unwrap(), "<b>");
    ///     },
    ///     e => panic!("Expecting Start(a), found {:?}", e),
    /// }
    /// ```
    pub fn read_text_unescaped<K: AsRef<[u8]>>(&mut self, end: K) -> ResultPos<String> {
        self.buffer.clear();
        self.inner.read_text_unescaped(end, self.buffer)
    }

    /// Gets the current BufRead position
    /// Useful when debugging errors
    pub fn buffer_position(&self) -> usize {
        self.inner.buffer_position()
    }

    /// Reads the next event
    #[inline]
    pub fn read_event(&mut self) -> ResultPos<BytesEvent> {
        self.buffer.clear();
        self.inner.read_event(self.buffer)
    }
}

// /// Iterator on xml returning `Event`s
// impl<'a, B: BufRead> Iterator for Reader<'a, B> {
//     type Item = ResultPos<BytesEvent<'a>>;
// 
//     fn next<'b>(&'b mut self) -> Option<Self::Item> {
//         self.buffer.clear();
//         self.inner.next_event(self.buffer)
//     }
// }

//! Reporting line numbers of events and errors while streaming a document.
//!
//! The `Reader` only tracks a byte offset (`buffer_position()` and
//! `error_position()`). When the whole document is in memory you can count the
//! newlines before that offset yourself (see the documentation of
//! `Reader::into_inner`). When streaming from a file or a socket the bytes
//! before the offset are already gone, so this example counts them as they pass
//! by instead: `LineCounter` is a `BufRead` that counts the `\n` bytes consumed
//! from it, and the `Reader` is built on top of it.
//!
//! The `Reader` consumes exactly the bytes of each event it returns, so the
//! counter, read *before* the next call to `read_event_into`, gives the line
//! where the next event (or the markup that failed to parse) starts. This holds
//! as long as whitespace is not trimmed (`Config::trim_text`), because trimmed
//! whitespace is skipped inside the same call that returns the following event.
//!
//! Run it with:
//!
//! ```console
//! cargo run --example line_numbers
//! ```

use std::io::{self, BufRead, Read};

use quick_xml::events::Event;
use quick_xml::reader::Reader;

/// A buffered reader that counts lines in the data consumed from it.
struct LineCounter<R> {
    inner: R,
    buf: Box<[u8]>,
    /// Start of the not yet consumed data in `buf`
    pos: usize,
    /// End of the valid data in `buf`
    filled: usize,
    /// Number of the line (starting from 1) of the next byte to be consumed
    line: usize,
}

impl<R: Read> LineCounter<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            buf: vec![0; 8 * 1024].into_boxed_slice(),
            pos: 0,
            filled: 0,
            line: 1,
        }
    }

    fn line(&self) -> usize {
        self.line
    }
}

impl<R: Read> Read for LineCounter<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let available = self.fill_buf()?;
        let len = available.len().min(out.len());
        out[..len].copy_from_slice(&available[..len]);
        self.consume(len);
        Ok(len)
    }
}

impl<R: Read> BufRead for LineCounter<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.pos >= self.filled {
            self.filled = self.inner.read(&mut self.buf)?;
            self.pos = 0;
        }
        Ok(&self.buf[self.pos..self.filled])
    }

    fn consume(&mut self, amt: usize) {
        let end = (self.pos + amt).min(self.filled);
        self.line += memchr::memchr_iter(b'\n', &self.buf[self.pos..end]).count();
        self.pos = end;
    }
}

const XML: &str = r#"<catalog>
  <book id="1">
    <title>The Rust Programming Language</title>
  </book>
  <book id="2">
    <title>Programming Rust</titel>
  </book>
</catalog>"#;

fn main() {
    // Any `Read` works here, for example a `std::fs::File`
    let mut reader = Reader::from_reader(LineCounter::new(XML.as_bytes()));
    let mut buf = Vec::new();

    let mut books = Vec::new();
    let error = loop {
        // Line where the next event starts
        let line = reader.get_ref().line();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == "book" => {
                println!("<book> at line {}", line);
                books.push(line);
            }
            Ok(Event::Eof) => unreachable!("the document contains an error"),
            Ok(_) => {}
            Err(e) => {
                println!("error at line {}: {}", line, e);
                break line;
            }
        }
        buf.clear();
    };

    assert_eq!(books, [2, 5]);
    // `</titel>` does not match `<title>`
    assert_eq!(error, 6);
}

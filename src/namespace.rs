//! Module for managing `XmlnsReader` iterator

use {XmlReader, Event, Element};
use error::ResultPos;
use std::io::BufRead;

#[derive(Clone)]
struct Namespace {
    prefix: Vec<u8>,
    value: Vec<u8>,
    element_name: Vec<u8>,
    level: i32,
}

impl Namespace {
    #[inline]
    fn is_match(&self, name: &[u8]) -> bool {
        let len = self.prefix.len();
        name.len() > len && name[len] == b':' && &name[..len] == &*self.prefix
    }
}

/// `XmlnsReader` iterator which wraps `XmlReader` iterator and
/// adds namespace resolutions
///
/// # Example
///
/// ```
/// use quick_xml::{XmlReader, Event};
/// use quick_xml::namespace::XmlnsReader;
///
/// let xml = r#"<tag1 att1 = "test">
///                 <tag2><!--Test comment-->Test</tag2>
///                 <tag2>Test 2</tag2>
///             </tag1>"#;
/// let mut reader = XmlReader::from(xml).trim_text(true)
///                  .namespaced();
/// let mut count = 0;
/// let mut txt = Vec::new();
/// // need to use `while let` in order to have access to `reader.resolve` 
/// // for attributes namespaces
/// while let Some(r) = reader.next() {
///     match r {
///         // XmlnsReader iterates ResultPos<(Option<&[u8]>, Event)> with 
///         // the Option<&[u8]> being the resolved Namespace, if any
///         Ok((ref n, Event::Start(ref e))) => {
///             match e.name() {
///                 b"tag1" => println!("attributes keys: {:?}",
///                                  e.attributes()
///                                  // use `reader.resolve` to get attribute
///                                  // namespace resolution
///                                  .map(|a| reader.resolve(a.unwrap().0))
///                                  .collect::<Vec<_>>()),
///                 b"tag2" => count += 1,
///                 _ => (),
///             }
///         },
///         Ok((_, Event::Text(e))) => txt.push(e.into_string()),
///         Err((e, pos)) => panic!("{:?} at position {}", e, pos),
///         _ => (),
///     }
/// }
/// ```
#[derive(Clone)]
pub struct XmlnsReader<R: BufRead> {
    reader: XmlReader<R>,
    namespaces: Vec<Namespace>,
}

impl<R: BufRead> XmlnsReader<R> {
    /// Converts a `XmlReader` into a `XmlnsReader` iterator
    #[inline]
    pub fn new(reader: XmlReader<R>) -> XmlnsReader<R> {
        XmlnsReader {
            reader: reader,
            namespaces: Vec::new(),
        }
    }

    /// Resolves a qualified name into (namespace value, local name)
    pub fn resolve<'a, 'b>(&'a self, qname: &'b [u8]) 
        -> (Option<&'a [u8]>, &'b [u8]) 
    {
        match self.namespaces.iter().rev().find(|ref n| n.is_match(qname)) {
            Some(n) => (Some(&n.value), &qname[(n.prefix.len() + 1)..]),
            None => (None, qname),
        }
    }

    fn find_namespace_value(&self, e: &Element) -> Option<Vec<u8>> {
        self.namespaces
            .iter()
            .rev() // iterate in reverse order to find the most recent one
            .find(|ref n| n.is_match(e.name()))
            .map(|ref n| n.value.clone())
    }

    fn pop_empty_namespaces(&mut self) {
        match self.namespaces.iter().rev().position(|n| n.level > 0) {
            Some(0) | None => (),
            Some(p) => {
                let len = self.namespaces.len() - p;
                self.namespaces.truncate(len)
            }
        }
    }

    fn push_new_namespaces(&mut self, e: &Element) {
        // adds new namespaces for attributes starting with 'xmlns:'
        for a in e.attributes().with_checks(false) {
            if let Ok((k, v)) = a {
                if k.len() > 6 && &k[..6] == b"xmlns:" {
                    self.namespaces.push(Namespace {
                        prefix: k[6..].to_vec(),
                        value: v.to_vec(),
                        element_name: e.name().to_vec(),
                        level: 1,
                    });
                }
            } else {
                break;
            }
        }
    }

    fn update_existing_ns_level(&mut self, e: &Element, increment: i32) {
        let name = e.name();
        for n in &mut self.namespaces {
            if name == &*n.element_name {
                n.level += increment;
            }
        }
    }
}

impl<R: BufRead> Iterator for XmlnsReader<R> {
    type Item = ResultPos<(Option<Vec<u8>>, Event)>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.next() {
            Some(Ok(Event::Start(e))) => {
                self.update_existing_ns_level(&e, 1);
                self.push_new_namespaces(&e);
                Some(Ok((self.find_namespace_value(&e), Event::Start(e))))
            }
            Some(Ok(Event::Empty(e))) => {
                Some(Ok((self.find_namespace_value(&e), Event::Empty(e))))
            }
            Some(Ok(Event::End(e))) => {
                self.update_existing_ns_level(&e, -1);
                self.pop_empty_namespaces();
                Some(Ok((self.find_namespace_value(&e), Event::End(e))))
            }
            Some(Ok(e)) => Some(Ok((None, e))),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}

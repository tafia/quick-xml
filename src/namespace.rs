//! Module for managing `XmlnsReader` iterator

use {XmlReader, Event};
use error::ResultPos;
use std::io::BufRead;

#[derive(Clone)]
struct Namespace {
    prefix: Vec<u8>,
    value: Vec<u8>,
    element_name: Vec<u8>,
    level: usize,
}

impl Namespace {
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
}

impl<R: BufRead> Iterator for XmlnsReader<R> {
    type Item = ResultPos<(Option<Vec<u8>>, Event)>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.next() {
            Some(Ok(Event::Start(e))) => {
                // increment existing namespace level if this same element
                {
                    let name = e.name();
                    for n in &mut self.namespaces {
                        if name == &*n.element_name {
                            n.level += 1;
                        }
                    }
                }
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
                // search namespace value
                // iterate in reverse order to find the most recent one
                let namespace = self.namespaces
                    .iter()
                    .rev()
                    .find(|ref n| n.is_match(e.name()))
                    .map(|ref n| n.value.clone());

                Some(Ok((namespace, Event::Start(e))))
            }
            Some(Ok(Event::End(e))) => {
                // decrement levels and remove namespaces with 0 level
                {
                    let name = e.name();
                    for n in &mut self.namespaces {
                        if name == &*n.element_name {
                            n.level -= 1;
                        }
                    }
                }
                match self.namespaces.iter().rev().position(|n| n.level > 0) {
                    Some(0) | None => (),
                    Some(p) => {
                        let len = self.namespaces.len() - p;
                        self.namespaces.truncate(len)
                    }
                }
                let namespace = {
                    let name = e.name();
                    self.namespaces
                        .iter()
                        .rev()
                        .find(|ref n| n.is_match(name))
                        .map(|ref n| n.value.clone())
                };
                Some(Ok((namespace, Event::End(e))))
            }
            Some(Ok(e)) => Some(Ok((None, e))),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}

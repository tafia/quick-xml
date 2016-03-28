//! Module for managing Namespaced iterator

use {XmlReader, Event};
use error::ResultPos;
use std::io::BufRead;

struct Namespace {
    prefix: Vec<u8>,
    value: Vec<u8>,
    element_name: Vec<u8>,
    level: u8,
}

impl Namespace {
    fn is_match(&self, name: &[u8]) -> bool {
        let len = self.prefix.len();
        name.len() > len && 
        name[len] == b':' &&
        &name[..len] == &*self.prefix
    }
}

/// Namespaced iterator which wraps XmlReader iterator and
/// adds namespace resolutions
pub struct Namespaced<R: BufRead> {
    reader: XmlReader<R>,
    namespaces: Vec<Namespace>,
}

impl<R: BufRead> Namespaced<R> {

    /// Converts a `XmlReader` into a `Namespaced` iterator
    pub fn new(reader: XmlReader<R>) -> Namespaced<R> {
        Namespaced {
            reader: reader,
            namespaces: Vec::new(),
        }
    }

    /// Resolves a qualified name into (namespace value, local name)
    pub fn resolve<'a, 'b>(&'a self, qname: &'b[u8]) -> (Option<&'a[u8]>, &'b[u8]) {
        match self.namespaces.iter().rev().find(|ref n| n.is_match(qname)) {
            Some(n) => (Some(&n.value), &qname[(n.prefix.len() + 1)..]),
            None => (None, qname),
        }
    }
}

impl<R: BufRead> Iterator for Namespaced<R> {
    type Item = ResultPos<(Option<Vec<u8>>, Event)>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.next() {
            Some(Ok(Event::Start(e))) => {
                let namespace = {
                    let name = e.name();
                    // increment existing namespace level if this same element
                    for n in self.namespaces.iter_mut() {
                        if name == &*n.element_name {
                            n.level += 1;
                        }
                    }
                    // clone namespace value, if any
                    // iterate in reverse order to find the most recent one
                    self.namespaces.iter().rev()
                        .find(|ref n| n.is_match(name))
                        .map(|ref n| n.value.clone())
                };
                // adds new namespaces for attributes starting with 'xmlns:'
                for a in e.attributes() {
                    if let Ok((k, v)) = a {
                        if k.len() > 6 && &k[..6] == b"xmlns:" {
                            self.namespaces.push(Namespace {
                                prefix: k[6..].to_vec(), 
                                value: v.to_vec(),
                                element_name: e.name().to_vec(),
                                level: 1,
                            });
                        }
                    }
                }
                Some(Ok((namespace, Event::Start(e))))
            }
            Some(Ok(Event::End(e))) => {
                // decrement levels and remove namespaces with 0 level
                let mut to_remove = false;
                {
                    let name = e.name();
                    for n in self.namespaces.iter_mut() {
                        if name == &*n.element_name {
                            n.level -= 1;
                            to_remove |= n.level == 0;
                        }
                    }
                }
                if to_remove {
                    self.namespaces.retain(|n| n.level > 0);
                }
                let namespace = {
                    let name = e.name();
                    self.namespaces.iter().rev()
                        .find(|ref n| n.is_match(name))
                        .map(|ref n| n.value.clone())
                };
                Some(Ok((namespace, Event::End(e))))
            },
            Some(Ok(e)) => Some(Ok((None, e))),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}

//! Module for managing `XmlnsReader` iterator

use {XmlReader, Event, Element};
use error::ResultPos;
use std::io::BufRead;

/// A namespace declaration. Can either bind a namespace to a prefix or define the current default
/// namespace.
#[derive(Clone)]
struct Namespace {
    /// * `Some(prefix)` binds this namespace to `prefix`.
    /// * `None` defines the current default namespace.
    prefix: Option<Vec<u8>>,
    /// The namespace name (the URI) of this namespace declaration.
    ///
    /// The XML standard specifies that an empty namespace value 'removes' a namespace declaration
    /// for the extent of its scope. For prefix declarations that's not very interesting, but it is
    /// vital for default namespace declarations. With `xmlns=""` you can revert back to the default
    /// behaviour of leaving unqualified element names unqualified.
    value: Option<Vec<u8>>,
    /// Level of nesting at which this namespace was declared. The declaring element is included,
    /// i.e., a declaration on the document root has `level = 1`.
    /// This is used to pop the namespace when the element gets closed.
    level: i32,
}

impl Namespace {
    /// Check whether this namespace declaration matches the **qualified element name** name.
    /// Does not take default namespaces into account. The `matches_unqualified_elem` method is
    /// responsible for unqualified element names.
    ///
    /// [W3C Namespaces in XML 1.1 (2006)](http://w3.org/TR/xml-names11/#scoping-defaulting)
    #[inline]
    fn matches_qualified(&self, name: &[u8]) -> bool {
        if let Some(ref prefix) = self.prefix {
            let len = prefix.len();
            name.len() > len && name[len] == b':' && &name[..len] == &prefix[..]
        } else {
            false
        }
    }

    /// A namespace declaration matches unqualified elements if and only if it is a default
    /// namespace declaration (no prefix).
    ///
    /// [W3C Namespaces in XML 1.1 (2006)](http://w3.org/TR/xml-names11/#scoping-defaulting)
    #[inline]
    fn matches_unqualified_elem(&self) -> bool {
        self.prefix.is_none()
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
    /// The number of open tags at the moment. We need to keep track of this to know which namespace
    /// declarations to remove when we encounter an `End` event.
    nesting_level: i32,
    /// For `Empty` events keep the 'scope' of the element on the stack artificially. That way, the
    /// consumer has a chance to use `resolve` in the context of the empty element. We perform the
    /// pop as the first operation in the next `next()` call.
    pending_pop: bool
}

impl<R: BufRead> XmlnsReader<R> {
    /// Converts a `XmlReader` into a `XmlnsReader` iterator
    pub fn new(reader: XmlReader<R>) -> XmlnsReader<R> {
        XmlnsReader {
            reader: reader,
            namespaces: Vec::new(),
            nesting_level: 0,
            pending_pop: false
        }
    }

    /// Resolves a potentially qualified **attribute name** into (namespace name, local name).
    ///
    /// *Qualified* attribute names have the form `prefix:local-name` where the`prefix` is defined
    /// on any containing XML element via `xmlns:prefix="the:namespace:uri"`. The namespace prefix
    /// can be defined on the same element as the attribute in question.
    ///
    /// *Unqualified* attribute names do *not* inherit the current *default namespace*.
    pub fn resolve<'a, 'b>(&'a self, qname: &'b [u8]) 
        -> (Option<&'a [u8]>, &'b [u8]) 
    {
        // Unqualified attributes don't inherit the default namespace. We don't need to search the
        // namespace declaration stack for those.
        if !qname.contains(&b':') {
            return (None, qname)
        }

        match self.namespaces.iter().rev().find(|ref n| n.matches_qualified(qname)) {
            // Found closest matching namespace declaration `n`. The `unwrap` is fine because
            // `is_match_attr` doesn't return default namespace declarations.
            Some(&Namespace { ref prefix, value: Some(ref value), .. }) =>
                (Some(&value[..]), &qname[(prefix.as_ref().unwrap().len() + 1)..]),
            Some(&Namespace { ref prefix, value: None, .. }) =>
                (None, &qname[(prefix.as_ref().unwrap().len() + 1)..]),
            None => (None, qname),
        }
    }

    fn find_namespace_value(&self, e: &Element) -> Option<Vec<u8>> {
        // We pulled the qualified-vs-unqualified check out here so that it doesn't happen for each
        // namespace we are comparing against.
        let element_name = e.name();
        if element_name.contains(&b':') {
            // qualified name
            self.namespaces
                .iter()
                .rev() // iterate in reverse order to find the most recent one
                .find(|ref n| n.matches_qualified(element_name))
                .and_then(|ref n| n.value.as_ref().map(|ns| ns.clone()))
        } else {
            // unqualified name (inherits current default namespace)
            self.namespaces
                .iter()
                .rev() // iterate in reverse order to find the most recent one
                .find(|ref n| n.matches_unqualified_elem())
                .and_then(|ref n| n.value.as_ref().map(|ns| ns.clone()))
        }
    }

    fn pop_empty_namespaces(&mut self) {
        let current_level = self.nesting_level;
        // from the back (most deeply nested scope), look for the first scope that is still valid
        match self.namespaces.iter().rposition(|n| n.level <= current_level) {
            // none of the namespaces are valid, remove all of them
            None => self.namespaces.clear(),
            // drop all namespaces past the last valid namespace
            Some(last_valid_pos) => self.namespaces.truncate(last_valid_pos + 1)
        }
    }

    fn push_new_namespaces(&mut self, e: &Element) {
        // adds new namespaces for attributes starting with 'xmlns:' and for the 'xmlns'
        // (default namespace) attribute.
        for a in e.attributes().with_checks(false) {
            if let Ok((k, v)) = a {
                // Check for 'xmlns:any-prefix' and 'xmlns' at the same time:
                if k.len() >= 5 && &k[..5] == b"xmlns" && (k.len() == 5 || k[5] == b':') {
                    // We use an None prefix as the 'name' for the default namespace.
                    // That saves an allocation compared to an empty namespace name.
                    let prefix = if k.len() == 5 { None } else { Some(k[6..].to_vec()) };
                    let ns_value = if v.len() == 0 { None } else { Some(v.to_vec()) };
                    self.namespaces.push(Namespace {
                        prefix: prefix,
                        value: ns_value,
                        level: self.nesting_level,
                    });
                }
            } else {
                break;
            }
        }
    }
}

impl<R: BufRead> Iterator for XmlnsReader<R> {
    type Item = ResultPos<(Option<Vec<u8>>, Event)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pending_pop {
            self.pending_pop = false;
            self.nesting_level -= 1;
            self.pop_empty_namespaces();
        }
        match self.reader.next() {
            Some(Ok(Event::Start(e))) => {
                self.nesting_level += 1;
                self.push_new_namespaces(&e);
                Some(Ok((self.find_namespace_value(&e), Event::Start(e))))
            }
            Some(Ok(Event::Empty(e))) => {
                // For empty elements we need to 'artificially' keep the namespace scope on the
                // stack until the next `next()` call occurs.
                // Otherwise the caller has no chance to use `resolve` in the context of the
                // namespace declarations that are 'in scope' for the empty element alone.
                // Ex: <img rdf:nodeID="abc" xmlns:rdf="urn:the-rdf-uri" />
                self.nesting_level += 1;
                self.push_new_namespaces(&e);
                // notify next `next()` invocation that it needs to pop this namespace scope
                self.pending_pop = true;
                Some(Ok((self.find_namespace_value(&e), Event::Empty(e))))
            }
            Some(Ok(Event::End(e))) => {
                // need to determine namespace of end element *before* we pop the current
                // namespace scope. If namespace prefixes are shadowed or if default namespaces are
                // defined, it is vital that we resolve the namespace of the end tag in the scope
                // of that tag (not in the outer scope).
                let element_ns = self.find_namespace_value(&e);
                self.nesting_level -= 1;
                self.pop_empty_namespaces();
                Some(Ok((element_ns, Event::End(e))))
                // It could be argued that the 'End' event should also defer the 'pop' operation to
                // the next `next()` call. The end tag still technically belongs to the
                // 'tag scope'. Not sure if that behaviour is intuitive, though.
            }
            Some(Ok(e)) => Some(Ok((None, e))),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}

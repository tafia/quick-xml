//! Module for handling names according to the W3C [Namespaces in XML 1.1 (Second Edition)][spec]
//! specification
//!
//! [spec]: https://www.w3.org/TR/xml-names11

use crate::events::attributes::Attribute;
use crate::events::BytesStart;
use crate::utils::write_byte_string;
use memchr::memchr;
use std::fmt::{self, Debug, Formatter};

/// A [qualified name] of an element or an attribute, including an optional
/// namespace [prefix](Prefix) and a [local name](LocalName).
///
/// [qualified name]: https://www.w3.org/TR/xml-names11/#dt-qualname
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct QName<'a>(pub &'a [u8]);
impl<'a> QName<'a> {
    /// Converts this name to an internal slice representation.
    #[inline(always)]
    pub fn into_inner(self) -> &'a [u8] {
        self.0
    }

    /// Returns the index in the name where prefix ended
    #[inline(always)]
    fn index(&self) -> Option<usize> {
        memchr(b':', self.0)
    }
}
impl<'a> Debug for QName<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "QName(")?;
        write_byte_string(f, self.0)?;
        write!(f, ")")
    }
}
impl<'a> AsRef<[u8]> for QName<'a> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.0
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A [local (unqualified) name] of an element or an attribute, i.e. a name
/// without [prefix](Prefix).
///
/// [local (unqualified) name]: https://www.w3.org/TR/xml-names11/#dt-localname
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct LocalName<'a>(&'a [u8]);
impl<'a> LocalName<'a> {
    /// Converts this name to an internal slice representation.
    #[inline(always)]
    pub fn into_inner(self) -> &'a [u8] {
        self.0
    }
}
impl<'a> Debug for LocalName<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "LocalName(")?;
        write_byte_string(f, self.0)?;
        write!(f, ")")
    }
}
impl<'a> AsRef<[u8]> for LocalName<'a> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.0
    }
}
impl<'a> From<QName<'a>> for LocalName<'a> {
    /// Creates `LocalName` from a [`QName`]
    ///
    /// # Examples
    ///
    /// ```
    /// # use quick_xml::name::{LocalName, QName};
    ///
    /// let local: LocalName = QName(b"unprefixed").into();
    /// assert_eq!(local.as_ref(), b"unprefixed");
    ///
    /// let local: LocalName = QName(b"some:prefix").into();
    /// assert_eq!(local.as_ref(), b"prefix");
    /// ```
    #[inline]
    fn from(name: QName<'a>) -> Self {
        Self(name.index().map_or(&name.0, |i| &name.0[i + 1..]))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A [namespace prefix] part of the [qualified name](QName) of an element tag
/// or an attribute: a `prefix` in `<prefix:local-element-name>` or
/// `prefix:local-attribute-name="attribute value"`.
///
/// [namespace prefix]: https://www.w3.org/TR/xml-names11/#dt-prefix
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Prefix<'a>(&'a [u8]);
impl<'a> Prefix<'a> {
    /// Extracts internal slice
    #[inline(always)]
    pub fn into_inner(self) -> &'a [u8] {
        self.0
    }
}
impl<'a> Debug for Prefix<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Prefix(")?;
        write_byte_string(f, self.0)?;
        write!(f, ")")
    }
}
impl<'a> AsRef<[u8]> for Prefix<'a> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.0
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A [namespace name] that is declared in a `xmlns[:prefix]="namespace name"`.
///
/// [namespace name]: https://www.w3.org/TR/xml-names11/#dt-NSName
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Namespace<'a>(pub &'a [u8]);
impl<'a> Namespace<'a> {
    /// Converts this namespace to an internal slice representation.
    ///
    /// This is [non-normalized] attribute value, i.e. any entity references is
    /// not expanded and space characters are not removed. This means, that
    /// different byte slices, returned from this method, can represent the same
    /// namespace and would be treated by parser as identical.
    ///
    /// For example, if the entity **eacute** has been defined to be **é**,
    /// the empty tags below all contain namespace declarations binding the
    /// prefix `p` to the same [IRI reference], `http://example.org/rosé`.
    ///
    /// ```xml
    /// <p:foo xmlns:p="http://example.org/rosé" />
    /// <p:foo xmlns:p="http://example.org/ros&#xe9;" />
    /// <p:foo xmlns:p="http://example.org/ros&#xE9;" />
    /// <p:foo xmlns:p="http://example.org/ros&#233;" />
    /// <p:foo xmlns:p="http://example.org/ros&eacute;" />
    /// ```
    ///
    /// This is because XML entity references are expanded during attribute value
    /// normalization.
    ///
    /// [non-normalized]: https://www.w3.org/TR/REC-xml/#AVNormalize
    /// [IRI reference]: https://datatracker.ietf.org/doc/html/rfc3987
    #[inline(always)]
    pub fn into_inner(self) -> &'a [u8] {
        self.0
    }
    //TODO: implement value normalization and use it when comparing namespaces
}
impl<'a> Debug for Namespace<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Namespace(")?;
        write_byte_string(f, self.0)?;
        write!(f, ")")
    }
}
impl<'a> AsRef<[u8]> for Namespace<'a> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.0
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// An entry that contains index into the buffer with namespace bindings.
///
/// Defines a mapping from *[namespace prefix]* to *[namespace name]*.
/// If prefix is empty, defines a *default namespace* binding that applies to
/// unprefixed element names (unprefixed attribute names do not bind to any
/// namespace and they processing is dependent on the element in which their
/// defined).
///
/// [namespace prefix]: https://www.w3.org/TR/xml-names11/#dt-prefix
/// [namespace name]: https://www.w3.org/TR/xml-names11/#dt-NSName
#[derive(Debug, Clone)]
struct NamespaceEntry {
    /// Index of the namespace in the buffer
    start: usize,
    /// Length of the prefix
    /// * if greater than zero, then binds this namespace to the slice
    ///   `[start..start + prefix_len]` in the buffer.
    /// * else defines the current default namespace.
    prefix_len: usize,
    /// The length of a namespace name (the URI) of this namespace declaration.
    /// Name started just after prefix and extend for `value_len` bytes.
    ///
    /// The XML standard [specifies] that an empty namespace value 'removes' a namespace declaration
    /// for the extent of its scope. For prefix declarations that's not very interesting, but it is
    /// vital for default namespace declarations. With `xmlns=""` you can revert back to the default
    /// behaviour of leaving unqualified element names unqualified.
    ///
    /// [specifies]: https://www.w3.org/TR/xml-names11/#scoping
    value_len: usize,
    /// Level of nesting at which this namespace was declared. The declaring element is included,
    /// i.e., a declaration on the document root has `level = 1`.
    /// This is used to pop the namespace when the element gets closed.
    level: i32,
}

impl NamespaceEntry {
    /// Gets the namespace name (the URI) slice out of namespace buffer
    ///
    /// Returns `None` if namespace for this prefix was explicitly removed from
    /// scope, using `xmlns[:prefix]=""`
    #[inline]
    fn namespace<'b>(&self, buffer: &'b [u8]) -> Option<Namespace<'b>> {
        if self.value_len == 0 {
            None
        } else {
            let start = self.start + self.prefix_len;
            Some(Namespace(&buffer[start..start + self.value_len]))
        }
    }

    /// Check if the namespace matches the potentially qualified name
    #[inline]
    fn is_match(&self, buffer: &[u8], name: QName) -> bool {
        let qname = name.into_inner();
        if self.prefix_len == 0 {
            !qname.contains(&b':')
        } else {
            qname.get(self.prefix_len).map_or(false, |n| *n == b':')
                && qname.starts_with(&buffer[self.start..self.start + self.prefix_len])
        }
    }
}

/// A namespace management buffer.
///
/// Holds all internal logic to push/pop namespaces with their levels.
#[derive(Debug, Default, Clone)]
pub(crate) struct NamespaceResolver {
    /// A stack of namespace bindings to prefixes that currently in scope
    bindings: Vec<NamespaceEntry>,
    /// The number of open tags at the moment. We need to keep track of this to know which namespace
    /// declarations to remove when we encounter an `End` event.
    nesting_level: i32,
}

impl NamespaceResolver {
    /// Begins a new scope and add to it all [namespace bindings] that found in
    /// the specified start element.
    ///
    /// [namespace binding]: https://www.w3.org/TR/xml-names11/#dt-NSDecl
    pub fn push(&mut self, start: &BytesStart, buffer: &mut Vec<u8>) {
        self.nesting_level += 1;
        let level = self.nesting_level;
        // adds new namespaces for attributes starting with 'xmlns:' and for the 'xmlns'
        // (default namespace) attribute.
        for a in start.attributes().with_checks(false) {
            if let Ok(Attribute { key: k, value: v }) = a {
                if k.starts_with(b"xmlns") {
                    match k.get(5) {
                        None => {
                            let start = buffer.len();
                            buffer.extend_from_slice(&*v);
                            self.bindings.push(NamespaceEntry {
                                start,
                                prefix_len: 0,
                                value_len: v.len(),
                                level,
                            });
                        }
                        Some(&b':') => {
                            let start = buffer.len();
                            buffer.extend_from_slice(&k[6..]);
                            buffer.extend_from_slice(&*v);
                            self.bindings.push(NamespaceEntry {
                                start,
                                prefix_len: k.len() - 6,
                                value_len: v.len(),
                                level,
                            });
                        }
                        _ => break,
                    }
                }
            } else {
                break;
            }
        }
    }

    /// Ends a top-most scope by popping all [namespace binding], that was added by
    /// last call to [`Self::push()`].
    ///
    /// [namespace binding]: https://www.w3.org/TR/xml-names11/#dt-NSDecl
    pub fn pop(&mut self, buffer: &mut Vec<u8>) {
        self.nesting_level -= 1;
        let current_level = self.nesting_level;
        // from the back (most deeply nested scope), look for the first scope that is still valid
        match self.bindings.iter().rposition(|n| n.level <= current_level) {
            // none of the namespaces are valid, remove all of them
            None => {
                buffer.clear();
                self.bindings.clear();
            }
            // drop all namespaces past the last valid namespace
            Some(last_valid_pos) => {
                if let Some(len) = self.bindings.get(last_valid_pos + 1).map(|n| n.start) {
                    buffer.truncate(len);
                    self.bindings.truncate(last_valid_pos + 1);
                }
            }
        }
    }

    /// Resolves a potentially qualified **element name** or **attribute name**
    /// into (namespace name, local name).
    ///
    /// *Qualified* names have the form `prefix:local-name` where the `prefix` is
    /// defined on any containing XML element via `xmlns:prefix="the:namespace:uri"`.
    /// The namespace prefix can be defined on the same element as the element or
    /// attribute in question.
    ///
    /// *Unqualified* attribute names do *not* inherit the current *default namespace*.
    ///
    /// # Lifetimes
    ///
    /// - `'n`: lifetime of an attribute or an element name
    /// - `'ns`: lifetime of a namespaces buffer, where all found namespaces are stored
    #[inline]
    pub fn resolve<'n, 'ns>(
        &self,
        name: QName<'n>,
        buffer: &'ns [u8],
        use_default: bool,
    ) -> (Option<Namespace<'ns>>, LocalName<'n>) {
        let qname = name.into_inner();
        self.bindings
            .iter()
            .rfind(|n| n.is_match(buffer, name))
            .map_or((None, LocalName(qname)), |n| {
                let len = n.prefix_len;
                if len > 0 {
                    (n.namespace(buffer), LocalName(&qname[len + 1..]))
                } else if use_default {
                    (n.namespace(buffer), LocalName(qname))
                } else {
                    (None, LocalName(qname))
                }
            })
    }

    /// Finds a [namespace name] for a given qualified **element name**, borrow
    /// it from the specified buffer.
    ///
    /// Returns `None`, if:
    /// - name is unqualified
    /// - prefix not found in the current scope
    /// - prefix was [unbound] using `xmlns:prefix=""`
    ///
    /// # Lifetimes
    ///
    /// - `'ns`: lifetime of a namespaces buffer, where all found namespaces are stored
    ///
    /// [namespace name]: https://www.w3.org/TR/xml-names11/#dt-NSName
    /// [unbound]: https://www.w3.org/TR/xml-names11/#scoping
    #[inline]
    pub fn find<'ns>(&self, element_name: QName, buffer: &'ns [u8]) -> Option<Namespace<'ns>> {
        self.bindings
            .iter()
            .rfind(|n| n.is_match(buffer, element_name))
            .and_then(|n| n.namespace(buffer))
    }
}

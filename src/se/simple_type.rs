//! Contains Serde `Serializer` for XML [simple types] [as defined] in the XML Schema.
//!
//! [simple types]: https://www.w3schools.com/xml/el_simpletype.asp
//! [as defined]: https://www.w3.org/TR/xmlschema11-1/#Simple_Type_Definition

use crate::escapei::_escape;
use crate::se::QuoteLevel;
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteTarget {
    /// Escape data for a text content. No additional escape symbols
    Text,
    /// Escape data for a double-quoted attribute. `"` always escaped
    DoubleQAttr,
    /// Escape data for a single-quoted attribute. `'` always escaped
    SingleQAttr,
}

/// Escapes atomic value that could be part of a `xs:list`. All whitespace characters
/// additionally escaped
fn escape_item(value: &str, target: QuoteTarget, level: QuoteLevel) -> Cow<str> {
    use QuoteLevel::*;
    use QuoteTarget::*;

    match (target, level) {
        (_, Full) => _escape(value, |ch| match ch {
            // Spaces used as delimiters of list items, cannot be used in the item
            b' ' | b'\r' | b'\n' | b'\t' => true,
            // Required characters to escape
            b'&' | b'<' | b'>' | b'\'' | b'\"' => true,
            _ => false,
        }),
        //----------------------------------------------------------------------
        (Text, Partial) => _escape(value, |ch| match ch {
            // Spaces used as delimiters of list items, cannot be used in the item
            b' ' | b'\r' | b'\n' | b'\t' => true,
            // Required characters to escape
            b'&' | b'<' | b'>' => true,
            _ => false,
        }),
        (Text, Minimal) => _escape(value, |ch| match ch {
            // Spaces used as delimiters of list items, cannot be used in the item
            b' ' | b'\r' | b'\n' | b'\t' => true,
            // Required characters to escape
            b'&' | b'<' => true,
            _ => false,
        }),
        //----------------------------------------------------------------------
        (DoubleQAttr, Partial) => _escape(value, |ch| match ch {
            // Spaces used as delimiters of list items, cannot be used in the item
            b' ' | b'\r' | b'\n' | b'\t' => true,
            // Required characters to escape
            b'&' | b'<' | b'>' => true,
            // Double quoted attribute should escape quote
            b'"' => true,
            _ => false,
        }),
        (DoubleQAttr, Minimal) => _escape(value, |ch| match ch {
            // Spaces used as delimiters of list items, cannot be used in the item
            b' ' | b'\r' | b'\n' | b'\t' => true,
            // Required characters to escape
            b'&' | b'<' => true,
            // Double quoted attribute should escape quote
            b'"' => true,
            _ => false,
        }),
        //----------------------------------------------------------------------
        (SingleQAttr, Partial) => _escape(value, |ch| match ch {
            // Spaces used as delimiters of list items
            b' ' | b'\r' | b'\n' | b'\t' => true,
            // Required characters to escape
            b'&' | b'<' | b'>' => true,
            // Single quoted attribute should escape quote
            b'\'' => true,
            _ => false,
        }),
        (SingleQAttr, Minimal) => _escape(value, |ch| match ch {
            // Spaces used as delimiters of list items
            b' ' | b'\r' | b'\n' | b'\t' => true,
            // Required characters to escape
            b'&' | b'<' => true,
            // Single quoted attribute should escape quote
            b'\'' => true,
            _ => false,
        }),
    }
}

/// Escapes XSD simple type value
fn escape_list(value: &str, target: QuoteTarget, level: QuoteLevel) -> Cow<str> {
    use QuoteLevel::*;
    use QuoteTarget::*;

    match (target, level) {
        (_, Full) => _escape(value, |ch| match ch {
            // Required characters to escape
            b'&' | b'<' | b'>' | b'\'' | b'\"' => true,
            _ => false,
        }),
        //----------------------------------------------------------------------
        (Text, Partial) => _escape(value, |ch| match ch {
            // Required characters to escape
            b'&' | b'<' | b'>' => true,
            _ => false,
        }),
        (Text, Minimal) => _escape(value, |ch| match ch {
            // Required characters to escape
            b'&' | b'<' => true,
            _ => false,
        }),
        //----------------------------------------------------------------------
        (DoubleQAttr, Partial) => _escape(value, |ch| match ch {
            // Required characters to escape
            b'&' | b'<' | b'>' => true,
            // Double quoted attribute should escape quote
            b'"' => true,
            _ => false,
        }),
        (DoubleQAttr, Minimal) => _escape(value, |ch| match ch {
            // Required characters to escape
            b'&' | b'<' => true,
            // Double quoted attribute should escape quote
            b'"' => true,
            _ => false,
        }),
        //----------------------------------------------------------------------
        (SingleQAttr, Partial) => _escape(value, |ch| match ch {
            // Required characters to escape
            b'&' | b'<' | b'>' => true,
            // Single quoted attribute should escape quote
            b'\'' => true,
            _ => false,
        }),
        (SingleQAttr, Minimal) => _escape(value, |ch| match ch {
            // Required characters to escape
            b'&' | b'<' => true,
            // Single quoted attribute should escape quote
            b'\'' => true,
            _ => false,
        }),
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    mod escape_item {
        use super::*;

        mod full {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn text() {
                assert_eq!(
                    escape_item("text<\"'&> \t\r\ntext", QuoteTarget::Text, QuoteLevel::Full),
                    "text&lt;&quot;&apos;&amp;&gt;&#32;&#9;&#10;&#13;text"
                );
            }

            #[test]
            fn double_quote_attr() {
                assert_eq!(
                    escape_item(
                        "text<\"'&> \t\r\ntext",
                        QuoteTarget::DoubleQAttr,
                        QuoteLevel::Full
                    ),
                    "text&lt;&quot;&apos;&amp;&gt;&#32;&#9;&#10;&#13;text"
                );
            }

            #[test]
            fn single_quote_attr() {
                assert_eq!(
                    escape_item(
                        "text<\"'&> \t\r\ntext",
                        QuoteTarget::SingleQAttr,
                        QuoteLevel::Full
                    ),
                    "text&lt;&quot;&apos;&amp;&gt;&#32;&#9;&#10;&#13;text"
                );
            }
        }

        mod partial {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn text() {
                assert_eq!(
                    escape_item(
                        "text<\"'&> \t\r\ntext",
                        QuoteTarget::Text,
                        QuoteLevel::Partial
                    ),
                    "text&lt;\"'&amp;&gt;&#32;&#9;&#10;&#13;text"
                );
            }

            #[test]
            fn double_quote_attr() {
                assert_eq!(
                    escape_item(
                        "text<\"'&> \t\r\ntext",
                        QuoteTarget::DoubleQAttr,
                        QuoteLevel::Partial
                    ),
                    "text&lt;&quot;'&amp;&gt;&#32;&#9;&#10;&#13;text"
                );
            }

            #[test]
            fn single_quote_attr() {
                assert_eq!(
                    escape_item(
                        "text<\"'&> \t\r\ntext",
                        QuoteTarget::SingleQAttr,
                        QuoteLevel::Partial
                    ),
                    "text&lt;\"&apos;&amp;&gt;&#32;&#9;&#10;&#13;text"
                );
            }
        }

        mod minimal {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn text() {
                assert_eq!(
                    escape_item(
                        "text<\"'&> \t\r\ntext",
                        QuoteTarget::Text,
                        QuoteLevel::Minimal
                    ),
                    "text&lt;\"'&amp;>&#32;&#9;&#10;&#13;text"
                );
            }

            #[test]
            fn double_quote_attr() {
                assert_eq!(
                    escape_item(
                        "text<\"'&> \t\r\ntext",
                        QuoteTarget::DoubleQAttr,
                        QuoteLevel::Minimal
                    ),
                    "text&lt;&quot;'&amp;>&#32;&#9;&#10;&#13;text"
                );
            }

            #[test]
            fn single_quote_attr() {
                assert_eq!(
                    escape_item(
                        "text<\"'&> \t\r\ntext",
                        QuoteTarget::SingleQAttr,
                        QuoteLevel::Minimal
                    ),
                    "text&lt;\"&apos;&amp;>&#32;&#9;&#10;&#13;text"
                );
            }
        }
    }

    mod escape_list {
        use super::*;

        mod full {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn text() {
                assert_eq!(
                    escape_list("text<\"'&> \t\r\ntext", QuoteTarget::Text, QuoteLevel::Full),
                    "text&lt;&quot;&apos;&amp;&gt; \t\r\ntext"
                );
            }

            #[test]
            fn double_quote_attr() {
                assert_eq!(
                    escape_list(
                        "text<\"'&> \t\r\ntext",
                        QuoteTarget::DoubleQAttr,
                        QuoteLevel::Full
                    ),
                    "text&lt;&quot;&apos;&amp;&gt; \t\r\ntext"
                );
            }

            #[test]
            fn single_quote_attr() {
                assert_eq!(
                    escape_list(
                        "text<\"'&> \t\r\ntext",
                        QuoteTarget::SingleQAttr,
                        QuoteLevel::Full
                    ),
                    "text&lt;&quot;&apos;&amp;&gt; \t\r\ntext"
                );
            }
        }

        mod partial {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn text() {
                assert_eq!(
                    escape_list(
                        "text<\"'&> \t\r\ntext",
                        QuoteTarget::Text,
                        QuoteLevel::Partial
                    ),
                    "text&lt;\"'&amp;&gt; \t\r\ntext"
                );
            }

            #[test]
            fn double_quote_attr() {
                assert_eq!(
                    escape_list(
                        "text<\"'&> \t\r\ntext",
                        QuoteTarget::DoubleQAttr,
                        QuoteLevel::Partial
                    ),
                    "text&lt;&quot;'&amp;&gt; \t\r\ntext"
                );
            }

            #[test]
            fn single_quote_attr() {
                assert_eq!(
                    escape_list(
                        "text<\"'&> \t\r\ntext",
                        QuoteTarget::SingleQAttr,
                        QuoteLevel::Partial
                    ),
                    "text&lt;\"&apos;&amp;&gt; \t\r\ntext"
                );
            }
        }

        mod minimal {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn text() {
                assert_eq!(
                    escape_list(
                        "text<\"'&> \t\r\ntext",
                        QuoteTarget::Text,
                        QuoteLevel::Minimal
                    ),
                    "text&lt;\"'&amp;> \t\r\ntext"
                );
            }

            #[test]
            fn double_quote_attr() {
                assert_eq!(
                    escape_list(
                        "text<\"'&> \t\r\ntext",
                        QuoteTarget::DoubleQAttr,
                        QuoteLevel::Minimal
                    ),
                    "text&lt;&quot;'&amp;> \t\r\ntext"
                );
            }

            #[test]
            fn single_quote_attr() {
                assert_eq!(
                    escape_list(
                        "text<\"'&> \t\r\ntext",
                        QuoteTarget::SingleQAttr,
                        QuoteLevel::Minimal
                    ),
                    "text&lt;\"&apos;&amp;> \t\r\ntext"
                );
            }
        }
    }
}

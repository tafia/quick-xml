//! Manage xml character escapes

use memchr::memchr2_iter;
use std::borrow::Cow;
use std::ops::Range;

#[cfg(test)]
use pretty_assertions::assert_eq;

/// Error for XML escape / unescape.
#[derive(Clone, Debug)]
pub enum EscapeError {
    /// Entity with Null character
    EntityWithNull(Range<usize>),
    /// Unrecognized escape symbol
    UnrecognizedSymbol(Range<usize>, String),
    /// Cannot find `;` after `&`
    UnterminatedEntity(Range<usize>),
    /// Cannot convert Hexa to utf8
    TooLongHexadecimal,
    /// Character is not a valid hexadecimal value
    InvalidHexadecimal(char),
    /// Cannot convert decimal to hexa
    TooLongDecimal,
    /// Character is not a valid decimal value
    InvalidDecimal(char),
    /// Not a valid unicode codepoint
    InvalidCodepoint(u32),
}

impl std::fmt::Display for EscapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            EscapeError::EntityWithNull(e) => write!(
                f,
                "Error while escaping character at range {:?}: Null character entity not allowed",
                e
            ),
            EscapeError::UnrecognizedSymbol(rge, res) => write!(
                f,
                "Error while escaping character at range {:?}: Unrecognized escape symbol: {:?}",
                rge, res
            ),
            EscapeError::UnterminatedEntity(e) => write!(
                f,
                "Error while escaping character at range {:?}: Cannot find ';' after '&'",
                e
            ),
            EscapeError::TooLongHexadecimal => write!(f, "Cannot convert hexadecimal to utf8"),
            EscapeError::InvalidHexadecimal(e) => {
                write!(f, "'{}' is not a valid hexadecimal character", e)
            }
            EscapeError::TooLongDecimal => write!(f, "Cannot convert decimal to utf8"),
            EscapeError::InvalidDecimal(e) => write!(f, "'{}' is not a valid decimal character", e),
            EscapeError::InvalidCodepoint(n) => write!(f, "'{}' is not a valid codepoint", n),
        }
    }
}

impl std::error::Error for EscapeError {}

/// Escapes an `&str` and replaces all xml special characters (`<`, `>`, `&`, `'`, `"`)
/// with their corresponding xml escaped value.
///
/// This function performs following replacements:
///
/// | Character | Replacement
/// |-----------|------------
/// | `<`       | `&lt;`
/// | `>`       | `&gt;`
/// | `&`       | `&amp;`
/// | `'`       | `&apos;`
/// | `"`       | `&quot;`
///
/// This function performs following replacements:
///
/// | Character | Replacement
/// |-----------|------------
/// | `<`       | `&lt;`
/// | `>`       | `&gt;`
/// | `&`       | `&amp;`
/// | `'`       | `&apos;`
/// | `"`       | `&quot;`
pub fn escape(raw: &str) -> Cow<str> {
    _escape(raw, |ch| matches!(ch, b'<' | b'>' | b'&' | b'\'' | b'\"'))
}

/// Escapes an `&str` and replaces xml special characters (`<`, `>`, `&`)
/// with their corresponding xml escaped value.
///
/// Should only be used for escaping text content. In XML text content, it is allowed
/// (though not recommended) to leave the quote special characters `"` and `'` unescaped.
///
/// This function performs following replacements:
///
/// | Character | Replacement
/// |-----------|------------
/// | `<`       | `&lt;`
/// | `>`       | `&gt;`
/// | `&`       | `&amp;`
///
/// This function performs following replacements:
///
/// | Character | Replacement
/// |-----------|------------
/// | `<`       | `&lt;`
/// | `>`       | `&gt;`
/// | `&`       | `&amp;`
pub fn partial_escape(raw: &str) -> Cow<str> {
    _escape(raw, |ch| matches!(ch, b'<' | b'>' | b'&'))
}

/// XML standard [requires] that only `<` and `&` was escaped in text content or
/// attribute value. All other characters not necessary to be escaped, although
/// for compatibility with SGML they also should be escaped. Practically, escaping
/// only those characters is enough.
///
/// This function performs following replacements:
///
/// | Character | Replacement
/// |-----------|------------
/// | `<`       | `&lt;`
/// | `&`       | `&amp;`
///
/// [requires]: https://www.w3.org/TR/xml11/#syntax
pub fn minimal_escape(raw: &str) -> Cow<str> {
    _escape(raw, |ch| matches!(ch, b'<' | b'&'))
}

/// Escapes an `&str` and replaces a subset of xml special characters (`<`, `>`,
/// `&`, `'`, `"`) with their corresponding xml escaped value.
pub(crate) fn _escape<F: Fn(u8) -> bool>(raw: &str, escape_chars: F) -> Cow<str> {
    let bytes = raw.as_bytes();
    let mut escaped = None;
    let mut iter = bytes.iter();
    let mut pos = 0;
    while let Some(i) = iter.position(|&b| escape_chars(b)) {
        if escaped.is_none() {
            escaped = Some(Vec::with_capacity(raw.len()));
        }
        let escaped = escaped.as_mut().expect("initialized");
        let new_pos = pos + i;
        escaped.extend_from_slice(&bytes[pos..new_pos]);
        match bytes[new_pos] {
            b'<' => escaped.extend_from_slice(b"&lt;"),
            b'>' => escaped.extend_from_slice(b"&gt;"),
            b'\'' => escaped.extend_from_slice(b"&apos;"),
            b'&' => escaped.extend_from_slice(b"&amp;"),
            b'"' => escaped.extend_from_slice(b"&quot;"),

            // This set of escapes handles characters that should be escaped
            // in elements of xs:lists, because those characters works as
            // delimiters of list elements
            b'\t' => escaped.extend_from_slice(b"&#9;"),
            b'\n' => escaped.extend_from_slice(b"&#10;"),
            b'\r' => escaped.extend_from_slice(b"&#13;"),
            b' ' => escaped.extend_from_slice(b"&#32;"),
            _ => unreachable!(
                "Only '<', '>','\', '&', '\"', '\\t', '\\r', '\\n', and ' ' are escaped"
            ),
        }
        pos = new_pos + 1;
    }

    if let Some(mut escaped) = escaped {
        if let Some(raw) = bytes.get(pos..) {
            escaped.extend_from_slice(raw);
        }
        // SAFETY: we operate on UTF-8 input and search for an one byte chars only,
        // so all slices that was put to the `escaped` is a valid UTF-8 encoded strings
        // TODO: Can be replaced with `unsafe { String::from_utf8_unchecked() }`
        // if unsafe code will be allowed
        Cow::Owned(String::from_utf8(escaped).unwrap())
    } else {
        Cow::Borrowed(raw)
    }
}

/// Unescape an `&str` and replaces all xml escaped characters (`&...;`) into
/// their corresponding value.
pub fn unescape(raw: &str) -> Result<Cow<str>, EscapeError> {
    unescape_with(raw, resolve_predefined_entity)
}

/// Unescape an `&str` and replaces all xml escaped characters (`&...;`) into
/// their corresponding value, using a resolver function for custom entities.
///
/// Predefined entities will be resolved _after_ trying to resolve with `resolve_entity`,
/// which allows you to override default behavior which required in some XML dialects.
///
/// Character references (`&#hh;`) cannot be overridden, they are resolved before
/// calling `resolve_entity`.
///
/// Note, that entities will not be resolved recursively. In order to satisfy the
/// XML [requirements] you should unescape nested entities by yourself.
///
/// # Example
///
/// ```
/// use quick_xml::escape::resolve_predefined_entity;
/// # use quick_xml::escape::unescape_with;
/// # use pretty_assertions::assert_eq;
/// let override_named_entities = |entity: &str| match entity {
///     // Override standard entities
///     "lt" => Some("FOO"),
///     "gt" => Some("BAR"),
///     // Resolve custom entities
///     "baz" => Some("&lt;"),
///     // Delegate other entities to the default implementation
///     _ => resolve_predefined_entity(entity),
/// };
///
/// assert_eq!(
///     unescape_with("&amp;&lt;test&gt;&baz;", override_named_entities).unwrap(),
///     "&FOOtestBAR&lt;"
/// );
/// ```
///
/// [requirements]: https://www.w3.org/TR/xml11/#intern-replacement
pub fn unescape_with<'input, 'entity, F>(
    raw: &'input str,
    mut resolve_entity: F,
) -> Result<Cow<'input, str>, EscapeError>
where
    // the lifetime of the output comes from a capture or is `'static`
    F: FnMut(&str) -> Option<&'entity str>,
{
    let bytes = raw.as_bytes();
    let mut unescaped = None;
    let mut last_end = 0;
    let mut iter = memchr2_iter(b'&', b';', bytes);
    while let Some(start) = iter.by_ref().find(|p| bytes[*p] == b'&') {
        match iter.next() {
            Some(end) if bytes[end] == b';' => {
                // append valid data
                if unescaped.is_none() {
                    unescaped = Some(String::with_capacity(raw.len()));
                }
                let unescaped = unescaped.as_mut().expect("initialized");
                unescaped.push_str(&raw[last_end..start]);

                // search for character correctness
                let pat = &raw[start + 1..end];
                if let Some(entity) = pat.strip_prefix('#') {
                    let codepoint = parse_number(entity, start..end)?;
                    unescaped.push_str(codepoint.encode_utf8(&mut [0u8; 4]));
                } else if let Some(value) = resolve_entity(pat) {
                    unescaped.push_str(value);
                } else {
                    return Err(EscapeError::UnrecognizedSymbol(
                        start + 1..end,
                        pat.to_string(),
                    ));
                }

                last_end = end + 1;
            }
            _ => return Err(EscapeError::UnterminatedEntity(start..raw.len())),
        }
    }

    if let Some(mut unescaped) = unescaped {
        if let Some(raw) = raw.get(last_end..) {
            unescaped.push_str(raw);
        }
        Ok(Cow::Owned(unescaped))
    } else {
        Ok(Cow::Borrowed(raw))
    }
}

/// Resolves predefined XML entities. If specified entity is not a predefined XML
/// entity, `None` is returned.
///
/// The complete list of predefined entities are defined in the [specification].
///
/// ```
/// # use quick_xml::escape::resolve_predefined_entity;
/// # use pretty_assertions::assert_eq;
/// assert_eq!(resolve_predefined_entity("lt"), Some("<"));
/// assert_eq!(resolve_predefined_entity("gt"), Some(">"));
/// assert_eq!(resolve_predefined_entity("amp"), Some("&"));
/// assert_eq!(resolve_predefined_entity("apos"), Some("'"));
/// assert_eq!(resolve_predefined_entity("quot"), Some("\""));
///
/// assert_eq!(resolve_predefined_entity("foo"), None);
/// ```
///
/// [specification]: https://www.w3.org/TR/xml11/#sec-predefined-ent
pub const fn resolve_predefined_entity(entity: &str) -> Option<&'static str> {
    // match over strings are not allowed in const functions
    let s = match entity.as_bytes() {
        b"lt" => "<",
        b"gt" => ">",
        b"amp" => "&",
        b"apos" => "'",
        b"quot" => "\"",
        _ => return None,
    };
    Some(s)
}

fn parse_number(bytes: &str, range: Range<usize>) -> Result<char, EscapeError> {
    let code = if let Some(hex_digits) = bytes.strip_prefix('x') {
        parse_hexadecimal(hex_digits)
    } else {
        parse_decimal(bytes)
    }?;
    if code == 0 {
        return Err(EscapeError::EntityWithNull(range));
    }
    match std::char::from_u32(code) {
        Some(c) => Ok(c),
        None => Err(EscapeError::InvalidCodepoint(code)),
    }
}

fn parse_hexadecimal(bytes: &str) -> Result<u32, EscapeError> {
    // maximum code is 0x10FFFF => 6 characters
    if bytes.len() > 6 {
        return Err(EscapeError::TooLongHexadecimal);
    }
    let mut code = 0;
    for b in bytes.bytes() {
        code <<= 4;
        code += match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            b => return Err(EscapeError::InvalidHexadecimal(b as char)),
        } as u32;
    }
    Ok(code)
}

fn parse_decimal(bytes: &str) -> Result<u32, EscapeError> {
    // maximum code is 0x10FFFF = 1114111 => 7 characters
    if bytes.len() > 7 {
        return Err(EscapeError::TooLongDecimal);
    }
    let mut code = 0;
    for b in bytes.bytes() {
        code *= 10;
        code += match b {
            b'0'..=b'9' => b - b'0',
            b => return Err(EscapeError::InvalidDecimal(b as char)),
        } as u32;
    }
    Ok(code)
}

#[test]
fn test_unescape() {
    let unchanged = unescape("test").unwrap();
    // assert_eq does not check that Cow is borrowed, but we explicitly use Cow
    // because it influences diff
    // TODO: use assert_matches! when stabilized and other features will bump MSRV
    assert_eq!(unchanged, Cow::Borrowed("test"));
    assert!(matches!(unchanged, Cow::Borrowed(_)));

    assert_eq!(
        unescape("&lt;&amp;test&apos;&quot;&gt;").unwrap(),
        "<&test'\">"
    );
    assert_eq!(unescape("&#x30;").unwrap(), "0");
    assert_eq!(unescape("&#48;").unwrap(), "0");
    assert!(unescape("&foo;").is_err());
}

#[test]
fn test_unescape_with() {
    let custom_entities = |ent: &str| match ent {
        "foo" => Some("BAR"),
        _ => None,
    };

    let unchanged = unescape_with("test", custom_entities).unwrap();
    // assert_eq does not check that Cow is borrowed, but we explicitly use Cow
    // because it influences diff
    // TODO: use assert_matches! when stabilized and other features will bump MSRV
    assert_eq!(unchanged, Cow::Borrowed("test"));
    assert!(matches!(unchanged, Cow::Borrowed(_)));

    assert!(unescape_with("&lt;", custom_entities).is_err());
    assert_eq!(unescape_with("&#x30;", custom_entities).unwrap(), "0");
    assert_eq!(unescape_with("&#48;", custom_entities).unwrap(), "0");
    assert_eq!(unescape_with("&foo;", custom_entities).unwrap(), "BAR");
    assert!(unescape_with("&fop;", custom_entities).is_err());
}

#[test]
fn test_escape() {
    let unchanged = escape("test");
    // assert_eq does not check that Cow is borrowed, but we explicitly use Cow
    // because it influences diff
    // TODO: use assert_matches! when stabilized and other features will bump MSRV
    assert_eq!(unchanged, Cow::Borrowed("test"));
    assert!(matches!(unchanged, Cow::Borrowed(_)));

    assert_eq!(escape("<&\"'>"), "&lt;&amp;&quot;&apos;&gt;");
    assert_eq!(escape("<test>"), "&lt;test&gt;");
    assert_eq!(escape("\"a\"bc"), "&quot;a&quot;bc");
    assert_eq!(escape("\"a\"b&c"), "&quot;a&quot;b&amp;c");
    assert_eq!(
        escape("prefix_\"a\"b&<>c"),
        "prefix_&quot;a&quot;b&amp;&lt;&gt;c"
    );
}

#[test]
fn test_partial_escape() {
    let unchanged = partial_escape("test");
    // assert_eq does not check that Cow is borrowed, but we explicitly use Cow
    // because it influences diff
    // TODO: use assert_matches! when stabilized and other features will bump MSRV
    assert_eq!(unchanged, Cow::Borrowed("test"));
    assert!(matches!(unchanged, Cow::Borrowed(_)));

    assert_eq!(partial_escape("<&\"'>"), "&lt;&amp;\"'&gt;");
    assert_eq!(partial_escape("<test>"), "&lt;test&gt;");
    assert_eq!(partial_escape("\"a\"bc"), "\"a\"bc");
    assert_eq!(partial_escape("\"a\"b&c"), "\"a\"b&amp;c");
    assert_eq!(
        partial_escape("prefix_\"a\"b&<>c"),
        "prefix_\"a\"b&amp;&lt;&gt;c"
    );
}

#[test]
fn test_minimal_escape() {
    assert_eq!(minimal_escape("test"), Cow::Borrowed("test"));
    assert_eq!(minimal_escape("<&\"'>"), "&lt;&amp;\"'>");
    assert_eq!(minimal_escape("<test>"), "&lt;test>");
    assert_eq!(minimal_escape("\"a\"bc"), "\"a\"bc");
    assert_eq!(minimal_escape("\"a\"b&c"), "\"a\"b&amp;c");
    assert_eq!(
        minimal_escape("prefix_\"a\"b&<>c"),
        "prefix_\"a\"b&amp;&lt;>c"
    );
}

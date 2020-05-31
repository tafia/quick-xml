//! Manage xml character escapes

use memchr;
use std::borrow::Cow;

#[derive(Debug)]
pub enum EscapeError {
    /// Entity with Null character
    EntityWithNull(::std::ops::Range<usize>),
    /// Unrecognized escape symbol
    UnrecognizedSymbol(
        ::std::ops::Range<usize>,
        ::std::result::Result<String, ::std::string::FromUtf8Error>,
    ),
    /// Cannot find `;` after `&`
    UnterminatedEntity(::std::ops::Range<usize>),
    /// Cannot convert Hexa to utf8
    TooLongHexadecimal,
    /// Character is not a valid hexadecimal value
    InvalidHexadecimal(char),
    /// Cannot convert decimal to hexa
    TooLongDecimal,
    /// Character is not a valid decimal value
    InvalidDecimal(char),
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
        }
    }
}

impl std::error::Error for EscapeError {}

// UTF-8 ranges and tags for encoding characters
const TAG_CONT: u8 = 0b1000_0000;
const TAG_TWO_B: u8 = 0b1100_0000;
const TAG_THREE_B: u8 = 0b1110_0000;
const TAG_FOUR_B: u8 = 0b1111_0000;
const MAX_ONE_B: u32 = 0x80;
const MAX_TWO_B: u32 = 0x800;
const MAX_THREE_B: u32 = 0x10000;

/// Escapes a `&[u8]` and replaces all xml special characters (<, >, &, ', ") with their
/// corresponding xml escaped value.
pub fn escape(raw: &[u8]) -> Cow<[u8]> {
    fn to_escape(b: u8) -> bool {
        match b {
            b'<' | b'>' | b'\'' | b'&' | b'"' => true,
            _ => false,
        }
    }

    let mut escaped = None;
    let mut bytes = raw.iter();
    let mut pos = 0;
    while let Some(i) = bytes.position(|&b| to_escape(b)) {
        if escaped.is_none() {
            escaped = Some(Vec::with_capacity(raw.len()));
        }
        let escaped = escaped.as_mut().expect("initialized");
        let new_pos = pos + i;
        escaped.extend_from_slice(&raw[pos..new_pos]);
        match raw[new_pos] {
            b'<' => escaped.extend_from_slice(b"&lt;"),
            b'>' => escaped.extend_from_slice(b"&gt;"),
            b'\'' => escaped.extend_from_slice(b"&apos;"),
            b'&' => escaped.extend_from_slice(b"&amp;"),
            b'"' => escaped.extend_from_slice(b"&quot;"),
            _ => unreachable!("Only '<', '>','\', '&' and '\"' are escaped"),
        }
        pos = new_pos + 1;
    }

    if let Some(mut escaped) = escaped {
        if let Some(raw) = raw.get(pos..) {
            escaped.extend_from_slice(raw);
        }
        Cow::Owned(escaped)
    } else {
        Cow::Borrowed(raw)
    }
}

/// Unescape a `&[u8]` and replaces all xml escaped characters ('&...;') into their corresponding
/// value
pub fn unescape(raw: &[u8]) -> Result<Cow<[u8]>, EscapeError> {
    let mut unescaped = None;
    let mut last_end = 0;
    let mut iter = memchr::memchr2_iter(b'&', b';', raw);
    while let Some(start) = iter.by_ref().find(|p| raw[*p] == b'&') {
        match iter.next() {
            Some(end) if raw[end] == b';' => {
                // append valid data
                if unescaped.is_none() {
                    unescaped = Some(Vec::with_capacity(raw.len()));
                }
                let unescaped = unescaped.as_mut().expect("initialized");
                unescaped.extend_from_slice(&raw[last_end..start]);

                // search for character correctness
                match &raw[start + 1..end] {
                    b"lt" => unescaped.push(b'<'),
                    b"gt" => unescaped.push(b'>'),
                    b"amp" => unescaped.push(b'&'),
                    b"apos" => unescaped.push(b'\''),
                    b"quot" => unescaped.push(b'\"'),
                    bytes => {
                        let code = if bytes.starts_with(b"#x") {
                            parse_hexadecimal(&bytes[2..])
                        } else if bytes.starts_with(b"#") {
                            parse_decimal(&bytes[1..])
                        } else {
                            Err(EscapeError::UnrecognizedSymbol(
                                start + 1..end,
                                String::from_utf8(bytes.to_vec()),
                            ))
                        }?;
                        if code == 0 {
                            return Err(EscapeError::EntityWithNull(start..end));
                        }
                        push_utf8(unescaped, code);
                    }
                }
                last_end = end + 1;
            }
            _ => return Err(EscapeError::UnterminatedEntity(start..raw.len())),
        }
    }

    if let Some(mut unescaped) = unescaped {
        if let Some(raw) = raw.get(last_end..) {
            unescaped.extend_from_slice(raw);
        }
        Ok(Cow::Owned(unescaped))
    } else {
        Ok(Cow::Borrowed(raw))
    }
}

fn push_utf8(buf: &mut Vec<u8>, code: u32) {
    if code < MAX_ONE_B {
        buf.push(code as u8);
    } else if code < MAX_TWO_B {
        buf.push((code >> 6 & 0x1F) as u8 | TAG_TWO_B);
        buf.push((code & 0x3F) as u8 | TAG_CONT);
    } else if code < MAX_THREE_B {
        buf.push((code >> 12 & 0x0F) as u8 | TAG_THREE_B);
        buf.push((code >> 6 & 0x3F) as u8 | TAG_CONT);
        buf.push((code & 0x3F) as u8 | TAG_CONT);
    } else {
        buf.push((code >> 18 & 0x07) as u8 | TAG_FOUR_B);
        buf.push((code >> 12 & 0x3F) as u8 | TAG_CONT);
        buf.push((code >> 6 & 0x3F) as u8 | TAG_CONT);
        buf.push((code & 0x3F) as u8 | TAG_CONT);
    }
}

fn parse_hexadecimal(bytes: &[u8]) -> Result<u32, EscapeError> {
    // maximum code is 0x10FFFF => 6 characters
    if bytes.len() > 6 {
        return Err(EscapeError::TooLongHexadecimal);
    }
    let mut code = 0;
    for &b in bytes {
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

fn parse_decimal(bytes: &[u8]) -> Result<u32, EscapeError> {
    // maximum code is 0x10FFFF = 1114111 => 7 characters
    if bytes.len() > 7 {
        return Err(EscapeError::TooLongDecimal);
    }
    let mut code = 0;
    for &b in bytes {
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
    assert_eq!(&*unescape(b"test").unwrap(), b"test");
    assert_eq!(&*unescape(b"&lt;test&gt;").unwrap(), b"<test>");
    assert_eq!(&*unescape(b"&#x30;").unwrap(), b"0");
    assert_eq!(&*unescape(b"&#48;").unwrap(), b"0");
}

#[test]
fn test_escape() {
    assert_eq!(&*escape(b"test"), b"test");
    assert_eq!(&*escape(b"<test>"), b"&lt;test&gt;");
    assert_eq!(&*escape(b"\"a\"bc"), b"&quot;a&quot;bc");
    assert_eq!(&*escape(b"\"a\"b&c"), b"&quot;a&quot;b&amp;c");
    assert_eq!(
        &*escape(b"prefix_\"a\"b&<>c"),
        "prefix_&quot;a&quot;b&amp;&lt;&gt;c".as_bytes()
    );
}

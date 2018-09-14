//! Manage xml character escapes

use memchr;
use std::borrow::Cow;

#[allow(missing_docs)]
#[derive(Fail, Debug)]
pub enum EscapeError {
    #[fail(
        display = "Error while escaping character at range {:?}: Null character entity not allowed",
        _0
    )]
    EntityWithNull(::std::ops::Range<usize>),

    #[fail(
        display = "Error while escaping character at range {:?}: Unrecognized escape symbol: {:?}",
        _0,
        _1
    )]
    UnrecognizedSymbol(
        ::std::ops::Range<usize>,
        ::std::result::Result<String, ::std::string::FromUtf8Error>,
    ),

    #[fail(
        display = "Error while escaping character at range {:?}: Cannot find ';' after '&'",
        _0
    )]
    UnterminatedEntity(::std::ops::Range<usize>),

    #[fail(display = "Cannot convert hexadecimal to utf8")]
    TooLongHexadecimal,

    #[fail(display = "'{}' is not a valid hexadecimal character", _0)]
    InvalidHexadecimal(char),

    #[fail(display = "Cannot convert decimal to utf8")]
    TooLongDecimal,

    #[fail(display = "'{}' is not a valid decimal character", _0)]
    InvalidDecimal(char),
}

// UTF-8 ranges and tags for encoding characters
const TAG_CONT: u8 = 0b1000_0000;
const TAG_TWO_B: u8 = 0b1100_0000;
const TAG_THREE_B: u8 = 0b1110_0000;
const TAG_FOUR_B: u8 = 0b1111_0000;
const MAX_ONE_B: u32 = 0x80;
const MAX_TWO_B: u32 = 0x800;
const MAX_THREE_B: u32 = 0x10000;

enum ByteOrChar {
    Byte(u8),
    Char(u32),
}

/// helper function to escape a `&[u8]` and replace all
/// xml special characters (<, >, &, ', ") with their corresponding
/// xml escaped value.
pub fn escape(raw: &[u8]) -> Cow<[u8]> {
    let mut escapes: Vec<(usize, &'static [u8])> = Vec::new();
    let mut bytes = raw.iter();
    fn to_escape(b: u8) -> bool {
        match b {
            b'<' | b'>' | b'\'' | b'&' | b'"' => true,
            _ => false,
        }
    }

    let mut loc = 0;
    while let Some(i) = bytes.position(|&b| to_escape(b)) {
        loc += i;
        match raw[loc] {
            b'<' => escapes.push((loc, b"&lt;")),
            b'>' => escapes.push((loc, b"&gt;")),
            b'\'' => escapes.push((loc, b"&apos;")),
            b'&' => escapes.push((loc, b"&amp;")),
            b'"' => escapes.push((loc, b"&quot;")),
            _ => unreachable!("Only '<', '>','\', '&' and '\"' are escaped"),
        }
        loc += 1;
    }

    if escapes.is_empty() {
        Cow::Borrowed(raw)
    } else {
        let len = raw.len();
        let mut v = Vec::with_capacity(len);
        let mut start = 0;
        for (i, r) in escapes {
            v.extend_from_slice(&raw[start..i]);
            v.extend_from_slice(r);
            start = i + 1;
        }

        if start < len {
            v.extend_from_slice(&raw[start..]);
        }
        Cow::Owned(v)
    }
}

/// helper function to unescape a `&[u8]` and replace all
/// xml escaped characters ('&...;') into their corresponding value
pub fn unescape(raw: &[u8]) -> Result<Cow<[u8]>, EscapeError> {
    let mut escapes = Vec::new();

    let mut start = 0;
    while let Some(i) = memchr::memchr(b'&', &raw[start..]) {
        start += i + 1;
        if let Some(j) = memchr::memchr(b';', &raw[start..]) {
            let end = start + j;
            // search for character correctness
            let b_o_c = match &raw[start..end] {
                b"lt" => ByteOrChar::Byte(b'<'),
                b"gt" => ByteOrChar::Byte(b'>'),
                b"amp" => ByteOrChar::Byte(b'&'),
                b"apos" => ByteOrChar::Byte(b'\''),
                b"quot" => ByteOrChar::Byte(b'\"'),
                b"#x0" | b"#0" => return Err(EscapeError::EntityWithNull(start..end)),
                bytes if bytes.starts_with(b"#x") => {
                    ByteOrChar::Char(parse_hexadecimal(&bytes[2..])?)
                }
                bytes if bytes.starts_with(b"#") => ByteOrChar::Char(parse_decimal(&bytes[1..])?),
                bytes => {
                    return Err(EscapeError::UnrecognizedSymbol(
                        start..end,
                        String::from_utf8(bytes.to_vec()),
                    ))
                }
            };
            escapes.push((start - 1..end, b_o_c));
            start = end + 1;
        } else {
            return Err(EscapeError::UnterminatedEntity(start..raw.len()));
        }
    }
    if escapes.is_empty() {
        Ok(Cow::Borrowed(raw))
    } else {
        let len = raw.len();
        let mut v = Vec::with_capacity(len);
        let mut start = 0;
        for (r, b) in escapes {
            v.extend_from_slice(&raw[start..r.start]);
            match b {
                ByteOrChar::Byte(b) => v.push(b),
                ByteOrChar::Char(c) => push_utf8(&mut v, c),
            }
            start = r.end + 1;
        }
        if start < raw.len() {
            v.extend_from_slice(&raw[start..]);
        }
        Ok(Cow::Owned(v))
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
            b'0'...b'9' => b - b'0',
            b'a'...b'f' => b - b'a' + 10,
            b'A'...b'F' => b - b'A' + 10,
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
            b'0'...b'9' => b - b'0',
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

//! Manage xml character escapes

use std::borrow::Cow;
use errors::Result;
use errors::ErrorKind::Escape;

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
pub fn escape(raw: &[u8]) -> Vec<u8> {
    let mut escaped = Vec::with_capacity(raw.len());
    for b in raw {
        match *b {
            b'<' => escaped.extend_from_slice(b"&lt;"),
            b'>' => escaped.extend_from_slice(b"&gt;"),
            b'\'' => escaped.extend_from_slice(b"&apos;"),
            b'&' => escaped.extend_from_slice(b"&amp;"),
            b'"' => escaped.extend_from_slice(b"&quot;"),
            _ => escaped.push(*b),
        }
    }
    escaped
}

/// helper function to unescape a `&[u8]` and replace all
/// xml escaped characters ('&...;') into their corresponding value
pub fn unescape(raw: &[u8]) -> Result<Cow<[u8]>> {
    let mut escapes = Vec::new();

    let mut start = 0;
    let mut bytes = raw.iter();
    while let Some(i) = bytes.by_ref().position(|b| *b == b'&') {
        start += i;
        if let Some(j) = bytes.by_ref().position(|b| *b == b';') {
            let end = start + j + 1;
            // search for character correctness
            let b_o_c = match &raw[start + 1..end] {
                b"lt" => ByteOrChar::Byte(b'<'),
                b"gt" => ByteOrChar::Byte(b'>'),
                b"amp" => ByteOrChar::Byte(b'&'),
                b"apos" => ByteOrChar::Byte(b'\''),
                b"quot" => ByteOrChar::Byte(b'\"'),
                b"#x0" | b"#0" => {
                    bail!(Escape("Null character entity not allowed".to_string(), start..end))
                }
                bytes if bytes.starts_with(b"#x") => {
                    ByteOrChar::Char(parse_hexadecimal(&bytes[2..])?)
                }
                bytes if bytes.starts_with(b"#") => ByteOrChar::Char(parse_decimal(&bytes[1..])?),
                _ => bail!(Escape("".to_string(), start..end)),
            };
            escapes.push((start..end, b_o_c));
            start = end + 1;
        } else {
            bail!(Escape("Cannot find ';' after '&'".to_string(), i..bytes.len()));
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

fn parse_hexadecimal(bytes: &[u8]) -> Result<u32> {
    let mut code = 0;
    for &b in bytes {
        code <<= 4;
        code += match b {
            b'0'...b'9' => b - b'0',
            b'a'...b'f' => b - b'a' + 10,
            b'A'...b'F' => b - b'A' + 10,
            b => bail!("'{}' is not a valid hexadecimal character", b as char),
        } as u32;
    }
    Ok(code)
}

fn parse_decimal(bytes: &[u8]) -> Result<u32> {
    let mut code = 0;
    for &b in bytes {
        code *= 10;
        code += match b {
            b'0'...b'9' => b - b'0',
            b => bail!("'{}' is not a valid decimal character", b as char),
        } as u32;
    }
    Ok(code)
}

#[test]
fn test_escape() {
    assert_eq!(&*unescape(b"test").unwrap(), b"test");
    assert_eq!(&*unescape(b"&lt;test&gt;").unwrap(), b"<test>");
    println!("{}",
             ::std::str::from_utf8(&*unescape(b"&#xa9;").unwrap()).unwrap());
    assert_eq!(&*unescape(b"&#x30;").unwrap(), b"0");
    assert_eq!(&*unescape(b"&#48;").unwrap(), b"0");
    assert_eq!(&*unescape(b"&#x30;").unwrap(), b"0");
}

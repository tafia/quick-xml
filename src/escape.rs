//! Manage xml character escapes

use std::borrow::Cow;
use errors::Result;
use errors::ErrorKind::Escape;
use memchr;

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
            _ => unreachable!(),
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
            start += i + 1;
            v.extend_from_slice(r);
        }

        if start < len {
            v.extend_from_slice(&raw[start..]);
        }
        Cow::Owned(v)
    }
}

/// helper function to unescape a `&[u8]` and replace all
/// xml escaped characters ('&...;') into their corresponding value
pub fn unescape(raw: &[u8]) -> Result<Cow<[u8]>> {
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
                b"#x0" | b"#0" => bail!(Escape(
                    "Null character entity not allowed".to_string(),
                    start..end
                )),
                bytes if bytes.starts_with(b"#x") => {
                    ByteOrChar::Char(parse_hexadecimal(&bytes[2..])?)
                }
                bytes if bytes.starts_with(b"#") => ByteOrChar::Char(parse_decimal(&bytes[1..])?),
                bytes => bail!(Escape(
                    format!(
                        "Unrecognized escape symbol: {:?}",
                        ::std::str::from_utf8(bytes)
                    ),
                    start..end
                )),
            };
            escapes.push((start - 1..end, b_o_c));
            start = end + 1;
        } else {
            bail!(Escape(
                "Cannot find ';' after '&'".to_string(),
                start..raw.len()
            ));
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
    println!(
        "{}",
        ::std::str::from_utf8(&*unescape(b"&#xa9;").unwrap()).unwrap()
    );
    assert_eq!(&*unescape(b"&#x30;").unwrap(), b"0");
    assert_eq!(&*unescape(b"&#48;").unwrap(), b"0");
    assert_eq!(&*unescape(b"&#x30;").unwrap(), b"0");
}

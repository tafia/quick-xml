//! Manage xml character escapes

use std::borrow::Cow;
use error::{Error, ResultPos};
use AsStr;

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

/// helper function to unescape a `&[u8]` and replace all
/// xml escaped characters ('&...;') into their corresponding value
pub fn unescape(raw: &[u8]) -> ResultPos<Cow<[u8]>> {
    let mut escapes = Vec::new();
    let mut bytes = raw.iter().enumerate();
    while let Some((i, &b)) = bytes.next() {
        if b == b'&' {
            if let Some((j, _)) = bytes.find(|&(_, &b)| b == b';') {
                // search for character correctness
                // copied and modified from xml-rs inside_reference.rs
                match &raw[(i + 1)..j] {
                    b"lt" => escapes.push((i..j, ByteOrChar::Byte(b'<'))),
                    b"gt" => escapes.push((i..j, ByteOrChar::Byte(b'>'))),
                    b"amp" => escapes.push((i..j, ByteOrChar::Byte(b'&'))),
                    b"apos" => escapes.push((i..j, ByteOrChar::Byte(b'\''))),
                    b"quot" => escapes.push((i..j, ByteOrChar::Byte(b'\"'))),
                    b"" => return Err((Error::Malformed("Encountered empty entity".to_owned()), i)),
                    b"#x0" | b"#0" => {
                        return Err((Error::Malformed("Null character entity is not allowed"
                            .to_owned()),
                                    i))
                    }
                    bytes if bytes.len() > 1 && bytes[0] == b'#' => {
                        if bytes[1] == b'x' {
                            let name = try!(bytes[2..].as_str().map_err(|e| (Error::from(e), i)));
                            match u32::from_str_radix(name, 16).ok() {
                                Some(c) => escapes.push((i..j, ByteOrChar::Char(c))),
                                None => {
                                    return Err((Error::Malformed(format!("Invalid hexadecimal \
                                                                          character number in \
                                                                          an entity: {}",
                                                                         name)),
                                                i))
                                }
                            }
                        } else {
                            let name = try!(bytes[1..].as_str().map_err(|e| (Error::from(e), i)));
                            match u32::from_str_radix(name, 10).ok() {
                                Some(c) => escapes.push((i..j, ByteOrChar::Char(c))),
                                None => {
                                    return Err((Error::Malformed(format!("Invalid decimal \
                                                                          character number in \
                                                                          an entity: {}",
                                                                         name)),
                                                i))
                                }
                            }
                        }
                    }
                    bytes => {
                        return Err((Error::Malformed(format!("Unexpected entity: {:?}",
                                                             bytes.as_str())),
                                    i))
                    }
                }
            } else {
                return Err((Error::Malformed("Cannot find ';' after '&'".to_owned()), i));
            }
        }
    }
    if escapes.is_empty() {
        Ok(Cow::Borrowed(raw))
    } else {
        let len = escapes.iter().fold(raw.len(), |c, &(ref r, _)| c - (r.end - r.start));
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

#[test]
fn test_escape() {
    assert_eq!(&*unescape(b"test").unwrap(), b"test");
    assert_eq!(&*unescape(b"&lt;test&gt;").unwrap(), b"<test>");
    println!("{}", &*unescape(b"&#xa9;").unwrap().as_str().unwrap());
    assert_eq!(&*unescape(b"&#x30;").unwrap(), b"0");
    assert_eq!(&*unescape(b"&#48;").unwrap(), b"0");
}

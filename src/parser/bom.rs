//! A parser for encoding detection using BOM and heuristics.

/// A result of feeding data into [`BomParser`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FeedResult {
    /// All fed bytes should be consumed, new portion should be feed.
    NeedData,
    /// Encoding detected as UTF-16 Big-Endian based on the first 4 bytes of content.
    /// Nothing should be consumed.
    Utf16Be,
    /// Encoding detected as UTF-16 Little-Endian based on the first 4 bytes of content.
    /// Nothing should be consumed.
    Utf16Le,
    /// Encoding detected as UTF-8 on the first 4 bytes of content.
    /// Nothing should be consumed.
    Utf8,
    /// Encoding detected as UTF-16 Big-Endian based on the first 4 bytes of content.
    /// The 2 bytes of BOM should be consumed.
    Utf16BeBom,
    /// Encoding detected as UTF-16 Little-Endian based on the first 4 bytes of content.
    /// The 2 bytes of BOM should be consumed.
    Utf16LeBom,
    /// Encoding detected as UTF-8 based on the first 3 bytes of content.
    /// The 3 bytes of BOM should be consumed.
    Utf8Bom,
    /// Encoding was not recognized. Nothing should be consumed.
    Unknown,
}

/// Implements automatic encoding detection of XML using the
/// [recommended algorithm](https://www.w3.org/TR/xml11/#sec-guessing).
///
/// IF encoding was not recognized, [`FeedResult::Unknown`] is returned, otherwise
/// `Utf*` variant is returned.
///
/// Because the [`encoding_rs`] crate supports only subset of those encodings, only
/// the supported subset are detected, which is UTF-8, UTF-16 BE and UTF-16 LE.
///
/// The algorithm suggests examine up to the first 4 bytes to determine encoding
/// according to the following table:
///
/// | Bytes       |Detected encoding
/// |-------------|------------------------------------------
/// | **BOM**
/// |`FE_FF_##_##`|UTF-16, big-endian
/// |`FF FE ## ##`|UTF-16, little-endian
/// |`EF BB BF`   |UTF-8
/// | **No BOM**
/// |`00 3C 00 3F`|UTF-16 BE or ISO-10646-UCS-2 BE or similar 16-bit BE (use declared encoding to find the exact one)
/// |`3C 00 3F 00`|UTF-16 LE or ISO-10646-UCS-2 LE or similar 16-bit LE (use declared encoding to find the exact one)
/// |`3C 3F 78 6D`|UTF-8, ISO 646, ASCII, some part of ISO 8859, Shift-JIS, EUC, or any other 7-bit, 8-bit, or mixed-width encoding which ensures that the characters of ASCII have their normal positions, width, and values; the actual encoding declaration must be read to detect which of these applies, but since all of these encodings use the same bit patterns for the relevant ASCII characters, the encoding declaration itself may be read reliably
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub enum BomParser {
    X00,
    X00_3C,
    X00_3C_00,

    X3C,
    X3C_00,
    X3C_00_3F,

    X3C_3F,
    X3C_3F_78, // <?x

    XFE,

    XFF,

    XEF,
    XEF_BB,
}
impl BomParser {
    pub fn feed(&mut self, bytes: &[u8]) -> FeedResult {
        for &byte in bytes.iter() {
            *self = match self {
                //----------------------------------------------------------------------------------
                // UTF-16 BE without BOM    00 < 00 ?
                //----------------------------------------------------------------------------------
                Self::X00 => match byte {
                    b'<' => Self::X00_3C,
                    _ => return FeedResult::Unknown,
                },
                Self::X00_3C => match byte {
                    0x00 => Self::X00_3C_00,
                    _ => return FeedResult::Unknown,
                },
                Self::X00_3C_00 => match byte {
                    b'?' => return FeedResult::Utf16Be,
                    _ => return FeedResult::Unknown,
                },
                //----------------------------------------------------------------------------------
                // UTF-16 LE without BOM    < 00 ? 00
                //----------------------------------------------------------------------------------
                Self::X3C => match byte {
                    0x00 => Self::X3C_00,
                    b'?' => Self::X3C_3F,
                    _ => return FeedResult::Unknown,
                },
                Self::X3C_00 => match byte {
                    b'?' => Self::X3C_00_3F,
                    _ => return FeedResult::Unknown,
                },
                Self::X3C_00_3F => match byte {
                    0x00 => return FeedResult::Utf16Le,
                    _ => return FeedResult::Unknown,
                },
                //----------------------------------------------------------------------------------
                // UTF-8-like without BOM   < ? x m
                //----------------------------------------------------------------------------------
                Self::X3C_3F => match byte {
                    b'x' => Self::X3C_3F_78,
                    _ => return FeedResult::Unknown,
                },
                Self::X3C_3F_78 => match byte {
                    b'm' => return FeedResult::Utf8,
                    _ => return FeedResult::Unknown,
                },
                //----------------------------------------------------------------------------------
                // UTF-16 BE with BOM       FE FF
                //----------------------------------------------------------------------------------
                Self::XFE => match byte {
                    0xFF => return FeedResult::Utf16BeBom,
                    _ => return FeedResult::Unknown,
                },
                //----------------------------------------------------------------------------------
                // UTF-16 LE with BOM       FF FE
                //----------------------------------------------------------------------------------
                Self::XFF => match byte {
                    0xFE => return FeedResult::Utf16LeBom,
                    _ => return FeedResult::Unknown,
                },
                //----------------------------------------------------------------------------------
                // UTF-8 with BOM           EF BB
                //----------------------------------------------------------------------------------
                Self::XEF => match byte {
                    0xBB => Self::XEF_BB,
                    _ => return FeedResult::Unknown,
                },
                Self::XEF_BB => match byte {
                    0xBF => return FeedResult::Utf8Bom,
                    _ => return FeedResult::Unknown,
                },
            }
        }
        FeedResult::NeedData
    }
}

//! Manage xml character escapes

use memchr;
use std::borrow::Cow;
use std::collections::HashMap;

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
    do_unescape(raw, None)
}

/// Unescape a `&[u8]` and replaces all xml escaped characters ('&...;') into their corresponding
/// value, using a dictionnary of custom entities.
///
/// # Pre-condition
///
/// The keys and values of `custom_entities`, if any, must be valid UTF-8.
pub fn unescape_with<'a>(
    raw: &'a [u8],
    custom_entities: &HashMap<Vec<u8>, Vec<u8>>,
) -> Result<Cow<'a, [u8]>, EscapeError> {
    do_unescape(raw, Some(custom_entities))
}

/// Unescape a `&[u8]` and replaces all xml escaped characters ('&...;') into their corresponding
/// value, using an optional dictionnary of custom entities.
///
/// # Pre-condition
///
/// The keys and values of `custom_entities`, if any, must be valid UTF-8.
pub fn do_unescape<'a>(
    raw: &'a [u8],
    custom_entities: Option<&HashMap<Vec<u8>, Vec<u8>>>,
) -> Result<Cow<'a, [u8]>, EscapeError> {
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
                #[cfg(not(feature = "escape-html"))]
                match &raw[start + 1..end] {
                    b"lt" => unescaped.push(b'<'),
                    b"gt" => unescaped.push(b'>'),
                    b"amp" => unescaped.push(b'&'),
                    b"apos" => unescaped.push(b'\''),
                    b"quot" => unescaped.push(b'\"'),
                    bytes if bytes.starts_with(b"#") => {
                        let bytes = &bytes[1..];
                        let code = if bytes.starts_with(b"x") {
                            parse_hexadecimal(&bytes[1..])
                        } else {
                            parse_decimal(&bytes)
                        }?;
                        if code == 0 {
                            return Err(EscapeError::EntityWithNull(start..end));
                        }
                        push_utf8(unescaped, code);
                    }
                    bytes => match custom_entities.and_then(|hm| hm.get(bytes)) {
                        Some(value) => unescaped.extend_from_slice(&value),
                        None => {
                            return Err(EscapeError::UnrecognizedSymbol(
                                start + 1..end,
                                String::from_utf8(bytes.to_vec()),
                            ))
                        }
                    },
                }

                #[cfg(feature = "escape-html")]
                match &raw[start + 1..end] {
                    // imported from https://dev.w3.org/html5/html-author/charref
                    b"Tab" => unescaped.push(b'\x09'),
                    b"NewLine" => unescaped.push(b'\x0A'),
                    b"excl" => {
                        unescaped.push(b'\x21');
                    }
                    b"quot" | b"QUOT" => {
                        unescaped.push(b'\x22');
                    }
                    b"num" => {
                        unescaped.push(b'\x23');
                    }
                    b"dollar" => {
                        unescaped.push(b'\x24');
                    }
                    b"percnt" => {
                        unescaped.push(b'\x25');
                    }
                    b"amp" | b"AMP" => {
                        unescaped.push(b'\x26');
                    }
                    b"apos" => {
                        unescaped.push(b'\x27');
                    }
                    b"lpar" => {
                        unescaped.push(b'\x28');
                    }
                    b"rpar" => {
                        unescaped.push(b'\x29');
                    }
                    b"ast" | b"midast" => {
                        unescaped.push(b'\x2A');
                    }
                    b"plus" => {
                        unescaped.push(b'\x2B');
                    }
                    b"comma" => {
                        unescaped.push(b'\x2C');
                    }
                    b"period" => {
                        unescaped.push(b'\x2E');
                    }
                    b"sol" => {
                        unescaped.push(b'\x2F');
                    }
                    b"colon" => {
                        unescaped.push(b'\x3A');
                    }
                    b"semi" => {
                        unescaped.push(b'\x3B');
                    }
                    b"lt" | b"LT" => {
                        unescaped.push(b'\x3C');
                    }
                    b"equals" => {
                        unescaped.push(b'\x3D');
                    }
                    b"gt" | b"GT" => {
                        unescaped.push(b'\x3E');
                    }
                    b"quest" => {
                        unescaped.push(b'\x3F');
                    }
                    b"commat" => {
                        unescaped.push(b'\x40');
                    }
                    b"lsqb" | b"lbrack" => {
                        unescaped.push(b'\x5B');
                    }
                    b"bsol" => {
                        unescaped.push(b'\x5C');
                    }
                    b"rsqb" | b"rbrack" => {
                        unescaped.push(b'\x5D');
                    }
                    b"Hat" => {
                        unescaped.push(b'\x5E');
                    }
                    b"lowbar" => {
                        unescaped.push(b'\x5F');
                    }
                    b"grave" | b"DiacriticalGrave" => {
                        unescaped.push(b'\x60');
                    }
                    b"lcub" | b"lbrace" => {
                        unescaped.push(b'\x7B');
                    }
                    b"verbar" | b"vert" | b"VerticalLine" => {
                        unescaped.push(b'\x7C');
                    }
                    b"rcub" | b"rbrace" => {
                        unescaped.push(b'\x7D');
                    }
                    b"nbsp" | b"NonBreakingSpace" => {
                        unescaped.push(b'\xA0');
                    }
                    b"iexcl" => {
                        unescaped.push(b'\xA1');
                    }
                    b"cent" => {
                        unescaped.push(b'\xA2');
                    }
                    b"pound" => {
                        unescaped.push(b'\xA3');
                    }
                    b"curren" => {
                        unescaped.push(b'\xA4');
                    }
                    b"yen" => {
                        unescaped.push(b'\xA5');
                    }
                    b"brvbar" => {
                        unescaped.push(b'\xA6');
                    }
                    b"sect" => {
                        unescaped.push(b'\xA7');
                    }
                    b"Dot" | b"die" | b"DoubleDot" | b"uml" => {
                        unescaped.push(b'\xA8');
                    }
                    b"copy" | b"COPY" => {
                        unescaped.push(b'\xA9');
                    }
                    b"ordf" => {
                        unescaped.push(b'\xAA');
                    }
                    b"laquo" => {
                        unescaped.push(b'\xAB');
                    }
                    b"not" => {
                        unescaped.push(b'\xAC');
                    }
                    b"shy" => {
                        unescaped.push(b'\xAD');
                    }
                    b"reg" | b"circledR" | b"REG" => {
                        unescaped.push(b'\xAE');
                    }
                    b"macr" | b"OverBar" | b"strns" => {
                        unescaped.push(b'\xAF');
                    }
                    b"deg" => {
                        unescaped.push(b'\xB0');
                    }
                    b"plusmn" | b"pm" | b"PlusMinus" => {
                        unescaped.push(b'\xB1');
                    }
                    b"sup2" => {
                        unescaped.push(b'\xB2');
                    }
                    b"sup3" => {
                        unescaped.push(b'\xB3');
                    }
                    b"acute" | b"DiacriticalAcute" => {
                        unescaped.push(b'\xB4');
                    }
                    b"micro" => {
                        unescaped.push(b'\xB5');
                    }
                    b"para" => {
                        unescaped.push(b'\xB6');
                    }
                    b"middot" | b"centerdot" | b"CenterDot" => {
                        unescaped.push(b'\xB7');
                    }
                    b"cedil" | b"Cedilla" => {
                        unescaped.push(b'\xB8');
                    }
                    b"sup1" => {
                        unescaped.push(b'\xB9');
                    }
                    b"ordm" => {
                        unescaped.push(b'\xBA');
                    }
                    b"raquo" => {
                        unescaped.push(b'\xBB');
                    }
                    b"frac14" => {
                        unescaped.push(b'\xBC');
                    }
                    b"frac12" | b"half" => {
                        unescaped.push(b'\xBD');
                    }
                    b"frac34" => {
                        unescaped.push(b'\xBE');
                    }
                    b"iquest" => {
                        unescaped.push(b'\xBF');
                    }
                    b"Agrave" => {
                        unescaped.push(b'\xC0');
                    }
                    b"Aacute" => {
                        unescaped.push(b'\xC1');
                    }
                    b"Acirc" => {
                        unescaped.push(b'\xC2');
                    }
                    b"Atilde" => {
                        unescaped.push(b'\xC3');
                    }
                    b"Auml" => {
                        unescaped.push(b'\xC4');
                    }
                    b"Aring" => {
                        unescaped.push(b'\xC5');
                    }
                    b"AElig" => {
                        unescaped.push(b'\xC6');
                    }
                    b"Ccedil" => {
                        unescaped.push(b'\xC7');
                    }
                    b"Egrave" => {
                        unescaped.push(b'\xC8');
                    }
                    b"Eacute" => {
                        unescaped.push(b'\xC9');
                    }
                    b"Ecirc" => {
                        unescaped.push(b'\xCA');
                    }
                    b"Euml" => {
                        unescaped.push(b'\xCB');
                    }
                    b"Igrave" => {
                        unescaped.push(b'\xCC');
                    }
                    b"Iacute" => {
                        unescaped.push(b'\xCD');
                    }
                    b"Icirc" => {
                        unescaped.push(b'\xCE');
                    }
                    b"Iuml" => {
                        unescaped.push(b'\xCF');
                    }
                    b"ETH" => {
                        unescaped.push(b'\xD0');
                    }
                    b"Ntilde" => {
                        unescaped.push(b'\xD1');
                    }
                    b"Ograve" => {
                        unescaped.push(b'\xD2');
                    }
                    b"Oacute" => {
                        unescaped.push(b'\xD3');
                    }
                    b"Ocirc" => {
                        unescaped.push(b'\xD4');
                    }
                    b"Otilde" => {
                        unescaped.push(b'\xD5');
                    }
                    b"Ouml" => {
                        unescaped.push(b'\xD6');
                    }
                    b"times" => {
                        unescaped.push(b'\xD7');
                    }
                    b"Oslash" => {
                        unescaped.push(b'\xD8');
                    }
                    b"Ugrave" => {
                        unescaped.push(b'\xD9');
                    }
                    b"Uacute" => {
                        unescaped.push(b'\xDA');
                    }
                    b"Ucirc" => {
                        unescaped.push(b'\xDB');
                    }
                    b"Uuml" => {
                        unescaped.push(b'\xDC');
                    }
                    b"Yacute" => {
                        unescaped.push(b'\xDD');
                    }
                    b"THORN" => {
                        unescaped.push(b'\xDE');
                    }
                    b"szlig" => {
                        unescaped.push(b'\xDF');
                    }
                    b"agrave" => {
                        unescaped.push(b'\xE0');
                    }
                    b"aacute" => {
                        unescaped.push(b'\xE1');
                    }
                    b"acirc" => {
                        unescaped.push(b'\xE2');
                    }
                    b"atilde" => {
                        unescaped.push(b'\xE3');
                    }
                    b"auml" => {
                        unescaped.push(b'\xE4');
                    }
                    b"aring" => {
                        unescaped.push(b'\xE5');
                    }
                    b"aelig" => {
                        unescaped.push(b'\xE6');
                    }
                    b"ccedil" => {
                        unescaped.push(b'\xE7');
                    }
                    b"egrave" => {
                        unescaped.push(b'\xE8');
                    }
                    b"eacute" => {
                        unescaped.push(b'\xE9');
                    }
                    b"ecirc" => {
                        unescaped.push(b'\xEA');
                    }
                    b"euml" => {
                        unescaped.push(b'\xEB');
                    }
                    b"igrave" => {
                        unescaped.push(b'\xEC');
                    }
                    b"iacute" => {
                        unescaped.push(b'\xED');
                    }
                    b"icirc" => {
                        unescaped.push(b'\xEE');
                    }
                    b"iuml" => {
                        unescaped.push(b'\xEF');
                    }
                    b"eth" => {
                        unescaped.push(b'\xF0');
                    }
                    b"ntilde" => {
                        unescaped.push(b'\xF1');
                    }
                    b"ograve" => {
                        unescaped.push(b'\xF2');
                    }
                    b"oacute" => {
                        unescaped.push(b'\xF3');
                    }
                    b"ocirc" => {
                        unescaped.push(b'\xF4');
                    }
                    b"otilde" => {
                        unescaped.push(b'\xF5');
                    }
                    b"ouml" => {
                        unescaped.push(b'\xF6');
                    }
                    b"divide" | b"div" => {
                        unescaped.push(b'\xF7');
                    }
                    b"oslash" => {
                        unescaped.push(b'\xF8');
                    }
                    b"ugrave" => {
                        unescaped.push(b'\xF9');
                    }
                    b"uacute" => {
                        unescaped.push(b'\xFA');
                    }
                    b"ucirc" => {
                        unescaped.push(b'\xFB');
                    }
                    b"uuml" => {
                        unescaped.push(b'\xFC');
                    }
                    b"yacute" => {
                        unescaped.push(b'\xFD');
                    }
                    b"thorn" => {
                        unescaped.push(b'\xFE');
                    }
                    b"yuml" => {
                        unescaped.push(b'\xFF');
                    }
                    b"Amacr" => {
                        unescaped.push(b'\x10');
                    }
                    b"amacr" => {
                        unescaped.push(b'\x10');
                    }
                    b"Abreve" => {
                        unescaped.push(b'\x10');
                    }
                    b"abreve" => {
                        unescaped.push(b'\x10');
                    }
                    b"Aogon" => {
                        unescaped.push(b'\x10');
                    }
                    b"aogon" => {
                        unescaped.push(b'\x10');
                    }
                    b"Cacute" => {
                        unescaped.push(b'\x10');
                    }
                    b"cacute" => {
                        unescaped.push(b'\x10');
                    }
                    b"Ccirc" => {
                        unescaped.push(b'\x10');
                    }
                    b"ccirc" => {
                        unescaped.push(b'\x10');
                    }
                    b"Cdot" => {
                        unescaped.push(b'\x10');
                    }
                    b"cdot" => {
                        unescaped.push(b'\x10');
                    }
                    b"Ccaron" => {
                        unescaped.push(b'\x10');
                    }
                    b"ccaron" => {
                        unescaped.push(b'\x10');
                    }
                    b"Dcaron" => {
                        unescaped.push(b'\x10');
                    }
                    b"dcaron" => {
                        unescaped.push(b'\x10');
                    }
                    b"Dstrok" => {
                        unescaped.push(b'\x11');
                    }
                    b"dstrok" => {
                        unescaped.push(b'\x11');
                    }
                    b"Emacr" => {
                        unescaped.push(b'\x11');
                    }
                    b"emacr" => {
                        unescaped.push(b'\x11');
                    }
                    b"Edot" => {
                        unescaped.push(b'\x11');
                    }
                    b"edot" => {
                        unescaped.push(b'\x11');
                    }
                    b"Eogon" => {
                        unescaped.push(b'\x11');
                    }
                    b"eogon" => {
                        unescaped.push(b'\x11');
                    }
                    b"Ecaron" => {
                        unescaped.push(b'\x11');
                    }
                    b"ecaron" => {
                        unescaped.push(b'\x11');
                    }
                    b"Gcirc" => {
                        unescaped.push(b'\x11');
                    }
                    b"gcirc" => {
                        unescaped.push(b'\x11');
                    }
                    b"Gbreve" => {
                        unescaped.push(b'\x11');
                    }
                    b"gbreve" => {
                        unescaped.push(b'\x11');
                    }
                    b"Gdot" => {
                        unescaped.push(b'\x12');
                    }
                    b"gdot" => {
                        unescaped.push(b'\x12');
                    }
                    b"Gcedil" => {
                        unescaped.push(b'\x12');
                    }
                    b"Hcirc" => {
                        unescaped.push(b'\x12');
                    }
                    b"hcirc" => {
                        unescaped.push(b'\x12');
                    }
                    b"Hstrok" => {
                        unescaped.push(b'\x12');
                    }
                    b"hstrok" => {
                        unescaped.push(b'\x12');
                    }
                    b"Itilde" => {
                        unescaped.push(b'\x12');
                    }
                    b"itilde" => {
                        unescaped.push(b'\x12');
                    }
                    b"Imacr" => {
                        unescaped.push(b'\x12');
                    }
                    b"imacr" => {
                        unescaped.push(b'\x12');
                    }
                    b"Iogon" => {
                        unescaped.push(b'\x12');
                    }
                    b"iogon" => {
                        unescaped.push(b'\x12');
                    }
                    b"Idot" => {
                        unescaped.push(b'\x13');
                    }
                    b"imath" | b"inodot" => {
                        unescaped.push(b'\x13');
                    }
                    b"IJlig" => {
                        unescaped.push(b'\x13');
                    }
                    b"ijlig" => {
                        unescaped.push(b'\x13');
                    }
                    b"Jcirc" => {
                        unescaped.push(b'\x13');
                    }
                    b"jcirc" => {
                        unescaped.push(b'\x13');
                    }
                    b"Kcedil" => {
                        unescaped.push(b'\x13');
                    }
                    b"kcedil" => {
                        unescaped.push(b'\x13');
                    }
                    b"kgreen" => {
                        unescaped.push(b'\x13');
                    }
                    b"Lacute" => {
                        unescaped.push(b'\x13');
                    }
                    b"lacute" => {
                        unescaped.push(b'\x13');
                    }
                    b"Lcedil" => {
                        unescaped.push(b'\x13');
                    }
                    b"lcedil" => {
                        unescaped.push(b'\x13');
                    }
                    b"Lcaron" => {
                        unescaped.push(b'\x13');
                    }
                    b"lcaron" => {
                        unescaped.push(b'\x13');
                    }
                    b"Lmidot" => {
                        unescaped.push(b'\x13');
                    }
                    b"lmidot" => {
                        unescaped.push(b'\x14');
                    }
                    b"Lstrok" => {
                        unescaped.push(b'\x14');
                    }
                    b"lstrok" => {
                        unescaped.push(b'\x14');
                    }
                    b"Nacute" => {
                        unescaped.push(b'\x14');
                    }
                    b"nacute" => {
                        unescaped.push(b'\x14');
                    }
                    b"Ncedil" => {
                        unescaped.push(b'\x14');
                    }
                    b"ncedil" => {
                        unescaped.push(b'\x14');
                    }
                    b"Ncaron" => {
                        unescaped.push(b'\x14');
                    }
                    b"ncaron" => {
                        unescaped.push(b'\x14');
                    }
                    b"napos" => {
                        unescaped.push(b'\x14');
                    }
                    b"ENG" => {
                        unescaped.push(b'\x14');
                    }
                    b"eng" => {
                        unescaped.push(b'\x14');
                    }
                    b"Omacr" => {
                        unescaped.push(b'\x14');
                    }
                    b"omacr" => {
                        unescaped.push(b'\x14');
                    }
                    b"Odblac" => {
                        unescaped.push(b'\x15');
                    }
                    b"odblac" => {
                        unescaped.push(b'\x15');
                    }
                    b"OElig" => {
                        unescaped.push(b'\x15');
                    }
                    b"oelig" => {
                        unescaped.push(b'\x15');
                    }
                    b"Racute" => {
                        unescaped.push(b'\x15');
                    }
                    b"racute" => {
                        unescaped.push(b'\x15');
                    }
                    b"Rcedil" => {
                        unescaped.push(b'\x15');
                    }
                    b"rcedil" => {
                        unescaped.push(b'\x15');
                    }
                    b"Rcaron" => {
                        unescaped.push(b'\x15');
                    }
                    b"rcaron" => {
                        unescaped.push(b'\x15');
                    }
                    b"Sacute" => {
                        unescaped.push(b'\x15');
                    }
                    b"sacute" => {
                        unescaped.push(b'\x15');
                    }
                    b"Scirc" => {
                        unescaped.push(b'\x15');
                    }
                    b"scirc" => {
                        unescaped.push(b'\x15');
                    }
                    b"Scedil" => {
                        unescaped.push(b'\x15');
                    }
                    b"scedil" => {
                        unescaped.push(b'\x15');
                    }
                    b"Scaron" => {
                        unescaped.push(b'\x16');
                    }
                    b"scaron" => {
                        unescaped.push(b'\x16');
                    }
                    b"Tcedil" => {
                        unescaped.push(b'\x16');
                    }
                    b"tcedil" => {
                        unescaped.push(b'\x16');
                    }
                    b"Tcaron" => {
                        unescaped.push(b'\x16');
                    }
                    b"tcaron" => {
                        unescaped.push(b'\x16');
                    }
                    b"Tstrok" => {
                        unescaped.push(b'\x16');
                    }
                    b"tstrok" => {
                        unescaped.push(b'\x16');
                    }
                    b"Utilde" => {
                        unescaped.push(b'\x16');
                    }
                    b"utilde" => {
                        unescaped.push(b'\x16');
                    }
                    b"Umacr" => {
                        unescaped.push(b'\x16');
                    }
                    b"umacr" => {
                        unescaped.push(b'\x16');
                    }
                    b"Ubreve" => {
                        unescaped.push(b'\x16');
                    }
                    b"ubreve" => {
                        unescaped.push(b'\x16');
                    }
                    b"Uring" => {
                        unescaped.push(b'\x16');
                    }
                    b"uring" => {
                        unescaped.push(b'\x16');
                    }
                    b"Udblac" => {
                        unescaped.push(b'\x17');
                    }
                    b"udblac" => {
                        unescaped.push(b'\x17');
                    }
                    b"Uogon" => {
                        unescaped.push(b'\x17');
                    }
                    b"uogon" => {
                        unescaped.push(b'\x17');
                    }
                    b"Wcirc" => {
                        unescaped.push(b'\x17');
                    }
                    b"wcirc" => {
                        unescaped.push(b'\x17');
                    }
                    b"Ycirc" => {
                        unescaped.push(b'\x17');
                    }
                    b"ycirc" => {
                        unescaped.push(b'\x17');
                    }
                    b"Yuml" => {
                        unescaped.push(b'\x17');
                    }
                    b"Zacute" => {
                        unescaped.push(b'\x17');
                    }
                    b"zacute" => {
                        unescaped.push(b'\x17');
                    }
                    b"Zdot" => {
                        unescaped.push(b'\x17');
                    }
                    b"zdot" => {
                        unescaped.push(b'\x17');
                    }
                    b"Zcaron" => {
                        unescaped.push(b'\x17');
                    }
                    b"zcaron" => {
                        unescaped.push(b'\x17');
                    }
                    b"fnof" => {
                        unescaped.push(b'\x19');
                    }
                    b"imped" => {
                        unescaped.push(b'\x1B');
                    }
                    b"gacute" => {
                        unescaped.push(b'\x1F');
                    }
                    b"jmath" => {
                        unescaped.push(b'\x23');
                    }
                    b"circ" => {
                        unescaped.push(b'\x2C');
                    }
                    b"caron" | b"Hacek" => {
                        unescaped.push(b'\x2C');
                    }
                    b"breve" | b"Breve" => {
                        unescaped.push(b'\x2D');
                    }
                    b"dot" | b"DiacriticalDot" => {
                        unescaped.push(b'\x2D');
                    }
                    b"ring" => {
                        unescaped.push(b'\x2D');
                    }
                    b"ogon" => {
                        unescaped.push(b'\x2D');
                    }
                    b"tilde" | b"DiacriticalTilde" => {
                        unescaped.push(b'\x2D');
                    }
                    b"dblac" | b"DiacriticalDoubleAcute" => {
                        unescaped.push(b'\x2D');
                    }
                    b"DownBreve" => {
                        unescaped.push(b'\x31');
                    }
                    b"UnderBar" => {
                        unescaped.push(b'\x33');
                    }
                    b"Alpha" => {
                        unescaped.push(b'\x39');
                    }
                    b"Beta" => {
                        unescaped.push(b'\x39');
                    }
                    b"Gamma" => {
                        unescaped.push(b'\x39');
                    }
                    b"Delta" => {
                        unescaped.push(b'\x39');
                    }
                    b"Epsilon" => {
                        unescaped.push(b'\x39');
                    }
                    b"Zeta" => {
                        unescaped.push(b'\x39');
                    }
                    b"Eta" => {
                        unescaped.push(b'\x39');
                    }
                    b"Theta" => {
                        unescaped.push(b'\x39');
                    }
                    b"Iota" => {
                        unescaped.push(b'\x39');
                    }
                    b"Kappa" => {
                        unescaped.push(b'\x39');
                    }
                    b"Lambda" => {
                        unescaped.push(b'\x39');
                    }
                    b"Mu" => {
                        unescaped.push(b'\x39');
                    }
                    b"Nu" => {
                        unescaped.push(b'\x39');
                    }
                    b"Xi" => {
                        unescaped.push(b'\x39');
                    }
                    b"Omicron" => {
                        unescaped.push(b'\x39');
                    }
                    b"Pi" => {
                        unescaped.push(b'\x3A');
                    }
                    b"Rho" => {
                        unescaped.push(b'\x3A');
                    }
                    b"Sigma" => {
                        unescaped.push(b'\x3A');
                    }
                    b"Tau" => {
                        unescaped.push(b'\x3A');
                    }
                    b"Upsilon" => {
                        unescaped.push(b'\x3A');
                    }
                    b"Phi" => {
                        unescaped.push(b'\x3A');
                    }
                    b"Chi" => {
                        unescaped.push(b'\x3A');
                    }
                    b"Psi" => {
                        unescaped.push(b'\x3A');
                    }
                    b"Omega" => {
                        unescaped.push(b'\x3A');
                    }
                    b"alpha" => {
                        unescaped.push(b'\x3B');
                    }
                    b"beta" => {
                        unescaped.push(b'\x3B');
                    }
                    b"gamma" => {
                        unescaped.push(b'\x3B');
                    }
                    b"delta" => {
                        unescaped.push(b'\x3B');
                    }
                    b"epsiv" | b"varepsilon" | b"epsilon" => {
                        unescaped.push(b'\x3B');
                    }
                    b"zeta" => {
                        unescaped.push(b'\x3B');
                    }
                    b"eta" => {
                        unescaped.push(b'\x3B');
                    }
                    b"theta" => {
                        unescaped.push(b'\x3B');
                    }
                    b"iota" => {
                        unescaped.push(b'\x3B');
                    }
                    b"kappa" => {
                        unescaped.push(b'\x3B');
                    }
                    b"lambda" => {
                        unescaped.push(b'\x3B');
                    }
                    b"mu" => {
                        unescaped.push(b'\x3B');
                    }
                    b"nu" => {
                        unescaped.push(b'\x3B');
                    }
                    b"xi" => {
                        unescaped.push(b'\x3B');
                    }
                    b"omicron" => {
                        unescaped.push(b'\x3B');
                    }
                    b"pi" => {
                        unescaped.push(b'\x3C');
                    }
                    b"rho" => {
                        unescaped.push(b'\x3C');
                    }
                    b"sigmav" | b"varsigma" | b"sigmaf" => {
                        unescaped.push(b'\x3C');
                    }
                    b"sigma" => {
                        unescaped.push(b'\x3C');
                    }
                    b"tau" => {
                        unescaped.push(b'\x3C');
                    }
                    b"upsi" | b"upsilon" => {
                        unescaped.push(b'\x3C');
                    }
                    b"phi" | b"phiv" | b"varphi" => {
                        unescaped.push(b'\x3C');
                    }
                    b"chi" => {
                        unescaped.push(b'\x3C');
                    }
                    b"psi" => {
                        unescaped.push(b'\x3C');
                    }
                    b"omega" => {
                        unescaped.push(b'\x3C');
                    }
                    b"thetav" | b"vartheta" | b"thetasym" => {
                        unescaped.push(b'\x3D');
                    }
                    b"Upsi" | b"upsih" => {
                        unescaped.push(b'\x3D');
                    }
                    b"straightphi" => {
                        unescaped.push(b'\x3D');
                    }
                    b"piv" | b"varpi" => {
                        unescaped.push(b'\x3D');
                    }
                    b"Gammad" => {
                        unescaped.push(b'\x3D');
                    }
                    b"gammad" | b"digamma" => {
                        unescaped.push(b'\x3D');
                    }
                    b"kappav" | b"varkappa" => {
                        unescaped.push(b'\x3F');
                    }
                    b"rhov" | b"varrho" => {
                        unescaped.push(b'\x3F');
                    }
                    b"epsi" | b"straightepsilon" => {
                        unescaped.push(b'\x3F');
                    }
                    b"bepsi" | b"backepsilon" => {
                        unescaped.push(b'\x3F');
                    }
                    b"IOcy" => {
                        unescaped.push(b'\x40');
                    }
                    b"DJcy" => {
                        unescaped.push(b'\x40');
                    }
                    b"GJcy" => {
                        unescaped.push(b'\x40');
                    }
                    b"Jukcy" => {
                        unescaped.push(b'\x40');
                    }
                    b"DScy" => {
                        unescaped.push(b'\x40');
                    }
                    b"Iukcy" => {
                        unescaped.push(b'\x40');
                    }
                    b"YIcy" => {
                        unescaped.push(b'\x40');
                    }
                    b"Jsercy" => {
                        unescaped.push(b'\x40');
                    }
                    b"LJcy" => {
                        unescaped.push(b'\x40');
                    }
                    b"NJcy" => {
                        unescaped.push(b'\x40');
                    }
                    b"TSHcy" => {
                        unescaped.push(b'\x40');
                    }
                    b"KJcy" => {
                        unescaped.push(b'\x40');
                    }
                    b"Ubrcy" => {
                        unescaped.push(b'\x40');
                    }
                    b"DZcy" => {
                        unescaped.push(b'\x40');
                    }
                    b"Acy" => {
                        unescaped.push(b'\x41');
                    }
                    b"Bcy" => {
                        unescaped.push(b'\x41');
                    }
                    b"Vcy" => {
                        unescaped.push(b'\x41');
                    }
                    b"Gcy" => {
                        unescaped.push(b'\x41');
                    }
                    b"Dcy" => {
                        unescaped.push(b'\x41');
                    }
                    b"IEcy" => {
                        unescaped.push(b'\x41');
                    }
                    b"ZHcy" => {
                        unescaped.push(b'\x41');
                    }
                    b"Zcy" => {
                        unescaped.push(b'\x41');
                    }
                    b"Icy" => {
                        unescaped.push(b'\x41');
                    }
                    b"Jcy" => {
                        unescaped.push(b'\x41');
                    }
                    b"Kcy" => {
                        unescaped.push(b'\x41');
                    }
                    b"Lcy" => {
                        unescaped.push(b'\x41');
                    }
                    b"Mcy" => {
                        unescaped.push(b'\x41');
                    }
                    b"Ncy" => {
                        unescaped.push(b'\x41');
                    }
                    b"Ocy" => {
                        unescaped.push(b'\x41');
                    }
                    b"Pcy" => {
                        unescaped.push(b'\x41');
                    }
                    b"Rcy" => {
                        unescaped.push(b'\x42');
                    }
                    b"Scy" => {
                        unescaped.push(b'\x42');
                    }
                    b"Tcy" => {
                        unescaped.push(b'\x42');
                    }
                    b"Ucy" => {
                        unescaped.push(b'\x42');
                    }
                    b"Fcy" => {
                        unescaped.push(b'\x42');
                    }
                    b"KHcy" => {
                        unescaped.push(b'\x42');
                    }
                    b"TScy" => {
                        unescaped.push(b'\x42');
                    }
                    b"CHcy" => {
                        unescaped.push(b'\x42');
                    }
                    b"SHcy" => {
                        unescaped.push(b'\x42');
                    }
                    b"SHCHcy" => {
                        unescaped.push(b'\x42');
                    }
                    b"HARDcy" => {
                        unescaped.push(b'\x42');
                    }
                    b"Ycy" => {
                        unescaped.push(b'\x42');
                    }
                    b"SOFTcy" => {
                        unescaped.push(b'\x42');
                    }
                    b"Ecy" => {
                        unescaped.push(b'\x42');
                    }
                    b"YUcy" => {
                        unescaped.push(b'\x42');
                    }
                    b"YAcy" => {
                        unescaped.push(b'\x42');
                    }
                    b"acy" => {
                        unescaped.push(b'\x43');
                    }
                    b"bcy" => {
                        unescaped.push(b'\x43');
                    }
                    b"vcy" => {
                        unescaped.push(b'\x43');
                    }
                    b"gcy" => {
                        unescaped.push(b'\x43');
                    }
                    b"dcy" => {
                        unescaped.push(b'\x43');
                    }
                    b"iecy" => {
                        unescaped.push(b'\x43');
                    }
                    b"zhcy" => {
                        unescaped.push(b'\x43');
                    }
                    b"zcy" => {
                        unescaped.push(b'\x43');
                    }
                    b"icy" => {
                        unescaped.push(b'\x43');
                    }
                    b"jcy" => {
                        unescaped.push(b'\x43');
                    }
                    b"kcy" => {
                        unescaped.push(b'\x43');
                    }
                    b"lcy" => {
                        unescaped.push(b'\x43');
                    }
                    b"mcy" => {
                        unescaped.push(b'\x43');
                    }
                    b"ncy" => {
                        unescaped.push(b'\x43');
                    }
                    b"ocy" => {
                        unescaped.push(b'\x43');
                    }
                    b"pcy" => {
                        unescaped.push(b'\x43');
                    }
                    b"rcy" => {
                        unescaped.push(b'\x44');
                    }
                    b"scy" => {
                        unescaped.push(b'\x44');
                    }
                    b"tcy" => {
                        unescaped.push(b'\x44');
                    }
                    b"ucy" => {
                        unescaped.push(b'\x44');
                    }
                    b"fcy" => {
                        unescaped.push(b'\x44');
                    }
                    b"khcy" => {
                        unescaped.push(b'\x44');
                    }
                    b"tscy" => {
                        unescaped.push(b'\x44');
                    }
                    b"chcy" => {
                        unescaped.push(b'\x44');
                    }
                    b"shcy" => {
                        unescaped.push(b'\x44');
                    }
                    b"shchcy" => {
                        unescaped.push(b'\x44');
                    }
                    b"hardcy" => {
                        unescaped.push(b'\x44');
                    }
                    b"ycy" => {
                        unescaped.push(b'\x44');
                    }
                    b"softcy" => {
                        unescaped.push(b'\x44');
                    }
                    b"ecy" => {
                        unescaped.push(b'\x44');
                    }
                    b"yucy" => {
                        unescaped.push(b'\x44');
                    }
                    b"yacy" => {
                        unescaped.push(b'\x44');
                    }
                    b"iocy" => {
                        unescaped.push(b'\x45');
                    }
                    b"djcy" => {
                        unescaped.push(b'\x45');
                    }
                    b"gjcy" => {
                        unescaped.push(b'\x45');
                    }
                    b"jukcy" => {
                        unescaped.push(b'\x45');
                    }
                    b"dscy" => {
                        unescaped.push(b'\x45');
                    }
                    b"iukcy" => {
                        unescaped.push(b'\x45');
                    }
                    b"yicy" => {
                        unescaped.push(b'\x45');
                    }
                    b"jsercy" => {
                        unescaped.push(b'\x45');
                    }
                    b"ljcy" => {
                        unescaped.push(b'\x45');
                    }
                    b"njcy" => {
                        unescaped.push(b'\x45');
                    }
                    b"tshcy" => {
                        unescaped.push(b'\x45');
                    }
                    b"kjcy" => {
                        unescaped.push(b'\x45');
                    }
                    b"ubrcy" => {
                        unescaped.push(b'\x45');
                    }
                    b"dzcy" => {
                        unescaped.push(b'\x45');
                    }
                    b"ensp" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x02');
                    }
                    b"emsp" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x03');
                    }
                    b"emsp13" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x04');
                    }
                    b"emsp14" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x05');
                    }
                    b"numsp" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x07');
                    }
                    b"puncsp" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x08');
                    }
                    b"thinsp" | b"ThinSpace" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x09');
                    }
                    b"hairsp" | b"VeryThinSpace" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x0A');
                    }
                    b"ZeroWidthSpace"
                    | b"NegativeVeryThinSpace"
                    | b"NegativeThinSpace"
                    | b"NegativeMediumSpace"
                    | b"NegativeThickSpace" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x0B');
                    }
                    b"zwnj" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x0C');
                    }
                    b"zwj" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x0D');
                    }
                    b"lrm" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x0E');
                    }
                    b"rlm" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x0F');
                    }
                    b"hyphen" | b"dash" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x10');
                    }
                    b"ndash" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x13');
                    }
                    b"mdash" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x14');
                    }
                    b"horbar" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x15');
                    }
                    b"Verbar" | b"Vert" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x16');
                    }
                    b"lsquo" | b"OpenCurlyQuote" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x18');
                    }
                    b"rsquo" | b"rsquor" | b"CloseCurlyQuote" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x19');
                    }
                    b"lsquor" | b"sbquo" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x1A');
                    }
                    b"ldquo" | b"OpenCurlyDoubleQuote" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x1C');
                    }
                    b"rdquo" | b"rdquor" | b"CloseCurlyDoubleQuote" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x1D');
                    }
                    b"ldquor" | b"bdquo" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x1E');
                    }
                    b"dagger" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x20');
                    }
                    b"Dagger" | b"ddagger" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x21');
                    }
                    b"bull" | b"bullet" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x22');
                    }
                    b"nldr" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x25');
                    }
                    b"hellip" | b"mldr" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x26');
                    }
                    b"permil" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x30');
                    }
                    b"pertenk" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x31');
                    }
                    b"prime" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x32');
                    }
                    b"Prime" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x33');
                    }
                    b"tprime" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x34');
                    }
                    b"bprime" | b"backprime" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x35');
                    }
                    b"lsaquo" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x39');
                    }
                    b"rsaquo" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x3A');
                    }
                    b"oline" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x3E');
                    }
                    b"caret" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x41');
                    }
                    b"hybull" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x43');
                    }
                    b"frasl" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x44');
                    }
                    b"bsemi" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x4F');
                    }
                    b"qprime" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x57');
                    }
                    b"MediumSpace" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x5F');
                    }
                    b"NoBreak" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x60');
                    }
                    b"ApplyFunction" | b"af" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x61');
                    }
                    b"InvisibleTimes" | b"it" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x62');
                    }
                    b"InvisibleComma" | b"ic" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\x63');
                    }
                    b"euro" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\xAC');
                    }
                    b"tdot" | b"TripleDot" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\xDB');
                    }
                    b"DotDot" => {
                        unescaped.push(b'\x20');
                        unescaped.push(b'\xDC');
                    }
                    b"Copf" | b"complexes" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x02');
                    }
                    b"incare" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x05');
                    }
                    b"gscr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x0A');
                    }
                    b"hamilt" | b"HilbertSpace" | b"Hscr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x0B');
                    }
                    b"Hfr" | b"Poincareplane" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x0C');
                    }
                    b"quaternions" | b"Hopf" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x0D');
                    }
                    b"planckh" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x0E');
                    }
                    b"planck" | b"hbar" | b"plankv" | b"hslash" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x0F');
                    }
                    b"Iscr" | b"imagline" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x10');
                    }
                    b"image" | b"Im" | b"imagpart" | b"Ifr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x11');
                    }
                    b"Lscr" | b"lagran" | b"Laplacetrf" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x12');
                    }
                    b"ell" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x13');
                    }
                    b"Nopf" | b"naturals" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x15');
                    }
                    b"numero" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x16');
                    }
                    b"copysr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x17');
                    }
                    b"weierp" | b"wp" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x18');
                    }
                    b"Popf" | b"primes" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x19');
                    }
                    b"rationals" | b"Qopf" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x1A');
                    }
                    b"Rscr" | b"realine" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x1B');
                    }
                    b"real" | b"Re" | b"realpart" | b"Rfr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x1C');
                    }
                    b"reals" | b"Ropf" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x1D');
                    }
                    b"rx" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x1E');
                    }
                    b"trade" | b"TRADE" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x22');
                    }
                    b"integers" | b"Zopf" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x24');
                    }
                    b"ohm" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x26');
                    }
                    b"mho" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x27');
                    }
                    b"Zfr" | b"zeetrf" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x28');
                    }
                    b"iiota" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x29');
                    }
                    b"angst" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x2B');
                    }
                    b"bernou" | b"Bernoullis" | b"Bscr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x2C');
                    }
                    b"Cfr" | b"Cayleys" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x2D');
                    }
                    b"escr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x2F');
                    }
                    b"Escr" | b"expectation" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x30');
                    }
                    b"Fscr" | b"Fouriertrf" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x31');
                    }
                    b"phmmat" | b"Mellintrf" | b"Mscr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x33');
                    }
                    b"order" | b"orderof" | b"oscr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x34');
                    }
                    b"alefsym" | b"aleph" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x35');
                    }
                    b"beth" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x36');
                    }
                    b"gimel" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x37');
                    }
                    b"daleth" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x38');
                    }
                    b"CapitalDifferentialD" | b"DD" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x45');
                    }
                    b"DifferentialD" | b"dd" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x46');
                    }
                    b"ExponentialE" | b"exponentiale" | b"ee" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x47');
                    }
                    b"ImaginaryI" | b"ii" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x48');
                    }
                    b"frac13" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x53');
                    }
                    b"frac23" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x54');
                    }
                    b"frac15" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x55');
                    }
                    b"frac25" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x56');
                    }
                    b"frac35" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x57');
                    }
                    b"frac45" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x58');
                    }
                    b"frac16" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x59');
                    }
                    b"frac56" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x5A');
                    }
                    b"frac18" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x5B');
                    }
                    b"frac38" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x5C');
                    }
                    b"frac58" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x5D');
                    }
                    b"frac78" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x5E');
                    }
                    b"larr" | b"leftarrow" | b"LeftArrow" | b"slarr" | b"ShortLeftArrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x90');
                    }
                    b"uarr" | b"uparrow" | b"UpArrow" | b"ShortUpArrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x91');
                    }
                    b"rarr" | b"rightarrow" | b"RightArrow" | b"srarr" | b"ShortRightArrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x92');
                    }
                    b"darr" | b"downarrow" | b"DownArrow" | b"ShortDownArrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x93');
                    }
                    b"harr" | b"leftrightarrow" | b"LeftRightArrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x94');
                    }
                    b"varr" | b"updownarrow" | b"UpDownArrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x95');
                    }
                    b"nwarr" | b"UpperLeftArrow" | b"nwarrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x96');
                    }
                    b"nearr" | b"UpperRightArrow" | b"nearrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x97');
                    }
                    b"searr" | b"searrow" | b"LowerRightArrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x98');
                    }
                    b"swarr" | b"swarrow" | b"LowerLeftArrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x99');
                    }
                    b"nlarr" | b"nleftarrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x9A');
                    }
                    b"nrarr" | b"nrightarrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x9B');
                    }
                    b"rarrw" | b"rightsquigarrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x9D');
                    }
                    b"Larr" | b"twoheadleftarrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x9E');
                    }
                    b"Uarr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\x9F');
                    }
                    b"Rarr" | b"twoheadrightarrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xA0');
                    }
                    b"Darr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xA1');
                    }
                    b"larrtl" | b"leftarrowtail" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xA2');
                    }
                    b"rarrtl" | b"rightarrowtail" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xA3');
                    }
                    b"LeftTeeArrow" | b"mapstoleft" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xA4');
                    }
                    b"UpTeeArrow" | b"mapstoup" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xA5');
                    }
                    b"map" | b"RightTeeArrow" | b"mapsto" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xA6');
                    }
                    b"DownTeeArrow" | b"mapstodown" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xA7');
                    }
                    b"larrhk" | b"hookleftarrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xA9');
                    }
                    b"rarrhk" | b"hookrightarrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xAA');
                    }
                    b"larrlp" | b"looparrowleft" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xAB');
                    }
                    b"rarrlp" | b"looparrowright" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xAC');
                    }
                    b"harrw" | b"leftrightsquigarrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xAD');
                    }
                    b"nharr" | b"nleftrightarrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xAE');
                    }
                    b"lsh" | b"Lsh" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xB0');
                    }
                    b"rsh" | b"Rsh" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xB1');
                    }
                    b"ldsh" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xB2');
                    }
                    b"rdsh" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xB3');
                    }
                    b"crarr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xB5');
                    }
                    b"cularr" | b"curvearrowleft" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xB6');
                    }
                    b"curarr" | b"curvearrowright" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xB7');
                    }
                    b"olarr" | b"circlearrowleft" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xBA');
                    }
                    b"orarr" | b"circlearrowright" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xBB');
                    }
                    b"lharu" | b"LeftVector" | b"leftharpoonup" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xBC');
                    }
                    b"lhard" | b"leftharpoondown" | b"DownLeftVector" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xBD');
                    }
                    b"uharr" | b"upharpoonright" | b"RightUpVector" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xBE');
                    }
                    b"uharl" | b"upharpoonleft" | b"LeftUpVector" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xBF');
                    }
                    b"rharu" | b"RightVector" | b"rightharpoonup" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xC0');
                    }
                    b"rhard" | b"rightharpoondown" | b"DownRightVector" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xC1');
                    }
                    b"dharr" | b"RightDownVector" | b"downharpoonright" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xC2');
                    }
                    b"dharl" | b"LeftDownVector" | b"downharpoonleft" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xC3');
                    }
                    b"rlarr" | b"rightleftarrows" | b"RightArrowLeftArrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xC4');
                    }
                    b"udarr" | b"UpArrowDownArrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xC5');
                    }
                    b"lrarr" | b"leftrightarrows" | b"LeftArrowRightArrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xC6');
                    }
                    b"llarr" | b"leftleftarrows" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xC7');
                    }
                    b"uuarr" | b"upuparrows" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xC8');
                    }
                    b"rrarr" | b"rightrightarrows" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xC9');
                    }
                    b"ddarr" | b"downdownarrows" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xCA');
                    }
                    b"lrhar" | b"ReverseEquilibrium" | b"leftrightharpoons" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xCB');
                    }
                    b"rlhar" | b"rightleftharpoons" | b"Equilibrium" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xCC');
                    }
                    b"nlArr" | b"nLeftarrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xCD');
                    }
                    b"nhArr" | b"nLeftrightarrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xCE');
                    }
                    b"nrArr" | b"nRightarrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xCF');
                    }
                    b"lArr" | b"Leftarrow" | b"DoubleLeftArrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xD0');
                    }
                    b"uArr" | b"Uparrow" | b"DoubleUpArrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xD1');
                    }
                    b"rArr" | b"Rightarrow" | b"Implies" | b"DoubleRightArrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xD2');
                    }
                    b"dArr" | b"Downarrow" | b"DoubleDownArrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xD3');
                    }
                    b"hArr" | b"Leftrightarrow" | b"DoubleLeftRightArrow" | b"iff" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xD4');
                    }
                    b"vArr" | b"Updownarrow" | b"DoubleUpDownArrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xD5');
                    }
                    b"nwArr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xD6');
                    }
                    b"neArr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xD7');
                    }
                    b"seArr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xD8');
                    }
                    b"swArr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xD9');
                    }
                    b"lAarr" | b"Lleftarrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xDA');
                    }
                    b"rAarr" | b"Rrightarrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xDB');
                    }
                    b"zigrarr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xDD');
                    }
                    b"larrb" | b"LeftArrowBar" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xE4');
                    }
                    b"rarrb" | b"RightArrowBar" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xE5');
                    }
                    b"duarr" | b"DownArrowUpArrow" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xF5');
                    }
                    b"loarr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xFD');
                    }
                    b"roarr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xFE');
                    }
                    b"hoarr" => {
                        unescaped.push(b'\x21');
                        unescaped.push(b'\xFF');
                    }
                    b"forall" | b"ForAll" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x00');
                    }
                    b"comp" | b"complement" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x01');
                    }
                    b"part" | b"PartialD" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x02');
                    }
                    b"exist" | b"Exists" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x03');
                    }
                    b"nexist" | b"NotExists" | b"nexists" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x04');
                    }
                    b"empty" | b"emptyset" | b"emptyv" | b"varnothing" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x05');
                    }
                    b"nabla" | b"Del" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x07');
                    }
                    b"isin" | b"isinv" | b"Element" | b"in" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x08');
                    }
                    b"notin" | b"NotElement" | b"notinva" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x09');
                    }
                    b"niv" | b"ReverseElement" | b"ni" | b"SuchThat" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x0B');
                    }
                    b"notni" | b"notniva" | b"NotReverseElement" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x0C');
                    }
                    b"prod" | b"Product" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x0F');
                    }
                    b"coprod" | b"Coproduct" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x10');
                    }
                    b"sum" | b"Sum" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x11');
                    }
                    b"minus" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x12');
                    }
                    b"mnplus" | b"mp" | b"MinusPlus" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x13');
                    }
                    b"plusdo" | b"dotplus" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x14');
                    }
                    b"setmn" | b"setminus" | b"Backslash" | b"ssetmn" | b"smallsetminus" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x16');
                    }
                    b"lowast" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x17');
                    }
                    b"compfn" | b"SmallCircle" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x18');
                    }
                    b"radic" | b"Sqrt" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x1A');
                    }
                    b"prop" | b"propto" | b"Proportional" | b"vprop" | b"varpropto" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x1D');
                    }
                    b"infin" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x1E');
                    }
                    b"angrt" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x1F');
                    }
                    b"ang" | b"angle" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x20');
                    }
                    b"angmsd" | b"measuredangle" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x21');
                    }
                    b"angsph" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x22');
                    }
                    b"mid" | b"VerticalBar" | b"smid" | b"shortmid" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x23');
                    }
                    b"nmid" | b"NotVerticalBar" | b"nsmid" | b"nshortmid" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x24');
                    }
                    b"par" | b"parallel" | b"DoubleVerticalBar" | b"spar" | b"shortparallel" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x25');
                    }
                    b"npar"
                    | b"nparallel"
                    | b"NotDoubleVerticalBar"
                    | b"nspar"
                    | b"nshortparallel" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x26');
                    }
                    b"and" | b"wedge" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x27');
                    }
                    b"or" | b"vee" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x28');
                    }
                    b"cap" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x29');
                    }
                    b"cup" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x2A');
                    }
                    b"int" | b"Integral" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x2B');
                    }
                    b"Int" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x2C');
                    }
                    b"tint" | b"iiint" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x2D');
                    }
                    b"conint" | b"oint" | b"ContourIntegral" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x2E');
                    }
                    b"Conint" | b"DoubleContourIntegral" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x2F');
                    }
                    b"Cconint" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x30');
                    }
                    b"cwint" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x31');
                    }
                    b"cwconint" | b"ClockwiseContourIntegral" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x32');
                    }
                    b"awconint" | b"CounterClockwiseContourIntegral" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x33');
                    }
                    b"there4" | b"therefore" | b"Therefore" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x34');
                    }
                    b"becaus" | b"because" | b"Because" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x35');
                    }
                    b"ratio" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x36');
                    }
                    b"Colon" | b"Proportion" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x37');
                    }
                    b"minusd" | b"dotminus" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x38');
                    }
                    b"mDDot" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x3A');
                    }
                    b"homtht" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x3B');
                    }
                    b"sim" | b"Tilde" | b"thksim" | b"thicksim" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x3C');
                    }
                    b"bsim" | b"backsim" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x3D');
                    }
                    b"ac" | b"mstpos" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x3E');
                    }
                    b"acd" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x3F');
                    }
                    b"wreath" | b"VerticalTilde" | b"wr" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x40');
                    }
                    b"nsim" | b"NotTilde" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x41');
                    }
                    b"esim" | b"EqualTilde" | b"eqsim" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x42');
                    }
                    b"sime" | b"TildeEqual" | b"simeq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x43');
                    }
                    b"nsime" | b"nsimeq" | b"NotTildeEqual" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x44');
                    }
                    b"cong" | b"TildeFullEqual" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x45');
                    }
                    b"simne" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x46');
                    }
                    b"ncong" | b"NotTildeFullEqual" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x47');
                    }
                    b"asymp" | b"ap" | b"TildeTilde" | b"approx" | b"thkap" | b"thickapprox" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x48');
                    }
                    b"nap" | b"NotTildeTilde" | b"napprox" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x49');
                    }
                    b"ape" | b"approxeq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x4A');
                    }
                    b"apid" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x4B');
                    }
                    b"bcong" | b"backcong" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x4C');
                    }
                    b"asympeq" | b"CupCap" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x4D');
                    }
                    b"bump" | b"HumpDownHump" | b"Bumpeq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x4E');
                    }
                    b"bumpe" | b"HumpEqual" | b"bumpeq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x4F');
                    }
                    b"esdot" | b"DotEqual" | b"doteq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x50');
                    }
                    b"eDot" | b"doteqdot" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x51');
                    }
                    b"efDot" | b"fallingdotseq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x52');
                    }
                    b"erDot" | b"risingdotseq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x53');
                    }
                    b"colone" | b"coloneq" | b"Assign" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x54');
                    }
                    b"ecolon" | b"eqcolon" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x55');
                    }
                    b"ecir" | b"eqcirc" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x56');
                    }
                    b"cire" | b"circeq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x57');
                    }
                    b"wedgeq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x59');
                    }
                    b"veeeq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x5A');
                    }
                    b"trie" | b"triangleq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x5C');
                    }
                    b"equest" | b"questeq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x5F');
                    }
                    b"ne" | b"NotEqual" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x60');
                    }
                    b"equiv" | b"Congruent" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x61');
                    }
                    b"nequiv" | b"NotCongruent" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x62');
                    }
                    b"le" | b"leq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x64');
                    }
                    b"ge" | b"GreaterEqual" | b"geq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x65');
                    }
                    b"lE" | b"LessFullEqual" | b"leqq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x66');
                    }
                    b"gE" | b"GreaterFullEqual" | b"geqq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x67');
                    }
                    b"lnE" | b"lneqq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x68');
                    }
                    b"gnE" | b"gneqq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x69');
                    }
                    b"Lt" | b"NestedLessLess" | b"ll" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x6A');
                    }
                    b"Gt" | b"NestedGreaterGreater" | b"gg" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x6B');
                    }
                    b"twixt" | b"between" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x6C');
                    }
                    b"NotCupCap" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x6D');
                    }
                    b"nlt" | b"NotLess" | b"nless" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x6E');
                    }
                    b"ngt" | b"NotGreater" | b"ngtr" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x6F');
                    }
                    b"nle" | b"NotLessEqual" | b"nleq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x70');
                    }
                    b"nge" | b"NotGreaterEqual" | b"ngeq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x71');
                    }
                    b"lsim" | b"LessTilde" | b"lesssim" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x72');
                    }
                    b"gsim" | b"gtrsim" | b"GreaterTilde" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x73');
                    }
                    b"nlsim" | b"NotLessTilde" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x74');
                    }
                    b"ngsim" | b"NotGreaterTilde" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x75');
                    }
                    b"lg" | b"lessgtr" | b"LessGreater" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x76');
                    }
                    b"gl" | b"gtrless" | b"GreaterLess" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x77');
                    }
                    b"ntlg" | b"NotLessGreater" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x78');
                    }
                    b"ntgl" | b"NotGreaterLess" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x79');
                    }
                    b"pr" | b"Precedes" | b"prec" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x7A');
                    }
                    b"sc" | b"Succeeds" | b"succ" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x7B');
                    }
                    b"prcue" | b"PrecedesSlantEqual" | b"preccurlyeq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x7C');
                    }
                    b"sccue" | b"SucceedsSlantEqual" | b"succcurlyeq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x7D');
                    }
                    b"prsim" | b"precsim" | b"PrecedesTilde" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x7E');
                    }
                    b"scsim" | b"succsim" | b"SucceedsTilde" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x7F');
                    }
                    b"npr" | b"nprec" | b"NotPrecedes" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x80');
                    }
                    b"nsc" | b"nsucc" | b"NotSucceeds" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x81');
                    }
                    b"sub" | b"subset" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x82');
                    }
                    b"sup" | b"supset" | b"Superset" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x83');
                    }
                    b"nsub" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x84');
                    }
                    b"nsup" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x85');
                    }
                    b"sube" | b"SubsetEqual" | b"subseteq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x86');
                    }
                    b"supe" | b"supseteq" | b"SupersetEqual" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x87');
                    }
                    b"nsube" | b"nsubseteq" | b"NotSubsetEqual" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x88');
                    }
                    b"nsupe" | b"nsupseteq" | b"NotSupersetEqual" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x89');
                    }
                    b"subne" | b"subsetneq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x8A');
                    }
                    b"supne" | b"supsetneq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x8B');
                    }
                    b"cupdot" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x8D');
                    }
                    b"uplus" | b"UnionPlus" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x8E');
                    }
                    b"sqsub" | b"SquareSubset" | b"sqsubset" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x8F');
                    }
                    b"sqsup" | b"SquareSuperset" | b"sqsupset" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x90');
                    }
                    b"sqsube" | b"SquareSubsetEqual" | b"sqsubseteq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x91');
                    }
                    b"sqsupe" | b"SquareSupersetEqual" | b"sqsupseteq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x92');
                    }
                    b"sqcap" | b"SquareIntersection" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x93');
                    }
                    b"sqcup" | b"SquareUnion" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x94');
                    }
                    b"oplus" | b"CirclePlus" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x95');
                    }
                    b"ominus" | b"CircleMinus" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x96');
                    }
                    b"otimes" | b"CircleTimes" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x97');
                    }
                    b"osol" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x98');
                    }
                    b"odot" | b"CircleDot" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x99');
                    }
                    b"ocir" | b"circledcirc" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x9A');
                    }
                    b"oast" | b"circledast" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x9B');
                    }
                    b"odash" | b"circleddash" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x9D');
                    }
                    b"plusb" | b"boxplus" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x9E');
                    }
                    b"minusb" | b"boxminus" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\x9F');
                    }
                    b"timesb" | b"boxtimes" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xA0');
                    }
                    b"sdotb" | b"dotsquare" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xA1');
                    }
                    b"vdash" | b"RightTee" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xA2');
                    }
                    b"dashv" | b"LeftTee" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xA3');
                    }
                    b"top" | b"DownTee" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xA4');
                    }
                    b"bottom" | b"bot" | b"perp" | b"UpTee" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xA5');
                    }
                    b"models" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xA7');
                    }
                    b"vDash" | b"DoubleRightTee" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xA8');
                    }
                    b"Vdash" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xA9');
                    }
                    b"Vvdash" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xAA');
                    }
                    b"VDash" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xAB');
                    }
                    b"nvdash" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xAC');
                    }
                    b"nvDash" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xAD');
                    }
                    b"nVdash" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xAE');
                    }
                    b"nVDash" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xAF');
                    }
                    b"prurel" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xB0');
                    }
                    b"vltri" | b"vartriangleleft" | b"LeftTriangle" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xB2');
                    }
                    b"vrtri" | b"vartriangleright" | b"RightTriangle" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xB3');
                    }
                    b"ltrie" | b"trianglelefteq" | b"LeftTriangleEqual" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xB4');
                    }
                    b"rtrie" | b"trianglerighteq" | b"RightTriangleEqual" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xB5');
                    }
                    b"origof" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xB6');
                    }
                    b"imof" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xB7');
                    }
                    b"mumap" | b"multimap" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xB8');
                    }
                    b"hercon" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xB9');
                    }
                    b"intcal" | b"intercal" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xBA');
                    }
                    b"veebar" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xBB');
                    }
                    b"barvee" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xBD');
                    }
                    b"angrtvb" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xBE');
                    }
                    b"lrtri" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xBF');
                    }
                    b"xwedge" | b"Wedge" | b"bigwedge" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xC0');
                    }
                    b"xvee" | b"Vee" | b"bigvee" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xC1');
                    }
                    b"xcap" | b"Intersection" | b"bigcap" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xC2');
                    }
                    b"xcup" | b"Union" | b"bigcup" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xC3');
                    }
                    b"diam" | b"diamond" | b"Diamond" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xC4');
                    }
                    b"sdot" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xC5');
                    }
                    b"sstarf" | b"Star" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xC6');
                    }
                    b"divonx" | b"divideontimes" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xC7');
                    }
                    b"bowtie" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xC8');
                    }
                    b"ltimes" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xC9');
                    }
                    b"rtimes" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xCA');
                    }
                    b"lthree" | b"leftthreetimes" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xCB');
                    }
                    b"rthree" | b"rightthreetimes" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xCC');
                    }
                    b"bsime" | b"backsimeq" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xCD');
                    }
                    b"cuvee" | b"curlyvee" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xCE');
                    }
                    b"cuwed" | b"curlywedge" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xCF');
                    }
                    b"Sub" | b"Subset" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xD0');
                    }
                    b"Sup" | b"Supset" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xD1');
                    }
                    b"Cap" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xD2');
                    }
                    b"Cup" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xD3');
                    }
                    b"fork" | b"pitchfork" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xD4');
                    }
                    b"epar" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xD5');
                    }
                    b"ltdot" | b"lessdot" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xD6');
                    }
                    b"gtdot" | b"gtrdot" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xD7');
                    }
                    b"Ll" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xD8');
                    }
                    b"Gg" | b"ggg" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xD9');
                    }
                    b"leg" | b"LessEqualGreater" | b"lesseqgtr" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xDA');
                    }
                    b"gel" | b"gtreqless" | b"GreaterEqualLess" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xDB');
                    }
                    b"cuepr" | b"curlyeqprec" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xDE');
                    }
                    b"cuesc" | b"curlyeqsucc" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xDF');
                    }
                    b"nprcue" | b"NotPrecedesSlantEqual" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xE0');
                    }
                    b"nsccue" | b"NotSucceedsSlantEqual" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xE1');
                    }
                    b"nsqsube" | b"NotSquareSubsetEqual" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xE2');
                    }
                    b"nsqsupe" | b"NotSquareSupersetEqual" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xE3');
                    }
                    b"lnsim" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xE6');
                    }
                    b"gnsim" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xE7');
                    }
                    b"prnsim" | b"precnsim" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xE8');
                    }
                    b"scnsim" | b"succnsim" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xE9');
                    }
                    b"nltri" | b"ntriangleleft" | b"NotLeftTriangle" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xEA');
                    }
                    b"nrtri" | b"ntriangleright" | b"NotRightTriangle" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xEB');
                    }
                    b"nltrie" | b"ntrianglelefteq" | b"NotLeftTriangleEqual" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xEC');
                    }
                    b"nrtrie" | b"ntrianglerighteq" | b"NotRightTriangleEqual" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xED');
                    }
                    b"vellip" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xEE');
                    }
                    b"ctdot" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xEF');
                    }
                    b"utdot" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xF0');
                    }
                    b"dtdot" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xF1');
                    }
                    b"disin" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xF2');
                    }
                    b"isinsv" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xF3');
                    }
                    b"isins" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xF4');
                    }
                    b"isindot" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xF5');
                    }
                    b"notinvc" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xF6');
                    }
                    b"notinvb" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xF7');
                    }
                    b"isinE" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xF9');
                    }
                    b"nisd" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xFA');
                    }
                    b"xnis" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xFB');
                    }
                    b"nis" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xFC');
                    }
                    b"notnivc" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xFD');
                    }
                    b"notnivb" => {
                        unescaped.push(b'\x22');
                        unescaped.push(b'\xFE');
                    }
                    b"barwed" | b"barwedge" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x05');
                    }
                    b"Barwed" | b"doublebarwedge" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x06');
                    }
                    b"lceil" | b"LeftCeiling" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x08');
                    }
                    b"rceil" | b"RightCeiling" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x09');
                    }
                    b"lfloor" | b"LeftFloor" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x0A');
                    }
                    b"rfloor" | b"RightFloor" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x0B');
                    }
                    b"drcrop" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x0C');
                    }
                    b"dlcrop" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x0D');
                    }
                    b"urcrop" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x0E');
                    }
                    b"ulcrop" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x0F');
                    }
                    b"bnot" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x10');
                    }
                    b"profline" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x12');
                    }
                    b"profsurf" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x13');
                    }
                    b"telrec" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x15');
                    }
                    b"target" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x16');
                    }
                    b"ulcorn" | b"ulcorner" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x1C');
                    }
                    b"urcorn" | b"urcorner" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x1D');
                    }
                    b"dlcorn" | b"llcorner" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x1E');
                    }
                    b"drcorn" | b"lrcorner" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x1F');
                    }
                    b"frown" | b"sfrown" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x22');
                    }
                    b"smile" | b"ssmile" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x23');
                    }
                    b"cylcty" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x2D');
                    }
                    b"profalar" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x2E');
                    }
                    b"topbot" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x36');
                    }
                    b"ovbar" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x3D');
                    }
                    b"solbar" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x3F');
                    }
                    b"angzarr" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\x7C');
                    }
                    b"lmoust" | b"lmoustache" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\xB0');
                    }
                    b"rmoust" | b"rmoustache" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\xB1');
                    }
                    b"tbrk" | b"OverBracket" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\xB4');
                    }
                    b"bbrk" | b"UnderBracket" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\xB5');
                    }
                    b"bbrktbrk" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\xB6');
                    }
                    b"OverParenthesis" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\xDC');
                    }
                    b"UnderParenthesis" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\xDD');
                    }
                    b"OverBrace" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\xDE');
                    }
                    b"UnderBrace" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\xDF');
                    }
                    b"trpezium" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\xE2');
                    }
                    b"elinters" => {
                        unescaped.push(b'\x23');
                        unescaped.push(b'\xE7');
                    }
                    b"blank" => {
                        unescaped.push(b'\x24');
                        unescaped.push(b'\x23');
                    }
                    b"oS" | b"circledS" => {
                        unescaped.push(b'\x24');
                        unescaped.push(b'\xC8');
                    }
                    b"boxh" | b"HorizontalLine" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x00');
                    }
                    b"boxv" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x02');
                    }
                    b"boxdr" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x0C');
                    }
                    b"boxdl" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x10');
                    }
                    b"boxur" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x14');
                    }
                    b"boxul" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x18');
                    }
                    b"boxvr" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x1C');
                    }
                    b"boxvl" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x24');
                    }
                    b"boxhd" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x2C');
                    }
                    b"boxhu" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x34');
                    }
                    b"boxvh" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x3C');
                    }
                    b"boxH" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x50');
                    }
                    b"boxV" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x51');
                    }
                    b"boxdR" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x52');
                    }
                    b"boxDr" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x53');
                    }
                    b"boxDR" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x54');
                    }
                    b"boxdL" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x55');
                    }
                    b"boxDl" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x56');
                    }
                    b"boxDL" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x57');
                    }
                    b"boxuR" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x58');
                    }
                    b"boxUr" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x59');
                    }
                    b"boxUR" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x5A');
                    }
                    b"boxuL" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x5B');
                    }
                    b"boxUl" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x5C');
                    }
                    b"boxUL" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x5D');
                    }
                    b"boxvR" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x5E');
                    }
                    b"boxVr" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x5F');
                    }
                    b"boxVR" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x60');
                    }
                    b"boxvL" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x61');
                    }
                    b"boxVl" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x62');
                    }
                    b"boxVL" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x63');
                    }
                    b"boxHd" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x64');
                    }
                    b"boxhD" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x65');
                    }
                    b"boxHD" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x66');
                    }
                    b"boxHu" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x67');
                    }
                    b"boxhU" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x68');
                    }
                    b"boxHU" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x69');
                    }
                    b"boxvH" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x6A');
                    }
                    b"boxVh" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x6B');
                    }
                    b"boxVH" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x6C');
                    }
                    b"uhblk" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x80');
                    }
                    b"lhblk" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x84');
                    }
                    b"block" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x88');
                    }
                    b"blk14" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x91');
                    }
                    b"blk12" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x92');
                    }
                    b"blk34" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\x93');
                    }
                    b"squ" | b"square" | b"Square" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xA1');
                    }
                    b"squf" | b"squarf" | b"blacksquare" | b"FilledVerySmallSquare" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xAA');
                    }
                    b"EmptyVerySmallSquare" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xAB');
                    }
                    b"rect" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xAD');
                    }
                    b"marker" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xAE');
                    }
                    b"fltns" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xB1');
                    }
                    b"xutri" | b"bigtriangleup" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xB3');
                    }
                    b"utrif" | b"blacktriangle" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xB4');
                    }
                    b"utri" | b"triangle" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xB5');
                    }
                    b"rtrif" | b"blacktriangleright" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xB8');
                    }
                    b"rtri" | b"triangleright" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xB9');
                    }
                    b"xdtri" | b"bigtriangledown" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xBD');
                    }
                    b"dtrif" | b"blacktriangledown" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xBE');
                    }
                    b"dtri" | b"triangledown" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xBF');
                    }
                    b"ltrif" | b"blacktriangleleft" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xC2');
                    }
                    b"ltri" | b"triangleleft" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xC3');
                    }
                    b"loz" | b"lozenge" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xCA');
                    }
                    b"cir" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xCB');
                    }
                    b"tridot" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xEC');
                    }
                    b"xcirc" | b"bigcirc" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xEF');
                    }
                    b"ultri" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xF8');
                    }
                    b"urtri" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xF9');
                    }
                    b"lltri" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xFA');
                    }
                    b"EmptySmallSquare" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xFB');
                    }
                    b"FilledSmallSquare" => {
                        unescaped.push(b'\x25');
                        unescaped.push(b'\xFC');
                    }
                    b"starf" | b"bigstar" => {
                        unescaped.push(b'\x26');
                        unescaped.push(b'\x05');
                    }
                    b"star" => {
                        unescaped.push(b'\x26');
                        unescaped.push(b'\x06');
                    }
                    b"phone" => {
                        unescaped.push(b'\x26');
                        unescaped.push(b'\x0E');
                    }
                    b"female" => {
                        unescaped.push(b'\x26');
                        unescaped.push(b'\x40');
                    }
                    b"male" => {
                        unescaped.push(b'\x26');
                        unescaped.push(b'\x42');
                    }
                    b"spades" | b"spadesuit" => {
                        unescaped.push(b'\x26');
                        unescaped.push(b'\x60');
                    }
                    b"clubs" | b"clubsuit" => {
                        unescaped.push(b'\x26');
                        unescaped.push(b'\x63');
                    }
                    b"hearts" | b"heartsuit" => {
                        unescaped.push(b'\x26');
                        unescaped.push(b'\x65');
                    }
                    b"diams" | b"diamondsuit" => {
                        unescaped.push(b'\x26');
                        unescaped.push(b'\x66');
                    }
                    b"sung" => {
                        unescaped.push(b'\x26');
                        unescaped.push(b'\x6A');
                    }
                    b"flat" => {
                        unescaped.push(b'\x26');
                        unescaped.push(b'\x6D');
                    }
                    b"natur" | b"natural" => {
                        unescaped.push(b'\x26');
                        unescaped.push(b'\x6E');
                    }
                    b"sharp" => {
                        unescaped.push(b'\x26');
                        unescaped.push(b'\x6F');
                    }
                    b"check" | b"checkmark" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\x13');
                    }
                    b"cross" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\x17');
                    }
                    b"malt" | b"maltese" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\x20');
                    }
                    b"sext" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\x36');
                    }
                    b"VerticalSeparator" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\x58');
                    }
                    b"lbbrk" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\x72');
                    }
                    b"rbbrk" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\x73');
                    }
                    b"lobrk" | b"LeftDoubleBracket" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\xE6');
                    }
                    b"robrk" | b"RightDoubleBracket" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\xE7');
                    }
                    b"lang" | b"LeftAngleBracket" | b"langle" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\xE8');
                    }
                    b"rang" | b"RightAngleBracket" | b"rangle" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\xE9');
                    }
                    b"Lang" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\xEA');
                    }
                    b"Rang" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\xEB');
                    }
                    b"loang" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\xEC');
                    }
                    b"roang" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\xED');
                    }
                    b"xlarr" | b"longleftarrow" | b"LongLeftArrow" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\xF5');
                    }
                    b"xrarr" | b"longrightarrow" | b"LongRightArrow" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\xF6');
                    }
                    b"xharr" | b"longleftrightarrow" | b"LongLeftRightArrow" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\xF7');
                    }
                    b"xlArr" | b"Longleftarrow" | b"DoubleLongLeftArrow" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\xF8');
                    }
                    b"xrArr" | b"Longrightarrow" | b"DoubleLongRightArrow" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\xF9');
                    }
                    b"xhArr" | b"Longleftrightarrow" | b"DoubleLongLeftRightArrow" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\xFA');
                    }
                    b"xmap" | b"longmapsto" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\xFC');
                    }
                    b"dzigrarr" => {
                        unescaped.push(b'\x27');
                        unescaped.push(b'\xFF');
                    }
                    b"nvlArr" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x02');
                    }
                    b"nvrArr" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x03');
                    }
                    b"nvHarr" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x04');
                    }
                    b"Map" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x05');
                    }
                    b"lbarr" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x0C');
                    }
                    b"rbarr" | b"bkarow" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x0D');
                    }
                    b"lBarr" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x0E');
                    }
                    b"rBarr" | b"dbkarow" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x0F');
                    }
                    b"RBarr" | b"drbkarow" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x10');
                    }
                    b"DDotrahd" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x11');
                    }
                    b"UpArrowBar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x12');
                    }
                    b"DownArrowBar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x13');
                    }
                    b"Rarrtl" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x16');
                    }
                    b"latail" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x19');
                    }
                    b"ratail" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x1A');
                    }
                    b"lAtail" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x1B');
                    }
                    b"rAtail" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x1C');
                    }
                    b"larrfs" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x1D');
                    }
                    b"rarrfs" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x1E');
                    }
                    b"larrbfs" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x1F');
                    }
                    b"rarrbfs" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x20');
                    }
                    b"nwarhk" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x23');
                    }
                    b"nearhk" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x24');
                    }
                    b"searhk" | b"hksearow" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x25');
                    }
                    b"swarhk" | b"hkswarow" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x26');
                    }
                    b"nwnear" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x27');
                    }
                    b"nesear" | b"toea" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x28');
                    }
                    b"seswar" | b"tosa" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x29');
                    }
                    b"swnwar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x2A');
                    }
                    b"rarrc" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x33');
                    }
                    b"cudarrr" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x35');
                    }
                    b"ldca" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x36');
                    }
                    b"rdca" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x37');
                    }
                    b"cudarrl" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x38');
                    }
                    b"larrpl" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x39');
                    }
                    b"curarrm" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x3C');
                    }
                    b"cularrp" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x3D');
                    }
                    b"rarrpl" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x45');
                    }
                    b"harrcir" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x48');
                    }
                    b"Uarrocir" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x49');
                    }
                    b"lurdshar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x4A');
                    }
                    b"ldrushar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x4B');
                    }
                    b"LeftRightVector" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x4E');
                    }
                    b"RightUpDownVector" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x4F');
                    }
                    b"DownLeftRightVector" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x50');
                    }
                    b"LeftUpDownVector" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x51');
                    }
                    b"LeftVectorBar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x52');
                    }
                    b"RightVectorBar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x53');
                    }
                    b"RightUpVectorBar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x54');
                    }
                    b"RightDownVectorBar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x55');
                    }
                    b"DownLeftVectorBar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x56');
                    }
                    b"DownRightVectorBar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x57');
                    }
                    b"LeftUpVectorBar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x58');
                    }
                    b"LeftDownVectorBar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x59');
                    }
                    b"LeftTeeVector" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x5A');
                    }
                    b"RightTeeVector" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x5B');
                    }
                    b"RightUpTeeVector" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x5C');
                    }
                    b"RightDownTeeVector" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x5D');
                    }
                    b"DownLeftTeeVector" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x5E');
                    }
                    b"DownRightTeeVector" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x5F');
                    }
                    b"LeftUpTeeVector" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x60');
                    }
                    b"LeftDownTeeVector" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x61');
                    }
                    b"lHar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x62');
                    }
                    b"uHar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x63');
                    }
                    b"rHar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x64');
                    }
                    b"dHar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x65');
                    }
                    b"luruhar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x66');
                    }
                    b"ldrdhar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x67');
                    }
                    b"ruluhar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x68');
                    }
                    b"rdldhar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x69');
                    }
                    b"lharul" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x6A');
                    }
                    b"llhard" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x6B');
                    }
                    b"rharul" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x6C');
                    }
                    b"lrhard" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x6D');
                    }
                    b"udhar" | b"UpEquilibrium" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x6E');
                    }
                    b"duhar" | b"ReverseUpEquilibrium" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x6F');
                    }
                    b"RoundImplies" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x70');
                    }
                    b"erarr" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x71');
                    }
                    b"simrarr" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x72');
                    }
                    b"larrsim" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x73');
                    }
                    b"rarrsim" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x74');
                    }
                    b"rarrap" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x75');
                    }
                    b"ltlarr" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x76');
                    }
                    b"gtrarr" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x78');
                    }
                    b"subrarr" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x79');
                    }
                    b"suplarr" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x7B');
                    }
                    b"lfisht" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x7C');
                    }
                    b"rfisht" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x7D');
                    }
                    b"ufisht" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x7E');
                    }
                    b"dfisht" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x7F');
                    }
                    b"lopar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x85');
                    }
                    b"ropar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x86');
                    }
                    b"lbrke" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x8B');
                    }
                    b"rbrke" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x8C');
                    }
                    b"lbrkslu" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x8D');
                    }
                    b"rbrksld" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x8E');
                    }
                    b"lbrksld" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x8F');
                    }
                    b"rbrkslu" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x90');
                    }
                    b"langd" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x91');
                    }
                    b"rangd" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x92');
                    }
                    b"lparlt" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x93');
                    }
                    b"rpargt" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x94');
                    }
                    b"gtlPar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x95');
                    }
                    b"ltrPar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x96');
                    }
                    b"vzigzag" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x9A');
                    }
                    b"vangrt" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x9C');
                    }
                    b"angrtvbd" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\x9D');
                    }
                    b"ange" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xA4');
                    }
                    b"range" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xA5');
                    }
                    b"dwangle" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xA6');
                    }
                    b"uwangle" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xA7');
                    }
                    b"angmsdaa" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xA8');
                    }
                    b"angmsdab" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xA9');
                    }
                    b"angmsdac" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xAA');
                    }
                    b"angmsdad" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xAB');
                    }
                    b"angmsdae" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xAC');
                    }
                    b"angmsdaf" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xAD');
                    }
                    b"angmsdag" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xAE');
                    }
                    b"angmsdah" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xAF');
                    }
                    b"bemptyv" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xB0');
                    }
                    b"demptyv" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xB1');
                    }
                    b"cemptyv" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xB2');
                    }
                    b"raemptyv" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xB3');
                    }
                    b"laemptyv" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xB4');
                    }
                    b"ohbar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xB5');
                    }
                    b"omid" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xB6');
                    }
                    b"opar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xB7');
                    }
                    b"operp" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xB9');
                    }
                    b"olcross" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xBB');
                    }
                    b"odsold" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xBC');
                    }
                    b"olcir" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xBE');
                    }
                    b"ofcir" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xBF');
                    }
                    b"olt" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xC0');
                    }
                    b"ogt" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xC1');
                    }
                    b"cirscir" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xC2');
                    }
                    b"cirE" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xC3');
                    }
                    b"solb" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xC4');
                    }
                    b"bsolb" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xC5');
                    }
                    b"boxbox" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xC9');
                    }
                    b"trisb" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xCD');
                    }
                    b"rtriltri" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xCE');
                    }
                    b"LeftTriangleBar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xCF');
                    }
                    b"RightTriangleBar" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xD0');
                    }
                    b"race" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xDA');
                    }
                    b"iinfin" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xDC');
                    }
                    b"infintie" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xDD');
                    }
                    b"nvinfin" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xDE');
                    }
                    b"eparsl" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xE3');
                    }
                    b"smeparsl" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xE4');
                    }
                    b"eqvparsl" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xE5');
                    }
                    b"lozf" | b"blacklozenge" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xEB');
                    }
                    b"RuleDelayed" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xF4');
                    }
                    b"dsol" => {
                        unescaped.push(b'\x29');
                        unescaped.push(b'\xF6');
                    }
                    b"xodot" | b"bigodot" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x00');
                    }
                    b"xoplus" | b"bigoplus" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x01');
                    }
                    b"xotime" | b"bigotimes" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x02');
                    }
                    b"xuplus" | b"biguplus" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x04');
                    }
                    b"xsqcup" | b"bigsqcup" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x06');
                    }
                    b"qint" | b"iiiint" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x0C');
                    }
                    b"fpartint" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x0D');
                    }
                    b"cirfnint" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x10');
                    }
                    b"awint" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x11');
                    }
                    b"rppolint" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x12');
                    }
                    b"scpolint" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x13');
                    }
                    b"npolint" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x14');
                    }
                    b"pointint" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x15');
                    }
                    b"quatint" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x16');
                    }
                    b"intlarhk" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x17');
                    }
                    b"pluscir" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x22');
                    }
                    b"plusacir" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x23');
                    }
                    b"simplus" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x24');
                    }
                    b"plusdu" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x25');
                    }
                    b"plussim" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x26');
                    }
                    b"plustwo" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x27');
                    }
                    b"mcomma" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x29');
                    }
                    b"minusdu" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x2A');
                    }
                    b"loplus" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x2D');
                    }
                    b"roplus" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x2E');
                    }
                    b"Cross" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x2F');
                    }
                    b"timesd" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x30');
                    }
                    b"timesbar" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x31');
                    }
                    b"smashp" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x33');
                    }
                    b"lotimes" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x34');
                    }
                    b"rotimes" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x35');
                    }
                    b"otimesas" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x36');
                    }
                    b"Otimes" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x37');
                    }
                    b"odiv" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x38');
                    }
                    b"triplus" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x39');
                    }
                    b"triminus" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x3A');
                    }
                    b"tritime" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x3B');
                    }
                    b"iprod" | b"intprod" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x3C');
                    }
                    b"amalg" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x3F');
                    }
                    b"capdot" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x40');
                    }
                    b"ncup" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x42');
                    }
                    b"ncap" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x43');
                    }
                    b"capand" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x44');
                    }
                    b"cupor" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x45');
                    }
                    b"cupcap" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x46');
                    }
                    b"capcup" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x47');
                    }
                    b"cupbrcap" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x48');
                    }
                    b"capbrcup" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x49');
                    }
                    b"cupcup" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x4A');
                    }
                    b"capcap" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x4B');
                    }
                    b"ccups" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x4C');
                    }
                    b"ccaps" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x4D');
                    }
                    b"ccupssm" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x50');
                    }
                    b"And" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x53');
                    }
                    b"Or" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x54');
                    }
                    b"andand" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x55');
                    }
                    b"oror" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x56');
                    }
                    b"orslope" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x57');
                    }
                    b"andslope" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x58');
                    }
                    b"andv" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x5A');
                    }
                    b"orv" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x5B');
                    }
                    b"andd" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x5C');
                    }
                    b"ord" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x5D');
                    }
                    b"wedbar" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x5F');
                    }
                    b"sdote" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x66');
                    }
                    b"simdot" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x6A');
                    }
                    b"congdot" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x6D');
                    }
                    b"easter" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x6E');
                    }
                    b"apacir" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x6F');
                    }
                    b"apE" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x70');
                    }
                    b"eplus" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x71');
                    }
                    b"pluse" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x72');
                    }
                    b"Esim" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x73');
                    }
                    b"Colone" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x74');
                    }
                    b"Equal" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x75');
                    }
                    b"eDDot" | b"ddotseq" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x77');
                    }
                    b"equivDD" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x78');
                    }
                    b"ltcir" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x79');
                    }
                    b"gtcir" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x7A');
                    }
                    b"ltquest" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x7B');
                    }
                    b"gtquest" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x7C');
                    }
                    b"les" | b"LessSlantEqual" | b"leqslant" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x7D');
                    }
                    b"ges" | b"GreaterSlantEqual" | b"geqslant" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x7E');
                    }
                    b"lesdot" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x7F');
                    }
                    b"gesdot" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x80');
                    }
                    b"lesdoto" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x81');
                    }
                    b"gesdoto" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x82');
                    }
                    b"lesdotor" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x83');
                    }
                    b"gesdotol" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x84');
                    }
                    b"lap" | b"lessapprox" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x85');
                    }
                    b"gap" | b"gtrapprox" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x86');
                    }
                    b"lne" | b"lneq" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x87');
                    }
                    b"gne" | b"gneq" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x88');
                    }
                    b"lnap" | b"lnapprox" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x89');
                    }
                    b"gnap" | b"gnapprox" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x8A');
                    }
                    b"lEg" | b"lesseqqgtr" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x8B');
                    }
                    b"gEl" | b"gtreqqless" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x8C');
                    }
                    b"lsime" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x8D');
                    }
                    b"gsime" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x8E');
                    }
                    b"lsimg" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x8F');
                    }
                    b"gsiml" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x90');
                    }
                    b"lgE" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x91');
                    }
                    b"glE" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x92');
                    }
                    b"lesges" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x93');
                    }
                    b"gesles" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x94');
                    }
                    b"els" | b"eqslantless" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x95');
                    }
                    b"egs" | b"eqslantgtr" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x96');
                    }
                    b"elsdot" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x97');
                    }
                    b"egsdot" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x98');
                    }
                    b"el" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x99');
                    }
                    b"eg" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x9A');
                    }
                    b"siml" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x9D');
                    }
                    b"simg" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x9E');
                    }
                    b"simlE" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\x9F');
                    }
                    b"simgE" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xA0');
                    }
                    b"LessLess" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xA1');
                    }
                    b"GreaterGreater" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xA2');
                    }
                    b"glj" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xA4');
                    }
                    b"gla" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xA5');
                    }
                    b"ltcc" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xA6');
                    }
                    b"gtcc" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xA7');
                    }
                    b"lescc" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xA8');
                    }
                    b"gescc" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xA9');
                    }
                    b"smt" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xAA');
                    }
                    b"lat" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xAB');
                    }
                    b"smte" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xAC');
                    }
                    b"late" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xAD');
                    }
                    b"bumpE" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xAE');
                    }
                    b"pre" | b"preceq" | b"PrecedesEqual" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xAF');
                    }
                    b"sce" | b"succeq" | b"SucceedsEqual" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xB0');
                    }
                    b"prE" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xB3');
                    }
                    b"scE" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xB4');
                    }
                    b"prnE" | b"precneqq" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xB5');
                    }
                    b"scnE" | b"succneqq" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xB6');
                    }
                    b"prap" | b"precapprox" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xB7');
                    }
                    b"scap" | b"succapprox" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xB8');
                    }
                    b"prnap" | b"precnapprox" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xB9');
                    }
                    b"scnap" | b"succnapprox" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xBA');
                    }
                    b"Pr" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xBB');
                    }
                    b"Sc" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xBC');
                    }
                    b"subdot" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xBD');
                    }
                    b"supdot" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xBE');
                    }
                    b"subplus" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xBF');
                    }
                    b"supplus" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xC0');
                    }
                    b"submult" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xC1');
                    }
                    b"supmult" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xC2');
                    }
                    b"subedot" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xC3');
                    }
                    b"supedot" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xC4');
                    }
                    b"subE" | b"subseteqq" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xC5');
                    }
                    b"supE" | b"supseteqq" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xC6');
                    }
                    b"subsim" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xC7');
                    }
                    b"supsim" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xC8');
                    }
                    b"subnE" | b"subsetneqq" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xCB');
                    }
                    b"supnE" | b"supsetneqq" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xCC');
                    }
                    b"csub" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xCF');
                    }
                    b"csup" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xD0');
                    }
                    b"csube" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xD1');
                    }
                    b"csupe" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xD2');
                    }
                    b"subsup" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xD3');
                    }
                    b"supsub" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xD4');
                    }
                    b"subsub" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xD5');
                    }
                    b"supsup" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xD6');
                    }
                    b"suphsub" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xD7');
                    }
                    b"supdsub" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xD8');
                    }
                    b"forkv" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xD9');
                    }
                    b"topfork" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xDA');
                    }
                    b"mlcp" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xDB');
                    }
                    b"Dashv" | b"DoubleLeftTee" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xE4');
                    }
                    b"Vdashl" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xE6');
                    }
                    b"Barv" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xE7');
                    }
                    b"vBar" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xE8');
                    }
                    b"vBarv" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xE9');
                    }
                    b"Vbar" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xEB');
                    }
                    b"Not" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xEC');
                    }
                    b"bNot" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xED');
                    }
                    b"rnmid" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xEE');
                    }
                    b"cirmid" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xEF');
                    }
                    b"midcir" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xF0');
                    }
                    b"topcir" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xF1');
                    }
                    b"nhpar" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xF2');
                    }
                    b"parsim" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xF3');
                    }
                    b"parsl" => {
                        unescaped.push(b'\x2A');
                        unescaped.push(b'\xFD');
                    }
                    b"fflig" => {
                        unescaped.push(b'\xFB');
                        unescaped.push(b'\x00');
                    }
                    b"filig" => {
                        unescaped.push(b'\xFB');
                        unescaped.push(b'\x01');
                    }
                    b"fllig" => {
                        unescaped.push(b'\xFB');
                        unescaped.push(b'\x02');
                    }
                    b"ffilig" => {
                        unescaped.push(b'\xFB');
                        unescaped.push(b'\x03');
                    }
                    b"ffllig" => {
                        unescaped.push(b'\xFB');
                        unescaped.push(b'\x04');
                    }
                    b"Ascr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x49');
                    }
                    b"Cscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x49');
                    }
                    b"Dscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x49');
                    }
                    b"Gscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4A');
                    }
                    b"Jscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4A');
                    }
                    b"Kscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4A');
                    }
                    b"Nscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4A');
                    }
                    b"Oscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4A');
                    }
                    b"Pscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4A');
                    }
                    b"Qscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4A');
                    }
                    b"Sscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4A');
                    }
                    b"Tscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4A');
                    }
                    b"Uscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4B');
                    }
                    b"Vscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4B');
                    }
                    b"Wscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4B');
                    }
                    b"Xscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4B');
                    }
                    b"Yscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4B');
                    }
                    b"Zscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4B');
                    }
                    b"ascr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4B');
                    }
                    b"bscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4B');
                    }
                    b"cscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4B');
                    }
                    b"dscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4B');
                    }
                    b"fscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4B');
                    }
                    b"hscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4B');
                    }
                    b"iscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4B');
                    }
                    b"jscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4B');
                    }
                    b"kscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4C');
                    }
                    b"lscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4C');
                    }
                    b"mscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4C');
                    }
                    b"nscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4C');
                    }
                    b"pscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4C');
                    }
                    b"qscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4C');
                    }
                    b"rscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4C');
                    }
                    b"sscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4C');
                    }
                    b"tscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4C');
                    }
                    b"uscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4C');
                    }
                    b"vscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4C');
                    }
                    b"wscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4C');
                    }
                    b"xscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4C');
                    }
                    b"yscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4C');
                    }
                    b"zscr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x4C');
                    }
                    b"Afr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x50');
                    }
                    b"Bfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x50');
                    }
                    b"Dfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x50');
                    }
                    b"Efr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x50');
                    }
                    b"Ffr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x50');
                    }
                    b"Gfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x50');
                    }
                    b"Jfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x50');
                    }
                    b"Kfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x50');
                    }
                    b"Lfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x50');
                    }
                    b"Mfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x51');
                    }
                    b"Nfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x51');
                    }
                    b"Ofr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x51');
                    }
                    b"Pfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x51');
                    }
                    b"Qfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x51');
                    }
                    b"Sfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x51');
                    }
                    b"Tfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x51');
                    }
                    b"Ufr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x51');
                    }
                    b"Vfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x51');
                    }
                    b"Wfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x51');
                    }
                    b"Xfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x51');
                    }
                    b"Yfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x51');
                    }
                    b"afr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x51');
                    }
                    b"bfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x51');
                    }
                    b"cfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x52');
                    }
                    b"dfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x52');
                    }
                    b"efr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x52');
                    }
                    b"ffr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x52');
                    }
                    b"gfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x52');
                    }
                    b"hfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x52');
                    }
                    b"ifr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x52');
                    }
                    b"jfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x52');
                    }
                    b"kfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x52');
                    }
                    b"lfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x52');
                    }
                    b"mfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x52');
                    }
                    b"nfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x52');
                    }
                    b"ofr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x52');
                    }
                    b"pfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x52');
                    }
                    b"qfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x52');
                    }
                    b"rfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x52');
                    }
                    b"sfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x53');
                    }
                    b"tfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x53');
                    }
                    b"ufr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x53');
                    }
                    b"vfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x53');
                    }
                    b"wfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x53');
                    }
                    b"xfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x53');
                    }
                    b"yfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x53');
                    }
                    b"zfr" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x53');
                    }
                    b"Aopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x53');
                    }
                    b"Bopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x53');
                    }
                    b"Dopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x53');
                    }
                    b"Eopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x53');
                    }
                    b"Fopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x53');
                    }
                    b"Gopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x53');
                    }
                    b"Iopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x54');
                    }
                    b"Jopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x54');
                    }
                    b"Kopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x54');
                    }
                    b"Lopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x54');
                    }
                    b"Mopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x54');
                    }
                    b"Oopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x54');
                    }
                    b"Sopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x54');
                    }
                    b"Topf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x54');
                    }
                    b"Uopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x54');
                    }
                    b"Vopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x54');
                    }
                    b"Wopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x54');
                    }
                    b"Xopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x54');
                    }
                    b"Yopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x55');
                    }
                    b"aopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x55');
                    }
                    b"bopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x55');
                    }
                    b"copf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x55');
                    }
                    b"dopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x55');
                    }
                    b"eopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x55');
                    }
                    b"fopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x55');
                    }
                    b"gopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x55');
                    }
                    b"hopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x55');
                    }
                    b"iopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x55');
                    }
                    b"jopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x55');
                    }
                    b"kopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x55');
                    }
                    b"lopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x55');
                    }
                    b"mopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x55');
                    }
                    b"nopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x55');
                    }
                    b"oopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x56');
                    }
                    b"popf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x56');
                    }
                    b"qopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x56');
                    }
                    b"ropf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x56');
                    }
                    b"sopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x56');
                    }
                    b"topf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x56');
                    }
                    b"uopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x56');
                    }
                    b"vopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x56');
                    }
                    b"wopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x56');
                    }
                    b"xopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x56');
                    }
                    b"yopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x56');
                    }
                    b"zopf" => {
                        unescaped.push(b'\x1D');
                        unescaped.push(b'\x56');
                    }
                    bytes if bytes.starts_with(b"#") => {
                        let bytes = &bytes[1..];
                        let code = if bytes.starts_with(b"x") {
                            parse_hexadecimal(&bytes[1..])
                        } else {
                            parse_decimal(&bytes)
                        }?;
                        if code == 0 {
                            return Err(EscapeError::EntityWithNull(start..end));
                        }
                        push_utf8(unescaped, code);
                    }
                    bytes => match custom_entities.and_then(|hm| hm.get(bytes)) {
                        Some(value) => unescaped.extend_from_slice(&value),
                        None => {
                            return Err(EscapeError::UnrecognizedSymbol(
                                start + 1..end,
                                String::from_utf8(bytes.to_vec()),
                            ))
                        }
                    },
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
    assert!(unescape(b"&foo;").is_err());
}

#[test]
fn test_unescape_with() {
    let custom_entities = vec![(b"foo".to_vec(), b"BAR".to_vec())]
        .into_iter()
        .collect();
    assert_eq!(&*unescape_with(b"test", &custom_entities).unwrap(), b"test");
    assert_eq!(
        &*unescape_with(b"&lt;test&gt;", &custom_entities).unwrap(),
        b"<test>"
    );
    assert_eq!(&*unescape_with(b"&#x30;", &custom_entities).unwrap(), b"0");
    assert_eq!(&*unescape_with(b"&#48;", &custom_entities).unwrap(), b"0");
    assert_eq!(&*unescape_with(b"&foo;", &custom_entities).unwrap(), b"BAR");
    assert!(unescape_with(b"&fop;", &custom_entities).is_err());
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

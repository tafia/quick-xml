//! Manage xml character escapes

use memchr;
use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Range;

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
    // Not a valid unicode codepoint
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

                let mut push_char = |c: char| {
                    unescaped.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
                };

                // search for character correctness
                #[cfg(not(feature = "escape-html"))]
                match &raw[start + 1..end] {
                    b"lt" => push_char('<'),
                    b"gt" => push_char('>'),
                    b"amp" => push_char('&'),
                    b"apos" => push_char('\''),
                    b"quot" => push_char('\"'),
                    bytes if bytes.starts_with(b"#") => {
                        push_char(parse_number(&bytes[1..], start..end)?);
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
                    b"Tab" => push_char('\u{09}'),
                    b"NewLine" => push_char('\u{0A}'),
                    b"excl" => {
                        push_char('\u{21}');
                    }
                    b"quot" | b"QUOT" => {
                        push_char('\u{22}');
                    }
                    b"num" => {
                        push_char('\u{23}');
                    }
                    b"dollar" => {
                        push_char('\u{24}');
                    }
                    b"percnt" => {
                        push_char('\u{25}');
                    }
                    b"amp" | b"AMP" => {
                        push_char('\u{26}');
                    }
                    b"apos" => {
                        push_char('\u{27}');
                    }
                    b"lpar" => {
                        push_char('\u{28}');
                    }
                    b"rpar" => {
                        push_char('\u{29}');
                    }
                    b"ast" | b"midast" => {
                        push_char('\u{2A}');
                    }
                    b"plus" => {
                        push_char('\u{2B}');
                    }
                    b"comma" => {
                        push_char('\u{2C}');
                    }
                    b"period" => {
                        push_char('\u{2E}');
                    }
                    b"sol" => {
                        push_char('\u{2F}');
                    }
                    b"colon" => {
                        push_char('\u{3A}');
                    }
                    b"semi" => {
                        push_char('\u{3B}');
                    }
                    b"lt" | b"LT" => {
                        push_char('\u{3C}');
                    }
                    b"equals" => {
                        push_char('\u{3D}');
                    }
                    b"gt" | b"GT" => {
                        push_char('\u{3E}');
                    }
                    b"quest" => {
                        push_char('\u{3F}');
                    }
                    b"commat" => {
                        push_char('\u{40}');
                    }
                    b"lsqb" | b"lbrack" => {
                        push_char('\u{5B}');
                    }
                    b"bsol" => {
                        push_char('\u{5C}');
                    }
                    b"rsqb" | b"rbrack" => {
                        push_char('\u{5D}');
                    }
                    b"Hat" => {
                        push_char('\u{5E}');
                    }
                    b"lowbar" => {
                        push_char('\u{5F}');
                    }
                    b"grave" | b"DiacriticalGrave" => {
                        push_char('\u{60}');
                    }
                    b"lcub" | b"lbrace" => {
                        push_char('\u{7B}');
                    }
                    b"verbar" | b"vert" | b"VerticalLine" => {
                        push_char('\u{7C}');
                    }
                    b"rcub" | b"rbrace" => {
                        push_char('\u{7D}');
                    }
                    b"nbsp" | b"NonBreakingSpace" => {
                        push_char('\u{A0}');
                    }
                    b"iexcl" => {
                        push_char('\u{A1}');
                    }
                    b"cent" => {
                        push_char('\u{A2}');
                    }
                    b"pound" => {
                        push_char('\u{A3}');
                    }
                    b"curren" => {
                        push_char('\u{A4}');
                    }
                    b"yen" => {
                        push_char('\u{A5}');
                    }
                    b"brvbar" => {
                        push_char('\u{A6}');
                    }
                    b"sect" => {
                        push_char('\u{A7}');
                    }
                    b"Dot" | b"die" | b"DoubleDot" | b"uml" => {
                        push_char('\u{A8}');
                    }
                    b"copy" | b"COPY" => {
                        push_char('\u{A9}');
                    }
                    b"ordf" => {
                        push_char('\u{AA}');
                    }
                    b"laquo" => {
                        push_char('\u{AB}');
                    }
                    b"not" => {
                        push_char('\u{AC}');
                    }
                    b"shy" => {
                        push_char('\u{AD}');
                    }
                    b"reg" | b"circledR" | b"REG" => {
                        push_char('\u{AE}');
                    }
                    b"macr" | b"OverBar" | b"strns" => {
                        push_char('\u{AF}');
                    }
                    b"deg" => {
                        push_char('\u{B0}');
                    }
                    b"plusmn" | b"pm" | b"PlusMinus" => {
                        push_char('\u{B1}');
                    }
                    b"sup2" => {
                        push_char('\u{B2}');
                    }
                    b"sup3" => {
                        push_char('\u{B3}');
                    }
                    b"acute" | b"DiacriticalAcute" => {
                        push_char('\u{B4}');
                    }
                    b"micro" => {
                        push_char('\u{B5}');
                    }
                    b"para" => {
                        push_char('\u{B6}');
                    }
                    b"middot" | b"centerdot" | b"CenterDot" => {
                        push_char('\u{B7}');
                    }
                    b"cedil" | b"Cedilla" => {
                        push_char('\u{B8}');
                    }
                    b"sup1" => {
                        push_char('\u{B9}');
                    }
                    b"ordm" => {
                        push_char('\u{BA}');
                    }
                    b"raquo" => {
                        push_char('\u{BB}');
                    }
                    b"frac14" => {
                        push_char('\u{BC}');
                    }
                    b"frac12" | b"half" => {
                        push_char('\u{BD}');
                    }
                    b"frac34" => {
                        push_char('\u{BE}');
                    }
                    b"iquest" => {
                        push_char('\u{BF}');
                    }
                    b"Agrave" => {
                        push_char('\u{C0}');
                    }
                    b"Aacute" => {
                        push_char('\u{C1}');
                    }
                    b"Acirc" => {
                        push_char('\u{C2}');
                    }
                    b"Atilde" => {
                        push_char('\u{C3}');
                    }
                    b"Auml" => {
                        push_char('\u{C4}');
                    }
                    b"Aring" => {
                        push_char('\u{C5}');
                    }
                    b"AElig" => {
                        push_char('\u{C6}');
                    }
                    b"Ccedil" => {
                        push_char('\u{C7}');
                    }
                    b"Egrave" => {
                        push_char('\u{C8}');
                    }
                    b"Eacute" => {
                        push_char('\u{C9}');
                    }
                    b"Ecirc" => {
                        push_char('\u{CA}');
                    }
                    b"Euml" => {
                        push_char('\u{CB}');
                    }
                    b"Igrave" => {
                        push_char('\u{CC}');
                    }
                    b"Iacute" => {
                        push_char('\u{CD}');
                    }
                    b"Icirc" => {
                        push_char('\u{CE}');
                    }
                    b"Iuml" => {
                        push_char('\u{CF}');
                    }
                    b"ETH" => {
                        push_char('\u{D0}');
                    }
                    b"Ntilde" => {
                        push_char('\u{D1}');
                    }
                    b"Ograve" => {
                        push_char('\u{D2}');
                    }
                    b"Oacute" => {
                        push_char('\u{D3}');
                    }
                    b"Ocirc" => {
                        push_char('\u{D4}');
                    }
                    b"Otilde" => {
                        push_char('\u{D5}');
                    }
                    b"Ouml" => {
                        push_char('\u{D6}');
                    }
                    b"times" => {
                        push_char('\u{D7}');
                    }
                    b"Oslash" => {
                        push_char('\u{D8}');
                    }
                    b"Ugrave" => {
                        push_char('\u{D9}');
                    }
                    b"Uacute" => {
                        push_char('\u{DA}');
                    }
                    b"Ucirc" => {
                        push_char('\u{DB}');
                    }
                    b"Uuml" => {
                        push_char('\u{DC}');
                    }
                    b"Yacute" => {
                        push_char('\u{DD}');
                    }
                    b"THORN" => {
                        push_char('\u{DE}');
                    }
                    b"szlig" => {
                        push_char('\u{DF}');
                    }
                    b"agrave" => {
                        push_char('\u{E0}');
                    }
                    b"aacute" => {
                        push_char('\u{E1}');
                    }
                    b"acirc" => {
                        push_char('\u{E2}');
                    }
                    b"atilde" => {
                        push_char('\u{E3}');
                    }
                    b"auml" => {
                        push_char('\u{E4}');
                    }
                    b"aring" => {
                        push_char('\u{E5}');
                    }
                    b"aelig" => {
                        push_char('\u{E6}');
                    }
                    b"ccedil" => {
                        push_char('\u{E7}');
                    }
                    b"egrave" => {
                        push_char('\u{E8}');
                    }
                    b"eacute" => {
                        push_char('\u{E9}');
                    }
                    b"ecirc" => {
                        push_char('\u{EA}');
                    }
                    b"euml" => {
                        push_char('\u{EB}');
                    }
                    b"igrave" => {
                        push_char('\u{EC}');
                    }
                    b"iacute" => {
                        push_char('\u{ED}');
                    }
                    b"icirc" => {
                        push_char('\u{EE}');
                    }
                    b"iuml" => {
                        push_char('\u{EF}');
                    }
                    b"eth" => {
                        push_char('\u{F0}');
                    }
                    b"ntilde" => {
                        push_char('\u{F1}');
                    }
                    b"ograve" => {
                        push_char('\u{F2}');
                    }
                    b"oacute" => {
                        push_char('\u{F3}');
                    }
                    b"ocirc" => {
                        push_char('\u{F4}');
                    }
                    b"otilde" => {
                        push_char('\u{F5}');
                    }
                    b"ouml" => {
                        push_char('\u{F6}');
                    }
                    b"divide" | b"div" => {
                        push_char('\u{F7}');
                    }
                    b"oslash" => {
                        push_char('\u{F8}');
                    }
                    b"ugrave" => {
                        push_char('\u{F9}');
                    }
                    b"uacute" => {
                        push_char('\u{FA}');
                    }
                    b"ucirc" => {
                        push_char('\u{FB}');
                    }
                    b"uuml" => {
                        push_char('\u{FC}');
                    }
                    b"yacute" => {
                        push_char('\u{FD}');
                    }
                    b"thorn" => {
                        push_char('\u{FE}');
                    }
                    b"yuml" => {
                        push_char('\u{FF}');
                    }
                    b"Amacr" => {
                        push_char('\u{10}');
                    }
                    b"amacr" => {
                        push_char('\u{10}');
                    }
                    b"Abreve" => {
                        push_char('\u{10}');
                    }
                    b"abreve" => {
                        push_char('\u{10}');
                    }
                    b"Aogon" => {
                        push_char('\u{10}');
                    }
                    b"aogon" => {
                        push_char('\u{10}');
                    }
                    b"Cacute" => {
                        push_char('\u{10}');
                    }
                    b"cacute" => {
                        push_char('\u{10}');
                    }
                    b"Ccirc" => {
                        push_char('\u{10}');
                    }
                    b"ccirc" => {
                        push_char('\u{10}');
                    }
                    b"Cdot" => {
                        push_char('\u{10}');
                    }
                    b"cdot" => {
                        push_char('\u{10}');
                    }
                    b"Ccaron" => {
                        push_char('\u{10}');
                    }
                    b"ccaron" => {
                        push_char('\u{10}');
                    }
                    b"Dcaron" => {
                        push_char('\u{10}');
                    }
                    b"dcaron" => {
                        push_char('\u{10}');
                    }
                    b"Dstrok" => {
                        push_char('\u{11}');
                    }
                    b"dstrok" => {
                        push_char('\u{11}');
                    }
                    b"Emacr" => {
                        push_char('\u{11}');
                    }
                    b"emacr" => {
                        push_char('\u{11}');
                    }
                    b"Edot" => {
                        push_char('\u{11}');
                    }
                    b"edot" => {
                        push_char('\u{11}');
                    }
                    b"Eogon" => {
                        push_char('\u{11}');
                    }
                    b"eogon" => {
                        push_char('\u{11}');
                    }
                    b"Ecaron" => {
                        push_char('\u{11}');
                    }
                    b"ecaron" => {
                        push_char('\u{11}');
                    }
                    b"Gcirc" => {
                        push_char('\u{11}');
                    }
                    b"gcirc" => {
                        push_char('\u{11}');
                    }
                    b"Gbreve" => {
                        push_char('\u{11}');
                    }
                    b"gbreve" => {
                        push_char('\u{11}');
                    }
                    b"Gdot" => {
                        push_char('\u{12}');
                    }
                    b"gdot" => {
                        push_char('\u{12}');
                    }
                    b"Gcedil" => {
                        push_char('\u{12}');
                    }
                    b"Hcirc" => {
                        push_char('\u{12}');
                    }
                    b"hcirc" => {
                        push_char('\u{12}');
                    }
                    b"Hstrok" => {
                        push_char('\u{12}');
                    }
                    b"hstrok" => {
                        push_char('\u{12}');
                    }
                    b"Itilde" => {
                        push_char('\u{12}');
                    }
                    b"itilde" => {
                        push_char('\u{12}');
                    }
                    b"Imacr" => {
                        push_char('\u{12}');
                    }
                    b"imacr" => {
                        push_char('\u{12}');
                    }
                    b"Iogon" => {
                        push_char('\u{12}');
                    }
                    b"iogon" => {
                        push_char('\u{12}');
                    }
                    b"Idot" => {
                        push_char('\u{13}');
                    }
                    b"imath" | b"inodot" => {
                        push_char('\u{13}');
                    }
                    b"IJlig" => {
                        push_char('\u{13}');
                    }
                    b"ijlig" => {
                        push_char('\u{13}');
                    }
                    b"Jcirc" => {
                        push_char('\u{13}');
                    }
                    b"jcirc" => {
                        push_char('\u{13}');
                    }
                    b"Kcedil" => {
                        push_char('\u{13}');
                    }
                    b"kcedil" => {
                        push_char('\u{13}');
                    }
                    b"kgreen" => {
                        push_char('\u{13}');
                    }
                    b"Lacute" => {
                        push_char('\u{13}');
                    }
                    b"lacute" => {
                        push_char('\u{13}');
                    }
                    b"Lcedil" => {
                        push_char('\u{13}');
                    }
                    b"lcedil" => {
                        push_char('\u{13}');
                    }
                    b"Lcaron" => {
                        push_char('\u{13}');
                    }
                    b"lcaron" => {
                        push_char('\u{13}');
                    }
                    b"Lmidot" => {
                        push_char('\u{13}');
                    }
                    b"lmidot" => {
                        push_char('\u{14}');
                    }
                    b"Lstrok" => {
                        push_char('\u{14}');
                    }
                    b"lstrok" => {
                        push_char('\u{14}');
                    }
                    b"Nacute" => {
                        push_char('\u{14}');
                    }
                    b"nacute" => {
                        push_char('\u{14}');
                    }
                    b"Ncedil" => {
                        push_char('\u{14}');
                    }
                    b"ncedil" => {
                        push_char('\u{14}');
                    }
                    b"Ncaron" => {
                        push_char('\u{14}');
                    }
                    b"ncaron" => {
                        push_char('\u{14}');
                    }
                    b"napos" => {
                        push_char('\u{14}');
                    }
                    b"ENG" => {
                        push_char('\u{14}');
                    }
                    b"eng" => {
                        push_char('\u{14}');
                    }
                    b"Omacr" => {
                        push_char('\u{14}');
                    }
                    b"omacr" => {
                        push_char('\u{14}');
                    }
                    b"Odblac" => {
                        push_char('\u{15}');
                    }
                    b"odblac" => {
                        push_char('\u{15}');
                    }
                    b"OElig" => {
                        push_char('\u{15}');
                    }
                    b"oelig" => {
                        push_char('\u{15}');
                    }
                    b"Racute" => {
                        push_char('\u{15}');
                    }
                    b"racute" => {
                        push_char('\u{15}');
                    }
                    b"Rcedil" => {
                        push_char('\u{15}');
                    }
                    b"rcedil" => {
                        push_char('\u{15}');
                    }
                    b"Rcaron" => {
                        push_char('\u{15}');
                    }
                    b"rcaron" => {
                        push_char('\u{15}');
                    }
                    b"Sacute" => {
                        push_char('\u{15}');
                    }
                    b"sacute" => {
                        push_char('\u{15}');
                    }
                    b"Scirc" => {
                        push_char('\u{15}');
                    }
                    b"scirc" => {
                        push_char('\u{15}');
                    }
                    b"Scedil" => {
                        push_char('\u{15}');
                    }
                    b"scedil" => {
                        push_char('\u{15}');
                    }
                    b"Scaron" => {
                        push_char('\u{16}');
                    }
                    b"scaron" => {
                        push_char('\u{16}');
                    }
                    b"Tcedil" => {
                        push_char('\u{16}');
                    }
                    b"tcedil" => {
                        push_char('\u{16}');
                    }
                    b"Tcaron" => {
                        push_char('\u{16}');
                    }
                    b"tcaron" => {
                        push_char('\u{16}');
                    }
                    b"Tstrok" => {
                        push_char('\u{16}');
                    }
                    b"tstrok" => {
                        push_char('\u{16}');
                    }
                    b"Utilde" => {
                        push_char('\u{16}');
                    }
                    b"utilde" => {
                        push_char('\u{16}');
                    }
                    b"Umacr" => {
                        push_char('\u{16}');
                    }
                    b"umacr" => {
                        push_char('\u{16}');
                    }
                    b"Ubreve" => {
                        push_char('\u{16}');
                    }
                    b"ubreve" => {
                        push_char('\u{16}');
                    }
                    b"Uring" => {
                        push_char('\u{16}');
                    }
                    b"uring" => {
                        push_char('\u{16}');
                    }
                    b"Udblac" => {
                        push_char('\u{17}');
                    }
                    b"udblac" => {
                        push_char('\u{17}');
                    }
                    b"Uogon" => {
                        push_char('\u{17}');
                    }
                    b"uogon" => {
                        push_char('\u{17}');
                    }
                    b"Wcirc" => {
                        push_char('\u{17}');
                    }
                    b"wcirc" => {
                        push_char('\u{17}');
                    }
                    b"Ycirc" => {
                        push_char('\u{17}');
                    }
                    b"ycirc" => {
                        push_char('\u{17}');
                    }
                    b"Yuml" => {
                        push_char('\u{17}');
                    }
                    b"Zacute" => {
                        push_char('\u{17}');
                    }
                    b"zacute" => {
                        push_char('\u{17}');
                    }
                    b"Zdot" => {
                        push_char('\u{17}');
                    }
                    b"zdot" => {
                        push_char('\u{17}');
                    }
                    b"Zcaron" => {
                        push_char('\u{17}');
                    }
                    b"zcaron" => {
                        push_char('\u{17}');
                    }
                    b"fnof" => {
                        push_char('\u{19}');
                    }
                    b"imped" => {
                        push_char('\u{1B}');
                    }
                    b"gacute" => {
                        push_char('\u{1F}');
                    }
                    b"jmath" => {
                        push_char('\u{23}');
                    }
                    b"circ" => {
                        push_char('\u{2C}');
                    }
                    b"caron" | b"Hacek" => {
                        push_char('\u{2C}');
                    }
                    b"breve" | b"Breve" => {
                        push_char('\u{2D}');
                    }
                    b"dot" | b"DiacriticalDot" => {
                        push_char('\u{2D}');
                    }
                    b"ring" => {
                        push_char('\u{2D}');
                    }
                    b"ogon" => {
                        push_char('\u{2D}');
                    }
                    b"tilde" | b"DiacriticalTilde" => {
                        push_char('\u{2D}');
                    }
                    b"dblac" | b"DiacriticalDoubleAcute" => {
                        push_char('\u{2D}');
                    }
                    b"DownBreve" => {
                        push_char('\u{31}');
                    }
                    b"UnderBar" => {
                        push_char('\u{33}');
                    }
                    b"Alpha" => {
                        push_char('\u{39}');
                    }
                    b"Beta" => {
                        push_char('\u{39}');
                    }
                    b"Gamma" => {
                        push_char('\u{39}');
                    }
                    b"Delta" => {
                        push_char('\u{39}');
                    }
                    b"Epsilon" => {
                        push_char('\u{39}');
                    }
                    b"Zeta" => {
                        push_char('\u{39}');
                    }
                    b"Eta" => {
                        push_char('\u{39}');
                    }
                    b"Theta" => {
                        push_char('\u{39}');
                    }
                    b"Iota" => {
                        push_char('\u{39}');
                    }
                    b"Kappa" => {
                        push_char('\u{39}');
                    }
                    b"Lambda" => {
                        push_char('\u{39}');
                    }
                    b"Mu" => {
                        push_char('\u{39}');
                    }
                    b"Nu" => {
                        push_char('\u{39}');
                    }
                    b"Xi" => {
                        push_char('\u{39}');
                    }
                    b"Omicron" => {
                        push_char('\u{39}');
                    }
                    b"Pi" => {
                        push_char('\u{3A}');
                    }
                    b"Rho" => {
                        push_char('\u{3A}');
                    }
                    b"Sigma" => {
                        push_char('\u{3A}');
                    }
                    b"Tau" => {
                        push_char('\u{3A}');
                    }
                    b"Upsilon" => {
                        push_char('\u{3A}');
                    }
                    b"Phi" => {
                        push_char('\u{3A}');
                    }
                    b"Chi" => {
                        push_char('\u{3A}');
                    }
                    b"Psi" => {
                        push_char('\u{3A}');
                    }
                    b"Omega" => {
                        push_char('\u{3A}');
                    }
                    b"alpha" => {
                        push_char('\u{3B}');
                    }
                    b"beta" => {
                        push_char('\u{3B}');
                    }
                    b"gamma" => {
                        push_char('\u{3B}');
                    }
                    b"delta" => {
                        push_char('\u{3B}');
                    }
                    b"epsiv" | b"varepsilon" | b"epsilon" => {
                        push_char('\u{3B}');
                    }
                    b"zeta" => {
                        push_char('\u{3B}');
                    }
                    b"eta" => {
                        push_char('\u{3B}');
                    }
                    b"theta" => {
                        push_char('\u{3B}');
                    }
                    b"iota" => {
                        push_char('\u{3B}');
                    }
                    b"kappa" => {
                        push_char('\u{3B}');
                    }
                    b"lambda" => {
                        push_char('\u{3B}');
                    }
                    b"mu" => {
                        push_char('\u{3B}');
                    }
                    b"nu" => {
                        push_char('\u{3B}');
                    }
                    b"xi" => {
                        push_char('\u{3B}');
                    }
                    b"omicron" => {
                        push_char('\u{3B}');
                    }
                    b"pi" => {
                        push_char('\u{3C}');
                    }
                    b"rho" => {
                        push_char('\u{3C}');
                    }
                    b"sigmav" | b"varsigma" | b"sigmaf" => {
                        push_char('\u{3C}');
                    }
                    b"sigma" => {
                        push_char('\u{3C}');
                    }
                    b"tau" => {
                        push_char('\u{3C}');
                    }
                    b"upsi" | b"upsilon" => {
                        push_char('\u{3C}');
                    }
                    b"phi" | b"phiv" | b"varphi" => {
                        push_char('\u{3C}');
                    }
                    b"chi" => {
                        push_char('\u{3C}');
                    }
                    b"psi" => {
                        push_char('\u{3C}');
                    }
                    b"omega" => {
                        push_char('\u{3C}');
                    }
                    b"thetav" | b"vartheta" | b"thetasym" => {
                        push_char('\u{3D}');
                    }
                    b"Upsi" | b"upsih" => {
                        push_char('\u{3D}');
                    }
                    b"straightphi" => {
                        push_char('\u{3D}');
                    }
                    b"piv" | b"varpi" => {
                        push_char('\u{3D}');
                    }
                    b"Gammad" => {
                        push_char('\u{3D}');
                    }
                    b"gammad" | b"digamma" => {
                        push_char('\u{3D}');
                    }
                    b"kappav" | b"varkappa" => {
                        push_char('\u{3F}');
                    }
                    b"rhov" | b"varrho" => {
                        push_char('\u{3F}');
                    }
                    b"epsi" | b"straightepsilon" => {
                        push_char('\u{3F}');
                    }
                    b"bepsi" | b"backepsilon" => {
                        push_char('\u{3F}');
                    }
                    b"IOcy" => {
                        push_char('\u{40}');
                    }
                    b"DJcy" => {
                        push_char('\u{40}');
                    }
                    b"GJcy" => {
                        push_char('\u{40}');
                    }
                    b"Jukcy" => {
                        push_char('\u{40}');
                    }
                    b"DScy" => {
                        push_char('\u{40}');
                    }
                    b"Iukcy" => {
                        push_char('\u{40}');
                    }
                    b"YIcy" => {
                        push_char('\u{40}');
                    }
                    b"Jsercy" => {
                        push_char('\u{40}');
                    }
                    b"LJcy" => {
                        push_char('\u{40}');
                    }
                    b"NJcy" => {
                        push_char('\u{40}');
                    }
                    b"TSHcy" => {
                        push_char('\u{40}');
                    }
                    b"KJcy" => {
                        push_char('\u{40}');
                    }
                    b"Ubrcy" => {
                        push_char('\u{40}');
                    }
                    b"DZcy" => {
                        push_char('\u{40}');
                    }
                    b"Acy" => {
                        push_char('\u{41}');
                    }
                    b"Bcy" => {
                        push_char('\u{41}');
                    }
                    b"Vcy" => {
                        push_char('\u{41}');
                    }
                    b"Gcy" => {
                        push_char('\u{41}');
                    }
                    b"Dcy" => {
                        push_char('\u{41}');
                    }
                    b"IEcy" => {
                        push_char('\u{41}');
                    }
                    b"ZHcy" => {
                        push_char('\u{41}');
                    }
                    b"Zcy" => {
                        push_char('\u{41}');
                    }
                    b"Icy" => {
                        push_char('\u{41}');
                    }
                    b"Jcy" => {
                        push_char('\u{41}');
                    }
                    b"Kcy" => {
                        push_char('\u{41}');
                    }
                    b"Lcy" => {
                        push_char('\u{41}');
                    }
                    b"Mcy" => {
                        push_char('\u{41}');
                    }
                    b"Ncy" => {
                        push_char('\u{41}');
                    }
                    b"Ocy" => {
                        push_char('\u{41}');
                    }
                    b"Pcy" => {
                        push_char('\u{41}');
                    }
                    b"Rcy" => {
                        push_char('\u{42}');
                    }
                    b"Scy" => {
                        push_char('\u{42}');
                    }
                    b"Tcy" => {
                        push_char('\u{42}');
                    }
                    b"Ucy" => {
                        push_char('\u{42}');
                    }
                    b"Fcy" => {
                        push_char('\u{42}');
                    }
                    b"KHcy" => {
                        push_char('\u{42}');
                    }
                    b"TScy" => {
                        push_char('\u{42}');
                    }
                    b"CHcy" => {
                        push_char('\u{42}');
                    }
                    b"SHcy" => {
                        push_char('\u{42}');
                    }
                    b"SHCHcy" => {
                        push_char('\u{42}');
                    }
                    b"HARDcy" => {
                        push_char('\u{42}');
                    }
                    b"Ycy" => {
                        push_char('\u{42}');
                    }
                    b"SOFTcy" => {
                        push_char('\u{42}');
                    }
                    b"Ecy" => {
                        push_char('\u{42}');
                    }
                    b"YUcy" => {
                        push_char('\u{42}');
                    }
                    b"YAcy" => {
                        push_char('\u{42}');
                    }
                    b"acy" => {
                        push_char('\u{43}');
                    }
                    b"bcy" => {
                        push_char('\u{43}');
                    }
                    b"vcy" => {
                        push_char('\u{43}');
                    }
                    b"gcy" => {
                        push_char('\u{43}');
                    }
                    b"dcy" => {
                        push_char('\u{43}');
                    }
                    b"iecy" => {
                        push_char('\u{43}');
                    }
                    b"zhcy" => {
                        push_char('\u{43}');
                    }
                    b"zcy" => {
                        push_char('\u{43}');
                    }
                    b"icy" => {
                        push_char('\u{43}');
                    }
                    b"jcy" => {
                        push_char('\u{43}');
                    }
                    b"kcy" => {
                        push_char('\u{43}');
                    }
                    b"lcy" => {
                        push_char('\u{43}');
                    }
                    b"mcy" => {
                        push_char('\u{43}');
                    }
                    b"ncy" => {
                        push_char('\u{43}');
                    }
                    b"ocy" => {
                        push_char('\u{43}');
                    }
                    b"pcy" => {
                        push_char('\u{43}');
                    }
                    b"rcy" => {
                        push_char('\u{44}');
                    }
                    b"scy" => {
                        push_char('\u{44}');
                    }
                    b"tcy" => {
                        push_char('\u{44}');
                    }
                    b"ucy" => {
                        push_char('\u{44}');
                    }
                    b"fcy" => {
                        push_char('\u{44}');
                    }
                    b"khcy" => {
                        push_char('\u{44}');
                    }
                    b"tscy" => {
                        push_char('\u{44}');
                    }
                    b"chcy" => {
                        push_char('\u{44}');
                    }
                    b"shcy" => {
                        push_char('\u{44}');
                    }
                    b"shchcy" => {
                        push_char('\u{44}');
                    }
                    b"hardcy" => {
                        push_char('\u{44}');
                    }
                    b"ycy" => {
                        push_char('\u{44}');
                    }
                    b"softcy" => {
                        push_char('\u{44}');
                    }
                    b"ecy" => {
                        push_char('\u{44}');
                    }
                    b"yucy" => {
                        push_char('\u{44}');
                    }
                    b"yacy" => {
                        push_char('\u{44}');
                    }
                    b"iocy" => {
                        push_char('\u{45}');
                    }
                    b"djcy" => {
                        push_char('\u{45}');
                    }
                    b"gjcy" => {
                        push_char('\u{45}');
                    }
                    b"jukcy" => {
                        push_char('\u{45}');
                    }
                    b"dscy" => {
                        push_char('\u{45}');
                    }
                    b"iukcy" => {
                        push_char('\u{45}');
                    }
                    b"yicy" => {
                        push_char('\u{45}');
                    }
                    b"jsercy" => {
                        push_char('\u{45}');
                    }
                    b"ljcy" => {
                        push_char('\u{45}');
                    }
                    b"njcy" => {
                        push_char('\u{45}');
                    }
                    b"tshcy" => {
                        push_char('\u{45}');
                    }
                    b"kjcy" => {
                        push_char('\u{45}');
                    }
                    b"ubrcy" => {
                        push_char('\u{45}');
                    }
                    b"dzcy" => {
                        push_char('\u{45}');
                    }
                    b"ensp" => {
                        push_char('\u{2002}');
                    }
                    b"emsp" => {
                        push_char('\u{2003}');
                    }
                    b"emsp13" => {
                        push_char('\u{2004}');
                    }
                    b"emsp14" => {
                        push_char('\u{2005}');
                    }
                    b"numsp" => {
                        push_char('\u{2007}');
                    }
                    b"puncsp" => {
                        push_char('\u{2008}');
                    }
                    b"thinsp" | b"ThinSpace" => {
                        push_char('\u{2009}');
                    }
                    b"hairsp" | b"VeryThinSpace" => {
                        push_char('\u{200A}');
                    }
                    b"ZeroWidthSpace"
                    | b"NegativeVeryThinSpace"
                    | b"NegativeThinSpace"
                    | b"NegativeMediumSpace"
                    | b"NegativeThickSpace" => {
                        push_char('\u{200B}');
                    }
                    b"zwnj" => {
                        push_char('\u{200C}');
                    }
                    b"zwj" => {
                        push_char('\u{200D}');
                    }
                    b"lrm" => {
                        push_char('\u{200E}');
                    }
                    b"rlm" => {
                        push_char('\u{200F}');
                    }
                    b"hyphen" | b"dash" => {
                        push_char('\u{2010}');
                    }
                    b"ndash" => {
                        push_char('\u{2013}');
                    }
                    b"mdash" => {
                        push_char('\u{2014}');
                    }
                    b"horbar" => {
                        push_char('\u{2015}');
                    }
                    b"Verbar" | b"Vert" => {
                        push_char('\u{2016}');
                    }
                    b"lsquo" | b"OpenCurlyQuote" => {
                        push_char('\u{2018}');
                    }
                    b"rsquo" | b"rsquor" | b"CloseCurlyQuote" => {
                        push_char('\u{2019}');
                    }
                    b"lsquor" | b"sbquo" => {
                        push_char('\u{201A}');
                    }
                    b"ldquo" | b"OpenCurlyDoubleQuote" => {
                        push_char('\u{201C}');
                    }
                    b"rdquo" | b"rdquor" | b"CloseCurlyDoubleQuote" => {
                        push_char('\u{201D}');
                    }
                    b"ldquor" | b"bdquo" => {
                        push_char('\u{201E}');
                    }
                    b"dagger" => {
                        push_char('\u{2020}');
                    }
                    b"Dagger" | b"ddagger" => {
                        push_char('\u{2021}');
                    }
                    b"bull" | b"bullet" => {
                        push_char('\u{2022}');
                    }
                    b"nldr" => {
                        push_char('\u{2025}');
                    }
                    b"hellip" | b"mldr" => {
                        push_char('\u{2026}');
                    }
                    b"permil" => {
                        push_char('\u{2030}');
                    }
                    b"pertenk" => {
                        push_char('\u{2031}');
                    }
                    b"prime" => {
                        push_char('\u{2032}');
                    }
                    b"Prime" => {
                        push_char('\u{2033}');
                    }
                    b"tprime" => {
                        push_char('\u{2034}');
                    }
                    b"bprime" | b"backprime" => {
                        push_char('\u{2035}');
                    }
                    b"lsaquo" => {
                        push_char('\u{2039}');
                    }
                    b"rsaquo" => {
                        push_char('\u{203A}');
                    }
                    b"oline" => {
                        push_char('\u{203E}');
                    }
                    b"caret" => {
                        push_char('\u{2041}');
                    }
                    b"hybull" => {
                        push_char('\u{2043}');
                    }
                    b"frasl" => {
                        push_char('\u{2044}');
                    }
                    b"bsemi" => {
                        push_char('\u{204F}');
                    }
                    b"qprime" => {
                        push_char('\u{2057}');
                    }
                    b"MediumSpace" => {
                        push_char('\u{205F}');
                    }
                    b"NoBreak" => {
                        push_char('\u{2060}');
                    }
                    b"ApplyFunction" | b"af" => {
                        push_char('\u{2061}');
                    }
                    b"InvisibleTimes" | b"it" => {
                        push_char('\u{2062}');
                    }
                    b"InvisibleComma" | b"ic" => {
                        push_char('\u{2063}');
                    }
                    b"euro" => {
                        push_char('\u{20AC}');
                    }
                    b"tdot" | b"TripleDot" => {
                        push_char('\u{20DB}');
                    }
                    b"DotDot" => {
                        push_char('\u{20DC}');
                    }
                    b"Copf" | b"complexes" => {
                        push_char('\u{2102}');
                    }
                    b"incare" => {
                        push_char('\u{2105}');
                    }
                    b"gscr" => {
                        push_char('\u{210A}');
                    }
                    b"hamilt" | b"HilbertSpace" | b"Hscr" => {
                        push_char('\u{210B}');
                    }
                    b"Hfr" | b"Poincareplane" => {
                        push_char('\u{210C}');
                    }
                    b"quaternions" | b"Hopf" => {
                        push_char('\u{210D}');
                    }
                    b"planckh" => {
                        push_char('\u{210E}');
                    }
                    b"planck" | b"hbar" | b"plankv" | b"hslash" => {
                        push_char('\u{210F}');
                    }
                    b"Iscr" | b"imagline" => {
                        push_char('\u{2110}');
                    }
                    b"image" | b"Im" | b"imagpart" | b"Ifr" => {
                        push_char('\u{2111}');
                    }
                    b"Lscr" | b"lagran" | b"Laplacetrf" => {
                        push_char('\u{2112}');
                    }
                    b"ell" => {
                        push_char('\u{2113}');
                    }
                    b"Nopf" | b"naturals" => {
                        push_char('\u{2115}');
                    }
                    b"numero" => {
                        push_char('\u{2116}');
                    }
                    b"copysr" => {
                        push_char('\u{2117}');
                    }
                    b"weierp" | b"wp" => {
                        push_char('\u{2118}');
                    }
                    b"Popf" | b"primes" => {
                        push_char('\u{2119}');
                    }
                    b"rationals" | b"Qopf" => {
                        push_char('\u{211A}');
                    }
                    b"Rscr" | b"realine" => {
                        push_char('\u{211B}');
                    }
                    b"real" | b"Re" | b"realpart" | b"Rfr" => {
                        push_char('\u{211C}');
                    }
                    b"reals" | b"Ropf" => {
                        push_char('\u{211D}');
                    }
                    b"rx" => {
                        push_char('\u{211E}');
                    }
                    b"trade" | b"TRADE" => {
                        push_char('\u{2122}');
                    }
                    b"integers" | b"Zopf" => {
                        push_char('\u{2124}');
                    }
                    b"ohm" => {
                        push_char('\u{2126}');
                    }
                    b"mho" => {
                        push_char('\u{2127}');
                    }
                    b"Zfr" | b"zeetrf" => {
                        push_char('\u{2128}');
                    }
                    b"iiota" => {
                        push_char('\u{2129}');
                    }
                    b"angst" => {
                        push_char('\u{212B}');
                    }
                    b"bernou" | b"Bernoullis" | b"Bscr" => {
                        push_char('\u{212C}');
                    }
                    b"Cfr" | b"Cayleys" => {
                        push_char('\u{212D}');
                    }
                    b"escr" => {
                        push_char('\u{212F}');
                    }
                    b"Escr" | b"expectation" => {
                        push_char('\u{2130}');
                    }
                    b"Fscr" | b"Fouriertrf" => {
                        push_char('\u{2131}');
                    }
                    b"phmmat" | b"Mellintrf" | b"Mscr" => {
                        push_char('\u{2133}');
                    }
                    b"order" | b"orderof" | b"oscr" => {
                        push_char('\u{2134}');
                    }
                    b"alefsym" | b"aleph" => {
                        push_char('\u{2135}');
                    }
                    b"beth" => {
                        push_char('\u{2136}');
                    }
                    b"gimel" => {
                        push_char('\u{2137}');
                    }
                    b"daleth" => {
                        push_char('\u{2138}');
                    }
                    b"CapitalDifferentialD" | b"DD" => {
                        push_char('\u{2145}');
                    }
                    b"DifferentialD" | b"dd" => {
                        push_char('\u{2146}');
                    }
                    b"ExponentialE" | b"exponentiale" | b"ee" => {
                        push_char('\u{2147}');
                    }
                    b"ImaginaryI" | b"ii" => {
                        push_char('\u{2148}');
                    }
                    b"frac13" => {
                        push_char('\u{2153}');
                    }
                    b"frac23" => {
                        push_char('\u{2154}');
                    }
                    b"frac15" => {
                        push_char('\u{2155}');
                    }
                    b"frac25" => {
                        push_char('\u{2156}');
                    }
                    b"frac35" => {
                        push_char('\u{2157}');
                    }
                    b"frac45" => {
                        push_char('\u{2158}');
                    }
                    b"frac16" => {
                        push_char('\u{2159}');
                    }
                    b"frac56" => {
                        push_char('\u{215A}');
                    }
                    b"frac18" => {
                        push_char('\u{215B}');
                    }
                    b"frac38" => {
                        push_char('\u{215C}');
                    }
                    b"frac58" => {
                        push_char('\u{215D}');
                    }
                    b"frac78" => {
                        push_char('\u{215E}');
                    }
                    b"larr" | b"leftarrow" | b"LeftArrow" | b"slarr" | b"ShortLeftArrow" => {
                        push_char('\u{2190}');
                    }
                    b"uarr" | b"uparrow" | b"UpArrow" | b"ShortUpArrow" => {
                        push_char('\u{2191}');
                    }
                    b"rarr" | b"rightarrow" | b"RightArrow" | b"srarr" | b"ShortRightArrow" => {
                        push_char('\u{2192}');
                    }
                    b"darr" | b"downarrow" | b"DownArrow" | b"ShortDownArrow" => {
                        push_char('\u{2193}');
                    }
                    b"harr" | b"leftrightarrow" | b"LeftRightArrow" => {
                        push_char('\u{2194}');
                    }
                    b"varr" | b"updownarrow" | b"UpDownArrow" => {
                        push_char('\u{2195}');
                    }
                    b"nwarr" | b"UpperLeftArrow" | b"nwarrow" => {
                        push_char('\u{2196}');
                    }
                    b"nearr" | b"UpperRightArrow" | b"nearrow" => {
                        push_char('\u{2197}');
                    }
                    b"searr" | b"searrow" | b"LowerRightArrow" => {
                        push_char('\u{2198}');
                    }
                    b"swarr" | b"swarrow" | b"LowerLeftArrow" => {
                        push_char('\u{2199}');
                    }
                    b"nlarr" | b"nleftarrow" => {
                        push_char('\u{219A}');
                    }
                    b"nrarr" | b"nrightarrow" => {
                        push_char('\u{219B}');
                    }
                    b"rarrw" | b"rightsquigarrow" => {
                        push_char('\u{219D}');
                    }
                    b"Larr" | b"twoheadleftarrow" => {
                        push_char('\u{219E}');
                    }
                    b"Uarr" => {
                        push_char('\u{219F}');
                    }
                    b"Rarr" | b"twoheadrightarrow" => {
                        push_char('\u{21A0}');
                    }
                    b"Darr" => {
                        push_char('\u{21A1}');
                    }
                    b"larrtl" | b"leftarrowtail" => {
                        push_char('\u{21A2}');
                    }
                    b"rarrtl" | b"rightarrowtail" => {
                        push_char('\u{21A3}');
                    }
                    b"LeftTeeArrow" | b"mapstoleft" => {
                        push_char('\u{21A4}');
                    }
                    b"UpTeeArrow" | b"mapstoup" => {
                        push_char('\u{21A5}');
                    }
                    b"map" | b"RightTeeArrow" | b"mapsto" => {
                        push_char('\u{21A6}');
                    }
                    b"DownTeeArrow" | b"mapstodown" => {
                        push_char('\u{21A7}');
                    }
                    b"larrhk" | b"hookleftarrow" => {
                        push_char('\u{21A9}');
                    }
                    b"rarrhk" | b"hookrightarrow" => {
                        push_char('\u{21AA}');
                    }
                    b"larrlp" | b"looparrowleft" => {
                        push_char('\u{21AB}');
                    }
                    b"rarrlp" | b"looparrowright" => {
                        push_char('\u{21AC}');
                    }
                    b"harrw" | b"leftrightsquigarrow" => {
                        push_char('\u{21AD}');
                    }
                    b"nharr" | b"nleftrightarrow" => {
                        push_char('\u{21AE}');
                    }
                    b"lsh" | b"Lsh" => {
                        push_char('\u{21B0}');
                    }
                    b"rsh" | b"Rsh" => {
                        push_char('\u{21B1}');
                    }
                    b"ldsh" => {
                        push_char('\u{21B2}');
                    }
                    b"rdsh" => {
                        push_char('\u{21B3}');
                    }
                    b"crarr" => {
                        push_char('\u{21B5}');
                    }
                    b"cularr" | b"curvearrowleft" => {
                        push_char('\u{21B6}');
                    }
                    b"curarr" | b"curvearrowright" => {
                        push_char('\u{21B7}');
                    }
                    b"olarr" | b"circlearrowleft" => {
                        push_char('\u{21BA}');
                    }
                    b"orarr" | b"circlearrowright" => {
                        push_char('\u{21BB}');
                    }
                    b"lharu" | b"LeftVector" | b"leftharpoonup" => {
                        push_char('\u{21BC}');
                    }
                    b"lhard" | b"leftharpoondown" | b"DownLeftVector" => {
                        push_char('\u{21BD}');
                    }
                    b"uharr" | b"upharpoonright" | b"RightUpVector" => {
                        push_char('\u{21BE}');
                    }
                    b"uharl" | b"upharpoonleft" | b"LeftUpVector" => {
                        push_char('\u{21BF}');
                    }
                    b"rharu" | b"RightVector" | b"rightharpoonup" => {
                        push_char('\u{21C0}');
                    }
                    b"rhard" | b"rightharpoondown" | b"DownRightVector" => {
                        push_char('\u{21C1}');
                    }
                    b"dharr" | b"RightDownVector" | b"downharpoonright" => {
                        push_char('\u{21C2}');
                    }
                    b"dharl" | b"LeftDownVector" | b"downharpoonleft" => {
                        push_char('\u{21C3}');
                    }
                    b"rlarr" | b"rightleftarrows" | b"RightArrowLeftArrow" => {
                        push_char('\u{21C4}');
                    }
                    b"udarr" | b"UpArrowDownArrow" => {
                        push_char('\u{21C5}');
                    }
                    b"lrarr" | b"leftrightarrows" | b"LeftArrowRightArrow" => {
                        push_char('\u{21C6}');
                    }
                    b"llarr" | b"leftleftarrows" => {
                        push_char('\u{21C7}');
                    }
                    b"uuarr" | b"upuparrows" => {
                        push_char('\u{21C8}');
                    }
                    b"rrarr" | b"rightrightarrows" => {
                        push_char('\u{21C9}');
                    }
                    b"ddarr" | b"downdownarrows" => {
                        push_char('\u{21CA}');
                    }
                    b"lrhar" | b"ReverseEquilibrium" | b"leftrightharpoons" => {
                        push_char('\u{21CB}');
                    }
                    b"rlhar" | b"rightleftharpoons" | b"Equilibrium" => {
                        push_char('\u{21CC}');
                    }
                    b"nlArr" | b"nLeftarrow" => {
                        push_char('\u{21CD}');
                    }
                    b"nhArr" | b"nLeftrightarrow" => {
                        push_char('\u{21CE}');
                    }
                    b"nrArr" | b"nRightarrow" => {
                        push_char('\u{21CF}');
                    }
                    b"lArr" | b"Leftarrow" | b"DoubleLeftArrow" => {
                        push_char('\u{21D0}');
                    }
                    b"uArr" | b"Uparrow" | b"DoubleUpArrow" => {
                        push_char('\u{21D1}');
                    }
                    b"rArr" | b"Rightarrow" | b"Implies" | b"DoubleRightArrow" => {
                        push_char('\u{21D2}');
                    }
                    b"dArr" | b"Downarrow" | b"DoubleDownArrow" => {
                        push_char('\u{21D3}');
                    }
                    b"hArr" | b"Leftrightarrow" | b"DoubleLeftRightArrow" | b"iff" => {
                        push_char('\u{21D4}');
                    }
                    b"vArr" | b"Updownarrow" | b"DoubleUpDownArrow" => {
                        push_char('\u{21D5}');
                    }
                    b"nwArr" => {
                        push_char('\u{21D6}');
                    }
                    b"neArr" => {
                        push_char('\u{21D7}');
                    }
                    b"seArr" => {
                        push_char('\u{21D8}');
                    }
                    b"swArr" => {
                        push_char('\u{21D9}');
                    }
                    b"lAarr" | b"Lleftarrow" => {
                        push_char('\u{21DA}');
                    }
                    b"rAarr" | b"Rrightarrow" => {
                        push_char('\u{21DB}');
                    }
                    b"zigrarr" => {
                        push_char('\u{21DD}');
                    }
                    b"larrb" | b"LeftArrowBar" => {
                        push_char('\u{21E4}');
                    }
                    b"rarrb" | b"RightArrowBar" => {
                        push_char('\u{21E5}');
                    }
                    b"duarr" | b"DownArrowUpArrow" => {
                        push_char('\u{21F5}');
                    }
                    b"loarr" => {
                        push_char('\u{21FD}');
                    }
                    b"roarr" => {
                        push_char('\u{21FE}');
                    }
                    b"hoarr" => {
                        push_char('\u{21FF}');
                    }
                    b"forall" | b"ForAll" => {
                        push_char('\u{2200}');
                    }
                    b"comp" | b"complement" => {
                        push_char('\u{2201}');
                    }
                    b"part" | b"PartialD" => {
                        push_char('\u{2202}');
                    }
                    b"exist" | b"Exists" => {
                        push_char('\u{2203}');
                    }
                    b"nexist" | b"NotExists" | b"nexists" => {
                        push_char('\u{2204}');
                    }
                    b"empty" | b"emptyset" | b"emptyv" | b"varnothing" => {
                        push_char('\u{2205}');
                    }
                    b"nabla" | b"Del" => {
                        push_char('\u{2207}');
                    }
                    b"isin" | b"isinv" | b"Element" | b"in" => {
                        push_char('\u{2208}');
                    }
                    b"notin" | b"NotElement" | b"notinva" => {
                        push_char('\u{2209}');
                    }
                    b"niv" | b"ReverseElement" | b"ni" | b"SuchThat" => {
                        push_char('\u{220B}');
                    }
                    b"notni" | b"notniva" | b"NotReverseElement" => {
                        push_char('\u{220C}');
                    }
                    b"prod" | b"Product" => {
                        push_char('\u{220F}');
                    }
                    b"coprod" | b"Coproduct" => {
                        push_char('\u{2210}');
                    }
                    b"sum" | b"Sum" => {
                        push_char('\u{2211}');
                    }
                    b"minus" => {
                        push_char('\u{2212}');
                    }
                    b"mnplus" | b"mp" | b"MinusPlus" => {
                        push_char('\u{2213}');
                    }
                    b"plusdo" | b"dotplus" => {
                        push_char('\u{2214}');
                    }
                    b"setmn" | b"setminus" | b"Backslash" | b"ssetmn" | b"smallsetminus" => {
                        push_char('\u{2216}');
                    }
                    b"lowast" => {
                        push_char('\u{2217}');
                    }
                    b"compfn" | b"SmallCircle" => {
                        push_char('\u{2218}');
                    }
                    b"radic" | b"Sqrt" => {
                        push_char('\u{221A}');
                    }
                    b"prop" | b"propto" | b"Proportional" | b"vprop" | b"varpropto" => {
                        push_char('\u{221D}');
                    }
                    b"infin" => {
                        push_char('\u{221E}');
                    }
                    b"angrt" => {
                        push_char('\u{221F}');
                    }
                    b"ang" | b"angle" => {
                        push_char('\u{2220}');
                    }
                    b"angmsd" | b"measuredangle" => {
                        push_char('\u{2221}');
                    }
                    b"angsph" => {
                        push_char('\u{2222}');
                    }
                    b"mid" | b"VerticalBar" | b"smid" | b"shortmid" => {
                        push_char('\u{2223}');
                    }
                    b"nmid" | b"NotVerticalBar" | b"nsmid" | b"nshortmid" => {
                        push_char('\u{2224}');
                    }
                    b"par" | b"parallel" | b"DoubleVerticalBar" | b"spar" | b"shortparallel" => {
                        push_char('\u{2225}');
                    }
                    b"npar"
                    | b"nparallel"
                    | b"NotDoubleVerticalBar"
                    | b"nspar"
                    | b"nshortparallel" => {
                        push_char('\u{2226}');
                    }
                    b"and" | b"wedge" => {
                        push_char('\u{2227}');
                    }
                    b"or" | b"vee" => {
                        push_char('\u{2228}');
                    }
                    b"cap" => {
                        push_char('\u{2229}');
                    }
                    b"cup" => {
                        push_char('\u{222A}');
                    }
                    b"int" | b"Integral" => {
                        push_char('\u{222B}');
                    }
                    b"Int" => {
                        push_char('\u{222C}');
                    }
                    b"tint" | b"iiint" => {
                        push_char('\u{222D}');
                    }
                    b"conint" | b"oint" | b"ContourIntegral" => {
                        push_char('\u{222E}');
                    }
                    b"Conint" | b"DoubleContourIntegral" => {
                        push_char('\u{222F}');
                    }
                    b"Cconint" => {
                        push_char('\u{2230}');
                    }
                    b"cwint" => {
                        push_char('\u{2231}');
                    }
                    b"cwconint" | b"ClockwiseContourIntegral" => {
                        push_char('\u{2232}');
                    }
                    b"awconint" | b"CounterClockwiseContourIntegral" => {
                        push_char('\u{2233}');
                    }
                    b"there4" | b"therefore" | b"Therefore" => {
                        push_char('\u{2234}');
                    }
                    b"becaus" | b"because" | b"Because" => {
                        push_char('\u{2235}');
                    }
                    b"ratio" => {
                        push_char('\u{2236}');
                    }
                    b"Colon" | b"Proportion" => {
                        push_char('\u{2237}');
                    }
                    b"minusd" | b"dotminus" => {
                        push_char('\u{2238}');
                    }
                    b"mDDot" => {
                        push_char('\u{223A}');
                    }
                    b"homtht" => {
                        push_char('\u{223B}');
                    }
                    b"sim" | b"Tilde" | b"thksim" | b"thicksim" => {
                        push_char('\u{223C}');
                    }
                    b"bsim" | b"backsim" => {
                        push_char('\u{223D}');
                    }
                    b"ac" | b"mstpos" => {
                        push_char('\u{223E}');
                    }
                    b"acd" => {
                        push_char('\u{223F}');
                    }
                    b"wreath" | b"VerticalTilde" | b"wr" => {
                        push_char('\u{2240}');
                    }
                    b"nsim" | b"NotTilde" => {
                        push_char('\u{2241}');
                    }
                    b"esim" | b"EqualTilde" | b"eqsim" => {
                        push_char('\u{2242}');
                    }
                    b"sime" | b"TildeEqual" | b"simeq" => {
                        push_char('\u{2243}');
                    }
                    b"nsime" | b"nsimeq" | b"NotTildeEqual" => {
                        push_char('\u{2244}');
                    }
                    b"cong" | b"TildeFullEqual" => {
                        push_char('\u{2245}');
                    }
                    b"simne" => {
                        push_char('\u{2246}');
                    }
                    b"ncong" | b"NotTildeFullEqual" => {
                        push_char('\u{2247}');
                    }
                    b"asymp" | b"ap" | b"TildeTilde" | b"approx" | b"thkap" | b"thickapprox" => {
                        push_char('\u{2248}');
                    }
                    b"nap" | b"NotTildeTilde" | b"napprox" => {
                        push_char('\u{2249}');
                    }
                    b"ape" | b"approxeq" => {
                        push_char('\u{224A}');
                    }
                    b"apid" => {
                        push_char('\u{224B}');
                    }
                    b"bcong" | b"backcong" => {
                        push_char('\u{224C}');
                    }
                    b"asympeq" | b"CupCap" => {
                        push_char('\u{224D}');
                    }
                    b"bump" | b"HumpDownHump" | b"Bumpeq" => {
                        push_char('\u{224E}');
                    }
                    b"bumpe" | b"HumpEqual" | b"bumpeq" => {
                        push_char('\u{224F}');
                    }
                    b"esdot" | b"DotEqual" | b"doteq" => {
                        push_char('\u{2250}');
                    }
                    b"eDot" | b"doteqdot" => {
                        push_char('\u{2251}');
                    }
                    b"efDot" | b"fallingdotseq" => {
                        push_char('\u{2252}');
                    }
                    b"erDot" | b"risingdotseq" => {
                        push_char('\u{2253}');
                    }
                    b"colone" | b"coloneq" | b"Assign" => {
                        push_char('\u{2254}');
                    }
                    b"ecolon" | b"eqcolon" => {
                        push_char('\u{2255}');
                    }
                    b"ecir" | b"eqcirc" => {
                        push_char('\u{2256}');
                    }
                    b"cire" | b"circeq" => {
                        push_char('\u{2257}');
                    }
                    b"wedgeq" => {
                        push_char('\u{2259}');
                    }
                    b"veeeq" => {
                        push_char('\u{225A}');
                    }
                    b"trie" | b"triangleq" => {
                        push_char('\u{225C}');
                    }
                    b"equest" | b"questeq" => {
                        push_char('\u{225F}');
                    }
                    b"ne" | b"NotEqual" => {
                        push_char('\u{2260}');
                    }
                    b"equiv" | b"Congruent" => {
                        push_char('\u{2261}');
                    }
                    b"nequiv" | b"NotCongruent" => {
                        push_char('\u{2262}');
                    }
                    b"le" | b"leq" => {
                        push_char('\u{2264}');
                    }
                    b"ge" | b"GreaterEqual" | b"geq" => {
                        push_char('\u{2265}');
                    }
                    b"lE" | b"LessFullEqual" | b"leqq" => {
                        push_char('\u{2266}');
                    }
                    b"gE" | b"GreaterFullEqual" | b"geqq" => {
                        push_char('\u{2267}');
                    }
                    b"lnE" | b"lneqq" => {
                        push_char('\u{2268}');
                    }
                    b"gnE" | b"gneqq" => {
                        push_char('\u{2269}');
                    }
                    b"Lt" | b"NestedLessLess" | b"ll" => {
                        push_char('\u{226A}');
                    }
                    b"Gt" | b"NestedGreaterGreater" | b"gg" => {
                        push_char('\u{226B}');
                    }
                    b"twixt" | b"between" => {
                        push_char('\u{226C}');
                    }
                    b"NotCupCap" => {
                        push_char('\u{226D}');
                    }
                    b"nlt" | b"NotLess" | b"nless" => {
                        push_char('\u{226E}');
                    }
                    b"ngt" | b"NotGreater" | b"ngtr" => {
                        push_char('\u{226F}');
                    }
                    b"nle" | b"NotLessEqual" | b"nleq" => {
                        push_char('\u{2270}');
                    }
                    b"nge" | b"NotGreaterEqual" | b"ngeq" => {
                        push_char('\u{2271}');
                    }
                    b"lsim" | b"LessTilde" | b"lesssim" => {
                        push_char('\u{2272}');
                    }
                    b"gsim" | b"gtrsim" | b"GreaterTilde" => {
                        push_char('\u{2273}');
                    }
                    b"nlsim" | b"NotLessTilde" => {
                        push_char('\u{2274}');
                    }
                    b"ngsim" | b"NotGreaterTilde" => {
                        push_char('\u{2275}');
                    }
                    b"lg" | b"lessgtr" | b"LessGreater" => {
                        push_char('\u{2276}');
                    }
                    b"gl" | b"gtrless" | b"GreaterLess" => {
                        push_char('\u{2277}');
                    }
                    b"ntlg" | b"NotLessGreater" => {
                        push_char('\u{2278}');
                    }
                    b"ntgl" | b"NotGreaterLess" => {
                        push_char('\u{2279}');
                    }
                    b"pr" | b"Precedes" | b"prec" => {
                        push_char('\u{227A}');
                    }
                    b"sc" | b"Succeeds" | b"succ" => {
                        push_char('\u{227B}');
                    }
                    b"prcue" | b"PrecedesSlantEqual" | b"preccurlyeq" => {
                        push_char('\u{227C}');
                    }
                    b"sccue" | b"SucceedsSlantEqual" | b"succcurlyeq" => {
                        push_char('\u{227D}');
                    }
                    b"prsim" | b"precsim" | b"PrecedesTilde" => {
                        push_char('\u{227E}');
                    }
                    b"scsim" | b"succsim" | b"SucceedsTilde" => {
                        push_char('\u{227F}');
                    }
                    b"npr" | b"nprec" | b"NotPrecedes" => {
                        push_char('\u{2280}');
                    }
                    b"nsc" | b"nsucc" | b"NotSucceeds" => {
                        push_char('\u{2281}');
                    }
                    b"sub" | b"subset" => {
                        push_char('\u{2282}');
                    }
                    b"sup" | b"supset" | b"Superset" => {
                        push_char('\u{2283}');
                    }
                    b"nsub" => {
                        push_char('\u{2284}');
                    }
                    b"nsup" => {
                        push_char('\u{2285}');
                    }
                    b"sube" | b"SubsetEqual" | b"subseteq" => {
                        push_char('\u{2286}');
                    }
                    b"supe" | b"supseteq" | b"SupersetEqual" => {
                        push_char('\u{2287}');
                    }
                    b"nsube" | b"nsubseteq" | b"NotSubsetEqual" => {
                        push_char('\u{2288}');
                    }
                    b"nsupe" | b"nsupseteq" | b"NotSupersetEqual" => {
                        push_char('\u{2289}');
                    }
                    b"subne" | b"subsetneq" => {
                        push_char('\u{228A}');
                    }
                    b"supne" | b"supsetneq" => {
                        push_char('\u{228B}');
                    }
                    b"cupdot" => {
                        push_char('\u{228D}');
                    }
                    b"uplus" | b"UnionPlus" => {
                        push_char('\u{228E}');
                    }
                    b"sqsub" | b"SquareSubset" | b"sqsubset" => {
                        push_char('\u{228F}');
                    }
                    b"sqsup" | b"SquareSuperset" | b"sqsupset" => {
                        push_char('\u{2290}');
                    }
                    b"sqsube" | b"SquareSubsetEqual" | b"sqsubseteq" => {
                        push_char('\u{2291}');
                    }
                    b"sqsupe" | b"SquareSupersetEqual" | b"sqsupseteq" => {
                        push_char('\u{2292}');
                    }
                    b"sqcap" | b"SquareIntersection" => {
                        push_char('\u{2293}');
                    }
                    b"sqcup" | b"SquareUnion" => {
                        push_char('\u{2294}');
                    }
                    b"oplus" | b"CirclePlus" => {
                        push_char('\u{2295}');
                    }
                    b"ominus" | b"CircleMinus" => {
                        push_char('\u{2296}');
                    }
                    b"otimes" | b"CircleTimes" => {
                        push_char('\u{2297}');
                    }
                    b"osol" => {
                        push_char('\u{2298}');
                    }
                    b"odot" | b"CircleDot" => {
                        push_char('\u{2299}');
                    }
                    b"ocir" | b"circledcirc" => {
                        push_char('\u{229A}');
                    }
                    b"oast" | b"circledast" => {
                        push_char('\u{229B}');
                    }
                    b"odash" | b"circleddash" => {
                        push_char('\u{229D}');
                    }
                    b"plusb" | b"boxplus" => {
                        push_char('\u{229E}');
                    }
                    b"minusb" | b"boxminus" => {
                        push_char('\u{229F}');
                    }
                    b"timesb" | b"boxtimes" => {
                        push_char('\u{22A0}');
                    }
                    b"sdotb" | b"dotsquare" => {
                        push_char('\u{22A1}');
                    }
                    b"vdash" | b"RightTee" => {
                        push_char('\u{22A2}');
                    }
                    b"dashv" | b"LeftTee" => {
                        push_char('\u{22A3}');
                    }
                    b"top" | b"DownTee" => {
                        push_char('\u{22A4}');
                    }
                    b"bottom" | b"bot" | b"perp" | b"UpTee" => {
                        push_char('\u{22A5}');
                    }
                    b"models" => {
                        push_char('\u{22A7}');
                    }
                    b"vDash" | b"DoubleRightTee" => {
                        push_char('\u{22A8}');
                    }
                    b"Vdash" => {
                        push_char('\u{22A9}');
                    }
                    b"Vvdash" => {
                        push_char('\u{22AA}');
                    }
                    b"VDash" => {
                        push_char('\u{22AB}');
                    }
                    b"nvdash" => {
                        push_char('\u{22AC}');
                    }
                    b"nvDash" => {
                        push_char('\u{22AD}');
                    }
                    b"nVdash" => {
                        push_char('\u{22AE}');
                    }
                    b"nVDash" => {
                        push_char('\u{22AF}');
                    }
                    b"prurel" => {
                        push_char('\u{22B0}');
                    }
                    b"vltri" | b"vartriangleleft" | b"LeftTriangle" => {
                        push_char('\u{22B2}');
                    }
                    b"vrtri" | b"vartriangleright" | b"RightTriangle" => {
                        push_char('\u{22B3}');
                    }
                    b"ltrie" | b"trianglelefteq" | b"LeftTriangleEqual" => {
                        push_char('\u{22B4}');
                    }
                    b"rtrie" | b"trianglerighteq" | b"RightTriangleEqual" => {
                        push_char('\u{22B5}');
                    }
                    b"origof" => {
                        push_char('\u{22B6}');
                    }
                    b"imof" => {
                        push_char('\u{22B7}');
                    }
                    b"mumap" | b"multimap" => {
                        push_char('\u{22B8}');
                    }
                    b"hercon" => {
                        push_char('\u{22B9}');
                    }
                    b"intcal" | b"intercal" => {
                        push_char('\u{22BA}');
                    }
                    b"veebar" => {
                        push_char('\u{22BB}');
                    }
                    b"barvee" => {
                        push_char('\u{22BD}');
                    }
                    b"angrtvb" => {
                        push_char('\u{22BE}');
                    }
                    b"lrtri" => {
                        push_char('\u{22BF}');
                    }
                    b"xwedge" | b"Wedge" | b"bigwedge" => {
                        push_char('\u{22C0}');
                    }
                    b"xvee" | b"Vee" | b"bigvee" => {
                        push_char('\u{22C1}');
                    }
                    b"xcap" | b"Intersection" | b"bigcap" => {
                        push_char('\u{22C2}');
                    }
                    b"xcup" | b"Union" | b"bigcup" => {
                        push_char('\u{22C3}');
                    }
                    b"diam" | b"diamond" | b"Diamond" => {
                        push_char('\u{22C4}');
                    }
                    b"sdot" => {
                        push_char('\u{22C5}');
                    }
                    b"sstarf" | b"Star" => {
                        push_char('\u{22C6}');
                    }
                    b"divonx" | b"divideontimes" => {
                        push_char('\u{22C7}');
                    }
                    b"bowtie" => {
                        push_char('\u{22C8}');
                    }
                    b"ltimes" => {
                        push_char('\u{22C9}');
                    }
                    b"rtimes" => {
                        push_char('\u{22CA}');
                    }
                    b"lthree" | b"leftthreetimes" => {
                        push_char('\u{22CB}');
                    }
                    b"rthree" | b"rightthreetimes" => {
                        push_char('\u{22CC}');
                    }
                    b"bsime" | b"backsimeq" => {
                        push_char('\u{22CD}');
                    }
                    b"cuvee" | b"curlyvee" => {
                        push_char('\u{22CE}');
                    }
                    b"cuwed" | b"curlywedge" => {
                        push_char('\u{22CF}');
                    }
                    b"Sub" | b"Subset" => {
                        push_char('\u{22D0}');
                    }
                    b"Sup" | b"Supset" => {
                        push_char('\u{22D1}');
                    }
                    b"Cap" => {
                        push_char('\u{22D2}');
                    }
                    b"Cup" => {
                        push_char('\u{22D3}');
                    }
                    b"fork" | b"pitchfork" => {
                        push_char('\u{22D4}');
                    }
                    b"epar" => {
                        push_char('\u{22D5}');
                    }
                    b"ltdot" | b"lessdot" => {
                        push_char('\u{22D6}');
                    }
                    b"gtdot" | b"gtrdot" => {
                        push_char('\u{22D7}');
                    }
                    b"Ll" => {
                        push_char('\u{22D8}');
                    }
                    b"Gg" | b"ggg" => {
                        push_char('\u{22D9}');
                    }
                    b"leg" | b"LessEqualGreater" | b"lesseqgtr" => {
                        push_char('\u{22DA}');
                    }
                    b"gel" | b"gtreqless" | b"GreaterEqualLess" => {
                        push_char('\u{22DB}');
                    }
                    b"cuepr" | b"curlyeqprec" => {
                        push_char('\u{22DE}');
                    }
                    b"cuesc" | b"curlyeqsucc" => {
                        push_char('\u{22DF}');
                    }
                    b"nprcue" | b"NotPrecedesSlantEqual" => {
                        push_char('\u{22E0}');
                    }
                    b"nsccue" | b"NotSucceedsSlantEqual" => {
                        push_char('\u{22E1}');
                    }
                    b"nsqsube" | b"NotSquareSubsetEqual" => {
                        push_char('\u{22E2}');
                    }
                    b"nsqsupe" | b"NotSquareSupersetEqual" => {
                        push_char('\u{22E3}');
                    }
                    b"lnsim" => {
                        push_char('\u{22E6}');
                    }
                    b"gnsim" => {
                        push_char('\u{22E7}');
                    }
                    b"prnsim" | b"precnsim" => {
                        push_char('\u{22E8}');
                    }
                    b"scnsim" | b"succnsim" => {
                        push_char('\u{22E9}');
                    }
                    b"nltri" | b"ntriangleleft" | b"NotLeftTriangle" => {
                        push_char('\u{22EA}');
                    }
                    b"nrtri" | b"ntriangleright" | b"NotRightTriangle" => {
                        push_char('\u{22EB}');
                    }
                    b"nltrie" | b"ntrianglelefteq" | b"NotLeftTriangleEqual" => {
                        push_char('\u{22EC}');
                    }
                    b"nrtrie" | b"ntrianglerighteq" | b"NotRightTriangleEqual" => {
                        push_char('\u{22ED}');
                    }
                    b"vellip" => {
                        push_char('\u{22EE}');
                    }
                    b"ctdot" => {
                        push_char('\u{22EF}');
                    }
                    b"utdot" => {
                        push_char('\u{22F0}');
                    }
                    b"dtdot" => {
                        push_char('\u{22F1}');
                    }
                    b"disin" => {
                        push_char('\u{22F2}');
                    }
                    b"isinsv" => {
                        push_char('\u{22F3}');
                    }
                    b"isins" => {
                        push_char('\u{22F4}');
                    }
                    b"isindot" => {
                        push_char('\u{22F5}');
                    }
                    b"notinvc" => {
                        push_char('\u{22F6}');
                    }
                    b"notinvb" => {
                        push_char('\u{22F7}');
                    }
                    b"isinE" => {
                        push_char('\u{22F9}');
                    }
                    b"nisd" => {
                        push_char('\u{22FA}');
                    }
                    b"xnis" => {
                        push_char('\u{22FB}');
                    }
                    b"nis" => {
                        push_char('\u{22FC}');
                    }
                    b"notnivc" => {
                        push_char('\u{22FD}');
                    }
                    b"notnivb" => {
                        push_char('\u{22FE}');
                    }
                    b"barwed" | b"barwedge" => {
                        push_char('\u{2305}');
                    }
                    b"Barwed" | b"doublebarwedge" => {
                        push_char('\u{2306}');
                    }
                    b"lceil" | b"LeftCeiling" => {
                        push_char('\u{2308}');
                    }
                    b"rceil" | b"RightCeiling" => {
                        push_char('\u{2309}');
                    }
                    b"lfloor" | b"LeftFloor" => {
                        push_char('\u{230A}');
                    }
                    b"rfloor" | b"RightFloor" => {
                        push_char('\u{230B}');
                    }
                    b"drcrop" => {
                        push_char('\u{230C}');
                    }
                    b"dlcrop" => {
                        push_char('\u{230D}');
                    }
                    b"urcrop" => {
                        push_char('\u{230E}');
                    }
                    b"ulcrop" => {
                        push_char('\u{230F}');
                    }
                    b"bnot" => {
                        push_char('\u{2310}');
                    }
                    b"profline" => {
                        push_char('\u{2312}');
                    }
                    b"profsurf" => {
                        push_char('\u{2313}');
                    }
                    b"telrec" => {
                        push_char('\u{2315}');
                    }
                    b"target" => {
                        push_char('\u{2316}');
                    }
                    b"ulcorn" | b"ulcorner" => {
                        push_char('\u{231C}');
                    }
                    b"urcorn" | b"urcorner" => {
                        push_char('\u{231D}');
                    }
                    b"dlcorn" | b"llcorner" => {
                        push_char('\u{231E}');
                    }
                    b"drcorn" | b"lrcorner" => {
                        push_char('\u{231F}');
                    }
                    b"frown" | b"sfrown" => {
                        push_char('\u{2322}');
                    }
                    b"smile" | b"ssmile" => {
                        push_char('\u{2323}');
                    }
                    b"cylcty" => {
                        push_char('\u{232D}');
                    }
                    b"profalar" => {
                        push_char('\u{232E}');
                    }
                    b"topbot" => {
                        push_char('\u{2336}');
                    }
                    b"ovbar" => {
                        push_char('\u{233D}');
                    }
                    b"solbar" => {
                        push_char('\u{233F}');
                    }
                    b"angzarr" => {
                        push_char('\u{237C}');
                    }
                    b"lmoust" | b"lmoustache" => {
                        push_char('\u{23B0}');
                    }
                    b"rmoust" | b"rmoustache" => {
                        push_char('\u{23B1}');
                    }
                    b"tbrk" | b"OverBracket" => {
                        push_char('\u{23B4}');
                    }
                    b"bbrk" | b"UnderBracket" => {
                        push_char('\u{23B5}');
                    }
                    b"bbrktbrk" => {
                        push_char('\u{23B6}');
                    }
                    b"OverParenthesis" => {
                        push_char('\u{23DC}');
                    }
                    b"UnderParenthesis" => {
                        push_char('\u{23DD}');
                    }
                    b"OverBrace" => {
                        push_char('\u{23DE}');
                    }
                    b"UnderBrace" => {
                        push_char('\u{23DF}');
                    }
                    b"trpezium" => {
                        push_char('\u{23E2}');
                    }
                    b"elinters" => {
                        push_char('\u{23E7}');
                    }
                    b"blank" => {
                        push_char('\u{2423}');
                    }
                    b"oS" | b"circledS" => {
                        push_char('\u{24C8}');
                    }
                    b"boxh" | b"HorizontalLine" => {
                        push_char('\u{2500}');
                    }
                    b"boxv" => {
                        push_char('\u{2502}');
                    }
                    b"boxdr" => {
                        push_char('\u{250C}');
                    }
                    b"boxdl" => {
                        push_char('\u{2510}');
                    }
                    b"boxur" => {
                        push_char('\u{2514}');
                    }
                    b"boxul" => {
                        push_char('\u{2518}');
                    }
                    b"boxvr" => {
                        push_char('\u{251C}');
                    }
                    b"boxvl" => {
                        push_char('\u{2524}');
                    }
                    b"boxhd" => {
                        push_char('\u{252C}');
                    }
                    b"boxhu" => {
                        push_char('\u{2534}');
                    }
                    b"boxvh" => {
                        push_char('\u{253C}');
                    }
                    b"boxH" => {
                        push_char('\u{2550}');
                    }
                    b"boxV" => {
                        push_char('\u{2551}');
                    }
                    b"boxdR" => {
                        push_char('\u{2552}');
                    }
                    b"boxDr" => {
                        push_char('\u{2553}');
                    }
                    b"boxDR" => {
                        push_char('\u{2554}');
                    }
                    b"boxdL" => {
                        push_char('\u{2555}');
                    }
                    b"boxDl" => {
                        push_char('\u{2556}');
                    }
                    b"boxDL" => {
                        push_char('\u{2557}');
                    }
                    b"boxuR" => {
                        push_char('\u{2558}');
                    }
                    b"boxUr" => {
                        push_char('\u{2559}');
                    }
                    b"boxUR" => {
                        push_char('\u{255A}');
                    }
                    b"boxuL" => {
                        push_char('\u{255B}');
                    }
                    b"boxUl" => {
                        push_char('\u{255C}');
                    }
                    b"boxUL" => {
                        push_char('\u{255D}');
                    }
                    b"boxvR" => {
                        push_char('\u{255E}');
                    }
                    b"boxVr" => {
                        push_char('\u{255F}');
                    }
                    b"boxVR" => {
                        push_char('\u{2560}');
                    }
                    b"boxvL" => {
                        push_char('\u{2561}');
                    }
                    b"boxVl" => {
                        push_char('\u{2562}');
                    }
                    b"boxVL" => {
                        push_char('\u{2563}');
                    }
                    b"boxHd" => {
                        push_char('\u{2564}');
                    }
                    b"boxhD" => {
                        push_char('\u{2565}');
                    }
                    b"boxHD" => {
                        push_char('\u{2566}');
                    }
                    b"boxHu" => {
                        push_char('\u{2567}');
                    }
                    b"boxhU" => {
                        push_char('\u{2568}');
                    }
                    b"boxHU" => {
                        push_char('\u{2569}');
                    }
                    b"boxvH" => {
                        push_char('\u{256A}');
                    }
                    b"boxVh" => {
                        push_char('\u{256B}');
                    }
                    b"boxVH" => {
                        push_char('\u{256C}');
                    }
                    b"uhblk" => {
                        push_char('\u{2580}');
                    }
                    b"lhblk" => {
                        push_char('\u{2584}');
                    }
                    b"block" => {
                        push_char('\u{2588}');
                    }
                    b"blk14" => {
                        push_char('\u{2591}');
                    }
                    b"blk12" => {
                        push_char('\u{2592}');
                    }
                    b"blk34" => {
                        push_char('\u{2593}');
                    }
                    b"squ" | b"square" | b"Square" => {
                        push_char('\u{25A1}');
                    }
                    b"squf" | b"squarf" | b"blacksquare" | b"FilledVerySmallSquare" => {
                        push_char('\u{25AA}');
                    }
                    b"EmptyVerySmallSquare" => {
                        push_char('\u{25AB}');
                    }
                    b"rect" => {
                        push_char('\u{25AD}');
                    }
                    b"marker" => {
                        push_char('\u{25AE}');
                    }
                    b"fltns" => {
                        push_char('\u{25B1}');
                    }
                    b"xutri" | b"bigtriangleup" => {
                        push_char('\u{25B3}');
                    }
                    b"utrif" | b"blacktriangle" => {
                        push_char('\u{25B4}');
                    }
                    b"utri" | b"triangle" => {
                        push_char('\u{25B5}');
                    }
                    b"rtrif" | b"blacktriangleright" => {
                        push_char('\u{25B8}');
                    }
                    b"rtri" | b"triangleright" => {
                        push_char('\u{25B9}');
                    }
                    b"xdtri" | b"bigtriangledown" => {
                        push_char('\u{25BD}');
                    }
                    b"dtrif" | b"blacktriangledown" => {
                        push_char('\u{25BE}');
                    }
                    b"dtri" | b"triangledown" => {
                        push_char('\u{25BF}');
                    }
                    b"ltrif" | b"blacktriangleleft" => {
                        push_char('\u{25C2}');
                    }
                    b"ltri" | b"triangleleft" => {
                        push_char('\u{25C3}');
                    }
                    b"loz" | b"lozenge" => {
                        push_char('\u{25CA}');
                    }
                    b"cir" => {
                        push_char('\u{25CB}');
                    }
                    b"tridot" => {
                        push_char('\u{25EC}');
                    }
                    b"xcirc" | b"bigcirc" => {
                        push_char('\u{25EF}');
                    }
                    b"ultri" => {
                        push_char('\u{25F8}');
                    }
                    b"urtri" => {
                        push_char('\u{25F9}');
                    }
                    b"lltri" => {
                        push_char('\u{25FA}');
                    }
                    b"EmptySmallSquare" => {
                        push_char('\u{25FB}');
                    }
                    b"FilledSmallSquare" => {
                        push_char('\u{25FC}');
                    }
                    b"starf" | b"bigstar" => {
                        push_char('\u{2605}');
                    }
                    b"star" => {
                        push_char('\u{2606}');
                    }
                    b"phone" => {
                        push_char('\u{260E}');
                    }
                    b"female" => {
                        push_char('\u{2640}');
                    }
                    b"male" => {
                        push_char('\u{2642}');
                    }
                    b"spades" | b"spadesuit" => {
                        push_char('\u{2660}');
                    }
                    b"clubs" | b"clubsuit" => {
                        push_char('\u{2663}');
                    }
                    b"hearts" | b"heartsuit" => {
                        push_char('\u{2665}');
                    }
                    b"diams" | b"diamondsuit" => {
                        push_char('\u{2666}');
                    }
                    b"sung" => {
                        push_char('\u{266A}');
                    }
                    b"flat" => {
                        push_char('\u{266D}');
                    }
                    b"natur" | b"natural" => {
                        push_char('\u{266E}');
                    }
                    b"sharp" => {
                        push_char('\u{266F}');
                    }
                    b"check" | b"checkmark" => {
                        push_char('\u{2713}');
                    }
                    b"cross" => {
                        push_char('\u{2717}');
                    }
                    b"malt" | b"maltese" => {
                        push_char('\u{2720}');
                    }
                    b"sext" => {
                        push_char('\u{2736}');
                    }
                    b"VerticalSeparator" => {
                        push_char('\u{2758}');
                    }
                    b"lbbrk" => {
                        push_char('\u{2772}');
                    }
                    b"rbbrk" => {
                        push_char('\u{2773}');
                    }
                    b"lobrk" | b"LeftDoubleBracket" => {
                        push_char('\u{27E6}');
                    }
                    b"robrk" | b"RightDoubleBracket" => {
                        push_char('\u{27E7}');
                    }
                    b"lang" | b"LeftAngleBracket" | b"langle" => {
                        push_char('\u{27E8}');
                    }
                    b"rang" | b"RightAngleBracket" | b"rangle" => {
                        push_char('\u{27E9}');
                    }
                    b"Lang" => {
                        push_char('\u{27EA}');
                    }
                    b"Rang" => {
                        push_char('\u{27EB}');
                    }
                    b"loang" => {
                        push_char('\u{27EC}');
                    }
                    b"roang" => {
                        push_char('\u{27ED}');
                    }
                    b"xlarr" | b"longleftarrow" | b"LongLeftArrow" => {
                        push_char('\u{27F5}');
                    }
                    b"xrarr" | b"longrightarrow" | b"LongRightArrow" => {
                        push_char('\u{27F6}');
                    }
                    b"xharr" | b"longleftrightarrow" | b"LongLeftRightArrow" => {
                        push_char('\u{27F7}');
                    }
                    b"xlArr" | b"Longleftarrow" | b"DoubleLongLeftArrow" => {
                        push_char('\u{27F8}');
                    }
                    b"xrArr" | b"Longrightarrow" | b"DoubleLongRightArrow" => {
                        push_char('\u{27F9}');
                    }
                    b"xhArr" | b"Longleftrightarrow" | b"DoubleLongLeftRightArrow" => {
                        push_char('\u{27FA}');
                    }
                    b"xmap" | b"longmapsto" => {
                        push_char('\u{27FC}');
                    }
                    b"dzigrarr" => {
                        push_char('\u{27FF}');
                    }
                    b"nvlArr" => {
                        push_char('\u{2902}');
                    }
                    b"nvrArr" => {
                        push_char('\u{2903}');
                    }
                    b"nvHarr" => {
                        push_char('\u{2904}');
                    }
                    b"Map" => {
                        push_char('\u{2905}');
                    }
                    b"lbarr" => {
                        push_char('\u{290C}');
                    }
                    b"rbarr" | b"bkarow" => {
                        push_char('\u{290D}');
                    }
                    b"lBarr" => {
                        push_char('\u{290E}');
                    }
                    b"rBarr" | b"dbkarow" => {
                        push_char('\u{290F}');
                    }
                    b"RBarr" | b"drbkarow" => {
                        push_char('\u{2910}');
                    }
                    b"DDotrahd" => {
                        push_char('\u{2911}');
                    }
                    b"UpArrowBar" => {
                        push_char('\u{2912}');
                    }
                    b"DownArrowBar" => {
                        push_char('\u{2913}');
                    }
                    b"Rarrtl" => {
                        push_char('\u{2916}');
                    }
                    b"latail" => {
                        push_char('\u{2919}');
                    }
                    b"ratail" => {
                        push_char('\u{291A}');
                    }
                    b"lAtail" => {
                        push_char('\u{291B}');
                    }
                    b"rAtail" => {
                        push_char('\u{291C}');
                    }
                    b"larrfs" => {
                        push_char('\u{291D}');
                    }
                    b"rarrfs" => {
                        push_char('\u{291E}');
                    }
                    b"larrbfs" => {
                        push_char('\u{291F}');
                    }
                    b"rarrbfs" => {
                        push_char('\u{2920}');
                    }
                    b"nwarhk" => {
                        push_char('\u{2923}');
                    }
                    b"nearhk" => {
                        push_char('\u{2924}');
                    }
                    b"searhk" | b"hksearow" => {
                        push_char('\u{2925}');
                    }
                    b"swarhk" | b"hkswarow" => {
                        push_char('\u{2926}');
                    }
                    b"nwnear" => {
                        push_char('\u{2927}');
                    }
                    b"nesear" | b"toea" => {
                        push_char('\u{2928}');
                    }
                    b"seswar" | b"tosa" => {
                        push_char('\u{2929}');
                    }
                    b"swnwar" => {
                        push_char('\u{292A}');
                    }
                    b"rarrc" => {
                        push_char('\u{2933}');
                    }
                    b"cudarrr" => {
                        push_char('\u{2935}');
                    }
                    b"ldca" => {
                        push_char('\u{2936}');
                    }
                    b"rdca" => {
                        push_char('\u{2937}');
                    }
                    b"cudarrl" => {
                        push_char('\u{2938}');
                    }
                    b"larrpl" => {
                        push_char('\u{2939}');
                    }
                    b"curarrm" => {
                        push_char('\u{293C}');
                    }
                    b"cularrp" => {
                        push_char('\u{293D}');
                    }
                    b"rarrpl" => {
                        push_char('\u{2945}');
                    }
                    b"harrcir" => {
                        push_char('\u{2948}');
                    }
                    b"Uarrocir" => {
                        push_char('\u{2949}');
                    }
                    b"lurdshar" => {
                        push_char('\u{294A}');
                    }
                    b"ldrushar" => {
                        push_char('\u{294B}');
                    }
                    b"LeftRightVector" => {
                        push_char('\u{294E}');
                    }
                    b"RightUpDownVector" => {
                        push_char('\u{294F}');
                    }
                    b"DownLeftRightVector" => {
                        push_char('\u{2950}');
                    }
                    b"LeftUpDownVector" => {
                        push_char('\u{2951}');
                    }
                    b"LeftVectorBar" => {
                        push_char('\u{2952}');
                    }
                    b"RightVectorBar" => {
                        push_char('\u{2953}');
                    }
                    b"RightUpVectorBar" => {
                        push_char('\u{2954}');
                    }
                    b"RightDownVectorBar" => {
                        push_char('\u{2955}');
                    }
                    b"DownLeftVectorBar" => {
                        push_char('\u{2956}');
                    }
                    b"DownRightVectorBar" => {
                        push_char('\u{2957}');
                    }
                    b"LeftUpVectorBar" => {
                        push_char('\u{2958}');
                    }
                    b"LeftDownVectorBar" => {
                        push_char('\u{2959}');
                    }
                    b"LeftTeeVector" => {
                        push_char('\u{295A}');
                    }
                    b"RightTeeVector" => {
                        push_char('\u{295B}');
                    }
                    b"RightUpTeeVector" => {
                        push_char('\u{295C}');
                    }
                    b"RightDownTeeVector" => {
                        push_char('\u{295D}');
                    }
                    b"DownLeftTeeVector" => {
                        push_char('\u{295E}');
                    }
                    b"DownRightTeeVector" => {
                        push_char('\u{295F}');
                    }
                    b"LeftUpTeeVector" => {
                        push_char('\u{2960}');
                    }
                    b"LeftDownTeeVector" => {
                        push_char('\u{2961}');
                    }
                    b"lHar" => {
                        push_char('\u{2962}');
                    }
                    b"uHar" => {
                        push_char('\u{2963}');
                    }
                    b"rHar" => {
                        push_char('\u{2964}');
                    }
                    b"dHar" => {
                        push_char('\u{2965}');
                    }
                    b"luruhar" => {
                        push_char('\u{2966}');
                    }
                    b"ldrdhar" => {
                        push_char('\u{2967}');
                    }
                    b"ruluhar" => {
                        push_char('\u{2968}');
                    }
                    b"rdldhar" => {
                        push_char('\u{2969}');
                    }
                    b"lharul" => {
                        push_char('\u{296A}');
                    }
                    b"llhard" => {
                        push_char('\u{296B}');
                    }
                    b"rharul" => {
                        push_char('\u{296C}');
                    }
                    b"lrhard" => {
                        push_char('\u{296D}');
                    }
                    b"udhar" | b"UpEquilibrium" => {
                        push_char('\u{296E}');
                    }
                    b"duhar" | b"ReverseUpEquilibrium" => {
                        push_char('\u{296F}');
                    }
                    b"RoundImplies" => {
                        push_char('\u{2970}');
                    }
                    b"erarr" => {
                        push_char('\u{2971}');
                    }
                    b"simrarr" => {
                        push_char('\u{2972}');
                    }
                    b"larrsim" => {
                        push_char('\u{2973}');
                    }
                    b"rarrsim" => {
                        push_char('\u{2974}');
                    }
                    b"rarrap" => {
                        push_char('\u{2975}');
                    }
                    b"ltlarr" => {
                        push_char('\u{2976}');
                    }
                    b"gtrarr" => {
                        push_char('\u{2978}');
                    }
                    b"subrarr" => {
                        push_char('\u{2979}');
                    }
                    b"suplarr" => {
                        push_char('\u{297B}');
                    }
                    b"lfisht" => {
                        push_char('\u{297C}');
                    }
                    b"rfisht" => {
                        push_char('\u{297D}');
                    }
                    b"ufisht" => {
                        push_char('\u{297E}');
                    }
                    b"dfisht" => {
                        push_char('\u{297F}');
                    }
                    b"lopar" => {
                        push_char('\u{2985}');
                    }
                    b"ropar" => {
                        push_char('\u{2986}');
                    }
                    b"lbrke" => {
                        push_char('\u{298B}');
                    }
                    b"rbrke" => {
                        push_char('\u{298C}');
                    }
                    b"lbrkslu" => {
                        push_char('\u{298D}');
                    }
                    b"rbrksld" => {
                        push_char('\u{298E}');
                    }
                    b"lbrksld" => {
                        push_char('\u{298F}');
                    }
                    b"rbrkslu" => {
                        push_char('\u{2990}');
                    }
                    b"langd" => {
                        push_char('\u{2991}');
                    }
                    b"rangd" => {
                        push_char('\u{2992}');
                    }
                    b"lparlt" => {
                        push_char('\u{2993}');
                    }
                    b"rpargt" => {
                        push_char('\u{2994}');
                    }
                    b"gtlPar" => {
                        push_char('\u{2995}');
                    }
                    b"ltrPar" => {
                        push_char('\u{2996}');
                    }
                    b"vzigzag" => {
                        push_char('\u{299A}');
                    }
                    b"vangrt" => {
                        push_char('\u{299C}');
                    }
                    b"angrtvbd" => {
                        push_char('\u{299D}');
                    }
                    b"ange" => {
                        push_char('\u{29A4}');
                    }
                    b"range" => {
                        push_char('\u{29A5}');
                    }
                    b"dwangle" => {
                        push_char('\u{29A6}');
                    }
                    b"uwangle" => {
                        push_char('\u{29A7}');
                    }
                    b"angmsdaa" => {
                        push_char('\u{29A8}');
                    }
                    b"angmsdab" => {
                        push_char('\u{29A9}');
                    }
                    b"angmsdac" => {
                        push_char('\u{29AA}');
                    }
                    b"angmsdad" => {
                        push_char('\u{29AB}');
                    }
                    b"angmsdae" => {
                        push_char('\u{29AC}');
                    }
                    b"angmsdaf" => {
                        push_char('\u{29AD}');
                    }
                    b"angmsdag" => {
                        push_char('\u{29AE}');
                    }
                    b"angmsdah" => {
                        push_char('\u{29AF}');
                    }
                    b"bemptyv" => {
                        push_char('\u{29B0}');
                    }
                    b"demptyv" => {
                        push_char('\u{29B1}');
                    }
                    b"cemptyv" => {
                        push_char('\u{29B2}');
                    }
                    b"raemptyv" => {
                        push_char('\u{29B3}');
                    }
                    b"laemptyv" => {
                        push_char('\u{29B4}');
                    }
                    b"ohbar" => {
                        push_char('\u{29B5}');
                    }
                    b"omid" => {
                        push_char('\u{29B6}');
                    }
                    b"opar" => {
                        push_char('\u{29B7}');
                    }
                    b"operp" => {
                        push_char('\u{29B9}');
                    }
                    b"olcross" => {
                        push_char('\u{29BB}');
                    }
                    b"odsold" => {
                        push_char('\u{29BC}');
                    }
                    b"olcir" => {
                        push_char('\u{29BE}');
                    }
                    b"ofcir" => {
                        push_char('\u{29BF}');
                    }
                    b"olt" => {
                        push_char('\u{29C0}');
                    }
                    b"ogt" => {
                        push_char('\u{29C1}');
                    }
                    b"cirscir" => {
                        push_char('\u{29C2}');
                    }
                    b"cirE" => {
                        push_char('\u{29C3}');
                    }
                    b"solb" => {
                        push_char('\u{29C4}');
                    }
                    b"bsolb" => {
                        push_char('\u{29C5}');
                    }
                    b"boxbox" => {
                        push_char('\u{29C9}');
                    }
                    b"trisb" => {
                        push_char('\u{29CD}');
                    }
                    b"rtriltri" => {
                        push_char('\u{29CE}');
                    }
                    b"LeftTriangleBar" => {
                        push_char('\u{29CF}');
                    }
                    b"RightTriangleBar" => {
                        push_char('\u{29D0}');
                    }
                    b"race" => {
                        push_char('\u{29DA}');
                    }
                    b"iinfin" => {
                        push_char('\u{29DC}');
                    }
                    b"infintie" => {
                        push_char('\u{29DD}');
                    }
                    b"nvinfin" => {
                        push_char('\u{29DE}');
                    }
                    b"eparsl" => {
                        push_char('\u{29E3}');
                    }
                    b"smeparsl" => {
                        push_char('\u{29E4}');
                    }
                    b"eqvparsl" => {
                        push_char('\u{29E5}');
                    }
                    b"lozf" | b"blacklozenge" => {
                        push_char('\u{29EB}');
                    }
                    b"RuleDelayed" => {
                        push_char('\u{29F4}');
                    }
                    b"dsol" => {
                        push_char('\u{29F6}');
                    }
                    b"xodot" | b"bigodot" => {
                        push_char('\u{2A00}');
                    }
                    b"xoplus" | b"bigoplus" => {
                        push_char('\u{2A01}');
                    }
                    b"xotime" | b"bigotimes" => {
                        push_char('\u{2A02}');
                    }
                    b"xuplus" | b"biguplus" => {
                        push_char('\u{2A04}');
                    }
                    b"xsqcup" | b"bigsqcup" => {
                        push_char('\u{2A06}');
                    }
                    b"qint" | b"iiiint" => {
                        push_char('\u{2A0C}');
                    }
                    b"fpartint" => {
                        push_char('\u{2A0D}');
                    }
                    b"cirfnint" => {
                        push_char('\u{2A10}');
                    }
                    b"awint" => {
                        push_char('\u{2A11}');
                    }
                    b"rppolint" => {
                        push_char('\u{2A12}');
                    }
                    b"scpolint" => {
                        push_char('\u{2A13}');
                    }
                    b"npolint" => {
                        push_char('\u{2A14}');
                    }
                    b"pointint" => {
                        push_char('\u{2A15}');
                    }
                    b"quatint" => {
                        push_char('\u{2A16}');
                    }
                    b"intlarhk" => {
                        push_char('\u{2A17}');
                    }
                    b"pluscir" => {
                        push_char('\u{2A22}');
                    }
                    b"plusacir" => {
                        push_char('\u{2A23}');
                    }
                    b"simplus" => {
                        push_char('\u{2A24}');
                    }
                    b"plusdu" => {
                        push_char('\u{2A25}');
                    }
                    b"plussim" => {
                        push_char('\u{2A26}');
                    }
                    b"plustwo" => {
                        push_char('\u{2A27}');
                    }
                    b"mcomma" => {
                        push_char('\u{2A29}');
                    }
                    b"minusdu" => {
                        push_char('\u{2A2A}');
                    }
                    b"loplus" => {
                        push_char('\u{2A2D}');
                    }
                    b"roplus" => {
                        push_char('\u{2A2E}');
                    }
                    b"Cross" => {
                        push_char('\u{2A2F}');
                    }
                    b"timesd" => {
                        push_char('\u{2A30}');
                    }
                    b"timesbar" => {
                        push_char('\u{2A31}');
                    }
                    b"smashp" => {
                        push_char('\u{2A33}');
                    }
                    b"lotimes" => {
                        push_char('\u{2A34}');
                    }
                    b"rotimes" => {
                        push_char('\u{2A35}');
                    }
                    b"otimesas" => {
                        push_char('\u{2A36}');
                    }
                    b"Otimes" => {
                        push_char('\u{2A37}');
                    }
                    b"odiv" => {
                        push_char('\u{2A38}');
                    }
                    b"triplus" => {
                        push_char('\u{2A39}');
                    }
                    b"triminus" => {
                        push_char('\u{2A3A}');
                    }
                    b"tritime" => {
                        push_char('\u{2A3B}');
                    }
                    b"iprod" | b"intprod" => {
                        push_char('\u{2A3C}');
                    }
                    b"amalg" => {
                        push_char('\u{2A3F}');
                    }
                    b"capdot" => {
                        push_char('\u{2A40}');
                    }
                    b"ncup" => {
                        push_char('\u{2A42}');
                    }
                    b"ncap" => {
                        push_char('\u{2A43}');
                    }
                    b"capand" => {
                        push_char('\u{2A44}');
                    }
                    b"cupor" => {
                        push_char('\u{2A45}');
                    }
                    b"cupcap" => {
                        push_char('\u{2A46}');
                    }
                    b"capcup" => {
                        push_char('\u{2A47}');
                    }
                    b"cupbrcap" => {
                        push_char('\u{2A48}');
                    }
                    b"capbrcup" => {
                        push_char('\u{2A49}');
                    }
                    b"cupcup" => {
                        push_char('\u{2A4A}');
                    }
                    b"capcap" => {
                        push_char('\u{2A4B}');
                    }
                    b"ccups" => {
                        push_char('\u{2A4C}');
                    }
                    b"ccaps" => {
                        push_char('\u{2A4D}');
                    }
                    b"ccupssm" => {
                        push_char('\u{2A50}');
                    }
                    b"And" => {
                        push_char('\u{2A53}');
                    }
                    b"Or" => {
                        push_char('\u{2A54}');
                    }
                    b"andand" => {
                        push_char('\u{2A55}');
                    }
                    b"oror" => {
                        push_char('\u{2A56}');
                    }
                    b"orslope" => {
                        push_char('\u{2A57}');
                    }
                    b"andslope" => {
                        push_char('\u{2A58}');
                    }
                    b"andv" => {
                        push_char('\u{2A5A}');
                    }
                    b"orv" => {
                        push_char('\u{2A5B}');
                    }
                    b"andd" => {
                        push_char('\u{2A5C}');
                    }
                    b"ord" => {
                        push_char('\u{2A5D}');
                    }
                    b"wedbar" => {
                        push_char('\u{2A5F}');
                    }
                    b"sdote" => {
                        push_char('\u{2A66}');
                    }
                    b"simdot" => {
                        push_char('\u{2A6A}');
                    }
                    b"congdot" => {
                        push_char('\u{2A6D}');
                    }
                    b"easter" => {
                        push_char('\u{2A6E}');
                    }
                    b"apacir" => {
                        push_char('\u{2A6F}');
                    }
                    b"apE" => {
                        push_char('\u{2A70}');
                    }
                    b"eplus" => {
                        push_char('\u{2A71}');
                    }
                    b"pluse" => {
                        push_char('\u{2A72}');
                    }
                    b"Esim" => {
                        push_char('\u{2A73}');
                    }
                    b"Colone" => {
                        push_char('\u{2A74}');
                    }
                    b"Equal" => {
                        push_char('\u{2A75}');
                    }
                    b"eDDot" | b"ddotseq" => {
                        push_char('\u{2A77}');
                    }
                    b"equivDD" => {
                        push_char('\u{2A78}');
                    }
                    b"ltcir" => {
                        push_char('\u{2A79}');
                    }
                    b"gtcir" => {
                        push_char('\u{2A7A}');
                    }
                    b"ltquest" => {
                        push_char('\u{2A7B}');
                    }
                    b"gtquest" => {
                        push_char('\u{2A7C}');
                    }
                    b"les" | b"LessSlantEqual" | b"leqslant" => {
                        push_char('\u{2A7D}');
                    }
                    b"ges" | b"GreaterSlantEqual" | b"geqslant" => {
                        push_char('\u{2A7E}');
                    }
                    b"lesdot" => {
                        push_char('\u{2A7F}');
                    }
                    b"gesdot" => {
                        push_char('\u{2A80}');
                    }
                    b"lesdoto" => {
                        push_char('\u{2A81}');
                    }
                    b"gesdoto" => {
                        push_char('\u{2A82}');
                    }
                    b"lesdotor" => {
                        push_char('\u{2A83}');
                    }
                    b"gesdotol" => {
                        push_char('\u{2A84}');
                    }
                    b"lap" | b"lessapprox" => {
                        push_char('\u{2A85}');
                    }
                    b"gap" | b"gtrapprox" => {
                        push_char('\u{2A86}');
                    }
                    b"lne" | b"lneq" => {
                        push_char('\u{2A87}');
                    }
                    b"gne" | b"gneq" => {
                        push_char('\u{2A88}');
                    }
                    b"lnap" | b"lnapprox" => {
                        push_char('\u{2A89}');
                    }
                    b"gnap" | b"gnapprox" => {
                        push_char('\u{2A8A}');
                    }
                    b"lEg" | b"lesseqqgtr" => {
                        push_char('\u{2A8B}');
                    }
                    b"gEl" | b"gtreqqless" => {
                        push_char('\u{2A8C}');
                    }
                    b"lsime" => {
                        push_char('\u{2A8D}');
                    }
                    b"gsime" => {
                        push_char('\u{2A8E}');
                    }
                    b"lsimg" => {
                        push_char('\u{2A8F}');
                    }
                    b"gsiml" => {
                        push_char('\u{2A90}');
                    }
                    b"lgE" => {
                        push_char('\u{2A91}');
                    }
                    b"glE" => {
                        push_char('\u{2A92}');
                    }
                    b"lesges" => {
                        push_char('\u{2A93}');
                    }
                    b"gesles" => {
                        push_char('\u{2A94}');
                    }
                    b"els" | b"eqslantless" => {
                        push_char('\u{2A95}');
                    }
                    b"egs" | b"eqslantgtr" => {
                        push_char('\u{2A96}');
                    }
                    b"elsdot" => {
                        push_char('\u{2A97}');
                    }
                    b"egsdot" => {
                        push_char('\u{2A98}');
                    }
                    b"el" => {
                        push_char('\u{2A99}');
                    }
                    b"eg" => {
                        push_char('\u{2A9A}');
                    }
                    b"siml" => {
                        push_char('\u{2A9D}');
                    }
                    b"simg" => {
                        push_char('\u{2A9E}');
                    }
                    b"simlE" => {
                        push_char('\u{2A9F}');
                    }
                    b"simgE" => {
                        push_char('\u{2AA0}');
                    }
                    b"LessLess" => {
                        push_char('\u{2AA1}');
                    }
                    b"GreaterGreater" => {
                        push_char('\u{2AA2}');
                    }
                    b"glj" => {
                        push_char('\u{2AA4}');
                    }
                    b"gla" => {
                        push_char('\u{2AA5}');
                    }
                    b"ltcc" => {
                        push_char('\u{2AA6}');
                    }
                    b"gtcc" => {
                        push_char('\u{2AA7}');
                    }
                    b"lescc" => {
                        push_char('\u{2AA8}');
                    }
                    b"gescc" => {
                        push_char('\u{2AA9}');
                    }
                    b"smt" => {
                        push_char('\u{2AAA}');
                    }
                    b"lat" => {
                        push_char('\u{2AAB}');
                    }
                    b"smte" => {
                        push_char('\u{2AAC}');
                    }
                    b"late" => {
                        push_char('\u{2AAD}');
                    }
                    b"bumpE" => {
                        push_char('\u{2AAE}');
                    }
                    b"pre" | b"preceq" | b"PrecedesEqual" => {
                        push_char('\u{2AAF}');
                    }
                    b"sce" | b"succeq" | b"SucceedsEqual" => {
                        push_char('\u{2AB0}');
                    }
                    b"prE" => {
                        push_char('\u{2AB3}');
                    }
                    b"scE" => {
                        push_char('\u{2AB4}');
                    }
                    b"prnE" | b"precneqq" => {
                        push_char('\u{2AB5}');
                    }
                    b"scnE" | b"succneqq" => {
                        push_char('\u{2AB6}');
                    }
                    b"prap" | b"precapprox" => {
                        push_char('\u{2AB7}');
                    }
                    b"scap" | b"succapprox" => {
                        push_char('\u{2AB8}');
                    }
                    b"prnap" | b"precnapprox" => {
                        push_char('\u{2AB9}');
                    }
                    b"scnap" | b"succnapprox" => {
                        push_char('\u{2ABA}');
                    }
                    b"Pr" => {
                        push_char('\u{2ABB}');
                    }
                    b"Sc" => {
                        push_char('\u{2ABC}');
                    }
                    b"subdot" => {
                        push_char('\u{2ABD}');
                    }
                    b"supdot" => {
                        push_char('\u{2ABE}');
                    }
                    b"subplus" => {
                        push_char('\u{2ABF}');
                    }
                    b"supplus" => {
                        push_char('\u{2AC0}');
                    }
                    b"submult" => {
                        push_char('\u{2AC1}');
                    }
                    b"supmult" => {
                        push_char('\u{2AC2}');
                    }
                    b"subedot" => {
                        push_char('\u{2AC3}');
                    }
                    b"supedot" => {
                        push_char('\u{2AC4}');
                    }
                    b"subE" | b"subseteqq" => {
                        push_char('\u{2AC5}');
                    }
                    b"supE" | b"supseteqq" => {
                        push_char('\u{2AC6}');
                    }
                    b"subsim" => {
                        push_char('\u{2AC7}');
                    }
                    b"supsim" => {
                        push_char('\u{2AC8}');
                    }
                    b"subnE" | b"subsetneqq" => {
                        push_char('\u{2ACB}');
                    }
                    b"supnE" | b"supsetneqq" => {
                        push_char('\u{2ACC}');
                    }
                    b"csub" => {
                        push_char('\u{2ACF}');
                    }
                    b"csup" => {
                        push_char('\u{2AD0}');
                    }
                    b"csube" => {
                        push_char('\u{2AD1}');
                    }
                    b"csupe" => {
                        push_char('\u{2AD2}');
                    }
                    b"subsup" => {
                        push_char('\u{2AD3}');
                    }
                    b"supsub" => {
                        push_char('\u{2AD4}');
                    }
                    b"subsub" => {
                        push_char('\u{2AD5}');
                    }
                    b"supsup" => {
                        push_char('\u{2AD6}');
                    }
                    b"suphsub" => {
                        push_char('\u{2AD7}');
                    }
                    b"supdsub" => {
                        push_char('\u{2AD8}');
                    }
                    b"forkv" => {
                        push_char('\u{2AD9}');
                    }
                    b"topfork" => {
                        push_char('\u{2ADA}');
                    }
                    b"mlcp" => {
                        push_char('\u{2ADB}');
                    }
                    b"Dashv" | b"DoubleLeftTee" => {
                        push_char('\u{2AE4}');
                    }
                    b"Vdashl" => {
                        push_char('\u{2AE6}');
                    }
                    b"Barv" => {
                        push_char('\u{2AE7}');
                    }
                    b"vBar" => {
                        push_char('\u{2AE8}');
                    }
                    b"vBarv" => {
                        push_char('\u{2AE9}');
                    }
                    b"Vbar" => {
                        push_char('\u{2AEB}');
                    }
                    b"Not" => {
                        push_char('\u{2AEC}');
                    }
                    b"bNot" => {
                        push_char('\u{2AED}');
                    }
                    b"rnmid" => {
                        push_char('\u{2AEE}');
                    }
                    b"cirmid" => {
                        push_char('\u{2AEF}');
                    }
                    b"midcir" => {
                        push_char('\u{2AF0}');
                    }
                    b"topcir" => {
                        push_char('\u{2AF1}');
                    }
                    b"nhpar" => {
                        push_char('\u{2AF2}');
                    }
                    b"parsim" => {
                        push_char('\u{2AF3}');
                    }
                    b"parsl" => {
                        push_char('\u{2AFD}');
                    }
                    b"fflig" => {
                        push_char('\u{FB00}');
                    }
                    b"filig" => {
                        push_char('\u{FB01}');
                    }
                    b"fllig" => {
                        push_char('\u{FB02}');
                    }
                    b"ffilig" => {
                        push_char('\u{FB03}');
                    }
                    b"ffllig" => {
                        push_char('\u{FB04}');
                    }
                    b"Ascr" => {
                        push_char('\u{1D49}');
                    }
                    b"Cscr" => {
                        push_char('\u{1D49}');
                    }
                    b"Dscr" => {
                        push_char('\u{1D49}');
                    }
                    b"Gscr" => {
                        push_char('\u{1D4A}');
                    }
                    b"Jscr" => {
                        push_char('\u{1D4A}');
                    }
                    b"Kscr" => {
                        push_char('\u{1D4A}');
                    }
                    b"Nscr" => {
                        push_char('\u{1D4A}');
                    }
                    b"Oscr" => {
                        push_char('\u{1D4A}');
                    }
                    b"Pscr" => {
                        push_char('\u{1D4A}');
                    }
                    b"Qscr" => {
                        push_char('\u{1D4A}');
                    }
                    b"Sscr" => {
                        push_char('\u{1D4A}');
                    }
                    b"Tscr" => {
                        push_char('\u{1D4A}');
                    }
                    b"Uscr" => {
                        push_char('\u{1D4B}');
                    }
                    b"Vscr" => {
                        push_char('\u{1D4B}');
                    }
                    b"Wscr" => {
                        push_char('\u{1D4B}');
                    }
                    b"Xscr" => {
                        push_char('\u{1D4B}');
                    }
                    b"Yscr" => {
                        push_char('\u{1D4B}');
                    }
                    b"Zscr" => {
                        push_char('\u{1D4B}');
                    }
                    b"ascr" => {
                        push_char('\u{1D4B}');
                    }
                    b"bscr" => {
                        push_char('\u{1D4B}');
                    }
                    b"cscr" => {
                        push_char('\u{1D4B}');
                    }
                    b"dscr" => {
                        push_char('\u{1D4B}');
                    }
                    b"fscr" => {
                        push_char('\u{1D4B}');
                    }
                    b"hscr" => {
                        push_char('\u{1D4B}');
                    }
                    b"iscr" => {
                        push_char('\u{1D4B}');
                    }
                    b"jscr" => {
                        push_char('\u{1D4B}');
                    }
                    b"kscr" => {
                        push_char('\u{1D4C}');
                    }
                    b"lscr" => {
                        push_char('\u{1D4C}');
                    }
                    b"mscr" => {
                        push_char('\u{1D4C}');
                    }
                    b"nscr" => {
                        push_char('\u{1D4C}');
                    }
                    b"pscr" => {
                        push_char('\u{1D4C}');
                    }
                    b"qscr" => {
                        push_char('\u{1D4C}');
                    }
                    b"rscr" => {
                        push_char('\u{1D4C}');
                    }
                    b"sscr" => {
                        push_char('\u{1D4C}');
                    }
                    b"tscr" => {
                        push_char('\u{1D4C}');
                    }
                    b"uscr" => {
                        push_char('\u{1D4C}');
                    }
                    b"vscr" => {
                        push_char('\u{1D4C}');
                    }
                    b"wscr" => {
                        push_char('\u{1D4C}');
                    }
                    b"xscr" => {
                        push_char('\u{1D4C}');
                    }
                    b"yscr" => {
                        push_char('\u{1D4C}');
                    }
                    b"zscr" => {
                        push_char('\u{1D4C}');
                    }
                    b"Afr" => {
                        push_char('\u{1D50}');
                    }
                    b"Bfr" => {
                        push_char('\u{1D50}');
                    }
                    b"Dfr" => {
                        push_char('\u{1D50}');
                    }
                    b"Efr" => {
                        push_char('\u{1D50}');
                    }
                    b"Ffr" => {
                        push_char('\u{1D50}');
                    }
                    b"Gfr" => {
                        push_char('\u{1D50}');
                    }
                    b"Jfr" => {
                        push_char('\u{1D50}');
                    }
                    b"Kfr" => {
                        push_char('\u{1D50}');
                    }
                    b"Lfr" => {
                        push_char('\u{1D50}');
                    }
                    b"Mfr" => {
                        push_char('\u{1D51}');
                    }
                    b"Nfr" => {
                        push_char('\u{1D51}');
                    }
                    b"Ofr" => {
                        push_char('\u{1D51}');
                    }
                    b"Pfr" => {
                        push_char('\u{1D51}');
                    }
                    b"Qfr" => {
                        push_char('\u{1D51}');
                    }
                    b"Sfr" => {
                        push_char('\u{1D51}');
                    }
                    b"Tfr" => {
                        push_char('\u{1D51}');
                    }
                    b"Ufr" => {
                        push_char('\u{1D51}');
                    }
                    b"Vfr" => {
                        push_char('\u{1D51}');
                    }
                    b"Wfr" => {
                        push_char('\u{1D51}');
                    }
                    b"Xfr" => {
                        push_char('\u{1D51}');
                    }
                    b"Yfr" => {
                        push_char('\u{1D51}');
                    }
                    b"afr" => {
                        push_char('\u{1D51}');
                    }
                    b"bfr" => {
                        push_char('\u{1D51}');
                    }
                    b"cfr" => {
                        push_char('\u{1D52}');
                    }
                    b"dfr" => {
                        push_char('\u{1D52}');
                    }
                    b"efr" => {
                        push_char('\u{1D52}');
                    }
                    b"ffr" => {
                        push_char('\u{1D52}');
                    }
                    b"gfr" => {
                        push_char('\u{1D52}');
                    }
                    b"hfr" => {
                        push_char('\u{1D52}');
                    }
                    b"ifr" => {
                        push_char('\u{1D52}');
                    }
                    b"jfr" => {
                        push_char('\u{1D52}');
                    }
                    b"kfr" => {
                        push_char('\u{1D52}');
                    }
                    b"lfr" => {
                        push_char('\u{1D52}');
                    }
                    b"mfr" => {
                        push_char('\u{1D52}');
                    }
                    b"nfr" => {
                        push_char('\u{1D52}');
                    }
                    b"ofr" => {
                        push_char('\u{1D52}');
                    }
                    b"pfr" => {
                        push_char('\u{1D52}');
                    }
                    b"qfr" => {
                        push_char('\u{1D52}');
                    }
                    b"rfr" => {
                        push_char('\u{1D52}');
                    }
                    b"sfr" => {
                        push_char('\u{1D53}');
                    }
                    b"tfr" => {
                        push_char('\u{1D53}');
                    }
                    b"ufr" => {
                        push_char('\u{1D53}');
                    }
                    b"vfr" => {
                        push_char('\u{1D53}');
                    }
                    b"wfr" => {
                        push_char('\u{1D53}');
                    }
                    b"xfr" => {
                        push_char('\u{1D53}');
                    }
                    b"yfr" => {
                        push_char('\u{1D53}');
                    }
                    b"zfr" => {
                        push_char('\u{1D53}');
                    }
                    b"Aopf" => {
                        push_char('\u{1D53}');
                    }
                    b"Bopf" => {
                        push_char('\u{1D53}');
                    }
                    b"Dopf" => {
                        push_char('\u{1D53}');
                    }
                    b"Eopf" => {
                        push_char('\u{1D53}');
                    }
                    b"Fopf" => {
                        push_char('\u{1D53}');
                    }
                    b"Gopf" => {
                        push_char('\u{1D53}');
                    }
                    b"Iopf" => {
                        push_char('\u{1D54}');
                    }
                    b"Jopf" => {
                        push_char('\u{1D54}');
                    }
                    b"Kopf" => {
                        push_char('\u{1D54}');
                    }
                    b"Lopf" => {
                        push_char('\u{1D54}');
                    }
                    b"Mopf" => {
                        push_char('\u{1D54}');
                    }
                    b"Oopf" => {
                        push_char('\u{1D54}');
                    }
                    b"Sopf" => {
                        push_char('\u{1D54}');
                    }
                    b"Topf" => {
                        push_char('\u{1D54}');
                    }
                    b"Uopf" => {
                        push_char('\u{1D54}');
                    }
                    b"Vopf" => {
                        push_char('\u{1D54}');
                    }
                    b"Wopf" => {
                        push_char('\u{1D54}');
                    }
                    b"Xopf" => {
                        push_char('\u{1D54}');
                    }
                    b"Yopf" => {
                        push_char('\u{1D55}');
                    }
                    b"aopf" => {
                        push_char('\u{1D55}');
                    }
                    b"bopf" => {
                        push_char('\u{1D55}');
                    }
                    b"copf" => {
                        push_char('\u{1D55}');
                    }
                    b"dopf" => {
                        push_char('\u{1D55}');
                    }
                    b"eopf" => {
                        push_char('\u{1D55}');
                    }
                    b"fopf" => {
                        push_char('\u{1D55}');
                    }
                    b"gopf" => {
                        push_char('\u{1D55}');
                    }
                    b"hopf" => {
                        push_char('\u{1D55}');
                    }
                    b"iopf" => {
                        push_char('\u{1D55}');
                    }
                    b"jopf" => {
                        push_char('\u{1D55}');
                    }
                    b"kopf" => {
                        push_char('\u{1D55}');
                    }
                    b"lopf" => {
                        push_char('\u{1D55}');
                    }
                    b"mopf" => {
                        push_char('\u{1D55}');
                    }
                    b"nopf" => {
                        push_char('\u{1D55}');
                    }
                    b"oopf" => {
                        push_char('\u{1D56}');
                    }
                    b"popf" => {
                        push_char('\u{1D56}');
                    }
                    b"qopf" => {
                        push_char('\u{1D56}');
                    }
                    b"ropf" => {
                        push_char('\u{1D56}');
                    }
                    b"sopf" => {
                        push_char('\u{1D56}');
                    }
                    b"topf" => {
                        push_char('\u{1D56}');
                    }
                    b"uopf" => {
                        push_char('\u{1D56}');
                    }
                    b"vopf" => {
                        push_char('\u{1D56}');
                    }
                    b"wopf" => {
                        push_char('\u{1D56}');
                    }
                    b"xopf" => {
                        push_char('\u{1D56}');
                    }
                    b"yopf" => {
                        push_char('\u{1D56}');
                    }
                    b"zopf" => {
                        push_char('\u{1D56}');
                    }
                    bytes if bytes.starts_with(b"#") => {
                        push_char(parse_number(&bytes[1..], start..end)?);
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

fn parse_number(bytes: &[u8], range: Range<usize>) -> Result<char, EscapeError> {
    let code = if bytes.starts_with(b"x") {
        parse_hexadecimal(&bytes[1..])
    } else {
        parse_decimal(&bytes)
    }?;
    if code == 0 {
        return Err(EscapeError::EntityWithNull(range));
    }
    match std::char::from_u32(code) {
        Some(c) => Ok(c),
        None => Err(EscapeError::InvalidCodepoint(code)),
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

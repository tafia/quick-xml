use encoding_rs::*;
use serde::Deserialize;
use serde_json::from_reader;
use std::collections::BTreeMap;
use std::fs::{write, File};

type Index = Vec<Option<u32>>;

/// Representation of https://github.com/whatwg/encoding/blob/main/indexes.json
///
/// `ASCII = \u{0000}..=\u{007F}`
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Indexes {
    /// List of pairs _(index, codepoint)_.
    ///
    /// Unused by the generator, included to prevent getting into `single_byte`
    gb18030_ranges: Vec<(usize, u32)>,

    /// Normalization table of code points in the range `\u{FF61}` to `\u{FF9F}`
    /// for `ISO-2022-JP` encoding.
    ///
    /// First entry in the vector is a normalized value for `\u{FF61}`, the last
    /// is for `\u{FF9F}` (63 entries).
    ///
    /// Unused by the generator, included to prevent getting into `single_byte`
    iso_2022_jp_katakana: Vec<u32>,

    /// List of code points that can be encoded by the [`BIG5`] encoding.
    ///
    /// ```text
    /// ASCII + big5[((0xA1 - 0x81) * 157)..]
    /// ```
    /// <https://encoding.spec.whatwg.org/#big5-encoder>
    big5: Index,

    /// List of code points that can be encoded by the [`EUC_KR`] encoding.
    ///
    /// ```text
    /// ASCII + EUC-KR table
    /// ```
    /// <https://encoding.spec.whatwg.org/#euc-kr-encoder>
    euc_kr: Index,

    /// List of code points that can be encoded by the following encoding:
    ///
    /// ## [`GBK`]
    /// ```text
    /// ASCII + gb18030 table - U+E5E5
    /// ```
    /// <https://encoding.spec.whatwg.org/#gb18030-encoder>
    ///
    /// ## [`GB18030`]
    /// ```text
    /// all Unicode - U+E5E5
    /// ```
    /// <https://encoding.spec.whatwg.org/#gb18030-encoder>
    gb18030: Index,

    /// List of code points that can be encoded by the following encoding:
    ///
    /// ## [`EUC_JP`]
    /// ```text
    /// ASCII + U+00A5 + U+203E + U+FF61..=U+FF9F + U+2212 + jis0208 table (== ISO_2022_JP)
    /// ```
    /// <https://encoding.spec.whatwg.org/#euc-jp-encoder>
    ///
    /// ## [`ISO_2022_JP`]
    /// ```text
    /// ASCII + U+00A5 + U+203E + U+2212 + U+FF61..=U+FF9F + jis0208 table (== EUC_JP)
    /// ```
    /// <https://encoding.spec.whatwg.org/#iso-2022-jp-encoder>
    ///
    /// ## [`SHIFT_JIS`]
    /// ```text
    /// ASCII + U+0080 + U+00A5 + U+203E + U+FF61..=U+FF9F + U+2212 + jis0208 table
    /// (without jis0208[8272..=8835], but that slice contains code points that duplicated
    /// in the other part of that table)
    /// ```
    /// <https://encoding.spec.whatwg.org/#shift_jis-encoder>
    jis0208: Index,

    /// Unused by the generator, included to prevent getting into `single_byte`
    jis0212: Index,

    /// List of code points that can be encoded by the single-byte encodings.
    ///
    /// ```text
    /// ASCII + corresponding table.
    /// ```
    ///
    /// <https://encoding.spec.whatwg.org/#single-byte-encoder>
    #[serde(flatten)]
    single_byte: BTreeMap<String, Index>,
}

/// > XML 1.1 allows the use of character references to the control characters
/// > #x1 through #x1F, most of which are forbidden in XML 1.0. For reasons of
/// > robustness, however, these characters still cannot be used directly in
/// > documents. In order to improve the robustness of character encoding detection,
/// > the additional control characters #x7F through #x9F, which were freely allowed
/// > in XML 1.0 documents, now must also appear only as character references.
/// > (Whitespace characters are of course exempt.)
///
/// https://www.w3.org/TR/xml11/#sec-xml11
fn is_literal_xml11_char(ch: char) -> bool {
    // https://www.w3.org/TR/xml11/#NT-Char
    match ch {
        '\u{0001}'..='\u{D7FF}' => match ch {
            // These chars can only appear as character references
            // https://www.w3.org/TR/xml11/#NT-RestrictedChar
            '\u{0001}'..='\u{0008}' => false,
            '\u{000B}'..='\u{000C}' => false,
            '\u{000E}'..='\u{001F}' => false,
            '\u{007F}'..='\u{0084}' => false,
            '\u{0086}'..='\u{009F}' => false,
            _ => true,
        },
        '\u{E000}'..='\u{FFFD}' => true,
        '\u{10000}'..='\u{10FFFF}' => true,
        _ => false,
    }
}

/// Almost all characters can form a name. Citation from <https://www.w3.org/TR/xml11/#sec-xml11>:
///
/// > The overall philosophy of names has changed since XML 1.0. Whereas XML 1.0
/// > provided a rigid definition of names, wherein everything that was not permitted
/// > was forbidden, XML 1.1 names are designed so that everything that is not
/// > forbidden (for a specific reason) is permitted. Since Unicode will continue
/// > to grow past version 4.0, further changes to XML can be avoided by allowing
/// > almost any character, including those not yet assigned, in names.
///
/// <https://www.w3.org/TR/xml11/#NT-NameStartChar>
fn is_xml11_name_start_char(ch: char) -> bool {
    match ch {
        ':'
        | 'A'..='Z'
        | '_'
        | 'a'..='z'
        | '\u{00C0}'..='\u{00D6}'
        | '\u{00D8}'..='\u{00F6}'
        | '\u{00F8}'..='\u{02FF}'
        | '\u{0370}'..='\u{037D}'
        | '\u{037F}'..='\u{1FFF}'
        | '\u{200C}'..='\u{200D}'
        | '\u{2070}'..='\u{218F}'
        | '\u{2C00}'..='\u{2FEF}'
        | '\u{3001}'..='\u{D7FF}'
        | '\u{F900}'..='\u{FDCF}'
        | '\u{FDF0}'..='\u{FFFD}'
        | '\u{10000}'..='\u{EFFFF}' => true,
        _ => false,
    }
}

fn make_alphabet<I>(enc: &'static Encoding, codepoints: I) -> String
where
    I: IntoIterator<Item = char>,
{
    let iter = codepoints.into_iter();
    let mut alphabet = String::with_capacity(iter.size_hint().1.unwrap_or(256) * 4);
    // ASCII bytes (0x00 - 0x7F) does not included in encoding tables
    for ch in '\u{0000}'..='\u{007F}' {
        if is_literal_xml11_char(ch) {
            alphabet.push(ch);
        }
    }
    for (pointer, cp) in iter.enumerate() {
        // BIG5 encoding has unmappable code points in their index
        // https://github.com/whatwg/encoding/issues/293
        //
        // 0-5023 - pointers of unmapped characters (0x8140-0xA13F in Big5)
        // 5024   - pointer of a U+3000 (0xA140 in Big5)
        if enc == BIG5 && pointer < 5024 {
            continue;
        }
        // SHIFT_JIS: codepoints[8272..=8835] should be excluded
        // https://encoding.spec.whatwg.org/#index-shift_jis-pointer
        if enc == SHIFT_JIS && (8272..=8835).contains(&pointer) {
            continue;
        }

        if is_literal_xml11_char(cp) {
            alphabet.push(cp);
        }
    }
    alphabet
}

fn make_xml<I>(enc: &'static Encoding, codepoints: I)
where
    I: IntoIterator<Item = char>,
{
    println!(
        "{} - single:{}, ascii:{}",
        enc.name(),
        enc.is_single_byte(),
        enc.is_ascii_compatible()
    );
    println!("  - making alphabet");

    let alphabet = make_alphabet(enc, codepoints);

    println!("  - making xml");

    let name = alphabet.replace(|ch| !is_xml11_name_start_char(ch), "");
    let xml = format!(
        r#"<?xml version="1.1" encoding="{encoding}"?>
<!--This is generated file. Edit <quick-xml>/test-gen/src/main.rs instead-->
<root attribute1="{attr1}"
      attribute2='{attr2}'
      {attr_name}={attr3}
>
  <?{pi}?>
  <!--{comment}-->
  {text}
  <ns:{element} ns:attribute="value1" xmlns:ns="namespace"/>
  <![CDATA[{text}]]>
</root>"#,
        encoding = enc.name(),
        // https://www.w3.org/TR/xml11/#NT-AttValue
        attr1 = alphabet.replace(|ch| matches!(ch, '<' | '&' | '"'), ""),
        attr2 = alphabet.replace(|ch| matches!(ch, '<' | '&' | '\''), ""),
        attr_name = name,
        attr3 = name,
        pi = name,
        comment = alphabet,
        // https://www.w3.org/TR/xml11/#dt-chardata
        text = alphabet.replace(|ch| matches!(ch, '<' | '&'), ""),
        element = name,
    );

    println!(
        "  - encode and write ../tests/documents/encoding/{}.xml",
        enc.name()
    );
    let (result, actual, _) = enc.encode(&xml);
    if enc == actual && enc != UTF_8 {
        write(
            format!("../tests/documents/encoding/{}.xml", enc.name()),
            result,
        )
        .unwrap();
    }
}
fn process_index(enc: &'static Encoding, codepoints: &Index) {
    make_xml(
        enc,
        codepoints.into_iter().filter_map(|cp| {
            // `char` cannot be deserialized from integer in JSON directly
            cp.map(|cp| char::from_u32(cp).expect(&format!("`{}` is not a code point", cp)))
        }),
    )
}

/// Generates test files in {quick-xml}/tests/documents/encoding/{}.xml
fn main() {
    let index = "encoding/indexes.json";
    let file = File::open(index).expect(&format!(
        r#"unable to load `{}`. Probably `encoding` submodule does not fetched? Try to run

        git submodule update --init -- encoding

        in the current working dir (i. e. <quick-xml>/test-gen/)
        "#,
        index
    ));
    let indexes: Indexes = from_reader(file).expect(&format!("invalid format of `{}`", index));

    process_index(BIG5, &indexes.big5);
    process_index(EUC_KR, &indexes.euc_kr);

    process_index(GBK, &indexes.gb18030);
    // It is too expensive to generate full Unicode alphabet, but at least pass significant part of them
    process_index(GB18030, &indexes.gb18030);

    process_index(EUC_JP, &indexes.jis0208);
    process_index(ISO_2022_JP, &indexes.jis0208);
    process_index(SHIFT_JIS, &indexes.jis0208);

    for (label, codepoints) in indexes.single_byte.into_iter() {
        let enc = Encoding::for_label(label.as_bytes())
            .expect(&format!("label `{}` is unsupported", label));

        process_index(enc, &codepoints);
    }
    // https://encoding.spec.whatwg.org/#x-user-defined-decoder
    make_xml(X_USER_DEFINED, '\u{F780}'..='\u{F7FF}');
}

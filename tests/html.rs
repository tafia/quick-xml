use pretty_assertions::assert_eq;
use quick_xml::encoding::Decoder;
use quick_xml::escape::unescape;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::{QName, ResolveResult};
use quick_xml::reader::NsReader;
use std::str::from_utf8;

#[test]
fn html5() {
    test(
        include_str!("documents/html5.html"),
        include_str!("documents/html5.txt"),
        false,
    );
}

#[test]
fn escaped_characters_html() {
    test(
        r#"<e attr="&planck;&Egrave;&ell;&#x1D55D;&bigodot;">&boxDR;&boxDL;&#x02554;&#x02557;&#9556;&#9559;</e>"#,
        r#"
            |StartElement(e [attr="ℏÈℓ𝕝⨀"])
            |Reference(boxDR)
            |Reference(boxDL)
            |Reference(#x02554)
            |Reference(#x02557)
            |Reference(#9556)
            |Reference(#9559)
            |EndElement(e)
            |EndDocument
        "#,
        true,
    )
}

#[track_caller]
fn test(input: &str, output: &str, trim: bool) {
    test_bytes(input.as_bytes(), output.as_bytes(), trim);
}

#[track_caller]
fn test_bytes(input: &[u8], output: &[u8], trim: bool) {
    let mut reader = NsReader::from_reader(input);
    let config = reader.config_mut();
    config.trim_text(trim);
    config.check_comments = true;

    let mut spec_lines = SpecIter(output).enumerate();

    let mut decoder = reader.decoder();
    loop {
        let line = match reader.read_resolved_event() {
            Ok((_, Event::Decl(e))) => {
                // Declaration could change decoder
                decoder = reader.decoder();

                let version_cow = e.version().unwrap();
                let version = decoder.decode(version_cow.as_ref()).unwrap();
                let encoding_cow = e.encoding().unwrap().unwrap();
                let encoding = decoder.decode(encoding_cow.as_ref()).unwrap();
                format!("StartDocument({}, {})", version, encoding)
            }
            Ok((_, Event::PI(e))) => {
                format!("ProcessingInstruction(PI={})", decoder.decode(&e).unwrap())
            }
            Ok((_, Event::DocType(e))) => format!("DocType({})", decoder.decode(&e).unwrap()),
            Ok((n, Event::Start(e))) => {
                let name = namespace_name(n, e.name(), decoder);
                match make_attrs(&e, decoder) {
                    Ok(attrs) if attrs.is_empty() => format!("StartElement({})", &name),
                    Ok(attrs) => format!("StartElement({} [{}])", &name, &attrs),
                    Err(e) => format!("StartElement({}, attr-error: {})", &name, &e),
                }
            }
            Ok((n, Event::Empty(e))) => {
                let name = namespace_name(n, e.name(), decoder);
                match make_attrs(&e, decoder) {
                    Ok(attrs) if attrs.is_empty() => format!("EmptyElement({})", &name),
                    Ok(attrs) => format!("EmptyElement({} [{}])", &name, &attrs),
                    Err(e) => format!("EmptyElement({}, attr-error: {})", &name, &e),
                }
            }
            Ok((n, Event::End(e))) => {
                let name = namespace_name(n, e.name(), decoder);
                format!("EndElement({})", name)
            }
            Ok((_, Event::Comment(e))) => format!("Comment({})", decoder.decode(&e).unwrap()),
            Ok((_, Event::CData(e))) => format!("CData({})", decoder.decode(&e).unwrap()),
            Ok((_, Event::Text(e))) => match unescape(&decoder.decode(&e).unwrap()) {
                Ok(c) => format!("Characters({})", &c),
                Err(err) => format!("FailedUnescape({:?}; {})", e.as_ref(), err),
            },
            Ok((_, Event::GeneralRef(e))) => match unescape(&decoder.decode(&e).unwrap()) {
                Ok(c) => format!("Reference({})", &c),
                Err(err) => format!("FailedUnescape({:?}; {})", e.as_ref(), err),
            },
            Ok((_, Event::Eof)) => "EndDocument".to_string(),
            Err(e) => format!("Error: {}", e),
        };
        if let Some((n, spec)) = spec_lines.next() {
            if spec.trim() == "EndDocument" {
                break;
            }
            assert_eq!(
                line.trim(),
                spec.trim(),
                "Unexpected event at line {}",
                n + 1
            );
        } else {
            if line == "EndDocument" {
                break;
            }
            panic!("Unexpected event: {}", line);
        }
    }
}

fn namespace_name(n: ResolveResult, name: QName, decoder: Decoder) -> String {
    let name = decoder.decode(name.as_ref()).unwrap();
    match n {
        // Produces string '{namespace}prefixed_name'
        ResolveResult::Bound(n) => format!("{{{}}}{}", decoder.decode(n.as_ref()).unwrap(), name),
        _ => name.to_string(),
    }
}

fn make_attrs(e: &BytesStart, decoder: Decoder) -> ::std::result::Result<String, String> {
    let mut atts = Vec::new();
    for a in e.attributes() {
        match a {
            Ok(a) => {
                if a.key.as_namespace_binding().is_none() {
                    let key = decoder.decode(a.key.as_ref()).unwrap();
                    let value = decoder.decode(a.value.as_ref()).unwrap();
                    let unescaped_value = unescape(&value).unwrap();
                    atts.push(format!(
                        "{}=\"{}\"",
                        key,
                        // unescape does not change validity of an UTF-8 string
                        &unescaped_value
                    ));
                }
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(atts.join(", "))
}

struct SpecIter<'a>(&'a [u8]);

impl<'a> Iterator for SpecIter<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        let start = self
            .0
            .iter()
            .position(|b| !matches!(*b, b' ' | b'\r' | b'\n' | b'\t' | b'|' | b':' | b'0'..=b'9'))
            .unwrap_or(0);

        if let Some(p) = self.0.windows(3).position(|w| w == b")\r\n") {
            let (prev, next) = self.0.split_at(p + 1);
            self.0 = &next[1..];
            Some(from_utf8(&prev[start..]).expect("Error decoding to uft8"))
        } else if let Some(p) = self.0.windows(2).position(|w| w == b")\n") {
            let (prev, next) = self.0.split_at(p + 1);
            self.0 = next;
            Some(from_utf8(&prev[start..]).expect("Error decoding to uft8"))
        } else if self.0.is_empty() {
            None
        } else {
            let p = self.0;
            self.0 = &[];
            Some(from_utf8(&p[start..]).unwrap())
        }
    }
}

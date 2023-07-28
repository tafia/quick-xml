//! Cases that was found by fuzzing

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use quick_xml::Error;

#[test]
fn fuzz_53() {
    let data: &[u8] = b"\xe9\x00\x00\x00\x00\x00\x00\x00\x00\
\x00\x00\x00\x00\n(\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\
\x00<>\x00\x08\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00<<\x00\x00\x00";
    let mut reader = Reader::from_reader(data);
    let mut buf = vec![];
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) | Err(..) => break,
            _ => buf.clear(),
        }
    }
}

#[test]
fn fuzz_101() {
    let data: &[u8] = b"\x00\x00<\x00\x00\x0a>&#44444444401?#\x0a413518\
                       #\x0a\x0a\x0a;<:<)(<:\x0a\x0a\x0a\x0a;<:\x0a\x0a\
                       <:\x0a\x0a\x0a\x0a\x0a<\x00*\x00\x00\x00\x00";
    let mut reader = Reader::from_reader(data);
    let mut buf = vec![];
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                for a in e.attributes() {
                    if a.ok().map_or(true, |a| a.unescape_value().is_err()) {
                        break;
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if e.unescape().is_err() {
                    break;
                }
            }
            Ok(Event::Eof) | Err(..) => break,
            _ => (),
        }
        buf.clear();
    }
}

#[test]
fn fuzz_empty_doctype() {
    let data: &[u8] = b"<!doctype  \n    >";
    let mut reader = Reader::from_reader(data);
    let mut buf = Vec::new();
    assert!(matches!(
        reader.read_event_into(&mut buf).unwrap_err(),
        Error::EmptyDocType
    ));
    assert_eq!(reader.read_event_into(&mut buf).unwrap(), Event::Eof);
}

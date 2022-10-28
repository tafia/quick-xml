#![no_main]
use libfuzzer_sys::fuzz_target;

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    // fuzzed code goes here
    let cursor = Cursor::new(data);
    let mut reader = Reader::from_reader(cursor);
    let mut buf = vec![];
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                for a in e.attributes() {
                    if a.ok().map_or(false, |a| a.unescape_value().is_err()) {
                        break;
                    }
                }
            }
            Ok(Event::Text(ref e)) | Ok(Event::Comment(ref e))
            | Ok(Event::PI(ref e))
            | Ok(Event::DocType(ref e)) => {
                if e.unescape().is_err() {
                    break;
                }
            }
            Ok(Event::CData(e)) => {
                if e.escape().is_err() {
                    break;
                }
            }
            Ok(Event::Decl(ref e)) => {
                let _ = e.version();
                let _ = e.encoding();
                let _ = e.standalone();
            }
            Ok(Event::End(_)) => (),
            Ok(Event::Eof) | Err(..) => break,
        }
        buf.clear();
    }
});

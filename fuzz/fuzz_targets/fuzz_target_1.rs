#![no_main]
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

use quick_xml::{events::Event, reader::Reader, writer::Writer};
use std::io::Cursor;

macro_rules! debug_format {
    ($x:expr) => {
        let _unused = std::hint::black_box(format!("{:?}", $x));
    };
}

fn round_trip<R>(reader: &mut Reader<R>) -> ()
where
    R: std::io::BufRead,
{
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = vec![];
    let config = reader.config_mut();
    config.expand_empty_elements = true;
    config.trim_text(true);
    loop {
        let event_result = reader.read_event_into(&mut buf);
        if let Ok(ref event) = event_result {
            let _event = black_box(event.borrow());
            let _event = black_box(event.as_ref());
            debug_format!(event);
            debug_format!(writer.write_event(event.borrow()));
        }
        match event_result {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                debug_format!(e);
                debug_format!(e.name());
                for a in e.attributes() {
                    debug_format!(a);
                    if a.ok().map_or(false, |a| a.unescape_value().is_err()) {
                        break;
                    }
                }
            }
            Ok(Event::Text(ref e))
            | Ok(Event::Comment(ref e))
            | Ok(Event::DocType(ref e)) => {
                debug_format!(e);
                if let Err(err) = e.decode() {
                    debug_format!(err);
                    break;
                }
            }
            Ok(Event::CData(e)) => {
                if let Err(err) = e.escape() {
                    let _displayed = black_box(format!("{}", err));
                    debug_format!(err);
                    break;
                }
            }
            Ok(Event::GeneralRef(ref e)) => {
                debug_format!(e);
                debug_format!(e.is_char_ref());
                debug_format!(e.resolve_char_ref());
            }
            Ok(Event::PI(ref e)) => {
                debug_format!(e);
            }
            Ok(Event::Decl(ref e)) => {
                debug_format!(e);
                let _ = black_box(e.version());
                let _ = black_box(e.encoding());
                let _ = black_box(e.standalone());
            }
            Ok(Event::End(e)) => {
                debug_format!(e.local_name());
                let name = e.name();
                debug_format!(name);
                debug_format!(name.prefix());
                debug_format!(name.local_name());
                debug_format!(name.decompose());
                debug_format!(name.as_namespace_binding());
                debug_format!(e);
            }
            Err(e) => {
                debug_format!(e);
                break;
            }
            Ok(Event::Eof) => break,
        }
        buf.clear();
    }
    let _round_trip = std::hint::black_box(writer.into_inner().into_inner());
}

fuzz_target!(|data: &[u8]| {
    // From reader
    let cursor = Cursor::new(data);
    let mut reader = Reader::from_reader(cursor);
    _ = std::hint::black_box(round_trip(&mut reader));

    // From str
    if let Ok(s) = std::str::from_utf8(data) {
        let mut reader = Reader::from_str(s);
        _ = std::hint::black_box(round_trip(&mut reader));
    }
});

#![no_main]
extern crate libfuzzer_sys;
extern crate quick_xml;

use quick_xml::reader::Reader;
use std::io::Cursor;

#[export_name="rust_fuzzer_test_input"]
pub extern fn go(data: &[u8]) {
    // fuzzed code goes here
    let cursor = Cursor::new(data);
    let mut reader = Reader::from_reader(cursor);
    let mut buf = vec![];
    loop {
        match reader.read_event(&mut buf) {
            Ok(quick_xml::events::Event::Eof) | Err(..) => break,
            _ => buf.clear(),
        }
    }
}


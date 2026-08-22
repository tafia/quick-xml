//! Fuzz target that drives `Reader::from_reader` over a `BufRead` whose
//! `fill_buf` returns small windows, forcing the parser to handle state
//! that spans chunk boundaries.
//!
//! The default `fuzz_target_1` feeds the parser via `Cursor::new(data)`,
//! and `<Cursor as BufRead>::fill_buf` always returns the entire remaining
//! input in a single window. A class of parser bugs only manifests when
//! the underlying reader hands back a partial window and the parser must
//! resume on the next `fill_buf` call — see issues #950 and #957, both of
//! which the regression tests in `tests/issues.rs` reproduce by wrapping
//! the input in `BufReader::with_capacity(4, ..)`. This harness exposes
//! that same shape to the fuzzer with a fuzz-controlled capacity.

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::hint::black_box;
use std::io::{BufReader, Cursor};

use quick_xml::events::Event;
use quick_xml::reader::Reader;

fuzz_target!(|data: &[u8]| {
    // First byte selects the `BufReader` capacity in `1..=255`. Capacity 0
    // is not legal, and any capacity `>= data.len()` degenerates to the
    // Cursor case already covered by `fuzz_target_1`, so a small range is
    // both sufficient and the most efficient use of fuzz budget.
    let Some((&cap_byte, xml)) = data.split_first() else { return };
    let capacity = (cap_byte as usize).max(1);

    let mut reader =
        Reader::from_reader(BufReader::with_capacity(capacity, Cursor::new(xml)));
    let mut buf = Vec::new();
    loop {
        // Touch the event payload enough to exercise the borrowed-data
        // invariants on chunked inputs. Mirrors the shape of `fuzz_target_1`
        // without duplicating its full breadth — this target is about
        // chunk-boundary state, not API surface coverage.
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Err(e) => {
                let _ = black_box(format!("{e:?}"));
                break;
            }
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let _ = black_box(e.name());
                for a in e.attributes() {
                    if a.is_err() {
                        break;
                    }
                }
            }
            Ok(Event::Text(ref e)) | Ok(Event::DocType(ref e)) => {
                let _ = black_box(e);
            }
            Ok(Event::Comment(ref e)) => {
                let _ = black_box(e);
            }
            Ok(Event::CData(e)) => {
                let _ = black_box(e.escape());
            }
            Ok(Event::End(ref e)) => {
                let _ = black_box(e.name());
            }
            Ok(_) => {}
        }
        buf.clear();
    }
});

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use quick_xml::events::{BytesCData, BytesPI, BytesText, Event};
use quick_xml::reader::{Config, NsReader, Reader};
use quick_xml::writer::Writer;
use std::{hint::black_box, io::Cursor};

#[derive(Debug, arbitrary::Arbitrary)]
enum ElementWriterFunc<'a> {
    WriteTextContent(&'a str),
    WriteCDataContent(&'a str),
    WritePiContent(&'a str),
    WriteEmpty,
    // TODO: We can't automatically generate an arbitrary function
    // WriteInnerContent,
}

fn arbitrary_name(u: &mut Unstructured) -> arbitrary::Result<String> {
    let s = String::arbitrary(u)?;
    if s.is_empty() || !s.chars().all(char::is_alphanumeric) {
        return Err(arbitrary::Error::IncorrectFormat);
    }
    return Ok(s);
}

#[derive(Debug, arbitrary::Arbitrary)]
enum WriterFunc<'a> {
    WriteEvent(Event<'a>),
    WriteBom,
    WriteIndent,
    CreateElement {
        #[arbitrary(with = arbitrary_name)]
        name: String,
        func: ElementWriterFunc<'a>,
        attributes: Vec<(&'a str, &'a str)>,
    },
}

#[derive(Debug, arbitrary::Arbitrary)]
struct Driver<'a> {
    writer_funcs: Vec<WriterFunc<'a>>,
    reader_config: Config,
}

fn fuzz_round_trip(driver: Driver) -> quick_xml::Result<()> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let writer_funcs = driver.writer_funcs;
    for writer_func in writer_funcs.iter() {
        // TODO: Handle error cases.
        use WriterFunc::*;
        match writer_func {
            WriteEvent(event) => writer.write_event(event.borrow())?,
            WriteBom => writer.write_bom()?,
            WriteIndent => writer.write_indent()?,
            CreateElement {
                name,
                func,
                attributes,
            } => {
                let element_writer = writer
                    .create_element(name)
                    .with_attributes(attributes.into_iter().copied());
                use ElementWriterFunc::*;
                match func {
                    WriteTextContent(text) => {
                        element_writer.write_text_content(BytesText::from_escaped(*text))?;
                    }
                    WriteCDataContent(text) => {
                        _ = element_writer.write_cdata_content(BytesCData::new(*text))?;
                    }
                    WritePiContent(text) => {
                        _ = element_writer.write_pi_content(BytesPI::new(*text))?;
                    }
                    WriteEmpty => {
                        _ = element_writer.write_empty()?;
                    }
                }
            }
        }
    }
    let xml = writer.into_inner().into_inner();
    // The str should be valid as we just generated it, unwrapping **should** be safe.
    let mut reader = Reader::from_str(std::str::from_utf8(&xml).unwrap());
    *reader.config_mut() = driver.reader_config.clone();

    loop {
        let event = black_box(reader.read_event()?);
        if event == Event::Eof {
            break;
        }
    }

    let mut reader = NsReader::from_reader(&xml[..]);
    *reader.config_mut() = driver.reader_config;

    loop {
        let event = black_box(reader.read_event()?);
        if event == Event::Eof {
            break;
        }
    }
    Ok(())
}

fuzz_target!(|driver: Driver| {
    if let Err(e) = fuzz_round_trip(driver) {
        black_box(format!("{e:?}"));
    }
});

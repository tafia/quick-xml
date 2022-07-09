fn main() {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let xml = "<tag1>text1</tag1><tag1>text2</tag1>\
               <tag1>text3</tag1><tag1><tag2>text4</tag2></tag1>";

    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) if e.name().as_ref() == b"tag2" => {
                // read_text_into for buffered readers not implemented
                let txt = reader
                    .read_text(e.name())
                    .expect("Cannot decode text value");
                println!("{:?}", txt);
            }
            Ok(Event::Eof) => break, // exits the loop when reaching end of file
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (), // There are several other `Event`s we do not consider here
        }
    }
}

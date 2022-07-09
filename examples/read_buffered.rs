// This example demonstrates how a reader (for example when reading from a file)
// can be buffered. In that case, data read from the file is written to a supplied
// buffer and returned XML events borrow from that buffer.
// That way, allocations can be kept to a minimum.

fn main() -> Result<(), quick_xml::Error> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_file("tests/documents/document.xml")?;
    reader.trim_text(true);

    let mut buf = Vec::new();

    let mut count = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                let name = reader.decoder().decode(name.as_ref())?;
                println!("read start event {:?}", name.as_ref());
                count += 1;
            }
            Ok(Event::Eof) => break, // exits the loop when reaching end of file
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (), // There are several other `Event`s we do not consider here
        }
    }

    println!("read {} start events in total", count);

    Ok(())
}

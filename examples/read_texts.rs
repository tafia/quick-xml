#[cfg(feature = "asynchronous")]
use tokio::runtime::Runtime;

fn main() {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let xml = "<tag1>text1</tag1><tag1>text2</tag1>\
               <tag1>text3</tag1><tag1><tag2>text4</tag2></tag1>";

    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut txt = Vec::new();
    let mut buf = Vec::new();

    #[cfg(feature = "asynchronous")]
    let mut runtime = Runtime::new().expect("Runtime cannot be initialized");

    loop {
        #[cfg(feature = "asynchronous")]
        let event = runtime.block_on(async { reader.read_event(&mut buf).await });

        #[cfg(not(feature = "asynchronous"))]
        let event = reader.read_event(&mut buf);

        match event {
            Ok(Event::Start(ref e)) if e.name() == b"tag2" => {
                #[cfg(feature = "asynchronous")]
                let text = runtime.block_on(async {
                    reader
                        .read_text(b"tag2", &mut Vec::new())
                        .await
                        .expect("Cannot decode text value")
                });

                #[cfg(not(feature = "asynchronous"))]
                let text = reader
                    .read_text(b"tag2", &mut Vec::new())
                    .expect("Cannot decode text value");

                txt.push(text);
                println!("{:?}", txt);
            }
            Ok(Event::Eof) => break, // exits the loop when reaching end of file
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (), // There are several other `Event`s we do not consider here
        }
        buf.clear();
    }
}

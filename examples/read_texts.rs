use quick_xml::events::Event;
#[cfg(feature = "asynchronous")]
use quick_xml::AsyncReader;
use quick_xml::Reader;
#[cfg(feature = "asynchronous")]
use tokio::runtime::Runtime;

#[cfg(feature = "asynchronous")]
async fn read_text_async(xml: &str) {
    let mut reader = AsyncReader::from_str(xml);
    reader.trim_text(true);

    let mut txt = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf).await {
            Ok(Event::Start(ref e)) if e.name() == b"tag2" => {
                #[cfg(feature = "asynchronous")]
                let text = reader
                    .read_text(b"tag2", &mut Vec::new())
                    .await
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

fn read_text(xml: &str) {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut txt = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) if e.name() == b"tag2" => {
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

fn main() {
    let xml = "<tag1>text1</tag1><tag1>text2</tag1>\
               <tag1>text3</tag1><tag1><tag2>text4</tag2></tag1>";

    read_text(xml);

    #[cfg(feature = "asynchronous")]
    let runtime = Runtime::new().expect("Runtime cannot be initialized");

    #[cfg(feature = "asynchronous")]
    runtime.block_on(async { read_text_async(xml).await });
}

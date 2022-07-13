use std::path::PathBuf;

use quick_xml::events::Event::*;
use quick_xml::Reader;

#[tokio::test]
async fn test_sample() {
    let src: &[u8] = include_bytes!("documents/sample_rss.xml");
    let mut reader = Reader::from_async_reader(src);
    let mut buf = Vec::new();
    let mut count = 0;
    loop {
        match reader.read_event_into_async(&mut buf).await.unwrap() {
            Start(_) => count += 1,
            Decl(e) => println!("{:?}", e.version()),
            Eof => break,
            _ => (),
        }
        buf.clear();
    }
    println!("{}", count);
}

#[cfg(feature = "async-fs")]
#[tokio::test]
async fn test_read_file() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut reader = Reader::from_file_async(path.join("tests/documents/sample_rss.xml"))
        .await
        .unwrap();
    let mut buf = Vec::new();
    let mut count = 0;
    loop {
        match reader.read_event_into_async(&mut buf).await.unwrap() {
            Start(_) => count += 1,
            Decl(e) => println!("{:?}", e.version()),
            Eof => break,
            _ => (),
        }
        buf.clear();
    }
    println!("{}", count);
}

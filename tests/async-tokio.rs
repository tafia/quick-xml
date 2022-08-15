use quick_xml::events::Event::*;
use quick_xml::reader::Reader;

#[tokio::test]
async fn test_sample() {
    let src = include_str!("documents/sample_rss.xml");
    let mut reader = Reader::from_reader(src.as_bytes());
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

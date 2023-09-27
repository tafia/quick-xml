use pretty_assertions::assert_eq;
use quick_xml::events::Event::*;
use quick_xml::reader::Reader;

#[tokio::test]
async fn test_sample() {
    let src = include_str!("documents/sample_rss.xml");
    let mut reader = Reader::from_reader(src.as_bytes());
    let mut buf = Vec::new();
    let mut count = 0;
    // Expected number of iterations, to prevent infinity loops if refactoring breaks test
    let mut reads = 0;
    loop {
        reads += 1;
        assert!(
            reads <= 5245,
            "too many events, possible infinity loop: {reads}"
        );
        match reader.read_event_into_async(&mut buf).await.unwrap() {
            Start(_) => count += 1,
            Decl(e) => assert_eq!(e.version().unwrap(), b"1.0".as_ref()),
            Eof => break,
            _ => (),
        }
        buf.clear();
    }
    assert_eq!((count, reads), (1247, 5245));
}

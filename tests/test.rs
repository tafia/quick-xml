use quick_xml::events::Event::*;
use quick_xml::reader::Reader;

#[test]
fn test_sample() {
    let src = include_str!("documents/sample_rss.xml");
    let mut r = Reader::from_str(src);
    let mut count = 0;
    loop {
        match r.read_event().unwrap() {
            Start(_) => count += 1,
            Decl(e) => println!("{:?}", e.version()),
            Eof => break,
            _ => (),
        }
    }
    println!("{}", count);
}

#[test]
fn test_clone_reader() {
    let mut reader = Reader::from_str("<tag>text</tag>");

    assert!(matches!(reader.read_event().unwrap(), Start(_)));

    let mut cloned = reader.clone();

    assert!(matches!(reader.read_event().unwrap(), Text(_)));
    assert!(matches!(reader.read_event().unwrap(), End(_)));

    assert!(matches!(cloned.read_event().unwrap(), Text(_)));
    assert!(matches!(cloned.read_event().unwrap(), End(_)));
}

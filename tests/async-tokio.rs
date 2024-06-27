use std::iter;

use pretty_assertions::assert_eq;
use quick_xml::events::{BytesStart, Event::*};
use quick_xml::name::QName;
use quick_xml::reader::Reader;

// Import `small_buffers_tests!`
#[macro_use]
mod helpers;

small_buffers_tests!(
    #[tokio::test]
    read_event_into_async: tokio::io::BufReader<_>,
    async, await
);

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

/// This tests checks that read_to_end() correctly returns span even when
/// text is trimmed from both sides
mod read_to_end {
    use super::*;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn text() {
        let mut r = Reader::from_str("<tag> text </tag>");
        //                            ^0   ^5    ^11
        r.config_mut().trim_text(true);

        let mut buf = Vec::new();
        assert_eq!(
            r.read_event_into_async(&mut buf).await.unwrap(),
            Start(BytesStart::new("tag"))
        );
        assert_eq!(
            r.read_to_end_into_async(QName(b"tag"), &mut buf)
                .await
                .unwrap(),
            5..11
        );
        assert_eq!(r.read_event_into_async(&mut buf).await.unwrap(), Eof);
    }

    #[tokio::test]
    async fn tag() {
        let mut r = Reader::from_str("<tag> <nested/> </tag>");
        //                            ^0   ^5         ^16
        r.config_mut().trim_text(true);

        let mut buf = Vec::new();
        assert_eq!(
            r.read_event_into_async(&mut buf).await.unwrap(),
            Start(BytesStart::new("tag"))
        );
        assert_eq!(
            r.read_to_end_into_async(QName(b"tag"), &mut buf)
                .await
                .unwrap(),
            5..16
        );
        assert_eq!(r.read_event_into_async(&mut buf).await.unwrap(), Eof);
    }
}

/// Regression test for https://github.com/tafia/quick-xml/issues/751
///
/// Actually, that error was not found in async reader, but we would to test it as well.
#[tokio::test]
async fn issue751() {
    let mut text = Vec::new();
    let mut chunk = Vec::new();
    chunk.extend_from_slice(b"<content>");
    for data in iter::repeat(b"some text inside").take(1000) {
        chunk.extend_from_slice(data);
        text.extend_from_slice(data);
    }
    chunk.extend_from_slice(b"</content>");

    let mut reader = Reader::from_reader(quick_xml::utils::Fountain {
        chunk: &chunk,
        consumed: 0,
        overall_read: 0,
    });
    let mut buf = Vec::new();
    let mut starts = 0u64;
    let mut ends = 0u64;
    let mut texts = 0u64;
    loop {
        buf.clear();
        match reader.read_event_into_async(&mut buf).await {
            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
            Ok(Eof) => break,

            Ok(Start(e)) => {
                starts += 1;
                assert_eq!(
                    e.name(),
                    QName(b"content"),
                    "starts: {starts}, ends: {ends}, texts: {texts}"
                );
            }
            Ok(End(e)) => {
                ends += 1;
                assert_eq!(
                    e.name(),
                    QName(b"content"),
                    "starts: {starts}, ends: {ends}, texts: {texts}"
                );
            }
            Ok(Text(e)) => {
                texts += 1;
                assert_eq!(
                    e.as_ref(),
                    text,
                    "starts: {starts}, ends: {ends}, texts: {texts}"
                );
            }
            _ => (),
        }
        // If we successfully read more than `u32::MAX`, the test is passed
        if reader.get_ref().overall_read >= u32::MAX as u64 {
            break;
        }
    }
}

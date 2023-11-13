use std::time::Duration;

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

#[tokio::test]
async fn test_cancel_future() {
    use tokio::io::BufReader;

    // represents something like a TCP socket, that receives some XML data
    // every now and then
    struct MockXmlSource {
        next_message_ready: bool,
    }
    impl tokio::io::AsyncRead for MockXmlSource {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            if !self.next_message_ready {
                return std::task::Poll::Pending;
            }

            let response = "<tag></tag>";
            buf.put_slice(response.as_bytes());

            self.next_message_ready = false;

            std::task::Poll::Ready(Ok(()))
        }
    }

    let source = MockXmlSource {
        next_message_ready: false,
    };
    let reader = BufReader::new(source);
    let mut reader = Reader::from_reader(reader);

    for _ in 0..3 {
        // some new message has arrived on the wire
        reader.get_mut().get_mut().next_message_ready = true;

        for _ in 0..3 {
            let fut = async {

                // read start event
                let mut buf = Vec::new();
                let start_event = reader.read_event_into_async(&mut buf).await.unwrap();
                let Start(start_event) = start_event else {
                    panic!("Expected start event");
                };

                // read until end event
                let mut buf = Vec::new();
                reader
                    .read_to_end_into_async(start_event.name(), &mut buf)
                    .await
                    .unwrap();
            };

            // read the data. if it takes more than 1ms, assume we read all the
            // data for now and cancel the future.
            let timeout_fut = tokio::time::timeout(Duration::from_millis(1), fut);
            if timeout_fut.await.is_err() {
                break;
            }
        }
    }
}

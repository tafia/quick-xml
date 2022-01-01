use criterion::{self, criterion_group, criterion_main, Criterion};
use quick_xml::events::Event;
use quick_xml::Reader;

static SAMPLE: &[u8] = include_bytes!("../tests/sample_rss.xml");
static PLAYERS: &[u8] = include_bytes!("../tests/players.xml");

fn quick_xml_normal(c: &mut Criterion) {
    let mut group = c.benchmark_group("quick_xml_normal");
    group.bench_function("untrimmed", |b| {
        b.iter(|| {
            let mut r = Reader::from_reader(SAMPLE);
            r.check_end_names(false).check_comments(false);
            let mut count = criterion::black_box(0);
            let mut buf = Vec::new();
            loop {
                match r.read_event(&mut buf) {
                    Ok(Event::Start(_)) | Ok(Event::Empty(_)) => count += 1,
                    Ok(Event::Eof) => break,
                    _ => (),
                }
                buf.clear();
            }
            assert_eq!(count, 1550);
        })
    });

    group.bench_function("trimmed", |b| {
        b.iter(|| {
            let mut r = Reader::from_reader(SAMPLE);
            r.check_end_names(false)
                .check_comments(false)
                .trim_text(true);
            let mut count = criterion::black_box(0);
            let mut buf = Vec::new();
            loop {
                match r.read_event(&mut buf) {
                    Ok(Event::Start(_)) | Ok(Event::Empty(_)) => count += 1,
                    Ok(Event::Eof) => break,
                    _ => (),
                }
                buf.clear();
            }
            assert_eq!(count, 1550);
        });
    });
    group.finish();
}

fn quick_xml_namespaced(c: &mut Criterion) {
    let mut group = c.benchmark_group("quick_xml_namespaced");
    group.bench_function("untrimmed", |b| {
        b.iter(|| {
            let mut r = Reader::from_reader(SAMPLE);
            r.check_end_names(false).check_comments(false);
            let mut count = criterion::black_box(0);
            let mut buf = Vec::new();
            let mut ns_buf = Vec::new();
            loop {
                match r.read_namespaced_event(&mut buf, &mut ns_buf) {
                    Ok((_, Event::Start(_))) | Ok((_, Event::Empty(_))) => count += 1,
                    Ok((_, Event::Eof)) => break,
                    _ => (),
                }
                buf.clear();
            }
            assert_eq!(count, 1550);
        });
    });

    group.bench_function("trimmed", |b| {
        b.iter(|| {
            let mut r = Reader::from_reader(SAMPLE);
            r.check_end_names(false)
                .check_comments(false)
                .trim_text(true);
            let mut count = criterion::black_box(0);
            let mut buf = Vec::new();
            let mut ns_buf = Vec::new();
            loop {
                match r.read_namespaced_event(&mut buf, &mut ns_buf) {
                    Ok((_, Event::Start(_))) | Ok((_, Event::Empty(_))) => count += 1,
                    Ok((_, Event::Eof)) => break,
                    _ => (),
                }
                buf.clear();
            }
            assert_eq!(count, 1550);
        });
    });
    group.finish();
}

fn quick_xml_escaped(c: &mut Criterion) {
    let mut group = c.benchmark_group("quick_xml_escaped");
    group.bench_function("untrimmed", |b| {
        b.iter(|| {
            let mut buf = Vec::new();
            let mut r = Reader::from_reader(SAMPLE);
            r.check_end_names(false).check_comments(false);
            let mut count = criterion::black_box(0);
            let mut nbtxt = criterion::black_box(0);
            loop {
                match r.read_event(&mut buf) {
                    Ok(Event::Start(_)) | Ok(Event::Empty(_)) => count += 1,
                    Ok(Event::Text(ref e)) => nbtxt += e.unescaped().unwrap().len(),
                    Ok(Event::Eof) => break,
                    _ => (),
                }
                buf.clear();
            }
            assert_eq!(count, 1550);

            // Windows has \r\n instead of \n
            #[cfg(windows)]
            assert_eq!(nbtxt, 67661);

            #[cfg(not(windows))]
            assert_eq!(nbtxt, 66277);
        });
    });

    group.bench_function("trimmed", |b| {
        b.iter(|| {
            let mut buf = Vec::new();
            let mut r = Reader::from_reader(SAMPLE);
            r.check_end_names(false)
                .check_comments(false)
                .trim_text(true);
            let mut count = criterion::black_box(0);
            let mut nbtxt = criterion::black_box(0);
            loop {
                match r.read_event(&mut buf) {
                    Ok(Event::Start(_)) | Ok(Event::Empty(_)) => count += 1,
                    Ok(Event::Text(ref e)) => nbtxt += e.unescaped().unwrap().len(),
                    Ok(Event::Eof) => break,
                    _ => (),
                }
                buf.clear();
            }
            assert_eq!(count, 1550);

            // Windows has \r\n instead of \n
            #[cfg(windows)]
            assert_eq!(nbtxt, 50334);

            #[cfg(not(windows))]
            assert_eq!(nbtxt, 50261);
        });
    });
    group.finish();
}

fn quick_xml_one_event(c: &mut Criterion) {
    let mut group = c.benchmark_group("quick_xml_one_event");
    group.bench_function("text_event", |b| {
        let src = "Hello world!".repeat(512 / 12).into_bytes();
        let mut buf = Vec::with_capacity(1024);
        b.iter(|| {
            let mut r = Reader::from_reader(src.as_ref());
            let mut nbtxt = criterion::black_box(0);
            r.check_end_names(false).check_comments(false);
            match r.read_event(&mut buf) {
                Ok(Event::Text(ref e)) => nbtxt += e.unescaped().unwrap().len(),
                something_else => panic!("Did not expect {:?}", something_else),
            };

            buf.clear();

            assert_eq!(nbtxt, 504);
        })
    });

    group.bench_function("start_event_trimmed", |b| {
        let src = format!(r#"<hello target="{}">"#, "world".repeat(512 / 5)).into_bytes();
        let mut buf = Vec::with_capacity(1024);
        b.iter(|| {
            let mut r = Reader::from_reader(src.as_ref());
            let mut nbtxt = criterion::black_box(0);
            r.check_end_names(false)
                .check_comments(false)
                .trim_text(true);
            match r.read_event(&mut buf) {
                Ok(Event::Start(ref e)) => nbtxt += e.unescaped().unwrap().len(),
                something_else => panic!("Did not expect {:?}", something_else),
            };

            buf.clear();

            assert_eq!(nbtxt, 525);
        })
    });

    group.bench_function("comment_event_trimmed", |b| {
        let src = format!(r#"<!-- hello "{}" -->"#, "world".repeat(512 / 5)).into_bytes();
        let mut buf = Vec::with_capacity(1024);
        b.iter(|| {
            let mut r = Reader::from_reader(src.as_ref());
            let mut nbtxt = criterion::black_box(0);
            r.check_end_names(false)
                .check_comments(false)
                .trim_text(true);
            match r.read_event(&mut buf) {
                Ok(Event::Comment(ref e)) => nbtxt += e.unescaped().unwrap().len(),
                something_else => panic!("Did not expect {:?}", something_else),
            };

            buf.clear();

            assert_eq!(nbtxt, 520);
        })
    });

    group.bench_function("cdata_event_trimmed", |b| {
        let src = format!(r#"<![CDATA[hello "{}"]]>"#, "world".repeat(512 / 5)).into_bytes();
        let mut buf = Vec::with_capacity(1024);
        b.iter(|| {
            let mut r = Reader::from_reader(src.as_ref());
            let mut nbtxt = criterion::black_box(0);
            r.check_end_names(false)
                .check_comments(false)
                .trim_text(true);
            match r.read_event(&mut buf) {
                Ok(Event::CData(ref e)) => nbtxt += e.unescaped().unwrap().len(),
                something_else => panic!("Did not expect {:?}", something_else),
            };

            buf.clear();

            assert_eq!(nbtxt, 518);
        })
    });
    group.finish();
}

fn quick_xml_attributes(c: &mut Criterion) {
    let mut group = c.benchmark_group("quick_xml_attributes");
    group.bench_function("iter_attributes", |b| {
        b.iter(|| {
            let mut r = Reader::from_reader(PLAYERS);
            r.check_end_names(false).check_comments(false);
            let mut count = criterion::black_box(0);
            let mut buf = Vec::new();
            loop {
                match r.read_event(&mut buf) {
                    Ok(Event::Empty(e)) => {
                        for attr in e.attributes() {
                            let _attr = attr.unwrap();
                            count += 1
                        }
                    }
                    Ok(Event::Eof) => break,
                    _ => (),
                }
                buf.clear();
            }
            assert_eq!(count, 1041);
        })
    });

    group.bench_function("iter_attributes_no_checks", |b| {
        b.iter(|| {
            let mut r = Reader::from_reader(PLAYERS);
            r.check_end_names(false).check_comments(false);
            let mut count = criterion::black_box(0);
            let mut buf = Vec::new();
            loop {
                match r.read_event(&mut buf) {
                    Ok(Event::Empty(e)) => {
                        for attr in e.attributes().with_checks(false) {
                            let _attr = attr.unwrap();
                            count += 1
                        }
                    }
                    Ok(Event::Eof) => break,
                    _ => (),
                }
                buf.clear();
            }
            assert_eq!(count, 1041);
        })
    });

    group.bench_function("try_get_attribute", |b| {
        b.iter(|| {
            let mut r = Reader::from_reader(PLAYERS);
            r.check_end_names(false).check_comments(false);
            let mut count = criterion::black_box(0);
            let mut buf = Vec::new();
            loop {
                match r.read_event(&mut buf) {
                    Ok(Event::Empty(e)) if e.name() == b"player" => {
                        for name in ["num", "status", "avg"] {
                            if let Some(_attr) = e.try_get_attribute(name).unwrap() {
                                count += 1
                            }
                        }
                        assert!(e
                            .try_get_attribute("attribute-that-doesn't-exist")
                            .unwrap()
                            .is_none());
                    }
                    Ok(Event::Eof) => break,
                    _ => (),
                }
                buf.clear();
            }
            assert_eq!(count, 150);
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    quick_xml_normal,
    quick_xml_escaped,
    quick_xml_namespaced,
    quick_xml_one_event,
    quick_xml_attributes
);
criterion_main!(benches);

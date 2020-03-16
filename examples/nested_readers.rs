extern crate quick_xml;
use quick_xml::events::Event;
use quick_xml::Reader;
// a structure to capture the rows we've extracted
// from a ECMA-376 table in document.xml
#[derive(Debug, Clone)]
struct TableStat {
    index: u8,
    rows: Vec<Vec<String>>,
}
// demonstrate how to nest readers
// This is useful for when you need to traverse
// a few levels of a document to extract things.
fn main() -> Result<(), quick_xml::Error> {
    let mut buf = Vec::new();
    // buffer for nested reader
    let mut skip_buf = Vec::new();
    let mut count = 0;
    let mut reader = Reader::from_file("tests/documents/document.xml")?;
    let mut found_tables = Vec::new();
    loop {
        match reader.read_event(&mut buf)? {
            Event::Start(element) => match element.name() {
                b"w:tbl" => {
                    count += 1;
                    let mut stats = TableStat {
                        index: count,
                        rows: vec![],
                    };
                    // must define stateful variables
                    // outside the nested loop else they are overwritten
                    let mut row_index = 0;
                    loop {
                        skip_buf.clear();
                        match reader.read_event(&mut skip_buf)? {
                            Event::Start(element) => match element.name() {
                                b"w:tr" => {
                                    stats.rows.push(vec![]);
                                    row_index = stats.rows.len() - 1;
                                }
                                b"w:tc" => {
                                    stats.rows[row_index]
                                        .push(String::from_utf8(element.name().to_vec()).unwrap());
                                }
                                _ => {}
                            },
                            Event::End(element) => {
                                if element.name() == b"w:tbl" {
                                    found_tables.push(stats);
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    assert_eq!(found_tables.len(), 2);
    // pretty print the table
    println!("{:#?}", found_tables);
    assert_eq!(found_tables[0].rows.len(), 2);
    assert_eq!(found_tables[0].rows[0].len(), 4);
    assert_eq!(found_tables[0].rows[1].len(), 4);

    assert_eq!(found_tables[1].rows.len(), 2);
    assert_eq!(found_tables[1].rows[0].len(), 4);
    assert_eq!(found_tables[1].rows[1].len(), 4);
    Ok(())
}

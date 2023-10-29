//! Contains tests for config options of a parser.
//!
//! Each module has a name of a corresponding option and functions inside performs
//! testing of various option values.
//!
//! Please keep tests sorted (exceptions are allowed if options are tightly related).

use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::reader::Reader;

mod expand_empty_elements {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Self-closed elements should be reported as one `Empty` event
    #[test]
    fn false_() {
        let mut reader = Reader::from_str("<root/>");
        reader.expand_empty_elements(false);

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Empty(BytesStart::new("root"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }

    /// Self-closed elements should be reported as two events
    #[test]
    fn true_() {
        let mut reader = Reader::from_str("<root/>");
        reader.expand_empty_elements(true);

        assert_eq!(
            reader.read_event().unwrap(),
            Event::Start(BytesStart::new("root"))
        );
        assert_eq!(
            reader.read_event().unwrap(),
            Event::End(BytesEnd::new("root"))
        );
        assert_eq!(reader.read_event().unwrap(), Event::Eof);
    }
}

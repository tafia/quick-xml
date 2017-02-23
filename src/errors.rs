//! Error management module

#![allow(missing_docs)]

error_chain! {
    foreign_links {
        Io(::std::io::Error);
    }
    errors {
        At(i: usize, err: Box<Error>) {
            description("error occured at particular position")
            display("error occured at reader position: {}: {:?}", i, err)
        }
        EndEventMismatch(expected: String, found: String) {
            description("end event name mismatch with last start event name")
            display("expecting </{}> found </{}>", expected, found)
        }
        Attribute(msg: String, i: usize) {
            description("error while parsing attributes")
            display("error while parsing attribute at position {}: {}", i, msg)
        }
        Escape(msg: String, range: ::std::ops::Range<usize>) {
            description("error while escaping bytes")
            display("Error while escaping character at range {:?}: {}", range, msg)
        }
    }
}
